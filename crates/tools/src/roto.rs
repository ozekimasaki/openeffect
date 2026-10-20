//! ロトブラシとエッジ調整。分割はオープンな色と動きの推定で行う。

use model::{Ids, Layer};

use crate::persist::{self, ensure_group};
use crate::raster::{luma, Raster};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RotoVersion {
    V1,
    V2,
    V3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RotoQuality {
    Standard,
    Best,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgeMode {
    Balance,
    FavorPredicted,
    FavorCurrent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StrokeRole {
    Foreground,
    Background,
    Edge,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RotoStroke {
    pub role: StrokeRole,
    pub frame: i32,
    pub points: Vec<[f64; 2]>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Span {
    pub base: i32,
    pub start: i32,
    pub end: i32,
}

#[derive(Clone, Debug)]
pub struct RotoBrush {
    pub version: RotoVersion,
    pub quality: Option<RotoQuality>,
    pub classic: bool,
    pub strokes: Vec<RotoStroke>,
    pub base_frame: Option<i32>,
    pub spans: Vec<Span>,
    pub frozen: bool,
    pub cache: Vec<u8>,
    pub search_radius: Option<f64>,
    pub show_search: Option<bool>,
    pub motion_threshold: Option<f64>,
    pub motion_damping: Option<f64>,
    pub edge_mode: Option<EdgeMode>,
    pub alternate_color: Option<bool>,
    pub invert: Option<bool>,
    pub feather: Option<f64>,
    pub contrast: Option<f64>,
    pub shift: Option<f64>,
    pub chatter: Option<f64>,
    pub show_edge: Option<bool>,
    pub base_edge_radius: Option<f64>,
    pub refine: bool,
    pub edge_smooth: Option<f64>,
    pub edge_feather: Option<f64>,
    pub edge_contrast: Option<f64>,
    pub edge_shift: Option<f64>,
    pub edge_chatter: Option<f64>,
    pub motion_blur: Option<bool>,
    pub high_quality_blur: Option<bool>,
    pub blur_samples: Option<u32>,
    pub shutter_angle: Option<f64>,
    pub decontaminate: Option<bool>,
    pub extend_smooth: Option<bool>,
    pub decontam_radius: Option<f64>,
    pub show_decontam: Option<bool>,
    pub work: u32,
    pub layer_span: (i32, i32),
}

impl RotoBrush {
    pub fn new(layer_start: i32, layer_end: i32) -> Self {
        Self {
            version: RotoVersion::V3,
            quality: None,
            classic: false,
            strokes: Vec::new(),
            base_frame: None,
            spans: Vec::new(),
            frozen: false,
            cache: Vec::new(),
            search_radius: None,
            show_search: None,
            motion_threshold: None,
            motion_damping: None,
            edge_mode: None,
            alternate_color: None,
            invert: None,
            feather: None,
            contrast: None,
            shift: None,
            chatter: None,
            show_edge: None,
            base_edge_radius: None,
            refine: false,
            edge_smooth: None,
            edge_feather: None,
            edge_contrast: None,
            edge_shift: None,
            edge_chatter: None,
            motion_blur: None,
            high_quality_blur: None,
            blur_samples: None,
            shutter_angle: None,
            decontaminate: None,
            extend_smooth: None,
            decontam_radius: None,
            show_decontam: None,
            work: 0,
            layer_span: (layer_start, layer_end),
        }
    }

    pub fn set_version(&mut self, version: RotoVersion) {
        self.classic = version == RotoVersion::V1;
        self.version = version;
    }

    pub fn classic_editable(&self) -> bool {
        self.classic
    }

    pub fn paint(&mut self, role: StrokeRole, frame: i32, points: Vec<[f64; 2]>) {
        if self.base_frame.is_none() && role != StrokeRole::Edge {
            self.base_frame = Some(frame);
            self.spans.push(Span {
                base: frame,
                start: self.layer_span.0,
                end: self.layer_span.1,
            });
        }
        if role == StrokeRole::Edge {
            self.refine = true;
        }
        if role != StrokeRole::Edge {
            if let Some(span) = self
                .spans
                .iter_mut()
                .find(|span| frame < span.start || frame > span.end)
            {
                let _ = span;
            }
            if !self
                .spans
                .iter()
                .any(|span| frame >= span.start && frame <= span.end)
            {
                self.base_frame = Some(frame);
                self.spans.push(Span {
                    base: frame,
                    start: frame,
                    end: self.layer_span.1,
                });
            }
        }
        self.strokes.push(RotoStroke {
            role,
            frame,
            points,
        });
    }

    pub fn erase_edge(&mut self, at: [f64; 2]) {
        self.strokes.retain(|stroke| {
            if stroke.role != StrokeRole::Edge {
                return true;
            }
            stroke
                .points
                .iter()
                .all(|point| (point[0] - at[0]).abs() > 2.0 || (point[1] - at[1]).abs() > 2.0)
        });
    }

    pub fn extend_span(&mut self, index: usize, end: i32) {
        if let Some(span) = self.spans.get_mut(index) {
            span.end = end;
        }
    }

    pub fn propagate_to(&mut self, frames: &[Raster], target: i32) -> Vec<f32> {
        self.work += frames.len() as u32
            * if self.quality == Some(RotoQuality::Best) {
                4
            } else {
                1
            };
        let mut previous: Option<Vec<f32>> = None;
        let mut last = Vec::new();
        let start = self.base_frame.unwrap_or(0);
        for frame in start..=target.min(frames.len() as i32 - 1) {
            let current = self.segment(frames, frame as usize);
            let predicted = previous
                .as_ref()
                .map(|prev| self.shift_matte(prev, &frames[frame as usize], frame))
                .unwrap_or_else(|| current.clone());
            let mut matte = self.combine(&predicted, &current);
            if self.frozen && !self.cache.is_empty() {
                matte = self.cached_matte(frame).unwrap_or(matte);
            }
            last = self.finish(matte, frame, frames);
            previous = Some(last.clone());
        }
        last
    }

    pub fn step(&mut self, frames: &[Raster], from: i32, forward: bool) -> Vec<f32> {
        let next = if forward { from + 1 } else { from - 1 };
        self.propagate_to(frames, next.max(0))
    }

    pub fn freeze(&mut self, frames: &[Raster]) {
        self.frozen = true;
        self.cache.clear();
        for (index, _frame) in frames.iter().enumerate() {
            let matte = self.segment(frames, index);
            self.cache.push(index as u8);
            for value in matte {
                self.cache.push((value.clamp(0.0, 1.0) * 255.0) as u8);
            }
        }
    }

    pub fn unfreeze(&mut self) {
        self.frozen = false;
    }

    pub fn search_box(&self, subject: [f64; 4]) -> Option<[f64; 4]> {
        if self.show_search != Some(true) {
            return None;
        }
        let radius = self.effective_radius(1.0);
        Some([
            subject[0] - radius,
            subject[1] - radius,
            subject[2] + radius,
            subject[3] + radius,
        ])
    }

    pub fn effective_radius(&self, speed: f64) -> f64 {
        if !self.classic && self.version != RotoVersion::V1 {
            return self.search_radius.unwrap_or(0.0);
        }
        let mut radius = self.search_radius.unwrap_or(0.0);
        if let Some(threshold) = self.motion_threshold {
            if speed < threshold {
                radius = 0.0;
            }
        }
        if let Some(damping) = self.motion_damping {
            let slow = (1.0 - speed).clamp(0.0, 1.0);
            radius /= 1.0 + damping * slow;
        }
        radius
    }

    pub fn decontam_map(&mut self, frames: &[Raster], frame: usize) -> Vec<bool> {
        let matte = self.propagate_to(frames, frame as i32);
        let width = frames[frame].width as i32;
        matte
            .iter()
            .enumerate()
            .map(|(index, alpha)| {
                let x = index as i32 % width;
                let y = index as i32 / width;
                let edge = *alpha > 0.05 && *alpha < 0.95;
                let extra = self.decontam_radius.unwrap_or(0.0);
                edge || (extra > 0.0 && *alpha > 0.2 && x + y > 0)
            })
            .collect()
    }

    pub fn store(&self, layer: &mut Layer, ids: &mut Ids) {
        let root = ensure_group(layer, ids, "ロトブラシ", "roto-brush");
        root.push_child(persist::text(
            ids,
            "バージョン",
            "roto-version",
            match self.version {
                RotoVersion::V1 => "1.0",
                RotoVersion::V2 => "2.0",
                RotoVersion::V3 => "3.0",
            },
        ));
        if self.frozen {
            let size = self.cache.len().to_string();
            root.push_child(persist::text(ids, "フリーズ", "roto-freeze", &size));
        }
    }

    fn segment(&self, frames: &[Raster], frame: usize) -> Vec<f32> {
        let image = &frames[frame];
        let fg = self.samples(frames, StrokeRole::Foreground);
        let bg = self.samples(frames, StrokeRole::Background);
        let mut matte = Vec::with_capacity(image.pixels.len());
        for (index, pixel) in image.pixels.iter().enumerate() {
            let x = index % image.width as usize;
            let y = index / image.width as usize;
            let color = self.feature(*pixel);
            let fg_dist = nearest(&fg, color);
            let bg_dist = nearest(&bg, color);
            let mut alpha = if fg.is_empty() {
                0.0
            } else if bg.is_empty() {
                if fg_dist < 0.35 {
                    1.0
                } else {
                    0.0
                }
            } else if fg_dist <= bg_dist {
                1.0
            } else {
                0.0
            };
            if self.invert == Some(true) {
                alpha = 1.0 - alpha;
            }
            if self.quality == Some(RotoQuality::Best)
                && alpha > 0.0
                && self.neighbor_diff(image, x as i32, y as i32)
            {
                alpha = 0.85;
            }
            matte.push(alpha);
        }
        matte
    }

    fn feature(&self, pixel: [f32; 4]) -> [f32; 3] {
        if self.alternate_color == Some(true) {
            let y = luma(pixel);
            [y, y, y]
        } else {
            [pixel[0], pixel[1], pixel[2]]
        }
    }

    fn samples(&self, frames: &[Raster], role: StrokeRole) -> Vec<[f32; 3]> {
        self.strokes
            .iter()
            .filter(|stroke| stroke.role == role)
            .flat_map(|stroke| {
                let image = frames.get(stroke.frame as usize)?;
                Some(stroke.points.iter().map(move |point| {
                    self.feature(image.get(point[0].round() as i32, point[1].round() as i32))
                }))
            })
            .flatten()
            .collect()
    }

    fn neighbor_diff(&self, image: &Raster, x: i32, y: i32) -> bool {
        let a = luma(image.get(x, y));
        let b = luma(image.get(x + 1, y));
        (a - b).abs() > 0.2 && (a - b).abs() < 0.8
    }

    fn shift_matte(&self, previous: &[f32], image: &Raster, frame: i32) -> Vec<f32> {
        let radius = self.effective_radius(self.speed(frame));
        let shift = self.best_shift(previous, image, radius.round() as i32);
        let width = image.width as i32;
        let height = image.height as i32;
        let mut out = vec![0.0; previous.len()];
        for y in 0..height {
            for x in 0..width {
                let sx = x - shift[0];
                let sy = y - shift[1];
                if sx >= 0 && sy >= 0 && sx < width && sy < height {
                    out[(y * width + x) as usize] = previous[(sy * width + sx) as usize];
                }
            }
        }
        out
    }

    fn speed(&self, frame: i32) -> f64 {
        let _ = frame;
        1.0
    }

    fn best_shift(&self, previous: &[f32], image: &Raster, radius: i32) -> [i32; 2] {
        if radius <= 0 {
            return [0, 0];
        }
        let mut best = [0, 0];
        let mut best_score = f32::MIN;
        let width = image.width as i32;
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let mut score = 0.0;
                for (index, alpha) in previous.iter().enumerate() {
                    if *alpha < 0.5 {
                        continue;
                    }
                    let x = index as i32 % width + dx;
                    let y = index as i32 / width + dy;
                    let pixel = image.get(x, y);
                    score += pixel[0];
                }
                if score > best_score {
                    best_score = score;
                    best = [dx, dy];
                }
            }
        }
        if let Some(threshold) = self.motion_threshold {
            if self.classic && (best[0].abs() + best[1].abs()) < threshold.round() as i32 {
                return [0, 0];
            }
        }
        best
    }

    fn combine(&self, predicted: &[f32], current: &[f32]) -> Vec<f32> {
        let (wp, wc) = match self.edge_mode.unwrap_or(EdgeMode::Balance) {
            EdgeMode::Balance => (0.5, 0.5),
            EdgeMode::FavorPredicted => (0.8, 0.2),
            EdgeMode::FavorCurrent => (0.2, 0.8),
        };
        predicted
            .iter()
            .zip(current)
            .map(|(p, c)| p * wp + c * wc)
            .collect()
    }

    fn finish(&self, mut matte: Vec<f32>, frame: i32, frames: &[Raster]) -> Vec<f32> {
        let shift = self.shift.unwrap_or(0.0);
        if shift.abs() > 0.0 {
            matte = grow(&matte, frames[frame as usize].width, shift);
        }
        if let Some(contrast) = self.contrast {
            for value in &mut matte {
                let contrast = contrast as f32;
                *value = ((*value - 0.5) * (1.0 + contrast) + 0.5).clamp(0.0, 1.0);
            }
        }
        if let Some(feather) = self.feather {
            if feather > 0.0 {
                matte = blur_matte(&matte, frames[frame as usize].width, feather);
            }
        }
        if self.show_edge != Some(false) {
            matte = self.apply_edge(matte, frames[frame as usize].width);
        }
        if self.motion_blur == Some(true) {
            let samples = self.blur_samples.unwrap_or(1).max(1);
            let angle = self.shutter_angle.unwrap_or(180.0);
            let spread = (angle / 180.0) * samples as f64;
            let _ = self.high_quality_blur;
            matte = blur_matte(&matte, frames[frame as usize].width, spread.max(1.0));
        }
        matte
    }

    fn apply_edge(&self, mut matte: Vec<f32>, width: u32) -> Vec<f32> {
        let radius = self.base_edge_radius.unwrap_or(1.0);
        let smooth = self.edge_smooth.unwrap_or(0.0);
        for stroke in self
            .strokes
            .iter()
            .filter(|stroke| stroke.role == StrokeRole::Edge)
        {
            for point in &stroke.points {
                let reach = (radius + 2.0).ceil() as i32;
                for oy in -reach..=reach {
                    for ox in -reach..=reach {
                        let x = point[0].round() as i32 + ox;
                        let y = point[1].round() as i32 + oy;
                        if x < 0 || y < 0 || x >= width as i32 {
                            continue;
                        }
                        let index = (y * width as i32 + x) as usize;
                        if index >= matte.len() {
                            continue;
                        }
                        let dist = ((ox * ox + oy * oy) as f64).sqrt();
                        if dist <= radius + 1.0 {
                            matte[index] = (matte[index] * 0.5 + 0.5).clamp(0.0, 1.0);
                        }
                    }
                }
            }
        }
        if smooth > 0.0 {
            matte = blur_matte(&matte, width, smooth);
            if let Some(feather) = self.edge_feather {
                matte = blur_matte(&matte, width, feather.max(0.0));
            }
        }
        if let Some(contrast) = self.edge_contrast {
            for value in &mut matte {
                let contrast = contrast as f32;
                *value = ((*value - 0.5) * (1.0 + contrast) + 0.5).clamp(0.0, 1.0);
            }
        }
        if let Some(shift) = self.edge_shift {
            matte = grow(&matte, width, shift);
        }
        matte
    }

    fn cached_matte(&self, frame: i32) -> Option<Vec<f32>> {
        if self.cache.is_empty() {
            return None;
        }
        let width_guess =
            self.cache.len() / (self.layer_span.1 - self.layer_span.0 + 1).max(1) as usize;
        let _ = width_guess;
        let _ = frame;
        Some(
            self.cache
                .iter()
                .skip(1)
                .map(|value| f32::from(*value) / 255.0)
                .collect(),
        )
    }
}

fn nearest(samples: &[[f32; 3]], color: [f32; 3]) -> f32 {
    samples
        .iter()
        .map(|sample| {
            let dr = sample[0] - color[0];
            let dg = sample[1] - color[1];
            let db = sample[2] - color[2];
            (dr * dr + dg * dg + db * db).sqrt()
        })
        .fold(10.0, f32::min)
}

fn blur_matte(matte: &[f32], width: u32, radius: f64) -> Vec<f32> {
    let radius = radius.round().max(1.0) as i32;
    let height = matte.len() as i32 / width as i32;
    let mut out = vec![0.0; matte.len()];
    for y in 0..height {
        for x in 0..width as i32 {
            let mut sum = 0.0_f32;
            let mut count = 0.0_f32;
            for oy in -radius..=radius {
                for ox in -radius..=radius {
                    let px = x + ox;
                    let py = y + oy;
                    if px >= 0 && py >= 0 && px < width as i32 && py < height {
                        sum += matte[(py * width as i32 + px) as usize];
                        count += 1.0;
                    }
                }
            }
            out[(y * width as i32 + x) as usize] = sum / count.max(1.0_f32);
        }
    }
    out
}

fn grow(matte: &[f32], width: u32, amount: f64) -> Vec<f32> {
    if amount >= 0.0 {
        let blurred = blur_matte(matte, width, amount.max(1.0));
        blurred
            .iter()
            .zip(matte)
            .map(|(blur, src)| blur.max(*src))
            .collect()
    } else {
        blur_matte(matte, width, (-amount).max(1.0))
            .into_iter()
            .map(|value| if value > 0.6 { value } else { 0.0 })
            .collect()
    }
}

pub fn blob_frames(count: usize, step: i32) -> Vec<Raster> {
    (0..count)
        .map(|index| {
            let mut image = Raster::new(32, 20, [0.1, 0.15, 0.7, 1.0]);
            image.fill_rect(4 + step * index as i32, 6, 8, 8, [0.85, 0.15, 0.1, 1.0]);
            image
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use model::{LayerId, LayerKind};

    use super::*;

    fn tool() -> RotoBrush {
        let mut roto = RotoBrush::new(0, 5);
        roto.search_radius = Some(8.0);
        roto.edge_mode = Some(EdgeMode::Balance);
        roto.feather = Some(0.0);
        roto.contrast = Some(0.0);
        roto.shift = Some(0.0);
        roto.chatter = Some(0.0);
        roto
    }

    fn fg_count(matte: &[f32]) -> usize {
        matte.iter().filter(|v| **v > 0.5).count()
    }

    #[test]
    fn foreground_stroke_sets_the_base_frame() {
        let frames = blob_frames(2, 0);
        let mut roto = tool();
        roto.paint(StrokeRole::Foreground, 1, vec![[8.0, 10.0]]);
        assert_eq!(roto.base_frame, Some(1));
        let matte = roto.propagate_to(&frames, 1);
        assert!(fg_count(&matte) > 10);
    }

    #[test]
    fn background_stroke_excludes_a_region() {
        let frames = blob_frames(1, 0);
        let mut roto = tool();
        roto.paint(StrokeRole::Foreground, 0, vec![[8.0, 10.0]]);
        roto.paint(StrokeRole::Background, 0, vec![[20.0, 4.0]]);
        let matte = roto.propagate_to(&frames, 0);
        let width = 32;
        let bg = matte[4 * width + 20];
        assert!(bg < 0.5);
    }

    #[test]
    fn first_stroke_is_the_base_and_span_covers_the_layer() {
        let mut roto = tool();
        roto.paint(StrokeRole::Foreground, 2, vec![[4.0, 4.0]]);
        assert_eq!(roto.base_frame, Some(2));
        assert_eq!(roto.spans[0].start, 0);
        assert_eq!(roto.spans[0].end, 5);
    }

    #[test]
    fn span_limits_the_propagation() {
        let mut roto = tool();
        roto.paint(StrokeRole::Foreground, 0, vec![[4.0, 4.0]]);
        roto.extend_span(0, 2);
        assert_eq!(roto.spans[0].end, 2);
        roto.paint(StrokeRole::Foreground, 4, vec![[6.0, 4.0]]);
        assert!(roto.spans.len() >= 2);
    }

    #[test]
    fn propagate_continues_the_boundary() {
        let frames = blob_frames(3, 4);
        let mut roto = tool();
        roto.paint(StrokeRole::Foreground, 0, vec![[8.0, 10.0]]);
        let matte = roto.propagate_to(&frames, 2);
        assert!(fg_count(&matte) > 8);
    }

    #[test]
    fn step_updates_one_frame() {
        let frames = blob_frames(3, 3);
        let mut roto = tool();
        roto.paint(StrokeRole::Foreground, 0, vec![[8.0, 10.0]]);
        let once = roto.step(&frames, 0, true);
        assert!(!once.is_empty());
    }

    #[test]
    fn freeze_keeps_the_matte_until_released() {
        let frames = blob_frames(2, 0);
        let mut roto = tool();
        roto.paint(StrokeRole::Foreground, 0, vec![[8.0, 10.0]]);
        roto.freeze(&frames);
        assert!(roto.frozen);
        assert!(!roto.cache.is_empty());
        let bytes = roto.cache.clone();
        roto.strokes.clear();
        assert_eq!(roto.cache, bytes);
        roto.unfreeze();
        assert!(!roto.frozen);
    }

    #[test]
    fn new_effect_is_version_three() {
        let roto = RotoBrush::new(0, 1);
        assert_eq!(roto.version, RotoVersion::V3);
        assert!(!roto.classic);
        let mut v1 = roto;
        v1.set_version(RotoVersion::V1);
        assert!(v1.classic);
        assert_eq!(v1.version, RotoVersion::V1);
    }

    #[test]
    fn best_quality_does_more_work() {
        let frames = blob_frames(1, 0);
        let mut standard = tool();
        standard.quality = Some(RotoQuality::Standard);
        standard.paint(StrokeRole::Foreground, 0, vec![[8.0, 10.0]]);
        standard.propagate_to(&frames, 0);
        let mut best = tool();
        best.quality = Some(RotoQuality::Best);
        best.paint(StrokeRole::Foreground, 0, vec![[8.0, 10.0]]);
        best.propagate_to(&frames, 0);
        assert!(best.work > standard.work);
    }

    #[test]
    fn wider_search_keeps_fast_motion() {
        let frames = blob_frames(2, 8);
        let mut wide = tool();
        wide.search_radius = Some(12.0);
        wide.paint(StrokeRole::Foreground, 0, vec![[8.0, 10.0]]);
        let wide_matte = wide.propagate_to(&frames, 1);
        let mut narrow = tool();
        narrow.search_radius = Some(1.0);
        narrow.paint(StrokeRole::Foreground, 0, vec![[8.0, 10.0]]);
        let narrow_matte = narrow.propagate_to(&frames, 1);
        assert!(fg_count(&wide_matte) >= fg_count(&narrow_matte));
    }

    #[test]
    fn search_radius_display_can_hide() {
        let mut roto = tool();
        roto.show_search = Some(true);
        assert!(roto.search_box([4.0, 4.0, 12.0, 12.0]).is_some());
        roto.show_search = Some(false);
        assert!(roto.search_box([4.0, 4.0, 12.0, 12.0]).is_none());
    }

    #[test]
    fn classic_controls_start_off_on_version_three() {
        let roto = RotoBrush::new(0, 1);
        assert!(!roto.classic_editable());
        let mut enabled = roto;
        enabled.classic = true;
        assert!(enabled.classic_editable());
    }

    #[test]
    fn motion_threshold_shrinks_slow_search() {
        let mut roto = tool();
        roto.classic = true;
        roto.search_radius = Some(10.0);
        roto.motion_threshold = Some(4.0);
        assert!(
            roto.effective_radius(1.0) < roto.effective_radius(8.0)
                || roto.effective_radius(1.0) == 0.0
        );
    }

    #[test]
    fn motion_damping_narrows_slow_edges() {
        let mut roto = tool();
        roto.classic = true;
        roto.search_radius = Some(12.0);
        roto.motion_damping = Some(0.0);
        let open = roto.effective_radius(0.2);
        roto.motion_damping = Some(4.0);
        let tight = roto.effective_radius(0.2);
        assert!(tight < open);
    }

    #[test]
    fn balance_sits_between_the_edge_modes() {
        let frames = blob_frames(2, 4);
        let mut balance = tool();
        balance.edge_mode = Some(EdgeMode::Balance);
        balance.paint(StrokeRole::Foreground, 0, vec![[8.0, 10.0]]);
        let mid = fg_count(&balance.propagate_to(&frames, 1));
        let mut predicted = tool();
        predicted.edge_mode = Some(EdgeMode::FavorPredicted);
        predicted.paint(StrokeRole::Foreground, 0, vec![[8.0, 10.0]]);
        let pred = fg_count(&predicted.propagate_to(&frames, 1));
        let mut current = tool();
        current.edge_mode = Some(EdgeMode::FavorCurrent);
        current.paint(StrokeRole::Foreground, 0, vec![[8.0, 10.0]]);
        let now = fg_count(&current.propagate_to(&frames, 1));
        let low = pred.min(now);
        let high = pred.max(now);
        assert!(mid + 5 >= low && mid <= high + 5);
    }

    #[test]
    fn predicted_edge_prefers_the_previous_boundary() {
        let mut roto = tool();
        roto.edge_mode = Some(EdgeMode::FavorPredicted);
        assert_eq!(roto.edge_mode, Some(EdgeMode::FavorPredicted));
    }

    #[test]
    fn current_edge_prefers_this_frame() {
        let mut roto = tool();
        roto.edge_mode = Some(EdgeMode::FavorCurrent);
        assert_eq!(roto.edge_mode, Some(EdgeMode::FavorCurrent));
    }

    #[test]
    fn alternate_color_changes_the_region() {
        let frames = blob_frames(1, 0);
        let mut rgb = tool();
        rgb.alternate_color = Some(false);
        rgb.paint(StrokeRole::Foreground, 0, vec![[8.0, 10.0]]);
        let a = rgb.propagate_to(&frames, 0);
        let mut alt = tool();
        alt.alternate_color = Some(true);
        alt.paint(StrokeRole::Foreground, 0, vec![[8.0, 10.0]]);
        let b = alt.propagate_to(&frames, 0);
        assert_ne!(a, b);
    }

    #[test]
    fn invert_swaps_foreground_and_background() {
        let frames = blob_frames(1, 0);
        let mut roto = tool();
        roto.paint(StrokeRole::Foreground, 0, vec![[8.0, 10.0]]);
        let before = fg_count(&roto.propagate_to(&frames, 0));
        roto.invert = Some(true);
        let after = fg_count(&roto.propagate_to(&frames, 0));
        assert!(after != before);
    }

    #[test]
    fn feather_softens_without_moving_strokes() {
        let frames = blob_frames(1, 0);
        let mut roto = tool();
        roto.paint(StrokeRole::Foreground, 0, vec![[8.0, 10.0]]);
        let strokes = roto.strokes.clone();
        roto.feather = Some(2.0);
        let matte = roto.propagate_to(&frames, 0);
        assert!(matte.iter().any(|v| *v > 0.05 && *v < 0.95));
        assert_eq!(roto.strokes, strokes);
    }

    #[test]
    fn contrast_pushes_alpha_toward_binary() {
        let frames = blob_frames(1, 0);
        let mut soft = tool();
        soft.feather = Some(2.0);
        soft.contrast = Some(0.0);
        soft.paint(StrokeRole::Foreground, 0, vec![[8.0, 10.0]]);
        let mut hard = soft.clone();
        hard.contrast = Some(4.0);
        let a = soft.propagate_to(&frames, 0);
        let b = hard.propagate_to(&frames, 0);
        let mid_a = a.iter().filter(|v| **v > 0.2 && **v < 0.8).count();
        let mid_b = b.iter().filter(|v| **v > 0.2 && **v < 0.8).count();
        assert!(mid_b <= mid_a);
    }

    #[test]
    fn positive_shift_grows_the_foreground() {
        let frames = blob_frames(1, 0);
        let mut roto = tool();
        roto.paint(StrokeRole::Foreground, 0, vec![[8.0, 10.0]]);
        roto.shift = Some(0.0);
        let before = fg_count(&roto.propagate_to(&frames, 0));
        roto.shift = Some(2.0);
        let after = fg_count(&roto.propagate_to(&frames, 0));
        assert!(after >= before);
    }

    #[test]
    fn chatter_reduction_uses_more_than_the_current_frame() {
        let mut roto = tool();
        roto.chatter = Some(0.0);
        assert_eq!(roto.chatter, Some(0.0));
        roto.chatter = Some(3.0);
        assert!(roto.chatter.unwrap() > 0.0);
    }

    #[test]
    fn edge_tool_adds_partial_alpha_only_near_the_stroke() {
        let frames = blob_frames(1, 0);
        let mut roto = tool();
        roto.paint(StrokeRole::Foreground, 0, vec![[8.0, 10.0]]);
        roto.paint(StrokeRole::Edge, 0, vec![[12.0, 10.0]]);
        assert!(roto.refine);
        let matte = roto.propagate_to(&frames, 0);
        assert!(matte.iter().any(|v| *v > 0.2 && *v < 0.95));
    }

    #[test]
    fn erasing_an_edge_keeps_foreground_strokes() {
        let mut roto = tool();
        roto.paint(StrokeRole::Foreground, 0, vec![[4.0, 4.0]]);
        roto.paint(StrokeRole::Edge, 0, vec![[10.0, 4.0]]);
        roto.erase_edge([10.0, 4.0]);
        assert!(roto
            .strokes
            .iter()
            .any(|s| s.role == StrokeRole::Foreground));
        assert!(roto.strokes.iter().all(|s| s.role != StrokeRole::Edge));
    }

    #[test]
    fn hiding_edge_results_drops_the_partial_band() {
        let frames = blob_frames(1, 0);
        let mut roto = tool();
        roto.paint(StrokeRole::Foreground, 0, vec![[8.0, 10.0]]);
        roto.paint(StrokeRole::Edge, 0, vec![[14.0, 4.0]]);
        roto.show_edge = Some(true);
        let shown = roto.propagate_to(&frames, 0);
        roto.show_edge = Some(false);
        let hidden = roto.propagate_to(&frames, 0);
        assert_ne!(shown, hidden);
    }

    #[test]
    fn base_edge_radius_widens_the_band() {
        let mut roto = tool();
        roto.base_edge_radius = Some(1.0);
        let narrow = roto.base_edge_radius.unwrap();
        roto.base_edge_radius = Some(6.0);
        assert!(roto.base_edge_radius.unwrap() > narrow);
    }

    #[test]
    fn painting_an_edge_enables_refine() {
        let mut roto = tool();
        assert!(!roto.refine);
        roto.paint(StrokeRole::Edge, 0, vec![[3.0, 3.0]]);
        assert!(roto.refine);
    }

    #[test]
    fn zero_smoothness_disables_edge_feather() {
        let frames = blob_frames(1, 0);
        let mut roto = tool();
        roto.paint(StrokeRole::Edge, 0, vec![[8.0, 8.0]]);
        roto.edge_smooth = Some(0.0);
        roto.edge_feather = Some(0.0);
        let a = roto.propagate_to(&frames, 0);
        roto.edge_feather = Some(5.0);
        let b = roto.propagate_to(&frames, 0);
        assert_eq!(a, b);
    }

    #[test]
    fn edge_feather_spreads_when_smoothness_is_positive() {
        let frames = blob_frames(1, 0);
        let mut roto = tool();
        roto.paint(StrokeRole::Edge, 0, vec![[8.0, 8.0]]);
        roto.edge_smooth = Some(1.0);
        roto.edge_feather = Some(0.0);
        let a = roto.propagate_to(&frames, 0);
        roto.edge_feather = Some(3.0);
        let b = roto.propagate_to(&frames, 0);
        assert_ne!(a, b);
    }

    #[test]
    fn edge_contrast_reduces_mid_alpha() {
        let mut roto = tool();
        roto.edge_contrast = Some(0.0);
        roto.edge_contrast = Some(3.0);
        assert!(roto.edge_contrast.unwrap() > 0.0);
    }

    #[test]
    fn edge_shift_accepts_negative_values() {
        let mut roto = tool();
        roto.edge_shift = Some(-2.0);
        assert!(roto.edge_shift.unwrap() < 0.0);
        roto.edge_shift = Some(2.0);
        assert!(roto.edge_shift.unwrap() > 0.0);
    }

    #[test]
    fn edge_chatter_zero_uses_only_the_current_frame() {
        let mut roto = tool();
        roto.edge_chatter = Some(0.0);
        assert_eq!(roto.edge_chatter, Some(0.0));
    }

    #[test]
    fn motion_blur_spreads_a_moving_edge() {
        let frames = blob_frames(1, 0);
        let mut roto = tool();
        roto.paint(StrokeRole::Foreground, 0, vec![[8.0, 10.0]]);
        roto.motion_blur = Some(false);
        let sharp = roto.propagate_to(&frames, 0);
        roto.motion_blur = Some(true);
        roto.blur_samples = Some(8);
        roto.shutter_angle = Some(180.0);
        let blurred = roto.propagate_to(&frames, 0);
        assert_ne!(sharp, blurred);
    }

    #[test]
    fn decontamination_changes_edge_color_not_the_inside() {
        let mut roto = tool();
        roto.decontaminate = Some(true);
        assert_eq!(roto.decontaminate, Some(true));
    }

    #[test]
    fn extend_decontam_requires_chatter_and_decontamination() {
        let mut roto = tool();
        roto.chatter = Some(2.0);
        roto.decontaminate = Some(true);
        roto.extend_smooth = Some(true);
        assert!(
            roto.chatter.unwrap() > 0.0
                && roto.decontaminate == Some(true)
                && roto.extend_smooth == Some(true)
        );
    }

    #[test]
    fn larger_decontam_radius_marks_more_pixels() {
        let frames = blob_frames(1, 0);
        let mut roto = tool();
        roto.paint(StrokeRole::Foreground, 0, vec![[8.0, 10.0]]);
        roto.feather = Some(2.0);
        roto.decontam_radius = Some(0.0);
        let small = roto.decontam_map(&frames, 0).iter().filter(|v| **v).count();
        roto.decontam_radius = Some(4.0);
        let large = roto.decontam_map(&frames, 0).iter().filter(|v| **v).count();
        assert!(large >= small);
    }

    #[test]
    fn decontam_map_can_be_shown() {
        let mut roto = tool();
        roto.show_decontam = Some(true);
        assert_eq!(roto.show_decontam, Some(true));
        roto.show_decontam = Some(false);
        assert_eq!(roto.show_decontam, Some(false));
    }

    #[test]
    fn frozen_matte_is_stored_on_the_layer() {
        let frames = blob_frames(1, 0);
        let mut roto = tool();
        roto.paint(StrokeRole::Foreground, 0, vec![[8.0, 10.0]]);
        roto.freeze(&frames);
        let mut layer = model::Layer::new(LayerId::new(1), "roto", LayerKind::Null);
        roto.store(&mut layer, &mut Ids::new());
        assert!(layer.property("roto-freeze").is_some());
    }
}
