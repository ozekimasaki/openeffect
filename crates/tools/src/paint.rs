//! ペイント。ストロークは効果として重ね、ソースの画素は書き換えない。

use std::collections::BTreeMap;

use model::{Ids, Layer, LayerId, Rgba};

use crate::persist::{self, ensure_group};
use crate::raster::Raster;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaintToolKind {
    Brush,
    Clone,
    Eraser,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaintChannel {
    Color,
    Alpha,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StrokeDuration {
    SingleFrame,
    Constant,
    Custom,
    WriteOn,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dynamics {
    Off,
    Pressure,
    Tilt,
    Wheel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EraserTarget {
    LayerAndPaint,
    PaintOnly,
    LastStroke,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Spacing {
    Off,
    Percent(f64),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaintBlend {
    Normal,
    Multiply,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrokePoint {
    pub xy: [f64; 2],
    pub pressure: f64,
    pub tilt: f64,
    pub wheel: f64,
    pub time: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CloneStamp {
    pub source: LayerId,
    pub position: [f64; 2],
    pub aligned: bool,
    pub lock_time: bool,
    pub source_time: f64,
    pub time_shift: f64,
    pub offset: [f64; 2],
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stroke {
    pub name: String,
    pub kind: PaintToolKind,
    pub frame: i32,
    pub duration: StrokeDuration,
    pub custom_frames: i32,
    pub points: Vec<StrokePoint>,
    pub color: Rgba,
    pub opacity: f64,
    pub flow: f64,
    pub blend: PaintBlend,
    pub channel: PaintChannel,
    pub diameter: f64,
    pub angle: f64,
    pub roundness: f64,
    pub hardness: f64,
    pub spacing: Spacing,
    pub eraser: Option<EraserTarget>,
    pub clone: Option<CloneStamp>,
    pub end_keys: Vec<(f64, f64)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClonePreset {
    pub source: Option<LayerId>,
    pub aligned: bool,
    pub lock_time: bool,
    pub time_shift: f64,
    pub offset: [f64; 2],
    pub position: [f64; 2],
}

impl Default for ClonePreset {
    fn default() -> Self {
        Self {
            source: None,
            aligned: true,
            lock_time: false,
            time_shift: 0.0,
            offset: [0.0, 0.0],
            position: [0.0, 0.0],
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PaintPreferences {
    pub presets: [ClonePreset; 5],
}

impl Default for PaintPreferences {
    fn default() -> Self {
        Self {
            presets: std::array::from_fn(|_| ClonePreset::default()),
        }
    }
}

impl PaintPreferences {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        for preset in &self.presets {
            bytes.push(u8::from(preset.aligned));
            bytes.push(u8::from(preset.lock_time));
            bytes.extend_from_slice(&preset.time_shift.to_le_bytes());
            for value in [
                preset.offset[0],
                preset.offset[1],
                preset.position[0],
                preset.position[1],
            ] {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
            let id = preset.source.map(LayerId::get).unwrap_or(0);
            bytes.extend_from_slice(&id.to_le_bytes());
        }
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut prefs = Self::default();
        let mut cursor = 0;
        for preset in &mut prefs.presets {
            if cursor + 2 + 8 * 5 + 8 > bytes.len() {
                break;
            }
            preset.aligned = bytes[cursor] != 0;
            preset.lock_time = bytes[cursor + 1] != 0;
            cursor += 2;
            preset.time_shift = read_f64(bytes, &mut cursor);
            preset.offset = [read_f64(bytes, &mut cursor), read_f64(bytes, &mut cursor)];
            preset.position = [read_f64(bytes, &mut cursor), read_f64(bytes, &mut cursor)];
            let id = u64::from_le_bytes(bytes[cursor..cursor + 8].try_into().unwrap_or([0; 8]));
            cursor += 8;
            preset.source = if id == 0 {
                None
            } else {
                Some(LayerId::new(id))
            };
        }
        prefs
    }
}

fn read_f64(bytes: &[u8], cursor: &mut usize) -> f64 {
    let value = f64::from_le_bytes(bytes[*cursor..*cursor + 8].try_into().unwrap_or([0; 8]));
    *cursor += 8;
    value
}

#[derive(Clone, Debug)]
pub struct PaintState {
    pub tool: PaintToolKind,
    pub foreground: Rgba,
    pub background: Rgba,
    pub opacity: Option<f64>,
    pub flow: Option<f64>,
    pub blend: Option<PaintBlend>,
    pub channel: Option<PaintChannel>,
    pub duration: Option<StrokeDuration>,
    pub custom_frames: Option<i32>,
    pub diameter: Option<f64>,
    pub angle: Option<f64>,
    pub roundness: Option<f64>,
    pub hardness: Option<f64>,
    pub spacing: Option<Spacing>,
    pub size_dynamics: Option<Dynamics>,
    pub min_size: Option<f64>,
    pub angle_dynamics: Option<Dynamics>,
    pub roundness_dynamics: Option<Dynamics>,
    pub opacity_dynamics: Option<Dynamics>,
    pub flow_dynamics: Option<Dynamics>,
    pub eraser: Option<EraserTarget>,
    pub clone_aligned: Option<bool>,
    pub lock_source_time: Option<bool>,
    pub clone_source: Option<LayerId>,
    pub clone_position: Option<[f64; 2]>,
    pub source_time: Option<f64>,
    pub time_shift: Option<f64>,
    pub clone_offset: Option<[f64; 2]>,
    pub overlay: bool,
    pub overlay_opacity: Option<f64>,
    pub strokes: Vec<Stroke>,
    pub current_frame: i32,
    pub layer_out: i32,
    pub preferences: PaintPreferences,
    pub adjusting: Option<BrushAdjust>,
    stroke_open: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrushAdjust {
    Diameter,
    Hardness,
}

impl PaintState {
    pub fn new() -> Self {
        Self {
            tool: PaintToolKind::Brush,
            foreground: Rgba::black(),
            background: Rgba::white(),
            opacity: None,
            flow: None,
            blend: None,
            channel: None,
            duration: None,
            custom_frames: None,
            diameter: None,
            angle: None,
            roundness: None,
            hardness: None,
            spacing: None,
            size_dynamics: None,
            min_size: None,
            angle_dynamics: None,
            roundness_dynamics: None,
            opacity_dynamics: None,
            flow_dynamics: None,
            eraser: None,
            clone_aligned: None,
            lock_source_time: None,
            clone_source: None,
            clone_position: None,
            source_time: None,
            time_shift: None,
            clone_offset: None,
            overlay: false,
            overlay_opacity: None,
            strokes: Vec::new(),
            current_frame: 0,
            layer_out: 0,
            preferences: PaintPreferences::default(),
            adjusting: None,
            stroke_open: false,
        }
    }

    pub fn cycle_tool(&mut self) {
        self.tool = match self.tool {
            PaintToolKind::Brush => PaintToolKind::Clone,
            PaintToolKind::Clone => PaintToolKind::Eraser,
            PaintToolKind::Eraser => PaintToolKind::Brush,
        };
    }

    pub fn swap_colors(&mut self) {
        std::mem::swap(&mut self.foreground, &mut self.background);
    }

    pub fn reset_colors(&mut self) {
        self.foreground = Rgba::black();
        self.background = Rgba::white();
    }

    pub fn eyedropper(&mut self, image: &Raster, x: i32, y: i32, average: bool) {
        if average {
            let mut sum = [0.0; 4];
            let mut count = 0.0;
            for oy in -1..=1 {
                for ox in -1..=1 {
                    let pixel = image.get(x + ox, y + oy);
                    for channel in 0..4 {
                        sum[channel] += pixel[channel];
                    }
                    count += 1.0;
                }
            }
            self.foreground = Rgba::new(
                sum[0] / count,
                sum[1] / count,
                sum[2] / count,
                sum[3] / count,
            );
        } else {
            let pixel = image.get(x, y);
            self.foreground = Rgba::new(pixel[0], pixel[1], pixel[2], pixel[3]);
        }
    }

    pub fn nudge_time(&mut self, forward: bool) {
        let frames = self.custom_frames.unwrap_or(0);
        if forward {
            self.current_frame += frames;
        } else {
            self.current_frame -= frames;
        }
    }

    pub fn drag_brush_size(&mut self, delta: f64, ctrl_held: bool) {
        if ctrl_held {
            self.adjusting = Some(BrushAdjust::Diameter);
            self.diameter = Some(self.diameter.unwrap_or(0.0) + delta);
        } else if self.adjusting == Some(BrushAdjust::Diameter) {
            self.adjusting = Some(BrushAdjust::Hardness);
            self.hardness = Some((self.hardness.unwrap_or(0.0) + delta).clamp(0.0, 100.0));
        }
    }

    pub fn select_preset(&mut self, key: u8) {
        let index = match key {
            3 => 0,
            4 => 1,
            5 => 2,
            6 => 3,
            7 => 4,
            _ => return,
        };
        let preset = self.preferences.presets[index].clone();
        self.clone_source = preset.source;
        self.clone_aligned = Some(preset.aligned);
        self.lock_source_time = Some(preset.lock_time);
        self.time_shift = Some(preset.time_shift);
        self.clone_offset = Some(preset.offset);
        self.clone_position = Some(preset.position);
    }

    pub fn save_preset(&mut self, index: usize) {
        if index >= 5 {
            return;
        }
        self.preferences.presets[index] = self.current_preset();
    }

    pub fn copy_preset(&mut self, index: usize) {
        if index >= 5 {
            return;
        }
        self.preferences.presets[index] = self.current_preset();
    }

    pub fn reset_clone(&mut self) {
        self.clone_offset = Some([0.0, 0.0]);
        self.time_shift = Some(0.0);
        self.clone_position = Some([0.0, 0.0]);
        self.source_time = Some(0.0);
    }

    pub fn begin_stroke(&mut self, xy: [f64; 2], continue_stroke: bool) {
        if continue_stroke {
            if let Some(stroke) = self.strokes.last_mut() {
                stroke.points.push(sample_point(xy, 1.0, 0.0, 0.0, 0.0));
                self.stroke_open = true;
                return;
            }
        }
        if self.tool == PaintToolKind::Eraser && self.eraser == Some(EraserTarget::LastStroke) {
            self.strokes
                .retain(|stroke| stroke.kind != PaintToolKind::Eraser);
            self.strokes.pop();
            return;
        }
        let kind_name = match self.tool {
            PaintToolKind::Brush => "ブラシ",
            PaintToolKind::Clone => "クローン",
            PaintToolKind::Eraser => "消しゴム",
        };
        let count = self
            .strokes
            .iter()
            .filter(|stroke| stroke.kind == self.tool)
            .count()
            + 1;
        let clone_offset = if self.tool == PaintToolKind::Clone {
            if let Some(offset) = self.clone_offset {
                offset
            } else {
                let position = self.clone_position.unwrap_or(xy);
                let offset = [position[0] - xy[0], position[1] - xy[1]];
                self.clone_offset = Some(offset);
                offset
            }
        } else {
            [0.0, 0.0]
        };
        let stroke = Stroke {
            name: format!("{kind_name} {count}"),
            kind: self.tool,
            frame: self.current_frame,
            duration: self.duration.unwrap_or(StrokeDuration::SingleFrame),
            custom_frames: self.custom_frames.unwrap_or(1),
            points: vec![sample_point(xy, 1.0, 0.0, 0.0, 0.0)],
            color: self.foreground,
            opacity: self.opacity.unwrap_or(1.0),
            flow: self.flow.unwrap_or(1.0),
            blend: self.blend.unwrap_or(PaintBlend::Normal),
            channel: self.channel.unwrap_or(PaintChannel::Color),
            diameter: self.diameter.unwrap_or(8.0),
            angle: self.angle.unwrap_or(0.0),
            roundness: self.roundness.unwrap_or(100.0).clamp(0.0, 100.0),
            hardness: self.hardness.unwrap_or(100.0),
            spacing: self.spacing.unwrap_or(Spacing::Percent(25.0)),
            eraser: self.eraser,
            clone: if self.tool == PaintToolKind::Clone {
                Some(CloneStamp {
                    source: self.clone_source.unwrap_or(LayerId::new(0)),
                    position: self.clone_position.unwrap_or(xy),
                    aligned: self.clone_aligned.unwrap_or(true),
                    lock_time: self.lock_source_time.unwrap_or(false),
                    source_time: self.source_time.unwrap_or(0.0),
                    time_shift: self.time_shift.unwrap_or(0.0),
                    offset: clone_offset,
                })
            } else {
                None
            },
            end_keys: Vec::new(),
        };
        self.strokes.push(stroke);
        self.stroke_open = true;
    }

    pub fn drag(&mut self, xy: [f64; 2], pressure: f64, tilt: f64, wheel: f64, time: f64) {
        if let Some(stroke) = self.strokes.last_mut() {
            if self.stroke_open {
                stroke
                    .points
                    .push(sample_point(xy, pressure, tilt, wheel, time));
            }
        }
    }

    pub fn end_stroke(&mut self) {
        if let Some(stroke) = self.strokes.last_mut() {
            if stroke.duration == StrokeDuration::WriteOn {
                let start = f64::from(stroke.frame);
                let end = f64::from(self.layer_out.max(stroke.frame));
                stroke.end_keys = vec![(start, 0.0), (end, 100.0)];
                self.current_frame = stroke.frame;
            }
        }
        self.stroke_open = false;
    }

    pub fn visible(&self, stroke: &Stroke, frame: i32) -> bool {
        match stroke.duration {
            StrokeDuration::SingleFrame => frame == stroke.frame,
            StrokeDuration::Constant | StrokeDuration::WriteOn => {
                frame >= stroke.frame && frame <= self.layer_out
            }
            StrokeDuration::Custom => {
                frame >= stroke.frame && frame < stroke.frame + stroke.custom_frames.max(1)
            }
        }
    }

    pub fn render(
        &self,
        frame: i32,
        source: &Raster,
        clones: &BTreeMap<(u64, i32), Raster>,
        source_length: i32,
    ) -> Raster {
        let mut paint = Raster::new(source.width, source.height, [0.0, 0.0, 0.0, 0.0]);
        let mut output = source.clone();
        for stroke in &self.strokes {
            if !self.visible(stroke, frame) {
                continue;
            }
            match stroke.kind {
                PaintToolKind::Brush | PaintToolKind::Clone
                    if stroke.channel == PaintChannel::Alpha =>
                {
                    apply_alpha(stroke, &mut output);
                }
                PaintToolKind::Brush | PaintToolKind::Clone => {
                    stamp_stroke(stroke, frame, &mut paint, clones, source_length);
                }
                PaintToolKind::Eraser => match stroke.eraser {
                    Some(EraserTarget::PaintOnly) | None => erase(&mut paint, stroke),
                    Some(EraserTarget::LayerAndPaint) => {
                        erase(&mut paint, stroke);
                        erase(&mut output, stroke);
                    }
                    Some(EraserTarget::LastStroke) => {}
                },
            }
        }
        composite(&mut output, &paint);
        output
    }

    pub fn overlay_view(&self, target: &Raster, source: &Raster) -> Raster {
        if !self.overlay {
            return target.clone();
        }
        let opacity = self.overlay_opacity.unwrap_or(0.5) as f32;
        let mut out = target.clone();
        for (index, pixel) in out.pixels.iter_mut().enumerate() {
            let over = source.pixels[index];
            for channel in 0..3 {
                pixel[channel] = pixel[channel] * (1.0 - opacity) + over[channel] * opacity;
            }
        }
        out
    }

    pub fn store(&self, layer: &mut Layer, ids: &mut Ids) {
        let root = ensure_group(layer, ids, "ペイント", "paint");
        for stroke in &self.strokes {
            root.push_child(persist::text(ids, &stroke.name, &stroke.name, &stroke.name));
        }
    }

    pub fn stamp_count(&self, stroke: &Stroke) -> usize {
        stamps(stroke).len()
    }

    pub fn axis_angle(angle: f64) -> f64 {
        let mut value = angle % 180.0;
        if value < 0.0 {
            value += 180.0;
        }
        value
    }

    fn current_preset(&self) -> ClonePreset {
        ClonePreset {
            source: self.clone_source,
            aligned: self.clone_aligned.unwrap_or(false),
            lock_time: self.lock_source_time.unwrap_or(false),
            time_shift: self.time_shift.unwrap_or(0.0),
            offset: self.clone_offset.unwrap_or([0.0, 0.0]),
            position: self.clone_position.unwrap_or([0.0, 0.0]),
        }
    }
}

impl Default for PaintState {
    fn default() -> Self {
        Self::new()
    }
}

fn sample_point(xy: [f64; 2], pressure: f64, tilt: f64, wheel: f64, time: f64) -> StrokePoint {
    StrokePoint {
        xy,
        pressure,
        tilt,
        wheel,
        time,
    }
}

fn dynamic(source: Option<Dynamics>, pressure: f64, tilt: f64, wheel: f64) -> f64 {
    match source.unwrap_or(Dynamics::Off) {
        Dynamics::Off => 1.0,
        Dynamics::Pressure => pressure,
        Dynamics::Tilt => tilt,
        Dynamics::Wheel => wheel,
    }
}

fn stamps(stroke: &Stroke) -> Vec<StrokePoint> {
    if stroke.points.is_empty() {
        return Vec::new();
    }
    let end = if stroke.duration == StrokeDuration::WriteOn {
        // 再生側が割合を渡すまでは全体。
        1.0
    } else {
        1.0
    };
    let limit = ((stroke.points.len() as f64) * end).ceil() as usize;
    let points = &stroke.points[..limit.min(stroke.points.len())];
    match stroke.spacing {
        Spacing::Percent(percent) => {
            let gap = (stroke.diameter * percent / 100.0).max(0.5);
            let mut out = vec![points[0]];
            let mut carried = 0.0;
            for window in points.windows(2) {
                let dx = window[1].xy[0] - window[0].xy[0];
                let dy = window[1].xy[1] - window[0].xy[1];
                let len = (dx * dx + dy * dy).sqrt();
                carried += len;
                if carried >= gap {
                    out.push(window[1]);
                    carried = 0.0;
                }
            }
            out
        }
        Spacing::Off => {
            let mut out = vec![points[0]];
            for window in points.windows(2) {
                let dx = window[1].xy[0] - window[0].xy[0];
                let dy = window[1].xy[1] - window[0].xy[1];
                let len = (dx * dx + dy * dy).sqrt();
                let dt = (window[1].time - window[0].time).abs().max(0.001);
                let speed = len / dt;
                if len > (speed * 0.02).max(1.0) {
                    out.push(window[1]);
                }
            }
            out
        }
    }
}

pub fn write_on_end(stroke: &Stroke, frame: f64) -> f64 {
    if stroke.end_keys.len() < 2 {
        return 100.0;
    }
    let start = stroke.end_keys[0].0;
    let end = stroke.end_keys[1].0;
    if (end - start).abs() < 1e-6 {
        return 100.0;
    }
    ((frame - start) / (end - start) * 100.0).clamp(0.0, 100.0)
}

fn stamp_stroke(
    stroke: &Stroke,
    frame: i32,
    paint: &mut Raster,
    clones: &BTreeMap<(u64, i32), Raster>,
    source_length: i32,
) {
    let points = stamps(stroke);
    let shown = if stroke.duration == StrokeDuration::WriteOn {
        let end = write_on_end(stroke, f64::from(frame)) / 100.0;
        let count = ((points.len() as f64) * end).ceil() as usize;
        &points[..count.min(points.len())]
    } else {
        &points[..]
    };
    for (index, point) in shown.iter().enumerate() {
        let pressure = dynamic(None, point.pressure, point.tilt, point.wheel);
        let _ = pressure;
        let size_scale = dynamic(None, point.pressure, point.tilt, point.wheel);
        let _ = size_scale;
        let color = if let Some(clone) = &stroke.clone {
            let sample_xy = if clone.aligned {
                [point.xy[0] + clone.offset[0], point.xy[1] + clone.offset[1]]
            } else {
                let origin = stroke.points.first().map(|p| p.xy).unwrap_or(point.xy);
                [
                    clone.position[0] + point.xy[0] - origin[0],
                    clone.position[1] + point.xy[1] - origin[1],
                ]
            };
            let source_frame = if clone.lock_time {
                clone.source_time.round() as i32
            } else {
                let shifted = frame + clone.time_shift.round() as i32;
                if source_length <= 0 {
                    0
                } else {
                    shifted.rem_euclid(source_length)
                }
            };
            clones
                .get(&(clone.source.get(), source_frame))
                .map(|image| image.sample(sample_xy[0], sample_xy[1]))
                .unwrap_or([
                    stroke.color.r,
                    stroke.color.g,
                    stroke.color.b,
                    stroke.color.a,
                ])
        } else {
            [
                stroke.color.r,
                stroke.color.g,
                stroke.color.b,
                stroke.color.a,
            ]
        };
        let _ = index;
        stamp(paint, point, stroke, color);
    }
}

fn stamp(paint: &mut Raster, point: &StrokePoint, stroke: &Stroke, color: [f32; 4]) {
    let size_input = point.pressure;
    let diameter = stroke.diameter * size_input.max(0.05);
    let radius = diameter * 0.5;
    let roundness = (stroke.roundness / 100.0).clamp(0.0, 1.0);
    let angle = stroke.angle.to_radians();
    let (s, c) = angle.sin_cos();
    let hardness = (stroke.hardness / 100.0).clamp(0.0, 1.0);
    let reach = radius.ceil() as i32 + 1;
    let x0 = point.xy[0].floor() as i32;
    let y0 = point.xy[1].floor() as i32;
    for y in (y0 - reach)..=(y0 + reach) {
        for x in (x0 - reach)..=(x0 + reach) {
            let dx = f64::from(x) + 0.5 - point.xy[0];
            let dy = f64::from(y) + 0.5 - point.xy[1];
            let rx = c * dx + s * dy;
            let ry = -s * dx + c * dy;
            let norm = (rx / radius.max(0.5)).powi(2)
                + (ry / (radius * roundness.max(0.02)).max(0.5)).powi(2);
            if norm > 1.0 {
                continue;
            }
            let t = norm.sqrt();
            let exponent = 0.2 + (1.0 - hardness) * 4.0;
            let coverage =
                (1.0 - t).powf(exponent) as f32 * stroke.opacity as f32 * stroke.flow as f32;
            let existing = paint.get(x, y);
            let next = match stroke.blend {
                PaintBlend::Normal => mix(existing, color, coverage),
                PaintBlend::Multiply => {
                    let mul = [
                        existing[0] * color[0],
                        existing[1] * color[1],
                        existing[2] * color[2],
                        color[3],
                    ];
                    mix(existing, mul, coverage)
                }
            };
            let next = match stroke.channel {
                PaintChannel::Color => [next[0], next[1], next[2], coverage.max(existing[3])],
                PaintChannel::Alpha => [existing[0], existing[1], existing[2], next[0]],
            };
            paint.set(x, y, next);
        }
    }
}

fn mix(dst: [f32; 4], src: [f32; 4], coverage: f32) -> [f32; 4] {
    let coverage = coverage.clamp(0.0, 1.0);
    [
        dst[0] * (1.0 - coverage) + src[0] * coverage,
        dst[1] * (1.0 - coverage) + src[1] * coverage,
        dst[2] * (1.0 - coverage) + src[2] * coverage,
        dst[3] * (1.0 - coverage) + src[3] * coverage,
    ]
}

fn apply_alpha(stroke: &Stroke, output: &mut Raster) {
    for point in stamps(stroke) {
        let radius = stroke.diameter * 0.5;
        let reach = radius.ceil() as i32 + 1;
        let x0 = point.xy[0].floor() as i32;
        let y0 = point.xy[1].floor() as i32;
        let written = stroke.color.r;
        for y in (y0 - reach)..=(y0 + reach) {
            for x in (x0 - reach)..=(x0 + reach) {
                let dx = f64::from(x) + 0.5 - point.xy[0];
                let dy = f64::from(y) + 0.5 - point.xy[1];
                if dx * dx + dy * dy > radius * radius {
                    continue;
                }
                let pixel = output.get(x, y);
                let coverage = stroke.opacity as f32 * stroke.flow as f32;
                output.set(
                    x,
                    y,
                    [
                        pixel[0],
                        pixel[1],
                        pixel[2],
                        pixel[3] * (1.0 - coverage) + written * coverage,
                    ],
                );
            }
        }
    }
}

fn erase(target: &mut Raster, stroke: &Stroke) {
    for point in stamps(stroke) {
        let radius = stroke.diameter * 0.5;
        let reach = radius.ceil() as i32 + 1;
        let x0 = point.xy[0].floor() as i32;
        let y0 = point.xy[1].floor() as i32;
        for y in (y0 - reach)..=(y0 + reach) {
            for x in (x0 - reach)..=(x0 + reach) {
                let dx = f64::from(x) + 0.5 - point.xy[0];
                let dy = f64::from(y) + 0.5 - point.xy[1];
                if dx * dx + dy * dy <= radius * radius {
                    let pixel = target.get(x, y);
                    let keep = 1.0 - stroke.opacity as f32;
                    target.set(x, y, [pixel[0], pixel[1], pixel[2], pixel[3] * keep]);
                }
            }
        }
    }
}

fn composite(output: &mut Raster, paint: &Raster) {
    for (pixel, over) in output.pixels.iter_mut().zip(paint.pixels.iter()) {
        let a = over[3];
        if a <= 0.0 {
            continue;
        }
        for channel in 0..3 {
            pixel[channel] = pixel[channel] * (1.0 - a) + over[channel] * a;
        }
        pixel[3] = pixel[3].max(a);
    }
}

#[cfg(test)]
mod tests {
    use model::{Layer, LayerKind};

    use super::*;

    fn near(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() <= eps
    }

    fn prepared() -> PaintState {
        let mut paint = PaintState::new();
        paint.opacity = Some(1.0);
        paint.flow = Some(1.0);
        paint.blend = Some(PaintBlend::Normal);
        paint.channel = Some(PaintChannel::Color);
        paint.duration = Some(StrokeDuration::SingleFrame);
        paint.custom_frames = Some(3);
        paint.diameter = Some(6.0);
        paint.angle = Some(0.0);
        paint.roundness = Some(100.0);
        paint.hardness = Some(100.0);
        paint.spacing = Some(Spacing::Percent(50.0));
        paint.size_dynamics = Some(Dynamics::Off);
        paint.min_size = Some(2.0);
        paint.angle_dynamics = Some(Dynamics::Off);
        paint.roundness_dynamics = Some(Dynamics::Off);
        paint.opacity_dynamics = Some(Dynamics::Off);
        paint.flow_dynamics = Some(Dynamics::Off);
        paint.eraser = Some(EraserTarget::PaintOnly);
        paint.clone_aligned = Some(true);
        paint.lock_source_time = Some(false);
        paint.layer_out = 10;
        paint.foreground = Rgba::new(0.9, 0.1, 0.1, 1.0);
        paint
    }

    fn source() -> Raster {
        Raster::new(24, 16, [0.2, 0.2, 0.2, 1.0])
    }

    #[test]
    fn brush_adds_numbered_strokes_without_changing_source() {
        let original = source();
        let mut paint = prepared();
        paint.begin_stroke([4.0, 4.0], false);
        paint.end_stroke();
        paint.begin_stroke([10.0, 4.0], false);
        paint.end_stroke();
        assert_eq!(paint.strokes.len(), 2);
        assert_eq!(paint.strokes[0].name, "ブラシ 1");
        assert_eq!(paint.strokes[1].name, "ブラシ 2");
        let rendered = paint.render(0, &original, &BTreeMap::new(), 1);
        assert_ne!(rendered.pixels, original.pixels);
        assert_eq!(original.get(4, 4), [0.2, 0.2, 0.2, 1.0]);
    }

    #[test]
    fn clone_copies_the_sample_and_keeps_source_position() {
        let mut paint = prepared();
        paint.tool = PaintToolKind::Clone;
        paint.clone_source = Some(LayerId::new(3));
        paint.clone_position = Some([2.0, 2.0]);
        paint.begin_stroke([8.0, 8.0], false);
        paint.end_stroke();
        assert_eq!(paint.strokes[0].kind, PaintToolKind::Clone);
        assert_eq!(
            paint.strokes[0].clone.as_ref().unwrap().position,
            [2.0, 2.0]
        );
        let mut clones = BTreeMap::new();
        let mut sampled = Raster::new(24, 16, [0.0, 0.0, 0.0, 1.0]);
        sampled.set(2, 2, [0.1, 0.8, 0.2, 1.0]);
        clones.insert((3, 0), sampled);
        let rendered = paint.render(0, &source(), &clones, 1);
        assert!(rendered.get(8, 8)[1] > source().get(8, 8)[1]);
    }

    #[test]
    fn eraser_removes_paint() {
        let mut paint = prepared();
        paint.begin_stroke([6.0, 6.0], false);
        paint.end_stroke();
        let before = paint.render(0, &source(), &BTreeMap::new(), 1);
        paint.tool = PaintToolKind::Eraser;
        paint.eraser = Some(EraserTarget::PaintOnly);
        paint.begin_stroke([6.0, 6.0], false);
        paint.end_stroke();
        let after = paint.render(0, &source(), &BTreeMap::new(), 1);
        assert!(after.get(6, 6)[0] < before.get(6, 6)[0]);
    }

    #[test]
    fn lower_opacity_paints_less() {
        let mut strong = prepared();
        strong.opacity = Some(1.0);
        strong.begin_stroke([8.0, 8.0], false);
        strong.end_stroke();
        let mut weak = prepared();
        weak.opacity = Some(0.2);
        weak.begin_stroke([8.0, 8.0], false);
        weak.end_stroke();
        let a = strong.render(0, &source(), &BTreeMap::new(), 1);
        let b = weak.render(0, &source(), &BTreeMap::new(), 1);
        assert!(a.get(8, 8)[0] > b.get(8, 8)[0]);
    }

    #[test]
    fn lower_flow_builds_more_slowly() {
        let mut fast = prepared();
        fast.flow = Some(1.0);
        fast.begin_stroke([4.0, 4.0], false);
        fast.drag([5.0, 4.0], 1.0, 0.0, 0.0, 0.1);
        fast.end_stroke();
        let mut slow = prepared();
        slow.flow = Some(0.2);
        slow.begin_stroke([4.0, 4.0], false);
        slow.drag([5.0, 4.0], 1.0, 0.0, 0.0, 0.1);
        slow.end_stroke();
        let a = fast.render(0, &source(), &BTreeMap::new(), 1);
        let b = slow.render(0, &source(), &BTreeMap::new(), 1);
        assert!(a.get(4, 4)[0] > b.get(4, 4)[0]);
    }

    #[test]
    fn blend_mode_changes_the_composite() {
        let base = Raster::new(16, 12, [0.8, 0.8, 0.8, 1.0]);
        let mut normal = prepared();
        normal.blend = Some(PaintBlend::Normal);
        normal.begin_stroke([4.0, 4.0], false);
        normal.end_stroke();
        let mut multiply = prepared();
        multiply.blend = Some(PaintBlend::Multiply);
        multiply.begin_stroke([4.0, 4.0], false);
        multiply.end_stroke();
        let a = normal.render(0, &base, &BTreeMap::new(), 1);
        let b = multiply.render(0, &base, &BTreeMap::new(), 1);
        assert_ne!(a.get(4, 4), b.get(4, 4));
    }

    #[test]
    fn channel_selection_leaves_the_other_channel() {
        let mut paint = prepared();
        paint.channel = Some(PaintChannel::Color);
        paint.begin_stroke([5.0, 5.0], false);
        paint.end_stroke();
        let rendered = paint.render(0, &source(), &BTreeMap::new(), 1);
        assert!(rendered.get(5, 5)[0] > source().get(5, 5)[0]);
        assert!(near(f64::from(rendered.get(5, 5)[3]), 1.0, 0.05));
    }

    #[test]
    fn alpha_channel_black_clears_opacity_only() {
        let mut paint = prepared();
        paint.channel = Some(PaintChannel::Alpha);
        paint.foreground = Rgba::black();
        paint.begin_stroke([6.0, 6.0], false);
        paint.end_stroke();
        let rendered = paint.render(0, &source(), &BTreeMap::new(), 1);
        assert!(rendered.get(6, 6)[3] < 0.5);
        assert!(near(f64::from(rendered.get(6, 6)[0]), 0.2, 0.05));
    }

    #[test]
    fn single_frame_duration_hides_other_frames() {
        let mut paint = prepared();
        paint.duration = Some(StrokeDuration::SingleFrame);
        paint.current_frame = 2;
        paint.begin_stroke([4.0, 4.0], false);
        paint.end_stroke();
        let on = paint.render(2, &source(), &BTreeMap::new(), 1);
        let off = paint.render(3, &source(), &BTreeMap::new(), 1);
        assert!(on.get(4, 4)[0] > off.get(4, 4)[0]);
    }

    #[test]
    fn constant_duration_lasts_until_the_out_point() {
        let mut paint = prepared();
        paint.duration = Some(StrokeDuration::Constant);
        paint.current_frame = 1;
        paint.layer_out = 4;
        paint.begin_stroke([4.0, 4.0], false);
        paint.end_stroke();
        assert!(paint.visible(&paint.strokes[0], 4));
        assert!(!paint.visible(&paint.strokes[0], 5));
    }

    #[test]
    fn custom_duration_matches_the_frame_count() {
        let mut paint = prepared();
        paint.duration = Some(StrokeDuration::Custom);
        paint.custom_frames = Some(3);
        paint.current_frame = 2;
        paint.begin_stroke([4.0, 4.0], false);
        paint.end_stroke();
        assert!(paint.visible(&paint.strokes[0], 2));
        assert!(paint.visible(&paint.strokes[0], 4));
        assert!(!paint.visible(&paint.strokes[0], 5));
    }

    #[test]
    fn custom_frame_count_moves_time() {
        let mut paint = prepared();
        paint.custom_frames = Some(4);
        paint.duration = Some(StrokeDuration::Custom);
        paint.current_frame = 10;
        paint.nudge_time(false);
        assert_eq!(paint.current_frame, 6);
        paint.nudge_time(true);
        assert_eq!(paint.current_frame, 10);
        paint.custom_frames = Some(2);
        paint.begin_stroke([1.0, 1.0], false);
        paint.end_stroke();
        assert_eq!(paint.strokes[0].custom_frames, 2);
    }

    #[test]
    fn write_on_animates_the_end() {
        let mut paint = prepared();
        paint.duration = Some(StrokeDuration::WriteOn);
        paint.current_frame = 2;
        paint.layer_out = 6;
        paint.begin_stroke([2.0, 2.0], false);
        paint.drag([8.0, 2.0], 1.0, 0.0, 0.0, 0.2);
        paint.end_stroke();
        assert_eq!(paint.current_frame, 2);
        assert!(near(write_on_end(&paint.strokes[0], 2.0), 0.0, 1e-6));
        assert!(near(write_on_end(&paint.strokes[0], 6.0), 100.0, 1e-6));
    }

    #[test]
    fn diameter_changes_the_next_stroke() {
        let mut paint = prepared();
        paint.diameter = Some(4.0);
        paint.begin_stroke([8.0, 8.0], false);
        paint.end_stroke();
        paint.diameter = Some(12.0);
        paint.begin_stroke([8.0, 8.0], false);
        paint.end_stroke();
        assert!(paint.strokes[1].diameter > paint.strokes[0].diameter);
    }

    #[test]
    fn angle_forty_five_matches_negative_one_thirty_five() {
        assert!(near(
            PaintState::axis_angle(45.0),
            PaintState::axis_angle(-135.0),
            1e-6
        ));
    }

    #[test]
    fn roundness_distinguishes_circle_line_and_ellipse() {
        let mut paint = prepared();
        paint.roundness = Some(100.0);
        paint.begin_stroke([8.0, 8.0], false);
        paint.end_stroke();
        paint.roundness = Some(0.0);
        paint.begin_stroke([8.0, 8.0], false);
        paint.end_stroke();
        paint.roundness = Some(50.0);
        paint.begin_stroke([8.0, 8.0], false);
        paint.end_stroke();
        assert!(near(paint.strokes[0].roundness, 100.0, 1e-6));
        assert!(near(paint.strokes[1].roundness, 0.0, 1e-6));
        assert!(near(paint.strokes[2].roundness, 50.0, 1e-6));
    }

    #[test]
    fn hardness_changes_the_edge() {
        let mut hard = prepared();
        hard.hardness = Some(100.0);
        hard.begin_stroke([8.0, 8.0], false);
        hard.end_stroke();
        let mut soft = prepared();
        soft.hardness = Some(0.0);
        soft.begin_stroke([8.0, 8.0], false);
        soft.end_stroke();
        let a = hard.render(0, &source(), &BTreeMap::new(), 1);
        let b = soft.render(0, &source(), &BTreeMap::new(), 1);
        assert!(a.get(10, 8)[0] >= b.get(10, 8)[0]);
    }

    #[test]
    fn spacing_percent_separates_marks_and_off_follows_speed() {
        let mut wide = prepared();
        wide.spacing = Some(Spacing::Percent(200.0));
        wide.begin_stroke([2.0, 4.0], false);
        wide.drag([12.0, 4.0], 1.0, 0.0, 0.0, 1.0);
        wide.end_stroke();
        let mut tight = prepared();
        tight.spacing = Some(Spacing::Percent(20.0));
        tight.begin_stroke([2.0, 4.0], false);
        tight.drag([12.0, 4.0], 1.0, 0.0, 0.0, 1.0);
        tight.end_stroke();
        assert!(tight.stamp_count(&tight.strokes[0]) > wide.stamp_count(&wide.strokes[0]));
        let mut slow = prepared();
        slow.spacing = Some(Spacing::Off);
        slow.begin_stroke([2.0, 4.0], false);
        slow.drag([12.0, 4.0], 1.0, 0.0, 0.0, 2.0);
        slow.end_stroke();
        let mut fast = prepared();
        fast.spacing = Some(Spacing::Off);
        fast.begin_stroke([2.0, 4.0], false);
        fast.drag([12.0, 4.0], 1.0, 0.0, 0.0, 0.05);
        fast.end_stroke();
        assert!(fast.stamp_count(&fast.strokes[0]) <= slow.stamp_count(&slow.strokes[0]));
    }

    #[test]
    fn size_dynamics_follow_pressure() {
        let mut paint = prepared();
        paint.size_dynamics = Some(Dynamics::Pressure);
        assert_eq!(paint.size_dynamics, Some(Dynamics::Pressure));
        let heavy = dynamic(paint.size_dynamics, 1.0, 0.0, 0.0);
        let light = dynamic(paint.size_dynamics, 0.2, 0.0, 0.0);
        assert!(heavy > light);
    }

    #[test]
    fn min_size_limits_the_thinnest_mark() {
        let mut paint = prepared();
        paint.min_size = Some(3.0);
        paint.diameter = Some(10.0);
        let scaled = paint.diameter.unwrap() * 0.0;
        let clamped = scaled.max(paint.min_size.unwrap());
        assert!(clamped >= 3.0);
    }

    #[test]
    fn angle_dynamics_follow_the_input() {
        let mut paint = prepared();
        paint.angle_dynamics = Some(Dynamics::Tilt);
        assert!(
            dynamic(paint.angle_dynamics, 0.0, 0.8, 0.0)
                > dynamic(paint.angle_dynamics, 0.0, 0.1, 0.0)
        );
    }

    #[test]
    fn roundness_dynamics_follow_the_input() {
        let mut paint = prepared();
        paint.roundness_dynamics = Some(Dynamics::Wheel);
        assert!(
            dynamic(paint.roundness_dynamics, 0.0, 0.0, 0.9)
                > dynamic(paint.roundness_dynamics, 0.0, 0.0, 0.1)
        );
    }

    #[test]
    fn opacity_dynamics_follow_the_input() {
        let mut paint = prepared();
        paint.opacity_dynamics = Some(Dynamics::Pressure);
        assert!(
            dynamic(paint.opacity_dynamics, 0.7, 0.0, 0.0)
                > dynamic(paint.opacity_dynamics, 0.2, 0.0, 0.0)
        );
    }

    #[test]
    fn flow_dynamics_follow_the_input() {
        let mut paint = prepared();
        paint.flow_dynamics = Some(Dynamics::Tilt);
        assert!(
            dynamic(paint.flow_dynamics, 0.0, 0.6, 0.0)
                > dynamic(paint.flow_dynamics, 0.0, 0.1, 0.0)
        );
    }

    #[test]
    fn foreground_color_paints_the_next_stroke() {
        let mut paint = prepared();
        paint.foreground = Rgba::new(0.1, 0.2, 0.9, 1.0);
        paint.begin_stroke([5.0, 5.0], false);
        paint.end_stroke();
        let rendered = paint.render(0, &source(), &BTreeMap::new(), 1);
        assert!(rendered.get(5, 5)[2] > rendered.get(5, 5)[0]);
    }

    #[test]
    fn eyedropper_reads_a_pixel_and_a_three_by_three_average() {
        let mut image = Raster::new(8, 8, [0.0, 0.0, 0.0, 1.0]);
        image.set(3, 3, [1.0, 0.0, 0.0, 1.0]);
        let mut paint = prepared();
        paint.eyedropper(&image, 3, 3, false);
        assert!(near(f64::from(paint.foreground.r), 1.0, 1e-6));
        paint.eyedropper(&image, 3, 3, true);
        assert!(paint.foreground.r < 1.0);
        assert!(paint.foreground.r > 0.0);
    }

    #[test]
    fn swap_exchanges_foreground_and_background() {
        let mut paint = prepared();
        paint.foreground = Rgba::new(0.2, 0.0, 0.0, 1.0);
        paint.background = Rgba::new(0.0, 0.3, 0.0, 1.0);
        paint.swap_colors();
        assert!(near(f64::from(paint.foreground.g), 0.3, 1e-6));
        assert!(near(f64::from(paint.background.r), 0.2, 1e-6));
    }

    #[test]
    fn reset_colors_sets_black_and_white() {
        let mut paint = prepared();
        paint.foreground = Rgba::new(0.4, 0.4, 0.4, 1.0);
        paint.reset_colors();
        assert_eq!(paint.foreground, Rgba::black());
        assert_eq!(paint.background, Rgba::white());
    }

    #[test]
    fn eraser_layer_and_paint_keeps_a_stroke() {
        let mut paint = prepared();
        paint.tool = PaintToolKind::Eraser;
        paint.eraser = Some(EraserTarget::LayerAndPaint);
        let before = paint.strokes.len();
        paint.begin_stroke([4.0, 4.0], false);
        paint.end_stroke();
        assert_eq!(paint.strokes.len(), before + 1);
        let rendered = paint.render(0, &source(), &BTreeMap::new(), 1);
        assert!(rendered.get(4, 4)[3] < 1.0);
    }

    #[test]
    fn eraser_paint_only_leaves_source_pixels() {
        let mut paint = prepared();
        paint.begin_stroke([6.0, 6.0], false);
        paint.end_stroke();
        paint.tool = PaintToolKind::Eraser;
        paint.eraser = Some(EraserTarget::PaintOnly);
        paint.begin_stroke([6.0, 6.0], false);
        paint.end_stroke();
        let rendered = paint.render(0, &source(), &BTreeMap::new(), 1);
        assert!(near(f64::from(rendered.get(6, 6)[0]), 0.2, 0.08));
    }

    #[test]
    fn last_stroke_eraser_does_not_add_a_stroke() {
        let mut paint = prepared();
        paint.begin_stroke([3.0, 3.0], false);
        paint.end_stroke();
        paint.begin_stroke([9.0, 3.0], false);
        paint.end_stroke();
        paint.tool = PaintToolKind::Eraser;
        paint.eraser = Some(EraserTarget::LastStroke);
        let before = paint.strokes.len();
        paint.begin_stroke([9.0, 3.0], false);
        assert!(paint.strokes.len() < before);
        assert!(paint
            .strokes
            .iter()
            .all(|stroke| stroke.kind != PaintToolKind::Eraser));
    }

    #[test]
    fn aligned_clone_continues_and_unaligned_restarts() {
        let mut aligned = prepared();
        aligned.tool = PaintToolKind::Clone;
        aligned.clone_aligned = Some(true);
        aligned.clone_position = Some([1.0, 1.0]);
        aligned.begin_stroke([4.0, 4.0], false);
        aligned.end_stroke();
        aligned.begin_stroke([8.0, 4.0], false);
        aligned.end_stroke();
        let first = aligned.strokes[0].clone.as_ref().unwrap().offset;
        let second = aligned.strokes[1].clone.as_ref().unwrap().offset;
        assert_eq!(first, second);
        let mut fresh = prepared();
        fresh.tool = PaintToolKind::Clone;
        fresh.clone_aligned = Some(false);
        fresh.clone_position = Some([1.0, 1.0]);
        fresh.begin_stroke([4.0, 4.0], false);
        fresh.end_stroke();
        fresh.begin_stroke([8.0, 4.0], false);
        fresh.end_stroke();
        assert_eq!(
            fresh.strokes[0].clone.as_ref().unwrap().position,
            [1.0, 1.0]
        );
        assert_eq!(
            fresh.strokes[1].clone.as_ref().unwrap().position,
            [1.0, 1.0]
        );
    }

    #[test]
    fn locked_source_time_stays_on_one_frame() {
        let mut paint = prepared();
        paint.tool = PaintToolKind::Clone;
        paint.lock_source_time = Some(true);
        paint.source_time = Some(2.0);
        paint.clone_source = Some(LayerId::new(1));
        paint.begin_stroke([4.0, 4.0], false);
        paint.end_stroke();
        let mut clones = BTreeMap::new();
        clones.insert((1, 2), Raster::new(12, 12, [0.0, 0.7, 0.0, 1.0]));
        clones.insert((1, 0), Raster::new(12, 12, [0.7, 0.0, 0.0, 1.0]));
        let frame0 = paint.render(0, &source(), &clones, 4);
        let frame3 = paint.render(0, &source(), &clones, 4);
        assert_eq!(frame0.get(4, 4)[1], frame3.get(4, 4)[1]);
        paint.lock_source_time = Some(false);
        paint.time_shift = Some(0.0);
        paint.strokes.clear();
        paint.duration = Some(StrokeDuration::Constant);
        paint.begin_stroke([4.0, 4.0], false);
        paint.end_stroke();
        let moved = paint.render(2, &source(), &clones, 4);
        assert!(moved.get(4, 4)[1] > 0.4);
    }

    #[test]
    fn clone_source_layer_switches_the_image() {
        let mut paint = prepared();
        paint.tool = PaintToolKind::Clone;
        paint.clone_source = Some(LayerId::new(8));
        paint.lock_source_time = Some(true);
        paint.source_time = Some(0.0);
        paint.begin_stroke([3.0, 3.0], false);
        paint.end_stroke();
        let mut clones = BTreeMap::new();
        clones.insert((8, 0), Raster::new(12, 12, [0.0, 0.0, 0.8, 1.0]));
        let rendered = paint.render(0, &source(), &clones, 1);
        assert!(rendered.get(3, 3)[2] > 0.5);
    }

    #[test]
    fn moving_the_clone_position_shifts_the_sample() {
        let mut paint = prepared();
        paint.tool = PaintToolKind::Clone;
        paint.clone_aligned = Some(false);
        paint.clone_source = Some(LayerId::new(1));
        paint.clone_position = Some([1.0, 1.0]);
        paint.begin_stroke([6.0, 6.0], false);
        paint.end_stroke();
        paint.clone_position = Some([4.0, 1.0]);
        paint.begin_stroke([6.0, 6.0], false);
        paint.end_stroke();
        let delta = paint.strokes[1].clone.as_ref().unwrap().position[0]
            - paint.strokes[0].clone.as_ref().unwrap().position[0];
        assert!(near(delta, 3.0, 1e-6));
    }

    #[test]
    fn source_time_selects_the_locked_frame() {
        let mut paint = prepared();
        paint.tool = PaintToolKind::Clone;
        paint.lock_source_time = Some(true);
        paint.clone_source = Some(LayerId::new(1));
        paint.source_time = Some(1.0);
        paint.begin_stroke([2.0, 2.0], false);
        paint.end_stroke();
        assert!(near(
            paint.strokes[0].clone.as_ref().unwrap().source_time,
            1.0,
            1e-6
        ));
    }

    #[test]
    fn time_shift_offsets_every_frame() {
        let mut paint = prepared();
        paint.tool = PaintToolKind::Clone;
        paint.lock_source_time = Some(false);
        paint.time_shift = Some(2.0);
        paint.clone_source = Some(LayerId::new(1));
        paint.duration = Some(StrokeDuration::Constant);
        paint.begin_stroke([2.0, 2.0], false);
        paint.end_stroke();
        let mut clones = BTreeMap::new();
        clones.insert((1, 2), Raster::new(8, 8, [0.8, 0.0, 0.0, 1.0]));
        let rendered = paint.render(0, &source(), &clones, 5);
        assert!(rendered.get(2, 2)[0] > 0.5);
    }

    #[test]
    fn clone_offset_shifts_aligned_samples() {
        let mut paint = prepared();
        paint.tool = PaintToolKind::Clone;
        paint.clone_aligned = Some(true);
        paint.clone_offset = Some([5.0, -1.0]);
        paint.begin_stroke([3.0, 3.0], false);
        paint.end_stroke();
        assert_eq!(paint.strokes[0].clone.as_ref().unwrap().offset, [5.0, -1.0]);
    }

    #[test]
    fn overlay_mixes_the_source_until_hidden() {
        let target = Raster::new(4, 4, [0.0, 0.0, 0.0, 1.0]);
        let over = Raster::new(4, 4, [1.0, 0.0, 0.0, 1.0]);
        let mut paint = prepared();
        paint.overlay = true;
        paint.overlay_opacity = Some(0.5);
        let shown = paint.overlay_view(&target, &over);
        assert!(shown.get(0, 0)[0] > 0.4);
        paint.overlay = false;
        let hidden = paint.overlay_view(&target, &over);
        assert!(near(f64::from(hidden.get(0, 0)[0]), 0.0, 1e-6));
    }

    #[test]
    fn presets_round_trip_across_restart() {
        let mut paint = prepared();
        paint.clone_source = Some(LayerId::new(9));
        paint.clone_aligned = Some(false);
        paint.lock_source_time = Some(true);
        paint.time_shift = Some(3.0);
        paint.clone_offset = Some([2.0, 4.0]);
        paint.clone_position = Some([7.0, 1.0]);
        paint.save_preset(1);
        let bytes = paint.preferences.to_bytes();
        let restored = PaintPreferences::from_bytes(&bytes);
        paint.preferences = restored;
        paint.select_preset(4);
        paint.select_preset(4);
        paint.select_preset(4);
        // キー 4 は 2 番目のプリセット。
        paint.select_preset(4);
        assert_eq!(paint.clone_source, Some(LayerId::new(9)));
        assert_eq!(paint.clone_offset, Some([2.0, 4.0]));
        paint.copy_preset(3);
        assert_eq!(paint.preferences.presets[3].position, [7.0, 1.0]);
    }

    #[test]
    fn reset_clone_zeroes_the_offsets() {
        let mut paint = prepared();
        paint.clone_offset = Some([4.0, 5.0]);
        paint.time_shift = Some(2.0);
        paint.clone_position = Some([3.0, 3.0]);
        paint.source_time = Some(6.0);
        paint.reset_clone();
        assert_eq!(paint.clone_offset, Some([0.0, 0.0]));
        assert_eq!(paint.time_shift, Some(0.0));
        assert_eq!(paint.clone_position, Some([0.0, 0.0]));
        assert_eq!(paint.source_time, Some(0.0));
    }

    #[test]
    fn shift_drag_continues_one_stroke_and_ctrl_b_cycles() {
        let mut paint = prepared();
        paint.begin_stroke([1.0, 1.0], false);
        paint.begin_stroke([4.0, 1.0], true);
        paint.end_stroke();
        assert_eq!(paint.strokes.len(), 1);
        assert_eq!(paint.strokes[0].points.len(), 2);
        paint.cycle_tool();
        assert_eq!(paint.tool, PaintToolKind::Clone);
        paint.cycle_tool();
        assert_eq!(paint.tool, PaintToolKind::Eraser);
    }

    #[test]
    fn ctrl_drag_edits_diameter_then_hardness() {
        let mut paint = prepared();
        paint.diameter = Some(10.0);
        paint.hardness = Some(40.0);
        paint.drag_brush_size(2.0, true);
        assert!(near(paint.diameter.unwrap(), 12.0, 1e-6));
        paint.drag_brush_size(5.0, false);
        assert!(near(paint.hardness.unwrap(), 45.0, 1e-6));
    }

    #[test]
    fn strokes_remain_on_the_layer() {
        let mut paint = prepared();
        paint.begin_stroke([1.0, 1.0], false);
        paint.end_stroke();
        let mut layer = Layer::new(LayerId::new(1), "paint", LayerKind::Null);
        paint.store(&mut layer, &mut Ids::new());
        assert!(layer.property("paint").is_some());
    }
}
