// Still WIP
#![allow(unused_variables)]

use clap::{App, Arg};
use itertools::Itertools;
use libraw::util::{Grid, Position, Size};
use libraw::{util, Color, RawFile};

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

fn assign_into<'a, G: Grid<'a, Color>>(
    reds: &mut [u16],
    greens: &mut [u16],
    blues: &mut [u16],
    row: &[u16],
    row_map: G,
) {
    for x in vec![&reds, &greens, &blues].iter() {
        assert_eq!(x.len(), REQUIRED_CAPACITY);
    }
    assert_eq!(row.len(), STRIPE_WIDTH);

    for (pos, val) in row.iter().enumerate() {
        // produces the sequence 0,1,1,2,3,3,4,5,5...
        let squashed_idx = (pos - 1) * 2 / 3 + 1;
        match row_map.at(Position(pos, 0)) {
            Color::Red => reds[squashed_idx] = *val,
            Color::Green => greens[squashed_idx] = *val,
            Color::Blue => blues[squashed_idx] = *val,
        }
    }
}

// Ok, the idea here is:
// it's a weighted average of the values around it.
// - take the value immediately 'above' this one. Call this rb.
// - of the other three values:
//   - choose the two that are closest to rb
//   - call these two values `close`
// - now, compute close[0] + close[1] + 2*rb / 4.
fn choose_fill_val(rb: u16, others: &[u16; 3]) -> u16 {
    let others = others
        .iter()
        .copied()
        .sorted_by_key(|v| (*v as i32 - rb as i32).abs())
        .collect_vec();
    (others[0] + others[1] + 2 * rb) / 4
}

fn fill_blanks(row: &mut [u16], rprev: &[u16], rprevprev: &[u16]) {
    for (idx, x) in row.iter_mut().enumerate() {
        if *x == UNSET {
            *x = choose_fill_val(
                rprev[idx],
                &[rprev[idx - 1], rprev[idx + 1], rprevprev[idx]],
            )
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

    let img_grid = util::wrap(
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
    let cm = util::wrap(&xtmap, Size(6, 6));
    let strip = util::subgrid(
        &img_grid,
        Position(0, 0),
        Size(STRIPE_WIDTH, file.img_params().raw_height as usize),
    );
    let line_no: usize = 6;
    // grab the greens into g2

    let r0: Vec<u16> = vec![UNSET; 512];
    let r1: Vec<u16> = vec![UNSET; 512];
    let b0: Vec<u16> = vec![UNSET; 512];
    let b1: Vec<u16> = vec![UNSET; 512];
    let mut g2: Vec<u16> = vec![UNSET; 512];
    let mut g3: Vec<u16> = vec![UNSET; 512];
    let mut r2: Vec<u16> = vec![UNSET; 512];
    let mut b2: Vec<u16> = vec![UNSET; 512];

    assign_into(
        &mut r2,
        &mut g2,
        &mut b2,
        strip.row(0),
        util::subgrid(&cm, Position(0, 0), Size(6, 1)),
    );
    assign_into(
        &mut r2,
        &mut g3,
        &mut b2,
        strip.row(1),
        util::subgrid(&cm, Position(0, 1), Size(6, 1)),
    );

    fill_blanks(&mut r2, &r1, &r0);
    fill_blanks(&mut b2, &b1, &b0);

    //let r2: Vec<>
    println!("G2: {:#?}", g2);
    println!("R2: {:#?}", r2);
    println!("B2: {:#?}", b2);
}
