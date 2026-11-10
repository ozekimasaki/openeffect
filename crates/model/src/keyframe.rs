use crate::property::PropertyValue;
use crate::time::Time;

/// 時間補間。Hold は出ていく側に付く。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TemporalInterpolation {
    Linear,
    Bezier,
    ContinuousBezier,
    AutoBezier,
    Hold,
}

/// 空間補間。初期の空間変化は AutoBezier。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SpatialInterpolation {
    Linear,
    Bezier,
    ContinuousBezier,
    AutoBezier,
}

/// キーの時刻を固定するか、速度が滑らかになるよう動かすか。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Roving {
    LockToTime,
    RoveAcrossTime,
}

/// 入来または出発の速度と影響。影響の許容範囲は 0.1 から 100。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KeyframeEase {
    pub speed: f64,
    pub influence: f64,
}

impl KeyframeEase {
    pub const fn new(speed: f64, influence: f64) -> Self {
        Self { speed, influence }
    }
}

impl Default for KeyframeEase {
    fn default() -> Self {
        Self {
            speed: 0.0,
            influence: 0.1,
        }
    }
}

/// プロパティのキーフレーム。
#[derive(Clone, Debug, PartialEq)]
pub struct Keyframe {
    pub time: Time,
    pub value: PropertyValue,
    pub temporal_in: TemporalInterpolation,
    pub temporal_out: TemporalInterpolation,
    pub spatial: SpatialInterpolation,
    pub ease_in: KeyframeEase,
    pub ease_out: KeyframeEase,
    pub roving: Roving,
}

impl Keyframe {
    pub fn at(time: Time, value: PropertyValue) -> Self {
        Self {
            time,
            value,
            temporal_in: TemporalInterpolation::Linear,
            temporal_out: TemporalInterpolation::Linear,
            spatial: SpatialInterpolation::AutoBezier,
            ease_in: KeyframeEase::default(),
            ease_out: KeyframeEase::default(),
            roving: Roving::LockToTime,
        }
    }
}

/// マーカーキーの値。
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Marker {
    pub comment: String,
    pub chapter: String,
    pub cue_point_name: String,
    pub duration_seconds: f64,
    pub event_cue_point: bool,
    pub frame_target: String,
    pub url: String,
    pub label: u8,
    pub protected_region: bool,
    pub parameters: Vec<(String, String)>,
}

impl Marker {
    pub fn new(comment: impl Into<String>) -> Self {
        Self {
            comment: comment.into(),
            ..Self::default()
        }
    }
}
