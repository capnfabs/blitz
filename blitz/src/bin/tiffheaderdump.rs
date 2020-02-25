use clap::{App, Arg};

use libraw::tiff;

use libraw::tiff::{parse_ifd, FieldType, IfdEntry, TiffFile};
use memmap::Mmap;

use libraw::raf::RafFile;
use std::convert::TryInto;
use std::error::Error;
use std::fs::File;

fn main() {
    let matches = App::new("TIFF Header Dump")
        .arg(Arg::with_name("TIFF FILE").required(true).index(1))
        .arg(
            Arg::with_name("tags")
                .long("tag")
                .short("t")
                .multiple(true)
                .takes_value(true),
        )
        .arg(Arg::with_name("Dump All").long("dump-all").short("v"))
        .get_matches();

    let input = matches.value_of("TIFF FILE").unwrap();
    let dump_tags = matches.values_of("tags");

    let dump_tags = if let Some(tag_iter) = dump_tags {
        tag_iter.map(|x| hexdec(x).unwrap()).collect()
    } else {
        vec![]
    };

    let dump_all_values = matches.is_present("Dump All");

    main_command(input, &dump_tags, dump_all_values);
}

fn hexdec(data: &str) -> Result<u16, Box<dyn Error>> {
    let mut data = data.to_string();
    if data.len() % 2 != 0 {
        data = format!("0{}", data);
    }
    let bytes: &[u8] = &hex::decode(data)?;
    Ok(u16::from_be_bytes(bytes.try_into()?))
}

fn main_command(img_file: &str, tags: &[u16], print_all_data: bool) {
    println!("Opening file: {:?}", img_file);

    if img_file.to_lowercase().ends_with(".raf") {
        println!("Treating as RAF File");
        let raf = RafFile::open(img_file).unwrap();
        let offsets = raf.file_parts().unwrap();
        println!("JPEG Part:");
        dump_tiff_details(tags, print_all_data, offsets.jpeg_exif_tiff);
        println!("-----------");
        println!("Raw Part:");
        dump_tiff_details(tags, print_all_data, offsets.raw);
        println!("-----------");
    } else {
        let file = File::open(img_file).unwrap();
        let mmap = unsafe { Mmap::map(&file) }.unwrap();
        dump_tiff_details(tags, print_all_data, &mmap);
    }
}

fn dump_tiff_details(tags: &[u16], print_all_data: bool, data: &[u8]) -> () {
    let (_, file) = tiff::parse_tiff(&data).unwrap();
    for (id, ifd) in file.ifds.iter().enumerate() {
        println!("IFD #{}", id);
        dump_entries(tags, &file, &ifd, print_all_data);
        println!("-----------")
    }
    for ifd in &file.ifds {
        for entry in ifd.iter().filter(|tag| {
            tag.tag == 0x14A || tag.tag == 0xF000 || tag.tag == 0x8769 || tag.tag == 0xA005
        }) {
            assert_eq!(entry.count, 1);
            let offset = entry.val_u32().unwrap() as usize;
            let subifd = &data[offset..];
            let (_, (parsed, _)) = parse_ifd(subifd).unwrap();
            println!("!!SubIFD from tag {:X}!!", entry.tag);
            dump_entries(tags, &file, &parsed, print_all_data);
            println!("--!!SubIFD from tag {:X}!!--", entry.tag);
        }
    }
}

fn dump_entries(tags: &[u16], file: &TiffFile, parsed: &Vec<IfdEntry>, dump_all_data: bool) -> () {
    for entry in parsed {
        if tags.len() == 0 || tags.contains(&entry.tag) {
            let inline_val =
                if entry.count < 4 || (entry.field_type == FieldType::Ascii && entry.count < 80) {
                    Some(file.debug_value_for_ifd_entry(entry))
                } else {
                    None
                };

            let offset = entry.val_as_offset();

            print!(
                "Tag: {:X}, Type: {:?}, Count: {}",
                entry.tag, entry.field_type, entry.count
            );

            if let Some(offset) = offset {
                print!(", Offset: {}", offset);
            }

            if let Some(val) = inline_val {
                println!(", Val: {}", val);
            } else if dump_all_data {
                println!(", Val:\n{}\n---", file.debug_value_for_ifd_entry(&entry))
            } else {
                // Just print the terminator
                println!()
            }
        }
    }
}
