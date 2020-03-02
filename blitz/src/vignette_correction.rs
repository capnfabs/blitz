use itertools::Itertools;
extern crate nalgebra as na;

use libraw::tiff::SRational;
use na::{DMatrix, DVector};
use std::convert::TryInto;

const OUTPUT_COEFS: usize = 5;

#[derive(Debug, PartialEq, Clone)]
pub struct VignetteCorrection([f32; OUTPUT_COEFS]);

impl VignetteCorrection {
    pub fn apply_gain(&self, center_distance: f32, value: f32) -> f32 {
        self.compute_gain(center_distance) * value
    }
    fn compute_gain(&self, center_distance: f32) -> f32 {
        //assert!(center_distance <= 1f32);
        let [k0, k1, k2, k3, k4] = self.0;
        let r2 = center_distance.powi(2);
        let r4 = r2.powi(2);
        let r6 = r2.powi(3);
        let r8 = r2.powi(4);
        let r10 = r2.powi(5);
        1f32 + k0 * r2 + k1 * r4 + k2 * r6 + k3 * r8 + k4 * r10
    }
}

fn linear_gain_to_coefs(xs: &[f32], ys: &[f32]) -> [f32; OUTPUT_COEFS] {
    assert_eq!(xs.len(), ys.len());

    // num rows, num cols
    let xmat = DMatrix::from_fn(xs.len(), OUTPUT_COEFS, |r, c| {
        xs[r].powi(2 * (c + 1) as i32)
    });
    let yvec = DVector::from_iterator(ys.len(), ys.iter().copied().map(|y| y - 1.0));
    let xmat_t = xmat.transpose();
    let beta = ((&xmat_t * xmat).try_inverse().unwrap() * &xmat_t) * yvec;
    // There's almost certainly a better way of doing this
    beta.iter()
        .copied()
        .collect_vec()
        .as_slice()
        .try_into()
        .unwrap()
}

// TODO: should this get moved to a Fuji sublibrary thing?
pub fn from_fuji_tags(entry: &[SRational]) -> VignetteCorrection {
    let SRational(_max_pixels, max_points) = entry[0];
    let max_points = max_points as usize;
    let x_vals = &entry[1..(max_points + 1)];
    let y_vals = &entry[(max_points + 1)..];
    assert_eq!(x_vals.len(), max_points);
    assert_eq!(y_vals.len(), max_points);
    let x_vals: Vec<f32> = x_vals
        .iter()
        .map(|sr: &SRational| {
            let &SRational(a, b) = sr;
            (0.5 + a as f32) / (b as f32 + 1.0)
        })
        .collect_vec();
    let y_vals: Vec<f32> = y_vals
        .iter()
        .copied()
        .map(|sr| 2.0 - sr.into_f32() / 100.0)
        .collect_vec();
    let coefs = linear_gain_to_coefs(&x_vals, &y_vals);
    VignetteCorrection(coefs)
}

#[cfg(test)]
mod test {
    use crate::vignette_correction::{
        from_fuji_tags, fuji_tiff_tag_to_vignette_coefs, linear_gain_to_coefs, VignetteCorrection,
    };
    use itertools::Itertools;
    use libraw::tiff::SRational;

    #[test]
    fn test_from_tiff_tag() {
        let data = [
            SRational(3605, 11),
            SRational(0, 10),
            SRational(1, 10),
            SRational(2, 10),
            SRational(3, 10),
            SRational(4, 10),
            SRational(5, 10),
            SRational(6, 10),
            SRational(7, 10),
            SRational(8, 10),
            SRational(9, 10),
            SRational(10, 10),
            SRational(10000, 100),
            SRational(9972, 100),
            SRational(9924, 100),
            SRational(9835, 100),
            SRational(9687, 100),
            SRational(9490, 100),
            SRational(9280, 100),
            SRational(9074, 100),
            SRational(8840, 100),
            SRational(8330, 100),
            SRational(7372, 100),
        ];
        let result = from_fuji_tags(&data);
        assert_eq!(
            result,
            VignetteCorrection([0.06590392, 1.1629808, -3.502386, 4.069807, -1.4535313])
        );
    }

    #[test]
    fn test_linear_gain_to_coefs() {}
}
