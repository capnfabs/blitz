use blitz::common::Pixel;
use blitz::demosaic::{Demosaic, Nearest};
use blitz::diagnostics::TermImage;
use blitz::{diagnostics, histo, pathutils, vignette_correction};
use clap::{App, Arg, ArgMatches};
use image::{imageops, DynamicImage, ImageBuffer, ImageFormat};
use itertools::Itertools;
use libraw::raf::{ParsedRafFile, RafFile};
extern crate nalgebra as na;
use blitz::camera_specific_junk::dng_cam1_to_xyz;
use blitz::levels::{cam_to_srgb, make_black_sub_task};
use blitz::tasks::{par_index_map_raiso, par_index_map_siso, SingleInputSingleOutput};
use image::imageops::FilterType::Lanczos3;
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
        .arg(Arg::with_name("stats").long("stats"))
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
    } else {
        println!("Resizing...");
        let img = imageops::resize(&rendered, 563, 375, Lanczos3);
        println!("Displaying...");
        DynamicImage::ImageRgb8(img).display();
    }
}

fn print_stats(img: &ArrayView2<f32>) {
    println!("Histogram!");
    let histo_width = 1000;
    let h = histo::Histo::from_iter(img.iter().map(|&x| x.powf(1.0 / 2.2)), histo_width);
    diagnostics::render_histogram(&h, 600, 1000).display();
    println!();
}

fn render_raw(img: &ParsedRafFile, output_stats: bool) -> image::RgbImage {
    let raf = img;
    let ri = &img.render_info();

    let mapping = Array2::from_shape_vec((6, 6).set_f(true), ri.xtrans_mapping.clone()).unwrap();

    let src = ArrayView2::from_shape(
        (ri.width as usize, ri.height as usize).set_f(true),
        ri.raw_data,
    )
    .unwrap();

    // Some setup
    let max = (1 << 14) as f32;
    let wb = ri.white_bal;
    let scale_factors = make_normalized_wb_coefs([wb.red as f32, wb.green as f32, wb.blue as f32]);
    let matrix = dng_cam1_to_xyz();

    // Define steps
    let devignette = make_devignetter(raf);
    let black_sub = make_black_sub_task(ri.black_levels.clone());
    let convert_to_float = |_: usize, _: usize, val: u16| val as f32 / max;
    let apply_wb = move |pixel: &Pixel<f32>| Pixel {
        red: pixel.red * scale_factors[0],
        green: pixel.green * scale_factors[1],
        blue: pixel.blue * scale_factors[2],
    };
    let clamp = |pixel: &Pixel<_>| Pixel {
        red: float_clamp(pixel.red),
        green: float_clamp(pixel.green),
        blue: float_clamp(pixel.blue),
    };
    let convert_to_srgb = |pixel: &Pixel<_>| cam_to_srgb(&matrix, pixel);

    // Run steps
    // This is the "operating on single values" phase.
    let img = par_index_map_siso(&src, |x, y, val| {
        let val = devignette(x, y, val);
        let val = black_sub(x, y, val);
        let val = convert_to_float(x, y, val);
        val
    });

    if output_stats {
        print_stats(&img.view());
    }

    // This is "demosaic" and then "operate on single values again".
    let img = par_index_map_raiso(&img.view(), |x, y, data: &ArrayView2<_>| {
        let val = Nearest::demosaic(data, &mapping, x, y);
        let val = apply_wb(&val);
        let val = clamp(&val);
        let val = convert_to_srgb(&val);
        val
    });

    // Last step: crop and convert.
    let (output_width, output_height) = ri.crop_rect.size();
    let buf = ImageBuffer::from_fn(output_width as u32, output_height as u32, |x, y| {
        img[(
            ri.crop_rect.left + x as usize,
            ri.crop_rect.top + y as usize,
        )]
    });

    println!("Done rendering");
    buf
}

fn float_clamp(val: f32) -> f32 {
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

trait Sized {
    fn width(&self) -> usize;
    fn height(&self) -> usize;
}

impl<'a, T> Sized for ArrayView2<'a, T> {
    fn width(&self) -> usize {
        self.nrows()
    }

    fn height(&self) -> usize {
        self.ncols()
    }
}

impl<'a, T> Sized for ArrayViewMut2<'a, T> {
    fn width(&self) -> usize {
        self.nrows()
    }

    fn height(&self) -> usize {
        self.ncols()
    }
}

fn make_devignetter(raf: &ParsedRafFile) -> impl SingleInputSingleOutput<u16, u16> {
    let devignette = vignette_correction::from_fuji_tags(raf.vignette_attenuation());
    let w = raf.render_info().width as i32;
    let h = raf.render_info().height as i32;
    let dvg = move |x: usize, y: usize, val: u16| {
        let x = x as i32;
        let y = y as i32;
        let x = (x - (w / 2)) as f32;
        let y = (y - (h / 2)) as f32;
        let pos = (x * x + y * y).sqrt() / 3605.0;
        let result = devignette.apply_gain(pos, val as f32);
        result as u16
    };
    dvg
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
