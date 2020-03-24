use blitz::common::Pixel;
use blitz::demosaic::{Demosaic, Nearest};
use blitz::diagnostics::TermImage;
use blitz::{diagnostics, histo, levels, pathutils, vignette_correction};
use clap::{App, Arg, ArgMatches};
use histogram::Histogram;
use image::{ImageBuffer, ImageFormat};
use itertools::Itertools;
use libraw::griditer::GridIterator;
use libraw::raf::{ParsedRafFile, RafFile};
extern crate nalgebra as na;
use blitz::camera_specific_junk::{cam_xyz, dng_cam1_to_xyz, dng_cam2_to_xyz};
use blitz::levels::cam_to_srgb;
use ndarray::prelude::*;
use ndarray::Array2;
use ordered_float::NotNan;

struct Flags {
    render: bool,
    open: bool,
    stats: bool,
}

fn main() {
    let matches = App::new("Blitz")
        .arg(Arg::with_name("render").short("r").long("render"))
        .arg(Arg::with_name("open").long("open"))
        .arg(Arg::with_name("INPUT").required(true).index(1))
        .get_matches();

    let input = matches.value_of("INPUT").unwrap();
    let flags = make_flags(&matches);

    load_and_maybe_render(input, &flags);
}

fn make_flags(matches: &ArgMatches) -> Flags {
    let render = matches.occurrences_of("render") == 1;
    let open = matches.occurrences_of("open") == 1;
    let stats = matches.occurrences_of("stats") == 1;
    Flags {
        render,
        open,
        stats,
    }
}

fn load_and_maybe_render(img_file: &str, flags: &Flags) {
    println!("Loading RAW data: native");
    let file = RafFile::open(img_file).unwrap();
    println!("Opened file: {:?}", file);
    println!("Parsing...");
    let details = file.parse_raw().unwrap();

    println!("Parsed.");

    if !flags.render {
        return;
    }

    let raw_preview_filename = pathutils::get_output_path("native");
    let rendered = render_raw(&details, flags.stats);
    println!("Saving");
    rendered
        .save_with_format(&raw_preview_filename, ImageFormat::Tiff)
        .unwrap();
    pathutils::set_readonly(&raw_preview_filename);
    println!("Done saving");
    if flags.open {
        pathutils::open_preview(&raw_preview_filename);
    }
}

fn make_histogram<T, U>(iter: T) -> Histogram
where
    T: std::iter::Iterator<Item = U>,
    U: Into<u64>,
{
    let mut h = histogram::Histogram::new();
    for v in iter {
        h.increment(v.into()).unwrap();
    }
    h
}

fn print_stats(value_iter: impl Iterator<Item = u16> + Clone) {
    println!("Percentile chart!");
    // Percentile chart
    let values_curve = make_histogram(value_iter.clone());
    diagnostics::render_tone_curve(&values_curve, 600, 400).display();

    println!();
    println!("Histogram!");
    let h = histo::Histo::from_iter(value_iter);
    diagnostics::render_histogram(&h, 600, 1000).display();
    println!();
}

fn render_raw(img: &ParsedRafFile, output_stats: bool) -> image::RgbImage {
    let raf = img;
    let img = &img.render_info();

    let mapping = Array2::from_shape_vec((6, 6).set_f(true), img.xtrans_mapping.clone()).unwrap();

    let img_data = img.raw_data.clone();
    let mut img_mdg = Array2::from_shape_vec(
        (img.width as usize, img.height as usize).set_f(true),
        img_data,
    )
    .unwrap();

    devignette(raf, img.width, img.height, &mut img_mdg.indexed_iter_mut());

    levels::black_sub(img_mdg.indexed_iter_mut(), &img.black_levels);

    if output_stats {
        print_stats(img_mdg.iter().copied());
    }

    let max = (1 << 14) as f32;

    // Let's do some WB.
    let wb = img.white_bal;
    // I think the problem here is an interaction between WB scaling and the
    // matrix?
    let scale_factors = make_normalized_wb_coefs([wb.red as f32, wb.green as f32, wb.blue as f32]);

    let matrix = dng_cam1_to_xyz();

    let buf = ImageBuffer::from_fn(img.width as u32, img.height as u32, |x, y| {
        let demo = Nearest::demosaic(&img_mdg, &mapping, x as u16, y as u16);

        let pixel = Pixel {
            red: demo.red as f32 / max,
            green: demo.green as f32 / max,
            blue: demo.blue as f32 / max,
        };
        let pixel = Pixel {
            red: pixel.red * scale_factors[0],
            green: pixel.green * scale_factors[1],
            blue: pixel.blue * scale_factors[2],
        };
        // I'm still not sure how you decide upon _when_ to clamp, but
        // "immediately before a colorspace conversion"
        // doesn't sound like a terrible place

        let pixel = Pixel {
            red: clamp(pixel.red),
            green: clamp(pixel.green),
            blue: clamp(pixel.blue),
        };
        // Camera -> XYZ -> sRGB
        cam_to_srgb(&matrix, &pixel)
    });

    println!("Done rendering");
    buf
}

fn clamp(val: f32) -> f32 {
    let min = 0.0;
    let max = 1.0;
    assert!(min <= max);
    let mut x = val;
    if x < min {
        x = min;
    }
    if x > max {
        x = max;
    }
    x
}

// TODO: make some changes such that this works better:
// - Define a 'VignetteCorrection' to be from center to the edge of the RAW data
//   so that we can get rid of the hardcoded 3605.
// - Important part is that this is _well defined_, not how we choose to represent it.
fn devignette<'a>(raf: &ParsedRafFile, width: u16, height: u16, img: impl GridIterator<'a>) {
    let devignette = vignette_correction::from_fuji_tags(raf.vignette_attenuation());
    let dvg = |x: usize, y: usize, val: u16| {
        let x = x as i32;
        let y = y as i32;
        let w = width as i32;
        let h = height as i32;
        let x = (x - (w / 2)) as f32;
        let y = (y - (h / 2)) as f32;
        let pos = (x * x + y * y).sqrt() / 3605.0;
        devignette.apply_gain(pos, val as f32)
    };
    for ((x, y), v) in img {
        *v = dvg(x, y, *v) as u16;
    }
}

/// Returns whitebalance coefficients normalized such that the smallest is 1.
/// TODO: figure out why this needs to be > 1; it's almost as if we need to do
///  this in order to make the image clip intentionally, otherwise we get things
///  that look pink.
fn make_normalized_wb_coefs(coefs: [f32; 3]) -> [f32; 3] {
    println!("coefs {:?}", coefs);
    let minval = coefs
        .iter()
        .cloned()
        .filter(|v| *v != 0.0)
        .map_into::<NotNan<f32>>()
        .min()
        .unwrap()
        .into_inner();
    [coefs[0] / minval, coefs[1] / minval, coefs[2] / minval]
}
