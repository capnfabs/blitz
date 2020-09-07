use clap::{App, Arg};

use libraw::tiff;

use libraw::tiff::{parse_ifd, FieldType, IfdEntry, TiffFile};
use memmap::Mmap;

use libraw::raf::RafFile;
use libraw::tifflabels::{label_for_tag, TagPath};
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
    if img_file.to_lowercase().ends_with(".raf") {
        println!("Treating as RAF File");
        let raf = RafFile::open(img_file).unwrap();
        let offsets = raf.file_parts().unwrap();
        println!("JPEG Part:");
        dump_tiff_details(
            TagPath::PreviewExif,
            tags,
            print_all_data,
            offsets.jpeg_exif_tiff,
        );
        println!("-----------");
        println!("Raw Part:");
        dump_tiff_details(TagPath::Raw, tags, print_all_data, offsets.raw);
        println!("-----------");
    } else {
        // Should be treated as a DNG probably
        let file = File::open(img_file).unwrap();
        let mmap = unsafe { Mmap::map(&file) }.unwrap();
        // TODO: choice of TagPath here is wrong; fix it.
        dump_tiff_details(TagPath::PreviewExif, tags, print_all_data, &mmap);
    }
}

fn dump_tiff_details(context: TagPath, tags: &[u16], print_all_data: bool, data: &[u8]) {
    let (_, file) = tiff::parse_tiff(&data).unwrap();
    for (id, ifd) in file.ifds.iter().enumerate() {
        println!("IFD #{}", id);
        dump_entries(context, tags, &file, &ifd, print_all_data);
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
            let subcontext = if entry.tag == 0x8769 {
                TagPath::PreviewExif
            } else {
                context
            };
            dump_entries(subcontext, tags, &file, &parsed, print_all_data);
        }
    }
}

fn dump_entries(
    context: TagPath,
    tags: &[u16],
    file: &TiffFile,
    parsed: &[IfdEntry],
    dump_all_data: bool,
) {
    for entry in parsed {
        if tags.is_empty() || tags.contains(&entry.tag) {
            let inline_val =
                if entry.count < 4 || (entry.field_type == FieldType::Ascii && entry.count < 80) {
                    Some(file.debug_value_for_ifd_entry(entry))
                } else {
                    None
                };

            let offset = entry.val_as_offset();

            let tag_label: String = label_for_tag(context, entry.tag)
                .unwrap_or("[??]")
                .chars()
                .take(30)
                .collect();

            print!(
                "{:<30} Tag: {:X}, Type: {:?}, Count: {}",
                &tag_label, entry.tag, entry.field_type, entry.count
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
