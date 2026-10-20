//! 3D カメラトラック。公開されている幾何で姿勢を解き、他社の実装は使わない。

use std::cmp::Ordering;

use model::{
    CameraNodes, Footage, FootageKind, Ids, ItemId, Layer, LayerId, LayerKind, LightKind, Project,
    PropertyValue, Rgba, Text, Time,
};

use crate::error::ToolError;
use crate::linalg::{
    closest_point_on_rays, identity, mat_mul, mul_mat_vec, normalize, rotation_between,
    rotation_ypr, scale, sub, transpose, Mat3, Vec3,
};
use crate::persist::{self, ensure_group};
use crate::raster::Raster;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShotType {
    FixedHorizontalFov,
    VariableZoom,
    SpecifiedHorizontalFov,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SolveMethod {
    Auto,
    General,
    MostlyPlanar,
    TripodPan,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConcreteMethod {
    General,
    MostlyPlanar,
    TripodPan,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointDisplay {
    Solved3d,
    Source2d,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    pub position: Vec3,
    pub rotation: Mat3,
    pub hfov: f64,
}

impl Pose {
    pub fn identity(hfov: f64) -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            rotation: identity(),
            hfov,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Observation {
    pub point_id: u64,
    pub frame: i32,
    pub xy: [f64; 2],
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScenePoint {
    pub id: u64,
    pub color: [f32; 3],
    pub position: Vec3,
    pub observations: Vec<(i32, [f64; 2])>,
}

#[derive(Clone, Debug)]
pub struct CameraTracker {
    pub shot: Option<ShotType>,
    pub hfov: Option<f64>,
    pub method: Option<SolveMethod>,
    pub method_used: Option<ConcreteMethod>,
    pub display: Option<PointDisplay>,
    pub render_points: Option<bool>,
    pub point_size: Option<f64>,
    pub detailed: Option<bool>,
    pub auto_delete_across_time: Option<bool>,
    pub hide_warning: Option<bool>,
    pub needs_resolve: bool,
    pub solved: bool,
    pub running: bool,
    pub average_error: Option<f64>,
    pub target: Option<Vec3>,
    pub target_size: Option<f64>,
    pub ground_origin: Option<Vec3>,
    pub ground_normal: Option<Vec3>,
    pub image_width: u32,
    pub image_height: u32,
    pub points: Vec<ScenePoint>,
    pub poses: Vec<(i32, Pose)>,
    observations: Vec<Observation>,
}

impl CameraTracker {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            shot: None,
            hfov: None,
            method: None,
            method_used: None,
            display: None,
            render_points: None,
            point_size: None,
            detailed: None,
            auto_delete_across_time: None,
            hide_warning: None,
            needs_resolve: false,
            solved: false,
            running: false,
            average_error: None,
            target: None,
            target_size: None,
            ground_origin: None,
            ground_normal: None,
            image_width: width,
            image_height: height,
            points: Vec::new(),
            poses: Vec::new(),
            observations: Vec::new(),
        }
    }

    pub fn hfov_editable(&self) -> bool {
        self.shot == Some(ShotType::SpecifiedHorizontalFov)
    }

    pub fn set_shot(&mut self, shot: ShotType) -> Result<(), ToolError> {
        self.shot = Some(shot);
        self.needs_resolve = true;
        if self.observations.is_empty() {
            return Ok(());
        }
        self.solve()
    }

    pub fn set_hfov(&mut self, degrees: f64) -> Result<(), ToolError> {
        if !self.hfov_editable() {
            return Err(ToolError::new(
                "水平視野角は指定ショットのときだけ編集できる",
            ));
        }
        self.hfov = Some(degrees);
        self.needs_resolve = true;
        if self.observations.is_empty() {
            return Ok(());
        }
        self.solve()
    }

    pub fn set_method(&mut self, method: SolveMethod) -> Result<(), ToolError> {
        self.method = Some(method);
        self.needs_resolve = true;
        if self.observations.is_empty() {
            return Ok(());
        }
        self.solve()
    }

    pub fn warning_visible(&self) -> bool {
        self.needs_resolve && self.hide_warning != Some(true)
    }

    pub fn display_radius(&self) -> Option<f64> {
        self.point_size
    }

    pub fn points_visible(&self, effect_selected: bool) -> bool {
        effect_selected || self.render_points == Some(true)
    }

    pub fn set_observations(&mut self, observations: Vec<Observation>) {
        self.observations = observations;
    }

    pub fn cancel(&mut self) {
        self.running = false;
        self.solved = false;
    }

    pub fn analyze_frames(&mut self, frames: &[Raster]) -> Result<(), ToolError> {
        if frames.is_empty() {
            return Err(ToolError::new("フレームがない"));
        }
        self.running = true;
        let threshold = if self.detailed == Some(true) {
            0.3
        } else {
            0.72
        };
        let dots = detect_peaks(&frames[0], threshold);
        self.observations.clear();
        for (index, (x, y, color)) in dots.iter().enumerate() {
            let id = index as u64 + 1;
            for (frame, image) in frames.iter().enumerate() {
                let xy =
                    track_color(image, *x, *y, *color).unwrap_or([f64::from(*x), f64::from(*y)]);
                self.observations.push(Observation {
                    point_id: id,
                    frame: frame as i32,
                    xy,
                });
            }
            let _ = color;
        }
        self.image_width = frames[0].width;
        self.image_height = frames[0].height;
        let colors: Vec<[f32; 3]> = dots.iter().map(|dot| dot.2).collect();
        self.solve()?;
        for (point, color) in self.points.iter_mut().zip(colors) {
            point.color = color;
        }
        self.running = false;
        Ok(())
    }

    pub fn solve(&mut self) -> Result<(), ToolError> {
        let shot = self
            .shot
            .ok_or_else(|| ToolError::new("ショットタイプは要確認のため未設定"))?;
        let method = self
            .method
            .ok_or_else(|| ToolError::new("解決方法は要確認のため未設定"))?;
        if self.observations.is_empty() {
            return Err(ToolError::new("トラック点がない"));
        }
        let width = f64::from(self.image_width);
        let height = f64::from(self.image_height);
        let ids = point_ids(&self.observations);
        let frames = frame_ids(&self.observations);
        let (used, points, poses) = match method {
            SolveMethod::TripodPan => {
                let solved = solve_tripod(
                    &self.observations,
                    &ids,
                    &frames,
                    width,
                    height,
                    shot,
                    self.hfov,
                );
                (ConcreteMethod::TripodPan, solved.0, solved.1)
            }
            SolveMethod::MostlyPlanar => {
                let solved = solve_planar(
                    &self.observations,
                    &ids,
                    &frames,
                    width,
                    height,
                    shot,
                    self.hfov,
                );
                (ConcreteMethod::MostlyPlanar, solved.0, solved.1)
            }
            SolveMethod::General => {
                let solved = solve_general(
                    &self.observations,
                    &ids,
                    &frames,
                    width,
                    height,
                    shot,
                    self.hfov,
                );
                (ConcreteMethod::General, solved.0, solved.1)
            }
            SolveMethod::Auto => {
                let tripod = solve_tripod(
                    &self.observations,
                    &ids,
                    &frames,
                    width,
                    height,
                    shot,
                    self.hfov,
                );
                let planar = solve_planar(
                    &self.observations,
                    &ids,
                    &frames,
                    width,
                    height,
                    shot,
                    self.hfov,
                );
                let general = solve_general(
                    &self.observations,
                    &ids,
                    &frames,
                    width,
                    height,
                    shot,
                    self.hfov,
                );
                let te = mean_error(&self.observations, &tripod.0, &tripod.1, width, height);
                let pe = mean_error(&self.observations, &planar.0, &planar.1, width, height);
                let ge = mean_error(&self.observations, &general.0, &general.1, width, height);
                if te <= pe && te <= ge {
                    (ConcreteMethod::TripodPan, tripod.0, tripod.1)
                } else if pe <= ge {
                    (ConcreteMethod::MostlyPlanar, planar.0, planar.1)
                } else {
                    (ConcreteMethod::General, general.0, general.1)
                }
            }
        };
        self.method_used = Some(used);
        self.points = points;
        self.poses = poses;
        self.average_error = Some(mean_error(
            &self.observations,
            &self.points,
            &self.poses,
            width,
            height,
        ));
        self.solved = true;
        self.needs_resolve = false;
        if self.target.is_none() {
            self.target = self.points.first().map(|point| point.position);
        }
        Ok(())
    }

    pub fn view_points(&self, frame: i32) -> Vec<[f64; 2]> {
        let display = self.display.unwrap_or(PointDisplay::Solved3d);
        match display {
            PointDisplay::Source2d => self
                .points
                .iter()
                .filter_map(|point| {
                    point
                        .observations
                        .iter()
                        .find(|obs| obs.0 == frame)
                        .map(|obs| obs.1)
                })
                .collect(),
            PointDisplay::Solved3d => {
                let pose = self.pose_at(frame);
                self.points
                    .iter()
                    .filter_map(|point| {
                        pose.as_ref().and_then(|pose| {
                            project(
                                point.position,
                                pose,
                                f64::from(self.image_width),
                                f64::from(self.image_height),
                            )
                        })
                    })
                    .collect()
            }
        }
    }

    pub fn stored_bytes(&self) -> usize {
        self.observations.len() * 24 + self.points.len() * 48 + self.poses.len() * 96
    }

    pub fn delete_points(&mut self, ids: &[u64], frame: Option<i32>) -> Result<(), ToolError> {
        let across = self.auto_delete_across_time == Some(true) || frame.is_none();
        if across {
            self.observations.retain(|obs| !ids.contains(&obs.point_id));
        } else if let Some(frame) = frame {
            self.observations
                .retain(|obs| !(ids.contains(&obs.point_id) && obs.frame == frame));
        }
        self.solve()
    }

    pub fn move_target(&mut self, desired: Vec3) {
        let snapped = if let (Some(origin), Some(normal)) = (self.ground_origin, self.ground_normal)
        {
            let n = normalize(normal);
            let delta = desired[0] * n[0] + desired[1] * n[1] + desired[2] * n[2]
                - (origin[0] * n[0] + origin[1] * n[1] + origin[2] * n[2]);
            sub(desired, scale(n, delta))
        } else {
            desired
        };
        self.target = Some(snapped);
    }

    pub fn set_ground(&mut self, origin: Vec3, normal: Vec3) {
        self.ground_origin = Some(origin);
        self.ground_normal = Some(normal);
        self.target = Some(origin);
    }

    fn pose_at(&self, frame: i32) -> Option<Pose> {
        self.poses
            .iter()
            .find(|pose| pose.0 == frame)
            .map(|pose| pose.1)
            .or_else(|| self.poses.first().map(|pose| pose.1))
    }
}

pub fn project(point: Vec3, pose: &Pose, width: f64, height: f64) -> Option<[f64; 2]> {
    let camera = mul_mat_vec(pose.rotation, sub(point, pose.position));
    if camera[2] <= 1e-6 {
        return None;
    }
    let focal = focal_px(width, pose.hfov);
    Some([
        width * 0.5 + focal * camera[0] / camera[2],
        height * 0.5 - focal * camera[1] / camera[2],
    ])
}

pub fn focal_px(width: f64, hfov: f64) -> f64 {
    (width * 0.5) / (hfov.to_radians() * 0.5).tan().max(1e-4)
}

fn pixel_to_ray(xy: [f64; 2], width: f64, height: f64, hfov: f64) -> Vec3 {
    let focal = focal_px(width, hfov);
    normalize([
        (xy[0] - width * 0.5) / focal,
        (height * 0.5 - xy[1]) / focal,
        1.0,
    ])
}

fn point_ids(observations: &[Observation]) -> Vec<u64> {
    let mut ids = Vec::new();
    for obs in observations {
        if !ids.contains(&obs.point_id) {
            ids.push(obs.point_id);
        }
    }
    ids
}

fn frame_ids(observations: &[Observation]) -> Vec<i32> {
    let mut frames = Vec::new();
    for obs in observations {
        if !frames.contains(&obs.frame) {
            frames.push(obs.frame);
        }
    }
    frames.sort_unstable();
    frames
}

fn obs_at<'a>(observations: &'a [Observation], id: u64, frame: i32) -> Option<&'a Observation> {
    observations
        .iter()
        .find(|obs| obs.point_id == id && obs.frame == frame)
}

fn color_for(id: u64) -> [f32; 3] {
    let hue = (id * 47 % 255) as f32 / 255.0;
    [0.2 + hue, 0.85 - hue * 0.4, 0.3 + (1.0 - hue) * 0.5]
}

fn solve_tripod(
    observations: &[Observation],
    ids: &[u64],
    frames: &[i32],
    width: f64,
    height: f64,
    shot: ShotType,
    hfov: Option<f64>,
) -> (Vec<ScenePoint>, Vec<(i32, Pose)>) {
    let base_fov = choose_fov(shot, hfov, 50.0);
    let frame0 = frames[0];
    let mut poses = Vec::new();
    let mut points = Vec::new();
    let rays0: Vec<(u64, Vec3)> = ids
        .iter()
        .filter_map(|id| {
            let obs = obs_at(observations, *id, frame0)?;
            Some((*id, pixel_to_ray(obs.xy, width, height, base_fov)))
        })
        .collect();
    for id in ids {
        if let Some((_, ray)) = rays0.iter().find(|ray| ray.0 == *id) {
            points.push(ScenePoint {
                id: *id,
                color: color_for(*id),
                position: scale(*ray, 4.0),
                observations: observations
                    .iter()
                    .filter(|obs| obs.point_id == *id)
                    .map(|obs| (obs.frame, obs.xy))
                    .collect(),
            });
        }
    }
    for (index, frame) in frames.iter().enumerate() {
        let fov = if shot == ShotType::VariableZoom && index > 0 {
            search_zoom_fov(&rays0, observations, *frame, width, height)
        } else {
            fov_for(shot, hfov, base_fov, index)
        };
        let mut src = Vec::new();
        let mut dst = Vec::new();
        for (id, ray0) in &rays0 {
            if let Some(obs) = obs_at(observations, *id, *frame) {
                src.push(*ray0);
                dst.push(pixel_to_ray(obs.xy, width, height, fov));
            }
        }
        let rotation = if src.len() >= 2 {
            rotation_between(&src, &dst)
        } else {
            identity()
        };
        poses.push((
            *frame,
            Pose {
                position: [0.0, 0.0, 0.0],
                rotation,
                hfov: fov,
            },
        ));
    }
    (points, poses)
}

fn solve_planar(
    observations: &[Observation],
    ids: &[u64],
    frames: &[i32],
    width: f64,
    height: f64,
    shot: ShotType,
    hfov: Option<f64>,
) -> (Vec<ScenePoint>, Vec<(i32, Pose)>) {
    let (mut points, mut poses) =
        solve_tripod(observations, ids, frames, width, height, shot, hfov);
    let depth = 6.0;
    for point in &mut points {
        let z = point.position[2].max(0.2);
        point.position = scale(point.position, depth / z);
    }
    for pose in &mut poses {
        *pose = (
            pose.0,
            refine_pose(pose.1, &points, observations, pose.0, width, height, true),
        );
    }
    (points, poses)
}

fn solve_general(
    observations: &[Observation],
    ids: &[u64],
    frames: &[i32],
    width: f64,
    height: f64,
    shot: ShotType,
    hfov: Option<f64>,
) -> (Vec<ScenePoint>, Vec<(i32, Pose)>) {
    let (mut points, mut poses) =
        solve_tripod(observations, ids, frames, width, height, shot, hfov);
    let depth = 6.0;
    let focal = focal_px(width, poses.first().map(|pose| pose.1.hfov).unwrap_or(50.0));
    for pose in &mut poses {
        let flow = median_flow(observations, frames[0], pose.0);
        pose.1.position = [-flow[0] / focal * depth, flow[1] / focal * depth, 0.0];
    }
    if frames.len() >= 2 {
        let mut pass = 0;
        let mut passes = 4;
        while pass < passes {
            let pose_a = poses[0].1;
            let mid = poses.len() / 2;
            let pose_b = poses[mid].1;
            let frame_b = poses[mid].0;
            for point in &mut points {
                let Some(a) = obs_at(observations, point.id, frames[0]) else {
                    continue;
                };
                let Some(b) = obs_at(observations, point.id, frame_b) else {
                    continue;
                };
                let ray_a = world_ray(&pose_a, pixel_to_ray(a.xy, width, height, pose_a.hfov));
                let ray_b = world_ray(&pose_b, pixel_to_ray(b.xy, width, height, pose_b.hfov));
                let hit = closest_point_on_rays(pose_a.position, ray_a, pose_b.position, ray_b);
                if hit[2] > 0.3 && hit[2] < 40.0 {
                    point.position = hit;
                }
            }
            if pass == 0 {
                let mean = points.iter().map(|point| point.position[2]).sum::<f64>()
                    / points.len().max(1) as f64;
                let variance = points
                    .iter()
                    .map(|point| (point.position[2] - mean).powi(2))
                    .sum::<f64>()
                    / points.len().max(1) as f64;
                // ほぼ平面だと自由な奥行きは不安定なので、反復でノイズに合わせない。
                if variance < 0.35 {
                    passes = 1;
                }
            }
            if pass + 1 < passes {
                for pose in &mut poses {
                    *pose = (
                        pose.0,
                        refine_pose(pose.1, &points, observations, pose.0, width, height, true),
                    );
                }
            }
            pass += 1;
        }
    }
    (points, poses)
}

fn search_zoom_fov(
    rays0: &[(u64, Vec3)],
    observations: &[Observation],
    frame: i32,
    width: f64,
    height: f64,
) -> f64 {
    let mut best_fov = 50.0;
    let mut best_error = f64::MAX;
    let mut fov = 24.0;
    while fov <= 90.0 {
        let mut src = Vec::new();
        let mut dst = Vec::new();
        for (id, ray0) in rays0 {
            if let Some(obs) = obs_at(observations, *id, frame) {
                src.push(*ray0);
                dst.push(pixel_to_ray(obs.xy, width, height, fov));
            }
        }
        if src.len() < 2 {
            break;
        }
        let rotation = rotation_between(&src, &dst);
        let mut error = 0.0;
        for (from, to) in src.iter().zip(dst.iter()) {
            let mapped = mul_mat_vec(rotation, *from);
            let delta = sub(mapped, *to);
            error += delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2];
        }
        if error < best_error {
            best_error = error;
            best_fov = fov;
        }
        fov += 1.0;
    }
    best_fov
}

fn median_flow(observations: &[Observation], frame_a: i32, frame_b: i32) -> [f64; 2] {
    let mut flows = Vec::new();
    for obs in observations.iter().filter(|obs| obs.frame == frame_a) {
        if let Some(other) = obs_at(observations, obs.point_id, frame_b) {
            flows.push([other.xy[0] - obs.xy[0], other.xy[1] - obs.xy[1]]);
        }
    }
    if flows.is_empty() {
        return [0.0, 0.0];
    }
    flows.sort_by(|a, b| a[0].partial_cmp(&b[0]).unwrap_or(Ordering::Equal));
    let mid = flows[flows.len() / 2];
    mid
}

fn world_ray(pose: &Pose, camera_ray: Vec3) -> Vec3 {
    normalize(mul_mat_vec(transpose(pose.rotation), camera_ray))
}

fn choose_fov(shot: ShotType, hfov: Option<f64>, fallback: f64) -> f64 {
    match shot {
        ShotType::SpecifiedHorizontalFov => hfov.unwrap_or(fallback),
        ShotType::FixedHorizontalFov | ShotType::VariableZoom => hfov.unwrap_or(fallback),
    }
}

fn fov_for(shot: ShotType, hfov: Option<f64>, base: f64, frame_index: usize) -> f64 {
    match shot {
        ShotType::SpecifiedHorizontalFov => hfov.unwrap_or(base),
        ShotType::FixedHorizontalFov => base,
        ShotType::VariableZoom => {
            let guess = base + frame_index as f64 * 4.0;
            hfov.unwrap_or(guess)
        }
    }
}

fn refine_pose(
    mut pose: Pose,
    points: &[ScenePoint],
    observations: &[Observation],
    frame: i32,
    width: f64,
    height: f64,
    allow_translation: bool,
) -> Pose {
    let obs: Vec<&Observation> = observations
        .iter()
        .filter(|obs| obs.frame == frame)
        .collect();
    if obs.len() < 3 {
        return pose;
    }
    let mut best_error = pose_error(&pose, points, &obs, width, height);
    let steps = [0.25, -0.25, 0.08, -0.08, 0.02, -0.02, 0.5, -0.5];
    for _ in 0..8 {
        let mut improved = false;
        for param in 0..6 {
            if !allow_translation && param >= 3 {
                continue;
            }
            for step in steps {
                let mut trial = pose;
                nudge(&mut trial, param, step);
                let error = pose_error(&trial, points, &obs, width, height);
                if error + 1e-6 < best_error {
                    best_error = error;
                    pose = trial;
                    improved = true;
                }
            }
        }
        if !improved {
            break;
        }
    }
    pose
}

fn nudge(pose: &mut Pose, param: usize, amount: f64) {
    match param {
        0 => pose.rotation = mat_mul(rotation_ypr(amount, 0.0, 0.0), pose.rotation),
        1 => pose.rotation = mat_mul(rotation_ypr(0.0, amount, 0.0), pose.rotation),
        2 => pose.rotation = mat_mul(rotation_ypr(0.0, 0.0, amount), pose.rotation),
        3 => pose.position[0] += amount,
        4 => pose.position[1] += amount,
        _ => pose.position[2] += amount,
    }
}

fn pose_error(
    pose: &Pose,
    points: &[ScenePoint],
    obs: &[&Observation],
    width: f64,
    height: f64,
) -> f64 {
    let mut sum = 0.0;
    let mut count = 0.0;
    for sample in obs {
        let Some(point) = points.iter().find(|point| point.id == sample.point_id) else {
            continue;
        };
        let Some(projected) = project(point.position, pose, width, height) else {
            sum += 100.0;
            count += 1.0;
            continue;
        };
        let dx = projected[0] - sample.xy[0];
        let dy = projected[1] - sample.xy[1];
        sum += (dx * dx + dy * dy).sqrt();
        count += 1.0;
    }
    if count == 0.0 {
        100.0
    } else {
        sum / count
    }
}

fn mean_error(
    observations: &[Observation],
    points: &[ScenePoint],
    poses: &[(i32, Pose)],
    width: f64,
    height: f64,
) -> f64 {
    let mut sum = 0.0;
    let mut count = 0.0;
    for obs in observations {
        let Some(point) = points.iter().find(|point| point.id == obs.point_id) else {
            continue;
        };
        let Some(pose) = poses.iter().find(|pose| pose.0 == obs.frame) else {
            continue;
        };
        let Some(projected) = project(point.position, &pose.1, width, height) else {
            continue;
        };
        let dx = projected[0] - obs.xy[0];
        let dy = projected[1] - obs.xy[1];
        sum += (dx * dx + dy * dy).sqrt();
        count += 1.0;
    }
    if count == 0.0 {
        0.0
    } else {
        sum / count
    }
}

fn channel_peak(pixel: [f32; 4]) -> f32 {
    pixel[0].max(pixel[1]).max(pixel[2])
}

fn detect_peaks(image: &Raster, threshold: f32) -> Vec<(i32, i32, [f32; 3])> {
    let mut found = Vec::new();
    for y in 1..image.height as i32 - 1 {
        for x in 1..image.width as i32 - 1 {
            let pixel = image.get(x, y);
            let value = channel_peak(pixel);
            if value < threshold {
                continue;
            }
            let peak = channel_peak(image.get(x - 1, y)) <= value
                && channel_peak(image.get(x + 1, y)) <= value
                && channel_peak(image.get(x, y - 1)) <= value
                && channel_peak(image.get(x, y + 1)) <= value;
            if !peak {
                continue;
            }
            if found
                .iter()
                .any(|dot: &(i32, i32, [f32; 3])| (dot.0 - x).abs() < 5 && (dot.1 - y).abs() < 5)
            {
                continue;
            }
            found.push((x, y, [pixel[0], pixel[1], pixel[2]]));
        }
    }
    found
}

fn track_color(image: &Raster, x: i32, y: i32, color: [f32; 3]) -> Option<[f64; 2]> {
    let mut best = None;
    let mut best_score = f32::MAX;
    for oy in -6..=6 {
        for ox in -6..=6 {
            let pixel = image.get(x + ox, y + oy);
            let score = (pixel[0] - color[0]).abs()
                + (pixel[1] - color[1]).abs()
                + (pixel[2] - color[2]).abs();
            if score < best_score {
                best_score = score;
                best = Some([f64::from(x + ox), f64::from(y + oy)]);
            }
        }
    }
    best
}

impl CameraTracker {
    pub fn attach_layer(&self, layer: &mut Layer, ids: &mut Ids) {
        let root = ensure_group(layer, ids, "3Dカメラトラッカー", "camera-tracker");
        if let Some(error) = self.average_error {
            root.push_child(persist::scalar(ids, "平均誤差", "average-error", error));
        }
        if let Some(method) = self.method_used {
            let name = match method {
                ConcreteMethod::General => "一般",
                ConcreteMethod::MostlyPlanar => "ほぼ平面",
                ConcreteMethod::TripodPan => "三脚パン",
            };
            root.push_child(persist::text(
                ids,
                "使用された解決方法",
                "method-used",
                name,
            ));
        }
    }

    pub fn create_solved_camera(
        &self,
        comp: &mut model::Composition,
        ids: &mut Ids,
    ) -> Result<LayerId, ToolError> {
        if !self.solved {
            return Err(ToolError::new("解が完了していないのでカメラを作れない"));
        }
        let id = ids.layer();
        let mut layer = Layer::new(
            id,
            "カメラ",
            LayerKind::Camera {
                nodes: CameraNodes::One,
            },
        );
        layer.three_d = true;
        let pose = self
            .poses
            .first()
            .map(|pose| pose.1)
            .unwrap_or(Pose::identity(50.0));
        layer.properties.push(persist::point(
            ids,
            "位置",
            "position",
            [pose.position[0], pose.position[1]],
        ));
        layer.properties.push(persist::scalar(
            ids,
            "水平視野角",
            "horizontal-fov",
            pose.hfov,
        ));
        comp.layers.push(layer);
        Ok(id)
    }

    pub fn create_text(
        &self,
        comp: &mut model::Composition,
        ids: &mut Ids,
        points: &[u64],
    ) -> Result<Vec<LayerId>, ToolError> {
        let camera = self.create_solved_camera(comp, ids)?;
        let mut ids_out = vec![camera];
        for (index, point_id) in points.iter().enumerate() {
            let point = self
                .points
                .iter()
                .find(|point| point.id == *point_id)
                .ok_or_else(|| ToolError::new("点がない"))?;
            let id = ids.layer();
            let mut text = Text::point(format!("テキスト {}", index + 1));
            if let Some(size) = self.target_size {
                text.style.size = size;
            }
            let mut layer = Layer::new(id, format!("テキスト {}", index + 1), LayerKind::Text);
            layer.three_d = true;
            layer.text = Some(text);
            let center = self.target.unwrap_or(point.position);
            layer.properties.push(model::Property::value(
                ids.property(),
                "位置",
                "position",
                PropertyValue::ThreeD(center),
            ));
            let frames: Vec<i32> = point.observations.iter().map(|obs| obs.0).collect();
            if let (Some(first), Some(last)) = (frames.iter().min(), frames.iter().max()) {
                layer.properties.push(persist::scalar(
                    ids,
                    "イン点",
                    "in-frame",
                    f64::from(*first),
                ));
                layer.properties.push(persist::scalar(
                    ids,
                    "アウト点",
                    "out-frame",
                    f64::from(*last),
                ));
            }
            comp.layers.insert(0, layer);
            ids_out.push(id);
        }
        Ok(ids_out)
    }

    pub fn create_solid(
        &self,
        project: &mut Project,
        comp_id: ItemId,
        ids: &mut Ids,
    ) -> Result<LayerId, ToolError> {
        let _camera = {
            let comp = project
                .composition_mut(comp_id)
                .ok_or_else(|| ToolError::new("コンポジションがない"))?;
            self.create_solved_camera(comp, ids)?
        };
        let size = self.target_size.unwrap_or(100.0).round().max(1.0) as u32;
        let item = ids.item();
        project.add_footage(Footage {
            id: item,
            name: "平面".into(),
            width: size,
            height: size,
            duration: Time::from_seconds(1.0),
            frame_rate: 24.0,
            kind: FootageKind::Solid {
                color: Rgba::white(),
            },
        });
        let comp = project.composition_mut(comp_id).expect("comp");
        let id = ids.layer();
        let mut layer = Layer::new(id, "平面 1", LayerKind::Solid { source: item });
        layer.three_d = true;
        let center = self.target.unwrap_or([0.0, 0.0, 0.0]);
        layer.properties.push(model::Property::value(
            ids.property(),
            "位置",
            "position",
            PropertyValue::ThreeD(center),
        ));
        comp.layers.insert(0, layer);
        Ok(id)
    }

    pub fn create_null(
        &self,
        comp: &mut model::Composition,
        ids: &mut Ids,
        points: &[u64],
    ) -> Result<Vec<LayerId>, ToolError> {
        let camera = self.create_solved_camera(comp, ids)?;
        let mut out = vec![camera];
        for (index, point_id) in points.iter().enumerate() {
            let point = self
                .points
                .iter()
                .find(|point| point.id == *point_id)
                .ok_or_else(|| ToolError::new("点がない"))?;
            let id = ids.layer();
            let mut layer = Layer::new(id, format!("ヌル {}", index + 1), LayerKind::Null);
            layer.three_d = true;
            let center = if points.len() == 1 {
                self.target.unwrap_or(point.position)
            } else {
                point.position
            };
            layer.properties.push(model::Property::value(
                ids.property(),
                "位置",
                "position",
                PropertyValue::ThreeD(center),
            ));
            comp.layers.insert(0, layer);
            out.push(id);
        }
        Ok(out)
    }

    pub fn create_shadow_catcher(
        &self,
        project: &mut Project,
        comp_id: ItemId,
        ids: &mut Ids,
    ) -> Result<(), ToolError> {
        let item = ids.item();
        project.add_footage(Footage {
            id: item,
            name: "シャドウキャッチャー".into(),
            width: self.image_width,
            height: self.image_height,
            duration: Time::from_seconds(1.0),
            frame_rate: 24.0,
            kind: FootageKind::Solid {
                color: Rgba::white(),
            },
        });
        let comp = project
            .composition_mut(comp_id)
            .ok_or_else(|| ToolError::new("コンポジションがない"))?;
        let id = ids.layer();
        let mut layer = Layer::new(
            id,
            "シャドウキャッチャー",
            LayerKind::Solid { source: item },
        );
        layer.three_d = true;
        layer.properties.push(persist::boolean(
            ids,
            "影だけ受ける",
            "accepts-shadows-only",
            true,
        ));
        comp.layers.insert(0, layer);
        let has_light = comp
            .layers
            .iter()
            .any(|layer| matches!(layer.kind, LayerKind::Light { .. }));
        if !has_light {
            let light = ids.layer();
            let mut layer = Layer::new(
                light,
                "ライト",
                LayerKind::Light {
                    kind: LightKind::Point,
                },
            );
            layer.three_d = true;
            comp.layers.insert(0, layer);
        }
        Ok(())
    }
}

pub fn observe(
    points: &[Vec3],
    poses: &[(i32, Pose)],
    width: f64,
    height: f64,
) -> Vec<Observation> {
    let mut observations = Vec::new();
    for (index, point) in points.iter().enumerate() {
        for (frame, pose) in poses {
            if let Some(xy) = project(*point, pose, width, height) {
                observations.push(Observation {
                    point_id: index as u64 + 1,
                    frame: *frame,
                    xy,
                });
            }
        }
    }
    observations
}

#[cfg(test)]
mod tests {
    use model::{BitsPerChannel, Composition, LayerKind, Project, PropertyValue, Time};

    use super::*;

    fn near(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() <= eps
    }

    fn comp(project: &mut Project) -> ItemId {
        let id = project.alloc_item();
        project.add_composition(Composition::new(
            id,
            "comp",
            128,
            72,
            24.0,
            Time::from_seconds(1.0),
        ));
        id
    }

    fn tripod_scene() -> (CameraTracker, Vec<Observation>) {
        let width = 160.0;
        let height = 90.0;
        let points = vec![
            [-1.0, -0.6, 5.0],
            [1.2, -0.4, 5.5],
            [0.2, 0.8, 4.5],
            [-1.4, 0.5, 6.0],
            [0.8, 0.2, 5.2],
            [-0.3, -0.9, 4.8],
        ];
        let mut poses = Vec::new();
        for frame in 0..4 {
            let mut pose = Pose::identity(48.0);
            pose.rotation = rotation_ypr(f64::from(frame) * 0.04, 0.0, f64::from(frame) * 0.02);
            poses.push((frame, pose));
        }
        let observations = observe(&points, &poses, width, height);
        let mut tracker = CameraTracker::new(160, 90);
        tracker.shot = Some(ShotType::FixedHorizontalFov);
        tracker.method = Some(SolveMethod::TripodPan);
        tracker.set_observations(observations.clone());
        (tracker, observations)
    }

    #[test]
    fn analyze_shows_colored_points() {
        let mut frame = Raster::new(48, 32, [0.02, 0.02, 0.02, 1.0]);
        frame.set(12, 10, [0.9, 0.1, 0.1, 1.0]);
        frame.set(30, 18, [0.1, 0.8, 0.2, 1.0]);
        frame.set(20, 24, [0.2, 0.2, 0.95, 1.0]);
        let mut tracker = CameraTracker::new(48, 32);
        tracker.shot = Some(ShotType::FixedHorizontalFov);
        tracker.method = Some(SolveMethod::TripodPan);
        tracker.analyze_frames(&[frame]).unwrap();
        assert!(tracker.solved);
        assert!(tracker.points.len() >= 3);
        assert!(tracker.points.iter().any(|point| point.color[0] > 0.7));
        assert!(tracker.points.iter().any(|point| point.color[2] > 0.7));
    }

    #[test]
    fn cancel_blocks_camera_creation() {
        let (mut tracker, _) = tripod_scene();
        tracker.running = true;
        tracker.cancel();
        let mut project = Project::new(BitsPerChannel::Bpc8);
        let comp_id = comp(&mut project);
        let mut ids = model::Ids::new();
        let composition = project.composition_mut(comp_id).unwrap();
        assert!(tracker.create_solved_camera(composition, &mut ids).is_err());
        assert!(!tracker.running);
    }

    #[test]
    fn fixed_hfov_disables_input_and_resolves() {
        let (mut tracker, _) = tripod_scene();
        tracker.set_shot(ShotType::FixedHorizontalFov).unwrap();
        assert!(!tracker.hfov_editable());
        assert!(tracker.set_hfov(35.0).is_err());
        assert!(tracker.solved);
        assert!(!tracker.needs_resolve);
    }

    #[test]
    fn variable_zoom_beats_fixed_fov() {
        let width = 160.0;
        let height = 90.0;
        let points = vec![
            [-1.0, -0.4, 4.0],
            [1.0, -0.2, 8.0],
            [0.0, 0.6, 5.0],
            [-0.6, 0.3, 7.0],
            [0.7, -0.5, 6.0],
        ];
        let mut observations = Vec::new();
        for frame in 0..3 {
            let mut pose = Pose::identity(40.0 + f64::from(frame) * 12.0);
            pose.rotation = identity();
            for (index, point) in points.iter().enumerate() {
                observations.push(Observation {
                    point_id: index as u64 + 1,
                    frame,
                    xy: project(*point, &pose, width, height).unwrap(),
                });
            }
        }
        let mut variable = CameraTracker::new(160, 90);
        variable.shot = Some(ShotType::VariableZoom);
        variable.method = Some(SolveMethod::TripodPan);
        variable.hfov = Some(40.0);
        variable.set_observations(observations.clone());
        variable.solve().unwrap();
        let mut fixed = CameraTracker::new(160, 90);
        fixed.shot = Some(ShotType::FixedHorizontalFov);
        fixed.method = Some(SolveMethod::TripodPan);
        fixed.set_observations(observations);
        fixed.solve().unwrap();
        assert!(variable.average_error.unwrap() < fixed.average_error.unwrap());
    }

    #[test]
    fn only_specified_hfov_is_editable() {
        let (mut tracker, _) = tripod_scene();
        tracker.set_shot(ShotType::SpecifiedHorizontalFov).unwrap();
        assert!(tracker.hfov_editable());
        tracker.set_shot(ShotType::VariableZoom).unwrap();
        assert!(!tracker.hfov_editable());
    }

    #[test]
    fn changing_hfov_resolves_into_the_camera() {
        let (mut tracker, _) = tripod_scene();
        tracker.set_shot(ShotType::SpecifiedHorizontalFov).unwrap();
        tracker.set_hfov(42.0).unwrap();
        assert!(near(tracker.poses[0].1.hfov, 42.0, 1e-6));
        let mut project = Project::new(BitsPerChannel::Bpc8);
        let comp_id = comp(&mut project);
        let mut ids = model::Ids::new();
        let composition = project.composition_mut(comp_id).unwrap();
        tracker.create_solved_camera(composition, &mut ids).unwrap();
        let camera = composition
            .layers
            .iter()
            .find(|layer| matches!(layer.kind, LayerKind::Camera { .. }))
            .unwrap();
        match camera
            .property("horizontal-fov")
            .and_then(|p| p.base_value())
        {
            Some(PropertyValue::Scalar(value)) => assert!(near(*value, 42.0, 1e-6)),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn display_switches_to_solved_points() {
        let (mut tracker, _) = tripod_scene();
        tracker.solve().unwrap();
        tracker.display = Some(PointDisplay::Solved3d);
        let view = tracker.view_points(0);
        assert!(!view.is_empty());
        let projected =
            project(tracker.points[0].position, &tracker.poses[0].1, 160.0, 90.0).unwrap();
        assert!(near(view[0][0], projected[0], 0.2));
    }

    #[test]
    fn display_switches_to_source_points() {
        let (mut tracker, observations) = tripod_scene();
        tracker.solve().unwrap();
        tracker.display = Some(PointDisplay::Source2d);
        let view = tracker.view_points(0);
        let source = observations
            .iter()
            .find(|obs| obs.frame == 0 && obs.point_id == tracker.points[0].id)
            .unwrap();
        assert!(near(view[0][0], source.xy[0], 1e-6));
        assert!(near(view[0][1], source.xy[1], 1e-6));
    }

    #[test]
    fn render_flag_keeps_points_when_deselected() {
        let mut tracker = CameraTracker::new(32, 32);
        tracker.render_points = Some(true);
        assert!(tracker.points_visible(false));
        tracker.render_points = Some(false);
        assert!(tracker.points_visible(true));
        assert!(!tracker.points_visible(false));
    }

    #[test]
    fn point_size_does_not_move_the_solve() {
        let (mut tracker, _) = tripod_scene();
        tracker.solve().unwrap();
        let before = tracker.points[0].position;
        tracker.point_size = Some(8.0);
        assert!(near(tracker.display_radius().unwrap(), 8.0, 1e-9));
        tracker.point_size = Some(20.0);
        assert_eq!(tracker.points[0].position, before);
    }

    #[test]
    fn create_camera_adds_a_camera() {
        let (mut tracker, _) = tripod_scene();
        tracker.solve().unwrap();
        let mut project = Project::new(BitsPerChannel::Bpc8);
        let comp_id = comp(&mut project);
        let mut ids = model::Ids::new();
        let composition = project.composition_mut(comp_id).unwrap();
        tracker.create_solved_camera(composition, &mut ids).unwrap();
        assert!(composition
            .layers
            .iter()
            .any(|layer| matches!(layer.kind, LayerKind::Camera { .. })));
    }

    #[test]
    fn text_and_camera_share_the_target() {
        let (mut tracker, _) = tripod_scene();
        tracker.solve().unwrap();
        tracker.target_size = Some(48.0);
        let mut project = Project::new(BitsPerChannel::Bpc8);
        let comp_id = comp(&mut project);
        let mut ids = model::Ids::new();
        let composition = project.composition_mut(comp_id).unwrap();
        let ids_out = tracker.create_text(composition, &mut ids, &[1]).unwrap();
        assert!(ids_out.len() >= 2);
        assert!(composition
            .layers
            .iter()
            .any(|layer| matches!(layer.kind, LayerKind::Camera { .. })));
        let text = composition
            .layers
            .iter()
            .find(|layer| matches!(layer.kind, LayerKind::Text))
            .unwrap();
        assert!(near(text.text.as_ref().unwrap().style.size, 48.0, 1e-6));
        assert!(text.three_d);
    }

    #[test]
    fn solid_and_camera_use_target_size() {
        let (mut tracker, _) = tripod_scene();
        tracker.solve().unwrap();
        tracker.target_size = Some(30.0);
        let mut project = Project::new(BitsPerChannel::Bpc8);
        let comp_id = comp(&mut project);
        let mut ids = model::Ids::new();
        tracker
            .create_solid(&mut project, comp_id, &mut ids)
            .unwrap();
        let footage = project
            .footage
            .iter()
            .find(|item| item.name == "平面")
            .unwrap();
        assert_eq!(footage.width, 30);
        assert!(project
            .composition(comp_id)
            .unwrap()
            .layers
            .iter()
            .any(|layer| matches!(layer.kind, LayerKind::Camera { .. })));
    }

    #[test]
    fn null_sits_on_the_target_center() {
        let (mut tracker, _) = tripod_scene();
        tracker.solve().unwrap();
        let center = tracker.points[0].position;
        tracker.target = Some(center);
        let mut project = Project::new(BitsPerChannel::Bpc8);
        let comp_id = comp(&mut project);
        let mut ids = model::Ids::new();
        let composition = project.composition_mut(comp_id).unwrap();
        tracker
            .create_null(composition, &mut ids, &[tracker.points[0].id])
            .unwrap();
        let null_layer = composition
            .layers
            .iter()
            .find(|layer| matches!(layer.kind, LayerKind::Null))
            .unwrap();
        match null_layer.property("position").unwrap().base_value() {
            Some(PropertyValue::ThreeD(value)) => {
                assert!(near(value[0], center[0], 1e-6));
                assert!(near(value[2], center[2], 1e-6));
            }
            other => panic!("{other:?}"),
        }
        assert!(composition
            .layers
            .iter()
            .any(|layer| matches!(layer.kind, LayerKind::Camera { .. })));
    }

    #[test]
    fn one_layer_per_selected_point() {
        let (mut tracker, _) = tripod_scene();
        tracker.solve().unwrap();
        let ids_sel: Vec<u64> = tracker
            .points
            .iter()
            .take(3)
            .map(|point| point.id)
            .collect();
        let mut project = Project::new(BitsPerChannel::Bpc8);
        let comp_id = comp(&mut project);
        let mut ids = model::Ids::new();
        let composition = project.composition_mut(comp_id).unwrap();
        tracker
            .create_text(composition, &mut ids, &ids_sel)
            .unwrap();
        let texts = composition
            .layers
            .iter()
            .filter(|layer| matches!(layer.kind, LayerKind::Text))
            .count();
        assert_eq!(texts, 3);
        assert!(composition
            .layers
            .iter()
            .any(|layer| layer.name == "テキスト 1"));
        assert!(composition
            .layers
            .iter()
            .any(|layer| layer.name == "テキスト 3"));
    }

    #[test]
    fn shadow_catcher_adds_light_when_missing() {
        let tracker = CameraTracker::new(64, 48);
        let mut project = Project::new(BitsPerChannel::Bpc8);
        let comp_id = comp(&mut project);
        let mut ids = model::Ids::new();
        tracker
            .create_shadow_catcher(&mut project, comp_id, &mut ids)
            .unwrap();
        let footage = project
            .footage
            .iter()
            .find(|item| item.name == "シャドウキャッチャー")
            .unwrap();
        assert_eq!((footage.width, footage.height), (64, 48));
        match footage.kind {
            FootageKind::Solid { color } => assert_eq!(color, Rgba::white()),
            FootageKind::File | FootageKind::Placeholder => panic!("solid"),
        }
        let comp = project.composition(comp_id).unwrap();
        let catcher = comp
            .layers
            .iter()
            .find(|layer| layer.name == "シャドウキャッチャー")
            .unwrap();
        assert_eq!(
            catcher
                .property("accepts-shadows-only")
                .and_then(|p| p.base_value()),
            Some(&PropertyValue::Boolean(true))
        );
        assert!(comp
            .layers
            .iter()
            .any(|layer| matches!(layer.kind, LayerKind::Light { .. })));
    }

    #[test]
    fn ground_does_not_move_existing_layers() {
        let (mut tracker, _) = tripod_scene();
        tracker.solve().unwrap();
        tracker.target = Some(tracker.points[0].position);
        let mut project = Project::new(BitsPerChannel::Bpc8);
        let comp_id = comp(&mut project);
        let mut ids = model::Ids::new();
        let composition = project.composition_mut(comp_id).unwrap();
        tracker
            .create_null(composition, &mut ids, &[tracker.points[0].id])
            .unwrap();
        let before = match composition
            .layers
            .iter()
            .find(|layer| matches!(layer.kind, LayerKind::Null))
            .unwrap()
            .property("position")
            .unwrap()
            .base_value()
        {
            Some(PropertyValue::ThreeD(value)) => *value,
            _ => panic!("position"),
        };
        tracker.set_ground([3.0, 0.0, 5.0], [0.0, 1.0, 0.0]);
        let after = match composition
            .layers
            .iter()
            .find(|layer| matches!(layer.kind, LayerKind::Null))
            .unwrap()
            .property("position")
            .unwrap()
            .base_value()
        {
            Some(PropertyValue::ThreeD(value)) => *value,
            _ => panic!("position"),
        };
        assert_eq!(before, after);
        tracker
            .create_null(composition, &mut ids, &[tracker.points[1].id])
            .unwrap();
        let created = composition
            .layers
            .iter()
            .filter(|layer| matches!(layer.kind, LayerKind::Null))
            .count();
        assert!(created >= 2);
        let newest = composition.layers.iter().find(|layer| layer.name == "ヌル 1" && matches!(layer.property("position").and_then(|p| p.base_value()), Some(PropertyValue::ThreeD(value)) if near(value[0], 3.0, 1e-6)));
        assert!(newest.is_some());
    }

    #[test]
    fn moved_target_is_used_by_new_layers() {
        let (mut tracker, _) = tripod_scene();
        tracker.solve().unwrap();
        tracker.set_ground([0.0, 0.0, 5.0], [0.0, 0.0, 1.0]);
        tracker.move_target([1.5, -0.4, 9.0]);
        assert!(near(tracker.target.unwrap()[2], 5.0, 1e-6));
        let mut project = Project::new(BitsPerChannel::Bpc8);
        let comp_id = comp(&mut project);
        let mut ids = model::Ids::new();
        let composition = project.composition_mut(comp_id).unwrap();
        tracker.create_null(composition, &mut ids, &[1]).unwrap();
        match composition
            .layers
            .iter()
            .find(|layer| matches!(layer.kind, LayerKind::Null))
            .unwrap()
            .property("position")
            .unwrap()
            .base_value()
        {
            Some(PropertyValue::ThreeD(value)) => {
                assert!(near(value[0], tracker.target.unwrap()[0], 1e-6))
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn target_size_sets_text_and_solid() {
        let (mut tracker, _) = tripod_scene();
        tracker.solve().unwrap();
        tracker.target_size = Some(36.0);
        let mut project = Project::new(BitsPerChannel::Bpc8);
        let comp_id = comp(&mut project);
        let mut ids = model::Ids::new();
        {
            let composition = project.composition_mut(comp_id).unwrap();
            tracker.create_text(composition, &mut ids, &[1]).unwrap();
        }
        tracker
            .create_solid(&mut project, comp_id, &mut ids)
            .unwrap();
        let composition = project.composition(comp_id).unwrap();
        let text = composition
            .layers
            .iter()
            .find(|layer| matches!(layer.kind, LayerKind::Text))
            .unwrap();
        assert!(near(text.text.as_ref().unwrap().style.size, 36.0, 1e-6));
        assert_eq!(
            project
                .footage
                .iter()
                .find(|item| item.name == "平面")
                .unwrap()
                .width,
            36
        );
    }

    #[test]
    fn deleting_a_point_updates_error() {
        let (mut tracker, mut observations) = tripod_scene();
        observations.push(Observation {
            point_id: 99,
            frame: 0,
            xy: [5.0, 5.0],
        });
        observations.push(Observation {
            point_id: 99,
            frame: 1,
            xy: [150.0, 80.0],
        });
        tracker.set_observations(observations);
        tracker.solve().unwrap();
        let before = tracker.average_error.unwrap();
        tracker.delete_points(&[99], None).unwrap();
        assert!(tracker.points.iter().all(|point| point.id != 99));
        assert!(tracker.average_error.unwrap() < before);
    }

    #[test]
    fn auto_reports_a_concrete_method() {
        let (mut tracker, _) = tripod_scene();
        tracker.method = Some(SolveMethod::Auto);
        tracker.solve().unwrap();
        assert!(matches!(
            tracker.method_used,
            Some(
                ConcreteMethod::TripodPan | ConcreteMethod::MostlyPlanar | ConcreteMethod::General
            )
        ));
    }

    #[test]
    fn general_beats_tripod_when_the_camera_translates() {
        let width = 160.0;
        let height = 90.0;
        let points = vec![
            [-1.0, -0.5, 4.0],
            [1.0, -0.4, 8.0],
            [0.2, 0.7, 5.0],
            [-0.8, 0.4, 9.0],
            [0.6, 0.1, 6.0],
            [-0.2, -0.7, 7.0],
        ];
        let mut poses = Vec::new();
        for frame in 0..3 {
            let mut pose = Pose::identity(50.0);
            pose.position = [f64::from(frame) * 0.8, 0.0, 0.0];
            poses.push((frame, pose));
        }
        let observations = observe(&points, &poses, width, height);
        let mut general = CameraTracker::new(160, 90);
        general.shot = Some(ShotType::FixedHorizontalFov);
        general.method = Some(SolveMethod::General);
        general.set_observations(observations.clone());
        general.solve().unwrap();
        let mut tripod = CameraTracker::new(160, 90);
        tripod.shot = Some(ShotType::FixedHorizontalFov);
        tripod.method = Some(SolveMethod::TripodPan);
        tripod.set_observations(observations);
        tripod.solve().unwrap();
        assert!(
            general.average_error.unwrap() < tripod.average_error.unwrap(),
            "general {} tripod {}",
            general.average_error.unwrap(),
            tripod.average_error.unwrap()
        );
    }

    #[test]
    fn planar_beats_general_on_a_wall() {
        let width = 160.0;
        let height = 90.0;
        let points: Vec<Vec3> = (-2..3)
            .flat_map(|x| (-1..2).map(move |y| [x as f64 * 0.6, y as f64 * 0.5, 6.0]))
            .collect();
        let mut poses = Vec::new();
        for frame in 0..4 {
            let mut pose = Pose::identity(46.0);
            pose.position = [f64::from(frame) * 0.15, 0.0, 0.0];
            poses.push((frame, pose));
        }
        let mut observations = observe(&points, &poses, width, height);
        for (index, obs) in observations.iter_mut().enumerate() {
            obs.xy[0] += ((index * 17) % 5) as f64 * 0.05 - 0.1;
        }
        let mut planar = CameraTracker::new(160, 90);
        planar.shot = Some(ShotType::FixedHorizontalFov);
        planar.method = Some(SolveMethod::MostlyPlanar);
        planar.set_observations(observations.clone());
        planar.solve().unwrap();
        let mut general = CameraTracker::new(160, 90);
        general.shot = Some(ShotType::FixedHorizontalFov);
        general.method = Some(SolveMethod::General);
        general.set_observations(observations);
        general.solve().unwrap();
        assert!(
            planar.average_error.unwrap() < general.average_error.unwrap(),
            "planar {} general {}",
            planar.average_error.unwrap(),
            general.average_error.unwrap()
        );
    }

    #[test]
    fn tripod_pan_has_no_translation() {
        let (mut tracker, _) = tripod_scene();
        tracker.solve().unwrap();
        for (_, pose) in &tracker.poses {
            assert!(near(pose.position[0], 0.0, 1e-6));
            assert!(near(pose.position[1], 0.0, 1e-6));
            assert!(near(pose.position[2], 0.0, 1e-6));
        }
        assert_eq!(tracker.method_used, Some(ConcreteMethod::TripodPan));
    }

    #[test]
    fn used_method_is_concrete_after_auto() {
        let (mut tracker, _) = tripod_scene();
        tracker.method = Some(SolveMethod::Auto);
        tracker.solve().unwrap();
        let mut project = Project::new(BitsPerChannel::Bpc8);
        let comp_id = comp(&mut project);
        let mut ids = model::Ids::new();
        let composition = project.composition_mut(comp_id).unwrap();
        let layer_id = ids.layer();
        composition
            .layers
            .push(model::Layer::new(layer_id, "footage", LayerKind::Null));
        let layer = composition.layer_mut(layer_id).unwrap();
        tracker.attach_layer(layer, &mut ids);
        let label = layer.property("method-used").and_then(|p| p.base_value());
        assert!(matches!(label, Some(PropertyValue::Text(value)) if !value.is_empty()));
    }

    #[test]
    fn average_error_drops_when_a_bad_point_is_removed() {
        let (mut tracker, mut observations) = tripod_scene();
        observations.push(Observation {
            point_id: 7,
            frame: 0,
            xy: [1.0, 1.0],
        });
        observations.push(Observation {
            point_id: 7,
            frame: 2,
            xy: [140.0, 70.0],
        });
        tracker.set_observations(observations);
        tracker.solve().unwrap();
        let before = tracker.average_error.unwrap();
        tracker.delete_points(&[7], None).unwrap();
        assert!(tracker.average_error.unwrap() < before);
    }

    #[test]
    fn detailed_analysis_keeps_more_points() {
        let mut frame = Raster::new(40, 30, [0.0, 0.0, 0.0, 1.0]);
        frame.set(8, 8, [0.95, 0.2, 0.2, 1.0]);
        frame.set(28, 18, [0.2, 0.9, 0.3, 1.0]);
        frame.set(16, 22, [0.35, 0.35, 0.35, 1.0]);
        let mut plain = CameraTracker::new(40, 30);
        plain.shot = Some(ShotType::FixedHorizontalFov);
        plain.method = Some(SolveMethod::TripodPan);
        plain.detailed = Some(false);
        plain.analyze_frames(&[frame.clone()]).unwrap();
        let mut detailed = CameraTracker::new(40, 30);
        detailed.shot = Some(ShotType::FixedHorizontalFov);
        detailed.method = Some(SolveMethod::TripodPan);
        detailed.detailed = Some(true);
        detailed.analyze_frames(&[frame]).unwrap();
        assert!(detailed.points.len() > plain.points.len());
        assert!(detailed.stored_bytes() > plain.stored_bytes());
    }

    #[test]
    fn auto_delete_removes_the_feature_on_other_frames() {
        let (mut tracker, _) = tripod_scene();
        tracker.auto_delete_across_time = Some(true);
        tracker.solve().unwrap();
        let id = tracker.points[0].id;
        tracker.delete_points(&[id], Some(0)).unwrap();
        assert!(tracker.points.iter().all(|point| point.id != id));
        assert!(tracker.observations.iter().all(|obs| obs.point_id != id));
    }

    #[test]
    fn hiding_the_warning_keeps_the_solve() {
        let (mut tracker, _) = tripod_scene();
        tracker.solve().unwrap();
        tracker.needs_resolve = true;
        let error = tracker.average_error;
        tracker.hide_warning = Some(true);
        assert!(!tracker.warning_visible());
        assert_eq!(tracker.average_error, error);
        assert!(tracker.solved);
    }
}
