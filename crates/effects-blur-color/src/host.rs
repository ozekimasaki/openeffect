//! 1 フレームの契約に載らない入力。初期値の推測はしない。

use effect_api::{ApplyError, ApplyResult, Frame};
use model::LayerId;

use crate::color_math::Curve;

#[derive(Clone, Copy, Debug)]
pub struct LayerPixels<'a> {
    pub id: LayerId,
    pub frame: &'a Frame,
}

#[derive(Clone, Copy, Debug)]
pub struct CycleStop {
    pub position: f32,
    pub red: f32,
    pub green: f32,
    pub blue: f32,
    pub alpha: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArbitraryMap {
    pub entries: Vec<f32>,
    pub alpha: Option<Vec<f32>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StabilizeRef {
    pub black: [f32; 3],
    pub mid: [f32; 3],
    pub white: [f32; 3],
}

#[derive(Clone, Debug)]
pub struct CurveSet {
    pub master: Curve,
    pub red: Option<Curve>,
    pub green: Option<Curve>,
    pub blue: Option<Curve>,
    pub hue_hue: Option<Curve>,
    pub hue_sat: Option<Curve>,
    pub hue_luma: Option<Curve>,
    pub sat_sat: Option<Curve>,
    pub luma_sat: Option<Curve>,
}

impl CurveSet {
    pub fn master_identity() -> Self {
        Self {
            master: Curve::identity(),
            red: None,
            green: None,
            blue: None,
            hue_hue: None,
            hue_sat: None,
            hue_luma: None,
            sat_sat: None,
            luma_sat: None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Host<'a> {
    pub layers: &'a [LayerPixels<'a>],
    pub original: Option<&'a Frame>,
    pub before: &'a [Frame],
    pub after: &'a [Frame],
    pub frame_seconds: Option<f64>,
    pub curves: Option<&'a CurveSet>,
    pub map: Option<&'a ArbitraryMap>,
    pub cycle: Option<&'a [CycleStop]>,
    pub reference: Option<&'a StabilizeRef>,
    /// シーンカットと判定した近傍。true はそのフレームを時間平滑から外す。
    pub scene_cut: &'a [bool],
}

impl Host<'static> {
    pub const fn empty() -> Self {
        Self {
            layers: &[],
            original: None,
            before: &[],
            after: &[],
            frame_seconds: None,
            curves: None,
            map: None,
            cycle: None,
            reference: None,
            scene_cut: &[],
        }
    }
}

impl<'a> Host<'a> {
    pub fn layer_frame(&self, id: LayerId) -> ApplyResult<&'a Frame> {
        self.layers
            .iter()
            .find(|layer| layer.id == id)
            .map(|layer| layer.frame)
            .ok_or_else(|| ApplyError::new("レイヤーの画素が無い"))
    }
}
