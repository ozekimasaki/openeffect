use model::{BitsPerChannel, LayerId, Rgba};

use crate::image::{Audio, Image};

/// CPU 適用の追加入力。効果コントロールの値は別。
#[derive(Clone, Copy)]
pub struct RenderCx<'a> {
    pub time: f64,
    pub frame_rate: f64,
    pub bits: BitsPerChannel,
    pub layers: &'a [(LayerId, &'a Image)],
    pub history: &'a [(f64, &'a Image)],
    pub audio: Option<&'a Audio>,
    pub depth: Option<&'a Image>,
    pub object_id: Option<&'a Image>,
    pub material_id: Option<&'a Image>,
    pub accum: Option<&'a Image>,
    pub lut: Option<&'a [f32]>,
    pub clear: bool,
}

impl<'a> RenderCx<'a> {
    pub fn layer(&self, id: Option<LayerId>) -> Option<&'a Image> {
        let id = id?;
        self.layers
            .iter()
            .find(|(layer, _)| *layer == id)
            .map(|(_, image)| *image)
    }

    pub fn history_at(&self, time: f64) -> Option<&'a Image> {
        self.history
            .iter()
            .min_by(|(a, _), (b, _)| {
                (a - time)
                    .abs()
                    .partial_cmp(&(b - time).abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(_, image)| *image)
    }
}

/// エクスプレッション制御が露出する値。
#[derive(Clone, Debug, PartialEq)]
pub enum Exposed {
    Number(f64),
    Angle(f64),
    Bool(bool),
    Color(Rgba),
    Point([f64; 2]),
    Point3([f64; 3]),
    Index(u32),
    Layer(Option<LayerId>),
    Text(String),
    Numbers(Vec<f64>),
}

pub struct PaintOut {
    pub image: Image,
    pub audio: Option<Audio>,
    pub exposed: Option<Exposed>,
    pub skip_image_finale: bool,
    pub skip_audio_finale: bool,
}
