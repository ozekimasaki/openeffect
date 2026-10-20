//! 平面トラック。四隅を一つの面として追い、コーナーピン、トランスフォーム、マスクへ適用する。

use model::{Ids, Keyframe, Layer, LayerId, Mask, PropertyValue, ShapePath, Time};

use crate::error::ToolError;
use crate::linalg::{apply_homography, homography};
use crate::match2d::{capture_patch, match_patch, Patch};
use crate::persist::{self, ensure_group};
use crate::point_track::{Direction, StepControl};
use crate::raster::{CompareChannel, FrameClip, Raster};

#[derive(Clone, Debug, PartialEq)]
pub struct PlanarSample {
    pub frame: i32,
    pub corners: [[f64; 2]; 4],
}

#[derive(Clone, Debug)]
struct Interior {
    position: [f64; 2],
    template: Patch,
}

#[derive(Clone, Debug)]
pub struct PlanarTracker {
    pub corners: [[f64; 2]; 4],
    pub defined: bool,
    pub samples: Vec<PlanarSample>,
    pub current_frame: i32,
    pub running: bool,
    pub motion_source: Option<LayerId>,
    interiors: Vec<Interior>,
}

impl PlanarTracker {
    pub fn new() -> Self {
        Self {
            corners: [[0.0, 0.0]; 4],
            defined: false,
            samples: Vec::new(),
            current_frame: 0,
            running: false,
            motion_source: None,
            interiors: Vec::new(),
        }
    }

    pub fn define(&mut self, corners: [[f64; 2]; 4]) {
        self.corners = corners;
        self.defined = true;
        self.samples.clear();
        self.interiors.clear();
    }

    pub fn analyze_button(&self) -> &'static str {
        if self.running {
            "停止"
        } else {
            "解析"
        }
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
        if !self.defined {
            return Err(ToolError::new("平面領域が未指定"));
        }
        let frames: Vec<i32> = match direction {
            Direction::Forward => (self.current_frame..=clip.out_frame).collect(),
            Direction::Backward => (clip.in_frame..=self.current_frame).rev().collect(),
        };
        self.running = true;
        for frame in frames {
            if control(self, frame) == StepControl::Stop {
                self.running = false;
                return Ok(());
            }
            let image = clip
                .frame(frame)
                .ok_or_else(|| ToolError::new("フレームがない"))?;
            self.track_image(frame, image);
        }
        self.running = false;
        Ok(())
    }

    pub fn step(&mut self, clip: &FrameClip, direction: Direction) -> Result<(), ToolError> {
        if !self.defined {
            return Err(ToolError::new("平面領域が未指定"));
        }
        let next = match direction {
            Direction::Forward => self.current_frame + 1,
            Direction::Backward => self.current_frame - 1,
        };
        let image = clip
            .frame(next)
            .ok_or_else(|| ToolError::new("フレームがない"))?;
        if self.interiors.is_empty() {
            if let Some(current) = clip.frame(self.current_frame) {
                self.capture(current);
                self.upsert(self.current_frame, self.corners);
            }
        }
        self.track_image(next, image);
        self.current_frame = next;
        Ok(())
    }

    pub fn correct(&mut self, frame: i32, corners: [[f64; 2]; 4]) {
        self.corners = corners;
        self.upsert(frame, corners);
        self.interiors.clear();
        self.current_frame = frame;
    }

    pub fn apply_corner_pin(
        &self,
        target: &mut Layer,
        ids: &mut Ids,
        frame_rate: f64,
    ) -> Result<(), ToolError> {
        if self.samples.is_empty() {
            return Err(ToolError::new("平面の追跡がない"));
        }
        for corner in 0..4 {
            let name = format!("corner-pin-{corner}");
            let label = format!("コーナー{}", corner + 1);
            let property = persist::point(ids, &label, &name, self.samples[0].corners[corner]);
            target.properties.push(property);
            let property = target.property_mut(&name).expect("corner");
            let keys: Vec<(f64, [f64; 2])> = self
                .samples
                .iter()
                .map(|sample| (f64::from(sample.frame), sample.corners[corner]))
                .collect();
            persist::push_point_keys(property, &keys, frame_rate);
        }
        Ok(())
    }

    pub fn apply_transform(
        &self,
        target: &mut Layer,
        ids: &mut Ids,
        frame_rate: f64,
    ) -> Result<(), ToolError> {
        if self.samples.is_empty() {
            return Err(ToolError::new("平面の追跡がない"));
        }
        let reference = &self.samples[0];
        let base_pos = centroid(reference.corners);
        let base_angle = edge_angle(reference.corners);
        let base_len = mean_edge(reference.corners).max(1e-6);
        let mut positions = Vec::new();
        let mut rotations = Vec::new();
        let mut scales = Vec::new();
        for sample in &self.samples {
            positions.push((f64::from(sample.frame), centroid(sample.corners)));
            rotations.push((sample.frame, edge_angle(sample.corners) - base_angle));
            scales.push((sample.frame, mean_edge(sample.corners) / base_len));
        }
        let _ = base_pos;
        target
            .properties
            .push(persist::point(ids, "位置", "position", positions[0].1));
        persist::push_point_keys(
            target.property_mut("position").expect("position"),
            &positions,
            frame_rate,
        );
        target
            .properties
            .push(persist::scalar(ids, "回転", "rotation", 0.0));
        for (frame, value) in rotations {
            target
                .property_mut("rotation")
                .expect("rotation")
                .push_keyframe(Keyframe::at(
                    Time::from_frames(f64::from(frame), frame_rate),
                    PropertyValue::Scalar(value),
                ));
        }
        target
            .properties
            .push(persist::scalar(ids, "スケール", "scale", 1.0));
        for (frame, value) in scales {
            target
                .property_mut("scale")
                .expect("scale")
                .push_keyframe(Keyframe::at(
                    Time::from_frames(f64::from(frame), frame_rate),
                    PropertyValue::Scalar(value),
                ));
        }
        Ok(())
    }

    pub fn apply_mask(
        &self,
        target: &mut Layer,
        ids: &mut Ids,
        frame_rate: f64,
    ) -> Result<(), ToolError> {
        if self.samples.is_empty() {
            return Err(ToolError::new("平面の追跡がない"));
        }
        let mut mask = Mask::new(ids.property(), "平面マスク");
        mask.path = path_from(self.samples[0].corners);
        target.masks.push(mask);
        target.properties.push(model::Property::value(
            ids.property(),
            "マスクパス",
            "mask-path",
            PropertyValue::path(path_from(self.samples[0].corners)),
        ));
        for sample in &self.samples {
            target
                .property_mut("mask-path")
                .expect("path")
                .push_keyframe(Keyframe::at(
                    Time::from_frames(f64::from(sample.frame), frame_rate),
                    PropertyValue::path(path_from(sample.corners)),
                ));
        }
        Ok(())
    }

    pub fn reset(&mut self) {
        self.samples.clear();
        self.interiors.clear();
        self.defined = false;
    }

    pub fn store(&self, layer: &mut Layer, ids: &mut Ids) {
        let root = ensure_group(layer, ids, "平面トラック", "planar-track");
        root.push_child(persist::boolean(
            ids,
            "領域",
            "planar-defined",
            self.defined,
        ));
    }

    fn track_image(&mut self, frame: i32, image: &Raster) {
        if self.interiors.is_empty() {
            self.capture(image);
            self.upsert(frame, self.corners);
            return;
        }
        let mut src = Vec::new();
        let mut dst = Vec::new();
        for point in &mut self.interiors {
            let hit = match_patch(
                image,
                &point.template,
                point.position,
                [18.0, 18.0],
                [8.0, 8.0],
                true,
            );
            src.push(point.position);
            dst.push(hit.center);
            point.position = hit.center;
        }
        if let Some(transform) = homography(&src, &dst) {
            let mut next = self.corners;
            for (corner, slot) in self.corners.into_iter().zip(next.iter_mut()) {
                if let Some(mapped) = apply_homography(&transform, corner) {
                    *slot = mapped;
                }
            }
            self.corners = next;
        }
        self.upsert(frame, self.corners);
        self.capture(image);
    }

    fn capture(&mut self, image: &Raster) {
        self.interiors.clear();
        for v in [0.25, 0.5, 0.75] {
            for u in [0.25, 0.5, 0.75] {
                let position = bilinear(self.corners, u, v);
                let template =
                    capture_patch(image, position, [8.0, 8.0], CompareChannel::Luminance);
                self.interiors.push(Interior { position, template });
            }
        }
    }

    fn upsert(&mut self, frame: i32, corners: [[f64; 2]; 4]) {
        if let Some(sample) = self.samples.iter_mut().find(|sample| sample.frame == frame) {
            sample.corners = corners;
        } else {
            self.samples.push(PlanarSample { frame, corners });
        }
    }
}

impl Default for PlanarTracker {
    fn default() -> Self {
        Self::new()
    }
}

fn bilinear(corners: [[f64; 2]; 4], u: f64, v: f64) -> [f64; 2] {
    let top = [
        corners[0][0] * (1.0 - u) + corners[1][0] * u,
        corners[0][1] * (1.0 - u) + corners[1][1] * u,
    ];
    let bottom = [
        corners[3][0] * (1.0 - u) + corners[2][0] * u,
        corners[3][1] * (1.0 - u) + corners[2][1] * u,
    ];
    [
        top[0] * (1.0 - v) + bottom[0] * v,
        top[1] * (1.0 - v) + bottom[1] * v,
    ]
}

fn centroid(corners: [[f64; 2]; 4]) -> [f64; 2] {
    [
        corners.iter().map(|p| p[0]).sum::<f64>() / 4.0,
        corners.iter().map(|p| p[1]).sum::<f64>() / 4.0,
    ]
}

fn edge_angle(corners: [[f64; 2]; 4]) -> f64 {
    (corners[1][1] - corners[0][1])
        .atan2(corners[1][0] - corners[0][0])
        .to_degrees()
}

fn mean_edge(corners: [[f64; 2]; 4]) -> f64 {
    let mut sum = 0.0;
    for index in 0..4 {
        let a = corners[index];
        let b = corners[(index + 1) % 4];
        sum += ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt();
    }
    sum / 4.0
}

fn path_from(corners: [[f64; 2]; 4]) -> ShapePath {
    let mut path = ShapePath::new();
    path.closed = true;
    path.vertices = corners.to_vec();
    path.in_tangents = vec![[0.0, 0.0]; 4];
    path.out_tangents = vec![[0.0, 0.0]; 4];
    path
}

pub fn textured(width: u32, height: u32) -> Raster {
    let mut image = Raster::new(width, height, [0.08, 0.08, 0.08, 1.0]);
    for y in (2..height as i32).step_by(4) {
        for x in (2..width as i32).step_by(4) {
            let color = [
                ((x * 3) % 10) as f32 / 10.0,
                ((y * 5) % 10) as f32 / 10.0,
                0.7,
                1.0,
            ];
            image.fill_rect(x, y, 2, 2, color);
        }
    }
    image
}

#[cfg(test)]
mod tests {
    use model::{Ids, Layer, LayerKind, PropertyValue};

    use super::*;
    use crate::persist::{key_count, point_keys, scalar_keys};

    fn near(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() <= eps
    }

    fn quad() -> [[f64; 2]; 4] {
        [[12.0, 12.0], [44.0, 12.0], [44.0, 40.0], [12.0, 40.0]]
    }

    fn shift_clip(step: f64, spin: f64) -> FrameClip {
        let base = textured(72, 64);
        let mut frames = Vec::new();
        for index in 0..4 {
            let shifted = base.shift(step * f64::from(index), 0.0);
            frames.push(shifted.rotate(
                spin * f64::from(index),
                [28.0 + step * f64::from(index), 26.0],
            ));
        }
        FrameClip::from_frames(frames, 24.0)
    }

    fn layer() -> Layer {
        Layer::new(LayerId::new(1), "target", LayerKind::Null)
    }

    #[test]
    fn defining_corners_marks_the_region() {
        let mut tracker = PlanarTracker::new();
        assert!(!tracker.defined);
        tracker.define(quad());
        assert!(tracker.defined);
        assert_eq!(tracker.corners, quad());
    }

    #[test]
    fn forward_plane_tracks_orientation() {
        let clip = shift_clip(3.0, 4.0);
        let mut tracker = PlanarTracker::new();
        tracker.define(quad());
        tracker
            .analyze(&clip, Direction::Forward, |_, _| StepControl::Continue)
            .unwrap();
        assert!(tracker.samples.iter().any(|s| s.frame == clip.out_frame));
        let start = edge_angle(tracker.samples[0].corners);
        let end = edge_angle(tracker.samples.last().unwrap().corners);
        assert!((end - start).abs() > 2.0);
        assert!(
            tracker.samples.last().unwrap().corners[0][0] > tracker.samples[0].corners[0][0] + 4.0
        );
    }

    #[test]
    fn backward_plane_fills_the_in_point() {
        let clip = shift_clip(3.0, 0.0);
        let mut tracker = PlanarTracker::new();
        tracker.define([[18.0, 12.0], [50.0, 12.0], [50.0, 40.0], [18.0, 40.0]]);
        tracker.current_frame = 3;
        tracker
            .analyze(&clip, Direction::Backward, |_, _| StepControl::Continue)
            .unwrap();
        assert!(tracker.samples.iter().any(|s| s.frame == 0));
        assert!(tracker.samples.iter().any(|s| s.frame == 3));
    }

    #[test]
    fn step_forward_updates_only_the_next_frame() {
        let clip = shift_clip(4.0, 0.0);
        let mut tracker = PlanarTracker::new();
        tracker.define(quad());
        tracker.current_frame = 0;
        tracker.step(&clip, Direction::Forward).unwrap();
        assert_eq!(tracker.current_frame, 1);
        assert!(tracker.samples.iter().any(|s| s.frame == 1));
        assert!(tracker.samples.iter().all(|s| s.frame <= 1));
    }

    #[test]
    fn step_backward_updates_only_the_previous_frame() {
        let clip = shift_clip(4.0, 0.0);
        let mut tracker = PlanarTracker::new();
        tracker.define([[20.0, 12.0], [52.0, 12.0], [52.0, 40.0], [20.0, 40.0]]);
        tracker.current_frame = 2;
        tracker.step(&clip, Direction::Backward).unwrap();
        assert_eq!(tracker.current_frame, 1);
        assert!(tracker.samples.iter().any(|s| s.frame == 1));
    }

    #[test]
    fn stop_leaves_untracked_frames_empty() {
        let clip = shift_clip(3.0, 0.0);
        let mut tracker = PlanarTracker::new();
        tracker.define(quad());
        let mut saw = false;
        tracker
            .analyze(&clip, Direction::Forward, |tracker, frame| {
                saw = tracker.analyze_button() == "停止";
                if frame >= 2 {
                    StepControl::Stop
                } else {
                    StepControl::Continue
                }
            })
            .unwrap();
        assert!(saw);
        assert!(tracker.samples.iter().all(|s| s.frame < 2));
        assert!(!tracker.running);
    }

    #[test]
    fn manual_corners_seed_the_next_solve() {
        let clip = shift_clip(4.0, 0.0);
        let mut tracker = PlanarTracker::new();
        tracker.define(quad());
        let corrected = [[16.0, 14.0], [48.0, 14.0], [48.0, 42.0], [16.0, 42.0]];
        tracker.correct(1, corrected);
        let saved = tracker.samples.iter().find(|s| s.frame == 1).unwrap();
        assert_eq!(saved.corners, corrected);
        tracker.step(&clip, Direction::Forward).unwrap();
        let next = tracker.samples.iter().find(|s| s.frame == 2).unwrap();
        assert!(next.corners[0][0] > corrected[0][0] + 1.0);
    }

    #[test]
    fn corner_pin_matches_the_plane() {
        let clip = shift_clip(4.0, 0.0);
        let mut tracker = PlanarTracker::new();
        tracker.define(quad());
        tracker
            .analyze(&clip, Direction::Forward, |_, _| StepControl::Continue)
            .unwrap();
        let mut target = layer();
        tracker
            .apply_corner_pin(&mut target, &mut Ids::new(), 24.0)
            .unwrap();
        let keys = point_keys(&target, "corner-pin-0");
        assert_eq!(keys.len(), tracker.samples.len());
        assert!(near(keys[0][0], tracker.samples[0].corners[0][0], 1e-6));
        assert!(near(
            keys.last().unwrap()[0],
            tracker.samples.last().unwrap().corners[0][0],
            1e-6
        ));
    }

    #[test]
    fn transform_apply_skips_corner_pin() {
        let clip = shift_clip(5.0, 0.0);
        let mut tracker = PlanarTracker::new();
        tracker.define(quad());
        tracker
            .analyze(&clip, Direction::Forward, |_, _| StepControl::Continue)
            .unwrap();
        let mut target = layer();
        tracker
            .apply_transform(&mut target, &mut Ids::new(), 24.0)
            .unwrap();
        assert!(key_count(&target, "position") >= 2);
        assert!(scalar_keys(&target, "rotation").len() >= 2);
        assert!(scalar_keys(&target, "scale").len() >= 2);
        assert!(target.property("corner-pin-0").is_none());
        let positions = point_keys(&target, "position");
        assert!(
            positions.last().unwrap()[0] > positions[0][0] + 4.0,
            "{positions:?}"
        );
        let spin = shift_clip(0.0, 6.0);
        let mut turning = PlanarTracker::new();
        turning.define(quad());
        turning
            .analyze(&spin, Direction::Forward, |_, _| StepControl::Continue)
            .unwrap();
        let mut spun = layer();
        turning
            .apply_transform(&mut spun, &mut Ids::new(), 24.0)
            .unwrap();
        let rotations = scalar_keys(&spun, "rotation");
        assert!(
            (rotations.last().unwrap() - rotations[0]).abs() > 2.0,
            "{rotations:?}"
        );
    }

    #[test]
    fn mask_vertices_follow_corners() {
        let clip = shift_clip(3.0, 0.0);
        let mut tracker = PlanarTracker::new();
        tracker.define(quad());
        tracker
            .analyze(&clip, Direction::Forward, |_, _| StepControl::Continue)
            .unwrap();
        let mut target = layer();
        tracker
            .apply_mask(&mut target, &mut Ids::new(), 24.0)
            .unwrap();
        assert_eq!(target.masks.len(), 1);
        assert!(target.masks[0].path.closed);
        let property = target.property("mask-path").unwrap();
        for (key, sample) in property.keyframes().iter().zip(tracker.samples.iter()) {
            match &key.value {
                PropertyValue::Path(path) => {
                    assert_eq!(path.vertices.len(), 4);
                    assert!(near(path.vertices[0][0], sample.corners[0][0], 1e-6));
                    assert!(near(path.vertices[2][1], sample.corners[2][1], 1e-6));
                }
                other => panic!("unexpected {other:?}"),
            }
        }
    }

    #[test]
    fn reset_drops_track_and_keeps_applied_results() {
        let clip = shift_clip(3.0, 0.0);
        let mut tracker = PlanarTracker::new();
        tracker.define(quad());
        tracker
            .analyze(&clip, Direction::Forward, |_, _| StepControl::Continue)
            .unwrap();
        let mut target = layer();
        let mut ids = Ids::new();
        tracker
            .apply_corner_pin(&mut target, &mut ids, 24.0)
            .unwrap();
        tracker
            .apply_transform(&mut target, &mut ids, 24.0)
            .unwrap();
        let pins = key_count(&target, "corner-pin-0");
        tracker.reset();
        assert!(tracker.samples.is_empty());
        assert!(!tracker.defined);
        assert_eq!(key_count(&target, "corner-pin-0"), pins);
        assert!(key_count(&target, "position") >= 2);
    }
}
