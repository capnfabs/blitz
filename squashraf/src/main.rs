// Still WIP
#![allow(unused_variables)]

use clap::{App, Arg};
use itertools::Itertools;
use libraw::util::{DataGrid, Position, Size};
use libraw::{Color, RawFile};

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

fn assign_into(
    reds: &mut [u16],
    greens: &mut [u16],
    blues: &mut [u16],
    row: &[u16],
    row_map: DataGrid<Color>,
) {
    for x in vec![&reds, &greens, &blues].iter() {
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
        match color {
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

fn fill_blanks(row: &mut [u16], rprev: &[u16], rprevprev: &[u16]) {
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
    let strip = DataGrid::subgrid(
        &img_grid,
        Position(0, 0),
        Size(STRIPE_WIDTH, file.img_params().raw_height as usize),
    );
    // TODO: calculate this!
    // each 'line' is a block of 6x768
    let num_lines = 673;
    let zeros = LOTS_OF_ZEROS.to_vec();
    let mut prev_lines = vec![
        vec![zeros.clone(), zeros.clone()],
        vec![zeros.clone(), zeros.clone()],
        vec![zeros.clone(), zeros.clone()],
    ];
    let num_lines = 3;
    for line in 0..num_lines {
        let line = DataGrid::subgrid(&strip, Position(0, 6 * line), Size(STRIPE_WIDTH, 6));
        println!("datagrid: {:#?}", line);
        let mut reds = vec![vec![UNSET; 512]; 3];
        let mut greens = vec![vec![UNSET; 512]; 6];
        let mut blues = vec![vec![UNSET; 512]; 3];
        for i in 0..6 {
            assign_into(
                &mut reds[i / 2],
                &mut greens[i],
                &mut blues[i / 2],
                line.row(i),
                DataGrid::subgrid(&cm, Position(0, i), Size(6, 1)),
            );
        }

        // blank filling code
        let mut colors = vec![&mut reds, &mut greens, &mut blues];
        for color_idx in 0..3 {
            let prev_lines = &mut prev_lines[color_idx];
            let color = &mut colors[color_idx];
            for idx in 0..color.len() {
                let (history, future) = color.split_at_mut(idx);
                let (rprevprev, rprev) = match history.len() {
                    0 => (prev_lines[0].as_slice(), prev_lines[1].as_slice()),
                    1 => (prev_lines[1].as_slice(), history[0].as_slice()),
                    idx => (history[idx - 2].as_slice(), history[idx - 1].as_slice()),
                };
                fill_blanks(future[0].as_mut_slice(), rprev, rprevprev)
            }
        }

        for (label, color) in ["R", "G", "B"].iter().zip(colors) {
            for (i, row) in color.iter().enumerate() {
                println!("{}{}: {:?}", label, i + 2, row);
            }
        }

        // modify prev_lines.
        prev_lines[0][0] = reds[reds.len() - 2].clone();
        prev_lines[0][1] = reds[reds.len() - 1].clone();
        prev_lines[1][0] = greens[greens.len() - 2].clone();
        prev_lines[1][1] = greens[greens.len() - 1].clone();
        prev_lines[2][0] = blues[blues.len() - 2].clone();
        prev_lines[2][1] = blues[blues.len() - 1].clone();
    }
}
