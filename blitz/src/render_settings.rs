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
