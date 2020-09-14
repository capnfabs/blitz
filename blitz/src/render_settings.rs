pub struct RenderSettings {
    pub tone_curve: [f32; 5],
}

impl Default for RenderSettings {
    fn default() -> Self {
        RenderSettings {
            tone_curve: [1.0, 1.0, 1.0, 1.0, 1.0],
        }
    }
}
