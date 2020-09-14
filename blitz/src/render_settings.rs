#[derive(Debug, Clone)]
pub struct RenderSettings {
    pub tone_curve: Vec<f32>,
}

impl Default for RenderSettings {
    fn default() -> Self {
        RenderSettings {
            tone_curve: vec![1.5],
        }
    }
}
