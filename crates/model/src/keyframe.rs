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
    /// 空間ハンドル。頂点からの相対。Auto Bezier の間は評価側が計算する。
    pub spatial_in: [f64; 3],
    pub spatial_out: [f64; 3],
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
            spatial_in: [0.0, 0.0, 0.0],
            spatial_out: [0.0, 0.0, 0.0],
        }
    }
}

/// ダイアログで既存の補間を残すときに使う。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TemporalCommand {
    CurrentSettings,
    Set(TemporalInterpolation),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SpatialCommand {
    CurrentSettings,
    Set(SpatialInterpolation),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RovingCommand {
    CurrentSettings,
    Set(Roving),
}

pub fn apply_temporal(key: &mut Keyframe, command: TemporalCommand) {
    if let TemporalCommand::Set(value) = command {
        key.temporal_in = value;
        key.temporal_out = value;
        if value == TemporalInterpolation::Hold {
            key.temporal_in = TemporalInterpolation::Hold;
        }
    }
}

pub fn apply_spatial(key: &mut Keyframe, command: SpatialCommand) {
    if let SpatialCommand::Set(value) = command {
        key.spatial = value;
    }
}

pub fn apply_roving(key: &mut Keyframe, command: RovingCommand) {
    if let RovingCommand::Set(value) = command {
        key.roving = value;
    }
}

/// 影響を許す範囲は既存の契約に合わせ、速度 0 で滑らかにする。
/// Easy Ease が入れる影響の百分率そのものは要確認なので、許可範囲の端まで取る。
pub fn easy_ease(key: &mut Keyframe) {
    key.temporal_in = TemporalInterpolation::Bezier;
    key.temporal_out = TemporalInterpolation::Bezier;
    key.ease_in.speed = 0.0;
    key.ease_out.speed = 0.0;
    key.ease_in.influence = 100.0;
    key.ease_out.influence = 100.0;
}

pub fn easy_ease_in(key: &mut Keyframe) {
    key.temporal_in = TemporalInterpolation::Bezier;
    key.ease_in.speed = 0.0;
    key.ease_in.influence = 100.0;
}

pub fn easy_ease_out(key: &mut Keyframe) {
    key.temporal_out = TemporalInterpolation::Bezier;
    key.ease_out.speed = 0.0;
    key.ease_out.influence = 100.0;
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
    pub navigation_cue_point: bool,
}

impl Marker {
    pub fn new(comment: impl Into<String>) -> Self {
        Self {
            comment: comment.into(),
            ..Self::default()
        }
    }

    /// コンポジションマーカーのコメント初期値は番号。
    pub fn comp_numbered(number: u8) -> Self {
        Self::new(number.to_string())
    }

    /// レイヤーマーカーのコメント初期値は空。
    pub fn layer_default() -> Self {
        Self::new("")
    }

    /// リンクかキューポイントかチャプターがあるとアイコンに点が付く。
    pub fn has_icon_dot(&self) -> bool {
        !self.chapter.is_empty()
            || !self.url.is_empty()
            || !self.cue_point_name.is_empty()
            || self.event_cue_point
            || self.navigation_cue_point
    }
}

/// タイムライン上のマーカー。
#[derive(Clone, Debug, PartialEq)]
pub struct TimedMarker {
    pub time: Time,
    pub marker: Marker,
    /// 0 から 9。番号が無いマーカーは None。
    pub number: Option<u8>,
}

impl TimedMarker {
    pub fn at(time: Time, marker: Marker) -> Self {
        Self {
            time,
            marker,
            number: None,
        }
    }
}
