pub struct ScalerConfig {
    pub luminance_weight: f64,
    pub equal_color_tolerance: f64,
    pub center_direction_bias: f64,
    pub dominant_direction_threshold: f64,
    pub steep_direction_threshold: f64,
}

impl Default for ScalerConfig {
    fn default() -> Self {
        Self {
            luminance_weight: 1.0,
            equal_color_tolerance: 30.0,
            center_direction_bias: 4.0,
            dominant_direction_threshold: 3.6,
            steep_direction_threshold: 2.2,
        }
    }
}