// Still WIP

use clap::{App, Arg};
use itertools::Itertools;
use libraw::util::{DataGrid, Position, Size};
use libraw::{Color, RawFile};

mod colored;
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
    let num_lines = 2;
    for line in 0..num_lines {
        let line = stripe.subgrid(Position(0, 6 * line), Size(STRIPE_WIDTH, 6));
        let results = process_line(&line, &color_map, &mut gradients, &prev_lines);
        dump_colors(&results);
        prev_lines = collect_carry_lines(results);
        //dump_colors(&prev_lines);
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
) {
    // TODO: this is currently only for G2,R2
    // The ordering in which these are output is kinda gross
    // Do the first 4 green values from G2 (even)
    // Then alternate G2 even, R2 odd, G2 odd.
    // Doing R2 odd and G2 odd simultaneously isi important because they both
    // access the same set of gradients
    // Interleaving all three is important because that's the output format 😅
    let PROCESS = [
        ((Color::Red, 0), (Color::Green, 0), 0),
        ((Color::Green, 1), (Color::Blue, 0), 1),
        ((Color::Red, 1), (Color::Green, 2), 2),
        ((Color::Green, 3), (Color::Blue, 1), 0),
        ((Color::Red, 2), (Color::Green, 4), 1),
        ((Color::Green, 5), (Color::Blue, 2), 2),
    ];

    let data: Cursor<Vec<u8>> = Cursor::new(Vec::new());
    let mut output = bitbit::BitWriter::new(data);

    for (color_a, color_b, grad_set_idx) in &PROCESS {
        let ca_even = repeat(color_a).zip((0..512).step_by(2));
        let ca_odd = repeat(color_a).zip((0..512).skip(1).step_by(2));
        let cb_even = repeat(color_b).zip((0..512).step_by(2));
        let cb_odd = repeat(color_b).zip((0..512).skip(1).step_by(2));
        let zipped = zip_with_offset(ca_even.zip_eq(cb_even), 0, ca_odd.zip_eq(cb_odd), 4)
            .map(|(a, b)| (flatten(a), flatten(b)));

        // TODO need to skip generated vals somehow.
        for ((ca_even, cb_even), (ca_odd, cb_odd)) in zipped {
            for thing in vec![ca_even, cb_even, ca_odd, cb_odd] {
                if let Some(((color, row), idx)) = thing {
                    if !skip(*color, *row, idx) {
                        let (sample, code) = make_sample(
                            &colors,
                            gradients,
                            &carry_results,
                            *row,
                            *color,
                            idx,
                            *grad_set_idx,
                        );
                        for _ in 0..sample {
                            output.write_bit(true).unwrap();
                        }
                        output.write_bit(false).unwrap();
                        //output.write_bits(code)
                    }
                }
            }
        }
    }
}

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
    Absolute(u16),
    Relative {
        upper: u16,
        lower: u16,
        lower_bits: usize,
    },
}

fn make_sample(
    colors: &Colored<Vec<Vec<u16>>>,
    gradients: &mut ([[Grad; 41]; 3], [[Grad; 41]; 3]),
    carry_results: &Colored<Vec<Vec<u16>>>,
    row_idx: usize,
    color: Color,
    idx: usize,
    grad_set: usize,
) -> (u16, u16) {
    let is_even = idx % 2 == 0;
    let carry_results = &carry_results[color];
    let cdata = &colors[color];
    let (even_gradients, odd_gradients) = gradients;
    let (rprevprev, rprev) = match row_idx {
        0 => (carry_results[0].as_slice(), carry_results[1].as_slice()),
        1 => (carry_results[1].as_slice(), cdata[0].as_slice()),
        row => (cdata[row - 2].as_slice(), cdata[row - 1].as_slice()),
    };
    let (grad_set, (weighted_average, which_grad)) = if is_even {
        (
            &mut even_gradients[grad_set],
            grad_and_weighted_avg_even(idx, rprevprev, rprev),
        )
    } else {
        (
            &mut odd_gradients[grad_set],
            grad_and_weighted_avg_odd(idx, rprevprev, rprev),
        )
    };
    let grad = &mut grad_set[which_grad.abs() as usize];
    let actual_value = cdata[row_idx][idx];
    let grad_is_negative = which_grad < 0;
    unsafe { DUMP = is_even };
    let mut sample = compute_sample(weighted_average, actual_value, grad);
    let delta_is_negative = actual_value < weighted_average;

    let old_grad = *grad;
    // TODO: refactor this + delta_is_negative into one thing.
    // Finally: update gradient. This updates based on the absolute value of the delta.
    grad.update_from_value((actual_value as i32 - weighted_average as i32).abs());

    // FINALLY ENCODE SOME SHIT
    // sometimes the gradient will give us the wrong sign; in which case
    // we can flip the sign again by inverting all the bits. In both
    // cases, use the final bit as a 'sign' bit.
    // The 'code' distinction here isn't helpful, at all.
    // Should treat these as separate values.

    let sample = match sample {
        Sample::Relative {
            upper,
            lower: 0,
            lower_bits,
        } if upper > 0 && delta_is_negative != grad_is_negative => {
            // This amazing hack depends upon the subtraction of 1, below.
            Sample::Relative {
                upper: upper - 1,
                lower: 1 << lower_bits as u16,
                lower_bits,
            }
        }
        _ => sample,
    };

    let (s, c) = match sample {
        Sample::Absolute(val) => (41, (val - 1) << 1 | 0b1),
        Sample::Relative {
            upper,
            lower,
            lower_bits,
        } => {
            let c = if delta_is_negative != grad_is_negative && lower > 0 {
                (lower - 1) << 1 | 0b1
            } else {
                lower << 1
            };
            (upper, c)
        }
    };

    if idx % 2 == 0 {
        println!(
            "{}{}[{}]: ref: {}, actual: {}, sample: {}, code: {}, grad_before: {:?}, grad_after: {:?}",
            c4(color),
            row_idx,
            idx,
            weighted_average,
            actual_value,
            s,
            c,
            old_grad,
            grad
        );
    }

    (s, c)
}

fn c4(color: Color) -> &'static str {
    match color {
        Color::Red => "R",
        Color::Green => "G",
        Color::Blue => "B",
    }
}

static mut DUMP: bool = false;

fn compute_sample(weighted_average: u16, actual_value: u16, grad: &Grad) -> Sample {
    let delta = actual_value as i32 - weighted_average as i32;
    let delta = delta.abs() as u16;
    let orig_dec_bits = grad.bit_diff() as u16;
    let dec_bits = orig_dec_bits.saturating_sub(1);
    let split_mask = (1 << dec_bits) - 1;
    // 'sample' in libraw terminology
    let upper = (delta & (!split_mask)) >> dec_bits;
    let lower = delta & split_mask;
    if upper > 40 {
        if unsafe { DUMP } {
            println!("dec bits encode direct");
        }
        Sample::Absolute(actual_value)
    } else {
        if unsafe { DUMP } {
            println!("dec bits {}", orig_dec_bits,);
        }
        Sample::Relative {
            upper,
            lower,
            lower_bits: dec_bits as usize,
        }
    }
}

fn grad_and_weighted_avg_even(idx: usize, rprevprev: &[u16], rprev: &[u16]) -> (u16, i32) {
    let ec = load_even_coefficients(rprev, rprevprev, idx);
    let weighted_average = compute_weighted_average(ec);
    let which_grad = 9 * q_value(ec.north as i32 - ec.very_north as i32)
        + q_value(ec.northwest as i32 - ec.north as i32);
    //println!("even which grad {}, ec = {:?}", which_grad, ec);
    (weighted_average, which_grad)
}

fn grad_and_weighted_avg_odd(idx: usize, rprevprev: &[u16], rprev: &[u16]) -> (u16, i32) {
    (0, 0)
    /*let oc = load_odd_coefficients(rprev, rprevprev, idx);
    let weighted_average = compute_weighted_average(oc);
    let which_grad = 9 * q_value(ec.north as i32 - ec.very_north as i32)
        + q_value(ec.northwest as i32 - ec.north as i32);
    (weighted_average, which_grad)*/
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

#[allow(unused)]
fn dump_colors(colors: &Colored<Vec<Vec<u16>>>) {
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
