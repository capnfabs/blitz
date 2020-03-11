use itertools::Itertools;
use nalgebra::Matrix3;

type CamXyz = nalgebra::Matrix3<f32>;

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

pub fn cam_xyz() -> CamXyz {
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
    let rgb_from_cam = cam_from_rgb.try_inverse().unwrap();
    rgb_from_cam.normalize()
}
