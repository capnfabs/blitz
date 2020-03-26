use itertools::Itertools;
use nalgebra::Matrix3;

pub type ColorspaceMatrix = nalgebra::Matrix3<f32>;

pub fn dump_mat(label: &str, val: &Matrix3<f32>) {
    println!("{}: ", label);
    for row in val.row_iter() {
        println!(
            "{}",
            row.iter()
                .map(|x| format!("{:.6}", x))
                .collect_vec()
                .join(" ")
        )
    }
}

pub fn cam_rgb_linear() -> ColorspaceMatrix {
    // Value from libraw
    #[rustfmt::skip]
    let cam_from_xyz = Matrix3::new(
        11434.0, -4948.0, -1210.0,
        -3746.0, 12042.0,  1903.0,
         -666.0,  1479.0,  5235.0,
    ) / 10_000.0;

    let cam_from_rgb = cam_from_xyz * xyz_from_rgblin();
    dump_mat("cam_from_rgb", &cam_from_rgb);
    // line norm
    let rows_normalized: Vec<_> = cam_from_rgb.row_iter().map(|row| row / row.sum()).collect();
    let line_norm = Matrix3::from_rows(&rows_normalized);
    dump_mat("cam_from_rgb_normie", &line_norm);
    let rgb_from_cam = line_norm.pseudo_inverse(0.00001).unwrap().transpose();
    dump_mat("rgb_from_cam", &rgb_from_cam);
    rgb_from_cam
}

pub fn xyz_from_rgblin() -> ColorspaceMatrix {
    #[rustfmt::skip]
    let x = Matrix3::new(
        0.412453, 0.357580, 0.180423,
        0.212671, 0.715160, 0.072169,
        0.019334, 0.119193, 0.950227,
    );
    x
}

pub fn rgblin_from_xyz() -> ColorspaceMatrix {
    #[rustfmt::skip]
    let x = Matrix3::new(
         3.2406, -1.5372, -0.4986,
         -0.989,  1.8758,  0.0415,
         0.0557, -0.2040,  1.0570,
    );
    x
}

// TODO: I think this is cam -> xyz conversion, and I'm pretty sure I lifted this from an intermediate step in libraw.
pub fn cam_xyz() -> ColorspaceMatrix {
    Matrix3::new(
        0.53416154,
        0.41342894,
        0.05240952,
        0.16269031,
        1.01245195,
        -0.17514226,
        0.02199286,
        -0.23344274,
        1.21144988,
    )
}

// From tag C715 (ForwardMatrix2) in DSCF6233.dng
// Note that this is XYZ D50 and normally we use D65, not sure if it matters.
// This is calibration illuminant 21 (D65 == roughly 6504K)
pub fn dng_cam2_to_xyz() -> ColorspaceMatrix {
    #[rustfmt::skip]
    let mat = Matrix3::new(
        0.3909, 0.4132, 0.1602, 
        0.1935, 0.7584, 0.0481, 
        0.0909, 0.0015, 0.7326,
    );
    mat
}

// From tag C714 (ForwardMatrix1) in DSCF6233.dng
// Note that this is XYZ D50 and normally we use D65, not sure if it matters.
// This is calibration illuminant 17 (Standard Light A), approx 2856K
pub fn dng_cam1_to_xyz() -> ColorspaceMatrix {
    #[rustfmt::skip]
    let mat = Matrix3::new(
        0.4481, 0.4033, 0.1129, 
        0.2183, 0.7469, 0.0349, 
        0.1230, 0.0016, 0.7004,
    );
    mat
}
