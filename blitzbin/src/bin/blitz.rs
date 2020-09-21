use clap::{App, Arg, ArgMatches};
use image::imageops::FilterType::Lanczos3;
use image::{imageops, DynamicImage, ImageFormat};

use blitz::diagnostics::histogram::ToHistogram;

use blitz::render;
use blitzbin::diagnostics::TermImage;
use blitzbin::pathutils;
use libraw::raf::RafFile;

struct Flags {
    open: bool,
    stats: bool,
}

fn main() {
    let matches = App::new("Blitz")
        .arg(Arg::with_name("open").long("open"))
        .arg(Arg::with_name("stats").long("stats"))
        .arg(Arg::with_name("INPUT").required(true).index(1))
        .get_matches();

    let input = matches.value_of("INPUT").unwrap();
    let flags = make_flags(&matches);

    load_and_maybe_render(input, &flags);
}

fn make_flags(matches: &ArgMatches) -> Flags {
    let open = matches.occurrences_of("open") == 1;
    let stats = matches.occurrences_of("stats") == 1;
    Flags { open, stats }
}

fn load_and_maybe_render(img_file: &str, flags: &Flags) {
    println!("Loading RAW data: native");
    let file = RafFile::open(img_file).unwrap();
    println!(
        "Opened file: {}",
        file.path().file_name().and_then(|x| x.to_str()).unwrap()
    );
    println!("Parsing...");
    let details = file.parse_raw().unwrap();
    println!("Parsed.");

    let raw_preview_filename = pathutils::get_output_path("native");
    let rendered = render::render_raw(&details);
    if flags.stats {
        println!("Stats");
        let img = rendered.histogram().to_img(256, 128);
        DynamicImage::ImageRgba8(img).display();
    }
    println!("Saving");
    rendered
        .save_with_format(&raw_preview_filename, ImageFormat::Tiff)
        .unwrap();
    pathutils::set_readonly(&raw_preview_filename);
    println!("Done saving");
    if flags.open {
        pathutils::open_preview(&raw_preview_filename);
    } else {
        println!("Resizing...");
        let img = imageops::resize(&rendered, 563, 375, Lanczos3);
        println!("Displaying...");
        DynamicImage::ImageRgb8(img).display();
        println!("Saved to {}", raw_preview_filename.to_str().unwrap());
    }
}
