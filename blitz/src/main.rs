use crate::common::Pixel;
use clap::{App, Arg};
use image::{ImageBuffer, ImageFormat};
use itertools::Itertools;
use libraw::raf::{ParsedRafFile, RafFile};
use libraw::util::datagrid::{DataGrid, MutableDataGrid, Position, Size};
use ordered_float::NotNan;
use std::cmp::min;

mod common;
mod demosaic;
mod levels;
mod vignette_correction;

#[allow(unused_imports)]
use crate::demosaic::{Nearest, Passthru};
use demosaic::Demosaic;

mod pathutils;

fn main() {
    let matches = App::new("Blitz")
        .arg(Arg::with_name("render").short("r").long("render"))
        .arg(Arg::with_name("INPUT").required(true).index(1))
        .get_matches();

    let input = matches.value_of("INPUT").unwrap();
    let render = matches.occurrences_of("render") == 1;

    load_and_maybe_render(input, render);
}

fn load_and_maybe_render(img_file: &str, render: bool) {
    println!("Loading RAW data: native");
    let file = RafFile::open(img_file).unwrap();
    println!("Opened file: {:?}", file);
    println!("Parsing...");
    let details = file.parse_raw().unwrap();

    println!("Parsed.");

    if !render {
        return;
    }

    let raw_preview_filename = pathutils::get_output_path("native");
    let rendered = render_raw(&details);
    println!("Saving");
    rendered
        .save_with_format(&raw_preview_filename, ImageFormat::TIFF)
        .unwrap();
    pathutils::set_readonly(&raw_preview_filename);
    println!("Done saving");
    pathutils::open_preview(&raw_preview_filename);
}

fn render_raw(img: &ParsedRafFile) -> image::RgbImage {
    let raf = img;
    let img = &img.render_info();

    // Change 14 bit to 16 bit.
    //let img_data: Vec<u16> = img_data.iter().copied().map(|v| v << 2).collect();

    let mapping = DataGrid::wrap(&img.xtrans_mapping, Size(6, 6));

    let mut img_data = img.raw_data.clone();
    let mut img_mdg =
        MutableDataGrid::new(&mut img_data, Size(img.width as usize, img.height as usize));
    levels::black_sub(&mut img_mdg);

    let devignette = vignette_correction::from_fuji_tags(raf.vignette_attenuation());

    let dvg = |x: usize, y: usize, val: u16| {
        let x = x as i32;
        let y = y as i32;
        let w = img.width as i32;
        let h = img.height as i32;
        let x = (x - (w / 2)) as f32;
        let y = (y - (h / 2)) as f32;
        let pos = (x * x + y * y).sqrt() / 3605.0;
        devignette.apply_gain(pos, val as f32)
    };

    for (Position(x, y), v) in img_mdg.iter_pos_mut() {
        *v = dvg(x, y, *v) as u16;
    }
    let mut h = histogram::Histogram::new();
    for v in img_mdg.iter_mut() {
        h.increment(*v as u64).unwrap();
    }

    let img_grid = DataGrid::wrap(&img_data, Size(img.width as usize, img.height as usize));

    // Compute scaling params
    println!("Stats!");
    println!(
        "Max: {}\n99%: {}\n95%: {}\n75%: {}\n50%: {}\n25%: {}\n05%: {}\n01%: {}\nMin: {}",
        h.percentile(100.0).unwrap(),
        h.percentile(99.0).unwrap(),
        h.percentile(95.0).unwrap(),
        h.percentile(75.0).unwrap(),
        h.percentile(50.0).unwrap(),
        h.percentile(25.0).unwrap(),
        h.percentile(5.0).unwrap(),
        h.percentile(1.0).unwrap(),
        h.percentile(0.0).unwrap(),
    );
    let max = h.percentile(99.0).unwrap();
    // This is int scaling, so it'll be pretty crude (e.g. Green will only scale 4x, not 4.5x)
    // Camera scaling factors are 773, 302, 412. They are theoretically white balance but I don't know
    // how they work.

    // Let's do some WB.
    let wb = img.white_bal;
    let scale_factors =
        make_normalized_wb_coefs([wb.red as f32, wb.green as f32, wb.blue as f32, 0.0]);
    println!("scale_factors: {:?}", scale_factors);
    let scale_factors: Vec<f32> = scale_factors
        .iter()
        .map(|val| val * (std::u16::MAX as f32) / max as f32)
        .collect();
    println!("scale_factors: {:?}", scale_factors);
    let scale_factors: Vec<u16> = scale_factors.iter().copied().map(|v| v as u16).collect();
    println!("scale_factors: {:?}", scale_factors);

    let buf = ImageBuffer::from_fn(img.width as u32, img.height as u32, |x, y| {
        saturating_scale(
            Nearest::demosaic(&img_grid, &mapping, x as u16, y as u16),
            &scale_factors,
        )
        .to_rgb()
    });
    println!("Done rendering");
    buf
}

fn saturating_scale(p: Pixel<u16>, scale_factors: &[u16]) -> Pixel<u16> {
    Pixel {
        red: min(p.red as u32 * scale_factors[0] as u32, std::u16::MAX as u32) as u16,
        green: min(
            p.green as u32 * scale_factors[1] as u32,
            std::u16::MAX as u32,
        ) as u16,
        blue: min(
            p.blue as u32 * scale_factors[2] as u32,
            std::u16::MAX as u32,
        ) as u16,
    }
}

/// Returns whitebalance coefficients normalized such that the smallest is 1
fn make_normalized_wb_coefs(coefs: [f32; 4]) -> [f32; 3] {
    println!("coefs {:?}", coefs);
    let minval = coefs
        .iter()
        .cloned()
        .filter(|v| *v != 0.0)
        .map_into::<NotNan<f32>>()
        .min()
        .unwrap()
        .into_inner();
    println!("coefs min {:?}", minval);
    [coefs[0] / minval, coefs[1] / minval, coefs[2] / minval]
}
