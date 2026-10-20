use std::collections::BTreeSet;

use crate::id::PropertyId;
use crate::shape::{lerp_points, ShapePath};

/// 閉じたマスクの合成。新規は Add。開いたパスは None ではなくモードを持たない。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum MaskMode {
    #[default]
    Add,
    Subtract,
    Intersect,
    Difference,
    None,
}

impl MaskMode {
    pub const ALL: [Self; 5] = [
        Self::Add,
        Self::Subtract,
        Self::Intersect,
        Self::Difference,
        Self::None,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Add => "Add",
            Self::Subtract => "Subtract",
            Self::Intersect => "Intersect",
            Self::Difference => "Difference",
            Self::None => "None",
        }
    }
}

/// モードはキーフレームもエクスプレッションも持てない。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NotAnimatable;

/// スライダーで上げられるぼかしの上限。下限は要確認。
pub const FEATHER_SLIDER_MAX: f64 = 32_000.0;

/// 拡張をスクラブできる範囲。
pub const EXPANSION_SCRUB_MIN: f64 = -32_000.0;
pub const EXPANSION_SCRUB_MAX: f64 = 32_000.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum FeatherFalloff {
    #[default]
    Smooth,
    Linear,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FeatherProfile {
    Gaussian,
}

pub const FEATHER_PROFILE: FeatherProfile = FeatherProfile::Gaussian;

#[derive(Clone, Debug, PartialEq)]
pub struct VertexFeather {
    pub position: f64,
    pub outer: f64,
    pub inner: Option<f64>,
    pub tension: Option<f64>,
    pub corner_angle: Option<f64>,
    pub hold: Option<bool>,
}

/// レイヤーのマスク。不透明度 1 が 100% で、内側を不透明にする。
#[derive(Clone, Debug, PartialEq)]
pub struct Mask {
    pub id: PropertyId,
    pub name: String,
    pub path: ShapePath,
    pub mode: MaskMode,
    pub inverted: bool,
    pub feather: [f64; 2],
    pub expansion: f64,
    pub opacity: f64,
    pub feather_falloff: FeatherFalloff,
    pub per_vertex_feather: Vec<VertexFeather>,
}

impl Mask {
    pub fn new(id: PropertyId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            path: ShapePath::new(),
            mode: MaskMode::Add,
            inverted: false,
            feather: [0.0, 0.0],
            expansion: 0.0,
            opacity: 1.0,
            feather_falloff: FeatherFalloff::Smooth,
            per_vertex_feather: Vec::new(),
        }
    }

    pub fn effective_mode(&self) -> Option<MaskMode> {
        if self.path.closed {
            Some(self.mode)
        } else {
            None
        }
    }

    pub fn punches_alpha(&self) -> bool {
        self.path.closed && self.mode != MaskMode::None
    }

    pub fn close_with_mode(&mut self, mode: Option<MaskMode>) {
        self.path.closed = true;
        self.mode = mode.unwrap_or(MaskMode::Add);
    }

    pub fn try_keyframe_mode(&self) -> Result<(), NotAnimatable> {
        Err(NotAnimatable)
    }

    pub fn try_express_mode(&self) -> Result<(), NotAnimatable> {
        Err(NotAnimatable)
    }

    pub fn set_feather(&mut self, horizontal: f64, vertical: f64) {
        self.feather = [
            clamp_feather_slider(horizontal),
            clamp_feather_slider(vertical),
        ];
    }

    pub fn set_expansion(&mut self, expansion: f64) {
        self.expansion = clamp_expansion_scrub(expansion);
    }
}

pub fn clamp_feather_slider(value: f64) -> f64 {
    value.min(FEATHER_SLIDER_MAX)
}

pub fn clamp_expansion_scrub(value: f64) -> f64 {
    value.clamp(EXPANSION_SCRUB_MIN, EXPANSION_SCRUB_MAX)
}

/// 幅は端の内外に半分ずつ乗る。
pub fn feather_band(width: f64) -> (f64, f64) {
    let half = width * 0.5;
    (half, half)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayerRect {
    pub min: [f64; 2],
    pub max: [f64; 2],
}

impl LayerRect {
    pub fn contains(self, point: [f64; 2]) -> bool {
        point[0] >= self.min[0]
            && point[0] <= self.max[0]
            && point[1] >= self.min[1]
            && point[1] <= self.max[1]
    }
}

pub fn feather_applies_at(point: [f64; 2], bounds: LayerRect) -> bool {
    bounds.contains(point)
}

pub fn mask_influences(mask: &Mask, point: [f64; 2]) -> bool {
    mask.path.closed && mask.path.signed_distance(point) <= mask.expansion
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MaskAlpha {
    Known(f64),
    Unconfirmed,
}

pub fn composite_alpha(masks: &[Mask], point: [f64; 2], layer_alpha: f64) -> MaskAlpha {
    let mut current = MaskAlpha::Known(layer_alpha);
    let mut have_previous = false;
    let mut previous_inside = false;
    for mask in masks {
        if !mask.path.closed {
            continue;
        }
        let inside = mask_influences(mask, point);
        let contribution = if inside { mask.opacity } else { 0.0 };
        if mask.mode == MaskMode::None {
            have_previous = true;
            previous_inside = inside;
            continue;
        }
        current = combine_mask(
            current,
            mask.mode,
            contribution,
            inside,
            mask.opacity,
            have_previous,
            previous_inside,
            layer_alpha,
        );
        have_previous = true;
        previous_inside = inside;
    }
    current
}

fn combine_mask(
    current: MaskAlpha,
    mode: MaskMode,
    contribution: f64,
    inside: bool,
    opacity: f64,
    have_previous: bool,
    previous_inside: bool,
    layer_alpha: f64,
) -> MaskAlpha {
    match mode {
        MaskMode::Add => {
            if !have_previous {
                MaskAlpha::Known(contribution)
            } else {
                add_known(current, contribution)
            }
        }
        MaskMode::Subtract => match current {
            MaskAlpha::Known(value) => MaskAlpha::Known(value - contribution),
            MaskAlpha::Unconfirmed => MaskAlpha::Unconfirmed,
        },
        MaskMode::Intersect => {
            if !have_previous {
                if inside {
                    MaskAlpha::Known(layer_alpha + contribution)
                } else {
                    MaskAlpha::Unconfirmed
                }
            } else if inside && previous_inside {
                add_known(current, contribution)
            } else {
                MaskAlpha::Unconfirmed
            }
        }
        MaskMode::Difference => {
            if !have_previous {
                MaskAlpha::Known(layer_alpha - contribution)
            } else if inside && previous_inside {
                match current {
                    MaskAlpha::Known(value) => MaskAlpha::Known(value - contribution),
                    MaskAlpha::Unconfirmed => MaskAlpha::Unconfirmed,
                }
            } else if inside {
                MaskAlpha::Known(opacity)
            } else {
                current
            }
        }
        MaskMode::None => current,
    }
}

fn add_known(current: MaskAlpha, contribution: f64) -> MaskAlpha {
    match current {
        MaskAlpha::Known(value) => MaskAlpha::Known(value + contribution),
        MaskAlpha::Unconfirmed => MaskAlpha::Unconfirmed,
    }
}

pub fn effect_input_paths(masks: &[Mask]) -> Vec<&ShapePath> {
    masks.iter().map(|mask| &mask.path).collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PathTool {
    Selection,
    Pen,
    AddVertex,
    DeleteVertex,
    ConvertVertex,
    MaskFeather,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PathHit {
    None,
    Segment,
    Vertex,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Modifiers {
    pub alt: bool,
    pub ctrl: bool,
    pub shift: bool,
}

pub fn pen_after_g(current: PathTool, cycle_enabled: bool) -> PathTool {
    if !cycle_enabled {
        return current;
    }
    match current {
        PathTool::Pen => PathTool::MaskFeather,
        PathTool::MaskFeather => PathTool::Pen,
        other => other,
    }
}

pub fn momentary_path_tool(
    tool: PathTool,
    modifiers: Modifiers,
    hit: PathHit,
    key_v: bool,
) -> PathTool {
    if tool == PathTool::Pen && modifiers.alt && !modifiers.ctrl {
        return PathTool::ConvertVertex;
    }
    if tool == PathTool::Pen && (modifiers.ctrl || key_v) && !modifiers.alt {
        return PathTool::Selection;
    }
    if tool == PathTool::Selection && modifiers.ctrl && modifiers.alt {
        return match hit {
            PathHit::Segment => PathTool::AddVertex,
            PathHit::Vertex => PathTool::ConvertVertex,
            PathHit::None => tool,
        };
    }
    if tool == PathTool::Pen {
        return match hit {
            PathHit::Segment => PathTool::AddVertex,
            PathHit::Vertex => PathTool::DeleteVertex,
            PathHit::None => PathTool::Pen,
        };
    }
    tool
}

pub fn is_new_mask_chord(ctrl: bool, shift: bool, alt: bool, key: char) -> bool {
    ctrl && shift && !alt && key == 'N'
}

/// ペンは閉じる瞬間だけ。それ以外はマウスを押している間。
pub fn mask_mode_from_key(
    key: char,
    pen_tool: bool,
    pen_closing: bool,
    mouse_down: bool,
) -> Option<MaskMode> {
    let active = if pen_tool { pen_closing } else { mouse_down };
    if !active {
        return None;
    }
    match key {
        'A' => Some(MaskMode::Add),
        'S' => Some(MaskMode::Subtract),
        'I' => Some(MaskMode::Intersect),
        'F' => Some(MaskMode::Difference),
        'N' => Some(MaskMode::None),
        _ => None,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MaskPathProperty {
    pub stopwatch: bool,
    pub keys: Vec<ShapePath>,
}

impl MaskPathProperty {
    pub fn new(path: ShapePath) -> Self {
        Self {
            stopwatch: false,
            keys: vec![path],
        }
    }

    pub fn alt_shift_m(&mut self) {
        self.stopwatch = true;
    }
}

pub fn smart_mask_intermediate_frames(
    start_frame: i64,
    end_frame: i64,
    frames_per_second: f64,
    keys_per_second: f64,
) -> Vec<i64> {
    let mut frames = BTreeSet::new();
    if end_frame - start_frame > 1 {
        frames.insert(start_frame + 1);
        frames.insert(end_frame - 1);
    }
    if keys_per_second > 0.0 && frames_per_second > 0.0 {
        let end_time = end_frame as f64 / frames_per_second;
        let mut step = 1;
        loop {
            let time = f64::from(step) / keys_per_second;
            if time >= end_time {
                break;
            }
            let frame = time * frames_per_second;
            if (frame - frame.round()).abs() < 1e-6 {
                let frame = frame.round() as i64;
                if frame > start_frame && frame < end_frame {
                    frames.insert(frame);
                }
            }
            step += 1;
            if step > 100_000 {
                break;
            }
        }
    }
    frames.into_iter().collect()
}

pub fn smart_mask_uses_fields(fields: bool, rate: f64, frames_per_second: f64) -> bool {
    fields && (rate - frames_per_second).abs() < 1e-9
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MatchMethod {
    Auto,
    Curve,
    Polyline,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ResolvedMatch {
    Curve,
    Polyline,
}

pub fn resolve_match(method: MatchMethod, either_curved: bool) -> ResolvedMatch {
    match method {
        MatchMethod::Curve => ResolvedMatch::Curve,
        MatchMethod::Polyline => ResolvedMatch::Polyline,
        MatchMethod::Auto if either_curved => ResolvedMatch::Curve,
        MatchMethod::Auto => ResolvedMatch::Polyline,
    }
}

pub fn quality_matches_any_vertex(quality: f64) -> Option<bool> {
    if quality == 0.0 {
        Some(false)
    } else if quality == 100.0 {
        Some(true)
    } else {
        None
    }
}

pub fn bending_prefers_curve(resistance: f64) -> Option<bool> {
    if resistance == 0.0 {
        Some(true)
    } else if resistance == 100.0 {
        Some(false)
    } else {
        None
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SmartMaskResult {
    pub original_vertex_counts: Vec<usize>,
    pub generated: Vec<ShapePath>,
    pub created: u32,
    pub cancelled: bool,
}

pub fn apply_smart_mask(
    keys: &[ShapePath],
    add_vertices: bool,
    outline_percent: f64,
) -> SmartMaskResult {
    let original_vertex_counts = keys.iter().map(|path| path.vertices.len()).collect();
    let mut generated = Vec::new();
    if keys.len() >= 2 {
        let mut midway = ShapePath::polygon(
            lerp_points(&keys[0].vertices, &keys[1].vertices, 0.5),
            keys[0].closed,
        );
        if add_vertices {
            midway.vertices =
                subdivide_outline_percent(&midway.vertices, midway.closed, outline_percent);
        }
        generated.push(midway);
    }
    SmartMaskResult {
        original_vertex_counts,
        generated,
        created: 0,
        cancelled: false,
    }
}

pub fn subdivide_outline_percent(
    vertices: &[[f64; 2]],
    closed: bool,
    percent: f64,
) -> Vec<[f64; 2]> {
    if vertices.len() < 2 || percent <= 0.0 {
        return vertices.to_vec();
    }
    let mut lengths = Vec::new();
    let mut total = 0.0;
    let edges = if closed {
        vertices.len()
    } else {
        vertices.len() - 1
    };
    for index in 0..edges {
        let length = distance(vertices[index], vertices[(index + 1) % vertices.len()]);
        lengths.push(length);
        total += length;
    }
    let step = total * percent / 100.0;
    if step <= 0.0 {
        return vertices.to_vec();
    }
    let mut output = vec![vertices[0]];
    let mut next_mark = step;
    let mut walked = 0.0;
    for index in 0..edges {
        let start = vertices[index];
        let end = vertices[(index + 1) % vertices.len()];
        let length = lengths[index];
        while next_mark < walked + length && next_mark < total {
            let t = (next_mark - walked) / length;
            output.push([
                start[0] + (end[0] - start[0]) * t,
                start[1] + (end[1] - start[1]) * t,
            ]);
            next_mark += step;
        }
        walked += length;
        if !closed && index + 1 == edges {
            output.push(end);
        }
    }
    output
}

pub fn inner_feather_offset(feather: &VertexFeather) -> f64 {
    feather.inner.unwrap_or(0.0)
}

pub fn stop_boundary_on_path(offset: f64) -> f64 {
    offset.max(0.0)
}

pub fn set_span_outer(points: &mut [VertexFeather], start: usize, end: usize, outer: f64) {
    let end = end.min(points.len().saturating_sub(1));
    for point in &mut points[start..=end] {
        point.outer = outer;
    }
}

pub fn advance_feather_position(position: f64, delta: f64, length: f64) -> f64 {
    if length <= 0.0 {
        return 0.0;
    }
    let mut next = (position + delta) % length;
    if next < 0.0 {
        next += length;
    }
    next
}

pub fn feather_outer_at(points: &[VertexFeather], position: f64) -> Option<f64> {
    if points.is_empty() {
        return None;
    }
    let mut previous = &points[0];
    for point in points.iter().skip(1) {
        if point.position > position {
            if previous.hold == Some(true) {
                return Some(previous.outer);
            }
            let span = point.position - previous.position;
            let t = if span == 0.0 {
                0.0
            } else {
                (position - previous.position) / span
            };
            return Some(previous.outer + (point.outer - previous.outer) * t);
        }
        previous = point;
    }
    Some(previous.outer)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VertexFeatherInfo {
    pub count: usize,
    pub length: f64,
    pub position: f64,
    pub tension: Option<f64>,
    pub falloff: FeatherFalloff,
}

pub fn vertex_feather_info(
    points: &[VertexFeather],
    length: f64,
    index: usize,
    falloff: FeatherFalloff,
) -> Option<VertexFeatherInfo> {
    let point = points.get(index)?;
    Some(VertexFeatherInfo {
        count: points.len(),
        length,
        position: point.position,
        tension: point.tension,
        falloff,
    })
}

fn distance(start: [f64; 2], end: [f64; 2]) -> f64 {
    let x = end[0] - start[0];
    let y = end[1] - start[1];
    (x * x + y * y).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shape::{
        arrow_nudge, corresponding_pairs, scale_from_center, transform_points, PathKind, VertexKind,
    };

    fn near(left: f64, right: f64) -> bool {
        (left - right).abs() < 1e-6
    }

    fn near2(left: [f64; 2], right: [f64; 2]) -> bool {
        near(left[0], right[0]) && near(left[1], right[1])
    }

    fn rect_mask(id: u64, min: [f64; 2], max: [f64; 2], mode: MaskMode) -> Mask {
        let mut mask = Mask::new(PropertyId::new(id), "Mask");
        mask.path = ShapePath::polygon(
            vec![
                [min[0], min[1]],
                [max[0], min[1]],
                [max[0], max[1]],
                [min[0], max[1]],
            ],
            true,
        );
        mask.mode = mode;
        mask.opacity = 1.0;
        mask
    }

    #[test]
    fn pen_closed_path_punches_and_open_path_has_no_mode() {
        let mut mask = Mask::new(PropertyId::new(1), "Mask");
        assert_eq!(mask.mode, MaskMode::Add);
        assert_eq!(mask.effective_mode(), None);
        assert!(!mask.punches_alpha());
        mask.path = ShapePath::polygon(vec![[0.0, 0.0], [1.0, 0.0]], false);
        assert_eq!(mask.effective_mode(), None);
        assert_eq!(
            composite_alpha(&[mask.clone()], [0.2, 0.0], 0.4),
            MaskAlpha::Known(0.4)
        );
        mask.close_with_mode(mask_mode_from_key('S', true, true, false));
        assert_eq!(mask.mode, MaskMode::Subtract);
        assert!(mask.punches_alpha());
        assert_eq!(pen_after_g(PathTool::Pen, true), PathTool::MaskFeather);
        assert_eq!(pen_after_g(PathTool::MaskFeather, true), PathTool::Pen);
        assert_eq!(pen_after_g(PathTool::Pen, false), PathTool::Pen);
        let alt = Modifiers {
            alt: true,
            ctrl: false,
            shift: false,
        };
        assert_eq!(
            momentary_path_tool(PathTool::Pen, alt, PathHit::None, false),
            PathTool::ConvertVertex
        );
        let ctrl = Modifiers {
            alt: false,
            ctrl: true,
            shift: false,
        };
        assert_eq!(
            momentary_path_tool(PathTool::Pen, ctrl, PathHit::None, false),
            PathTool::Selection
        );
        assert_eq!(
            momentary_path_tool(
                PathTool::Pen,
                Modifiers {
                    alt: false,
                    ctrl: false,
                    shift: false
                },
                PathHit::None,
                true
            ),
            PathTool::Selection
        );
        assert_eq!(
            momentary_path_tool(
                PathTool::Pen,
                Modifiers {
                    alt: false,
                    ctrl: false,
                    shift: false
                },
                PathHit::Segment,
                false
            ),
            PathTool::AddVertex
        );
        assert_eq!(
            momentary_path_tool(
                PathTool::Pen,
                Modifiers {
                    alt: false,
                    ctrl: false,
                    shift: false
                },
                PathHit::Vertex,
                false
            ),
            PathTool::DeleteVertex
        );
        let chord = Modifiers {
            alt: true,
            ctrl: true,
            shift: false,
        };
        assert_eq!(
            momentary_path_tool(PathTool::Selection, chord, PathHit::Segment, false),
            PathTool::AddVertex
        );
        assert_eq!(
            momentary_path_tool(PathTool::Selection, chord, PathHit::Vertex, false),
            PathTool::ConvertVertex
        );
        assert!(is_new_mask_chord(true, true, false, 'N'));
        assert_eq!(mask_mode_from_key('A', true, false, false), None);
        assert_eq!(
            mask_mode_from_key('F', false, false, true),
            Some(MaskMode::Difference)
        );
        assert_eq!(mask_mode_from_key('F', false, false, false), None);
    }

    #[test]
    fn bezier_numbers_vertices_and_arms_the_stopwatch() {
        let mut start = ShapePath::polygon(vec![[0.0, 0.0], [2.0, 3.0], [1.0, 1.0]], true);
        assert_eq!(start.topmost_vertex(), Some(1));
        assert!(start.vertex_is_enlarged(1));
        assert_eq!(start.vertex_order()[0], 1);
        let end = ShapePath::polygon(vec![[0.0, 0.0], [4.0, 3.0], [1.0, 1.0]], true);
        let before = corresponding_pairs(&start, &end);
        assert_eq!(before[0], (1, 1));
        assert!(start.set_first_vertex(0));
        let after = corresponding_pairs(&start, &end);
        assert_ne!(before, after);
        let mid = crate::shape::interpolate_paired(&start, &end, 0.5);
        assert!(near2(mid[0], [2.0, 1.5]));
        let mut keys = vec![
            ShapePath::polygon(vec![[0.0, 0.0], [2.0, 0.0]], false),
            ShapePath::polygon(vec![[0.0, 1.0], [2.0, 1.0]], false),
        ];
        assert!(crate::shape::add_vertex_across_duration(
            &mut keys,
            0,
            0,
            0.5,
            [1.0, 2.0]
        ));
        assert!(near2(keys[0].vertices[1], [1.0, 2.0]));
        assert!(near2(keys[1].vertices[1], [1.0, 1.0]));
        assert!(crate::shape::delete_vertex_across_duration(&mut keys, 1));
        assert_eq!(keys[0].vertices.len(), 2);
        assert_eq!(keys[1].vertices.len(), 2);
        let mut property = MaskPathProperty::new(start);
        assert!(!property.stopwatch);
        property.alt_shift_m();
        assert!(property.stopwatch);
        assert!(near(arrow_nudge(false, 1.0), 1.0));
        assert!(near(arrow_nudge(true, 1.0), 10.0));
        assert!(near(arrow_nudge(false, 2.0), 0.5));
        assert_eq!(
            crate::shape::ShapePath::polygon(vec![[0.0, 0.0], [1.0, 0.0]], false)
                .vertex_order()
                .len(),
            2
        );
    }

    #[test]
    fn rotobezier_changes_shape_only_when_converting_from_manual() {
        let mut path = ShapePath::polygon(vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]], true);
        path.out_tangents[0] = [0.4, 0.2];
        let manual = path.evaluated_tangents();
        path.convert_to_rotobezier(1.0);
        let auto = path.evaluated_tangents();
        assert_ne!(manual, auto);
        let baked = auto.clone();
        path.convert_to_bezier();
        assert_eq!(path.kind, PathKind::Bezier);
        assert_eq!(path.evaluated_tangents(), baked);
        path.convert_to_rotobezier(1.0);
        let before = path.vertices.clone();
        assert!(path.set_tension(0, 0.25));
        assert_eq!(path.vertices, before);
        assert!(near(path.tensions[0], 0.25));
        let segment = path.vertices[0];
        assert!(path.drag_curve_segment(0, [0.0, 1.0]));
        assert!(near2(path.vertices[0], [segment[0], segment[1] + 1.0]));
        assert!(near2(path.vertices[1], [1.0, 1.0]));
    }

    #[test]
    fn convert_vertex_toggles_and_drag_creates_handles() {
        let mut path = ShapePath::polygon(vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]], true);
        assert_eq!(path.ctrl_alt_click(0), Some(VertexKind::Smooth));
        assert_eq!(path.ctrl_alt_click(0), Some(VertexKind::Corner));
        assert!(path.ctrl_alt_drag(0, [0.3, 0.1]));
        assert_eq!(path.vertex_kinds[0], VertexKind::Smooth);
        assert_eq!(path.out_tangents[0], [0.3, 0.1]);
        assert_eq!(path.in_tangents[0], [-0.3, -0.1]);
    }

    #[test]
    fn free_transform_rotates_around_its_own_anchor_and_lerps() {
        let points = vec![[1.0, 0.0], [0.0, 1.0]];
        let around_origin = transform_points(&points, [0.0, 0.0], [0.0, 0.0], 90.0, [1.0, 1.0]);
        let around_box = transform_points(&points, [2.0, 0.0], [0.0, 0.0], 90.0, [1.0, 1.0]);
        assert!(near2(around_origin[0], [0.0, -1.0]));
        assert_ne!(around_origin, around_box);
        let start = transform_points(&points, [0.0, 0.0], [0.0, 0.0], 0.0, [1.0, 1.0]);
        let end = transform_points(&points, [0.0, 0.0], [0.0, 0.0], 90.0, [1.0, 1.0]);
        let straight = lerp_points(&start, &end, 0.5);
        let arc = transform_points(&points, [0.0, 0.0], [0.0, 0.0], 45.0, [1.0, 1.0]);
        assert!(near2(straight[0], [0.5, -0.5]));
        assert!(!near2(straight[0], arc[0]));
        let square = vec![[0.0, 0.0], [2.0, 0.0], [2.0, 2.0], [0.0, 2.0]];
        let scaled = scale_from_center(&square, [2.0, 2.0]);
        assert!(near2(
            crate::shape::bbox_center(&scaled).unwrap(),
            [1.0, 1.0]
        ));
    }

    #[test]
    fn smart_mask_interpolation_places_the_documented_frames() {
        let frames = smart_mask_intermediate_frames(0, 30, 30.0, 10.0);
        assert_eq!(frames, vec![1, 3, 6, 9, 12, 15, 18, 21, 24, 27, 29]);
        let keys = vec![
            ShapePath::polygon(vec![[0.0, 0.0], [100.0, 0.0]], false),
            ShapePath::polygon(vec![[0.0, 1.0], [100.0, 1.0]], false),
        ];
        let before = keys[0].vertices.len();
        let result = apply_smart_mask(&keys, true, 5.0);
        assert_eq!(keys[0].vertices.len(), before);
        assert_eq!(result.original_vertex_counts, vec![before, before]);
        assert!(result.generated[0].vertices.len() > before);
        assert_eq!(quality_matches_any_vertex(0.0), Some(false));
        assert_eq!(quality_matches_any_vertex(100.0), Some(true));
        assert_eq!(quality_matches_any_vertex(40.0), None);
        assert_eq!(bending_prefers_curve(0.0), Some(true));
        assert_eq!(bending_prefers_curve(100.0), Some(false));
        assert_eq!(resolve_match(MatchMethod::Auto, true), ResolvedMatch::Curve);
        assert_eq!(
            resolve_match(MatchMethod::Auto, false),
            ResolvedMatch::Polyline
        );
        assert!(smart_mask_uses_fields(true, 30.0, 30.0));
        assert!(!smart_mask_uses_fields(true, 10.0, 30.0));
        let mut job = result;
        job.cancelled = true;
        assert!(job.cancelled);
    }

    #[test]
    fn feather_splits_the_width_and_stops_at_the_layer() {
        let (inner, outer) = feather_band(25.0);
        assert!(near(inner, 12.5));
        assert!(near(outer, 12.5));
        assert_eq!(FEATHER_PROFILE, FeatherProfile::Gaussian);
        let bounds = LayerRect {
            min: [0.0, 0.0],
            max: [10.0, 10.0],
        };
        assert!(!feather_applies_at([12.0, 5.0], bounds));
        assert!(feather_applies_at([9.0, 5.0], bounds));
        let mut mask = Mask::new(PropertyId::new(1), "Mask");
        mask.set_feather(32_001.0, -4.0);
        assert!(near(mask.feather[0], 32_000.0));
        assert!(near(mask.feather[1], -4.0));
    }

    #[test]
    fn expansion_moves_influence_not_vertices() {
        let mut mask = rect_mask(1, [0.0, 0.0], [10.0, 10.0], MaskMode::Add);
        let vertices = mask.path.vertices.clone();
        mask.set_expansion(32_001.0);
        assert!(near(mask.expansion, 32_000.0));
        mask.set_expansion(-32_001.0);
        assert!(near(mask.expansion, -32_000.0));
        mask.expansion = 1.5;
        assert_eq!(mask.path.vertices, vertices);
        assert!(mask_influences(&mask, [11.0, 5.0]));
        mask.expansion = 0.0;
        assert!(!mask_influences(&mask, [11.0, 5.0]));
        mask.expansion = -1.0;
        assert!(!mask_influences(&mask, [0.5, 5.0]));
        assert!(mask_influences(&mask, [5.0, 5.0]));
    }

    #[test]
    fn opacity_makes_the_inside_opaque_and_ignores_open_paths() {
        let mask = rect_mask(1, [0.0, 0.0], [4.0, 4.0], MaskMode::Add);
        assert_eq!(
            composite_alpha(&[mask.clone()], [1.0, 1.0], 0.2),
            MaskAlpha::Known(1.0)
        );
        assert_eq!(
            composite_alpha(&[mask], [8.0, 8.0], 0.2),
            MaskAlpha::Known(0.0)
        );
        let mut open = Mask::new(PropertyId::new(2), "Open");
        open.path = ShapePath::polygon(vec![[0.0, 0.0], [3.0, 0.0], [3.0, 3.0]], false);
        open.opacity = 1.0;
        assert_eq!(
            composite_alpha(&[open], [1.0, 1.0], 0.2),
            MaskAlpha::Known(0.2)
        );
    }

    #[test]
    fn add_mode_is_the_default_and_accumulates() {
        let created = Mask::new(PropertyId::new(1), "New");
        assert_eq!(created.mode, MaskMode::Add);
        assert!(created.try_keyframe_mode().is_err());
        assert!(created.try_express_mode().is_err());
        let lower = rect_mask(1, [0.0, 0.0], [4.0, 4.0], MaskMode::Add);
        let upper = rect_mask(2, [1.0, 1.0], [5.0, 5.0], MaskMode::Add);
        assert_eq!(
            composite_alpha(&[lower, upper], [2.0, 2.0], 0.0),
            MaskAlpha::Known(2.0)
        );
    }

    #[test]
    fn subtract_mode_cuts_a_hole_and_cannot_be_keyed() {
        let lower = rect_mask(1, [0.0, 0.0], [6.0, 6.0], MaskMode::Add);
        let hole = rect_mask(2, [2.0, 2.0], [4.0, 4.0], MaskMode::Subtract);
        assert!(hole.try_keyframe_mode().is_err());
        assert_eq!(
            composite_alpha(&[lower.clone(), hole.clone()], [3.0, 3.0], 0.0),
            MaskAlpha::Known(0.0)
        );
        assert_eq!(
            composite_alpha(&[lower, hole], [1.0, 1.0], 0.0),
            MaskAlpha::Known(1.0)
        );
    }

    #[test]
    fn intersect_accumulates_overlap_and_leaves_the_rest_unconfirmed() {
        let lower = rect_mask(1, [0.0, 0.0], [4.0, 4.0], MaskMode::Add);
        let upper = rect_mask(2, [2.0, 0.0], [6.0, 4.0], MaskMode::Intersect);
        assert!(upper.try_keyframe_mode().is_err());
        assert_eq!(
            composite_alpha(&[lower.clone(), upper.clone()], [3.0, 2.0], 0.0),
            MaskAlpha::Known(2.0)
        );
        assert_eq!(
            composite_alpha(&[lower, upper], [1.0, 2.0], 0.0),
            MaskAlpha::Unconfirmed
        );
    }

    #[test]
    fn difference_matches_a_solo_mask_outside_the_overlap() {
        let lower = rect_mask(1, [0.0, 0.0], [4.0, 4.0], MaskMode::Add);
        let upper = rect_mask(2, [2.0, 0.0], [6.0, 4.0], MaskMode::Difference);
        assert!(upper.try_keyframe_mode().is_err());
        assert_eq!(
            composite_alpha(&[lower.clone(), upper.clone()], [5.0, 2.0], 0.0),
            MaskAlpha::Known(1.0)
        );
        assert_eq!(
            composite_alpha(&[lower, upper], [3.0, 2.0], 0.0),
            MaskAlpha::Known(0.0)
        );
    }

    #[test]
    fn none_mode_keeps_alpha_and_the_path() {
        let mask = rect_mask(1, [0.0, 0.0], [4.0, 4.0], MaskMode::None);
        assert!(mask.try_express_mode().is_err());
        assert_eq!(
            composite_alpha(&[mask.clone()], [1.0, 1.0], 0.4),
            MaskAlpha::Known(0.4)
        );
        assert_eq!(effect_input_paths(&[mask]).len(), 1);
    }

    #[test]
    fn per_vertex_feather_widths_and_smooth_falloff() {
        let mask = Mask::new(PropertyId::new(1), "Mask");
        assert_eq!(mask.feather_falloff, FeatherFalloff::Smooth);
        let mut points = vec![
            VertexFeather {
                position: 0.0,
                outer: 2.0,
                inner: None,
                tension: None,
                corner_angle: None,
                hold: Some(false),
            },
            VertexFeather {
                position: 10.0,
                outer: 8.0,
                inner: Some(3.0),
                tension: Some(0.5),
                corner_angle: None,
                hold: Some(false),
            },
        ];
        assert!(near(inner_feather_offset(&points[0]), 0.0));
        assert!(near(inner_feather_offset(&points[1]), 3.0));
        assert_ne!(points[0].outer, points[1].outer);
        assert!(near(feather_outer_at(&points, 5.0).unwrap(), 5.0));
        points[0].hold = Some(true);
        assert!(near(feather_outer_at(&points, 5.0).unwrap(), 2.0));
        assert!(near(stop_boundary_on_path(-2.0), 0.0));
        assert!(near(stop_boundary_on_path(4.0), 4.0));
        set_span_outer(&mut points, 0, 0, 6.0);
        assert!(near(points[0].outer, 6.0));
        assert!(near(points[1].outer, 8.0));
        assert!(near(advance_feather_position(9.5, 1.0, 10.0), 0.5));
        let info = vertex_feather_info(&points, 10.0, 1, FeatherFalloff::Smooth).unwrap();
        assert_eq!(info.count, 2);
        assert_eq!(info.tension, Some(0.5));
        assert_eq!(info.falloff, FeatherFalloff::Smooth);
    }
}
