use crate::blend::BlendMode;
use crate::id::{ItemId, LayerId};
use crate::keyframe::TimedMarker;
use crate::mask::Mask;
use crate::property::Property;
use crate::shape::ShapeContents;
use crate::text::{Text, TextAnimator};
use crate::time::Time;
use crate::track_matte::TrackMatte;
use crate::transform::Transform3d;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LayerQuality {
    Best,
    Draft,
    Wireframe,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Sampling {
    Bilinear,
    Bicubic,
}

/// オフ、Frame Mix、Pixel Motion の順に回る。初期値はオフ。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum FrameBlend {
    #[default]
    Off,
    FrameMix,
    PixelMotion,
}

impl FrameBlend {
    pub const fn cycle(self) -> Self {
        match self {
            Self::Off => Self::FrameMix,
            Self::FrameMix => Self::PixelMotion,
            Self::PixelMotion => Self::Off,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EffectKind {
    Blur { radius: f64 },
    Generic,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LayerEffect {
    pub name: String,
    pub enabled: bool,
    pub kind: EffectKind,
}

/// イン点を動かさず伸ばすか、現在フレームを固定するか、アウト点を固定するか。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StretchHold {
    InPoint,
    CurrentFrame,
    OutPoint,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CameraNodes {
    One,
    Two,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LightKind {
    Parallel,
    Spot,
    Point,
    Ambient,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LayerKind {
    Footage { source: ItemId },
    Solid { source: ItemId },
    Precomp { source: ItemId },
    Null,
    Adjustment,
    Camera { nodes: CameraNodes },
    Light { kind: LightKind },
    Text,
    Shape,
    Guide,
    Placeholder { source: ItemId },
}

/// レイヤー。ブレンドと親子はキーフレームを持たない。
#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    pub id: LayerId,
    pub name: String,
    pub kind: LayerKind,
    pub parent: Option<LayerId>,
    pub blend_mode: BlendMode,
    pub track_matte: TrackMatte,
    pub preserve_underlying_transparency: bool,
    pub collapse_transformations: bool,
    pub motion_blur: bool,
    pub three_d: bool,
    pub masks: Vec<Mask>,
    pub properties: Vec<Property>,
    pub text: Option<Text>,
    pub text_animators: Vec<TextAnimator>,
    pub shape: Option<ShapeContents>,
    pub video: bool,
    pub audio: bool,
    pub solo: bool,
    pub locked: bool,
    pub shy: bool,
    pub guide: bool,
    pub adjustment: bool,
    pub quality: LayerQuality,
    pub sampling: Sampling,
    pub effects_enabled: bool,
    pub frame_blend: FrameBlend,
    pub effects: Vec<LayerEffect>,
    pub comment: String,
    pub label: Option<u8>,
    pub in_point: Time,
    pub out_point: Time,
    pub stretch: f64,
    pub audio_reversed: bool,
    pub markers: Vec<TimedMarker>,
    pub markers_locked: bool,
    pub transform: Transform3d,
    pub opacity: f64,
    /// カメラの焦点距離。初期値は要確認。
    pub focal_length: Option<f64>,
    /// タイムリマップ。None は無効。
    pub time_remap: Option<Vec<crate::keyframe::Keyframe>>,
}

impl Layer {
    pub fn new(id: LayerId, name: impl Into<String>, kind: LayerKind) -> Self {
        Self {
            id,
            name: name.into(),
            kind,
            parent: None,
            blend_mode: BlendMode::Normal,
            track_matte: TrackMatte::none(),
            preserve_underlying_transparency: false,
            collapse_transformations: false,
            motion_blur: false,
            three_d: false,
            masks: Vec::new(),
            properties: Vec::new(),
            text: None,
            text_animators: Vec::new(),
            shape: None,
            video: true,
            audio: true,
            solo: false,
            locked: false,
            shy: false,
            guide: false,
            adjustment: matches!(kind, LayerKind::Adjustment),
            quality: LayerQuality::Best,
            sampling: Sampling::Bilinear,
            effects_enabled: true,
            frame_blend: FrameBlend::Off,
            effects: Vec::new(),
            comment: String::new(),
            label: None,
            in_point: Time::ZERO,
            out_point: Time::ZERO,
            stretch: 1.0,
            audio_reversed: false,
            markers: Vec::new(),
            markers_locked: false,
            transform: Transform3d::identity(),
            opacity: 1.0,
            focal_length: None,
            time_remap: None,
        }
    }

    pub fn is_guide(&self) -> bool {
        self.guide || matches!(self.kind, LayerKind::Guide)
    }

    pub fn is_adjustment(&self) -> bool {
        self.adjustment || matches!(self.kind, LayerKind::Adjustment)
    }

    pub fn is_light(&self) -> bool {
        matches!(self.kind, LayerKind::Light { .. })
    }

    pub fn is_camera(&self) -> bool {
        matches!(self.kind, LayerKind::Camera { .. })
    }

    pub fn is_visual(&self) -> bool {
        !self.is_light() && !self.is_camera() && !matches!(self.kind, LayerKind::Null)
    }

    pub fn source_id(&self) -> Option<ItemId> {
        match self.kind {
            LayerKind::Footage { source }
            | LayerKind::Solid { source }
            | LayerKind::Precomp { source }
            | LayerKind::Placeholder { source } => Some(source),
            LayerKind::Null
            | LayerKind::Adjustment
            | LayerKind::Camera { .. }
            | LayerKind::Light { .. }
            | LayerKind::Text
            | LayerKind::Shape
            | LayerKind::Guide => None,
        }
    }

    pub fn duration(&self) -> Time {
        Time::from_seconds(self.out_point.seconds() - self.in_point.seconds())
    }

    /// コンポジション時刻を、ストレッチと時間反転を考慮したソース時刻へ写す。
    pub fn source_time(&self, comp_time: Time, source_duration: Time) -> Time {
        let local = comp_time.seconds() - self.in_point.seconds();
        let speed = self.stretch.abs();
        let traveled = if speed == 0.0 { 0.0 } else { local / speed };
        if self.stretch < 0.0 {
            Time::from_seconds(source_duration.seconds() - traveled)
        } else {
            Time::from_seconds(traveled)
        }
    }

    pub fn played_audio_duration(&self) -> Time {
        self.duration()
    }

    pub fn enable_time_remap(&mut self, source_duration: Time) {
        let head = crate::keyframe::Keyframe::at(self.in_point, crate::property::PropertyValue::Scalar(0.0));
        let mut tail = crate::keyframe::Keyframe::at(
            self.out_point,
            crate::property::PropertyValue::Scalar(source_duration.seconds()),
        );
        tail.temporal_out = crate::keyframe::TemporalInterpolation::Linear;
        self.time_remap = Some(vec![head, tail]);
    }

    /// 末尾キーをソース先頭へ、先頭キーをソース末尾へ置き、逆再生にする。
    pub fn reverse_time_remap(&mut self) {
        let Some(keys) = &mut self.time_remap else {
            return;
        };
        if keys.len() < 2 {
            return;
        }
        let head = match keys[0].value {
            crate::property::PropertyValue::Scalar(value) => value,
            _ => 0.0,
        };
        let tail = match keys[1].value {
            crate::property::PropertyValue::Scalar(value) => value,
            _ => 0.0,
        };
        keys[0].value = crate::property::PropertyValue::Scalar(tail);
        keys[1].value = crate::property::PropertyValue::Scalar(head);
    }

    pub fn freeze_frame(&mut self, comp_time: Time, source_time: Time) {
        let mut key = crate::keyframe::Keyframe::at(
            comp_time,
            crate::property::PropertyValue::Scalar(source_time.seconds()),
        );
        key.temporal_out = crate::keyframe::TemporalInterpolation::Hold;
        key.temporal_in = crate::keyframe::TemporalInterpolation::Hold;
        self.time_remap = Some(vec![key]);
    }

    pub fn freeze_on_last_frame(&mut self, source_duration: Time, comp_end: Time) {
        let mut key = crate::keyframe::Keyframe::at(
            self.out_point,
            crate::property::PropertyValue::Scalar(source_duration.seconds()),
        );
        key.temporal_out = crate::keyframe::TemporalInterpolation::Hold;
        self.time_remap = Some(vec![key]);
        self.out_point = comp_end;
    }

    pub fn stretch_time(&mut self, factor: f64, hold: StretchHold, now: Time) {
        let old_in = self.in_point.seconds();
        let old_out = self.out_point.seconds();
        let old_span = old_out - old_in;
        let new_span = old_span * factor.abs();
        let sign = if self.stretch < 0.0 { -1.0 } else { 1.0 };
        self.stretch = sign * factor;
        match hold {
            StretchHold::InPoint => {
                self.out_point = Time::from_seconds(old_in + new_span);
            }
            StretchHold::OutPoint => {
                self.in_point = Time::from_seconds(old_out - new_span);
            }
            StretchHold::CurrentFrame => {
                let portion = if old_span == 0.0 {
                    0.0
                } else {
                    (now.seconds() - old_in) / old_span
                };
                self.in_point = Time::from_seconds(now.seconds() - portion * new_span);
                self.out_point = Time::from_seconds(self.in_point.seconds() + new_span);
            }
        }
        let origin = match hold {
            StretchHold::InPoint => old_in,
            StretchHold::OutPoint => old_out,
            StretchHold::CurrentFrame => now.seconds(),
        };
        scale_property_keys(&mut self.properties, origin, factor.abs());
        if let Some(keys) = &mut self.time_remap {
            scale_keys(keys, origin, factor.abs());
        }
    }

    pub fn time_reverse(&mut self) {
        self.stretch = -self.stretch.abs();
        self.audio_reversed = !self.audio_reversed;
    }

    pub fn property(&self, match_name: &str) -> Option<&Property> {
        self.properties.iter().find_map(|property| {
            if property.match_name == match_name {
                Some(property)
            } else {
                property.child(match_name)
            }
        })
    }

    pub fn property_mut(&mut self, match_name: &str) -> Option<&mut Property> {
        for property in &mut self.properties {
            if property.match_name == match_name {
                return Some(property);
            }
            if property.child(match_name).is_some() {
                return property.child_mut(match_name);
            }
        }
        None
    }
}

fn scale_property_keys(properties: &mut [crate::property::Property], origin: f64, factor: f64) {
    for property in properties {
        match &mut property.body {
            crate::property::PropertyBody::Group { children } => {
                scale_property_keys(children, origin, factor);
            }
            crate::property::PropertyBody::Value { keyframes, .. } => {
                scale_keys(keyframes, origin, factor);
            }
        }
    }
}

fn scale_keys(keys: &mut [crate::keyframe::Keyframe], origin: f64, factor: f64) {
    for key in keys {
        let shifted = origin + (key.time.seconds() - origin) * factor;
        key.time = Time::from_seconds(shifted);
    }
}
