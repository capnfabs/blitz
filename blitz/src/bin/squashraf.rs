use clap::{App, Arg};
use libraw::fuji_compressed;

use libraw::raf::RafFile;
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
    let file = RafFile::open(img_file).unwrap();
    let file = file.parse_raw().unwrap();
    let render_info = file.render_info();
    println!("Opened file: {:?}", file);

    let img_grid = DataGrid::wrap(
        render_info.raw_data,
        Size(render_info.width as usize, render_info.height as usize),
    );
    let cm = DataGrid::wrap(&render_info.xtrans_mapping, Size(6, 6));

    // This is where we're going to write the output to.
    let mut data: Cursor<Vec<u8>> = Cursor::new(Vec::new());
    fuji_compressed::compress(img_grid, &cm, &mut data);
}
