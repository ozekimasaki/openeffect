use crate::id::ItemId;
use crate::layer::Layer;
use crate::time::Time;

/// モーションブラーのシャッター。角度の単位は度。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MotionBlurSettings {
    pub shutter_angle_degrees: f64,
    pub shutter_phase_degrees: f64,
    pub samples_per_frame: u32,
    pub adaptive_sample_limit: u32,
}

impl Default for MotionBlurSettings {
    fn default() -> Self {
        Self {
            shutter_angle_degrees: 0.0,
            shutter_phase_degrees: 0.0,
            samples_per_frame: 1,
            adaptive_sample_limit: 1,
        }
    }
}

/// コンポジション。`layers[0]` がタイムラインの先頭（手前）。
#[derive(Clone, Debug, PartialEq)]
pub struct Composition {
    pub id: ItemId,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub pixel_aspect: f64,
    pub frame_rate: f64,
    pub duration: Time,
    pub work_area_start: Time,
    pub work_area_duration: Time,
    pub layers: Vec<Layer>,
    pub motion_blur: MotionBlurSettings,
}

impl Composition {
    pub fn new(
        id: ItemId,
        name: impl Into<String>,
        width: u32,
        height: u32,
        frame_rate: f64,
        duration: Time,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            width,
            height,
            pixel_aspect: 1.0,
            frame_rate,
            duration,
            work_area_start: Time::ZERO,
            work_area_duration: duration,
            layers: Vec::new(),
            motion_blur: MotionBlurSettings::default(),
        }
    }

    pub fn layer(&self, id: crate::id::LayerId) -> Option<&Layer> {
        self.layers.iter().find(|layer| layer.id == id)
    }

    pub fn layer_mut(&mut self, id: crate::id::LayerId) -> Option<&mut Layer> {
        self.layers.iter_mut().find(|layer| layer.id == id)
    }
}
