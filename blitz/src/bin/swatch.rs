use blitz::camera_specific_junk::cam_xyz;
use blitz::common::Pixel;
use blitz::levels::cam_to_srgb;
use clap::{App, Arg};
use image::{DynamicImage, ImageBuffer, Luma};
use nalgebra::Vector3;
use palette::white_point::D65;
use palette::Xyz;
use std::fs::create_dir_all;
use std::path::Path;

use blitz::diagnostics::TermImage;
use itertools::iproduct;

extern crate nalgebra as na;

fn main() {
    let matches = App::new("Dump Swatch")
        .arg(
            Arg::with_name("Size")
                .short("s")
                .long("size")
                .default_value("512"),
        )
        .arg(Arg::with_name("Output Directory").required(true).index(1))
        .get_matches();

    let output = matches.value_of("Output Directory").unwrap();
    //let axis_size = matches.value_of("Size").unwrap().parse().unwrap();

    create_dir_all(output).unwrap();

    let matrix = cam_xyz();

    let (img, unmappable_frac) = render_xy_chart(&matrix);
    img.display();
    println!("{:.2}% of values not mappable", unmappable_frac);

    /*
    for z in 0..axis_size {
        let path = Path::new(output).join(format!("color-{}.png", z));
        make_for_fixed_z(axis_size, &matrix, &path, z);
    }*/
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

pub fn render_xy_chart(matrix: &nalgebra::Matrix3<f32>) -> (impl TermImage, f32) {
    let axis_size = 512;
    let mut buf = ImageBuffer::new(axis_size, axis_size);
    let resolution = 500;
    let mut unmappable = 0;

    for (r, g, b) in iproduct!(0..resolution, 0..resolution, 0..resolution) {
        let red = r as f32 / (axis_size - 1) as f32;
        let green = g as f32 / (axis_size - 1) as f32;
        let blue = b as f32 / (axis_size - 1) as f32;
        let cam = Vector3::new(red, green, blue);
        // Such that they sum to 1
        let cam = cam.normalize();
        let xyz: Vector3<f32> = matrix * cam;
        if let &[x, y, z] = xyz.as_slice() {
            //println!("xyz: {}", xyz);
            if x >= 0.0 && y >= 0.0 && z >= 0.0 {
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
        unmappable as f32 / (resolution as f32).powi(3),
    )
}

pub fn normalize_xyz(xyz: Xyz<D65, f32>) -> (f32, f32) {
    // Input vector must be normalized
    let (x, y, z) = xyz.into();
    let inverse_z = 1.0 / z;
    (x / inverse_z, y / inverse_z)
}
