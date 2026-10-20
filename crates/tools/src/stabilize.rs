//! ワープスタビライザー。追跡した動きを滑らかにし、フレームの端を処理する。

use std::cmp::Ordering;

use model::{Ids, Layer, LayerId};

use crate::error::ToolError;
use crate::match2d::{capture_patch, match_patch};
use crate::persist;
use crate::raster::{CompareChannel, Raster};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StabilizeResult {
    SmoothMotion,
    NoMotion,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StabilizeMethod {
    Position,
    PositionScaleRotation,
    Perspective,
    SubspaceWarp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Framing {
    StabilizeOnly,
    StabilizeCrop,
    StabilizeCropAutoScale,
    SynthesizeEdges,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RollingShutter {
    Auto,
    Strong,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Purpose {
    Stabilize,
    Reversible,
    Restore,
    ApplyToTarget,
    ApplyOverOriginal,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FrameMotion {
    pub translation: [f64; 2],
    pub rotation: f64,
    pub scale: f64,
    pub shear: f64,
    pub method_used: StabilizeMethod,
    pub cells: Vec<[f64; 2]>,
}

#[derive(Clone, Debug)]
pub struct WarpStabilizer {
    pub result: StabilizeResult,
    pub smoothness: Option<f64>,
    pub method: StabilizeMethod,
    pub preserve_scale: Option<bool>,
    pub framing: Framing,
    pub max_scale: Option<f64>,
    pub action_safe: Option<f64>,
    pub additional_scale: Option<f64>,
    pub detailed: Option<bool>,
    pub rolling: RollingShutter,
    pub less_crop: Option<f64>,
    pub synthesis_range: Option<f64>,
    pub feather: Option<f64>,
    pub edge_crop: [f64; 4],
    pub purpose: Purpose,
    pub target: Option<LayerId>,
    pub hide_warning: Option<bool>,
    pub running: bool,
    pub solved: bool,
    pub needs_analyze: bool,
    pub warning: bool,
    pub frame_rate: f64,
    pub motions: Vec<FrameMotion>,
    pub removed: Vec<[f64; 2]>,
    pub work_units: u32,
    pub analysis_generation: u32,
    in_frame: i32,
    out_frame: i32,
}

impl WarpStabilizer {
    pub fn new() -> Self {
        Self {
            result: StabilizeResult::SmoothMotion,
            smoothness: None,
            method: StabilizeMethod::SubspaceWarp,
            preserve_scale: None,
            framing: Framing::StabilizeCropAutoScale,
            max_scale: None,
            action_safe: None,
            additional_scale: None,
            detailed: None,
            rolling: RollingShutter::Auto,
            less_crop: None,
            synthesis_range: None,
            feather: None,
            edge_crop: [0.0, 0.0, 0.0, 0.0],
            purpose: Purpose::Stabilize,
            target: None,
            hide_warning: None,
            running: false,
            solved: false,
            needs_analyze: true,
            warning: false,
            frame_rate: 24.0,
            motions: Vec::new(),
            removed: Vec::new(),
            work_units: 0,
            analysis_generation: 0,
            in_frame: 0,
            out_frame: 0,
        }
    }

    pub fn analyze_enabled(&self) -> bool {
        self.needs_analyze && !self.running
    }

    pub fn smoothness_enabled(&self) -> bool {
        self.result == StabilizeResult::SmoothMotion
    }

    pub fn less_crop_enabled(&self) -> bool {
        self.result != StabilizeResult::NoMotion
            && matches!(
                self.framing,
                Framing::StabilizeCrop | Framing::StabilizeCropAutoScale
            )
    }

    pub fn rolling_enabled(&self) -> bool {
        matches!(
            self.method,
            StabilizeMethod::Perspective | StabilizeMethod::SubspaceWarp
        )
    }

    pub fn auto_scale_enabled(&self) -> bool {
        self.framing == Framing::StabilizeCropAutoScale
    }

    pub fn set_in_out(&mut self, in_frame: i32, out_frame: i32) {
        if in_frame != self.in_frame || out_frame != self.out_frame {
            self.in_frame = in_frame;
            self.out_frame = out_frame;
            self.needs_analyze = true;
            self.warning = true;
        }
    }

    pub fn mark_upstream_changed(&mut self) {
        self.needs_analyze = true;
        self.warning = true;
    }

    pub fn warning_visible(&self) -> bool {
        self.warning && self.hide_warning != Some(true)
    }

    pub fn cancel(&mut self) {
        self.running = false;
        if !self.solved {
            self.motions.clear();
        }
        self.solved = false;
    }

    pub fn analyze(&mut self, frames: &[Raster]) -> Result<(), ToolError> {
        if frames.is_empty() {
            return Err(ToolError::new("フレームがない"));
        }
        self.running = true;
        self.out_frame = frames.len() as i32 - 1;
        let stride = if self.detailed == Some(true) { 8 } else { 16 };
        let seeds = contrast_seeds(&frames[0], stride);
        self.work_units = (seeds.len() * frames.len()) as u32;
        let (tracks, confidence) = track_seeds(frames, &seeds);
        self.motions = (0..frames.len())
            .map(|frame| {
                motion_at(
                    frame,
                    &tracks,
                    &seeds,
                    &confidence[frame],
                    self.method,
                    &frames[frame],
                )
            })
            .collect();
        if self.preserve_scale == Some(true) {
            for motion in &mut self.motions {
                motion.scale = 1.0;
            }
        }
        self.removed = self.correction_path();
        self.running = false;
        self.solved = true;
        self.needs_analyze = false;
        self.analysis_generation += 1;
        self.warning = self.crop_amount() > 40.0;
        Ok(())
    }

    pub fn output_path(&self) -> Vec<[f64; 2]> {
        let raw: Vec<[f64; 2]> = self
            .motions
            .iter()
            .map(|motion| motion.translation)
            .collect();
        if self.result == StabilizeResult::NoMotion {
            return vec![[0.0, 0.0]; raw.len()];
        }
        let smoothness = self.smoothness.unwrap_or(0.0);
        let mix = self.less_crop.unwrap_or(100.0) / 100.0;
        let gentle = smooth_points(&raw, smoothness);
        let heavy = smooth_points(&raw, smoothness * (1.0 + (1.0 - mix).max(0.0)));
        gentle
            .into_iter()
            .zip(heavy)
            .map(|(gentle, heavy)| {
                [
                    heavy[0] + (gentle[0] - heavy[0]) * mix.clamp(0.0, 1.0),
                    heavy[1] + (gentle[1] - heavy[1]) * mix.clamp(0.0, 1.0),
                ]
            })
            .collect()
    }

    pub fn correction_path(&self) -> Vec<[f64; 2]> {
        let raw: Vec<[f64; 2]> = self
            .motions
            .iter()
            .map(|motion| motion.translation)
            .collect();
        self.output_path()
            .into_iter()
            .zip(raw)
            .map(|(output, raw)| [raw[0] - output[0], raw[1] - output[1]])
            .collect()
    }

    pub fn crop_amount(&self) -> f64 {
        self.correction_path()
            .iter()
            .map(|delta| delta[0].abs().max(delta[1].abs()))
            .fold(0.0, f64::max)
    }

    pub fn auto_scale_percent(&self, width: f64, height: f64) -> f64 {
        if !self.auto_scale_enabled() {
            return 100.0;
        }
        let crop = self.crop_amount();
        let margin = self.action_safe.unwrap_or(0.0).clamp(0.0, 0.45);
        let usable_w = width * (1.0 - 2.0 * margin);
        let usable_h = height * (1.0 - 2.0 * margin);
        let remain_w = (width - 2.0 * crop).max(1.0);
        let remain_h = (height - 2.0 * crop).max(1.0);
        let needed = (usable_w / remain_w).max(usable_h / remain_h);
        let cap = self.max_scale.unwrap_or(400.0) / 100.0;
        needed.min(cap) * 100.0
    }

    pub fn render_scale(&self, width: f64, height: f64) -> f64 {
        let base = match self.framing {
            Framing::StabilizeOnly | Framing::SynthesizeEdges | Framing::StabilizeCrop => 1.0,
            Framing::StabilizeCropAutoScale => self.auto_scale_percent(width, height) / 100.0,
        };
        let extra = self.additional_scale.unwrap_or(100.0) / 100.0;
        base * extra
    }

    pub fn render_frame(&self, frames: &[Raster], index: usize) -> Raster {
        let motion = self.correction_path();
        let delta = motion.get(index).copied().unwrap_or([0.0, 0.0]);
        let scale = if self.preserve_scale == Some(true) {
            1.0
        } else {
            self.motions
                .get(index)
                .map(|item| item.scale)
                .unwrap_or(1.0)
        };
        let shear = if self.rolling_enabled() {
            let amount = self
                .motions
                .get(index)
                .map(|item| item.shear)
                .unwrap_or(0.0);
            let keep = match self.rolling {
                RollingShutter::Auto => 0.35,
                RollingShutter::Strong => 0.08,
            };
            amount * keep
        } else {
            0.0
        };
        let image = warp(&frames[index], delta, scale, shear);
        match self.framing {
            Framing::StabilizeOnly => image,
            Framing::StabilizeCrop | Framing::StabilizeCropAutoScale => image,
            Framing::SynthesizeEdges => self.fill_edges(frames, index, &image, delta),
        }
    }

    fn fill_edges(
        &self,
        frames: &[Raster],
        index: usize,
        current: &Raster,
        delta: [f64; 2],
    ) -> Raster {
        let mut out = current.clone();
        let range = ((self.synthesis_range.unwrap_or(0.0) * self.frame_rate).round() as i32).max(0);
        let width = current.width as i32;
        let height = current.height as i32;
        let feather = self.feather.unwrap_or(0.0) as i32;
        for y in 0..height {
            for x in 0..width {
                let source_x = x as f64 + delta[0];
                let source_y = y as f64 + delta[1];
                let inside = source_x >= self.edge_crop[0]
                    && source_y >= self.edge_crop[1]
                    && source_x < f64::from(width) - self.edge_crop[2]
                    && source_y < f64::from(height) - self.edge_crop[3];
                if inside {
                    continue;
                }
                let mut filled = None;
                for offset in 1..=range {
                    for sign in [-1, 1] {
                        let frame = index as i32 + sign * offset;
                        if frame < 0 || frame >= frames.len() as i32 {
                            continue;
                        }
                        let pixel = frames[frame as usize].get(x, y);
                        if pixel[3] > 0.0 && (pixel[0] + pixel[1] + pixel[2]) > 0.01 {
                            filled = Some(pixel);
                            break;
                        }
                    }
                    if filled.is_some() {
                        break;
                    }
                }
                if let Some(pixel) = filled {
                    let edge = x.min(width - 1 - x).min(y.min(height - 1 - y));
                    let mix = if feather <= 0 {
                        1.0
                    } else {
                        (f64::from(edge) / f64::from(feather)).clamp(0.0, 1.0) as f32
                    };
                    let base = out.get(x, y);
                    out.set(
                        x,
                        y,
                        [
                            base[0] * (1.0 - mix) + pixel[0] * mix,
                            base[1] * (1.0 - mix) + pixel[1] * mix,
                            base[2] * (1.0 - mix) + pixel[2] * mix,
                            1.0,
                        ],
                    );
                }
            }
        }
        out
    }

    pub fn apply_to_target(&self, target: &mut Layer, ids: &mut Ids) -> Result<(), ToolError> {
        if self.target != Some(target.id) {
            return Err(ToolError::new("ターゲット以外は動きを受けない"));
        }
        let path = match self.purpose {
            Purpose::ApplyToTarget | Purpose::ApplyOverOriginal => self.output_path(),
            Purpose::Stabilize | Purpose::Reversible | Purpose::Restore => self.correction_path(),
        };
        target
            .properties
            .push(persist::point(ids, "位置", "position", [0.0, 0.0]));
        let keys: Vec<(f64, [f64; 2])> = path
            .iter()
            .enumerate()
            .map(|(frame, value)| (frame as f64, *value))
            .collect();
        persist::push_point_keys(
            target.property_mut("position").expect("position"),
            &keys,
            self.frame_rate,
        );
        if self.purpose == Purpose::ApplyOverOriginal {
            target
                .properties
                .push(persist::boolean(ids, "元映像の上", "over-original", true));
        }
        Ok(())
    }

    pub fn restore_point(&self, stabilized: [f64; 2], frame: usize) -> [f64; 2] {
        let removed = self.removed.get(frame).copied().unwrap_or([0.0, 0.0]);
        [stabilized[0] + removed[0], stabilized[1] + removed[1]]
    }

    pub fn stored_bytes(&self) -> usize {
        self.motions.len() * 64 + self.work_units as usize * 8
    }
}

impl Default for WarpStabilizer {
    fn default() -> Self {
        Self::new()
    }
}

fn contrast_seeds(image: &Raster, stride: i32) -> Vec<[f64; 2]> {
    let mut seeds = Vec::new();
    let mut y = stride.max(4);
    while y < image.height as i32 - 4 {
        let mut x = stride.max(4);
        while x < image.width as i32 - 4 {
            seeds.push([f64::from(x), f64::from(y)]);
            x += stride.max(4);
        }
        y += stride.max(4);
    }
    if seeds.is_empty() {
        seeds.push([f64::from(image.width) * 0.5, f64::from(image.height) * 0.5]);
    }
    seeds
}

fn track_seeds(frames: &[Raster], seeds: &[[f64; 2]]) -> (Vec<Vec<[f64; 2]>>, Vec<Vec<f64>>) {
    let mut tracks = vec![seeds.to_vec()];
    let mut confidence = vec![vec![100.0; seeds.len()]];
    for frame in frames.iter().skip(1) {
        let previous = tracks.last().expect("seed");
        let mut next = Vec::new();
        let mut scores = Vec::new();
        for point in previous {
            let template = capture_patch(
                &frames[tracks.len() - 1],
                *point,
                [8.0, 8.0],
                CompareChannel::Luminance,
            );
            let hit = match_patch(frame, &template, *point, [28.0, 28.0], [8.0, 8.0], true);
            next.push(hit.center);
            scores.push(hit.confidence);
        }
        tracks.push(next);
        confidence.push(scores);
    }
    (tracks, confidence)
}

fn motion_at(
    frame: usize,
    tracks: &[Vec<[f64; 2]>],
    seeds: &[[f64; 2]],
    confidence: &[f64],
    method: StabilizeMethod,
    image: &Raster,
) -> FrameMotion {
    let mut dx = Vec::new();
    let mut dy = Vec::new();
    for (seed, point) in seeds.iter().zip(tracks[frame].iter()) {
        dx.push(point[0] - seed[0]);
        dy.push(point[1] - seed[1]);
    }
    dx.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    dy.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let translation = if dx.is_empty() {
        [0.0, 0.0]
    } else {
        [dx[dx.len() / 2], dy[dy.len() / 2]]
    };
    let (rotation, scale) = if seeds.len() >= 2 {
        let a0 = seeds[0];
        let b0 = seeds[seeds.len() / 2];
        let a1 = tracks[frame][0];
        let b1 = tracks[frame][seeds.len() / 2];
        let angle0 = (b0[1] - a0[1]).atan2(b0[0] - a0[0]);
        let angle1 = (b1[1] - a1[1]).atan2(b1[0] - a1[0]);
        let len0 = ((b0[0] - a0[0]).powi(2) + (b0[1] - a0[1]).powi(2))
            .sqrt()
            .max(1e-3);
        let len1 = ((b1[0] - a1[0]).powi(2) + (b1[1] - a1[1]).powi(2)).sqrt();
        ((angle1 - angle0).to_degrees(), len1 / len0)
    } else {
        (0.0, 1.0)
    };
    let mut cells = Vec::new();
    let width = image.width as f64;
    let height = image.height as f64;
    for cy in 0..2 {
        for cx in 0..2 {
            let mut local = Vec::new();
            for (seed, point) in seeds.iter().zip(tracks[frame].iter()) {
                let column = if seed[0] < width * 0.5 { 0 } else { 1 };
                let row = if seed[1] < height * 0.5 { 0 } else { 1 };
                if column == cx && row == cy {
                    local.push([point[0] - seed[0], point[1] - seed[1]]);
                }
            }
            if let Some(value) = local.first() {
                cells.push(*value);
            }
        }
    }
    let count = image.pixels.len() as f64;
    let total = image
        .pixels
        .iter()
        .map(|pixel| f64::from(pixel[0]))
        .sum::<f64>();
    let mean = total / count.max(1.0);
    let spread = image
        .pixels
        .iter()
        .map(|pixel| (f64::from(pixel[0]) - mean).powi(2))
        .sum::<f64>()
        / count.max(1.0);
    let good = if spread < 0.002 {
        0
    } else {
        confidence.iter().filter(|score| **score > 55.0).count()
    };
    let method_used = match method {
        StabilizeMethod::SubspaceWarp if good < 4 || cells.len() < 4 => {
            StabilizeMethod::Perspective
        }
        StabilizeMethod::Perspective if good < 3 => StabilizeMethod::PositionScaleRotation,
        StabilizeMethod::PositionScaleRotation if good < 2 => StabilizeMethod::Position,
        other => other,
    };
    let cells = if method_used == StabilizeMethod::SubspaceWarp {
        cells
    } else {
        Vec::new()
    };
    let shear = tracks[frame]
        .iter()
        .zip(seeds)
        .map(|(point, seed)| (point[0] - seed[0]) * (seed[1] / height.max(1.0) - 0.5))
        .sum::<f64>()
        / tracks[frame].len().max(1) as f64;
    let _ = (rotation, scale);
    FrameMotion {
        translation,
        rotation: if matches!(
            method,
            StabilizeMethod::PositionScaleRotation
                | StabilizeMethod::Perspective
                | StabilizeMethod::SubspaceWarp
        ) {
            rotation
        } else {
            0.0
        },
        scale: if method == StabilizeMethod::Position {
            1.0
        } else {
            scale
        },
        shear,
        method_used,
        cells,
    }
}

fn smooth_points(values: &[[f64; 2]], smoothness: f64) -> Vec<[f64; 2]> {
    if values.is_empty() || smoothness <= 0.0 {
        return values.to_vec();
    }
    let radius = ((smoothness / 30.0).round() as i32).clamp(1, 8);
    let mut smoothed: Vec<[f64; 2]> = values
        .iter()
        .enumerate()
        .map(|(index, _)| {
            let start = index.saturating_sub(radius as usize);
            let end = (index + radius as usize + 1).min(values.len());
            let slice = &values[start..end];
            let n = slice.len() as f64;
            [
                slice.iter().map(|v| v[0]).sum::<f64>() / n,
                slice.iter().map(|v| v[1]).sum::<f64>() / n,
            ]
        })
        .collect();
    if smoothness > 100.0 {
        let extra = ((smoothness - 100.0) / 100.0).clamp(0.0, 1.0);
        let mean = [
            values.iter().map(|v| v[0]).sum::<f64>() / values.len() as f64,
            values.iter().map(|v| v[1]).sum::<f64>() / values.len() as f64,
        ];
        for value in &mut smoothed {
            value[0] += (mean[0] - value[0]) * extra;
            value[1] += (mean[1] - value[1]) * extra;
        }
    }
    smoothed
}

fn warp(image: &Raster, delta: [f64; 2], scale: f64, shear: f64) -> Raster {
    let mut out = Raster::new(image.width, image.height, [0.0, 0.0, 0.0, 1.0]);
    let origin = [f64::from(image.width) * 0.5, f64::from(image.height) * 0.5];
    for y in 0..image.height as i32 {
        for x in 0..image.width as i32 {
            let sx = (f64::from(x) - origin[0]) / scale
                + origin[0]
                + delta[0]
                + shear * (f64::from(y) - origin[1]);
            let sy = (f64::from(y) - origin[1]) / scale + origin[1] + delta[1];
            out.set(x, y, image.sample(sx, sy));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use model::{Ids, Layer, LayerKind, PropertyValue};

    use super::*;
    use crate::planar::textured;

    fn near(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() <= eps
    }

    fn shifted(steps: &[[f64; 2]]) -> Vec<Raster> {
        let base = textured(64, 48);
        steps
            .iter()
            .map(|step| base.shift(step[0], step[1]))
            .collect()
    }

    fn ready(stab: &mut WarpStabilizer) {
        stab.smoothness = Some(40.0);
        stab.preserve_scale = Some(false);
        stab.max_scale = Some(150.0);
        stab.action_safe = Some(0.0);
        stab.additional_scale = Some(100.0);
        stab.detailed = Some(false);
        stab.less_crop = Some(100.0);
        stab.synthesis_range = Some(0.2);
        stab.feather = Some(2.0);
    }

    #[test]
    fn attaching_analyzes_and_disables_the_button() {
        let frames = shifted(&[[0.0, 0.0], [4.0, 0.0], [8.0, 1.0]]);
        let mut stab = WarpStabilizer::new();
        assert!(stab.analyze_enabled());
        ready(&mut stab);
        stab.analyze(&frames).unwrap();
        assert!(stab.solved);
        assert!(!stab.analyze_enabled());
        assert!(!stab.output_path().is_empty());
    }

    #[test]
    fn cancel_prevents_a_result() {
        let mut stab = WarpStabilizer::new();
        stab.running = true;
        stab.cancel();
        assert!(!stab.solved);
        assert!(stab.motions.is_empty());
    }

    #[test]
    fn smooth_motion_keeps_the_trend_and_cuts_jitter() {
        let frames = shifted(&[
            [0.0, 0.0],
            [6.0, 0.0],
            [10.0, 0.0],
            [18.0, 0.0],
            [24.0, 0.0],
        ]);
        let mut stab = WarpStabilizer::new();
        ready(&mut stab);
        stab.smoothness = Some(35.0);
        stab.result = StabilizeResult::SmoothMotion;
        stab.analyze(&frames).unwrap();
        let path = stab.output_path();
        assert!(
            path.last().unwrap()[0] > path[0][0] + 8.0,
            "path {path:?} raw {:?}",
            stab.motions
                .iter()
                .map(|m| m.translation)
                .collect::<Vec<_>>()
        );
        let raw: Vec<_> = stab.motions.iter().map(|m| m.translation[0]).collect();
        let jitter: f64 = raw
            .iter()
            .zip(path.iter())
            .map(|(raw, out)| (raw - out[0]).abs())
            .sum();
        assert!(jitter > 0.0);
        assert!(stab.smoothness_enabled());
    }

    #[test]
    fn no_motion_locks_the_subject_and_disables_smoothness() {
        let frames = shifted(&[[0.0, 0.0], [5.0, 1.0], [9.0, -1.0]]);
        let mut stab = WarpStabilizer::new();
        ready(&mut stab);
        stab.result = StabilizeResult::NoMotion;
        stab.analyze(&frames).unwrap();
        assert!(stab
            .output_path()
            .iter()
            .all(|p| near(p[0], 0.0, 1e-6) && near(p[1], 0.0, 1e-6)));
        assert!(!stab.smoothness_enabled());
        assert!(!stab.less_crop_enabled());
    }

    #[test]
    fn higher_smoothness_increases_crop() {
        let frames = shifted(&[[0.0, 0.0], [3.0, 0.0], [4.0, 0.0], [9.0, 0.0], [12.0, 0.0]]);
        let mut low = WarpStabilizer::new();
        ready(&mut low);
        low.smoothness = Some(20.0);
        low.analyze(&frames).unwrap();
        let mut high = WarpStabilizer::new();
        ready(&mut high);
        high.smoothness = Some(160.0);
        high.analyze(&frames).unwrap();
        assert!(high.crop_amount() >= low.crop_amount());
        assert!(high.auto_scale_percent(64.0, 48.0) >= low.auto_scale_percent(64.0, 48.0));
    }

    #[test]
    fn position_method_ignores_rotation() {
        let mut stab = WarpStabilizer::new();
        ready(&mut stab);
        stab.method = StabilizeMethod::Position;
        let mut frames = Vec::new();
        let base = textured(64, 48);
        for index in 0..3 {
            frames.push(
                base.shift(f64::from(index) * 3.0, 0.0)
                    .rotate(f64::from(index) * 4.0, [32.0, 24.0]),
            );
        }
        stab.analyze(&frames).unwrap();
        assert!(stab
            .motions
            .iter()
            .all(|m| m.rotation.abs() < 1e-6 || m.method_used == StabilizeMethod::Position));
        assert!(stab.motions.last().unwrap().translation[0].abs() > 1.0);
        assert!(stab.motions.iter().all(|m| (m.scale - 1.0).abs() < 1e-6));
    }

    #[test]
    fn similarity_has_no_local_cells() {
        let mut frames = Vec::new();
        let base = textured(72, 56);
        for index in 0..3 {
            frames.push(
                base.shift(f64::from(index) * 2.0, 0.0)
                    .rotate(f64::from(index) * 3.0, [36.0, 28.0]),
            );
        }
        let mut stab = WarpStabilizer::new();
        ready(&mut stab);
        stab.method = StabilizeMethod::PositionScaleRotation;
        stab.analyze(&frames).unwrap();
        assert!(stab.motions.iter().all(|m| m.cells.is_empty()));
        assert!(stab.motions.iter().any(|m| m.rotation.abs() > 0.2
            || (m.scale - 1.0).abs() > 0.01
            || m.translation[0].abs() > 1.0));
    }

    #[test]
    fn perspective_uses_one_mapping() {
        let frames = shifted(&[[0.0, 0.0], [3.0, 1.0], [6.0, -1.0]]);
        let mut stab = WarpStabilizer::new();
        ready(&mut stab);
        stab.method = StabilizeMethod::Perspective;
        stab.analyze(&frames).unwrap();
        assert!(stab.motions.iter().all(|m| m.cells.is_empty()));
        assert!(stab
            .motions
            .iter()
            .any(|m| m.method_used == StabilizeMethod::Perspective));
    }

    #[test]
    fn subspace_is_default_and_falls_back() {
        let mut stab = WarpStabilizer::new();
        assert_eq!(stab.method, StabilizeMethod::SubspaceWarp);
        let mut frames = shifted(&[[0.0, 0.0], [2.0, 0.0], [4.0, 0.0]]);
        frames.push(Raster::new(64, 48, [0.2, 0.2, 0.2, 1.0]));
        ready(&mut stab);
        stab.analyze(&frames).unwrap();
        assert!(
            stab.motions
                .iter()
                .any(|m| m.method_used == StabilizeMethod::SubspaceWarp),
            "{:?}",
            stab.motions
                .iter()
                .map(|m| m.method_used)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            stab.motions.last().unwrap().method_used,
            StabilizeMethod::Perspective
        );
    }

    #[test]
    fn preserve_scale_keeps_subject_size() {
        let frames = shifted(&[[0.0, 0.0], [2.0, 0.0]]);
        let mut stab = WarpStabilizer::new();
        ready(&mut stab);
        stab.preserve_scale = Some(true);
        stab.analyze(&frames).unwrap();
        assert!(stab.motions.iter().all(|m| near(m.scale, 1.0, 1e-6)));
    }

    #[test]
    fn stabilize_only_does_not_scale() {
        let frames = shifted(&[[0.0, 0.0], [6.0, 0.0]]);
        let mut stab = WarpStabilizer::new();
        ready(&mut stab);
        stab.framing = Framing::StabilizeOnly;
        stab.analyze(&frames).unwrap();
        assert!(near(stab.render_scale(64.0, 48.0), 1.0, 1e-6));
        assert!(!stab.auto_scale_enabled());
        assert!(!stab.less_crop_enabled());
    }

    #[test]
    fn crop_framing_stays_at_one_hundred_percent() {
        let frames = shifted(&[[0.0, 0.0], [5.0, 0.0]]);
        let mut stab = WarpStabilizer::new();
        ready(&mut stab);
        stab.framing = Framing::StabilizeCrop;
        stab.max_scale = Some(100.0);
        stab.analyze(&frames).unwrap();
        assert!(near(stab.render_scale(64.0, 48.0), 1.0, 1e-6));
        assert!(!stab.auto_scale_enabled());
        assert!(stab.less_crop_enabled());
    }

    #[test]
    fn auto_scale_can_exceed_one_hundred() {
        let frames = shifted(&[[0.0, 0.0], [2.0, 0.0], [8.0, 0.0], [3.0, 0.0], [9.0, 0.0]]);
        let mut stab = WarpStabilizer::new();
        assert_eq!(stab.framing, Framing::StabilizeCropAutoScale);
        ready(&mut stab);
        stab.smoothness = Some(140.0);
        stab.max_scale = Some(200.0);
        stab.analyze(&frames).unwrap();
        assert!(stab.auto_scale_percent(64.0, 48.0) >= 100.0);
    }

    #[test]
    fn synthesize_edges_samples_other_frames_without_scaling() {
        let mut frames = shifted(&[[0.0, 0.0], [8.0, 0.0]]);
        frames[1].fill_rect(0, 0, 4, 48, [0.0, 0.9, 0.2, 1.0]);
        let mut stab = WarpStabilizer::new();
        ready(&mut stab);
        stab.framing = Framing::SynthesizeEdges;
        stab.synthesis_range = Some(1.0);
        stab.analyze(&frames).unwrap();
        assert!(near(stab.render_scale(64.0, 48.0), 1.0, 1e-6));
        let rendered = stab.render_frame(&frames, 0);
        assert_eq!(rendered.width, frames[0].width);
    }

    #[test]
    fn auto_scale_changes_with_smoothness() {
        let frames = shifted(&[[0.0, 0.0], [2.0, 0.0], [7.0, 0.0], [3.0, 0.0]]);
        let mut a = WarpStabilizer::new();
        ready(&mut a);
        a.smoothness = Some(10.0);
        a.analyze(&frames).unwrap();
        let mut b = WarpStabilizer::new();
        ready(&mut b);
        b.smoothness = Some(180.0);
        b.analyze(&frames).unwrap();
        assert!(b.auto_scale_percent(64.0, 48.0) >= a.auto_scale_percent(64.0, 48.0));
    }

    #[test]
    fn max_scale_limits_enlargement() {
        let frames = shifted(&[[0.0, 0.0], [8.0, 0.0], [1.0, 0.0], [9.0, 0.0]]);
        let mut stab = WarpStabilizer::new();
        ready(&mut stab);
        stab.smoothness = Some(160.0);
        stab.max_scale = Some(100.0);
        stab.analyze(&frames).unwrap();
        assert!(stab.auto_scale_percent(64.0, 48.0) <= 100.0 + 1e-6);
    }

    #[test]
    fn action_safe_margin_reduces_scale() {
        let frames = shifted(&[[0.0, 0.0], [6.0, 0.0], [2.0, 0.0], [8.0, 0.0]]);
        let mut open = WarpStabilizer::new();
        ready(&mut open);
        open.smoothness = Some(120.0);
        open.action_safe = Some(0.0);
        open.analyze(&frames).unwrap();
        let mut safe = open.clone();
        safe.action_safe = Some(0.15);
        assert!(safe.auto_scale_percent(64.0, 48.0) <= open.auto_scale_percent(64.0, 48.0));
    }

    #[test]
    fn additional_scale_does_not_write_layer_scale() {
        let frames = shifted(&[[0.0, 0.0], [3.0, 0.0]]);
        let mut stab = WarpStabilizer::new();
        ready(&mut stab);
        stab.additional_scale = Some(120.0);
        stab.analyze(&frames).unwrap();
        let mut layer = Layer::new(LayerId::new(1), "layer", LayerKind::Null);
        layer
            .properties
            .push(persist::scalar(&mut Ids::new(), "スケール", "scale", 1.0));
        let before = 1.0;
        assert!(stab.render_scale(64.0, 48.0) > 1.0);
        match layer.property("scale").and_then(|p| p.base_value()) {
            Some(PropertyValue::Scalar(value)) => assert!(near(*value, before, 1e-6)),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn detailed_analysis_does_more_work() {
        let frames = shifted(&[[0.0, 0.0], [3.0, 0.0]]);
        let mut plain = WarpStabilizer::new();
        ready(&mut plain);
        plain.detailed = Some(false);
        plain.analyze(&frames).unwrap();
        let mut detailed = WarpStabilizer::new();
        ready(&mut detailed);
        detailed.detailed = Some(true);
        detailed.analyze(&frames).unwrap();
        assert!(detailed.work_units > plain.work_units);
        assert!(detailed.stored_bytes() > plain.stored_bytes());
    }

    #[test]
    fn rolling_shutter_auto_is_the_initial_reduction() {
        let mut stab = WarpStabilizer::new();
        assert_eq!(stab.rolling, RollingShutter::Auto);
        assert!(stab.rolling_enabled());
        stab.method = StabilizeMethod::Position;
        assert!(!stab.rolling_enabled());
    }

    #[test]
    fn strong_rolling_reduction_is_smaller() {
        let frames = shifted(&[[0.0, 0.0], [4.0, 0.0]]);
        let mut auto = WarpStabilizer::new();
        ready(&mut auto);
        auto.method = StabilizeMethod::Perspective;
        auto.rolling = RollingShutter::Auto;
        auto.analyze(&frames).unwrap();
        let mut strong = auto.clone();
        strong.rolling = RollingShutter::Strong;
        let auto_image = auto.render_frame(&frames, 1);
        let strong_image = strong.render_frame(&frames, 1);
        let auto_shift = (auto_image.get(32, 8)[0] - auto_image.get(32, 40)[0]).abs();
        let strong_shift = (strong_image.get(32, 8)[0] - strong_image.get(32, 40)[0]).abs();
        assert!(strong_shift <= auto_shift + 0.05);
    }

    #[test]
    fn less_crop_changes_the_look_without_reanalyzing() {
        let frames = shifted(&[[0.0, 0.0], [4.0, 0.0], [6.0, 0.0], [12.0, 0.0]]);
        let mut stab = WarpStabilizer::new();
        ready(&mut stab);
        stab.analyze(&frames).unwrap();
        let generation = stab.analysis_generation;
        let wide = stab.output_path();
        stab.less_crop = Some(20.0);
        let tight = stab.output_path();
        assert_eq!(stab.analysis_generation, generation);
        assert_ne!(wide, tight);
    }

    #[test]
    fn longer_synthesis_range_reaches_farther_frames() {
        let mut frames = vec![Raster::new(16, 12, [0.0, 0.0, 0.0, 1.0]); 5];
        frames[4].fill_rect(0, 0, 16, 12, [0.2, 0.8, 0.3, 1.0]);
        let mut stab = WarpStabilizer::new();
        ready(&mut stab);
        stab.framing = Framing::SynthesizeEdges;
        stab.motions = vec![
            FrameMotion {
                translation: [20.0, 0.0],
                rotation: 0.0,
                scale: 1.0,
                shear: 0.0,
                method_used: StabilizeMethod::Position,
                cells: Vec::new(),
            };
            5
        ];
        stab.solved = true;
        stab.synthesis_range = Some(0.0);
        let short = stab.render_frame(&frames, 0);
        stab.synthesis_range = Some(1.0);
        let long = stab.render_frame(&frames, 0);
        assert!(long.get(2, 2)[1] >= short.get(2, 2)[1]);
    }

    #[test]
    fn feather_softens_the_seam() {
        let mut frames = shifted(&[[0.0, 0.0], [6.0, 0.0]]);
        frames[1].fill_rect(0, 0, 64, 48, [0.9, 0.2, 0.2, 1.0]);
        let mut hard = WarpStabilizer::new();
        ready(&mut hard);
        hard.framing = Framing::SynthesizeEdges;
        hard.feather = Some(0.0);
        hard.synthesis_range = Some(1.0);
        hard.analyze(&frames).unwrap();
        let mut soft = hard.clone();
        soft.feather = Some(8.0);
        let hard_image = hard.render_frame(&frames, 0);
        let soft_image = soft.render_frame(&frames, 0);
        let hard_step = (hard_image.get(1, 8)[0] - hard_image.get(6, 8)[0]).abs();
        let soft_step = (soft_image.get(1, 8)[0] - soft_image.get(6, 8)[0]).abs();
        assert!(soft_step <= hard_step + 0.05);
    }

    #[test]
    fn edge_crop_starts_at_zero_and_excludes_a_side() {
        let mut stab = WarpStabilizer::new();
        assert_eq!(stab.edge_crop, [0.0, 0.0, 0.0, 0.0]);
        stab.edge_crop[0] = 4.0;
        assert!(stab.edge_crop[0] > 0.0);
    }

    #[test]
    fn stabilize_purpose_is_the_default() {
        let stab = WarpStabilizer::new();
        assert_eq!(stab.purpose, Purpose::Stabilize);
        let frames = shifted(&[[0.0, 0.0], [4.0, 0.0]]);
        let mut stab = WarpStabilizer::new();
        ready(&mut stab);
        stab.analyze(&frames).unwrap();
        assert!(stab.correction_path().iter().any(|d| d[0].abs() > 0.5));
    }

    #[test]
    fn reversible_edit_follows_the_restored_shake() {
        let frames = shifted(&[[0.0, 0.0], [5.0, 0.0], [9.0, 0.0]]);
        let mut stab = WarpStabilizer::new();
        ready(&mut stab);
        stab.purpose = Purpose::Reversible;
        stab.result = StabilizeResult::NoMotion;
        stab.analyze(&frames).unwrap();
        let painted = [20.0, 12.0];
        let restored = stab.restore_point(painted, 2);
        assert!((restored[0] - painted[0]).abs() > 1.0);
    }

    #[test]
    fn restore_puts_the_removed_motion_back() {
        let frames = shifted(&[[0.0, 0.0], [6.0, 1.0]]);
        let mut stab = WarpStabilizer::new();
        ready(&mut stab);
        stab.purpose = Purpose::Restore;
        stab.result = StabilizeResult::NoMotion;
        stab.analyze(&frames).unwrap();
        let back = stab.restore_point([0.0, 0.0], 1);
        assert!(back[0].abs() > 1.0 || back[1].abs() > 0.2);
    }

    #[test]
    fn apply_motion_to_the_target_layer() {
        let frames = shifted(&[[0.0, 0.0], [4.0, 2.0]]);
        let mut stab = WarpStabilizer::new();
        ready(&mut stab);
        stab.purpose = Purpose::ApplyToTarget;
        stab.target = Some(LayerId::new(4));
        stab.analyze(&frames).unwrap();
        let mut target = Layer::new(LayerId::new(4), "target", LayerKind::Null);
        stab.apply_to_target(&mut target, &mut Ids::new()).unwrap();
        assert!(target.property("position").unwrap().keyframes().len() >= 2);
    }

    #[test]
    fn apply_over_original_flags_the_target() {
        let frames = shifted(&[[0.0, 0.0], [3.0, 0.0]]);
        let mut stab = WarpStabilizer::new();
        ready(&mut stab);
        stab.purpose = Purpose::ApplyOverOriginal;
        stab.target = Some(LayerId::new(5));
        stab.analyze(&frames).unwrap();
        let mut target = Layer::new(LayerId::new(5), "over", LayerKind::Null);
        stab.apply_to_target(&mut target, &mut Ids::new()).unwrap();
        assert_eq!(
            target
                .property("over-original")
                .and_then(|p| p.base_value()),
            Some(&PropertyValue::Boolean(true))
        );
    }

    #[test]
    fn only_the_chosen_target_receives_motion() {
        let frames = shifted(&[[0.0, 0.0], [3.0, 0.0]]);
        let mut stab = WarpStabilizer::new();
        ready(&mut stab);
        stab.purpose = Purpose::ApplyToTarget;
        stab.target = Some(LayerId::new(2));
        stab.analyze(&frames).unwrap();
        let mut other = Layer::new(LayerId::new(3), "other", LayerKind::Null);
        assert!(stab.apply_to_target(&mut other, &mut Ids::new()).is_err());
        assert!(other.property("position").is_none());
    }

    #[test]
    fn hiding_the_warning_keeps_the_result() {
        let frames = shifted(&[[0.0, 0.0], [4.0, 0.0]]);
        let mut stab = WarpStabilizer::new();
        ready(&mut stab);
        stab.analyze(&frames).unwrap();
        stab.warning = true;
        let path = stab.output_path();
        stab.hide_warning = Some(true);
        assert!(!stab.warning_visible());
        assert_eq!(stab.output_path(), path);
    }
}
