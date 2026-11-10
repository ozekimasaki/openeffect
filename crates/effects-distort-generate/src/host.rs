use std::collections::BTreeMap;

use effect_api::{
    ApplyError, ApplyResult, CpuApply, CpuInput, Frame, ParamList, ParamValue, ParamValues,
};
use model::{BitsPerChannel, BlendMode, LayerId, MaskMode, Rgba, Time};

use crate::image::Image;
use crate::render::render;
use crate::spec::{parameters, EffectKind, Keyframe, ParamKind, ParamSpec};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Quality {
    Best,
    Draft,
    Wireframe,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Number(f64),
    Bool(bool),
    Color(Rgba),
    Point([f64; 2]),
    Index(u32),
    Layer(Option<LayerId>),
}

#[derive(Clone, Debug)]
pub struct MaskShape {
    pub name: String,
    pub path: Vec<[f64; 2]>,
    pub closed: bool,
    pub mode: MaskMode,
    pub opacity: f64,
    pub feather: f64,
    pub order: u32,
}

impl MaskShape {
    pub fn polygon(name: impl Into<String>, path: Vec<[f64; 2]>, order: u32) -> Self {
        Self {
            name: name.into(),
            path,
            closed: true,
            mode: MaskMode::Add,
            opacity: 1.0,
            feather: 0.0,
            order,
        }
    }
}

#[derive(Clone, Debug)]
pub struct AudioClip {
    pub sample_rate: u32,
    pub channels: u8,
    pub samples: Vec<f32>,
    /// タイムリマップ、効果、ストレッチ、レベルを通したあとの音声。効果は読まない。
    pub processed: Vec<f32>,
}

#[derive(Clone, Debug)]
pub struct SourceLayer {
    pub id: LayerId,
    pub frame: Frame,
    pub masks: Vec<MaskShape>,
    pub audio: Option<AudioClip>,
    /// プリコンポーズしたレイヤーだけ、効果とマスクを焼き込んだフレームを渡す。
    pub precomposed: bool,
}

/// 手ブレ補正とローリングシャッター用の、時刻ごとの動き。
#[derive(Clone, Debug)]
pub struct MotionSample {
    pub time: f64,
    pub translation: [f64; 2],
    pub points: u32,
    /// 行ごとの横ずれ。ローリングシャッターの入力。
    pub row_shift: Vec<f32>,
}

#[derive(Clone, Debug)]
pub struct TrackObs {
    pub time: f64,
    pub id: u32,
    pub xy: [f64; 2],
}

#[derive(Clone, Debug)]
pub struct Host {
    pub quality: Quality,
    pub comp_width: u32,
    pub comp_height: u32,
    pub layer_motion_blur: bool,
    pub comp_motion_blur: bool,
    pub comp_shutter: f64,
    pub masks: Vec<MaskShape>,
    pub layers: Vec<SourceLayer>,
    pub audio: Option<AudioClip>,
    pub motion: Vec<MotionSample>,
    pub tracks: Vec<TrackObs>,
    pub frame_rate: f64,
    /// 表示名が要確認のシャッター角度（度）。
    pub shutter: Option<f64>,
}

impl Default for Host {
    fn default() -> Self {
        Self {
            quality: Quality::Best,
            comp_width: 0,
            comp_height: 0,
            layer_motion_blur: false,
            comp_motion_blur: false,
            comp_shutter: 180.0,
            masks: Vec::new(),
            layers: Vec::new(),
            audio: None,
            motion: Vec::new(),
            tracks: Vec::new(),
            frame_rate: 24.0,
            shutter: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LiquifyTool {
    Warp,
    Turbulence,
    TwirlClockwise,
    TwirlCounterclockwise,
    Pucker,
    Bloat,
    ShiftPixels,
    Reflection,
    Clone,
    Reconstruction,
}

#[derive(Clone, Debug)]
pub struct LiquifyStroke {
    pub tool: LiquifyTool,
    pub from: [f64; 2],
    pub to: [f64; 2],
    pub hold: f64,
}

#[derive(Clone, Debug, Default)]
pub struct Analysis {
    pub running: bool,
    pub done: bool,
    pub cancelled: bool,
    pub camera_ready: bool,
    pub camera_created: bool,
    pub camera_zoom: Option<f64>,
    pub camera_path: Vec<[f64; 3]>,
    pub stab_method: Option<u32>,
    pub stab_shift: Vec<[f64; 2]>,
    pub warning: bool,
    pub deleted_track_ids: Vec<u32>,
}

#[derive(Clone, Debug)]
pub struct MeshPoint {
    pub col: u32,
    pub row: u32,
    pub at: [f64; 2],
    pub selected: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Runtime {
    pub strokes: Vec<LiquifyStroke>,
    pub clone_source: Option<[f64; 2]>,
    pub pins: Vec<[f64; 2]>,
    pub edges: Vec<([f64; 2], [f64; 2], [f64; 2])>,
    pub mesh: Vec<MeshPoint>,
    pub pairs: Vec<([f64; 2], [f64; 2])>,
    pub analysis: Analysis,
    pub flare_center: Option<[f64; 2]>,
}

#[derive(Clone, Debug)]
pub struct Instance {
    pub kind: EffectKind,
    values: BTreeMap<String, Value>,
    roles: BTreeMap<String, Value>,
    keys: BTreeMap<String, Vec<(f64, Value)>>,
    pub host: Host,
    pub runtime: Runtime,
    pressed: BTreeMap<String, u32>,
}

impl Instance {
    pub fn new(kind: EffectKind) -> Self {
        Self {
            kind,
            values: BTreeMap::new(),
            roles: BTreeMap::new(),
            keys: BTreeMap::new(),
            host: Host::default(),
            runtime: Runtime::default(),
            pressed: BTreeMap::new(),
        }
    }

    pub fn spec(&self) -> Vec<ParamSpec> {
        parameters(self.kind)
    }

    pub fn param(&self, name: &str) -> Option<ParamSpec> {
        self.spec().into_iter().find(|param| param.name == name)
    }

    pub fn set(&mut self, name: &str, value: Value) -> ApplyResult<()> {
        if self.param(name).is_none() {
            return Err(ApplyError::new(format!(
                "{} has no confirmed parameter {name}",
                self.kind.name()
            )));
        }
        self.values.insert(name.to_string(), value);
        Ok(())
    }

    pub fn set_number(&mut self, name: &str, value: f64) -> ApplyResult<()> {
        self.set(name, Value::Number(value))
    }

    pub fn set_bool(&mut self, name: &str, value: bool) -> ApplyResult<()> {
        self.set(name, Value::Bool(value))
    }

    pub fn set_point(&mut self, name: &str, x: f64, y: f64) -> ApplyResult<()> {
        self.set(name, Value::Point([x, y]))
    }

    pub fn set_color(&mut self, name: &str, color: Rgba) -> ApplyResult<()> {
        self.set(name, Value::Color(color))
    }

    pub fn set_choice(&mut self, name: &str, choice: &str) -> ApplyResult<()> {
        let spec = self.param(name).ok_or_else(|| {
            ApplyError::new(format!(
                "{} has no confirmed parameter {name}",
                self.kind.name()
            ))
        })?;
        let index = spec
            .choices
            .iter()
            .position(|item| item == choice)
            .ok_or_else(|| ApplyError::new(format!("{name} has no choice {choice}")))?;
        self.set(name, Value::Index(index as u32))
    }

    pub fn set_layer(&mut self, name: &str, id: Option<LayerId>) -> ApplyResult<()> {
        self.set(name, Value::Layer(id))
    }

    /// 表示名が確定していない役割。キーは Effect Controls の項目名ではない。
    pub fn set_role(&mut self, role: &str, value: Value) {
        self.roles.insert(role.to_string(), value);
    }

    pub fn set_blend(&mut self, name: &str, mode: Option<BlendMode>) {
        let index = match mode {
            None => u32::MAX,
            Some(mode) => BlendMode::ALL
                .iter()
                .position(|item| *item == mode)
                .unwrap_or(0) as u32,
        };
        if self.param(name).is_some() {
            self.values.insert(name.to_string(), Value::Index(index));
        } else {
            self.roles.insert(name.to_string(), Value::Index(index));
        }
    }

    pub fn add_key(&mut self, name: &str, time: f64, value: Value) {
        self.keys
            .entry(name.to_string())
            .or_default()
            .push((time, value));
        self.keys
            .get_mut(name)
            .unwrap()
            .sort_by(|a, b| a.0.total_cmp(&b.0));
    }

    pub fn press(&mut self, name: &str) -> ApplyResult<()> {
        let spec = self
            .param(name)
            .ok_or_else(|| ApplyError::new(format!("{} has no button {name}", self.kind.name())))?;
        if spec.kind != ParamKind::Button {
            return Err(ApplyError::new(format!("{name} is not a button")));
        }
        if name == "Cancel" && !self.runtime.analysis.running {
            return Err(ApplyError::new("Cancel is only available during analysis"));
        }
        *self.pressed.entry(name.to_string()).or_insert(0) += 1;
        match (self.kind, name) {
            (EffectKind::DetailPreservingUpscale, "Fit to Comp Width") => {
                self.roles.insert("fit".into(), Value::Index(0));
            }
            (EffectKind::DetailPreservingUpscale, "Fit to Comp Height") => {
                self.roles.insert("fit".into(), Value::Index(1));
            }
            (EffectKind::WarpStabilizer, "Analyze") | (EffectKind::CameraTracker, "Analyze") => {
                self.runtime.analysis.running = true;
                self.runtime.analysis.cancelled = false;
                self.runtime.analysis.done = false;
                self.runtime.analysis.camera_ready = false;
            }
            (EffectKind::WarpStabilizer, "Cancel") | (EffectKind::CameraTracker, "Cancel") => {
                self.runtime.analysis.running = false;
                self.runtime.analysis.cancelled = true;
            }
            (EffectKind::CameraTracker, "Create Camera") => {
                if !self.runtime.analysis.camera_ready {
                    return Err(ApplyError::new("Create Camera waits for analysis"));
                }
                self.runtime.analysis.camera_created = true;
            }
            _ => {}
        }
        Ok(())
    }

    pub fn press_count(&self, name: &str) -> u32 {
        self.pressed.get(name).copied().unwrap_or(0)
    }

    pub fn finish_analysis(&mut self) {
        if self.runtime.analysis.cancelled {
            self.runtime.analysis.running = false;
            return;
        }
        self.runtime.analysis.running = false;
        self.runtime.analysis.done = true;
        if self.kind == EffectKind::CameraTracker {
            self.runtime.analysis.camera_ready = true;
            self.runtime.analysis.camera_path = solve_camera(&self.host.tracks);
            self.runtime.analysis.camera_zoom = self.num("Horizontal Angle of View");
        }
        if self.kind == EffectKind::WarpStabilizer {
            let (method, shift) = solve_stabilizer(self);
            self.runtime.analysis.stab_method = Some(method);
            self.runtime.analysis.stab_shift = shift;
            self.runtime.analysis.warning = self.host.motion.iter().any(|sample| {
                sample.translation[0].abs() > 40.0 || sample.translation[1].abs() > 40.0
            });
        }
    }

    pub fn number_at(&self, name: &str, time: f64) -> Option<f64> {
        if let Some(value) = self.keyed(name, time) {
            return as_number(&value);
        }
        self.values
            .get(name)
            .and_then(as_number)
            .or_else(|| self.param(name).and_then(|spec| spec.default_number))
    }

    pub fn num(&self, name: &str) -> Option<f64> {
        self.number_at(name, 0.0)
    }

    pub fn flag_at(&self, name: &str, time: f64) -> Option<bool> {
        if let Some(Value::Bool(value)) = self.keyed(name, time) {
            return Some(value);
        }
        match self.values.get(name) {
            Some(Value::Bool(value)) => Some(*value),
            _ => self.param(name).and_then(|spec| spec.default_bool),
        }
    }

    pub fn flag(&self, name: &str) -> Option<bool> {
        self.flag_at(name, 0.0)
    }

    pub fn index_at(&self, name: &str, time: f64) -> Option<u32> {
        if let Some(Value::Index(value)) = self.keyed(name, time) {
            return Some(value);
        }
        match self.values.get(name) {
            Some(Value::Index(value)) => Some(*value),
            _ => self.param(name).and_then(|spec| spec.default_index),
        }
    }

    pub fn choice(&self, name: &str) -> Option<String> {
        let index = self.index_at(name, 0.0)?;
        self.param(name)?.choices.get(index as usize).cloned()
    }

    pub fn point_at(&self, name: &str, time: f64) -> Option<[f64; 2]> {
        if let Some(Value::Point(value)) = self.keyed(name, time) {
            return Some(value);
        }
        match self.values.get(name) {
            Some(Value::Point(value)) => Some(*value),
            _ => None,
        }
    }

    pub fn color_at(&self, name: &str, time: f64) -> Option<Rgba> {
        if let Some(Value::Color(value)) = self.keyed(name, time) {
            return Some(value);
        }
        match self.values.get(name) {
            Some(Value::Color(value)) => Some(*value),
            _ => None,
        }
    }

    pub fn layer_value(&self, name: &str) -> Option<Option<LayerId>> {
        match self.values.get(name) {
            Some(Value::Layer(value)) => Some(*value),
            _ => None,
        }
    }

    pub fn role_number(&self, role: &str) -> Option<f64> {
        self.roles.get(role).and_then(as_number)
    }

    pub fn role_point(&self, role: &str) -> Option<[f64; 2]> {
        match self.roles.get(role) {
            Some(Value::Point(value)) => Some(*value),
            _ => None,
        }
    }

    pub fn role_index(&self, role: &str) -> Option<u32> {
        match self.roles.get(role) {
            Some(Value::Index(value)) => Some(*value),
            _ => None,
        }
    }

    pub fn role_color(&self, role: &str) -> Option<Rgba> {
        match self.roles.get(role) {
            Some(Value::Color(value)) => Some(*value),
            _ => None,
        }
    }

    pub fn role_bool(&self, role: &str) -> Option<bool> {
        match self.roles.get(role) {
            Some(Value::Bool(value)) => Some(*value),
            _ => None,
        }
    }

    pub fn blend_of(&self, name: &str) -> Option<BlendMode> {
        let index = self.index_at(name, 0.0).or_else(|| self.role_index(name))?;
        if index == u32::MAX {
            return None;
        }
        BlendMode::ALL.get(index as usize).copied()
    }

    /// 未設定のとき、解説が初期値を書いている合成だけを返す。
    pub fn blend_or_none(&self, name: &str) -> Option<BlendMode> {
        if self.values.contains_key(name) || self.roles.contains_key(name) {
            return self.blend_of(name);
        }
        if matches!(self.kind, EffectKind::Checkerboard | EffectKind::Grid)
            && name == "Blending Mode"
        {
            return None;
        }
        self.blend_of(name)
    }

    pub fn enabled(&self, name: &str) -> bool {
        enabled(self, name)
    }

    pub fn keys_of(&self, name: &str) -> Vec<(f64, Value)> {
        self.keys.get(name).cloned().unwrap_or_default()
    }

    pub fn keyframe_of(&self, name: &str) -> Option<Keyframe> {
        self.param(name).map(|param| param.keyframe)
    }

    fn keyed(&self, name: &str, time: f64) -> Option<Value> {
        let keys = self.keys.get(name)?;
        if keys.is_empty() {
            return None;
        }
        let method = self.index_at("Interpolation Method", time);
        let discrete = method == Some(0) && self.kind == EffectKind::Reshape
            || method == Some(1) && self.kind == EffectKind::Smear;
        if discrete {
            return keys
                .iter()
                .rev()
                .find(|(key_time, _)| (*key_time - time).abs() < 1e-6)
                .map(|(_, value)| value.clone())
                .or_else(|| keys.last().map(|(_, value)| value.clone()));
        }
        let smooth = method == Some(2);
        interpolate(keys, time, smooth)
    }
}

fn as_number(value: &Value) -> Option<f64> {
    match value {
        Value::Number(value) => Some(*value),
        Value::Index(value) => Some(f64::from(*value)),
        Value::Bool(value) => Some(if *value { 1.0 } else { 0.0 }),
        _ => None,
    }
}

fn interpolate(keys: &[(f64, Value)], time: f64, smooth: bool) -> Option<Value> {
    if keys.len() == 1 {
        return Some(keys[0].1.clone());
    }
    if time <= keys[0].0 {
        return Some(keys[0].1.clone());
    }
    if time >= keys[keys.len() - 1].0 {
        return Some(keys[keys.len() - 1].1.clone());
    }
    let mut index = 0;
    while index + 1 < keys.len() && keys[index + 1].0 < time {
        index += 1;
    }
    let (t0, v0) = &keys[index];
    let (t1, v1) = &keys[index + 1];
    let mut span = if (*t1 - *t0).abs() < 1e-9 {
        0.0
    } else {
        (time - t0) / (t1 - t0)
    };
    if smooth && keys.len() >= 3 {
        span = span * span * (3.0 - 2.0 * span);
    }
    match (v0, v1) {
        (Value::Number(a), Value::Number(b)) => Some(Value::Number(a + (b - a) * span)),
        (Value::Point(a), Value::Point(b)) => Some(Value::Point([
            a[0] + (b[0] - a[0]) * span,
            a[1] + (b[1] - a[1]) * span,
        ])),
        (Value::Color(a), Value::Color(b)) => Some(Value::Color(Rgba::new(
            lerp_f(a.r, b.r, span),
            lerp_f(a.g, b.g, span),
            lerp_f(a.b, b.b, span),
            lerp_f(a.a, b.a, span),
        ))),
        _ => Some(if span < 0.5 { v0.clone() } else { v1.clone() }),
    }
}

fn lerp_f(a: f32, b: f32, t: f64) -> f32 {
    a + (b - a) * t as f32
}

fn enabled(inst: &Instance, name: &str) -> bool {
    let choice = |param: &str| inst.choice(param);
    match (inst.kind, name) {
        (EffectKind::Magnify, "Resize Layer") => {
            choice("Link").as_deref() == Some("None") || inst.index_at("Link", 0.0).is_none()
        }
        (EffectKind::OpticsCompensation, "Resize") => {
            inst.flag("Reverse Lens Distortion") == Some(true)
        }
        (EffectKind::RollingShutterRepair, "Detailed Analysis") => {
            choice("Method").as_deref() == Some("Warp")
        }
        (EffectKind::RollingShutterRepair, "Pixel Motion Detail") => {
            choice("Method").as_deref() == Some("Pixel Motion")
        }
        (EffectKind::AdvancedLightning, "Direction") => matches!(
            choice("Lightning Type").as_deref(),
            Some("Direction" | "Strike" | "Breaking" | "Bouncy" | "Two-Way Strike")
        ),
        (EffectKind::AdvancedLightning, "Outer Radius") => {
            matches!(
                choice("Lightning Type").as_deref(),
                Some("Omni" | "Anywhere")
            )
        }
        (EffectKind::AdvancedLightning, "Main Core Collision Only") => {
            inst.num("Alpha Obstacle").unwrap_or(0.0) != 0.0
        }
        (EffectKind::AudioSpectrum | EffectKind::AudioWaveform, "Start Point" | "End Point") => {
            inst.role_index("path").unwrap_or(0) == 0
        }
        (EffectKind::AudioSpectrum, "Dynamic Hue Phase" | "Color Symmetry") => {
            inst.num("Hue Interpolation").unwrap_or(0.0) > 0.0
        }
        (EffectKind::CellPattern, "Overflow") => !cell_is_sharp(inst),
        (EffectKind::Scribble, "Mask") => choice("Scribble").as_deref() == Some("Single Mask"),
        (EffectKind::Scribble, "End Cap") => inst.role_index("fill_type") == Some(2),
        (EffectKind::Scribble, "Miter Limit") => choice("Join").as_deref() == Some("Miter"),
        (EffectKind::Scribble, "Fill Paths Sequentially") => {
            choice("Scribble").as_deref() == Some("All Masks")
        }
        (EffectKind::Scribble, "Wiggles/Second") => {
            matches!(choice("Wiggle Type").as_deref(), Some("Smooth" | "Jumpy"))
        }
        (EffectKind::PaintBucket, "Feather Softness") => {
            choice("Stroke").as_deref() == Some("Feather")
        }
        (EffectKind::PaintBucket, "Spread Radius") => {
            matches!(choice("Stroke").as_deref(), Some("Spread" | "Choke"))
        }
        (EffectKind::PaintBucket, "Stroke Width") => choice("Stroke").as_deref() == Some("Stroke"),
        (EffectKind::Vegas, "Input Layer") => choice("Stroke").as_deref() == Some("Image Contours"),
        (EffectKind::WarpStabilizer, "Smoothness") => {
            choice("Result").as_deref() != Some("No Motion")
        }
        (EffectKind::WarpStabilizer, "Crop Less <-> Smooth More") => {
            choice("Result").as_deref() != Some("No Motion")
                && !matches!(
                    choice("Framing").as_deref(),
                    Some("Stabilize Only" | "Stabilize, Synthesize Edges")
                )
        }
        (EffectKind::WarpStabilizer, "Maximum Scale" | "Action-Safe Margin") => {
            choice("Framing").as_deref() == Some("Stabilize, Crop, Auto-scale")
        }
        (EffectKind::WarpStabilizer, "Rolling Shutter Ripple") => matches!(
            choice("Method").as_deref(),
            Some("Subspace Warp" | "Perspective")
        ),
        (
            EffectKind::WarpStabilizer,
            "Synthesis Edge Feather" | "Synthesis Input Range (seconds)",
        ) => choice("Framing").as_deref() == Some("Stabilize, Synthesize Edges"),
        (EffectKind::WarpStabilizer, "Target Layer") => matches!(
            choice("Objective").as_deref(),
            Some("Apply Motion to Target" | "Apply Motion to Target over Original")
        ),
        (EffectKind::Glasses3d, "Convergence Offset") => {
            choice("3D View").as_deref() != Some("Stereo Pair (Side by Side)")
        }
        (EffectKind::Glasses3d, "Units") => !matches!(
            choice("3D View").as_deref(),
            Some("Stereo Pair (Side by Side)" | "Over Under")
        ),
        (EffectKind::Glasses3d, "Balance") => matches!(
            choice("3D View").as_deref(),
            Some("Balanced Red Green LR" | "Balanced Red Blue LR" | "Balanced Colored Red Blue")
        ),
        (EffectKind::RadioWaves, "Sides" | "Curve Size" | "Curvyness" | "Star" | "Star Depth") => {
            inst.role_index("wave_type").unwrap_or(0) == 0
        }
        (EffectKind::RadioWaves, "Mask") => inst.role_index("wave_type") == Some(1),
        (
            EffectKind::RadioWaves,
            "Source Layer" | "Source Center" | "Value Channel" | "Invert Input" | "Value Threshold"
            | "Pre-Blur" | "Tolerance" | "Contour",
        ) => inst.role_index("wave_type") == Some(2),
        _ => true,
    }
}

pub fn cell_pattern_index(inst: &Instance) -> u32 {
    inst.role_index("pattern").unwrap_or(0)
}

pub fn cell_is_sharp(inst: &Instance) -> bool {
    matches!(cell_pattern_index(inst), 5 | 6 | 7)
}

fn solve_camera(tracks: &[TrackObs]) -> Vec<[f64; 3]> {
    let mut times: Vec<f64> = tracks.iter().map(|track| track.time).collect();
    times.sort_by(f64::total_cmp);
    times.dedup();
    if times.is_empty() {
        return Vec::new();
    }
    let mut path = Vec::new();
    let origin = tracks
        .iter()
        .find(|track| track.time == times[0])
        .map(|track| track.xy)
        .unwrap_or([0.0, 0.0]);
    for time in times {
        let mut acc = [0.0; 2];
        let mut count = 0.0;
        for track in tracks.iter().filter(|track| track.time == time) {
            acc[0] += track.xy[0];
            acc[1] += track.xy[1];
            count += 1.0;
        }
        if count > 0.0 {
            path.push([
                -(acc[0] / count - origin[0]),
                -(acc[1] / count - origin[1]),
                0.0,
            ]);
        }
    }
    path
}

fn solve_stabilizer(inst: &Instance) -> (u32, Vec<[f64; 2]>) {
    let requested = inst.index_at("Method", 0.0).unwrap_or(3);
    let points = inst
        .host
        .motion
        .iter()
        .map(|sample| sample.points)
        .min()
        .unwrap_or(0);
    let method = fallback_method(requested, points);
    let smoothness = if inst.choice("Result").as_deref() == Some("No Motion") {
        1.0
    } else {
        (inst.num("Smoothness").unwrap_or(0.0) / 100.0).clamp(0.0, 4.0)
    };
    let mut raw: Vec<[f64; 2]> = inst
        .host
        .motion
        .iter()
        .map(|sample| sample.translation)
        .collect();
    if raw.is_empty() {
        raw.push([0.0, 0.0]);
    }
    let mean = if inst.choice("Result").as_deref() == Some("No Motion") {
        let n = raw.len() as f64;
        let sum = raw
            .iter()
            .fold([0.0, 0.0], |acc, item| [acc[0] + item[0], acc[1] + item[1]]);
        [sum[0] / n, sum[1] / n]
    } else {
        let mut smooth = raw.clone();
        let passes = (smoothness * 8.0).round() as u32;
        for _ in 0..passes {
            let prev = smooth.clone();
            for i in 1..smooth.len().saturating_sub(1) {
                smooth[i][0] = (prev[i - 1][0] + prev[i][0] + prev[i + 1][0]) / 3.0;
                smooth[i][1] = (prev[i - 1][1] + prev[i][1] + prev[i + 1][1]) / 3.0;
            }
        }
        return (
            method,
            raw.iter()
                .zip(smooth.iter())
                .map(|(sample, target)| [sample[0] - target[0], sample[1] - target[1]])
                .collect(),
        );
    };
    (
        method,
        raw.iter()
            .map(|sample| [sample[0] - mean[0], sample[1] - mean[1]])
            .collect(),
    )
}

pub fn fallback_method(requested: u32, points: u32) -> u32 {
    let mut method = requested.min(3);
    let needed = [1, 2, 4, 8];
    while method > 0 && points < needed[method as usize] {
        method -= 1;
    }
    method
}

pub fn api_params(kind: EffectKind) -> ParamList {
    let mut list = ParamList::new();
    for param in parameters(kind) {
        if !param.default_confirmed {
            continue;
        }
        match param.kind {
            ParamKind::Number | ParamKind::Integer | ParamKind::Percent
                if param.min.is_some() && param.max.is_some() && param.default_number.is_some() =>
            {
                list.define_float(
                    param.name,
                    param.default_number.unwrap_or(0.0),
                    param.min.unwrap_or(0.0),
                    param.max.unwrap_or(0.0),
                );
            }
            ParamKind::Popup if param.choices_complete && param.default_index.is_some() => {
                list.define_popup(param.name, param.choices, param.default_index.unwrap_or(0));
            }
            ParamKind::Angle if param.default_number.is_some() => {
                list.define_angle(param.name, param.default_number.unwrap_or(0.0));
            }
            ParamKind::Checkbox if param.default_bool.is_some() => {
                list.define_checkbox(param.name, param.default_bool.unwrap_or(false));
            }
            _ => {}
        }
    }
    list
}

pub fn apply_instance(
    inst: &Instance,
    input: &CpuInput<'_>,
    output: &mut Frame,
) -> ApplyResult<()> {
    if !inst.kind.bits().contains(&input.bits) {
        return Err(ApplyError::new(format!(
            "{} does not support {} bpc",
            inst.kind.name(),
            input.bits.bits()
        )));
    }
    let mut local = inst.clone();
    apply_param_values(&mut local, input.params);
    let src = Image::from_frame(input.frame);
    let mut image = render(&local, &src, input.time.seconds())?;
    image.bits = input.bits;
    image.channels = src.channels;
    *output = image.to_frame();
    Ok(())
}

fn apply_param_values(inst: &mut Instance, params: &effect_api::ParamValues) {
    let list = api_params(inst.kind);
    if params.len() != list.len() || list.is_empty() {
        return;
    }
    for (index, param) in list.iter() {
        let Some(value) = params.get(index) else {
            continue;
        };
        let converted = match value {
            ParamValue::Float(value) | ParamValue::Angle(value) => Value::Number(*value),
            ParamValue::Checkbox(value) => Value::Bool(*value),
            ParamValue::Color(value) => Value::Color(*value),
            ParamValue::Point(value) => Value::Point(*value),
            ParamValue::Popup(value) => Value::Index(*value),
            ParamValue::Layer(value) => Value::Layer(*value),
        };
        inst.values.insert(param.name().to_string(), converted);
    }
}

impl CpuApply for Instance {
    fn apply_cpu(&self, input: &CpuInput<'_>, output: &mut Frame) -> ApplyResult<()> {
        apply_instance(self, input, output)
    }
}

#[derive(Clone, Debug, Default)]
pub struct Stack {
    pub items: Vec<Instance>,
}

impl Stack {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn add(&mut self, menu: &str) -> ApplyResult<&mut Instance> {
        let kind = EffectKind::from_menu(menu)
            .ok_or_else(|| ApplyError::new(format!("menu has no effect {menu}")))?;
        self.items.push(Instance::new(kind));
        let index = self.items.len() - 1;
        Ok(&mut self.items[index])
    }
}

impl CpuApply for Stack {
    fn apply_cpu(&self, input: &CpuInput<'_>, output: &mut Frame) -> ApplyResult<()> {
        if self.items.is_empty() {
            *output = input.frame.clone();
            return Ok(());
        }
        let mut current = input.frame.clone();
        let mut saw_fill_only = false;
        for item in &self.items {
            if item.kind == EffectKind::PaintBucket
                && item.choice("Blending Mode").as_deref() == Some("Fill Only")
                && saw_fill_only
            {
                continue;
            }
            if item.kind == EffectKind::PaintBucket && item.role_index("blend") == Some(u32::MAX) {
                // role blend MAX is Fill Only when the choice list is unconfirmed
            }
            if item.kind == EffectKind::PaintBucket && item.role_bool("fill_only") == Some(true) {
                if saw_fill_only {
                    continue;
                }
                saw_fill_only = true;
            }
            let mut next = current.clone();
            let step = CpuInput {
                time: input.time,
                bits: input.bits,
                params: input.params,
                frame: &current,
            };
            item.apply_cpu(&step, &mut next)?;
            current = next;
        }
        *output = current;
        Ok(())
    }
}

pub fn run(inst: &Instance, frame: &Frame, time: f64) -> ApplyResult<Frame> {
    run_bits(inst, frame, frame.bits(), time)
}

pub fn run_bits(
    inst: &Instance,
    frame: &Frame,
    bits: BitsPerChannel,
    time: f64,
) -> ApplyResult<Frame> {
    let params = ParamValues::new();
    let input = CpuInput {
        time: Time::from_seconds(time),
        bits,
        params: &params,
        frame,
    };
    let mut output = frame.clone();
    inst.apply_cpu(&input, &mut output)?;
    Ok(output)
}
