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

    #[rustfmt::skip]
    let xyz_from_rgb_linear = Matrix3::new(
        0.412453, 0.357580, 0.180423, 
        0.212671, 0.715160, 0.072169, 
        0.019334, 0.119193, 0.950227,
    );
    let cam_from_rgb = cam_from_xyz * xyz_from_rgb_linear;
    dump_mat("cam_from_rgb", &cam_from_rgb);
    // line norm
    let rows_normalized: Vec<_> = cam_from_rgb.row_iter().map(|row| row / row.sum()).collect();
    let line_norm = Matrix3::from_rows(&rows_normalized);
    dump_mat("cam_from_rgb_normie", &line_norm);
    let rgb_from_cam = line_norm.pseudo_inverse(0.00001).unwrap().transpose();
    dump_mat("rgb_from_cam", &rgb_from_cam);
    rgb_from_cam
}

pub fn cam_xyz() -> ColorspaceMatrix {
    // I'm pretty sure I lifted this from an intermediate step in libraw.
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
