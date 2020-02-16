use clap::{App, Arg};
use itertools::Itertools;
use libraw::fuji_compressed;
use libraw::RawFile;

use libraw::util::datagrid::{DataGrid, Size};
use std::io::Cursor;

fn main() {
    let matches = App::new("Squashraf")
        .arg(Arg::with_name("INPUT").required(true).index(1))
        .get_matches();

    let input = matches.value_of("INPUT").unwrap();

    squash_raf(input);
}

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

    // This is where we're going to write the output to.
    let mut data: Cursor<Vec<u8>> = Cursor::new(Vec::new());
    fuji_compressed::compress(img_grid, &cm, &mut data);
}
