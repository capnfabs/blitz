use blitz::camera_specific_junk::cam_xyz;
use blitz::common::Pixel;
use blitz::diagnostics::TermImage;
use blitz::levels::cam_to_srgb;
use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use image::{DynamicImage, ImageBuffer, Luma};
use itertools::iproduct;
use nalgebra::Vector3;
use palette::white_point::D65;
use palette::Xyz;
use std::fs::create_dir_all;
use std::path::Path;

fn main() {
    let matches = App::new("Swatchtool")
        .setting(AppSettings::SubcommandRequired)
        .subcommand(
            SubCommand::with_name("gradients")
                .arg(
                    Arg::with_name("Size")
                        .short("s")
                        .long("size")
                        .default_value("512"),
                )
                .arg(Arg::with_name("Output Directory").required(true).index(1)),
        )
        .subcommand(
            SubCommand::with_name("xy")
                .arg(
                    Arg::with_name("Axis Size")
                        .short("s")
                        .long("size")
                        .default_value("512"),
                )
                .arg(
                    Arg::with_name("Sample Count")
                        .long("samples")
                        .short("n")
                        .default_value("125000000"),
                ),
        )
        .get_matches();

    match matches.subcommand() {
        ("gradients", Some(opts)) => cmd_gradients(opts),
        ("xy", Some(opts)) => cmd_xy(opts),
        _ => unreachable!("Must match subcommand"),
    }
}

fn make_for_fixed_z(
    axis_size: u32,
    matrix: &nalgebra::Matrix3<f32>,
    path: impl AsRef<Path>,
    z: u32,
) {
    let buffer = ImageBuffer::from_fn(axis_size, axis_size, |x, y| {
        // Camera -> XYZ -> sRGB
        let red = x as f32 / (axis_size - 1) as f32;
        let green = y as f32 / (axis_size - 1) as f32;
        let blue = z as f32 / (axis_size - 1) as f32;
        cam_to_srgb(&matrix, &Pixel { red, green, blue })
    });

    buffer.save(path).unwrap();
}

fn cmd_gradients(opts: &ArgMatches) {
    let output = opts.value_of("Output Directory").unwrap();
    let axis_size = opts.value_of("Size").unwrap().parse().unwrap();
    create_dir_all(output).unwrap();
    let matrix = cam_xyz();
    for z in 0..axis_size {
        let path = Path::new(output).join(format!("color-{}.png", z));
        make_for_fixed_z(axis_size, &matrix, &path, z);
    }
}

fn cmd_xy(opts: &ArgMatches) {
    let matrix = cam_xyz();
    let size = opts.value_of("Axis Size").unwrap().parse().unwrap();
    let sample_count = opts.value_of("Sample Count").unwrap().parse().unwrap();
    println!("Generating chroma XY chart with {} samples", sample_count);
    let (img, unmappable_frac) = render_xy_chart(&matrix, size, sample_count);
    img.display();
    println!("{:.2}% of values not mappable", unmappable_frac * 100.0);
}

pub fn render_xy_chart(
    matrix: &nalgebra::Matrix3<f32>,
    axis_size: u32,
    sample_count: u32,
) -> (impl TermImage, f32) {
    let mut buf = ImageBuffer::new(axis_size, axis_size);
    let samples_per_axis = (sample_count as f32).powf(1.0 / 3.0) as u32;
    let mut unmappable: u32 = 0;

    for (r, g, b) in iproduct!(
        0..samples_per_axis,
        0..samples_per_axis,
        0..samples_per_axis
    ) {
        let red = r as f32 / (samples_per_axis - 1) as f32;
        let green = g as f32 / (samples_per_axis - 1) as f32;
        let blue = b as f32 / (samples_per_axis - 1) as f32;
        let cam = Vector3::new(red, green, blue);
        // Such that they sum to 1
        let cam = cam.normalize();
        let xyz: Vector3<f32> = matrix * cam;
        if let &[x, y, z] = xyz.as_slice() {
            if x >= 0.0 && y >= 0.0 && z >= 0.0 && x <= 1.0 && y <= 1.0 && z <= 1.0 {
                let x_img = (x * axis_size as f32) as u32;
                let y_img = (y * axis_size as f32) as u32;
                buf[(x_img, axis_size - 1 - y_img)] = Luma([255u8]);
            } else {
                unmappable += 1;
            }
        } else {
            unreachable!("Should map");
        }
    }
    (
        DynamicImage::ImageLuma8(buf),
        unmappable as f32 / sample_count as f32,
    )
}

pub fn normalize_xyz(xyz: Xyz<D65, f32>) -> (f32, f32) {
    // Input vector must be normalized
    let (x, y, z) = xyz.into();
    let inverse_z = 1.0 / z;
    (x / inverse_z, y / inverse_z)
}
