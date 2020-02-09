// Still WIP

use clap::{App, Arg};
use itertools::Itertools;
use libraw::util::{DataGrid, Position, Size};
use libraw::{Color, RawFile};

mod colored;
mod zip_with_offset;

use crate::zip_with_offset::zip_with_offset;

use crate::colored::Colored;
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

fn assign_into(colors: &mut Colored<&mut Vec<u16>>, row: &[u16], row_map: &DataGrid<Color>) {
    for (_, x) in colors.iter() {
        assert_eq!(x.len(), REQUIRED_CAPACITY);
    }
    assert_eq!(row.len(), STRIPE_WIDTH);

    for (pos, val) in row.iter().enumerate() {
        // produces the sequence 0,1,1,2,3,3,4,5,5...
        let squashed_idx = (((pos as i32 - 1) * 2).div_euclid(3) + 1) as usize;
        let color = row_map.at(Position(pos, 0));
        if squashed_idx == 0 || squashed_idx == 1 {
            //println!("pos {}, squashed {}, color {:?}", pos, squashed_idx, color);
        }
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
fn compute_weighted_average(ec: EvenCoefficients) -> u16 {
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

fn fill_blanks_in_row(row: &mut [u16], rprev: &[u16], rprevprev: &[u16]) {
    for (idx, x) in row.iter_mut().enumerate() {
        if *x == UNSET {
            *x = compute_weighted_average(load_even_coefficients(rprev, rprevprev, idx))
        }
    }
}

// Safe to use as a sentinel because the image is only ever 14 bits
// and this is bigger than that.
const UNSET: u16 = 0xFFFF;

const LOTS_OF_ZEROS: [u16; 512] = [0; 512];

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

fn process_stripe(stripe: &DataGrid<u16>, color_map: &DataGrid<Color>) {
    let zeros = LOTS_OF_ZEROS.to_vec();
    let mut prev_lines = Colored::new(
        vec![zeros.clone(), zeros.clone()],
        vec![zeros.clone(), zeros.clone()],
        vec![zeros.clone(), zeros.clone()],
    );

    let mut gradients = (
        [[Grad(GRADIENT_MAX_VALUE, 1); 41]; 3],
        [[Grad(GRADIENT_MAX_VALUE, 1); 41]; 3],
    );

    // TODO: calculate the num_lines and use the legit value.
    // each 'line' is a block of 6x768
    let num_lines = 3;
    for line in 0..num_lines {
        let line = stripe.subgrid(Position(0, 6 * line), Size(STRIPE_WIDTH, 6));
        let results = process_line(&line, &color_map, &mut gradients, &prev_lines);
        prev_lines = collect_carry_lines(results);
    }
}

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
// TODO: give these better names
const GRADIENT_MAX_VALUE: i32 = 256;
const GRADIENT_MIN_VALUE: i32 = 64;

impl Grad {
    fn bit_diff(self) -> usize {
        let Grad(a, b) = self;

        if b >= a {
            return 0;
        }

        // TODO: pretty sure we can use leading_zeros for this and subtract results.
        let mut bits: usize = 1;
        while (b << bits as i32) < a {
            bits += 1;
        }
        bits
    }

    fn update_from_value(&mut self, value: i32) {
        self.0 += value;
        if self.1 == GRADIENT_MIN_VALUE {
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

fn process_line(
    line: &DataGrid<u16>,
    color_map: &DataGrid<Color>,
    gradients: &mut (Gradients, Gradients),
    carry_results: &Colored<Vec<Vec<u16>>>,
) -> Colored<Vec<Vec<u16>>> {
    let mut colors = Colored::new(
        vec![vec![UNSET; 512]; 3],
        vec![vec![UNSET; 512]; 6],
        vec![vec![UNSET; 512]; 3],
    );
    for i in 0..6 {
        let (r, g, b) = colors.split_mut();
        let mut line_colors = Colored::new(&mut r[i / 2], &mut g[i], &mut b[i / 2]);
        assign_into(
            &mut line_colors,
            line.row(i),
            &color_map.subgrid(Position(0, i), Size(6, 1)),
        );
    }

    fill_blanks_in_line(carry_results, &mut colors);

    make_samples_for_line(&mut colors, gradients, carry_results);

    print_color_info(&colors);

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
    } else if v <= 0x12 {
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
) {
    let (even_gradients, _) = gradients;

    // TODO: this is currently only for G2,R2
    // The ordering in which these are output is kinda gross
    // Do the first 4 green values from G2 (even)
    // Then alternate G2 even, R2 odd, G2 odd.
    // Doing R2 odd and G2 odd simultaneously isi important because they both
    // access the same set of gradients
    // Interleaving all three is important because that's the output format 😅
    let row_idx = 0;
    let green_even = repeat(Color::Green).zip((0..512).step_by(2));
    let green_odd = repeat(Color::Green).zip((0..512).skip(1).step_by(2));
    let red_odd = repeat(Color::Red).zip((0..512).skip(1).step_by(2));
    let zipped =
        zip_with_offset(green_even, 0, green_odd.zip_eq(red_odd), 4).map(|(a, b)| (a, flatten(b)));
    for (green_even, (green_odd, red_odd)) in zipped {
        // TODO: implement processing
        if let Some((color, idx)) = green_even {
            let ec = load_even_coefficients(
                // TODO: adapt these for the row_idx
                &carry_results[color][1],
                &carry_results[color][0],
                idx,
            );
            let which_grad_signed = 9 * q_value(ec.north as i32 - ec.very_north as i32)
                + q_value(ec.northwest as i32 - ec.north as i32);
            let which_grad = which_grad_signed.abs() as usize;
            let weighted_average = compute_weighted_average(ec);
            let actual_value = colors[color][row_idx][idx];
            let delta = actual_value as i32 - weighted_average as i32;

            let delta_was_negative = delta < 0;
            let delta = delta.abs() as u16;

            // TODO: even_gradients[0] is based on row, generalize
            let grad = &mut even_gradients[0][which_grad];
            let dec_bits = grad.bit_diff() as u16;

            let split_mask = (1 << dec_bits) - 1;
            // 'sample' in libraw terminology
            let upper = (delta & (!split_mask)) >> dec_bits;
            let lower = delta & split_mask;

            // sometimes the gradient will give us the wrong sign; in which case
            // we can flip the sign again by inverting all the bits. In both
            // cases, use the final bit as a 'sign' bit.
            let code = if (which_grad_signed < 0) == delta_was_negative {
                lower << 1 | 0b0
            } else {
                (!lower << 1) | 0b1
            };

            let old_grad = *grad;

            // finally: update gradient
            grad.update_from_value(lower as i32);

            println!(
                "G2[{}]: sample: {}, code: {}, grad_idx: {}, grad_before: {:?}, grad_after: {:?}",
                idx, upper, code, which_grad, old_grad, grad
            );
        }
        if let Some((_color, _idx)) = green_odd {
            // process green_odd
        }
        if let Some((_color, _idx)) = red_odd {
            // process red_odd
        }
    }
}

fn load_even_coefficients(rprev: &[u16], rprevprev: &[u16], idx: usize) -> EvenCoefficients {
    let last_idx = rprev.len() - 1;
    let leftmost_other = if idx == 0 {
        rprevprev[0]
    } else {
        rprev[idx - 1]
    };
    let rightmost_other = if idx == last_idx {
        rprevprev[rprevprev.len() - 1]
    } else {
        rprev[idx + 1]
    };
    EvenCoefficients {
        north: rprev[idx],
        northwest: leftmost_other,
        northeast: rightmost_other,
        very_north: rprevprev[idx],
    }
}

fn print_color_info(colors: &Colored<Vec<Vec<u16>>>) {
    for (label, (_, color)) in ["R", "G", "B"].iter().zip(colors.iter()) {
        for (i, row) in color.iter().enumerate() {
            println!("{}{}: {:?}", label, i + 2, row);
        }
    }
}

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
