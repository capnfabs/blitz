use crate::common::Pixel;
use itertools::Itertools;
use libraw::griditer::{BlackPattern, GridIterator, IndexWrapped2};
use nalgebra::{Matrix3, Vector3};
use palette::{LinSrgb, Srgb};

pub fn black_sub<'a>(grid: impl GridIterator<'a>, black_pattern: &BlackPattern) {
    for (pos, x) in grid {
        let &black = black_pattern.index_wrapped(pos.0, pos.1);
        *x = x.saturating_sub(black);
    }
}

pub fn gamma_curve(power: f32, max: u16) -> Vec<u16> {
    let fmax = max as f32;
    (0..=max)
        .map(|x| fmax * (x as f32 / fmax).powf(1.0 / power))
        .map(|x| x as u16)
        .collect_vec()
}

pub fn apply_gamma<'a>(grid: impl GridIterator<'a>) {
    let gamma = gamma_curve(2.2, (1 << 14) - 1);
    for (_, x) in grid {
        *x = gamma[*x as usize];
    }
}

pub fn cam_to_srgb(matrix: &Matrix3<f32>, px: &Pixel<f32>) -> image::Rgb<u8> {
    let cam = Vector3::new(px.red, px.green, px.blue);
    let matrix = matrix.normalize();
    let xyz: Vector3<f32> = matrix * cam;
    if let &[x, y, z] = xyz.as_slice() {
        let xyz = palette::Xyz::new(x, y, z);
        let srgb: Srgb = xyz.into();
        let (r, g, b) = srgb.into_components();
        image::Rgb([(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8])
    } else {
        unreachable!("Should map");
    }
}
