use model::{MotionBlurSettings, Time};

/// モーションブラーの入力。角度の単位は度。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MotionBlurSample {
    pub enabled: bool,
    pub shutter_angle_degrees: f64,
    pub shutter_phase_degrees: f64,
    pub samples_per_frame: u32,
    pub adaptive_sample_limit: u32,
}

impl MotionBlurSample {
    pub fn from_settings(layer_enabled: bool, settings: &MotionBlurSettings) -> Self {
        Self {
            enabled: layer_enabled,
            shutter_angle_degrees: settings.shutter_angle_degrees,
            shutter_phase_degrees: settings.shutter_phase_degrees,
            samples_per_frame: settings.samples_per_frame,
            adaptive_sample_limit: settings.adaptive_sample_limit,
        }
    }
}

impl Default for MotionBlurSample {
    fn default() -> Self {
        Self::from_settings(false, &MotionBlurSettings::default())
    }
}

/// モーションブラーの入力を受ける。既定はそのまま返す。
pub trait MotionBlurEval {
    fn with_motion_blur(&self, time: Time, blur: &MotionBlurSample) -> MotionBlurSample {
        let _ = (self, time);
        *blur
    }
}
