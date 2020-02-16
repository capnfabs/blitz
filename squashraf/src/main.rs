// Still WIP

use clap::{App, Arg};
use itertools::Itertools;
use libraw::util::{DataGrid, Position, Size};
use libraw::{Color, RawFile};

mod colored;
mod evenodd;
mod zip_with_offset;

use crate::zip_with_offset::zip_with_offset;

use crate::colored::Colored;
use std::io::Cursor;
use std::iter::repeat;

fn main() {
    let matches = App::new("Squashraf")
        .arg(Arg::with_name("INPUT").required(true).index(1))
        .get_matches();

    let input = matches.value_of("INPUT").unwrap();

    squash_raf(input);
}

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

// Ok, the idea here is:
// it's a weighted average of the values around it.
// - take the value immediately 'above' this one. Call this rb.
// - of the other three values:
//   - choose the two that are closest to rb
//   - call these two values `close`
// - now, compute close[0] + close[1] + 2*rb / 4.
// BUT ALSO
// if both north_west and north_east are equidistant from north, than use those for the weighted
// average regardless of how big they are. This feels like an implementation detail in the original
// fuji code. My guess is we can get better compression ratios by doing something different, but I don't know!
fn compute_weighted_average_even(ec: EvenCoefficients) -> u16 {
    let distance = |v: u16| (v as i32 - ec.north as i32).abs();
    let others = [ec.northwest, ec.northeast, ec.very_north]
        .iter()
        .copied()
        .sorted_by_key(|x| distance(*x))
        .collect_vec();

    if distance(ec.northwest) == distance(ec.northeast) {
        let val = (ec.northwest + ec.northeast + 2 * ec.north) / 4;
        return val;
    }

    (others[0] + others[1] + 2 * ec.north) / 4
}

fn compute_weighted_average_odd(oc: OddCoefficients) -> u16 {
    // If `north` is not in-between `north_west` and `north_east`
    // Then presumably it represents that there's not a continuous variation
    // horizontally. In that case, we want to factor `north` into the
    // computation, otherwise we don't care about it.
    // I'm entirely unsure _why_ this wasn't done for everything, it feels like
    // additional complexity for little benefit.
    if (oc.north > oc.north_west && oc.north > oc.north_east)
        || (oc.north < oc.north_west && oc.north < oc.north_east)
    {
        // Note on typing here: This will all fit in a u16, because we've got 4x max u14s.
        (oc.east + oc.west + 2 * oc.north) / 4
    } else {
        (oc.west + oc.east) / 2
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

// Safe to use as a sentinel because the image is only ever 14 bits
// and this is bigger than that.
const UNSET: u16 = 0xFFFF;

fn squash_raf(img_file: &str) {
    println!("Loading RAW data: libraw");
    let file = RawFile::open(img_file).unwrap();
    println!("Opened file: {:?}", file);

    let img_grid = DataGrid::wrap(
        file.raw_data(),
        Size(
            file.img_params().raw_width as usize,
            file.img_params().raw_height as usize,
        ),
    );
    let xtmap = file
        .xtrans_pixel_mapping()
        .iter()
        .flatten()
        .copied()
        .collect_vec();
    let cm = DataGrid::wrap(&xtmap, Size(6, 6));

    // TODO: loop this.
    let stripe = img_grid.subgrid(
        Position(0, 0),
        Size(STRIPE_WIDTH, file.img_params().raw_height as usize),
    );
    process_stripe(&stripe, &cm);
}

fn process_stripe(stripe: &DataGrid<u16>, color_map: &DataGrid<Color>) -> Vec<u8> {
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
    // These gradients *adapt over time*. They're comprised of two numbers:
    // - The first is SUM(ABS(difference between actual value and weighted average of previous pixels))
    // - The second is COUNT(processed pixels).
    // Periodically, they're 'squashed down' by dividing both values by two.
    // Two talk about the effect of this action, it's important to talk about what they're used for.
    // These two numbers are used as follows:
    // The 'bit diff' between the two is computed, which is effectively something vaguely logarithmic? I don't really understand how this works, and I probably should.
    let mut gradients = (
        [[Grad(GRADIENT_START_SUM_VALUE, 1); 41]; 3],
        [[Grad(GRADIENT_START_SUM_VALUE, 1); 41]; 3],
    );

    let num_lines = stripe.size().1 / 6;
    // This is where we're going to write the output to.
    let mut data: Cursor<Vec<u8>> = Cursor::new(Vec::new());
    let mut output = bitbit::BitWriter::new(&mut data);

    for line in 0..num_lines {
        let line = stripe.subgrid(Position(0, 6 * line), Size(STRIPE_WIDTH, 6));
        let results = process_line(&line, &color_map, &mut gradients, &prev_lines, &mut output);
        prev_lines = collect_carry_lines(results);
    }
    output.pad_to_byte().unwrap();
    // TODO: pad output to hit a 32-bit boundary (I think it's 32 bits, at least).
    data.into_inner()
}

// We need to pass some of the lines from previous lines to future lines, because they're used in calculations.
// For now, we clone them. It would be entirely possible to make that _not_ the case, but I couldn't be bothered
// for a v1, and this is mega-fast anyway.
fn collect_carry_lines(results: Colored<Vec<Vec<u16>>>) -> Colored<Vec<Vec<u16>>> {
    let reds = &results[Color::Red];
    let greens = &results[Color::Green];
    let blues = &results[Color::Blue];

    Colored::new(
        vec![reds[reds.len() - 2].clone(), reds[reds.len() - 1].clone()],
        vec![
            greens[greens.len() - 2].clone(),
            greens[greens.len() - 1].clone(),
        ],
        vec![
            blues[blues.len() - 2].clone(),
            blues[blues.len() - 1].clone(),
        ],
    )
}

#[derive(Debug, Clone, Copy)]
struct Grad(i32, i32);

type Gradients = [[Grad; 41]; 3];

// hardcoded for 14-bit sample size.
const GRADIENT_START_SUM_VALUE: i32 = 256;
const GRADIENT_MAX_COUNT: i32 = 64;

impl Grad {
    fn bit_diff(self) -> usize {
        let Grad(a, b) = self;
        let a = a as usize;
        let b = b as usize;

        if b < a {
            let mut dec_bits = 1;
            while dec_bits <= 12 && (b << dec_bits) < a {
                dec_bits += 1;
            }
            dec_bits
        } else {
            0
        }
    }

    fn update_from_value(&mut self, value: i32) {
        self.0 += value;
        if self.1 == GRADIENT_MAX_COUNT {
            self.0 /= 2;
            self.1 /= 2;
        }
        self.1 += 1;
    }
}

#[derive(Debug, Clone, Copy)]
struct EvenCoefficients {
    north: u16,
    northwest: u16,
    northeast: u16,
    very_north: u16,
}

type SampleTarget<'a> = bitbit::BitWriter<&'a mut Cursor<Vec<u8>>>;

// A 'line' is a [strip-width]x6 row of data. This corresponds to the Xtrans 6x6 grid, repeated [strip-width]/6 times horizontally.
fn process_line(
    line: &DataGrid<u16>,
    color_map: &DataGrid<Color>,
    gradients: &mut (Gradients, Gradients),
    carry_results: &Colored<Vec<Vec<u16>>>,
    output: &mut SampleTarget,
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

fn flatten<A, B>(opt: Option<(A, B)>) -> (Option<A>, Option<B>) {
    match opt {
        Some((a, b)) => (Some(a), Some(b)),
        None => (None, None),
    }
}

fn q_value(v: i32) -> i32 {
    if v <= -0x114 {
        -4
    } else if v <= -0x43 {
        -3
    } else if v <= -0x12 {
        -2
    } else if v < 0 {
        -1
    } else if v == 0 {
        0
    } else if v < 0x12 {
        1
    } else if v < 0x43 {
        2
    } else if v < 0x114 {
        3
    } else {
        4
    }
}

fn make_samples_for_line(
    colors: &Colored<Vec<Vec<u16>>>,
    gradients: &mut (Gradients, Gradients),
    carry_results: &Colored<Vec<Vec<u16>>>,
    output: &mut SampleTarget,
) {
    // We make samples interleaving two 'color lines' at a time.
    const PROCESS: [((Color, usize), (Color, usize), usize); 6] = [
        // Key for this: ((ColorA, row), (ColorB, row), gradient_set)
        ((Color::Red, 0), (Color::Green, 0), 0),
        ((Color::Green, 1), (Color::Blue, 0), 1),
        ((Color::Red, 1), (Color::Green, 2), 2),
        ((Color::Green, 3), (Color::Blue, 1), 0),
        ((Color::Red, 2), (Color::Green, 4), 1),
        ((Color::Green, 5), (Color::Blue, 2), 2),
    ];

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
                    if !skip(*color, *row, idx) {
                        let (sample, code, code_bits) = make_sample(
                            &colors,
                            gradients,
                            &carry_results,
                            *row,
                            *color,
                            idx,
                            *grad_set_idx,
                        );

                        // This is the encoding
                        // TODO: change the `output` thing to something that accumulates Sample structures.
                        // That would make this easier to test.
                        for _ in 0..sample {
                            output.write_bit(false).unwrap();
                        }
                        output.write_bit(true).unwrap();
                        if code_bits != 0 {
                            output.write_bits(code as u32, code_bits).unwrap();
                        }
                    }
                }
            }
        }
    }
}

// This is a hardcoded function defining pixels to skip. You probably shouldn't do this in the real
// world, but should instead do something else, like, store which things are computed / inferred and
// which one's aren't, and use that to work out if a value should be skipped or not.
fn skip(color: Color, row: usize, idx: usize) -> bool {
    if idx % 2 == 1 {
        // never skip odd
        false
    } else {
        match color {
            Color::Red => {
                (row == 0) || (row == 1 && (idx & 3 == 0)) || (row == 2 && (idx & 3 == 2))
            }
            Color::Green => (row == 2) || (row == 5),
            Color::Blue => {
                (row == 0) || (row == 1 && (idx & 3 == 2)) || (row == 2 && (idx & 3 == 0))
            }
        }
    }
}

enum Sample {
    // A sample that means 'just use the weighted average as-is'.
    Zero,
    // This represents the _entire delta_ between the weighted average and the
    // actual value. Use this when we're unable to use split-encoding because
    // we've got a large value of `upper`.
    EntireDelta(u16, bool),
    // This is the default 'split encoding' mechanism.
    SplitDelta {
        upper: u16,
        lower: u16,
        lower_bits: usize,
        invert: bool,
    },
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
) -> (u16, u16, usize) {
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
    let delta = actual_value as i32 - weighted_average as i32;

    // Finally: update gradient.
    grad.update_from_value(delta.abs());

    // TODO: move this; it belongs elsewhere.
    match sample {
        Sample::EntireDelta(val, invert) => (41, val << 1 | invert as u16, 14),
        Sample::SplitDelta {
            upper,
            lower,
            lower_bits,
            invert,
        } => (upper, lower << 1 | invert as u16, lower_bits),
        Sample::Zero => (0, 0, 0),
    }
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
    if upper > 40 {
        Sample::EntireDelta(abs_delta - 1, (delta < 0) == grad_instructs_subtraction)
    } else {
        if dec_bits == 0 {
            // TODO: how does this work / why is it true?
            assert_eq!(delta, 0);
            Sample::Zero
        } else if delta == 0 || (delta < 0) == grad_instructs_subtraction {
            // TODO: print when delta == 0 here.
            Sample::SplitDelta {
                upper,
                lower,
                lower_bits: dec_bits as usize,
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
                // TODO: right now, lower_bits includes the sign bit, and it shouldn't.
                // The issue is when dec_bits is 0; but we should probably handle that with a different enum value.
                lower_bits: dec_bits as usize,
                invert: true,
            }
        }
    }
}

fn grad_and_weighted_avg_even(idx: usize, rprevprev: &[u16], rprev: &[u16]) -> (u16, i32) {
    let ec = load_even_coefficients(rprev, rprevprev, idx);
    let weighted_average = compute_weighted_average_even(ec);
    let which_grad = 9 * q_value(ec.north as i32 - ec.very_north as i32)
        + q_value(ec.northwest as i32 - ec.north as i32);
    (weighted_average, which_grad)
}

fn grad_and_weighted_avg_odd(
    idx: usize,
    rprevprev: &[u16],
    rprev: &[u16],
    rthis: &[u16],
) -> (u16, i32) {
    let oc = load_odd_coefficients(rprevprev, rprev, rthis, idx);

    let weighted_average = compute_weighted_average_odd(oc);
    let which_grad = 9 * q_value(oc.north as i32 - oc.north_west as i32)
        + q_value(oc.north_west as i32 - oc.west as i32);
    (weighted_average, which_grad)
}

fn load_even_coefficients(rprev: &[u16], rprevprev: &[u16], idx: usize) -> EvenCoefficients {
    let leftmost_other = if idx == 0 {
        rprevprev[0]
    } else {
        rprev[idx - 1]
    };
    // Ensure that the vector is even-lengthed
    assert_eq!(rprev.len() % 2, 0);
    // Ensure that we're targeting an even value
    assert_eq!(idx % 2, 0);
    EvenCoefficients {
        north: rprev[idx],
        northwest: leftmost_other,
        // Note that row width is always a multiple of 2, so for an even index
        // there will always be at least one value to the right of it.
        northeast: rprev[idx + 1],
        very_north: rprevprev[idx],
    }
}

#[derive(Debug, Clone, Copy)]
struct OddCoefficients {
    west: u16,       // Ra
    north: u16,      // Rb
    north_west: u16, // Rc
    north_east: u16, // Rd
    east: u16,       // Rg
}

fn load_odd_coefficients(
    rprevprev: &[u16],
    rprev: &[u16],
    rthis: &[u16],
    idx: usize,
) -> OddCoefficients {
    // Ensure that the vector is even-lengthed
    assert_eq!(rprev.len() % 2, 0);
    assert_eq!(rprev.len(), rprevprev.len());
    assert_eq!(rprev.len(), rthis.len());
    // Ensure that we're targeting an odd value
    assert_eq!(idx % 2, 1);

    // the rightmost value is in a vaguely tricky situation. If it doesn't
    // exist, use the value immediately above instead.
    let last_idx = rprev.len() - 1;
    let (rightmost_rprev, rightmost_rthis) = if idx == last_idx {
        (rprevprev[rprevprev.len() - 1], rprev[rprev.len() - 1])
    } else {
        (rprev[idx + 1], rthis[idx + 1])
    };
    OddCoefficients {
        west: rthis[idx - 1],
        north: rprev[idx],
        north_west: rprev[idx - 1],
        north_east: rightmost_rprev,
        east: rightmost_rthis,
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
    use crate::{process_stripe, Grad, STRIPE_WIDTH};

    use libraw::util::{DataGrid, Size};

    use itertools::Itertools;
    use libraw::Color::{Blue, Green, Red};
    use std::convert::TryInto;
    use test_case::test_case;

    const UNCOMPRESSED: &[u8] = include_bytes!("DSCF2279-block0.uncompressed.bin");
    const COMPRESSED: &[u8] = include_bytes!("DSCF2279-block0.compressed.bin");

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
        let output = process_stripe(&stripe, &color_map);
        // TODO: add padding.
        // TODO: prevent printing on failure; but dump somewhere useful instead.
        assert_eq!(output.as_slice(), &COMPRESSED[0..COMPRESSED.len() - 6]);
        assert!(false);
    }

    #[test_case(Grad(256, 1) => 8)]
    #[test_case(Grad(256, 2) => 7)]
    #[test_case(Grad(256, 3) => 7)]
    #[test_case(Grad(397, 32) => 4)]
    #[test_case(Grad(397, 63) => 3)]
    #[test_case(Grad(397, 64) => 3)]
    #[test_case(Grad(397, 65) => 3)]
    #[test_case(Grad(397, 140) => 2)]
    #[test_case(Grad(397, 141) => 2)]
    #[test_case(Grad(397, 142) => 2)]
    fn bit_diff(grad: Grad) -> usize {
        grad.bit_diff()
    }
}
