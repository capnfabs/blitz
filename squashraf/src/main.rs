// Still WIP

use clap::{App, Arg};
use itertools::Itertools;
use libraw::util::{DataGrid, Position, Size};
use libraw::{Color, RawFile};

mod colored;

use crate::colored::Colored;

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
fn choose_fill_val(north: u16, north_west: u16, north_east: u16, very_north: u16) -> u16 {
    let distance = |v: u16| (v as i32 - north as i32).abs();
    let others = [north_west, north_east, very_north]
        .iter()
        .copied()
        .sorted_by_key(|x| distance(*x))
        .collect_vec();

    if distance(north_west) == distance(north_east) {
        let val = (north_west + north_east + 2 * north) / 4;
        return val;
    }

    let val = (others[0] + others[1] + 2 * north) / 4;
    if val == 1810 {
        println!(
            "val {}, rc {} rb {} rd {} rf {}",
            val, north_west, north, north_east, very_north
        );
    }
    val
}

fn fill_blanks_in_row(row: &mut [u16], rprev: &[u16], rprevprev: &[u16]) {
    let last_idx = row.len() - 1;
    for (idx, x) in row.iter_mut().enumerate() {
        if *x == UNSET {
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
            *x = choose_fill_val(rprev[idx], leftmost_other, rightmost_other, rprevprev[idx])
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
    // TODO: calculate the num_lines and use the legit value.
    // each 'line' is a block of 6x768
    let num_lines = 3;
    for line in 0..num_lines {
        let line = stripe.subgrid(Position(0, 6 * line), Size(STRIPE_WIDTH, 6));
        let results = process_line(&line, &color_map, &prev_lines);
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

fn process_line(
    line: &DataGrid<u16>,
    color_map: &DataGrid<Color>,
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

    print_color_info(&colors);

    colors
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
