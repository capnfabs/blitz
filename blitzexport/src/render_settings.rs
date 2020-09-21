use blitz::render_settings as brs;

#[repr(C)]
pub struct RenderSettings {
    tone_curve: [f32; 5],
    exposure_basis: f32,
}

const TONE_CURVE_CONST: f32 = 2.0;

impl RenderSettings {
    pub fn to_blitz_settings(&self) -> brs::RenderSettings {
        let coefs: Vec<_> = self
            .tone_curve
            .iter()
            .copied()
            .map(|x| TONE_CURVE_CONST.powf(x))
            .collect();
        let tc = brs::ToneCurve::new(&coefs);
        /*
        println!("Tonecurve: {:?}", tc);
        for i in 0..100 {
            println!("{},{}", i, tc.apply(i as f32 / 100.));
        }*/

        brs::RenderSettings {
            tone_curve: tc,
            exposure_basis: TONE_CURVE_CONST.powf(self.exposure_basis),
            // TODO: set this dynamically
            auto_contrast: true,
        }
    }
}
