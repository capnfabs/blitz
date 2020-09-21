extern crate nalgebra as na;

use image::ImageBuffer;
use itertools::Itertools;
use ndarray::prelude::*;
use ndarray::Array2;
use ordered_float::NotNan;
use palette::Hsv;

use libraw::raf::ParsedRafFile;

use crate::camera_specific_junk::dng_cam2_to_xyz;
use crate::common::Pixel;
use crate::demosaic::{Demosaic, Nearest};
use crate::levels::{cam_to_hsv, make_black_sub_task, to_rgb};
use crate::render_settings::RenderSettings;
use crate::tasks::{par_index_map_raiso, par_index_map_siso, SingleInputSingleOutput};
use crate::vignette_correction;

pub fn render_raw(img: &ParsedRafFile) -> image::RgbImage {
    render_raw_with_settings(img, &Default::default())
}

pub fn render_raw_with_settings(img: &ParsedRafFile, settings: &RenderSettings) -> image::RgbImage {
    println!("Settings: {:?}", settings);
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
    let matrix = dng_cam2_to_xyz();

    // Define steps
    //let devignette = make_devignetter(img);
    let black_sub = make_black_sub_task(ri.black_levels.clone());
    let convert_to_float = |_: usize, _: usize, val: u16| val as f32 / max;
    let apply_wb = move |pixel: &Pixel<f32>| Pixel {
        red: pixel.red * scale_factors[0],
        green: pixel.green * scale_factors[1],
        blue: pixel.blue * scale_factors[2],
    };

    let apply_curve = |pixel: &Hsv| {
        let val = pixel.value;
        let factor = settings.tone_curve.spline.clamped_sample(val).unwrap();
        let mut ret = pixel.clone();
        ret.value *= factor;
        ret
    };

    let convert_to_hsv = |pixel: &Pixel<_>| cam_to_hsv(&matrix, pixel);

    // Run steps
    // This is the "operating on single values" phase.
    let img = par_index_map_siso(&src, |x, y, val| {
        //let val = devignette(x, y, val);
        let val = black_sub(x, y, val);
        let val = convert_to_float(x, y, val);
        let val = val * (settings.exposure_basis);
        val
    });

    // This is "demosaic" and then "operate on single values again".
    let img = par_index_map_raiso(&img.view(), |x, y, data: &ArrayView2<_>| {
        let val = Nearest::demosaic(data, &mapping, x, y);
        let val = apply_wb(&val);
        // NOTE: we used to clamp here, but it looks like we don't need it anymore because we're
        // round-tripping through HSV?
        let val = convert_to_hsv(&val);
        val
    });

    let img = if settings.auto_contrast {
        // collect information
        let mut hist = hdrhistogram::Histogram::<u32>::new(3).unwrap();
        for pix in img.view() {
            hist.record((pix.value * std::u32::MAX as f32) as u64)
                .unwrap();
        }

        let val_at = |quant| {
            println!(
                "  {:4}%: {}",
                (quant * 100.) as u32,
                hist.value_at_quantile(quant) as f32 / std::u32::MAX as f32
            );
        };
        val_at(0.);
        val_at(0.01);
        val_at(0.05);
        val_at(0.5);
        val_at(0.95);
        val_at(0.99);
        val_at(1.);

        let s_min = hist.value_at_quantile(0.05) as f32 / std::u32::MAX as f32;
        let s_max = hist.value_at_quantile(0.95) as f32 / std::u32::MAX as f32;

        // apply auto-contrast-stretching
        // TODO: I think this is wrong and doesn't account for the log/lin boundary sufficiently.
        // It depends on what the Hsv type is doing.
        par_index_map_siso(&img.view(), |_x, _y, mut val: Hsv<_>| {
            val.value = (val.value - s_min) / (s_max - s_min);
            val
        })
    } else {
        img
    };

    let img = par_index_map_siso(&img.view(), |_x, _y, mut val: Hsv<_>| {
        val.saturation += settings.saturation_boost;
        to_rgb(&val)
    });

    // Last step: crop and convert.
    let (output_width, output_height) = ri.crop_rect.size();
    println!("Cropped to {}x{} pixels", output_width, output_height);
    let buf = ImageBuffer::from_fn(output_width as u32, output_height as u32, |x, y| {
        img[(
            ri.crop_rect.left + x as usize,
            ri.crop_rect.top + y as usize,
        )]
    });

    println!("Done rendering");
    buf
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
