use clap::{App, Arg};

use libraw::{fuji_meta, tiff};

use libraw::tiff::{parse_ifd, parse_tiff_with_options, FieldType, Ifd, IfdEntry, TiffFile};
use memmap::Mmap;

use itertools::Itertools;
use libraw::raf::RafFile;
use libraw::tifflabels::{label_for_tag, TagContext};
use std::convert::TryInto;
use std::error::Error;
use std::fs::File;

fn main() {
    let matches = App::new("TIFF Container Reader")
        .arg(
            Arg::with_name("TIFF CONTAINER FILE")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("tags")
                .long("tag")
                .short("t")
                .multiple(true)
                .takes_value(true),
        )
        .arg(Arg::with_name("Dump All").long("dump-all").short("v"))
        .get_matches();

    let input = matches.value_of("TIFF CONTAINER FILE").unwrap();
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
    let (_, container) = tiff::parse_tiff_with_options(&mmap, b"II*\0", true).unwrap();
    process_tiff_container(TagContext::Exif, tags, print_all_data, &container);
}

fn dump_raf(img_file: &str, tags: &[u16], print_all_data: bool) {
    println!("Treating as RAF File");
    let raf = RafFile::open(img_file).unwrap();
    let offsets = raf.file_parts().unwrap();
    println!("JPEG Part:");
    let (_, exif) = tiff::parse_tiff_with_options(&offsets.jpeg_exif_tiff, b"II*\0", true).unwrap();
    process_tiff_container(TagContext::Exif, tags, print_all_data, &exif);
    println!("-----------");
    println!("Raw Part:");
    let (_, raw_container) = tiff::parse_tiff_with_options(&offsets.raw, b"II*\0", true).unwrap();
    process_tiff_container(TagContext::FujiRaw, tags, print_all_data, &raw_container);
    println!("-----------");

    println!("Focus Info");
    let focus_info = fuji_meta::load_focus_info(&raf).unwrap();
    println!("{:?}", focus_info);
}

fn process_tiff_container(
    context: TagContext,
    tags: &[u16],
    print_all_data: bool,
    tiff_file: &TiffFile,
) {
    for (id, ifd) in tiff_file.ifds.iter().enumerate() {
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

fn process_makernotes<'a>(ifd: &'a Ifd<'_>, file: &TiffFile<'a>) -> Option<TiffFile<'a>> {
    let makernotes = ifd
        .iter()
        .filter(|tag| tag.tag == 0x927C)
        .exactly_one()
        .ok()?;
    let makernotes_content: &[u8] = file.data_for_ifd_entry(makernotes);
    let (_, makernotes_tiff) =
        parse_tiff_with_options(&makernotes_content, b"FUJIFILM", false).unwrap();

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
