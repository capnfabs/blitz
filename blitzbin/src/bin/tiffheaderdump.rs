#[macro_use]
extern crate lazy_static;

use clap::{App, Arg};

use libraw::tiff;

use libraw::tiff::{parse_ifd, parse_tiff_flex_prefix, FieldType, Ifd, IfdEntry, TiffFile};
use memmap::Mmap;

use itertools::Itertools;
use libraw::raf::RafFile;
use libraw::tifflabels::{label_for_tag, TagContext};
use std::collections::{HashMap, HashSet};
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

// Hexadecimal -> u16.
// I think this is way more complex than it needs to be? But I haven't bothered
// rewriting it because it's contained grossness.
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
        dump_raf(img_file, tags, print_all_data);
    } else {
        // Probably doesn't belong in an 'else' block but I was only ever using
        // this for DNGs and RAFs, so 🤷🏻‍♂️
        dump_dng(img_file, tags, print_all_data);
    }
}

fn dump_dng(img_file: &str, tags: &[u16], print_all_data: bool) {
    let file = File::open(img_file).unwrap();
    let mmap = unsafe { Mmap::map(&file) }.unwrap();
    // TODO: choice of TagPath here is wrong; fix it once we support DNGs in the
    //  labelling.
    let (_, container) = tiff::parse_tiff(&mmap).unwrap();
    process_tiff_container(TagContext::Exif, tags, print_all_data, &container);
}

fn dump_raf(img_file: &str, tags: &[u16], print_all_data: bool) {
    println!("Treating as RAF File");
    let raf = RafFile::open(img_file).unwrap();
    let offsets = raf.file_parts().unwrap();
    println!("JPEG Part:");
    let (_, exif) = tiff::parse_tiff(&offsets.jpeg_exif_tiff).unwrap();
    process_tiff_container(TagContext::Exif, tags, print_all_data, &exif);
    println!("-----------");
    println!("Raw Part:");
    let (_, raw_container) = tiff::parse_tiff(&offsets.raw).unwrap();
    process_tiff_container(TagContext::FujiRaw, tags, print_all_data, &raw_container);
    println!("-----------");
}

fn process_tiff_container(
    context: TagContext,
    tags: &[u16],
    print_all_data: bool,
    tiff_file: &TiffFile,
) {
    // println!("Process Tiff Container");
    // find all nested IFDs as well
    let nested_ifds = tiff_file
        .ifds
        .iter()
        .map(|ifd| find_nested_ifds(ifd, tiff_file.data))
        .fold(vec![], |mut a, mut b| {
            a.append(&mut b);
            a
        });
    // TODO: improve labelling on these IFDs, especially for the nested ones.
    for (id, ifd) in tiff_file.ifds.iter().chain(nested_ifds.iter()).enumerate() {
        println!("IFD #{}", id);
        format_and_print_ifd(context, tags, &tiff_file, &ifd, print_all_data);
        println!("-----------");
        if let Some(makernotes) = process_makernotes(ifd, &tiff_file) {
            println!("--Makernotes!!--");
            process_tiff_container(
                TagContext::ExifMakerNotes,
                tags,
                print_all_data,
                &makernotes,
            );
            println!("--/Makernotes!!--");
        }
    }
}

lazy_static! {
    static ref NESTED_IFD_TAGS: HashSet<u16> =
        [
            0x014A,  // ?? Probably DNG-ish
            0xF000, // Fuji RAW Section Pointer
            0xA005, // Interoperability IFD Pointer
            0x8769, // EXIF IFD Pointer
        ].iter().cloned().collect();
}

// Looks for nested IFDs recursively using a predefined selection of tags.
fn find_nested_ifds<'a>(ifd: &Ifd<'_>, file_data: &'a [u8]) -> Vec<Ifd<'a>> {
    //println!("Find Nested IFDs");
    let mut nested_ifds = vec![];

    for entry in ifd.iter().filter(|e| NESTED_IFD_TAGS.contains(&e.tag)) {
        assert_eq!(entry.count, 1);
        let offset = entry.val_u32().unwrap() as usize;
        let subifd = &file_data[offset..];
        let (_, (parsed, _)) = parse_ifd(subifd).unwrap();

        // Have to do this before the push because the push is a move
        let mut recursed = find_nested_ifds(&parsed, file_data);
        nested_ifds.push(parsed);
        nested_ifds.append(&mut recursed);
    }

    nested_ifds
}

fn process_makernotes<'a>(ifd: &'a Ifd<'_>, file: &TiffFile<'a>) -> Option<TiffFile<'a>> {
    let makernotes = ifd
        .iter()
        .filter(|tag| tag.tag == 0x927C)
        .exactly_one()
        .ok()?;
    // println!("Makernote {:X?}", makernotes);
    let makernotes_content: &[u8] = file.data_for_ifd_entry(makernotes);
    let (_, makernotes_tiff) = parse_tiff_flex_prefix(b"FUJIFILM", &makernotes_content).unwrap();

    Some(makernotes_tiff)
}

fn format_and_print_ifd(
    context: TagContext,
    tags: &[u16],
    file: &TiffFile,
    parsed: &[IfdEntry],
    dump_all_data: bool,
) {
    let print_all = tags.is_empty();
    for entry in parsed
        .iter()
        .filter(|entry| print_all || tags.contains(&entry.tag))
    {
        let inline_display_value =
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
            "{:<30} Tag: {:04X}, Type: {:?}, Count: {}",
            &tag_label, entry.tag, entry.field_type, entry.count
        );

        if let Some(offset) = offset {
            print!(", Offset: {}", offset);
        }

        if let Some(val) = inline_display_value {
            println!(", Val: {}", val);
        } else if dump_all_data {
            println!(", Val:\n{}\n---", file.debug_value_for_ifd_entry(&entry))
        } else {
            // Just print the terminator
            println!()
        }
    }
}
