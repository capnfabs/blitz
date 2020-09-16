use hdrhistogram;
use image::{GenericImage, Pixel, Rgba, RgbaImage};
use imageproc::drawing::{draw_filled_rect_mut, Canvas};
use imageproc::rect::Rect;
use itertools::Itertools;

pub struct Histogram {
    channel_histograms: Vec<hdrhistogram::Histogram<u32>>,
}

/// A canvas that adds pixels when drawing.
///
/// See the documentation for [`Canvas`](trait.Canvas.html)
/// for an example using this type.
pub struct BlendAdd<I>(pub I);

impl<I: GenericImage> Canvas for BlendAdd<I>
where
    I::Pixel: Copy + Clone,
{
    type Pixel = I::Pixel;

    fn dimensions(&self) -> (u32, u32) {
        self.0.dimensions()
    }

    fn get_pixel(&self, x: u32, y: u32) -> Self::Pixel {
        self.0.get_pixel(x, y)
    }

    fn draw_pixel(&mut self, x: u32, y: u32, color: Self::Pixel) {
        let px = self.0.get_pixel_mut(x, y);
        let val: Vec<_> = px
            .channels()
            .iter()
            .copied()
            .zip_eq(color.channels().iter().copied())
            .map(|(a, b)| a + b)
            .collect();
        *px = *Pixel::from_slice(&val)
    }
}

const COLORS: [Rgba<u8>; 3] = [
    Rgba([255, 0, 0, 85]),
    Rgba([0, 255, 0, 85]),
    Rgba([0, 0, 255, 85]),
];

impl Histogram {
    pub fn to_img(&self, width: u32, height: u32) -> RgbaImage {
        // TODO: this doesn't really work well for any value _other_ than 256.
        assert!(width >= 256);
        let img = RgbaImage::new(width, height);
        let mut canvas = BlendAdd(img);
        let largest = self.channel_histograms[0]
            .iter_linear(2)
            .map(|x| x.count_since_last_iteration())
            .max()
            .unwrap();
        for (hist, color) in self
            .channel_histograms
            .iter()
            .zip_eq(COLORS.iter().copied())
        {
            let mut start_x = 0;
            for it in hist.iter_linear(2) {
                let end_x = it.value_iterated_to();
                let bar_height = it.count_since_last_iteration() * height as u64 / largest;
                let bar_width = (end_x - start_x) * 256 / width as u64;
                //println!("xpos: {}, bar_height: {}", start_x, bar_height);
                if bar_height != 0 {
                    draw_filled_rect_mut(
                        &mut canvas,
                        Rect::at((start_x) as i32, (height - 1 - bar_height as u32) as i32)
                            .of_size(bar_width as u32, bar_height as u32),
                        color,
                    );
                    start_x = end_x;
                }
            }
        }
        canvas.0
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
