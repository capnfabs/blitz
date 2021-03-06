use blitz::render_settings as brs;
use blitz::render_settings::LensCorrections;

#[repr(C)]
pub struct RenderSettings {
    tone_curve: [f32; 5],
    exposure_basis: f32,
    auto_contrast: bool,
    saturation_boost: f32,
    vignette_correction: bool,
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
            auto_contrast: self.auto_contrast,
            saturation_boost: self.saturation_boost,
            lens_corrections: LensCorrections {
                vignette: self.vignette_correction,
            },
        }
    }
}
