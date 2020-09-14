use crate::common::Pixel;
use crate::tasks::SingleInputSingleOutput;
use itertools::Itertools;
use libraw::griditer::{BlackPattern, GridIterator, IndexWrapped2};
use nalgebra::{Matrix3, Vector3};
use ndarray::ArrayViewMut2;
use palette::chromatic_adaptation::AdaptInto;
use palette::white_point::{D50, D65};
use palette::{encoding, Hsl, Srgb, Xyz};

pub fn black_sub(black_pattern: &BlackPattern, grid: &mut ArrayViewMut2<u16>) {
    for (pos, x) in grid.indexed_iter_mut() {
        let &black = black_pattern.index_wrapped(pos.0, pos.1);
        *x = x.saturating_sub(black);
    }
}

pub fn make_black_sub_task(black_pattern: BlackPattern) -> impl SingleInputSingleOutput<u16> {
    move |x, y, val: u16| {
        let &black = black_pattern.index_wrapped(x, y);
        val.saturating_sub(black)
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
    let xyz: Vector3<f32> = matrix * cam;
    if let &[x, y, z] = xyz.as_slice() {
        let xyz: Xyz<D50> = Xyz::with_wp(x, y, z);
        let xyz: Xyz<D65> = xyz.adapt_into();
        let srgb: Srgb = xyz.into();
        let (r, g, b) = srgb.into_components();
        let rgb = image::Rgb([(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8]);
        rgb
    } else {
        unreachable!("Should map");
    }
}

pub fn cam_to_hsl(matrix: &Matrix3<f32>, px: &Pixel<f32>) -> Hsl<encoding::Srgb, f32> {
    let cam = Vector3::new(px.red, px.green, px.blue);
    let xyz: Vector3<f32> = matrix * cam;
    if let &[x, y, z] = xyz.as_slice() {
        let xyz: Xyz<D50> = Xyz::with_wp(x, y, z);
        let xyz: Xyz<D65> = xyz.adapt_into();
        let hsl: Hsl = xyz.into();
        hsl
    } else {
        unreachable!("Should map");
    }
}

pub fn hsl_to_rgb(hsl: &Hsl<encoding::Srgb, f32>) -> image::Rgb<u8> {
    let srgb: Srgb = (*hsl).into();
    let (r, g, b) = srgb.into_components();
    let rgb = image::Rgb([(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8]);
    rgb
}
