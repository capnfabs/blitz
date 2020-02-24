use clap::{App, Arg};

use libraw::tiff;

use memmap::Mmap;
use std::fs::File;

fn main() {
    let matches = App::new("TIFF Header Dump")
        .arg(Arg::with_name("TIFF FILE").required(true).index(1))
        .arg(Arg::with_name("data").long("data"))
        .get_matches();

    let input = matches.value_of("TIFF FILE").unwrap();
    let dump_data_less_than = matches
        .value_of("data")
        .map(|x| x.parse::<usize>().unwrap());

    dump_tiff(input, dump_data_less_than.unwrap_or(0));
}

fn dump_tiff(img_file: &str, dump_data_less_than: usize) {
    println!("Loading RAW data");
    let file = File::open(img_file).unwrap();
    let mmap = unsafe { Mmap::map(&file) }.unwrap();

    let (_, file) = tiff::parse_tiff(&mmap).unwrap();
    println!("Opened file: {:?}", img_file);

    for ifd in file.ifds {
        for entry in ifd {
            println!(
                "Tag: {:X}, Type: {:?}, Elements: {}",
                entry.tag, entry.field_type, entry.count
            );
            if dump_data_less_than >= entry.count as usize {
                //println!("Values: {:#?}", entry.value_debug())
            }
        }
    }
}
