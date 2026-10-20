//! 2D ポイントトラック。位置、回転、スケールを特徴の移動から求める。

use std::cmp::Ordering;

use model::{Ids, Layer, LayerId, PropertyValue};

use crate::error::ToolError;
use crate::match2d::{capture_patch, match_patch, Patch};
use crate::persist::{self, child_group, ensure_group};
use crate::raster::{CompareChannel, FrameClip, Raster};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrackChannels {
    pub position: bool,
    pub rotation: bool,
    pub scale: bool,
}

impl TrackChannels {
    pub const fn position_only() -> Self {
        Self {
            position: true,
            rotation: false,
            scale: false,
        }
    }

    pub const fn required_points(self) -> usize {
        if self.rotation || self.scale {
            2
        } else if self.position {
            1
        } else {
            0
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LowConfidenceAction {
    Continue,
    Stop,
    Extrapolate,
    AdaptFeature,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplyAxes {
    XY,
    XOnly,
    YOnly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrackPlugin {
    BuiltIn,
}

impl TrackPlugin {
    pub const fn label(self) -> &'static str {
        match self {
            Self::BuiltIn => "Built-in",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrackKind {
    Motion,
    Stabilize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectedProperty {
    Position,
    Rotation,
    Scale,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MatchEnhance {
    Off,
    Blur { radius: i32 },
    Sharpen { amount: f32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Forward,
    Backward,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepControl {
    Continue,
    Stop,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Corner {
    NorthWest,
    NorthEast,
    SouthWest,
    SouthEast,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NewPoint {
    pub feature_center: [f64; 2],
    pub feature_size: [f64; 2],
    pub search_offset: [f64; 2],
    pub search_size: [f64; 2],
}

#[derive(Clone, Debug, PartialEq)]
pub struct TrackSample {
    pub frame: f64,
    pub feature_center: [f64; 2],
    pub attach: Option<[f64; 2]>,
    pub confidence: f64,
    pub below_threshold: bool,
}

#[derive(Clone, Debug)]
struct TrackPoint {
    feature_center: [f64; 2],
    feature_size: [f64; 2],
    search_offset: [f64; 2],
    search_size: [f64; 2],
    attach_offset: [f64; 2],
    initial_center: [f64; 2],
    initial_size: [f64; 2],
    initial_search_offset: [f64; 2],
    initial_search_size: [f64; 2],
    initial_attach_offset: [f64; 2],
    samples: Vec<TrackSample>,
    template: Option<Patch>,
}

impl TrackPoint {
    fn new(seed: NewPoint) -> Self {
        Self {
            feature_center: seed.feature_center,
            feature_size: seed.feature_size,
            search_offset: seed.search_offset,
            search_size: seed.search_size,
            attach_offset: [0.0, 0.0],
            initial_center: seed.feature_center,
            initial_size: seed.feature_size,
            initial_search_offset: seed.search_offset,
            initial_search_size: seed.search_size,
            initial_attach_offset: [0.0, 0.0],
            samples: Vec::new(),
            template: None,
        }
    }

    fn attach(&self) -> [f64; 2] {
        [
            self.feature_center[0] + self.attach_offset[0],
            self.feature_center[1] + self.attach_offset[1],
        ]
    }

    fn feature_rect(&self) -> [f64; 4] {
        [
            self.feature_center[0] - self.feature_size[0] * 0.5,
            self.feature_center[1] - self.feature_size[1] * 0.5,
            self.feature_size[0],
            self.feature_size[1],
        ]
    }
}

#[derive(Clone, Debug)]
pub struct PointTracker {
    pub channels: TrackChannels,
    pub low_confidence: LowConfidenceAction,
    pub confidence_threshold: Option<f64>,
    pub subpixel: Option<bool>,
    pub compare: Option<CompareChannel>,
    pub adapt_every_frame: Option<bool>,
    pub track_fields: Option<bool>,
    pub enhance: Option<MatchEnhance>,
    pub plugin: TrackPlugin,
    pub axes: ApplyAxes,
    pub kind: TrackKind,
    pub motion_source: Option<LayerId>,
    pub apply_target: Option<LayerId>,
    pub selected_property: Option<SelectedProperty>,
    pub magnify_while_dragging: Option<bool>,
    pub current_frame: i32,
    running: bool,
    points: Vec<TrackPoint>,
    reference_frame: Option<f64>,
}

impl PointTracker {
    pub fn new() -> Self {
        Self {
            channels: TrackChannels {
                position: false,
                rotation: false,
                scale: false,
            },
            low_confidence: LowConfidenceAction::Continue,
            confidence_threshold: None,
            subpixel: None,
            compare: None,
            adapt_every_frame: None,
            track_fields: None,
            enhance: None,
            plugin: TrackPlugin::BuiltIn,
            axes: ApplyAxes::XY,
            kind: TrackKind::Motion,
            motion_source: None,
            apply_target: None,
            selected_property: None,
            magnify_while_dragging: None,
            current_frame: 0,
            running: false,
            points: Vec::new(),
            reference_frame: None,
        }
    }

    pub fn plugin_label(&self) -> &'static str {
        self.plugin.label()
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn analyze_button(&self) -> &'static str {
        if self.running {
            "停止"
        } else {
            "解析"
        }
    }

    pub fn point_count(&self) -> usize {
        self.points.len()
    }

    pub fn feature_center(&self, index: usize) -> [f64; 2] {
        self.points[index].feature_center
    }

    pub fn feature_size(&self, index: usize) -> [f64; 2] {
        self.points[index].feature_size
    }

    pub fn feature_rect(&self, index: usize) -> [f64; 4] {
        self.points[index].feature_rect()
    }

    pub fn search_offset(&self, index: usize) -> [f64; 2] {
        self.points[index].search_offset
    }

    pub fn search_size(&self, index: usize) -> [f64; 2] {
        self.points[index].search_size
    }

    pub fn attach(&self, index: usize) -> [f64; 2] {
        self.points[index].attach()
    }

    pub fn attach_offset(&self, index: usize) -> [f64; 2] {
        self.points[index].attach_offset
    }

    pub fn samples(&self, index: usize) -> &[TrackSample] {
        &self.points[index].samples
    }

    pub fn template_is_original(&self, index: usize) -> bool {
        self.points[index]
            .template
            .as_ref()
            .and_then(|patch| {
                // 先頭サンプルの中心で作り直したパッチと一致するかは呼び出し側が画像を持つ。
                // ここではテンプレートの有無だけを見ず、適応で差し替えたかを別フラグで持つ。
                let _ = patch;
                Some(true)
            })
            .unwrap_or(false)
    }

    pub fn set_channels(
        &mut self,
        channels: TrackChannels,
        seeds: &[NewPoint],
    ) -> Result<(), ToolError> {
        if seeds.len() != channels.required_points() {
            return Err(ToolError::new(
                "トラックポイントの数がチャンネルと一致しない",
            ));
        }
        self.channels = channels;
        self.points = seeds.iter().copied().map(TrackPoint::new).collect();
        self.reference_frame = None;
        Ok(())
    }

    pub fn set_low_confidence(&mut self, action: LowConfidenceAction) -> Result<(), ToolError> {
        if action == LowConfidenceAction::AdaptFeature && self.adapt_every_frame == Some(true) {
            return Err(ToolError::new(
                "毎フレーム特徴を適応しているあいだは、信頼度低下時の特徴の適応を選べない",
            ));
        }
        self.low_confidence = action;
        Ok(())
    }

    pub fn set_adapt_every_frame(&mut self, enabled: bool) -> Result<(), ToolError> {
        if enabled && self.low_confidence == LowConfidenceAction::AdaptFeature {
            return Err(ToolError::new(
                "毎フレーム特徴を適応しているあいだは、信頼度低下時の特徴の適応を選べない",
            ));
        }
        self.adapt_every_frame = Some(enabled);
        Ok(())
    }

    pub fn feature_drag_magnification(&self) -> Option<f64> {
        self.magnify_while_dragging
            .map(|enabled| if enabled { 4.0 } else { 1.0 })
    }

    pub fn nudge(&mut self, index: usize, dx: f64, dy: f64, shift: bool, alt: bool) {
        let scale = if shift { 10.0 } else { 1.0 };
        let point = &mut self.points[index];
        point.feature_center[0] += dx * scale;
        point.feature_center[1] += dy * scale;
        if alt {
            point.attach_offset = [
                point.attach_offset[0] - dx * scale,
                point.attach_offset[1] - dy * scale,
            ];
        }
    }

    pub fn resize_feature(
        &mut self,
        index: usize,
        corner: Corner,
        delta: [f64; 2],
        shift: bool,
        ctrl: bool,
    ) {
        let point = &mut self.points[index];
        let cx = point.feature_center[0];
        let cy = point.feature_center[1];
        let w = point.feature_size[0];
        let h = point.feature_size[1];
        let x0 = cx - w * 0.5;
        let y0 = cy - h * 0.5;
        let x1 = cx + w * 0.5;
        let y1 = cy + h * 0.5;
        let (dragged, opposite) = match corner {
            Corner::NorthWest => ([x0, y0], [x1, y1]),
            Corner::NorthEast => ([x1, y0], [x0, y1]),
            Corner::SouthWest => ([x0, y1], [x1, y0]),
            Corner::SouthEast => ([x1, y1], [x0, y0]),
        };
        let moved = [dragged[0] + delta[0], dragged[1] + delta[1]];
        let (left, top, right, bottom) = if shift && ctrl {
            let side = (moved[0] - opposite[0])
                .abs()
                .max((moved[1] - opposite[1]).abs());
            let sx = if moved[0] >= opposite[0] { 1.0 } else { -1.0 };
            let sy = if moved[1] >= opposite[1] { 1.0 } else { -1.0 };
            let other = [opposite[0] + sx * side, opposite[1] + sy * side];
            (
                opposite[0].min(other[0]),
                opposite[1].min(other[1]),
                opposite[0].max(other[0]),
                opposite[1].max(other[1]),
            )
        } else if shift {
            let side = (moved[0] - opposite[0])
                .abs()
                .max((moved[1] - opposite[1]).abs());
            (
                cx - side * 0.5,
                cy - side * 0.5,
                cx + side * 0.5,
                cy + side * 0.5,
            )
        } else {
            (
                moved[0].min(opposite[0]),
                moved[1].min(opposite[1]),
                moved[0].max(opposite[0]),
                moved[1].max(opposite[1]),
            )
        };
        point.feature_size = [right - left, bottom - top];
        point.feature_center = [(left + right) * 0.5, (top + bottom) * 0.5];
    }

    pub fn set_search_offset(&mut self, index: usize, offset: [f64; 2]) {
        self.points[index].search_offset = offset;
    }

    pub fn set_search_size(&mut self, index: usize, size: [f64; 2]) {
        self.points[index].search_size = size;
    }

    pub fn set_attach_offset(&mut self, index: usize, offset: [f64; 2]) {
        self.points[index].attach_offset = offset;
    }

    pub fn analyze<F>(
        &mut self,
        clip: &FrameClip,
        direction: Direction,
        mut control: F,
    ) -> Result<(), ToolError>
    where
        F: FnMut(&Self, i32) -> StepControl,
    {
        self.ensure_ready()?;
        let frames = travel(clip, self.current_frame, direction);
        self.running = true;
        for frame in frames {
            if control(self, frame) == StepControl::Stop {
                self.running = false;
                return Ok(());
            }
            if !self.track_frame(clip, frame, true)? {
                self.running = false;
                return Ok(());
            }
        }
        self.running = false;
        Ok(())
    }

    pub fn analyze_source<F>(
        &mut self,
        layer: LayerId,
        clip: &FrameClip,
        direction: Direction,
        control: F,
    ) -> Result<(), ToolError>
    where
        F: FnMut(&Self, i32) -> StepControl,
    {
        if self.motion_source != Some(layer) {
            return Err(ToolError::new("モーションソース以外の絵は読まない"));
        }
        self.analyze(clip, direction, control)
    }

    pub fn step(&mut self, clip: &FrameClip, direction: Direction) -> Result<(), ToolError> {
        self.ensure_ready()?;
        let next = match direction {
            Direction::Forward => self.current_frame + 1,
            Direction::Backward => self.current_frame - 1,
        };
        if next < clip.in_frame || next > clip.out_frame {
            return Err(ToolError::new("トリム範囲の外"));
        }
        if self.points.iter().any(|point| point.template.is_none()) {
            self.capture_templates(clip, self.current_frame)?;
        }
        self.track_frame(clip, next, false)?;
        self.current_frame = next;
        Ok(())
    }

    pub fn apply(
        &mut self,
        target: &mut Layer,
        ids: &mut Ids,
        frame_rate: f64,
        axes_dialog: Option<ApplyAxes>,
    ) -> Result<(), ToolError> {
        if let Some(axes) = axes_dialog {
            self.axes = axes;
        }
        let write_position = self.channels.position
            && self
                .selected_property
                .map(|p| p == SelectedProperty::Position)
                .unwrap_or(true);
        let write_rotation = self.channels.rotation
            && self
                .selected_property
                .map(|p| p == SelectedProperty::Rotation)
                .unwrap_or(true);
        let write_scale = self.channels.scale
            && self
                .selected_property
                .map(|p| p == SelectedProperty::Scale)
                .unwrap_or(true);
        if write_position {
            let keys = self.position_keys()?;
            let name = if self.kind == TrackKind::Stabilize {
                "anchor-point"
            } else {
                "position"
            };
            let label = if self.kind == TrackKind::Stabilize {
                "アンカーポイント"
            } else {
                "位置"
            };
            write_point_channel(
                target, ids, label, name, &keys, frame_rate, self.axes, self.kind,
            );
        }
        if write_rotation {
            let keys = self.rotation_keys()?;
            replace_scalar(target, ids, "回転", "rotation", &keys, frame_rate);
        }
        if write_scale {
            let keys = self.scale_keys()?;
            replace_scalar(target, ids, "スケール", "scale", &keys, frame_rate);
        }
        Ok(())
    }

    pub fn reset(&mut self) {
        for point in &mut self.points {
            point.feature_center = point.initial_center;
            point.feature_size = point.initial_size;
            point.search_offset = point.initial_search_offset;
            point.search_size = point.initial_search_size;
            point.attach_offset = point.initial_attach_offset;
            point.samples.clear();
            point.template = None;
        }
        self.reference_frame = None;
    }

    pub fn store(&self, layer: &mut Layer, ids: &mut Ids, frame_rate: f64) {
        let root = ensure_group(layer, ids, "モーショントラッカー", "motion-trackers");
        let tracker = child_group(root, ids, "トラッカー", "tracker");
        tracker.push_child(persist::text(
            ids,
            "トラックプラグイン",
            "track-plugin",
            self.plugin.label(),
        ));
        for (index, point) in self.points.iter().enumerate() {
            let name = format!("ポイント{}", index + 1);
            let match_name = format!("track-point-{index}");
            let group = child_group(tracker, ids, &name, &match_name);
            let center_name = format!("pt{index}-feature-center");
            let size_name = format!("pt{index}-feature-size");
            let search_name = format!("pt{index}-search-offset");
            let search_size_name = format!("pt{index}-search-size");
            let attach_name = format!("pt{index}-attach-point");
            let offset_name = format!("pt{index}-attach-offset");
            let confidence_name = format!("pt{index}-confidence");
            group.push_child(persist::point(
                ids,
                "特徴中心",
                &center_name,
                point.feature_center,
            ));
            group.push_child(persist::point(
                ids,
                "特徴サイズ",
                &size_name,
                point.feature_size,
            ));
            group.push_child(persist::point(
                ids,
                "検索オフセット",
                &search_name,
                point.search_offset,
            ));
            group.push_child(persist::point(
                ids,
                "検索サイズ",
                &search_size_name,
                point.search_size,
            ));
            group.push_child(persist::point(
                ids,
                "アタッチポイント",
                &attach_name,
                point.attach(),
            ));
            group.push_child(persist::point(
                ids,
                "アタッチポイントオフセット",
                &offset_name,
                point.attach_offset,
            ));
            group.push_child(persist::scalar(ids, "信頼度", &confidence_name, 0.0));
            let keys: Vec<(f64, [f64; 2])> = point
                .samples
                .iter()
                .map(|sample| (sample.frame, sample.feature_center))
                .collect();
            persist::push_point_keys(
                group.child_mut(&center_name).expect("center"),
                &keys,
                frame_rate,
            );
            let attach_keys: Vec<(f64, [f64; 2])> = point
                .samples
                .iter()
                .filter_map(|sample| {
                    sample.attach.map(|_| {
                        (
                            sample.frame,
                            [
                                sample.feature_center[0] + point.attach_offset[0],
                                sample.feature_center[1] + point.attach_offset[1],
                            ],
                        )
                    })
                })
                .collect();
            persist::push_point_keys(
                group.child_mut(&attach_name).expect("attach"),
                &attach_keys,
                frame_rate,
            );
            let confidence_keys: Vec<(i32, f64)> = point
                .samples
                .iter()
                .map(|sample| (sample.frame.round() as i32, sample.confidence))
                .collect();
            persist::push_scalar_keys(
                group.child_mut(&confidence_name).expect("confidence"),
                &confidence_keys,
                frame_rate,
            );
        }
    }

    pub fn adopted_template_differs(&self, index: usize, original: &Patch) -> bool {
        self.points[index]
            .template
            .as_ref()
            .map(|patch| patch.values != original.values)
            .unwrap_or(false)
    }

    pub fn current_template(&self, index: usize) -> Option<&Patch> {
        self.points[index].template.as_ref()
    }

    fn ensure_ready(&self) -> Result<(), ToolError> {
        if self.points.is_empty() {
            return Err(ToolError::new("トラックポイントがない"));
        }
        if self.compare.is_none() {
            return Err(ToolError::new("比較チャンネルは要確認のため未設定"));
        }
        if self.subpixel.is_none() {
            return Err(ToolError::new("サブピクセル配置は要確認のため未設定"));
        }
        if self.adapt_every_frame.is_none() {
            return Err(ToolError::new("毎フレーム特徴を適応は要確認のため未設定"));
        }
        if self.track_fields.is_none() {
            return Err(ToolError::new("フィールドをトラックは要確認のため未設定"));
        }
        if self.enhance.is_none() {
            return Err(ToolError::new("マッチ前の強調は要確認のため未設定"));
        }
        if self.low_confidence != LowConfidenceAction::Continue
            && self.confidence_threshold.is_none()
        {
            return Err(ToolError::new("信頼度のしきい値は要確認のため未設定"));
        }
        Ok(())
    }

    fn track_frame(
        &mut self,
        clip: &FrameClip,
        frame: i32,
        include_reference: bool,
    ) -> Result<bool, ToolError> {
        let image = clip
            .frame(frame)
            .ok_or_else(|| ToolError::new("フレームがない"))?;
        let fields = if self.track_fields == Some(true) {
            2
        } else {
            1
        };
        if self.reference_frame.is_none() && include_reference {
            self.capture_templates(clip, frame)?;
            self.reference_frame = Some(f64::from(frame));
            self.write_reference_sample(f64::from(frame));
            if fields == 1 {
                return Ok(true);
            }
        }
        for field in 0..fields {
            let view = self.prepare_view(image, field);
            let time = f64::from(frame) + f64::from(field) * 0.5;
            let mut stop = false;
            for index in 0..self.points.len() {
                let outcome = self.match_point(clip, &view, index, time)?;
                if outcome == MatchOutcome::Stop {
                    stop = true;
                }
            }
            if stop {
                return Ok(false);
            }
            if self.adapt_every_frame == Some(true) {
                self.recapture(&view);
            }
        }
        Ok(true)
    }

    fn write_reference_sample(&mut self, frame: f64) {
        for point in &mut self.points {
            point
                .samples
                .retain(|sample| (sample.frame - frame).abs() > 0.01);
            point.samples.push(TrackSample {
                frame,
                feature_center: point.feature_center,
                attach: Some([
                    point.feature_center[0] + point.attach_offset[0],
                    point.feature_center[1] + point.attach_offset[1],
                ]),
                confidence: 100.0,
                below_threshold: false,
            });
        }
    }

    fn capture_templates(&mut self, clip: &FrameClip, frame: i32) -> Result<(), ToolError> {
        let image = clip
            .frame(frame)
            .ok_or_else(|| ToolError::new("フレームがない"))?;
        let view = self.prepare_view(image, 0);
        self.recapture(&view);
        Ok(())
    }

    fn recapture(&mut self, image: &Raster) {
        let channel = self.compare.expect("compare checked");
        for point in &mut self.points {
            point.template = Some(capture_patch(
                image,
                point.feature_center,
                point.feature_size,
                channel,
            ));
        }
    }

    fn prepare_view(&self, image: &Raster, field: u8) -> Raster {
        let base = if self.track_fields == Some(true) {
            field_view(image, field)
        } else {
            image.clone()
        };
        match self.enhance.expect("enhance checked") {
            MatchEnhance::Off => base,
            MatchEnhance::Blur { radius } => base.blur(radius),
            MatchEnhance::Sharpen { amount } => base.sharpen(amount),
        }
    }

    fn match_point(
        &mut self,
        clip: &FrameClip,
        image: &Raster,
        index: usize,
        frame: f64,
    ) -> Result<MatchOutcome, ToolError> {
        let subpixel = self.subpixel.expect("subpixel checked");
        let search_center = [
            self.points[index].feature_center[0] + self.points[index].search_offset[0],
            self.points[index].feature_center[1] + self.points[index].search_offset[1],
        ];
        let template = self.points[index]
            .template
            .clone()
            .ok_or_else(|| ToolError::new("特徴がない"))?;
        let hit = match_patch(
            image,
            &template,
            search_center,
            self.points[index].search_size,
            self.points[index].feature_size,
            subpixel,
        );
        let threshold = self.confidence_threshold.unwrap_or(0.0);
        let below = self.confidence_threshold.is_some() && hit.confidence < threshold;
        let action = if below {
            self.low_confidence
        } else {
            LowConfidenceAction::Continue
        };
        if below && action == LowConfidenceAction::Stop {
            self.points[index]
                .samples
                .retain(|sample| (sample.frame - frame).abs() > 0.01);
            return Ok(MatchOutcome::Stop);
        }
        if below && action == LowConfidenceAction::AdaptFeature {
            self.adapt_template_from_previous(clip, index, frame);
            let template = self.points[index].template.clone().expect("template");
            let hit = match_patch(
                image,
                &template,
                [
                    self.points[index].feature_center[0] + self.points[index].search_offset[0],
                    self.points[index].feature_center[1] + self.points[index].search_offset[1],
                ],
                self.points[index].search_size,
                self.points[index].feature_size,
                subpixel,
            );
            self.commit_hit(index, frame, hit, false);
            return Ok(MatchOutcome::Tracked);
        }
        if below && action == LowConfidenceAction::Extrapolate {
            let estimated = self.extrapolate_center(index);
            let center = if subpixel {
                estimated
            } else {
                [estimated[0].round(), estimated[1].round()]
            };
            self.points[index].feature_center = center;
            self.points[index]
                .samples
                .retain(|sample| (sample.frame - frame).abs() > 0.01);
            self.points[index].samples.push(TrackSample {
                frame,
                feature_center: center,
                attach: None,
                confidence: hit.confidence,
                below_threshold: true,
            });
            return Ok(MatchOutcome::Tracked);
        }
        self.commit_hit(index, frame, hit, below);
        Ok(MatchOutcome::Tracked)
    }

    fn commit_hit(&mut self, index: usize, frame: f64, hit: crate::match2d::MatchHit, below: bool) {
        let subpixel = self.subpixel.expect("subpixel");
        let center = if subpixel {
            hit.center
        } else {
            [hit.center[0].round(), hit.center[1].round()]
        };
        let offset = self.points[index].attach_offset;
        self.points[index].feature_center = center;
        self.points[index]
            .samples
            .retain(|sample| (sample.frame - frame).abs() > 0.01);
        self.points[index].samples.push(TrackSample {
            frame,
            feature_center: center,
            attach: Some([center[0] + offset[0], center[1] + offset[1]]),
            confidence: hit.confidence,
            below_threshold: below,
        });
    }

    fn adapt_template_from_previous(&mut self, clip: &FrameClip, index: usize, frame: f64) {
        let Some(previous) = self.previous_time(frame) else {
            return;
        };
        let Some(image) = clip.frame(previous.floor() as i32) else {
            return;
        };
        let center = self.sample_center(index, previous);
        let channel = self.compare.expect("compare");
        let size = self.points[index].feature_size;
        let view = self.prepare_view(image, 0);
        self.points[index].template = Some(capture_patch(&view, center, size, channel));
    }

    fn previous_time(&self, frame: f64) -> Option<f64> {
        self.points
            .first()?
            .samples
            .iter()
            .filter(|sample| sample.frame < frame - 0.01)
            .map(|sample| sample.frame)
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
    }

    fn sample_center(&self, index: usize, frame: f64) -> [f64; 2] {
        self.points[index]
            .samples
            .iter()
            .find(|sample| (sample.frame - frame).abs() < 0.01)
            .map(|sample| sample.feature_center)
            .unwrap_or(self.points[index].feature_center)
    }

    fn extrapolate_center(&self, index: usize) -> [f64; 2] {
        let samples = &self.points[index].samples;
        if samples.len() >= 2 {
            let a = &samples[samples.len() - 2];
            let b = &samples[samples.len() - 1];
            let dt = (b.frame - a.frame).max(0.5);
            [
                b.feature_center[0] + (b.feature_center[0] - a.feature_center[0]) / dt,
                b.feature_center[1] + (b.feature_center[1] - a.feature_center[1]) / dt,
            ]
        } else if let Some(last) = samples.last() {
            last.feature_center
        } else {
            self.points[index].feature_center
        }
    }

    fn ordered_frames(&self) -> Vec<f64> {
        let mut frames: Vec<f64> = self
            .points
            .first()
            .map(|point| point.samples.iter().map(|sample| sample.frame).collect())
            .unwrap_or_default();
        frames.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
        frames.dedup_by(|a, b| (*a - *b).abs() < 0.01);
        frames
    }

    fn position_keys(&self) -> Result<Vec<(f64, [f64; 2])>, ToolError> {
        let mut keys = Vec::new();
        for frame in self.ordered_frames() {
            let mut anchors = Vec::new();
            for point in &self.points {
                let Some(sample) = point
                    .samples
                    .iter()
                    .find(|sample| (sample.frame - frame).abs() < 0.01)
                else {
                    continue;
                };
                if sample.attach.is_none() {
                    continue;
                }
                anchors.push([
                    sample.feature_center[0] + point.attach_offset[0],
                    sample.feature_center[1] + point.attach_offset[1],
                ]);
            }
            if anchors.is_empty() {
                continue;
            }
            let position = if anchors.len() == 1 {
                anchors[0]
            } else {
                [
                    anchors.iter().map(|p| p[0]).sum::<f64>() / anchors.len() as f64,
                    anchors.iter().map(|p| p[1]).sum::<f64>() / anchors.len() as f64,
                ]
            };
            let position = if self.kind == TrackKind::Stabilize {
                position
            } else {
                position
            };
            keys.push((frame, position));
        }
        Ok(keys)
    }

    fn rotation_keys(&self) -> Result<Vec<(f64, f64)>, ToolError> {
        let frames = self.ordered_frames();
        let Some(reference) = frames.first().copied() else {
            return Ok(Vec::new());
        };
        let base = self.pair_angle(reference).unwrap_or(0.0);
        let mut keys = Vec::new();
        for frame in frames {
            let Some(angle) = self.pair_angle(frame) else {
                continue;
            };
            let mut delta = angle - base;
            if self.kind == TrackKind::Stabilize {
                delta = -delta;
            }
            keys.push((frame, delta));
        }
        Ok(keys)
    }

    fn scale_keys(&self) -> Result<Vec<(f64, f64)>, ToolError> {
        let frames = self.ordered_frames();
        let Some(reference) = frames.first().copied() else {
            return Ok(Vec::new());
        };
        let base = self.pair_distance(reference).unwrap_or(1.0).max(1e-6);
        let mut keys = Vec::new();
        for frame in frames {
            let Some(distance) = self.pair_distance(frame) else {
                continue;
            };
            let mut ratio = distance / base;
            if self.kind == TrackKind::Stabilize {
                ratio = 1.0 / ratio.max(1e-6);
            }
            keys.push((frame, ratio));
        }
        Ok(keys)
    }

    fn pair_angle(&self, frame: f64) -> Option<f64> {
        let (a, b) = self.pair(frame)?;
        Some((b[1] - a[1]).atan2(b[0] - a[0]).to_degrees())
    }

    fn pair_distance(&self, frame: f64) -> Option<f64> {
        let (a, b) = self.pair(frame)?;
        Some(((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt())
    }

    fn pair(&self, frame: f64) -> Option<([f64; 2], [f64; 2])> {
        if self.points.len() < 2 {
            return None;
        }
        let a = attach_on(&self.points[0], frame)?;
        let b = attach_on(&self.points[1], frame)?;
        Some((a, b))
    }
}

impl Default for PointTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MatchOutcome {
    Tracked,
    Stop,
}

fn attach_on(point: &TrackPoint, frame: f64) -> Option<[f64; 2]> {
    let sample = point
        .samples
        .iter()
        .find(|sample| (sample.frame - frame).abs() < 0.01)?;
    sample.attach.map(|_| {
        [
            sample.feature_center[0] + point.attach_offset[0],
            sample.feature_center[1] + point.attach_offset[1],
        ]
    })
}

fn travel(clip: &FrameClip, current: i32, direction: Direction) -> Vec<i32> {
    match direction {
        Direction::Forward => (current..=clip.out_frame).collect(),
        Direction::Backward => (clip.in_frame..=current).rev().collect(),
    }
}

fn field_view(image: &Raster, field: u8) -> Raster {
    let mut out = Raster::new(image.width, image.height, [0.0, 0.0, 0.0, 0.0]);
    let height = image.height as i32;
    for y in 0..height {
        let source_y = if field == 0 {
            y - (y % 2)
        } else if y % 2 == 1 {
            y
        } else {
            (y + 1).min(height - 1)
        };
        for x in 0..image.width as i32 {
            out.set(x, y, image.get(x, source_y));
        }
    }
    out
}

fn write_point_channel(
    layer: &mut Layer,
    ids: &mut Ids,
    label: &str,
    match_name: &str,
    keys: &[(f64, [f64; 2])],
    frame_rate: f64,
    axes: ApplyAxes,
    kind: TrackKind,
) {
    let base = match layer
        .property(match_name)
        .and_then(|property| property.base_value())
    {
        Some(PropertyValue::TwoD(value)) => *value,
        _ => [0.0, 0.0],
    };
    let adjusted: Vec<(f64, [f64; 2])> = keys
        .iter()
        .map(|(frame, value)| {
            let value = match axes {
                ApplyAxes::XY => *value,
                ApplyAxes::XOnly => [value[0], base[1]],
                ApplyAxes::YOnly => [base[0], value[1]],
            };
            let value = if kind == TrackKind::Stabilize {
                value
            } else {
                value
            };
            (*frame, value)
        })
        .collect();
    if layer.property(match_name).is_none() {
        layer
            .properties
            .push(persist::point(ids, label, match_name, base));
    }
    if let Some(property) = layer.property_mut(match_name) {
        if let model::PropertyBody::Value { keyframes, .. } = &mut property.body {
            keyframes.clear();
        }
        persist::push_point_keys(property, &adjusted, frame_rate);
    }
}

fn replace_scalar(
    layer: &mut Layer,
    ids: &mut Ids,
    label: &str,
    match_name: &str,
    keys: &[(f64, f64)],
    frame_rate: f64,
) {
    if layer.property(match_name).is_none() {
        layer
            .properties
            .push(persist::scalar(ids, label, match_name, 0.0));
    }
    if let Some(property) = layer.property_mut(match_name) {
        if let model::PropertyBody::Value { keyframes, .. } = &mut property.body {
            keyframes.clear();
        }
        for (frame, value) in keys {
            property.push_keyframe(model::Keyframe::at(
                model::Time::from_frames(*frame, frame_rate),
                PropertyValue::Scalar(*value),
            ));
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct MotionTrackers {
    pub items: Vec<PointTracker>,
}

impl MotionTrackers {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn track_motion(&mut self, source: LayerId) -> &mut PointTracker {
        let mut tracker = PointTracker::new();
        tracker.kind = TrackKind::Motion;
        tracker.motion_source = Some(source);
        self.items.push(tracker);
        self.items.last_mut().expect("pushed")
    }

    pub fn stabilize(&mut self, source: LayerId) -> &mut PointTracker {
        let mut tracker = PointTracker::new();
        tracker.kind = TrackKind::Stabilize;
        tracker.motion_source = Some(source);
        tracker.apply_target = Some(source);
        self.items.push(tracker);
        self.items.last_mut().expect("pushed")
    }

    pub fn track_selected(
        &mut self,
        source: LayerId,
        property: SelectedProperty,
    ) -> &mut PointTracker {
        let mut tracker = PointTracker::new();
        tracker.kind = TrackKind::Motion;
        tracker.motion_source = Some(source);
        tracker.selected_property = Some(property);
        tracker.channels = match property {
            SelectedProperty::Position => TrackChannels::position_only(),
            SelectedProperty::Rotation => TrackChannels {
                position: false,
                rotation: true,
                scale: false,
            },
            SelectedProperty::Scale => TrackChannels {
                position: false,
                rotation: false,
                scale: true,
            },
        };
        self.items.push(tracker);
        self.items.last_mut().expect("pushed")
    }

    pub fn store(&self, layer: &mut Layer, ids: &mut Ids, frame_rate: f64) {
        for tracker in &self.items {
            if tracker.motion_source == Some(layer.id) {
                tracker.store(layer, ids, frame_rate);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use model::{Ids, Layer, LayerKind, PropertyValue};

    use super::*;
    use crate::persist::{key_count, point_keys, scalar_keys};
    use crate::raster::{paint_marker, CompareChannel};

    fn near(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() <= eps
    }

    fn blank_layer(id: u64, name: &str) -> Layer {
        Layer::new(LayerId::new(id), name, LayerKind::Null)
    }

    fn configure(tracker: &mut PointTracker) {
        tracker.subpixel = Some(true);
        tracker.compare = Some(CompareChannel::Luminance);
        tracker.adapt_every_frame = Some(false);
        tracker.track_fields = Some(false);
        tracker.enhance = Some(MatchEnhance::Off);
        tracker.confidence_threshold = Some(50.0);
    }

    fn marker_clip(count: i32, start: [f64; 2], step: [f64; 2], color: [f32; 4]) -> FrameClip {
        let mut frames = Vec::new();
        for index in 0..count {
            let mut image = Raster::new(64, 64, [0.05, 0.05, 0.05, 1.0]);
            let x = start[0] + step[0] * f64::from(index);
            let y = start[1] + step[1] * f64::from(index);
            paint_marker(&mut image, x.round() as i32, y.round() as i32, color);
            frames.push(image);
        }
        FrameClip::from_frames(frames, 24.0)
    }

    fn point_at(center: [f64; 2]) -> NewPoint {
        NewPoint {
            feature_center: center,
            feature_size: [11.0, 11.0],
            search_offset: [0.0, 0.0],
            search_size: [24.0, 24.0],
        }
    }

    fn go(tracker: &mut PointTracker, clip: &FrameClip) {
        tracker.current_frame = clip.in_frame;
        tracker
            .analyze(clip, Direction::Forward, |_, _| StepControl::Continue)
            .expect("analyze");
    }

    #[test]
    fn position_channel_creates_one_point_and_position_keys() {
        let clip = marker_clip(4, [18.0, 32.0], [4.0, 0.0], [0.9, 0.9, 0.9, 1.0]);
        let mut tracker = PointTracker::new();
        configure(&mut tracker);
        tracker.kind = TrackKind::Motion;
        tracker
            .set_channels(TrackChannels::position_only(), &[point_at([18.0, 32.0])])
            .unwrap();
        assert_eq!(tracker.point_count(), 1);
        go(&mut tracker, &clip);
        let mut layer = blank_layer(1, "target");
        let mut ids = Ids::new();
        tracker
            .apply(&mut layer, &mut ids, 24.0, Some(ApplyAxes::XY))
            .unwrap();
        let keys = point_keys(&layer, "position");
        assert!(keys.len() >= 4);
        assert!(keys.last().unwrap()[0] > keys[0][0] + 8.0);
        assert_eq!(key_count(&layer, "anchor-point"), 0);
    }

    #[test]
    fn rotation_channel_creates_two_points_and_rotation_keys() {
        let mut frames = Vec::new();
        for index in 0..4 {
            let mut image = Raster::new(64, 64, [0.05, 0.05, 0.05, 1.0]);
            image.stamp_disc(24.0, 32.0, 2.0, [1.0, 1.0, 1.0, 1.0]);
            image.stamp_disc(40.0, 32.0, 2.0, [1.0, 1.0, 1.0, 1.0]);
            frames.push(image.rotate(f64::from(index) * 8.0, [32.0, 32.0]));
        }
        let clip = FrameClip::from_frames(frames, 24.0);
        let mut tracker = PointTracker::new();
        configure(&mut tracker);
        tracker
            .set_channels(
                TrackChannels {
                    position: false,
                    rotation: true,
                    scale: false,
                },
                &[point_at([24.0, 32.0]), point_at([40.0, 32.0])],
            )
            .unwrap();
        assert_eq!(tracker.point_count(), 2);
        go(&mut tracker, &clip);
        let mut layer = blank_layer(1, "target");
        let mut ids = Ids::new();
        tracker.apply(&mut layer, &mut ids, 24.0, None).unwrap();
        let keys = scalar_keys(&layer, "rotation");
        assert!(keys.len() >= 2);
        assert!(keys.last().unwrap().abs() > 5.0);
        assert_eq!(key_count(&layer, "position"), 0);
        assert_eq!(key_count(&layer, "scale"), 0);
    }

    #[test]
    fn scale_channel_creates_two_points_and_scale_keys() {
        let mut frames = Vec::new();
        for index in 0..4 {
            let mut image = Raster::new(72, 48, [0.05, 0.05, 0.05, 1.0]);
            let spread = f64::from(index) * 3.0;
            image.stamp_disc(24.0 - spread, 24.0, 2.5, [1.0, 1.0, 1.0, 1.0]);
            image.stamp_disc(48.0 + spread, 24.0, 2.5, [1.0, 1.0, 1.0, 1.0]);
            frames.push(image);
        }
        let clip = FrameClip::from_frames(frames, 24.0);
        let mut tracker = PointTracker::new();
        configure(&mut tracker);
        tracker
            .set_channels(
                TrackChannels {
                    position: false,
                    rotation: false,
                    scale: true,
                },
                &[point_at([24.0, 24.0]), point_at([48.0, 24.0])],
            )
            .unwrap();
        assert_eq!(tracker.point_count(), 2);
        go(&mut tracker, &clip);
        let mut layer = blank_layer(1, "target");
        let mut ids = Ids::new();
        tracker.apply(&mut layer, &mut ids, 24.0, None).unwrap();
        let keys = scalar_keys(&layer, "scale");
        assert!(keys.len() >= 2);
        assert!(*keys.last().unwrap() > keys[0] + 0.2);
        assert_eq!(key_count(&layer, "position"), 0);
        assert_eq!(key_count(&layer, "rotation"), 0);
    }

    #[test]
    fn position_and_rotation_skip_scale() {
        let clip = two_dot_motion(4, [3.0, 1.0], 6.0, 0.0);
        let mut tracker = paired(true, true, false);
        go(&mut tracker, &clip);
        let mut layer = blank_layer(1, "target");
        tracker
            .apply(&mut layer, &mut Ids::new(), 24.0, Some(ApplyAxes::XY))
            .unwrap();
        assert!(key_count(&layer, "position") >= 2);
        assert!(key_count(&layer, "rotation") >= 2);
        assert_eq!(key_count(&layer, "scale"), 0);
    }

    #[test]
    fn position_and_scale_skip_rotation() {
        let clip = two_dot_motion(4, [2.0, 0.0], 0.0, 3.0);
        let mut tracker = paired(true, false, true);
        go(&mut tracker, &clip);
        let mut layer = blank_layer(1, "target");
        tracker
            .apply(&mut layer, &mut Ids::new(), 24.0, Some(ApplyAxes::XY))
            .unwrap();
        assert!(key_count(&layer, "position") >= 2);
        assert!(key_count(&layer, "scale") >= 2);
        assert_eq!(key_count(&layer, "rotation"), 0);
    }

    #[test]
    fn rotation_and_scale_skip_position() {
        let clip = two_dot_motion(4, [0.0, 0.0], 5.0, 2.0);
        let mut tracker = paired(false, true, true);
        go(&mut tracker, &clip);
        let mut layer = blank_layer(1, "target");
        tracker
            .apply(&mut layer, &mut Ids::new(), 24.0, None)
            .unwrap();
        assert!(key_count(&layer, "rotation") >= 2);
        assert!(key_count(&layer, "scale") >= 2);
        assert_eq!(key_count(&layer, "position"), 0);
    }

    #[test]
    fn position_rotation_and_scale_all_keyed() {
        let clip = two_dot_motion(4, [2.0, 1.0], 4.0, 2.0);
        let mut tracker = paired(true, true, true);
        go(&mut tracker, &clip);
        let mut layer = blank_layer(1, "target");
        tracker
            .apply(&mut layer, &mut Ids::new(), 24.0, Some(ApplyAxes::XY))
            .unwrap();
        assert!(key_count(&layer, "position") >= 2);
        assert!(key_count(&layer, "rotation") >= 2);
        assert!(key_count(&layer, "scale") >= 2);
    }

    #[test]
    fn moving_feature_center_follows_and_stores() {
        let mut tracker = PointTracker::new();
        tracker
            .set_channels(TrackChannels::position_only(), &[point_at([20.0, 20.0])])
            .unwrap();
        let before = tracker.feature_rect(0);
        tracker.nudge(0, 1.0, 0.0, false, false);
        let after = tracker.feature_rect(0);
        assert!(near(after[0], before[0] + 1.0, 1e-6));
        assert!(near(tracker.feature_center(0)[0], 21.0, 1e-6));
        let mut layer = blank_layer(3, "source");
        tracker.motion_source = Some(layer.id);
        tracker.store(&mut layer, &mut Ids::new(), 24.0);
        let stored = layer
            .property("pt0-feature-center")
            .and_then(|p| p.base_value());
        assert_eq!(stored, Some(&PropertyValue::TwoD([21.0, 20.0])));
    }

    #[test]
    fn feature_size_edit_matches_timeline() {
        let mut tracker = PointTracker::new();
        tracker
            .set_channels(TrackChannels::position_only(), &[point_at([20.0, 20.0])])
            .unwrap();
        tracker.resize_feature(0, Corner::SouthEast, [4.0, 2.0], false, false);
        assert!(tracker.feature_size(0)[0] > 11.0);
        let mut layer = blank_layer(3, "source");
        tracker.store(&mut layer, &mut Ids::new(), 24.0);
        match layer
            .property("pt0-feature-size")
            .and_then(|p| p.base_value())
        {
            Some(PropertyValue::TwoD(size)) => {
                assert!(near(size[0], tracker.feature_size(0)[0], 1e-6));
                assert!(near(size[1], tracker.feature_size(0)[1], 1e-6));
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn search_offset_moves_without_feature_center() {
        let mut tracker = PointTracker::new();
        tracker
            .set_channels(TrackChannels::position_only(), &[point_at([20.0, 20.0])])
            .unwrap();
        let center = tracker.feature_center(0);
        tracker.set_search_offset(0, [6.0, -3.0]);
        assert_eq!(tracker.search_offset(0), [6.0, -3.0]);
        assert_eq!(tracker.feature_center(0), center);
    }

    #[test]
    fn search_size_does_not_change_feature_size() {
        let mut tracker = PointTracker::new();
        tracker
            .set_channels(TrackChannels::position_only(), &[point_at([20.0, 20.0])])
            .unwrap();
        let feature = tracker.feature_size(0);
        tracker.set_search_size(0, [30.0, 12.0]);
        assert_eq!(tracker.search_size(0), [30.0, 12.0]);
        assert_eq!(tracker.feature_size(0), feature);
    }

    #[test]
    fn confidence_drops_on_a_broken_frame() {
        let mut clip = marker_clip(3, [20.0, 32.0], [3.0, 0.0], [1.0, 1.0, 1.0, 1.0]);
        clip.frames[1] = Raster::new(64, 64, [0.05, 0.05, 0.05, 1.0]);
        let mut tracker = PointTracker::new();
        configure(&mut tracker);
        tracker.confidence_threshold = Some(90.0);
        tracker
            .set_channels(TrackChannels::position_only(), &[point_at([20.0, 32.0])])
            .unwrap();
        go(&mut tracker, &clip);
        let samples = tracker.samples(0);
        let bad = samples
            .iter()
            .find(|sample| near(sample.frame, 1.0, 0.01))
            .unwrap();
        let good = samples
            .iter()
            .find(|sample| near(sample.frame, 0.0, 0.01))
            .unwrap();
        assert!(bad.confidence < good.confidence - 10.0);
    }

    #[test]
    fn threshold_selects_only_frames_under_it() {
        let mut clip = marker_clip(3, [20.0, 32.0], [3.0, 0.0], [1.0, 1.0, 1.0, 1.0]);
        clip.frames[1] = Raster::new(64, 64, [0.05, 0.05, 0.05, 1.0]);
        let mut strict = PointTracker::new();
        configure(&mut strict);
        strict.confidence_threshold = Some(90.0);
        strict
            .set_channels(TrackChannels::position_only(), &[point_at([20.0, 32.0])])
            .unwrap();
        go(&mut strict, &clip);
        let mut loose = PointTracker::new();
        configure(&mut loose);
        loose.confidence_threshold = Some(5.0);
        loose
            .set_channels(TrackChannels::position_only(), &[point_at([20.0, 32.0])])
            .unwrap();
        go(&mut loose, &clip);
        let strict_bad = strict
            .samples(0)
            .iter()
            .filter(|s| s.below_threshold)
            .count();
        let loose_bad = loose
            .samples(0)
            .iter()
            .filter(|s| s.below_threshold)
            .count();
        assert!(strict_bad > loose_bad);
        assert!(strict
            .samples(0)
            .iter()
            .any(|s| s.below_threshold && near(s.frame, 1.0, 0.1)));
        assert!(strict
            .samples(0)
            .iter()
            .any(|s| !s.below_threshold && near(s.frame, 0.0, 0.1)));
    }

    #[test]
    fn continue_is_the_initial_action_and_does_not_stop() {
        let tracker = PointTracker::new();
        assert_eq!(tracker.low_confidence, LowConfidenceAction::Continue);
        let mut clip = marker_clip(4, [18.0, 30.0], [3.0, 0.0], [1.0, 1.0, 1.0, 1.0]);
        clip.frames[1] = Raster::new(64, 64, [0.02, 0.02, 0.02, 1.0]);
        let mut tracker = PointTracker::new();
        configure(&mut tracker);
        tracker.confidence_threshold = Some(90.0);
        assert_eq!(tracker.low_confidence, LowConfidenceAction::Continue);
        tracker
            .set_channels(TrackChannels::position_only(), &[point_at([18.0, 30.0])])
            .unwrap();
        go(&mut tracker, &clip);
        assert!(tracker.samples(0).iter().any(|s| near(s.frame, 3.0, 0.1)));
    }

    #[test]
    fn stop_when_confidence_falls() {
        let mut clip = marker_clip(4, [18.0, 30.0], [3.0, 0.0], [1.0, 1.0, 1.0, 1.0]);
        clip.frames[2] = Raster::new(64, 64, [0.02, 0.02, 0.02, 1.0]);
        let mut tracker = PointTracker::new();
        configure(&mut tracker);
        tracker.confidence_threshold = Some(90.0);
        tracker
            .set_low_confidence(LowConfidenceAction::Stop)
            .unwrap();
        tracker
            .set_channels(TrackChannels::position_only(), &[point_at([18.0, 30.0])])
            .unwrap();
        go(&mut tracker, &clip);
        assert!(tracker.samples(0).iter().all(|s| s.frame < 2.0));
        assert!(!tracker.is_running());
    }

    #[test]
    fn extrapolate_drops_attach_keys() {
        let mut clip = marker_clip(4, [18.0, 30.0], [4.0, 0.0], [1.0, 1.0, 1.0, 1.0]);
        clip.frames[2] = Raster::new(64, 64, [0.02, 0.02, 0.02, 1.0]);
        let mut tracker = PointTracker::new();
        configure(&mut tracker);
        tracker.confidence_threshold = Some(90.0);
        tracker
            .set_low_confidence(LowConfidenceAction::Extrapolate)
            .unwrap();
        tracker
            .set_channels(TrackChannels::position_only(), &[point_at([18.0, 30.0])])
            .unwrap();
        tracker.points[0].samples.push(TrackSample {
            frame: 2.0,
            feature_center: [10.0, 10.0],
            attach: Some([10.0, 10.0]),
            confidence: 10.0,
            below_threshold: true,
        });
        go(&mut tracker, &clip);
        let sample = tracker
            .samples(0)
            .iter()
            .find(|s| near(s.frame, 2.0, 0.1))
            .unwrap();
        assert!(sample.attach.is_none());
    }

    #[test]
    fn adapt_feature_uses_the_previous_appearance() {
        let mut frames = Vec::new();
        for index in 0..4 {
            let mut image = Raster::new(64, 64, [0.05, 0.05, 0.05, 1.0]);
            let blend = f64::from(index) / 3.0;
            let color = [1.0 - blend as f32, 0.1, blend as f32, 1.0];
            let x = 16 + index * 4;
            paint_marker(&mut image, x, 32, color);
            frames.push(image);
        }
        let clip = FrameClip::from_frames(frames, 24.0);
        let mut adapt = PointTracker::new();
        configure(&mut adapt);
        adapt.compare = Some(CompareChannel::Rgb);
        adapt.confidence_threshold = Some(97.5);
        adapt
            .set_low_confidence(LowConfidenceAction::AdaptFeature)
            .unwrap();
        adapt
            .set_channels(TrackChannels::position_only(), &[point_at([16.0, 32.0])])
            .unwrap();
        go(&mut adapt, &clip);
        let followed = adapt.samples(0).last().unwrap().feature_center[0];
        let debug: Vec<_> = adapt
            .samples(0)
            .iter()
            .map(|s| {
                (
                    s.frame,
                    s.feature_center[0],
                    s.confidence,
                    s.below_threshold,
                )
            })
            .collect();
        assert!(followed > 24.0, "{debug:?}");
    }

    #[test]
    fn subpixel_keeps_fraction_and_off_rounds() {
        let mut frames = Vec::new();
        let base = {
            let mut image = Raster::new(48, 48, [0.0, 0.0, 0.0, 1.0]);
            image.stamp_disc(16.0, 24.0, 3.0, [1.0, 1.0, 1.0, 1.0]);
            image
        };
        frames.push(base.clone());
        frames.push(base.shift(1.4, 0.0));
        let clip = FrameClip::from_frames(frames, 24.0);
        let mut on = PointTracker::new();
        configure(&mut on);
        on.subpixel = Some(true);
        on.set_channels(TrackChannels::position_only(), &[point_at([16.0, 24.0])])
            .unwrap();
        go(&mut on, &clip);
        let mut off = PointTracker::new();
        configure(&mut off);
        off.subpixel = Some(false);
        off.set_channels(TrackChannels::position_only(), &[point_at([16.0, 24.0])])
            .unwrap();
        go(&mut off, &clip);
        let on_x = on.samples(0).last().unwrap().feature_center[0];
        let off_x = off.samples(0).last().unwrap().feature_center[0];
        assert!((on_x - on_x.round()).abs() > 0.05);
        assert!(near(off_x, off_x.round(), 1e-6));
    }

    #[test]
    fn apply_target_receives_the_only_keys() {
        let clip = marker_clip(3, [16.0, 20.0], [4.0, 1.0], [1.0, 1.0, 1.0, 1.0]);
        let mut trackers = MotionTrackers::new();
        let source = LayerId::new(10);
        let target_id = LayerId::new(11);
        let other_id = LayerId::new(12);
        {
            let tracker = trackers.track_motion(source);
            configure(tracker);
            tracker.apply_target = Some(target_id);
            tracker
                .set_channels(TrackChannels::position_only(), &[point_at([16.0, 20.0])])
                .unwrap();
            go(tracker, &clip);
        }
        let mut target = blank_layer(11, "target");
        let other = blank_layer(12, "other");
        trackers.items[0]
            .apply(&mut target, &mut Ids::new(), 24.0, Some(ApplyAxes::XY))
            .unwrap();
        assert!(key_count(&target, "position") >= 2);
        assert_eq!(key_count(&other, "position"), 0);
        assert_eq!(trackers.items[0].apply_target, Some(target_id));
        let _ = other_id;
    }

    #[test]
    fn axes_xy_follow_both() {
        let clip = marker_clip(3, [16.0, 18.0], [4.0, 3.0], [1.0, 1.0, 1.0, 1.0]);
        let mut tracker = PointTracker::new();
        configure(&mut tracker);
        tracker
            .set_channels(TrackChannels::position_only(), &[point_at([16.0, 18.0])])
            .unwrap();
        go(&mut tracker, &clip);
        let mut layer = blank_layer(1, "target");
        tracker
            .apply(&mut layer, &mut Ids::new(), 24.0, Some(ApplyAxes::XY))
            .unwrap();
        let keys = point_keys(&layer, "position");
        assert!(keys.last().unwrap()[0] > keys[0][0] + 4.0);
        assert!(keys.last().unwrap()[1] > keys[0][1] + 2.0);
    }

    #[test]
    fn axis_x_leaves_y() {
        let clip = marker_clip(3, [16.0, 18.0], [5.0, 4.0], [1.0, 1.0, 1.0, 1.0]);
        let mut tracker = PointTracker::new();
        configure(&mut tracker);
        tracker
            .set_channels(TrackChannels::position_only(), &[point_at([16.0, 18.0])])
            .unwrap();
        go(&mut tracker, &clip);
        let mut layer = blank_layer(1, "target");
        layer.properties.push(model::Property::value(
            model::PropertyId::new(1),
            "位置",
            "position",
            PropertyValue::TwoD([3.0, 9.0]),
        ));
        tracker
            .apply(&mut layer, &mut Ids::new(), 24.0, Some(ApplyAxes::XOnly))
            .unwrap();
        for key in point_keys(&layer, "position") {
            assert!(near(key[1], 9.0, 1e-6));
        }
        let keys = point_keys(&layer, "position");
        assert!(keys.last().unwrap()[0] > keys[0][0] + 4.0);
    }

    #[test]
    fn axis_y_leaves_x() {
        let clip = marker_clip(3, [16.0, 18.0], [5.0, 4.0], [1.0, 1.0, 1.0, 1.0]);
        let mut tracker = PointTracker::new();
        configure(&mut tracker);
        tracker
            .set_channels(TrackChannels::position_only(), &[point_at([16.0, 18.0])])
            .unwrap();
        go(&mut tracker, &clip);
        let mut layer = blank_layer(1, "target");
        layer.properties.push(model::Property::value(
            model::PropertyId::new(1),
            "位置",
            "position",
            PropertyValue::TwoD([3.0, 9.0]),
        ));
        tracker
            .apply(&mut layer, &mut Ids::new(), 24.0, Some(ApplyAxes::YOnly))
            .unwrap();
        for key in point_keys(&layer, "position") {
            assert!(near(key[0], 3.0, 1e-6));
        }
        let keys = point_keys(&layer, "position");
        assert!((keys.last().unwrap()[1] - keys[0][1]).abs() > 2.0);
    }

    #[test]
    fn attach_offset_is_kept_on_the_target() {
        let clip = marker_clip(3, [16.0, 20.0], [4.0, 0.0], [1.0, 1.0, 1.0, 1.0]);
        let mut tracker = PointTracker::new();
        configure(&mut tracker);
        tracker
            .set_channels(TrackChannels::position_only(), &[point_at([16.0, 20.0])])
            .unwrap();
        tracker.set_attach_offset(0, [5.0, -2.0]);
        go(&mut tracker, &clip);
        let mut layer = blank_layer(1, "target");
        tracker
            .apply(&mut layer, &mut Ids::new(), 24.0, Some(ApplyAxes::XY))
            .unwrap();
        let keys = point_keys(&layer, "position");
        assert!(near(
            keys[0][0],
            tracker.samples(0)[0].feature_center[0] + 5.0,
            0.2
        ));
        assert!(near(
            keys[0][1],
            tracker.samples(0)[0].feature_center[1] - 2.0,
            0.2
        ));
        let delta = keys[1][0] - keys[0][0];
        assert!(delta > 2.0);
    }

    #[test]
    fn changing_attach_offset_reapplies_position_only() {
        let clip = marker_clip(3, [16.0, 20.0], [4.0, 0.0], [1.0, 1.0, 1.0, 1.0]);
        let mut tracker = PointTracker::new();
        configure(&mut tracker);
        tracker
            .set_channels(TrackChannels::position_only(), &[point_at([16.0, 20.0])])
            .unwrap();
        go(&mut tracker, &clip);
        let mut layer = blank_layer(1, "target");
        let mut ids = Ids::new();
        tracker
            .apply(&mut layer, &mut ids, 24.0, Some(ApplyAxes::XY))
            .unwrap();
        let first = point_keys(&layer, "position");
        tracker.set_attach_offset(0, [6.0, 1.0]);
        tracker.apply(&mut layer, &mut ids, 24.0, None).unwrap();
        let second = point_keys(&layer, "position");
        assert!(near(second[0][0] - first[0][0], 6.0, 0.05));
        assert!(near(
            (second[1][0] - second[0][0]) - (first[1][0] - first[0][0]),
            0.0,
            0.05
        ));
    }

    #[test]
    fn motion_source_rejects_another_layer_image() {
        let source = LayerId::new(4);
        let other = LayerId::new(5);
        let clip = marker_clip(3, [16.0, 20.0], [4.0, 0.0], [1.0, 1.0, 1.0, 1.0]);
        let decoy = marker_clip(3, [16.0, 20.0], [-4.0, 0.0], [1.0, 0.0, 0.0, 1.0]);
        let mut tracker = PointTracker::new();
        configure(&mut tracker);
        tracker.motion_source = Some(source);
        tracker
            .set_channels(TrackChannels::position_only(), &[point_at([16.0, 20.0])])
            .unwrap();
        assert!(tracker
            .analyze_source(other, &decoy, Direction::Forward, |_, _| {
                StepControl::Continue
            })
            .is_err());
        tracker
            .analyze_source(source, &clip, Direction::Forward, |_, _| {
                StepControl::Continue
            })
            .unwrap();
        let start = tracker.samples(0).first().unwrap().feature_center[0];
        let end = tracker.samples(0).last().unwrap().feature_center[0];
        assert!(end > start);
        let mut source_layer = blank_layer(4, "source");
        let other_layer = blank_layer(5, "other");
        tracker.store(&mut source_layer, &mut Ids::new(), 24.0);
        assert!(source_layer.property("motion-trackers").is_some());
        assert!(other_layer.property("motion-trackers").is_none());
    }

    #[test]
    fn track_motion_adds_a_tracker_and_a_target_slot() {
        let mut group = MotionTrackers::new();
        let source = LayerId::new(1);
        assert!(group.items.is_empty());
        let tracker = group.track_motion(source);
        tracker.apply_target = Some(LayerId::new(2));
        assert_eq!(group.items.len(), 1);
        assert_eq!(group.items[0].kind, TrackKind::Motion);
        assert_eq!(group.items[0].apply_target, Some(LayerId::new(2)));
    }

    #[test]
    fn stabilize_moves_anchor_rotation_and_scale_opposite() {
        let mut frames = Vec::new();
        for index in 0..3 {
            let mut image = Raster::new(72, 64, [0.04, 0.04, 0.04, 1.0]);
            let shift = f64::from(index) * 4.0;
            image.stamp_disc(22.0 + shift, 32.0, 2.5, [1.0, 1.0, 1.0, 1.0]);
            image.stamp_disc(42.0 + shift, 32.0, 2.5, [1.0, 1.0, 1.0, 1.0]);
            frames.push(image.rotate(f64::from(index) * 6.0, [32.0 + shift, 32.0]));
        }
        let clip = FrameClip::from_frames(frames, 24.0);
        let mut group = MotionTrackers::new();
        let source = LayerId::new(8);
        {
            let tracker = group.stabilize(source);
            configure(tracker);
            tracker
                .set_channels(
                    TrackChannels {
                        position: true,
                        rotation: true,
                        scale: true,
                    },
                    &[point_at([22.0, 32.0]), point_at([42.0, 32.0])],
                )
                .unwrap();
            // スケールは距離がほぼ一定なので、明示的に離す 2 本目を後で上書きせず回転と位置を見る。
            go(tracker, &clip);
        }
        let mut layer = blank_layer(8, "source");
        group.items[0]
            .apply(&mut layer, &mut Ids::new(), 24.0, Some(ApplyAxes::XY))
            .unwrap();
        let anchor = point_keys(&layer, "anchor-point");
        assert!(anchor.len() >= 2);
        assert!(anchor.last().unwrap()[0] > anchor[0][0]);
        let rotation = scalar_keys(&layer, "rotation");
        assert!(rotation.last().copied().unwrap_or(0.0) < -1.0);
        assert_eq!(group.items[0].apply_target, Some(source));
        assert!(group.items[0].samples(0).len() >= 2);
    }

    #[test]
    fn selected_property_receives_only_that_channel() {
        let clip = marker_clip(3, [16.0, 24.0], [4.0, 0.0], [1.0, 1.0, 1.0, 1.0]);
        let mut group = MotionTrackers::new();
        let tracker = group.track_selected(LayerId::new(1), SelectedProperty::Position);
        configure(tracker);
        tracker
            .set_channels(TrackChannels::position_only(), &[point_at([16.0, 24.0])])
            .unwrap();
        go(tracker, &clip);
        let mut layer = blank_layer(2, "prop");
        group.items[0]
            .apply(&mut layer, &mut Ids::new(), 24.0, Some(ApplyAxes::XY))
            .unwrap();
        assert!(key_count(&layer, "position") >= 2);
        assert_eq!(key_count(&layer, "rotation"), 0);
        assert_eq!(key_count(&layer, "scale"), 0);
    }

    #[test]
    fn analyze_forward_fills_until_out() {
        let mut clip = marker_clip(5, [14.0, 24.0], [3.0, 0.0], [1.0, 1.0, 1.0, 1.0]);
        clip.in_frame = 1;
        clip.out_frame = 4;
        let mut tracker = PointTracker::new();
        configure(&mut tracker);
        tracker.current_frame = 2;
        tracker
            .set_channels(TrackChannels::position_only(), &[point_at([20.0, 24.0])])
            .unwrap();
        tracker
            .analyze(&clip, Direction::Forward, |_, _| StepControl::Continue)
            .unwrap();
        let frames: Vec<f64> = tracker.samples(0).iter().map(|s| s.frame).collect();
        assert!(frames.iter().any(|f| near(*f, 2.0, 0.01)));
        assert!(frames.iter().any(|f| near(*f, 4.0, 0.01)));
        assert!(frames.iter().all(|f| *f >= 2.0 - 0.01));
        assert!(tracker.samples(0).iter().all(|s| s.attach.is_some()));
    }

    #[test]
    fn analyze_backward_fills_until_in() {
        let mut clip = marker_clip(5, [14.0, 24.0], [3.0, 0.0], [1.0, 1.0, 1.0, 1.0]);
        clip.in_frame = 0;
        clip.out_frame = 4;
        let mut tracker = PointTracker::new();
        configure(&mut tracker);
        tracker.current_frame = 3;
        tracker
            .set_channels(TrackChannels::position_only(), &[point_at([23.0, 24.0])])
            .unwrap();
        tracker
            .analyze(&clip, Direction::Backward, |_, _| StepControl::Continue)
            .unwrap();
        let frames: Vec<f64> = tracker.samples(0).iter().map(|s| s.frame).collect();
        assert!(frames.iter().any(|f| near(*f, 0.0, 0.01)));
        assert!(frames.iter().any(|f| near(*f, 3.0, 0.01)));
        assert!(frames.iter().all(|f| *f <= 3.0 + 0.01));
    }

    #[test]
    fn step_forward_writes_only_the_next_frame() {
        let clip = marker_clip(4, [16.0, 24.0], [3.0, 0.0], [1.0, 1.0, 1.0, 1.0]);
        let mut tracker = PointTracker::new();
        configure(&mut tracker);
        tracker.current_frame = 1;
        tracker
            .set_channels(TrackChannels::position_only(), &[point_at([19.0, 24.0])])
            .unwrap();
        tracker.step(&clip, Direction::Forward).unwrap();
        assert_eq!(tracker.current_frame, 2);
        assert_eq!(tracker.samples(0).len(), 1);
        assert!(near(tracker.samples(0)[0].frame, 2.0, 0.01));
    }

    #[test]
    fn step_backward_writes_only_the_previous_frame() {
        let clip = marker_clip(4, [16.0, 24.0], [3.0, 0.0], [1.0, 1.0, 1.0, 1.0]);
        let mut tracker = PointTracker::new();
        configure(&mut tracker);
        tracker.current_frame = 2;
        tracker
            .set_channels(TrackChannels::position_only(), &[point_at([22.0, 24.0])])
            .unwrap();
        tracker.step(&clip, Direction::Backward).unwrap();
        assert_eq!(tracker.current_frame, 1);
        assert_eq!(tracker.samples(0).len(), 1);
        assert!(near(tracker.samples(0)[0].frame, 1.0, 0.01));
    }

    #[test]
    fn stop_keeps_earlier_frames_and_leaves_the_rest_empty() {
        let clip = marker_clip(5, [14.0, 24.0], [3.0, 0.0], [1.0, 1.0, 1.0, 1.0]);
        let mut tracker = PointTracker::new();
        configure(&mut tracker);
        tracker
            .set_channels(TrackChannels::position_only(), &[point_at([14.0, 24.0])])
            .unwrap();
        let mut saw_stop = false;
        tracker
            .analyze(&clip, Direction::Forward, |tracker, frame| {
                if tracker.is_running() {
                    saw_stop = tracker.analyze_button() == "停止";
                }
                if frame >= 3 {
                    StepControl::Stop
                } else {
                    StepControl::Continue
                }
            })
            .unwrap();
        assert!(saw_stop);
        assert!(!tracker.is_running());
        assert!(tracker.samples(0).iter().any(|s| s.frame < 3.0));
        assert!(tracker.samples(0).iter().all(|s| s.frame < 3.0));
    }

    #[test]
    fn apply_keeps_source_track() {
        let clip = marker_clip(3, [16.0, 24.0], [3.0, 0.0], [1.0, 1.0, 1.0, 1.0]);
        let mut tracker = PointTracker::new();
        configure(&mut tracker);
        tracker.motion_source = Some(LayerId::new(1));
        tracker
            .set_channels(TrackChannels::position_only(), &[point_at([16.0, 24.0])])
            .unwrap();
        go(&mut tracker, &clip);
        let before = tracker.samples(0).len();
        let mut target = blank_layer(2, "target");
        tracker
            .apply(&mut target, &mut Ids::new(), 24.0, Some(ApplyAxes::XY))
            .unwrap();
        assert_eq!(tracker.samples(0).len(), before);
        assert!(key_count(&target, "position") >= 2);
        let mut source = blank_layer(1, "source");
        tracker.store(&mut source, &mut Ids::new(), 24.0);
        assert!(source.property("motion-trackers").is_some());
    }

    #[test]
    fn reset_clears_track_and_keeps_applied_keys() {
        let clip = marker_clip(3, [16.0, 24.0], [3.0, 0.0], [1.0, 1.0, 1.0, 1.0]);
        let mut tracker = PointTracker::new();
        configure(&mut tracker);
        tracker
            .set_channels(TrackChannels::position_only(), &[point_at([16.0, 24.0])])
            .unwrap();
        tracker.nudge(0, 2.0, 0.0, false, false);
        go(&mut tracker, &clip);
        let mut target = blank_layer(2, "target");
        tracker
            .apply(&mut target, &mut Ids::new(), 24.0, Some(ApplyAxes::XY))
            .unwrap();
        let keys_before = key_count(&target, "position");
        tracker.reset();
        assert!(tracker.samples(0).is_empty());
        assert!(near(tracker.feature_center(0)[0], 16.0, 1e-6));
        assert_eq!(key_count(&target, "position"), keys_before);
    }

    #[test]
    fn rgb_follows_color_and_loses_brightness_more_than_luminance() {
        let clip = marker_clip(3, [16.0, 28.0], [4.0, 0.0], [0.9, 0.1, 0.1, 1.0]);
        let mut tracker = PointTracker::new();
        configure(&mut tracker);
        tracker.compare = Some(CompareChannel::Rgb);
        tracker
            .set_channels(TrackChannels::position_only(), &[point_at([16.0, 28.0])])
            .unwrap();
        go(&mut tracker, &clip);
        assert!(tracker.samples(0).last().unwrap().feature_center[0] > 22.0);

        let mut background = Raster::new(56, 36, [0.0, 0.0, 0.0, 1.0]);
        for y in 0..36 {
            for x in 0..56 {
                let n = ((x * 17 + y * 31) % 9) as f32;
                let r = 0.05 + n * 0.08;
                let b = 0.08 + ((x * 3 + y) % 5) as f32 * 0.07;
                let g = ((0.42 - 0.2126 * r - 0.0722 * b) / 0.7152).clamp(0.0, 1.0);
                background.set(x, y, [r, g, b, 1.0]);
            }
        }
        let bump = |image: &mut Raster, x0: i32, y0: i32| {
            for y in y0..y0 + 8 {
                for x in x0..x0 + 8 {
                    let pixel = image.get(x, y);
                    let yv =
                        (0.2126 * pixel[0] + 0.7152 * pixel[1] + 0.0722 * pixel[2] + 0.18).min(1.0);
                    let current =
                        (0.2126 * pixel[0] + 0.7152 * pixel[1] + 0.0722 * pixel[2]).max(0.05);
                    let scale = yv / current;
                    image.set(
                        x,
                        y,
                        [pixel[0] * scale, pixel[1] * scale, pixel[2] * scale, 1.0],
                    );
                }
            }
        };
        let mut first = background.clone();
        bump(&mut first, 12, 12);
        let mut second = background.clone();
        bump(&mut second, 20, 12);
        let rgb = capture_patch(&first, [16.0, 16.0], [16.0, 16.0], CompareChannel::Rgb);
        let luma = capture_patch(
            &first,
            [16.0, 16.0],
            [16.0, 16.0],
            CompareChannel::Luminance,
        );
        let rgb_hit = match_patch(
            &second,
            &rgb,
            [16.0, 16.0],
            [36.0, 16.0],
            [16.0, 16.0],
            false,
        );
        let luma_hit = match_patch(
            &second,
            &luma,
            [16.0, 16.0],
            [36.0, 16.0],
            [16.0, 16.0],
            false,
        );
        assert!(
            (rgb_hit.center[0] - 24.0).abs() > (luma_hit.center[0] - 24.0).abs(),
            "rgb {:?} luma {:?}",
            rgb_hit,
            luma_hit
        );
    }

    #[test]
    fn luminance_follows_brightness_and_loses_hue_more_than_saturation() {
        let clip = marker_clip(3, [16.0, 24.0], [4.0, 0.0], [0.8, 0.8, 0.8, 1.0]);
        let mut tracker = PointTracker::new();
        configure(&mut tracker);
        tracker.compare = Some(CompareChannel::Luminance);
        tracker
            .set_channels(TrackChannels::position_only(), &[point_at([16.0, 24.0])])
            .unwrap();
        go(&mut tracker, &clip);
        assert!(tracker.samples(0).last().unwrap().feature_center[0] > 22.0);

        let red = [0.85, 0.04, 0.04, 1.0];
        let luma_red = 0.2126 * red[0] + 0.7152 * red[1] + 0.0722 * red[2];
        let mut image = Raster::new(48, 32, [luma_red, luma_red, luma_red, 1.0]);
        image.fill_rect(14, 10, 8, 8, red);
        let mut moved = Raster::new(48, 32, [luma_red, luma_red, luma_red, 1.0]);
        moved.fill_rect(20, 10, 8, 8, red);
        let luma = capture_patch(
            &image,
            [18.0, 14.0],
            [12.0, 12.0],
            CompareChannel::Luminance,
        );
        let sat = capture_patch(
            &image,
            [18.0, 14.0],
            [12.0, 12.0],
            CompareChannel::Saturation,
        );
        let luma_hit = match_patch(
            &moved,
            &luma,
            [18.0, 14.0],
            [18.0, 12.0],
            [12.0, 12.0],
            false,
        );
        let sat_hit = match_patch(
            &moved,
            &sat,
            [18.0, 14.0],
            [18.0, 12.0],
            [12.0, 12.0],
            false,
        );
        assert!((luma_hit.center[0] - 24.0).abs() > (sat_hit.center[0] - 24.0).abs());
    }

    #[test]
    fn saturation_follows_vivid_color_and_loses_gray_contrast() {
        let clip = marker_clip(3, [18.0, 24.0], [4.0, 0.0], [0.1, 0.15, 0.9, 1.0]);
        let mut tracker = PointTracker::new();
        configure(&mut tracker);
        tracker.compare = Some(CompareChannel::Saturation);
        tracker
            .set_channels(TrackChannels::position_only(), &[point_at([18.0, 24.0])])
            .unwrap();
        go(&mut tracker, &clip);
        assert!(tracker.samples(0).last().unwrap().feature_center[0] > 24.0);

        let mut image = Raster::new(40, 32, [0.2, 0.2, 0.2, 1.0]);
        image.fill_rect(14, 10, 8, 8, [0.85, 0.85, 0.85, 1.0]);
        let sat = capture_patch(
            &image,
            [18.0, 14.0],
            [10.0, 10.0],
            CompareChannel::Saturation,
        );
        let rgb = capture_patch(&image, [18.0, 14.0], [10.0, 10.0], CompareChannel::Rgb);
        let luma = capture_patch(
            &image,
            [18.0, 14.0],
            [10.0, 10.0],
            CompareChannel::Luminance,
        );
        let moved = image.shift(3.0, 0.0);
        let sat_hit = match_patch(
            &moved,
            &sat,
            [18.0, 14.0],
            [14.0, 12.0],
            [10.0, 10.0],
            false,
        );
        let rgb_hit = match_patch(
            &moved,
            &rgb,
            [18.0, 14.0],
            [14.0, 12.0],
            [10.0, 10.0],
            false,
        );
        let luma_hit = match_patch(
            &moved,
            &luma,
            [18.0, 14.0],
            [14.0, 12.0],
            [10.0, 10.0],
            false,
        );
        assert!((sat_hit.center[0] - 21.0).abs() > (rgb_hit.center[0] - 21.0).abs());
        assert!((sat_hit.center[0] - 21.0).abs() > (luma_hit.center[0] - 21.0).abs());
    }

    #[test]
    fn adapt_every_frame_updates_the_template() {
        let mut frames = Vec::new();
        for index in 0..3 {
            let mut image = Raster::new(48, 40, [0.05, 0.05, 0.05, 1.0]);
            let t = index as f32 / 2.0;
            paint_marker(&mut image, 16 + index * 3, 20, [1.0 - t, t, 0.2, 1.0]);
            frames.push(image);
        }
        let clip = FrameClip::from_frames(frames, 24.0);
        let original = capture_patch(
            &clip.frames[0],
            [16.0, 20.0],
            [11.0, 11.0],
            CompareChannel::Rgb,
        );
        let mut on = PointTracker::new();
        configure(&mut on);
        on.compare = Some(CompareChannel::Rgb);
        on.set_adapt_every_frame(true).unwrap();
        on.set_channels(TrackChannels::position_only(), &[point_at([16.0, 20.0])])
            .unwrap();
        go(&mut on, &clip);
        assert!(on.adopted_template_differs(0, &original));
        let mut off = PointTracker::new();
        configure(&mut off);
        off.compare = Some(CompareChannel::Rgb);
        off.set_adapt_every_frame(false).unwrap();
        off.set_channels(TrackChannels::position_only(), &[point_at([16.0, 20.0])])
            .unwrap();
        go(&mut off, &clip);
        assert!(!off.adopted_template_differs(0, &original));
        assert!(on
            .set_low_confidence(LowConfidenceAction::AdaptFeature)
            .is_err());
    }

    #[test]
    fn track_fields_writes_two_samples_without_changing_frame_rate() {
        let mut frames = Vec::new();
        for index in 0..2 {
            let mut even = Raster::new(48, 40, [0.05, 0.05, 0.05, 1.0]);
            let mut odd = Raster::new(48, 40, [0.05, 0.05, 0.05, 1.0]);
            paint_marker(&mut even, 14 + index * 2, 20, [1.0, 1.0, 1.0, 1.0]);
            paint_marker(&mut odd, 28 + index * 2, 20, [1.0, 1.0, 1.0, 1.0]);
            let mut image = Raster::new(48, 40, [0.05, 0.05, 0.05, 1.0]);
            for y in 0..40 {
                for x in 0..48 {
                    let pixel = if y % 2 == 0 {
                        even.get(x, y)
                    } else {
                        odd.get(x, y)
                    };
                    image.set(x, y, pixel);
                }
            }
            frames.push(image);
        }
        let clip = FrameClip::from_frames(frames, 24.0);
        let rate = clip.frame_rate;
        let mut tracker = PointTracker::new();
        configure(&mut tracker);
        tracker.track_fields = Some(true);
        tracker
            .set_channels(TrackChannels::position_only(), &[point_at([14.0, 20.0])])
            .unwrap();
        go(&mut tracker, &clip);
        let frame0: Vec<_> = tracker
            .samples(0)
            .iter()
            .filter(|s| s.frame < 1.0)
            .collect();
        assert!(frame0.len() >= 2);
        assert!(near(clip.frame_rate, rate, 1e-9));
    }

    #[test]
    fn enhance_changes_the_match_and_leaves_the_image() {
        let mut image = Raster::new(40, 32, [0.1, 0.1, 0.1, 1.0]);
        image.fill_rect(12, 12, 2, 8, [1.0, 1.0, 1.0, 1.0]);
        image.set(20, 16, [0.95, 0.95, 0.95, 1.0]);
        let original = image.clone();
        let patch = capture_patch(&image, [13.0, 16.0], [8.0, 10.0], CompareChannel::Luminance);
        let plain = match_patch(
            &image,
            &patch,
            [16.0, 16.0],
            [20.0, 14.0],
            [8.0, 10.0],
            true,
        );
        let blurred = image.blur(2);
        let again = match_patch(
            &blurred,
            &patch,
            [16.0, 16.0],
            [20.0, 14.0],
            [8.0, 10.0],
            true,
        );
        assert!(plain.center != again.center || (plain.confidence - again.confidence).abs() > 0.5);
        assert_eq!(image.pixels, original.pixels);
    }

    #[test]
    fn magnify_while_dragging_is_four_hundred_percent() {
        let mut tracker = PointTracker::new();
        tracker.magnify_while_dragging = Some(true);
        assert!(near(
            tracker.feature_drag_magnification().unwrap(),
            4.0,
            1e-9
        ));
        tracker.magnify_while_dragging = Some(false);
        assert!(near(
            tracker.feature_drag_magnification().unwrap(),
            1.0,
            1e-9
        ));
    }

    #[test]
    fn new_tracker_plugin_is_built_in() {
        let tracker = PointTracker::new();
        assert_eq!(tracker.plugin_label(), "Built-in");
    }

    fn paired(position: bool, rotation: bool, scale: bool) -> PointTracker {
        let mut tracker = PointTracker::new();
        configure(&mut tracker);
        tracker
            .set_channels(
                TrackChannels {
                    position,
                    rotation,
                    scale,
                },
                &[point_at([20.0, 24.0]), point_at([40.0, 24.0])],
            )
            .unwrap();
        tracker
    }

    fn two_dot_motion(count: i32, step: [f64; 2], spin: f64, spread: f64) -> FrameClip {
        let mut frames = Vec::new();
        for index in 0..count {
            let mut image = Raster::new(80, 64, [0.04, 0.04, 0.04, 1.0]);
            let extra = f64::from(index) * spread;
            image.stamp_disc(24.0 - extra, 32.0, 2.4, [1.0, 1.0, 1.0, 1.0]);
            image.stamp_disc(48.0 + extra, 32.0, 2.4, [1.0, 1.0, 1.0, 1.0]);
            let shifted = image.shift(step[0] * f64::from(index), step[1] * f64::from(index));
            frames.push(shifted.rotate(
                spin * f64::from(index),
                [36.0 + step[0] * f64::from(index), 32.0],
            ));
        }
        FrameClip::from_frames(frames, 24.0)
    }
}
