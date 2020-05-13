use crate::SourceSpaces::{CamRgb, DngCamFwd1, DngCamFwd2, RgbLinearMatrix, SrgbPalette};
use blitz::camera_specific_junk::{
    cam_xyz, dng_cam1_to_xyz, dng_cam2_to_xyz, xyz_from_rgblin, ColorspaceMatrix,
};
use blitzbin::diagnostics::TermImage;
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
                    Arg::with_name("Source Space")
                        .long("source")
                        .default_value("srgb_palette"),
                )
                .arg(
                    Arg::with_name("Size")
                        .short("s")
                        .long("size")
                        .default_value("512"),
                )
                .arg(Arg::with_name("Output Directory").required(true).index(1)),
        )
        .subcommand(
            SubCommand::with_name("chromaplot")
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
        .subcommand(
            SubCommand::with_name("xy")
                .arg(
                    Arg::with_name("Source Space")
                        .long("source")
                        .default_value("srgb_palette"),
                )
                .arg(
                    Arg::with_name("Values")
                        .required(true)
                        .min_values(3)
                        .max_values(3)
                        .index(1),
                ),
        )
        .get_matches();

    match matches.subcommand() {
        ("gradients", Some(opts)) => cmd_gradients(opts),
        ("chromaplot", Some(opts)) => cmd_chromaplot(opts),
        ("xy", Some(opts)) => cmd_xy(opts),
        _ => unreachable!("Must match subcommand"),
    }
}

enum SourceSpaces {
    SrgbPalette,
    RgbLinearMatrix,
    CamRgb,
    DngCamFwd1,
    DngCamFwd2,
}

type MappingFunc = fn(f32, f32, f32) -> Xyz;

impl SourceSpaces {
    pub fn from_name(name: &str) -> Option<SourceSpaces> {
        match name {
            "srgb_palette" => Some(SrgbPalette),
            "srgb_matrix" => Some(RgbLinearMatrix),
            "camrgb" => Some(CamRgb),
            "dngcamfwd1" => Some(DngCamFwd1),
            "dngcamfwd2" => Some(DngCamFwd2),
            _ => None,
        }
    }

    #[allow(dead_code)]
    pub fn name(&self) -> &'static str {
        match self {
            SrgbPalette => "srgb_palette",
            RgbLinearMatrix => "srgb_matrix",
            CamRgb => "camrgb",
            DngCamFwd1 => "dngcamfwd1",
            DngCamFwd2 => "dngcamfwd2",
        }
    }
    pub fn mapping_func(&self) -> MappingFunc {
        match self {
            SrgbPalette => sst_palette_srgb,
            RgbLinearMatrix => sst_rgb_matrix,
            CamRgb => sst_cam_matrix,
            DngCamFwd1 => sst_dng_cam1_to_xyz,
            DngCamFwd2 => sst_dng_cam2_to_xyz,
        }
    }
}

fn make_for_fixed_z(axis_size: u32, mapping_func: MappingFunc, path: impl AsRef<Path>, z: u32) {
    let buffer = ImageBuffer::from_fn(axis_size, axis_size, |x, y| {
        // Source Space -> XYZ
        let r = x as f32 / (axis_size - 1) as f32;
        let g = y as f32 / (axis_size - 1) as f32;
        let b = z as f32 / (axis_size - 1) as f32;
        let xyz = mapping_func(r, g, b);
        // XYZ -> sRGB
        let srgb: Srgb = xyz.into();
        let (r, g, b) = srgb.into_components();
        let rgb = image::Rgb([(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8]);
        rgb
    });

    buffer.save(path).unwrap();
}

fn cmd_gradients(opts: &ArgMatches) {
    let output = opts.value_of("Output Directory").unwrap();
    let axis_size = opts.value_of("Size").unwrap().parse().unwrap();
    let source_space = opts.value_of("Source Space").unwrap();
    let render_function = SourceSpaces::from_name(source_space)
        .map(|x| x.mapping_func())
        .expect("Got invalid source matrix");

    create_dir_all(output).unwrap();
    for z in 0..axis_size {
        let path = Path::new(output).join(format!("color-{}.png", z));
        make_for_fixed_z(axis_size, render_function, &path, z);
    }
}

fn sst_internal_from_matrix(r: f32, g: f32, b: f32, matrix: ColorspaceMatrix) -> Xyz {
    let cam_rgb = Vector3::new(r, g, b);
    let xyz = matrix * cam_rgb;
    if let &[x, y, z] = xyz.as_slice() {
        Xyz::new(x, y, z)
    } else {
        unreachable!("xyz always three elems")
    }
}

fn sst_cam_matrix(r: f32, g: f32, b: f32) -> Xyz {
    sst_internal_from_matrix(r, g, b, cam_xyz())
}

fn sst_palette_srgb(r: f32, g: f32, b: f32) -> Xyz {
    Srgb::new(r, g, b).into()
}

fn sst_rgb_matrix(r: f32, g: f32, b: f32) -> Xyz {
    sst_internal_from_matrix(r, g, b, xyz_from_rgblin())
}

fn sst_dng_cam1_to_xyz(r: f32, g: f32, b: f32) -> Xyz {
    sst_internal_from_matrix(r, g, b, dng_cam1_to_xyz())
}

fn sst_dng_cam2_to_xyz(r: f32, g: f32, b: f32) -> Xyz {
    sst_internal_from_matrix(r, g, b, dng_cam2_to_xyz())
}

fn cmd_xy(opts: &ArgMatches) {
    let source_space = opts.value_of("Source Space").unwrap();
    let render_function = SourceSpaces::from_name(source_space)
        .map(|x| x.mapping_func())
        .expect("Got invalid source matrix");

    let components: Vec<f32> = opts
        .values_of("Values")
        .unwrap()
        .map(|x| x.parse().unwrap())
        .collect();

    let val = render_function(components[0], components[1], components[2]);
    println!("X: {}\nY: {}\nZ: {}", val.x, val.y, val.z);
}

fn cmd_chromaplot(opts: &ArgMatches) {
    let size = opts.value_of("Axis Size").unwrap().parse().unwrap();
    let sample_count = opts.value_of("Sample Count").unwrap().parse().unwrap();
    let source_space = opts.value_of("Source Space").unwrap();
    println!("Generating chroma XY chart with {} samples", sample_count);

    let render_function = SourceSpaces::from_name(source_space)
        .expect("Invalid source space")
        .mapping_func();
    let (img, unmappable_frac) = render_xy_chart(render_function, size, sample_count);
    img.display();
    println!(
        "{:.2}% of values not mappable to XY space",
        unmappable_frac * 100.0
    );
}

pub fn render_xy_chart(
    source_space_transform: MappingFunc,
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
        let xyz = source_space_transform(red, green, blue);
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
