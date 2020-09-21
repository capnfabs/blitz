use itertools::Itertools;
use splines;
use splines::{Interpolation, Key, Spline};
use std::iter::{once, repeat};

#[derive(Debug, Clone)]
pub struct ToneCurve {
    pub spline: Spline<f32, f32>,
}

impl ToneCurve {
    pub fn new(points: &[f32]) -> Self {
        // SPLINE TIME
        // https://en.wikipedia.org/wiki/Cubic_Hermite_spline#Catmull%E2%80%93Rom_spline
        // TODO: figure out what's going on with the edges, they're unhappy here.
        // Start and end at 1 because this is multiplicative.
        let x_inc = 1.0 / points.len() as f32;
        let start = Key::new(0., 1., Interpolation::Cosine);
        let end = Key::new(1., 1., Interpolation::default());

        // for 4 points, this should be 12.5, 37.5, 62.5, 87.5, i.e. split into 4 even bands
        let xs = (0..points.len()).map(|x| (x as f32 + 0.5) * x_inc);

        let iter = once(start)
            .chain(
                xs.zip_eq(points.iter().copied())
                    .map(|(x, y)| Key::new(x, y, Interpolation::CatmullRom)),
            )
            .chain(repeat(end).take(2));
        let spline = Spline::from_iter(iter);

        ToneCurve { spline }
    }

    pub fn apply(&self, val: f32) -> f32 {
        self.spline.sample(val).unwrap_or(f32::NAN)
    }
}

impl Default for ToneCurve {
    fn default() -> Self {
        let spl = Spline::from_vec(vec![
            // Flat curve
            Key::new(0.0, 1.0, Interpolation::Linear),
            Key::new(1.0, 1.0, Interpolation::Linear),
        ]);
        ToneCurve { spline: spl }
    }
}

#[derive(Debug, Clone)]
pub struct RenderSettings {
    pub tone_curve: ToneCurve,
    pub exposure_basis: f32,
    pub auto_contrast: bool,
}

impl Default for RenderSettings {
    fn default() -> Self {
        RenderSettings {
            tone_curve: ToneCurve::default(),
            exposure_basis: 1.0,
            // TODO: this should not be the default
            auto_contrast: true,
        }
    }
}
