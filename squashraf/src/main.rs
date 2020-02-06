// Still WIP
#![allow(unused_variables)]

use clap::{App, Arg};
use libraw::util::Size;
use libraw::Color::Green;
use libraw::{util, RawFile};

fn main() {
    let matches = App::new("Squashraf")
        .arg(Arg::with_name("INPUT").required(true).index(1))
        .get_matches();

    let input = matches.value_of("INPUT").unwrap();

    squash_raf(input);
}

const STRIPE_WIDTH: usize = 768;
//const LINE_HEIGHT: usize = 6;

fn squash_raf(img_file: &str) {
    println!("Loading RAW data: libraw");
    let file = RawFile::open(img_file).unwrap();
    println!("Opened file: {:?}", file);

    let data = util::wrap(
        file.raw_data(),
        Size(
            file.img_params().raw_width as usize,
            file.img_params().raw_height as usize,
        ),
    );
    let cm = file.xtrans_pixel_mapping();
    let line_no: usize = 6;
    // grab the greens into g2
    let g2: Vec<u16> = data.row(line_no)[..STRIPE_WIDTH]
        .iter()
        .enumerate()
        .filter_map(|(col, val)| {
            if cm[line_no % 6][(col) as usize % 6] == Green {
                Some(*val)
            } else {
                None
            }
        })
        .collect();
    //let r2: Vec<>
    println!("G2: {:#?}", g2);
}
