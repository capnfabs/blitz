use nalgebra::Matrix3;

type CamXyz = nalgebra::Matrix3<f32>;

pub fn cam_xyz() -> CamXyz {
    // Value from libraw
    #[rustfmt::skip]
    let _matrix = Matrix3::new(
        11434.0, -4948.0, -1210.0,
        -3746.0, 12042.0,  1903.0,
         -666.0,  1479.0,  5235.0,
    );

    let matrix_inv = Matrix3::new(
        0.53416154,
        0.41342894,
        0.05240952,
        0.16269031,
        1.01245195,
        -0.17514226,
        0.02199286,
        -0.23344274,
        1.21144988,
    );

    #[rustfmt::skip]
    let _xyz_rgb_linear = Matrix3::new(
         3.2406, -1.5372, -0.4986,
        -0.9689,  1.8758,  0.0415,
         0.0557, -0.2040,  1.0570,
    );
    matrix_inv
}
