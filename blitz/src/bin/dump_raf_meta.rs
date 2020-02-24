use clap::{App, Arg};

use itertools::Itertools;
use libraw::raf::{RafFile, Tag};
use std::error::Error;
use std::fs;
use std::fs::{read_dir, File};
use std::io::Write;
use std::path::PathBuf;

fn main() {
    let matches = App::new("Dump RAF Metadata Section")
        .arg(Arg::with_name("RAF directory").required(true).index(1))
        .arg(Arg::with_name("Output directory").required(true).index(2))
        .get_matches();

    let input = matches.value_of("RAF directory").unwrap();
    let output = matches.value_of("Output directory").unwrap();

    enumerate_vals(input, output).unwrap();
}

fn enumerate_vals(img_location: &str, output_directory: &str) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(output_directory).unwrap();
    for entry in read_dir(img_location)? {
        let entry = entry?;
        if !entry
            .file_name()
            .to_str()
            .unwrap()
            .to_lowercase()
            .ends_with(".raf")
        {
            continue;
        }
        let raf = RafFile::open(entry.path())?;
        let raf_meta = raf.parse_meta()?;
        let tag = raf_meta
            .iter()
            .filter_map(|tag| {
                if let Tag::RAFData(data) = tag {
                    Some(data)
                } else {
                    None
                }
            })
            .exactly_one()
            .unwrap();
        let mut img_name = {
            let mut filename = entry.file_name().to_str().unwrap().to_string();
            filename.truncate(filename.len() - 4);
            filename
        };
        img_name.push_str("-rafdata.bin");
        let path: PathBuf = [output_directory, &img_name].iter().collect();
        let mut file = File::create(&path).unwrap();
        file.write_all(tag).unwrap();
        println!(
            "Done processing {}, result saved in in {}",
            img_name,
            path.to_str().unwrap()
        );
        //println!("{}: {:?}", entry.file_name().to_str().unwrap(), tag);
    }
    Ok(())
}
