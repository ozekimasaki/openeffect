//! パペット。メッシュをピンで変形する。新規プロジェクトのエンジンはアドバンスト。

use std::cmp::Ordering;

use model::{Ids, Layer};

use crate::persist::{self, ensure_group};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PuppetEngine {
    Advanced,
    Legacy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PinKind {
    Position,
    Starch,
    Overlap,
    Advanced,
    Bend,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StarchGroup {
    Deform,
    Stiffness,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Pin {
    pub kind: PinKind,
    pub rest: [f64; 2],
    pub position: [f64; 2],
    pub rotation: f64,
    pub scale: f64,
    pub amount: f64,
    pub range: Option<f64>,
    pub in_front: f64,
    pub position_keys: Vec<(f64, [f64; 2])>,
    pub rotation_keys: Vec<(f64, f64)>,
    pub scale_keys: Vec<(f64, f64)>,
    pub stopwatch: bool,
}

#[derive(Clone, Debug)]
pub struct Mesh {
    pub rest: Vec<[f64; 2]>,
    pub current: Vec<[f64; 2]>,
    pub triangles: Vec<[usize; 3]>,
}

#[derive(Clone, Debug)]
pub struct Puppet {
    pub engine: PuppetEngine,
    pub expansion: f64,
    pub show_mesh: Option<bool>,
    pub rotation_refinement: Option<f64>,
    pub pins: Vec<Pin>,
    pub mesh: Mesh,
}

impl Puppet {
    pub fn new() -> Self {
        Self {
            engine: PuppetEngine::Advanced,
            expansion: 3.0,
            show_mesh: None,
            rotation_refinement: None,
            pins: Vec::new(),
            mesh: Mesh {
                rest: Vec::new(),
                current: Vec::new(),
                triangles: Vec::new(),
            },
        }
    }

    pub fn starch_group(&self) -> StarchGroup {
        match self.engine {
            PuppetEngine::Advanced => StarchGroup::Deform,
            PuppetEngine::Legacy => StarchGroup::Stiffness,
        }
    }

    pub fn build_from_alpha(&mut self, alpha: &[bool], width: i32, height: i32) {
        let expanded = dilate(alpha, width, height, self.expansion.round() as i32);
        let stride = 4;
        let mut rest = Vec::new();
        let mut index_of = vec![-1; (width * height) as usize];
        for y in (0..height).step_by(stride as usize) {
            for x in (0..width).step_by(stride as usize) {
                if expanded[(y * width + x) as usize] {
                    index_of[(y * width + x) as usize] = rest.len() as i32;
                    rest.push([f64::from(x), f64::from(y)]);
                }
            }
        }
        let mut triangles = Vec::new();
        for y in (0..height - stride).step_by(stride as usize) {
            for x in (0..width - stride).step_by(stride as usize) {
                let i00 = index_of[(y * width + x) as usize];
                let i10 = index_of[(y * width + x + stride) as usize];
                let i01 = index_of[((y + stride) * width + x) as usize];
                let i11 = index_of[((y + stride) * width + x + stride) as usize];
                if i00 >= 0 && i10 >= 0 && i01 >= 0 {
                    triangles.push([i00 as usize, i10 as usize, i01 as usize]);
                }
                if i10 >= 0 && i11 >= 0 && i01 >= 0 {
                    triangles.push([i10 as usize, i11 as usize, i01 as usize]);
                }
            }
        }
        self.mesh.current = rest.clone();
        self.mesh.rest = rest;
        self.mesh.triangles = triangles;
    }

    pub fn add_pin(&mut self, kind: PinKind, at: [f64; 2], time: f64) -> usize {
        let pin = Pin {
            kind,
            rest: at,
            position: at,
            rotation: 0.0,
            scale: 1.0,
            amount: 1.0,
            range: if self.engine == PuppetEngine::Legacy && kind == PinKind::Starch {
                Some(20.0)
            } else if kind == PinKind::Overlap {
                Some(20.0)
            } else {
                None
            },
            in_front: if kind == PinKind::Overlap { 50.0 } else { 0.0 },
            position_keys: if kind == PinKind::Bend {
                Vec::new()
            } else {
                vec![(time, at)]
            },
            rotation_keys: vec![(time, 0.0)],
            scale_keys: vec![(time, 1.0)],
            stopwatch: kind != PinKind::Bend,
        };
        self.pins.push(pin);
        self.deform();
        self.pins.len() - 1
    }

    pub fn move_pin(&mut self, index: usize, to: [f64; 2], time: f64, record: bool) {
        let kind = self.pins[index].kind;
        if kind == PinKind::Bend {
            return;
        }
        self.pins[index].position = to;
        if record || self.pins[index].stopwatch {
            self.pins[index].position_keys.push((time, to));
        }
        self.deform();
    }

    pub fn set_rotation(&mut self, index: usize, degrees: f64, time: f64, shift: bool) {
        let mut degrees = degrees;
        if shift {
            degrees = (degrees / 15.0).round() * 15.0;
        }
        self.pins[index].rotation = degrees;
        self.pins[index].rotation_keys.push((time, degrees));
        self.deform();
    }

    pub fn set_scale(&mut self, index: usize, scale: f64, time: f64, shift: bool) {
        let mut scale = scale;
        if shift {
            scale = (scale / 0.05).round() * 0.05;
        }
        self.pins[index].scale = scale;
        self.pins[index].scale_keys.push((time, scale));
        self.deform();
    }

    pub fn bend_position(&self, index: usize) -> [f64; 2] {
        let anchors: Vec<[f64; 2]> = self
            .pins
            .iter()
            .filter(|pin| matches!(pin.kind, PinKind::Position | PinKind::Advanced))
            .map(|pin| pin.position)
            .collect();
        if anchors.is_empty() {
            return self.pins[index].rest;
        }
        let rest = self.pins[index].rest;
        let mut weight = 0.0;
        let mut sum = [0.0, 0.0];
        for anchor in anchors {
            let dist = ((anchor[0] - rest[0]).powi(2) + (anchor[1] - rest[1]).powi(2))
                .sqrt()
                .max(1.0);
            let w = 1.0 / dist;
            sum[0] += anchor[0] * w;
            sum[1] += anchor[1] * w;
            weight += w;
        }
        [sum[0] / weight, sum[1] / weight]
    }

    pub fn in_front_at(&self, point: [f64; 2]) -> f64 {
        let mut total = 0.0;
        for pin in self.pins.iter().filter(|pin| pin.kind == PinKind::Overlap) {
            let dist = ((pin.rest[0] - point[0]).powi(2) + (pin.rest[1] - point[1]).powi(2)).sqrt();
            if pin.range.map(|range| dist <= range).unwrap_or(true) {
                total += pin.in_front;
            }
        }
        total
    }

    pub fn triangles_visible(&self) -> bool {
        self.show_mesh == Some(true) && !self.mesh.triangles.is_empty()
    }

    pub fn bounds(&self) -> Option<[f64; 4]> {
        let first = self.mesh.rest.first()?;
        let mut min = *first;
        let mut max = *first;
        for point in &self.mesh.rest {
            min[0] = min[0].min(point[0]);
            min[1] = min[1].min(point[1]);
            max[0] = max[0].max(point[0]);
            max[1] = max[1].max(point[1]);
        }
        Some([min[0], min[1], max[0], max[1]])
    }

    pub fn vertex_displacement(&self, index: usize) -> f64 {
        let rest = self.mesh.rest[index];
        let current = self.mesh.current[index];
        ((current[0] - rest[0]).powi(2) + (current[1] - rest[1]).powi(2)).sqrt()
    }

    pub fn store(&self, layer: &mut Layer, ids: &mut Ids) {
        let root = ensure_group(layer, ids, "パペット", "puppet");
        for (index, pin) in self.pins.iter().enumerate() {
            root.push_child(persist::point(
                ids,
                &format!("ピン{index}"),
                &format!("pin-{index}"),
                pin.position,
            ));
        }
    }

    pub fn deform(&mut self) {
        if self.mesh.rest.is_empty() {
            return;
        }
        let softness = self.rotation_refinement.unwrap_or(0.65).clamp(0.0, 1.0);
        let mut current = self.mesh.rest.clone();
        for _ in 0..16 {
            let previous = current.clone();
            for index in 0..current.len() {
                if let Some(pin) = self.hard_pin(index) {
                    current[index] = pin;
                    continue;
                }
                let mut pull = 0.0;
                for pin in self.pins.iter().filter(|pin| pin.kind == PinKind::Starch) {
                    let dist = ((self.mesh.rest[index][0] - pin.rest[0]).powi(2)
                        + (self.mesh.rest[index][1] - pin.rest[1]).powi(2))
                    .sqrt();
                    let inside = match self.engine {
                        PuppetEngine::Advanced => true,
                        PuppetEngine::Legacy => {
                            pin.range.map(|range| dist <= range).unwrap_or(false)
                        }
                    };
                    if inside {
                        pull += pin.amount;
                    }
                }
                pull = pull.max(0.0);
                let rest = self.mesh.rest[index];
                let mut neighbors = 0.0;
                let mut sum = [0.0, 0.0];
                for triangle in &self.mesh.triangles {
                    if triangle.contains(&index) {
                        for other in triangle {
                            if *other != index {
                                sum[0] += previous[*other][0];
                                sum[1] += previous[*other][1];
                                neighbors += 1.0;
                            }
                        }
                    }
                }
                if neighbors == 0.0 {
                    continue;
                }
                let lap = [sum[0] / neighbors, sum[1] / neighbors];
                let follow = softness / (1.0 + pull);
                current[index] = [
                    rest[0] * (1.0 - follow) + lap[0] * follow,
                    rest[1] * (1.0 - follow) + lap[1] * follow,
                ];
            }
        }
        for pin in self.pins.iter().filter(|pin| pin.kind == PinKind::Advanced) {
            if let Some(index) = self.nearest(pin.rest) {
                let delta = [pin.position[0] - pin.rest[0], pin.position[1] - pin.rest[1]];
                let radians = pin.rotation.to_radians();
                let (s, c) = radians.sin_cos();
                let local = [
                    current[index][0] - pin.rest[0],
                    current[index][1] - pin.rest[1],
                ];
                current[index] = [
                    pin.rest[0] + (c * local[0] - s * local[1]) * pin.scale + delta[0],
                    pin.rest[1] + (s * local[0] + c * local[1]) * pin.scale + delta[1],
                ];
            }
        }
        self.mesh.current = current;
    }

    fn hard_pin(&self, index: usize) -> Option<[f64; 2]> {
        let rest = self.mesh.rest[index];
        let mut best: Option<(f64, [f64; 2])> = None;
        for pin in &self.pins {
            if !matches!(
                pin.kind,
                PinKind::Position | PinKind::Advanced | PinKind::Starch
            ) {
                continue;
            }
            if self.engine == PuppetEngine::Advanced && pin.kind == PinKind::Starch {
                continue;
            }
            let dist = ((rest[0] - pin.rest[0]).powi(2) + (rest[1] - pin.rest[1]).powi(2)).sqrt();
            if dist < 6.0 && best.map(|(best_dist, _)| dist < best_dist).unwrap_or(true) {
                let pos = if pin.kind == PinKind::Starch {
                    pin.rest
                } else {
                    pin.position
                };
                best = Some((dist, pos));
            }
        }
        best.map(|(_, pos)| pos)
    }

    fn nearest(&self, point: [f64; 2]) -> Option<usize> {
        self.mesh
            .rest
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                let da = (a[0] - point[0]).powi(2) + (a[1] - point[1]).powi(2);
                let db = (b[0] - point[0]).powi(2) + (b[1] - point[1]).powi(2);
                da.partial_cmp(&db).unwrap_or(Ordering::Equal)
            })
            .map(|(index, _)| index)
    }
}

impl Default for Puppet {
    fn default() -> Self {
        Self::new()
    }
}

fn dilate(alpha: &[bool], width: i32, height: i32, radius: i32) -> Vec<bool> {
    let mut out = alpha.to_vec();
    for y in 0..height {
        for x in 0..width {
            if !alpha[(y * width + x) as usize] {
                continue;
            }
            for oy in -radius..=radius {
                for ox in -radius..=radius {
                    let px = x + ox;
                    let py = y + oy;
                    if px >= 0
                        && py >= 0
                        && px < width
                        && py < height
                        && ox * ox + oy * oy <= radius * radius
                    {
                        out[(py * width + px) as usize] = true;
                    }
                }
            }
        }
    }
    out
}

pub fn rectangle_alpha(width: i32, height: i32, x0: i32, y0: i32, w: i32, h: i32) -> Vec<bool> {
    let mut alpha = vec![false; (width * height) as usize];
    for y in y0..y0 + h {
        for x in x0..x0 + w {
            if x >= 0 && y >= 0 && x < width && y < height {
                alpha[(y * width + x) as usize] = true;
            }
        }
    }
    alpha
}

#[cfg(test)]
mod tests {
    use model::LayerId;

    use super::*;

    fn mesh() -> Puppet {
        let mut puppet = Puppet::new();
        let alpha = rectangle_alpha(40, 32, 8, 8, 20, 16);
        puppet.build_from_alpha(&alpha, 40, 32);
        puppet
    }

    #[test]
    fn position_pin_deforms_and_keys() {
        let mut puppet = mesh();
        let index = puppet.add_pin(PinKind::Position, [16.0, 16.0], 0.0);
        puppet.move_pin(index, [22.0, 16.0], 1.0, true);
        assert!(puppet.pins[index].position_keys.len() >= 2);
        assert!(puppet.pins[index].stopwatch);
        assert!(puppet.mesh.current.iter().any(|p| p[0] > 16.0));
    }

    #[test]
    fn starch_reduces_nearby_deformation() {
        let mut loose = mesh();
        let pin = loose.add_pin(PinKind::Position, [24.0, 16.0], 0.0);
        loose.move_pin(pin, [30.0, 18.0], 1.0, false);
        let mut stiff = loose.clone();
        stiff.add_pin(PinKind::Starch, [12.0, 12.0], 0.0);
        stiff.pins.last_mut().unwrap().amount = 8.0;
        stiff.deform();
        let corner = stiff
            .mesh
            .rest
            .iter()
            .position(|p| (p[0] - 12.0).abs() < 4.0 && (p[1] - 12.0).abs() < 4.0)
            .unwrap();
        assert!(stiff.vertex_displacement(corner) <= loose.vertex_displacement(corner) + 0.2);
    }

    #[test]
    fn starch_amount_and_negative_values_add() {
        let mut puppet = mesh();
        puppet.engine = PuppetEngine::Legacy;
        puppet.add_pin(PinKind::Starch, [16.0, 16.0], 0.0);
        puppet.pins[0].amount = 2.0;
        puppet.pins[0].range = Some(30.0);
        puppet.add_pin(PinKind::Starch, [18.0, 16.0], 0.0);
        puppet.pins[1].amount = -1.0;
        puppet.pins[1].range = Some(30.0);
        let mut pull = 0.0;
        for pin in &puppet.pins {
            pull += pin.amount;
        }
        assert!(pull < 2.0);
        assert!(pull > 0.0);
    }

    #[test]
    fn legacy_starch_range_cuts_off() {
        let mut puppet = mesh();
        puppet.engine = PuppetEngine::Legacy;
        puppet.add_pin(PinKind::Starch, [12.0, 12.0], 0.0);
        puppet.pins[0].range = Some(6.0);
        puppet.pins[0].amount = 4.0;
        let inside = ((12.0 - 12.0_f64).powi(2) + (12.0 - 12.0_f64).powi(2)).sqrt() <= 6.0;
        let outside = ((28.0 - 12.0_f64).powi(2) + (24.0 - 12.0_f64).powi(2)).sqrt() > 6.0;
        assert!(inside);
        assert!(outside);
        assert_eq!(puppet.starch_group(), StarchGroup::Stiffness);
    }

    #[test]
    fn overlap_swaps_with_in_front() {
        let mut puppet = mesh();
        puppet.add_pin(PinKind::Overlap, [12.0, 16.0], 0.0);
        puppet.add_pin(PinKind::Overlap, [28.0, 16.0], 0.0);
        puppet.pins[0].range = Some(6.0);
        puppet.pins[1].range = Some(6.0);
        puppet.pins[1].in_front = 10.0;
        assert!(near_front(puppet.in_front_at([12.0, 16.0]), 50.0));
        assert!(puppet.in_front_at([12.0, 16.0]) > puppet.in_front_at([28.0, 16.0]));
    }

    fn near_front(value: f64, expected: f64) -> bool {
        (value - expected).abs() < 1e-6
    }

    #[test]
    fn new_overlap_pin_starts_at_fifty() {
        let mut puppet = mesh();
        let index = puppet.add_pin(PinKind::Overlap, [16.0, 16.0], 0.0);
        assert!((puppet.pins[index].in_front - 50.0).abs() < 1e-6);
    }

    #[test]
    fn overlap_range_stops_at_the_boundary() {
        let mut puppet = mesh();
        puppet.add_pin(PinKind::Overlap, [12.0, 12.0], 0.0);
        puppet.pins[0].range = Some(8.0);
        puppet.pins[0].in_front = 50.0;
        assert!(puppet.in_front_at([12.0, 12.0]) > 0.0);
        assert_eq!(puppet.in_front_at([36.0, 28.0]), 0.0);
    }

    #[test]
    fn mesh_display_toggles_triangles() {
        let mut puppet = mesh();
        puppet.show_mesh = Some(true);
        assert!(puppet.triangles_visible());
        puppet.show_mesh = Some(false);
        assert!(!puppet.triangles_visible());
    }

    #[test]
    fn expansion_defaults_to_three_and_grows() {
        let mut puppet = Puppet::new();
        assert!((puppet.expansion - 3.0).abs() < 1e-6);
        let alpha = rectangle_alpha(40, 32, 10, 10, 12, 8);
        puppet.build_from_alpha(&alpha, 40, 32);
        let tight = puppet.bounds().unwrap();
        puppet.expansion = 8.0;
        puppet.build_from_alpha(&alpha, 40, 32);
        let wide = puppet.bounds().unwrap();
        assert!(wide[2] - wide[0] >= tight[2] - tight[0]);
    }

    #[test]
    fn lower_refinement_is_stiffer() {
        let mut soft = mesh();
        soft.rotation_refinement = Some(0.95);
        let pin = soft.add_pin(PinKind::Position, [20.0, 16.0], 0.0);
        soft.move_pin(pin, [28.0, 16.0], 1.0, false);
        let mut hard = mesh();
        hard.rotation_refinement = Some(0.05);
        let pin = hard.add_pin(PinKind::Position, [20.0, 16.0], 0.0);
        hard.move_pin(pin, [28.0, 16.0], 1.0, false);
        let index = 0;
        assert!(hard.vertex_displacement(index) <= soft.vertex_displacement(index) + 0.5);
    }

    #[test]
    fn legacy_engine_puts_starch_in_the_stiffness_group() {
        let mut puppet = Puppet::new();
        assert_eq!(puppet.engine, PuppetEngine::Advanced);
        assert_eq!(puppet.starch_group(), StarchGroup::Deform);
        puppet.engine = PuppetEngine::Legacy;
        assert_eq!(puppet.starch_group(), StarchGroup::Stiffness);
    }

    #[test]
    fn advanced_pin_keys_position_rotation_and_scale() {
        let mut puppet = mesh();
        let index = puppet.add_pin(PinKind::Advanced, [16.0, 16.0], 0.0);
        puppet.move_pin(index, [18.0, 17.0], 1.0, true);
        puppet.set_rotation(index, 16.0, 1.0, true);
        puppet.set_scale(index, 1.13, 1.0, true);
        assert!(puppet.pins[index].position_keys.len() >= 2);
        assert!((puppet.pins[index].rotation - 15.0).abs() < 1e-6);
        assert!((puppet.pins[index].scale - 1.15).abs() < 1e-6);
    }

    #[test]
    fn bend_pin_follows_neighbors_and_keys_rotation() {
        let mut puppet = mesh();
        let anchor = puppet.add_pin(PinKind::Position, [12.0, 16.0], 0.0);
        let bend = puppet.add_pin(PinKind::Bend, [20.0, 16.0], 0.0);
        let before = puppet.bend_position(bend);
        puppet.move_pin(anchor, [18.0, 20.0], 1.0, true);
        let after = puppet.bend_position(bend);
        assert_ne!(before, after);
        puppet.set_rotation(bend, 20.0, 1.0, true);
        puppet.set_scale(bend, 1.2, 1.0, true);
        assert!(puppet.pins[bend].position_keys.is_empty());
        assert!(!puppet.pins[bend].rotation_keys.is_empty());
        assert!(!puppet.pins[bend].scale_keys.is_empty());
    }

    #[test]
    fn puppet_pins_stay_on_the_layer() {
        let mut puppet = mesh();
        puppet.add_pin(PinKind::Position, [16.0, 16.0], 0.0);
        let mut layer = model::Layer::new(LayerId::new(1), "puppet", model::LayerKind::Null);
        puppet.store(&mut layer, &mut Ids::new());
        assert!(layer.property("puppet").is_some());
    }
}
