#![allow(unused)]

use crate::fuji_compressed::process_common::{
    collect_carry_lines, compute_weighted_average_even, flatten, grad_and_weighted_avg_even,
    grad_and_weighted_avg_odd, is_interpolated, load_even_coefficients, load_odd_coefficients,
    PROCESS, UNSET,
};
use crate::fuji_compressed::sample::{Grad, Gradients, Sample};
use crate::fuji_compressed::zip_with_offset::zip_with_offset;
use crate::util::colored::Colored;
use crate::util::datagrid::{DataGrid, MutableDataGrid, Offset, Position, Size};
use crate::Color;
use crate::Color::{Blue, Green, Red};
use bitbit::{BitReader, MSB};
use itertools::{zip, Itertools};
use std::io;
use std::io::SeekFrom;
use std::iter::repeat;
use std::process::Output;

// There's a max of 4 green pixels out of every 6, so we need 512 slots for every line of 768 pixels (for example)
// TODO: un-hardcode
const STRIPE_WIDTH: usize = 768;
// I think????
const IMG_WIDTH: usize = 6048;
const REQUIRED_CAPACITY: usize = STRIPE_WIDTH * 4 / 6;
const NUM_LINES: usize = 673;
const IMG_HEIGHT: usize = NUM_LINES * 6;
const STRIPE_SIZE: Size = Size(STRIPE_WIDTH, IMG_HEIGHT);

// TODO: this is temp and should be removed
pub fn make_color_map() -> DataGrid<'static, Color> {
    DataGrid::wrap(
        &[
            Green, Green, Red, Green, Green, Blue, Green, Green, Blue, Green, Green, Red, Blue,
            Red, Green, Red, Blue, Green, Green, Green, Blue, Green, Green, Red, Green, Green, Red,
            Green, Green, Blue, Red, Blue, Green, Blue, Red, Green,
        ],
        Size(6, 6),
    )
}

pub fn inflate(
    blocks: impl Iterator<Item = impl io::Read>,
    color_map: &DataGrid<Color>,
) -> Vec<u16> {
    let num_stripes = (IMG_WIDTH as f32 / STRIPE_WIDTH as f32).ceil() as usize;
    // TODO: maybe init as not zeros, but as unset instead.
    let mut output = vec![0; IMG_WIDTH * IMG_HEIGHT];
    let mut mg = MutableDataGrid::new(&mut output, Size(IMG_WIDTH, IMG_HEIGHT));
    for (stripe_num, block) in (0..num_stripes).zip(blocks) {
        if stripe_num == 7 {
            continue;
        }
        let stripe_start = stripe_num * STRIPE_WIDTH;
        let stripe_end = stripe_start + STRIPE_WIDTH;
        let mut stripe_grid = if stripe_end < IMG_WIDTH {
            mg.subgrid(Position(stripe_start, 0), STRIPE_SIZE)
        } else {
            mg.subgrid(
                Position(stripe_start, 0),
                Size(IMG_WIDTH - stripe_start, IMG_HEIGHT),
            )
        };
        println!("Starting stripe {}", stripe_num);
        inflate_stripe(block, color_map, &mut stripe_grid);
        println!("Finished stripe {}", stripe_num);
    }
    output
}

pub fn inflate_stripe(
    reader: impl io::Read,
    color_map: &DataGrid<Color>,
    output: &mut MutableDataGrid<u16>,
) {
    let mut r: BitReader<_, MSB> = BitReader::new(reader);

    let mut prev_lines = {
        let zeros = vec![0u16; REQUIRED_CAPACITY];
        Colored::new(
            vec![zeros.clone(), zeros.clone()],
            vec![zeros.clone(), zeros.clone()],
            vec![zeros.clone(), zeros.clone()],
        )
    };

    let mut gradients = ([[Grad::default(); 41]; 3], [[Grad::default(); 41]; 3]);

    for line in 0..NUM_LINES {
        let results = inflate_line(&mut r, &color_map, &mut gradients, &prev_lines);
        prev_lines = collect_carry_lines(&results);
        // TODO: extract this as a method.
        for row_idx in 0..6 {
            let (r, g, b) = results.split();
            let line_colors = Colored::new(&r[row_idx / 2], &g[row_idx], &b[row_idx / 2]);
            map_contiguous_colors_to_xtrans(
                output.row_mut(row_idx + line * 6),
                &line_colors,
                &color_map.subgrid(Position(0, row_idx), Size(6, 1)),
            );
        }
    }
}

fn map_contiguous_colors_to_xtrans(
    output_row: &mut [u16],
    colors: &Colored<&Vec<u16>>,
    row_color_map: &DataGrid<Color>,
) {
    for (_, x) in colors.iter() {
        assert_eq!(x.len(), REQUIRED_CAPACITY);
    }
    assert!(output_row.len() <= STRIPE_WIDTH);
    for (pos, val) in output_row.iter_mut().enumerate() {
        // TODO: extract into a method, also used in map_xtrans_to_contiguous_colors
        let squashed_idx = (((pos as i32 - 1) * 2).div_euclid(3) + 1) as usize;
        let color = row_color_map.at(Position(pos, 0));
        *val = colors[color][squashed_idx];
    }
}

pub trait ValueTarget {
    fn write_val(&mut self, value: u16);
}

fn inflate_line<R: io::Read>(
    reader: &mut BitReader<R, MSB>,
    color_map: &DataGrid<Color>,
    gradients: &mut (Gradients, Gradients),
    carry_results: &Colored<Vec<Vec<u16>>>,
) -> Colored<Vec<Vec<u16>>> {
    let mut colors = Colored::new(
        vec![vec![UNSET; 512]; 3],
        vec![vec![UNSET; 512]; 6],
        vec![vec![UNSET; 512]; 3],
    );
    for (color_a, color_b, grad_set_idx) in &PROCESS {
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
                    let value = if is_interpolated(*color, *row, idx) {
                        interpolate_value(&colors, &carry_results, *row, *color, idx)
                    } else {
                        compute_value_and_update_gradients(
                            reader,
                            &colors,
                            gradients,
                            &carry_results,
                            *row,
                            *color,
                            idx,
                            *grad_set_idx,
                        )
                    };
                    colors[*color][*row][idx] = value;
                }
            }
        }
    }
    colors
}

fn interpolate_value(
    colors: &Colored<Vec<Vec<u16>>>,
    carry_results: &Colored<Vec<Vec<u16>>>,
    row_idx: usize,
    color: Color,
    idx: usize,
) -> u16 {
    assert_eq!(idx % 2, 0);
    let carry_results = &carry_results[color];
    let cdata = &colors[color];
    let (rprevprev, rprev) = match row_idx {
        0 => (carry_results[0].as_slice(), carry_results[1].as_slice()),
        1 => (carry_results[1].as_slice(), cdata[0].as_slice()),
        row => (cdata[row - 2].as_slice(), cdata[row - 1].as_slice()),
    };
    let ec = load_even_coefficients(rprev, rprevprev, idx);
    let weighted_average = compute_weighted_average_even(ec);
    weighted_average
}

fn compute_value_and_update_gradients<R: io::Read>(
    reader: &mut BitReader<R, MSB>,
    colors: &Colored<Vec<Vec<u16>>>,
    gradients: &mut ([[Grad; 41]; 3], [[Grad; 41]; 3]),
    carry_results: &Colored<Vec<Vec<u16>>>,
    row_idx: usize,
    color: Color,
    idx: usize,
    grad_set: usize,
) -> u16 {
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
    let grad_instructs_subtraction = which_grad < 0;

    let dec_bits = grad.bit_diff() as usize;

    let sample = read_sample(reader, dec_bits).unwrap();

    let delta = sample_to_delta(sample);
    // Finally: update gradient.
    let actual_value = if grad_instructs_subtraction {
        (weighted_average as i32 - delta as i32)
    } else {
        (weighted_average as i32 + delta as i32)
    };

    let old_grad = grad.clone();

    grad.update_from_value(delta.abs());

    /*
    println!(
        "{}{}[{}]: ref: {}, actual: {}, grad_neg: {}, grad_before: {:?}, grad_after: {:?}",
        color.letter(),
        row_idx,
        idx,
        weighted_average,
        actual_value,
        grad_instructs_subtraction as u8,
        old_grad,
        grad,
    );*/

    assert!(actual_value < (1 << 14));
    actual_value as u16
}

fn read_sample<T: io::Read>(
    reader: &mut BitReader<T, MSB>,
    lower_bits: usize,
) -> io::Result<Sample> {
    let upper = {
        let mut count = 0;
        while !reader.read_bit()? {
            count += 1;
        }
        count
    };

    if upper > 40 {
        let lower = reader.read_bits(13)?;
        let invert = reader.read_bit()?;
        // TODO: i've named this wrong or something. It's apparently "don't invert".
        // Fix it in the EntireDelta type by naming it properly.
        Ok(Sample::EntireDelta(lower as u16, !invert))
    } else if lower_bits == 0 {
        // TODO: I've probably named this wrong, see above
        let invert = (upper & 0b1) != 0;
        Ok(Sample::JustUpper(upper >> 1, invert))
    } else {
        // TODO: the story around dec_bits is a hot mess and needs to be fixed.
        let lower_bits = lower_bits - 1;
        let lower = reader.read_bits(lower_bits)?;
        assert!(lower < (1 << 14));
        let lower = lower as u16;
        let invert = reader.read_bit()?;
        Ok(Sample::SplitDelta {
            upper,
            lower,
            lower_bits,
            invert,
        })
    }
}

fn sample_to_delta(sample: Sample) -> i32 {
    match sample {
        Sample::JustUpper(val, invert) => {
            let val = val as i32;
            if invert {
                -(val + 1)
            } else {
                val
            }
        }
        Sample::EntireDelta(val, invert) => {
            let val = val as i32 + 1;
            if invert {
                -val
            } else {
                val
            }
        }
        Sample::SplitDelta {
            upper,
            lower,
            lower_bits,
            invert,
        } => {
            let val = (upper << lower_bits as u16) | lower;
            let val = val as i32;
            if invert {
                -(val + 1)
            } else {
                val
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::fuji_compressed::inflate::{
        inflate_stripe, make_color_map, NUM_LINES, STRIPE_WIDTH,
    };
    use crate::fuji_compressed::process_common::UNSET;
    use crate::util::datagrid::{DataGrid, MutableDataGrid, Size};
    use crate::Color::{Blue, Green, Red};
    use itertools::Itertools;
    use std::convert::TryInto;
    use std::io::Cursor;

    #[test]
    fn end_to_end_stripe_inflate() {
        const UNCOMPRESSED: &[u8] = include_bytes!("testdata/DSCF2279-block0.uncompressed.bin");
        const COMPRESSED: &[u8] = include_bytes!("testdata/DSCF2279-block0.compressed.bin");

        let expected = UNCOMPRESSED
            .chunks_exact(2)
            .map(|x| u16::from_le_bytes(x.try_into().unwrap()))
            .collect_vec();

        let mut actual_data = vec![UNSET; STRIPE_WIDTH * NUM_LINES * 6];
        let mut output = MutableDataGrid::new(&mut actual_data, Size(STRIPE_WIDTH, NUM_LINES * 6));

        inflate_stripe(&mut COMPRESSED, &make_color_map(), &mut output);
        assert_eq!(actual_data.len(), expected.len());
        assert_eq!(actual_data, expected.as_slice());
    }
}
