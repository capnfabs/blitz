use itertools::Itertools;

use crate::fuji_compressed::bytecounter::ByteCounter;
use crate::fuji_compressed::process_common::{
    collect_carry_lines, compute_weighted_average_even, flatten, grad_and_weighted_avg_even,
    grad_and_weighted_avg_odd, is_interpolated, load_even_coefficients, PROCESS, UNSET,
};
use crate::fuji_compressed::sample::{Grad, Gradients, Sample};
use crate::fuji_compressed::zip_with_offset::zip_with_offset;
use crate::util::colored::Colored;
use crate::util::datagrid::{DataGrid, Position, Size};
use crate::Color;
use bitbit::BitWriter;
use std::io;
use std::iter::repeat;

const STRIPE_WIDTH: usize = 768;
// There's a max of 4 green pixels out of every 6, so we need 512 slots for every line of pixels
const REQUIRED_CAPACITY: usize = STRIPE_WIDTH * 4 / 6;

// Given a `row` of Xtrans sensor data, and a `row_map` which maps an index in
// the row to a color, populates `colors` with the pixels values in the right
// spot, using the Defined Layout.
fn map_xtrans_to_contiguous_colors(
    colors: &mut Colored<&mut Vec<u16>>,
    row: &[u16],
    row_map: &DataGrid<Color>,
) {
    for (_, x) in colors.iter() {
        assert_eq!(x.len(), REQUIRED_CAPACITY);
    }
    assert_eq!(row.len(), STRIPE_WIDTH);

    for (pos, val) in row.iter().enumerate() {
        // produces the sequence 0,1,1,2,3,3,4,5,5...
        // TODO: write why this works
        let squashed_idx = (((pos as i32 - 1) * 2).div_euclid(3) + 1) as usize;
        let color = row_map.at(Position(pos, 0));
        colors[color][squashed_idx] = *val;
    }
}

// Once `map_xtrans_to_contiguous_colors` has been called, you'll end up
// with a row where not every index has been filled in. This method fills in
// those blanks (which are all in `even` indices, because of the layout
// algorithm) with the weighted average.
fn fill_blanks_in_row(row: &mut [u16], rprev: &[u16], rprevprev: &[u16]) {
    for (idx, x) in row.iter_mut().enumerate() {
        if *x == UNSET {
            *x = compute_weighted_average_even(load_even_coefficients(rprev, rprevprev, idx))
        }
    }
}

pub fn compress<T: io::Write>(img_grid: DataGrid<u16>, cm: &DataGrid<Color>, mut data: T) {
    // TODO: loop this.
    let stripe = img_grid.subgrid(
        Position(0, 0),
        Size(STRIPE_WIDTH, img_grid.size().1 as usize),
    );
    let mut output = BitOutputSampleTarget::wrap(&mut data);
    process_stripe(&stripe, &cm, &mut output);
    output.finalize_block().unwrap();
}

fn process_stripe<T: SampleTarget>(
    stripe: &DataGrid<u16>,
    color_map: &DataGrid<Color>,
    output: &mut T,
) {
    let mut prev_lines = {
        let zeros = vec![0u16; REQUIRED_CAPACITY];
        Colored::new(
            vec![zeros.clone(), zeros.clone()],
            vec![zeros.clone(), zeros.clone()],
            vec![zeros.clone(), zeros.clone()],
        )
    };

    // Ok, there's a separate set of gradients for values in odd and even
    // x locations in the mapped dataset (I need a better name for 'the mapped dataset'!)
    // Each 'gradient set' has three sub-sets. I don't know why there's three.
    // Then, each of those sub-sets has 41 'gradients'.
    // You choose the gradient by:
    // - computing the difference between certain coefficients
    // - quantising those values
    // - looking up a gradient from those quantised values.
    let mut gradients = ([[Grad::default(); 41]; 3], [[Grad::default(); 41]; 3]);

    let num_lines = stripe.size().1 / 6;

    for line in 0..num_lines {
        let line = stripe.subgrid(Position(0, 6 * line), Size(STRIPE_WIDTH, 6));
        let results = process_line(&line, &color_map, &mut gradients, &prev_lines, output);
        prev_lines = collect_carry_lines(&results);
    }
}

trait SampleTarget {
    fn write(&mut self, sample: Sample) -> io::Result<()>;
}

struct BitOutputSampleTarget<'a, T: std::io::Write> {
    writer: BitWriter<ByteCounter<&'a mut T>>,
}

const BYTE_ALIGNMENT_TARGET: usize = 8;

impl<'a, T: std::io::Write> BitOutputSampleTarget<'a, T> {
    pub fn wrap(write: &'a mut T) -> Self {
        let bit_output = BitWriter::new(ByteCounter::new(write));
        BitOutputSampleTarget { writer: bit_output }
    }

    fn write_zeros_and_one(&mut self, num_zeros: usize) -> std::io::Result<()> {
        for _ in 0..num_zeros {
            self.writer.write_bit(false)?;
        }
        self.writer.write_bit(true)
    }

    pub fn finalize_block(&mut self) -> std::io::Result<()> {
        self.writer.pad_to_byte()?;
        let counter = self.writer.get_ref();
        let pad_bytes = BYTE_ALIGNMENT_TARGET - (counter.bytes_written() % BYTE_ALIGNMENT_TARGET);
        for _ in 0..pad_bytes {
            self.writer.write_byte(0)?
        }
        Ok(())
    }
}

impl<'a, T: std::io::Write> SampleTarget for BitOutputSampleTarget<'a, T> {
    fn write(&mut self, sample: Sample) -> io::Result<()> {
        match sample {
            Sample::Zero => self.writer.write_bit(true),
            Sample::EntireDelta(val, invert) => {
                self.write_zeros_and_one(41)?;
                self.writer.write_bits(val as u32, 13)?;
                self.writer.write_bit(invert)
            }
            Sample::SplitDelta {
                upper,
                lower,
                lower_bits,
                invert,
            } => {
                self.write_zeros_and_one(upper as usize)?;
                if lower_bits != 0 {
                    self.writer.write_bits(lower as u32, lower_bits)?;
                }
                self.writer.write_bit(invert)
            }
        }
    }
}

// A 'line' is a [strip-width]x6 row of data. This corresponds to the Xtrans 6x6 grid, repeated [strip-width]/6 times horizontally.
fn process_line<T: SampleTarget>(
    line: &DataGrid<u16>,
    color_map: &DataGrid<Color>,
    gradients: &mut (Gradients, Gradients),
    carry_results: &Colored<Vec<Vec<u16>>>,
    output: &mut T,
) -> Colored<Vec<Vec<u16>>> {
    let mut colors = Colored::new(
        vec![vec![UNSET; 512]; 3],
        vec![vec![UNSET; 512]; 6],
        vec![vec![UNSET; 512]; 3],
    );
    // This row_idx is the horizontal row in the Xtrans sensor data.
    for row_idx in 0..6 {
        let (r, g, b) = colors.split_mut();
        // In a 6x6 Xtrans layout, there's 20/36 green pixels, 8/36 blue pixels, 8/36 red pixels.
        // The lines with the _most_ green have 4/6 green pixels. The lines with the _most_ red or blue pixels have 2/6 pixels of that color.
        // What this means: you can comfortably 'squash' consecutive lines on top of each other, and if you choose the lines right, you won't get index collisions. I'll talk about that at length somewhere.
        // We can't do this for Green though, because there's so many Green pixels.
        let mut line_colors =
            Colored::new(&mut r[row_idx / 2], &mut g[row_idx], &mut b[row_idx / 2]);
        map_xtrans_to_contiguous_colors(
            &mut line_colors,
            line.row(row_idx),
            &color_map.subgrid(Position(0, row_idx), Size(6, 1)),
        );
    }

    fill_blanks_in_line(carry_results, &mut colors);

    // This is the thing that actually does the work.
    make_samples_for_line(&mut colors, gradients, carry_results, output);

    colors
}

fn make_samples_for_line<T: SampleTarget>(
    colors: &Colored<Vec<Vec<u16>>>,
    gradients: &mut (Gradients, Gradients),
    carry_results: &Colored<Vec<Vec<u16>>>,
    output: &mut T,
) {
    for (color_a, color_b, grad_set_idx) in &PROCESS {
        // The ordering in which these are output is kinda gross. For colors CA and CB:
        // Alternate between the first 4 even locations for both CA and CB
        // Then alternate between CA even, CB even, CA odd, CB odd
        // Note that both CA even and CB even could have gaps and therefore those indices could be skipped.
        // Note on ordering:
        // Doing CA even and CB even simultaneously is important because they both
        // update the same set of gradients, so you'll compute the wrong samples if you do this out of order.
        // Interleaving all four of them in this order is important because that's the output format.
        // If we instead _changed_ this to emit samples on a per-color basis,
        // we could zip them up later, and then we'd ben able to treat (ca_even, cb_even) separately from (ca_odd, cb_odd).
        // Note *also* that it's theoretically possible to update all the gradients in one step, and then output everything in another, but you'd be mixing up an awful lot of state to make that happen anyway. Compression algorithms where the coefficients are adaptive don't really lend themselves to immutability 😅
        let ca_even = repeat(color_a).zip((0..512).step_by(2));
        let ca_odd = repeat(color_a).zip((0..512).skip(1).step_by(2));
        let cb_even = repeat(color_b).zip((0..512).step_by(2));
        let cb_odd = repeat(color_b).zip((0..512).skip(1).step_by(2));
        // This starts processing the odd entries after the first 4 even entries are processed.
        let zipped = zip_with_offset(ca_even.zip_eq(cb_even), 0, ca_odd.zip_eq(cb_odd), 4)
            .map(|(a, b)| (flatten(a), flatten(b)));

        for ((ca_even, cb_even), (ca_odd, cb_odd)) in zipped {
            for thing in vec![ca_even, cb_even, ca_odd, cb_odd] {
                if let Some(((color, row), idx)) = thing {
                    if !is_interpolated(*color, *row, idx) {
                        let sample = make_sample(
                            &colors,
                            gradients,
                            &carry_results,
                            *row,
                            *color,
                            idx,
                            *grad_set_idx,
                        );
                        output.write(sample).unwrap();
                    }
                }
            }
        }
    }
}

// What it says on the tin. Makes a sample for the (Color;row;col) tuple in `colors`.
fn make_sample(
    colors: &Colored<Vec<Vec<u16>>>,
    gradients: &mut ([[Grad; 41]; 3], [[Grad; 41]; 3]),
    carry_results: &Colored<Vec<Vec<u16>>>,
    row_idx: usize,
    color: Color,
    idx: usize,
    grad_set: usize,
) -> Sample {
    let is_even = idx % 2 == 0;
    // Setup. Choose coefficients based on color / row etc
    let carry_results = &carry_results[color];
    let cdata = &colors[color];
    let (even_gradients, odd_gradients) = gradients;
    let (rprevprev, rprev) = match row_idx {
        0 => (carry_results[0].as_slice(), carry_results[1].as_slice()),
        1 => (carry_results[1].as_slice(), cdata[0].as_slice()),
        row => (cdata[row - 2].as_slice(), cdata[row - 1].as_slice()),
    };
    // Computation is different based on whether the index is odd or even.
    let (grad_set, (weighted_average, which_grad)) = if is_even {
        (
            &mut even_gradients[grad_set],
            grad_and_weighted_avg_even(idx, rprevprev, rprev),
        )
    } else {
        (
            &mut odd_gradients[grad_set],
            grad_and_weighted_avg_odd(idx, rprevprev, rprev, &cdata[row_idx]),
        )
    };
    let grad = &mut grad_set[which_grad.abs() as usize];
    let actual_value = cdata[row_idx][idx];
    let grad_is_negative = which_grad < 0;

    let sample = compute_sample(weighted_average, actual_value, grad, grad_is_negative);

    // Finally: update gradient.
    let delta = actual_value as i32 - weighted_average as i32;
    grad.update_from_value(delta.abs());

    println!("{:?}[{}][{}]: {:?}", color, row_idx, idx, sample);

    sample
}

// TODO: this function kinda needs to be explained better.
fn compute_sample(
    weighted_average: u16,
    actual_value: u16,
    grad: &Grad,
    grad_instructs_subtraction: bool,
) -> Sample {
    let delta = actual_value as i32 - weighted_average as i32;
    let abs_delta = delta.abs() as u16;
    let dec_bits = grad.bit_diff() as u16;
    let mask_dec_bits = dec_bits.saturating_sub(1);
    let split_mask = (1 << mask_dec_bits) - 1;
    // 'sample' in libraw terminology
    let upper = (abs_delta & (!split_mask)) >> mask_dec_bits;
    let lower = abs_delta & split_mask;

    // Here's the bit where we decide how to encode the sample.
    // TODO: we can probably change this structure such that it accepts some
    // input parameters and then decides on the encoding later. Could be useful
    // for making it clearer how this encoding process works?
    if upper > 40 {
        Sample::EntireDelta(abs_delta - 1, (delta < 0) == grad_instructs_subtraction)
    } else if dec_bits == 0 {
        // TODO: how does this work / why is it true?
        // If dec_bits is zero, delta is always zero, but it's not true in
        // reverse.
        assert_eq!(delta, 0);
        Sample::Zero
    } else if delta == 0 || (delta < 0) == grad_instructs_subtraction {
        Sample::SplitDelta {
            upper,
            lower,
            lower_bits: mask_dec_bits as usize,
            invert: false,
        }
    } else {
        // We're guaranteed this is non-zero by previous branch.
        // Subtract one and re-split.
        let abs_delta = abs_delta - 1;
        let upper = (abs_delta & (!split_mask)) >> mask_dec_bits;
        let lower = abs_delta & split_mask;

        Sample::SplitDelta {
            upper,
            lower,
            lower_bits: mask_dec_bits as usize,
            invert: true,
        }
    }
}

// The squashing process has left blanks in the lines. Our algorithm
// operates by taking weighted averages of neighbouring pixels though, and
// if we've got holes in the data, it's going to be complicated. So, we
// iterate through and fill in the blanks by computing _their_ weighted averages.
// There's never a hole in odd-indexed values, so we don't have to worry about
// gaps when doing the filling.
// Note that filling isn't technically necessary - we could do it on demand as required.
// But, we're going to iterate over every pixel in the set!
// There's an argument to be made that this 'muddies' the meaning of the data in 'colors' --
// I agree with that; but I think a better way of handling that is, rather than keeping the
// interpolated values separate, it would make sense to replace 'u16' by an enum or something.
fn fill_blanks_in_line(
    carry_results: &Colored<Vec<Vec<u16>>>,
    colors: &mut Colored<Vec<Vec<u16>>>,
) {
    for (color, cdata) in colors.iter_mut() {
        let prev_lines = &carry_results[color];
        for idx in 0..cdata.len() {
            let (history, future) = cdata.split_at_mut(idx);
            let (rprevprev, rprev) = match history.len() {
                0 => (prev_lines[0].as_slice(), prev_lines[1].as_slice()),
                1 => (prev_lines[1].as_slice(), history[0].as_slice()),
                idx => (history[idx - 2].as_slice(), history[idx - 1].as_slice()),
            };
            fill_blanks_in_row(future[0].as_mut_slice(), rprev, rprevprev)
        }
    }
}

#[cfg(test)]
mod test {

    use crate::fuji_compressed::compress::{process_stripe, BitOutputSampleTarget, STRIPE_WIDTH};
    use crate::util::datagrid::{DataGrid, Size};
    use crate::Color::{Blue, Green, Red};
    use itertools::Itertools;
    use std::convert::TryInto;
    use std::io::Cursor;

    const UNCOMPRESSED: &[u8] = include_bytes!("testdata/DSCF2279-block0.uncompressed.bin");
    const COMPRESSED: &[u8] = include_bytes!("testdata/DSCF2279-block0.compressed.bin");

    #[test]
    fn end_to_end_stripe_processor() {
        let input = UNCOMPRESSED
            .chunks_exact(2)
            .map(|x| u16::from_le_bytes(x.try_into().unwrap()))
            .collect_vec();

        let stripe = DataGrid::wrap(&input, Size(STRIPE_WIDTH, input.len() / STRIPE_WIDTH));
        let color_map = DataGrid::wrap(
            &[
                Green, Green, Red, Green, Green, Blue, Green, Green, Blue, Green, Green, Red, Blue,
                Red, Green, Red, Blue, Green, Green, Green, Blue, Green, Green, Red, Green, Green,
                Red, Green, Green, Blue, Red, Blue, Green, Blue, Red, Green,
            ],
            Size(6, 6),
        );
        let mut data: Cursor<Vec<u8>> = Cursor::new(Vec::new());

        let mut output = BitOutputSampleTarget::wrap(&mut data);
        process_stripe(&stripe, &color_map, &mut output);
        output.finalize_block().unwrap();
        let output = data.into_inner();
        // TODO: prevent printing on failure; but dump somewhere useful instead.
        let actual = output.as_slice();
        let expected = COMPRESSED;
        assert_eq!(actual.len(), expected.len());
        assert_eq!(actual, expected);
    }
}
