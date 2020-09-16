use hdrhistogram;
use image::{Rgb, RgbImage, Rgba, RgbaImage};
use imageproc::drawing::draw_filled_rect_mut;
use imageproc::rect::Rect;
use itertools::Itertools;

pub struct Histogram {
    channel_histograms: Vec<hdrhistogram::Histogram<u32>>,
}

impl Histogram {
    pub fn to_img(&self, width: u32, height: u32) -> RgbaImage {
        let mut img = RgbaImage::new(256, 128);
        let largest = self.channel_histograms[0]
            .iter_linear(2)
            .map(|x| x.count_since_last_iteration())
            .max()
            .unwrap();
        let mut last_value_iterated_to = 0;
        for it in self.channel_histograms[0].iter_linear(2) {
            let x_boundary = it.value_iterated_to();
            let bar_height = it.count_since_last_iteration() * 128 / largest;
            println!(
                "xpos: {}, bar_height: {}",
                last_value_iterated_to, bar_height
            );
            if bar_height != 0 {
                draw_filled_rect_mut(
                    &mut img,
                    Rect::at((last_value_iterated_to) as i32, (127 - bar_height) as i32).of_size(
                        (x_boundary - last_value_iterated_to) as u32,
                        bar_height as u32,
                    ),
                    Rgba::from([255, 0, 0, 85]),
                );
                last_value_iterated_to = x_boundary;
            }
        }
        img
    }
}

pub trait ToHistogram {
    fn histogram(&self) -> Histogram;
}

impl ToHistogram for image::RgbImage {
    fn histogram(&self) -> Histogram {
        // One per channel. TODO: make this generic somehow
        let mut hists = (0..3)
            .map(|_| hdrhistogram::Histogram::<u32>::new_with_bounds(1, 256, 3).unwrap())
            .collect_vec();

        for val in self.pixels() {
            for (hist, channel_val) in hists.iter_mut().zip_eq(val.0.iter().copied()) {
                hist.record(channel_val as u64).unwrap();
            }
        }

        Histogram {
            channel_histograms: hists,
        }
    }
}
