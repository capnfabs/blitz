use itertools::Itertools;
use std::iter::once;

pub struct ToneCurve {
    x_vals: Vec<f32>,
    tangents: Vec<f32>,
}

impl ToneCurve {
    pub fn new(points: &Vec<f32>) -> Self {
        // SPLINE TIME
        // https://en.wikipedia.org/wiki/Cubic_Hermite_spline#Catmull%E2%80%93Rom_spline
        let x_inc = 1.0 / points.len() as f32;
        // for 4 points, this should be 12.5, 37.5, 62.5, 87.5, i.e. split into 4 even bands
        let x_vals = (0..points.len()).map(|x| (x as f32 + 0.5) * x_inc);
        let xs = once(0.0).chain(x_vals).chain(once(1.0)).collect_vec();
        let ys = once(0.0)
            .chain(points.iter().copied())
            .chain(once(1.0))
            .collect_vec();
        assert_eq!(xs.len(), ys.len());

        let tangents = (1..(xs.len() - 1))
            .map(|k| {
                // as per wikipedia:
                // m_k =
                (ys[k + 1] - ys[k - 1]) / (xs[k + 1] - xs[k - 1])
            })
            .collect_vec();
        ToneCurve {
            x_vals: xs,
            tangents,
        }
    }

    pub fn apply(val: f32) -> f32 {}
}

#[derive(Debug, Clone)]
pub struct RenderSettings {
    pub tone_curve: Vec<f32>,
    pub exposure_basis: f32,
}

impl Default for RenderSettings {
    fn default() -> Self {
        RenderSettings {
            tone_curve: vec![1.5],
            exposure_basis: 1.0,
        }
    }
}
