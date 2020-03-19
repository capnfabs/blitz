use blitz::camera_specific_junk::{cam_xyz, xyz_from_rgblin};
use blitz::common::Pixel;
use blitz::diagnostics::TermImage;
use blitz::levels::cam_to_srgb;
use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use image::{DynamicImage, ImageBuffer, Luma};
use itertools::iproduct;
use nalgebra::Vector3;
use palette::{Srgb, Xyz};
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
                        .default_value("15000000"),
                )
                .arg(
                    Arg::with_name("Source Space")
                        .long("source")
                        .default_value("srgb_palette"),
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
/*
enum SourceSpaceTransform {
    CamMatrix,
    SrgbLinearMatrix,
    SrgbPaletteLib,
}
*/
fn sst_cam_matrix(input: (f32, f32, f32)) -> Xyz {
    let (r, g, b) = input;
    let cam_rgb = Vector3::new(r, g, b);
    let matrix = cam_xyz();
    let xyz = matrix * cam_rgb;
    if let &[x, y, z] = xyz.as_slice() {
        Xyz::new(x, y, z)
    } else {
        unreachable!("xyz always three elems")
    }
}

fn sst_palette_srgb(input: (f32, f32, f32)) -> Xyz {
    let (r, g, b) = input;
    Srgb::new(r, g, b).into()
}

fn sst_rgb_matrix(input: (f32, f32, f32)) -> Xyz {
    let (r, g, b) = input;
    let matrix = xyz_from_rgblin();
    let xyz = matrix * Vector3::new(r, g, b);
    if let &[x, y, z] = xyz.as_slice() {
        Xyz::new(x, y, z)
    } else {
        unreachable!("xyz always three elems")
    }
}

fn cmd_xy(opts: &ArgMatches) {
    let size = opts.value_of("Axis Size").unwrap().parse().unwrap();
    let sample_count = opts.value_of("Sample Count").unwrap().parse().unwrap();
    let source_space = opts.value_of("Source Space").unwrap();
    println!("Generating chroma XY chart with {} samples", sample_count);

    // TODO: use an enum for this.
    let render_function = match source_space {
        "srgb_palette" => sst_palette_srgb,
        "srgb_matrix" => sst_rgb_matrix,
        "camrgb" => sst_cam_matrix,
        _ => panic!("lol no"),
    };
    let (img, unmappable_frac) = render_xy_chart(render_function, size, sample_count);
    img.display();
    println!(
        "{:.2}% of values not mappable to XY space",
        unmappable_frac * 100.0
    );
}

pub fn render_xy_chart(
    source_space_transform: fn((f32, f32, f32)) -> Xyz,
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
        let xyz = source_space_transform((red, green, blue));
        let (x, y, z) = xyz.into_components();
        let sum = x + y + z;
        let x = x / sum;
        let y = y / sum;
        let z = z / sum;

        if x >= 0.0 && y >= 0.0 && z >= 0.0 && x < 1.0 && y < 1.0 {
            let x_img = (x * axis_size as f32) as u32;
            let y_img = (y * axis_size as f32) as u32;
            buf[(x_img, axis_size - 1 - y_img)] = Luma([255u8]);
        } else {
            unmappable += 1;
        }
    }
    (
        DynamicImage::ImageLuma8(buf),
        unmappable as f32 / sample_count as f32,
    )
}
