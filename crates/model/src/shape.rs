use crate::id::PropertyId;
use crate::property::Property;

/// 座標は +x が右、+y が上。要確認の初期値と範囲は決め打ちしない。

/// 頂点と接線を持つパス。接線は頂点からの相対。
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ShapePath {
    pub vertices: Vec<[f64; 2]>,
    pub in_tangents: Vec<[f64; 2]>,
    pub out_tangents: Vec<[f64; 2]>,
    pub closed: bool,
    pub feather_radii: Vec<f64>,
    pub feather_seg_locs: Vec<i32>,
    pub feather_rel_seg_locs: Vec<f64>,
    pub feather_rel_corner_angles: Vec<f64>,
    pub feather_tensions: Vec<f64>,
    pub feather_types: Vec<i32>,
    pub feather_interps: Vec<i32>,
    pub kind: PathKind,
    pub vertex_kinds: Vec<VertexKind>,
    pub tensions: Vec<f64>,
    pub first_vertex: Option<usize>,
}

/// ベジェか、曲率を自動で決めるロトベジェか。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum PathKind {
    #[default]
    Bezier,
    RotoBezier,
}

/// スムーズ点とコーナー点。どちらが初期かは要確認。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VertexKind {
    Corner,
    Smooth,
}

impl ShapePath {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn polygon(vertices: Vec<[f64; 2]>, closed: bool) -> Self {
        let mut path = Self::default();
        let count = vertices.len();
        path.vertices = vertices;
        path.in_tangents = vec![[0.0, 0.0]; count];
        path.out_tangents = vec![[0.0, 0.0]; count];
        path.closed = closed;
        path
    }

    pub fn topmost_vertex(&self) -> Option<usize> {
        let mut best: Option<(usize, f64)> = None;
        for (index, vertex) in self.vertices.iter().enumerate() {
            match best {
                Some((_, y)) if vertex[1] <= y => {}
                _ => best = Some((index, vertex[1])),
            }
        }
        best.map(|(index, _)| index)
    }

    pub fn resolved_first(&self) -> Option<usize> {
        let count = self.vertices.len();
        if count == 0 {
            return None;
        }
        Some(match self.first_vertex {
            Some(index) if index < count => index,
            _ => self.topmost_vertex().unwrap_or(0),
        })
    }

    /// 第1頂点からパス順に振った番号。
    pub fn vertex_order(&self) -> Vec<usize> {
        let count = self.vertices.len();
        let Some(first) = self.resolved_first() else {
            return Vec::new();
        };
        if self.closed {
            (0..count).map(|step| (first + step) % count).collect()
        } else {
            let mut order: Vec<usize> = (first..count).collect();
            order.extend(0..first);
            order
        }
    }

    pub fn set_first_vertex(&mut self, index: usize) -> bool {
        if index >= self.vertices.len() {
            return false;
        }
        self.first_vertex = Some(index);
        true
    }

    /// 第1頂点だけ、表示が他より大きい。拡大率そのものは要確認。
    pub fn vertex_is_enlarged(&self, index: usize) -> bool {
        self.resolved_first() == Some(index)
    }

    pub fn insert_on_segment(
        &mut self,
        segment: usize,
        t: f64,
        position: Option<[f64; 2]>,
    ) -> bool {
        let count = self.vertices.len();
        if count == 0 || segment >= count {
            return false;
        }
        let next = if self.closed {
            (segment + 1) % count
        } else if segment + 1 < count {
            segment + 1
        } else {
            return false;
        };
        let on_segment = lerp2(self.vertices[segment], self.vertices[next], t);
        let point = position.unwrap_or(on_segment);
        let at = segment + 1;
        self.vertices.insert(at, point);
        insert_pad(&mut self.in_tangents, count, at);
        insert_pad(&mut self.out_tangents, count, at);
        if self.vertex_kinds.len() == count {
            self.vertex_kinds.insert(at, VertexKind::Corner);
        }
        if self.tensions.len() == count {
            self.tensions.insert(at, 0.0);
        }
        true
    }

    pub fn remove_vertex(&mut self, index: usize) -> bool {
        if index >= self.vertices.len() {
            return false;
        }
        self.vertices.remove(index);
        remove_pad(&mut self.in_tangents, index);
        remove_pad(&mut self.out_tangents, index);
        if index < self.vertex_kinds.len() {
            self.vertex_kinds.remove(index);
        }
        if index < self.tensions.len() {
            self.tensions.remove(index);
        }
        if self.first_vertex == Some(index) {
            self.first_vertex = None;
        }
        true
    }

    pub fn evaluated_tangents(&self) -> (Vec<[f64; 2]>, Vec<[f64; 2]>) {
        let count = self.vertices.len();
        if self.kind == PathKind::Bezier {
            return (
                fit_vec(&self.in_tangents, count),
                fit_vec(&self.out_tangents, count),
            );
        }
        let mut incoming = vec![[0.0, 0.0]; count];
        let mut outgoing = vec![[0.0, 0.0]; count];
        for index in 0..count {
            let tension = self.tensions.get(index).copied().unwrap_or(0.0);
            let current = self.vertices[index];
            let prev = self.vertices[neighbor(count, index, -1, self.closed)];
            let next = self.vertices[neighbor(count, index, 1, self.closed)];
            incoming[index] = scale2(sub2(prev, current), tension);
            outgoing[index] = scale2(sub2(next, current), tension);
        }
        (incoming, outgoing)
    }

    /// 手動ベジェからロトベジェへ。テンションの初期値は要確認なので引数で渡す。
    pub fn convert_to_rotobezier(&mut self, tension: f64) {
        let count = self.vertices.len();
        self.kind = PathKind::RotoBezier;
        self.tensions = vec![tension; count];
    }

    /// ロトベジェからベジェへ。評価した形を接線に焼き、形は変えない。
    pub fn convert_to_bezier(&mut self) {
        let (incoming, outgoing) = self.evaluated_tangents();
        self.in_tangents = incoming;
        self.out_tangents = outgoing;
        self.kind = PathKind::Bezier;
    }

    /// ロトベジェの曲線セグメントをドラッグすると、両端の頂点も動く。
    pub fn drag_curve_segment(&mut self, segment: usize, delta: [f64; 2]) -> bool {
        if self.kind != PathKind::RotoBezier {
            return false;
        }
        let count = self.vertices.len();
        if segment >= count {
            return false;
        }
        let next = if self.closed {
            (segment + 1) % count
        } else if segment + 1 < count {
            segment + 1
        } else {
            return false;
        };
        self.vertices[segment] = add2(self.vertices[segment], delta);
        self.vertices[next] = add2(self.vertices[next], delta);
        true
    }

    pub fn set_tension(&mut self, index: usize, tension: f64) -> bool {
        if index >= self.vertices.len() {
            return false;
        }
        if self.tensions.len() < self.vertices.len() {
            self.tensions.resize(self.vertices.len(), 0.0);
        }
        self.tensions[index] = tension;
        true
    }

    fn ensure_vertex_kinds(&mut self) {
        if self.vertex_kinds.len() == self.vertices.len() {
            return;
        }
        self.vertex_kinds = (0..self.vertices.len())
            .map(|index| {
                let incoming = self.in_tangents.get(index).copied().unwrap_or([0.0, 0.0]);
                let outgoing = self.out_tangents.get(index).copied().unwrap_or([0.0, 0.0]);
                if incoming == [0.0, 0.0] && outgoing == [0.0, 0.0] {
                    VertexKind::Corner
                } else {
                    VertexKind::Smooth
                }
            })
            .collect();
    }

    pub fn ctrl_alt_click(&mut self, index: usize) -> Option<VertexKind> {
        if index >= self.vertices.len() {
            return None;
        }
        self.ensure_vertex_kinds();
        let next = match self.vertex_kinds[index] {
            VertexKind::Corner => VertexKind::Smooth,
            VertexKind::Smooth => VertexKind::Corner,
        };
        self.vertex_kinds[index] = next;
        if next == VertexKind::Corner {
            self.ensure_tangent_slots();
            self.in_tangents[index] = [0.0, 0.0];
            self.out_tangents[index] = [0.0, 0.0];
        }
        Some(next)
    }

    pub fn ctrl_alt_drag(&mut self, index: usize, handle: [f64; 2]) -> bool {
        if index >= self.vertices.len() {
            return false;
        }
        self.ensure_vertex_kinds();
        self.ensure_tangent_slots();
        self.vertex_kinds[index] = VertexKind::Smooth;
        self.out_tangents[index] = handle;
        self.in_tangents[index] = [-handle[0], -handle[1]];
        true
    }

    fn ensure_tangent_slots(&mut self) {
        let count = self.vertices.len();
        self.in_tangents.resize(count, [0.0, 0.0]);
        self.out_tangents.resize(count, [0.0, 0.0]);
    }

    pub fn contains(&self, point: [f64; 2], rule: FillRule) -> bool {
        if !self.closed {
            return false;
        }
        match rule {
            FillRule::NonZero => winding_number(&self.vertices, true, point) != 0,
            FillRule::EvenOdd => ray_crossings(&self.vertices, true, point) % 2 == 1,
        }
    }

    /// 外が正、内が負。
    pub fn signed_distance(&self, point: [f64; 2]) -> f64 {
        let distance = polyline_distance(&self.vertices, self.closed, point);
        if self.closed && winding_number(&self.vertices, true, point) != 0 {
            -distance
        } else {
            distance
        }
    }
}

/// 環境設定を変えないとき、頂点追加は全キーに残り、形が変わるのはその時刻だけ。
pub fn add_vertex_across_duration(
    keys: &mut [ShapePath],
    key_index: usize,
    segment: usize,
    t: f64,
    position_at_time: [f64; 2],
) -> bool {
    if key_index >= keys.len() {
        return false;
    }
    for (index, key) in keys.iter_mut().enumerate() {
        let position = if index == key_index {
            Some(position_at_time)
        } else {
            None
        };
        if !key.insert_on_segment(segment, t, position) {
            return false;
        }
    }
    true
}

pub fn delete_vertex_across_duration(keys: &mut [ShapePath], vertex: usize) -> bool {
    if keys.is_empty() {
        return false;
    }
    for key in keys.iter_mut() {
        if !key.remove_vertex(vertex) {
            return false;
        }
    }
    true
}

pub fn corresponding_pairs(start: &ShapePath, end: &ShapePath) -> Vec<(usize, usize)> {
    let left = start.vertex_order();
    let right = end.vertex_order();
    let count = left.len().min(right.len());
    (0..count)
        .map(|index| (left[index], right[index]))
        .collect()
}

pub fn interpolate_paired(start: &ShapePath, end: &ShapePath, t: f64) -> Vec<[f64; 2]> {
    corresponding_pairs(start, end)
        .into_iter()
        .map(|(left, right)| lerp2(start.vertices[left], end.vertices[right], t))
        .collect()
}

pub fn transform_points(
    points: &[[f64; 2]],
    anchor: [f64; 2],
    translation: [f64; 2],
    rotation_clockwise_deg: f64,
    scale: [f64; 2],
) -> Vec<[f64; 2]> {
    let radians = rotation_clockwise_deg.to_radians();
    let (sin, cos) = radians.sin_cos();
    points
        .iter()
        .map(|point| {
            let x = (point[0] - anchor[0]) * scale[0];
            let y = (point[1] - anchor[1]) * scale[1];
            let rotated_x = cos * x + sin * y;
            let rotated_y = -sin * x + cos * y;
            [
                rotated_x + anchor[0] + translation[0],
                rotated_y + anchor[1] + translation[1],
            ]
        })
        .collect()
}

pub fn lerp_points(start: &[[f64; 2]], end: &[[f64; 2]], t: f64) -> Vec<[f64; 2]> {
    let count = start.len().min(end.len());
    (0..count)
        .map(|index| lerp2(start[index], end[index], t))
        .collect()
}

pub fn bbox_center(points: &[[f64; 2]]) -> Option<[f64; 2]> {
    if points.is_empty() {
        return None;
    }
    let mut min = points[0];
    let mut max = points[0];
    for point in points.iter().skip(1) {
        min[0] = min[0].min(point[0]);
        min[1] = min[1].min(point[1]);
        max[0] = max[0].max(point[0]);
        max[1] = max[1].max(point[1]);
    }
    Some([(min[0] + max[0]) * 0.5, (min[1] + max[1]) * 0.5])
}

pub fn scale_from_center(points: &[[f64; 2]], scale: [f64; 2]) -> Vec<[f64; 2]> {
    let center = bbox_center(points).unwrap_or([0.0, 0.0]);
    transform_points(points, center, [0.0, 0.0], 0.0, scale)
}

/// 選択頂点の移動。拡大率 1 で 1 ピクセル、Shift で 10 ピクセル。
pub fn arrow_nudge(shift: bool, zoom: f64) -> f64 {
    let screen_pixels = if shift { 10.0 } else { 1.0 };
    if zoom > 0.0 {
        screen_pixels / zoom
    } else {
        screen_pixels
    }
}

/// 同じグループ内のパスをまとめる計算。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MergeMode {
    Merge,
    Add,
    Subtract,
    Intersect,
    ExcludeIntersections,
}

/// 塗りの内側判定。初期は非ゼロワインディング。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum FillRule {
    #[default]
    NonZero,
    EvenOdd,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LineCap {
    Butt,
    Round,
    Projecting,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LineJoin {
    Miter,
    Round,
    Bevel,
}

/// もう一方の UI ラベルは要確認。初期の描画はタイムラインの下から上。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum PaintComposite {
    #[default]
    BottomToTop,
    AbovePreviousInSameGroup,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FillPaintKind {
    None,
    Solid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ToolPaintKind {
    None,
    Solid,
    LinearGradient,
    RadialGradient,
}

impl ToolPaintKind {
    pub fn cycle(self) -> Self {
        match self {
            Self::None => Self::Solid,
            Self::Solid => Self::LinearGradient,
            Self::LinearGradient => Self::RadialGradient,
            Self::RadialGradient => Self::None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SolidPaint {
    pub color: Option<[f64; 4]>,
    pub rule: FillRule,
    pub composite: PaintComposite,
    pub opacity: Option<f64>,
    pub kind: FillPaintKind,
}

impl SolidPaint {
    pub fn new() -> Self {
        Self {
            color: None,
            rule: FillRule::NonZero,
            composite: PaintComposite::BottomToTop,
            opacity: None,
            kind: FillPaintKind::Solid,
        }
    }

    pub fn set_color(&mut self, color: [f64; 4]) {
        self.color = Some(clip_color(color));
    }
}

impl Default for SolidPaint {
    fn default() -> Self {
        Self::new()
    }
}

pub fn clip_unit(value: f64) -> f64 {
    value.clamp(0.0, 1.0)
}

pub fn clip_color(color: [f64; 4]) -> [f64; 4] {
    [
        clip_unit(color[0]),
        clip_unit(color[1]),
        clip_unit(color[2]),
        clip_unit(color[3]),
    ]
}

pub fn fill_contains(rule: FillRule, polygons: &[Vec<[f64; 2]>], point: [f64; 2]) -> bool {
    match rule {
        FillRule::NonZero => {
            polygons
                .iter()
                .map(|polygon| winding_number(polygon, true, point))
                .sum::<i32>()
                != 0
        }
        FillRule::EvenOdd => {
            polygons
                .iter()
                .map(|polygon| ray_crossings(polygon, true, point))
                .sum::<u32>()
                % 2
                == 1
        }
    }
}

/// タイムライン上側が手前になる初期順を、合成の指定が上書きする。
pub fn front_paint(composites: &[PaintComposite]) -> Option<usize> {
    if composites.is_empty() {
        return None;
    }
    let mut draw: Vec<usize> = (0..composites.len()).rev().collect();
    for index in 0..composites.len() {
        if composites[index] != PaintComposite::AbovePreviousInSameGroup || index == 0 {
            continue;
        }
        let previous = index - 1;
        draw.retain(|item| *item != index);
        let Some(position) = draw.iter().position(|item| *item == previous) else {
            continue;
        };
        draw.insert(position + 1, index);
    }
    draw.last().copied()
}

pub fn swatch<T: PartialEq + Clone>(values: &[T]) -> Swatch<T> {
    let Some(first) = values.first() else {
        return Swatch::Empty;
    };
    if values.iter().all(|value| value == first) {
        Swatch::Value(first.clone())
    } else {
        Swatch::Mixed
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Swatch<T> {
    Empty,
    Value(T),
    Mixed,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StrokeStyle {
    pub color: Option<[f64; 4]>,
    pub opacity: Option<f64>,
    pub width: Option<f64>,
    pub cap: Option<LineCap>,
    pub join: Option<LineJoin>,
    pub miter_limit: Option<f64>,
    pub dashes: Vec<f64>,
    pub gaps: Vec<f64>,
    pub offset: f64,
    pub offset_stopwatch: bool,
    pub taper_length_unit: Option<LengthUnit>,
    pub taper_start_length: Option<f64>,
    pub taper_end_length: Option<f64>,
    pub taper_start_width: Option<f64>,
    pub taper_end_width: Option<f64>,
    pub taper_start_ease: Option<f64>,
    pub taper_end_ease: Option<f64>,
    pub wave_amount: Option<f64>,
    pub wave_unit: Option<WaveUnit>,
    pub wavelength: Option<f64>,
    pub composite: PaintComposite,
}

impl StrokeStyle {
    pub fn new() -> Self {
        Self {
            color: None,
            opacity: None,
            width: None,
            cap: None,
            join: None,
            miter_limit: None,
            dashes: Vec::new(),
            gaps: Vec::new(),
            offset: 0.0,
            offset_stopwatch: false,
            taper_length_unit: None,
            taper_start_length: None,
            taper_end_length: None,
            taper_start_width: None,
            taper_end_width: None,
            taper_start_ease: None,
            taper_end_ease: None,
            wave_amount: None,
            wave_unit: None,
            wavelength: None,
            composite: PaintComposite::BottomToTop,
        }
    }

    pub fn set_color(&mut self, color: [f64; 4]) {
        self.color = Some(clip_color(color));
    }

    pub fn try_push_dash(&mut self, dash: f64, gap: f64) -> Result<(), DashLimit> {
        if self.dashes.len() >= 3 {
            return Err(DashLimit);
        }
        self.dashes.push(dash);
        self.gaps.push(gap);
        Ok(())
    }

    pub fn arm_offset_stopwatch(&mut self) {
        self.offset_stopwatch = true;
    }
}

impl Default for StrokeStyle {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DashLimit;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LengthUnit {
    Percent,
    Pixels,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WaveUnit {
    Pixels,
    Cycles,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TaperProfile {
    Sharp,
    Straight,
    Round,
}

pub fn cap_extension(cap: LineCap, stroke_width: f64) -> f64 {
    match cap {
        LineCap::Butt => 0.0,
        LineCap::Round | LineCap::Projecting => stroke_width,
    }
}

pub fn miter_tip_length(stroke_width: f64, interior_angle_radians: f64) -> f64 {
    let sine = (interior_angle_radians * 0.5).sin();
    if sine.abs() < 1e-12 {
        f64::INFINITY
    } else {
        stroke_width / sine
    }
}

pub fn miter_becomes_bevel(
    join: LineJoin,
    miter_limit: f64,
    stroke_width: f64,
    interior_angle_radians: f64,
) -> bool {
    match join {
        LineJoin::Bevel => true,
        LineJoin::Round => false,
        LineJoin::Miter => {
            miter_tip_length(stroke_width, interior_angle_radians) >= miter_limit * stroke_width
        }
    }
}

pub fn taper_profile(ease: f64) -> TaperProfile {
    if ease == 0.0 {
        TaperProfile::Straight
    } else if ease < 0.0 {
        TaperProfile::Sharp
    } else {
        TaperProfile::Round
    }
}

pub fn taper_straight_width(start_width: f64, end_width: f64, t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    start_width + (end_width - start_width) * t
}

/// パーセントの 50 はパスの中央。単位そのものの初期値は要確認。
pub fn taper_length_along(unit: LengthUnit, value: f64, path_length: f64) -> f64 {
    match unit {
        LengthUnit::Percent => path_length * value / 100.0,
        LengthUnit::Pixels => value,
    }
}

/// 周期のとき、波長 1 で波が 1 つ。
pub fn wave_count(unit: WaveUnit, wavelength: f64, path_length: f64) -> Option<f64> {
    match unit {
        WaveUnit::Cycles => Some(wavelength),
        WaveUnit::Pixels if wavelength != 0.0 => Some(path_length / wavelength),
        WaveUnit::Pixels => None,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ColorStop {
    pub offset: f64,
    pub color: [f64; 4],
}

#[derive(Clone, Debug, PartialEq)]
pub struct OpacityStop {
    pub offset: f64,
    pub opacity: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GradientGeometry {
    pub start: [f64; 2],
    pub end: [f64; 2],
    pub color_stops: Vec<ColorStop>,
    pub opacity_stops: Vec<OpacityStop>,
    pub highlight_length: Option<f64>,
    pub highlight_angle: Option<f64>,
    pub color_keys: Vec<(f64, Vec<ColorStop>)>,
}

impl GradientGeometry {
    pub fn at_layer_center(center: [f64; 2]) -> Self {
        Self {
            start: center,
            end: center,
            color_stops: Vec::new(),
            opacity_stops: Vec::new(),
            highlight_length: None,
            highlight_angle: None,
            color_keys: Vec::new(),
        }
    }

    pub fn push_color_key(&mut self, time_seconds: f64, stops: Vec<ColorStop>) {
        self.color_keys.push((time_seconds, stops));
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GradientFill {
    pub geometry: GradientGeometry,
    pub rule: FillRule,
    pub composite: PaintComposite,
    pub opacity: Option<f64>,
}

impl GradientFill {
    pub fn at_layer_center(center: [f64; 2]) -> Self {
        Self {
            geometry: GradientGeometry::at_layer_center(center),
            rule: FillRule::NonZero,
            composite: PaintComposite::BottomToTop,
            opacity: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GradientStroke {
    pub geometry: GradientGeometry,
    pub stroke: StrokeStyle,
}

impl GradientStroke {
    pub fn at_layer_center(center: [f64; 2]) -> Self {
        Self {
            geometry: GradientGeometry::at_layer_center(center),
            stroke: StrokeStyle::new(),
        }
    }
}

pub fn sample_linear(geometry: &GradientGeometry, point: [f64; 2]) -> [f64; 4] {
    let delta = sub2(geometry.end, geometry.start);
    let length_sq = dot2(delta, delta);
    let t = if length_sq == 0.0 {
        0.0
    } else {
        (dot2(sub2(point, geometry.start), delta) / length_sq).clamp(0.0, 1.0)
    };
    sample_stops(&geometry.color_stops, t)
}

/// 円形は開始点を中心、終了点を円周にする。ハイライト角の単位は要確認のため中心は開始点のまま。
pub fn sample_radial(geometry: &GradientGeometry, point: [f64; 2]) -> [f64; 4] {
    let radius = len2(sub2(geometry.end, geometry.start));
    let t = if radius == 0.0 {
        0.0
    } else {
        (len2(sub2(point, geometry.start)) / radius).clamp(0.0, 1.0)
    };
    sample_stops(&geometry.color_stops, t)
}

pub fn stroke_covers(vertices: &[[f64; 2]], closed: bool, width: f64, point: [f64; 2]) -> bool {
    polyline_distance(vertices, closed, point) <= width * 0.5
}

pub fn merge_contains(mode: MergeMode, polygons: &[Vec<[f64; 2]>], point: [f64; 2]) -> bool {
    match mode {
        MergeMode::Merge => {
            polygons
                .iter()
                .map(|polygon| winding_number(polygon, true, point))
                .sum::<i32>()
                != 0
        }
        MergeMode::Add => polygons
            .iter()
            .any(|polygon| winding_number(polygon, true, point) != 0),
        MergeMode::Subtract => {
            let Some(top) = polygons.first() else {
                return false;
            };
            winding_number(top, true, point) != 0
                && polygons
                    .iter()
                    .skip(1)
                    .all(|polygon| winding_number(polygon, true, point) == 0)
        }
        MergeMode::Intersect => {
            !polygons.is_empty()
                && polygons
                    .iter()
                    .all(|polygon| winding_number(polygon, true, point) != 0)
        }
        MergeMode::ExcludeIntersections => {
            let count = polygons
                .iter()
                .filter(|polygon| winding_number(polygon, true, point) != 0)
                .count();
            count >= 1 && count != polygons.len()
        }
    }
}

pub fn offset_closed(vertices: &[[f64; 2]], amount: f64) -> Vec<[f64; 2]> {
    let count = vertices.len();
    if count < 3 {
        return vertices.to_vec();
    }
    (0..count)
        .map(|index| {
            let previous = vertices[(index + count - 1) % count];
            let current = vertices[index];
            let next = vertices[(index + 1) % count];
            let mut normal_in = right_normal(sub2(current, previous));
            let mut normal_out = right_normal(sub2(next, current));
            if len2(normal_in) == 0.0 {
                normal_in = [0.0, 0.0];
            } else {
                normal_in = scale2(normal_in, 1.0 / len2(normal_in));
            }
            if len2(normal_out) == 0.0 {
                normal_out = [0.0, 0.0];
            } else {
                normal_out = scale2(normal_out, 1.0 / len2(normal_out));
            }
            let summed = add2(normal_in, normal_out);
            let outward = if len2(summed) == 0.0 {
                normal_out
            } else {
                scale2(summed, 1.0 / len2(summed))
            };
            let corner = (normal_in[0] * outward[0] + normal_in[1] * outward[1])
                .abs()
                .max(1e-6);
            add2(current, scale2(outward, amount / corner))
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq)]
pub struct OffsetPaths {
    pub amount: f64,
    pub join: Option<LineJoin>,
    pub miter_limit: Option<f64>,
    pub copies: u32,
    pub copy_offset: f64,
    pub copy_offset_stopwatch: bool,
}

impl OffsetPaths {
    pub fn new(amount: f64) -> Self {
        Self {
            amount,
            join: None,
            miter_limit: None,
            copies: 1,
            copy_offset: 0.0,
            copy_offset_stopwatch: false,
        }
    }

    pub fn arm_copy_offset_stopwatch(&mut self) {
        self.copy_offset_stopwatch = true;
    }

    pub fn polygons(&self, source: &[[f64; 2]]) -> Vec<Vec<[f64; 2]>> {
        (0..self.copies)
            .map(|index| offset_closed(source, self.amount + f64::from(index) * self.copy_offset))
            .collect()
    }
}

/// 符号とプロパティ名は要確認。頂点が外か内かだけを切り替える。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PuckerBloat {
    VerticesOutSegmentsIn,
    VerticesInSegmentsOut,
}

impl PuckerBloat {
    pub fn switched(self) -> Self {
        match self {
            Self::VerticesOutSegmentsIn => Self::VerticesInSegmentsOut,
            Self::VerticesInSegmentsOut => Self::VerticesOutSegmentsIn,
        }
    }
}

pub fn apply_pucker_bloat(
    vertices: &[[f64; 2]],
    state: PuckerBloat,
    magnitude: f64,
) -> Vec<[f64; 2]> {
    let Some(center) = bbox_center(vertices) else {
        return Vec::new();
    };
    let sign = match state {
        PuckerBloat::VerticesOutSegmentsIn => 1.0,
        PuckerBloat::VerticesInSegmentsOut => -1.0,
    };
    let mut output = Vec::new();
    for (index, vertex) in vertices.iter().enumerate() {
        output.push(push_from(center, *vertex, magnitude * sign));
        let next = vertices[(index + 1) % vertices.len()];
        let midpoint = lerp2(*vertex, next, 0.5);
        output.push(push_from(center, midpoint, -magnitude * sign));
    }
    output
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RepeaterComposite {
    InFront,
    Behind,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Repeater {
    pub copies: u32,
    pub offset: f64,
    pub position: [f64; 2],
    pub start_opacity: Option<f64>,
    pub end_opacity: Option<f64>,
    pub composite: Option<RepeaterComposite>,
}

impl Repeater {
    pub fn new(copies: u32) -> Self {
        Self {
            copies,
            offset: 0.0,
            position: [0.0, 0.0],
            start_opacity: None,
            end_opacity: None,
            composite: None,
        }
    }
}

pub fn repeater_copy_position(index: u32, offset: f64, step: [f64; 2]) -> [f64; 2] {
    let times = f64::from(index) + offset;
    [step[0] * times, step[1] * times]
}

pub fn repeater_copy_opacity(index: u32, copies: u32, start: f64, end: f64) -> f64 {
    if copies <= 1 {
        return start;
    }
    let t = f64::from(index) / (f64::from(copies) - 1.0);
    start + (end - start) * t
}

pub fn repeater_paints_as_compound(kinds: &[ShapeKind]) -> bool {
    let Some(repeater) = kinds.iter().position(|kind| *kind == ShapeKind::Repeater) else {
        return false;
    };
    let Some(paint) = kinds
        .iter()
        .position(|kind| matches!(content_class(*kind), ContentClass::Paint))
    else {
        return false;
    };
    repeater < paint
}

pub fn round_corners(vertices: &[[f64; 2]], closed: bool, radius: f64) -> Vec<[f64; 2]> {
    if !closed || vertices.len() < 3 || radius == 0.0 {
        return vertices.to_vec();
    }
    let count = vertices.len();
    let mut output = Vec::new();
    for index in 0..count {
        let previous = vertices[(index + count - 1) % count];
        let current = vertices[index];
        let next = vertices[(index + 1) % count];
        let to_previous = sub2(previous, current);
        let to_next = sub2(next, current);
        let previous_len = len2(to_previous);
        let next_len = len2(to_next);
        if previous_len < 1e-9 || next_len < 1e-9 {
            output.push(current);
            continue;
        }
        let distance = radius.min(previous_len * 0.5).min(next_len * 0.5);
        output.push(add2(current, scale2(to_previous, distance / previous_len)));
        output.push(add2(current, scale2(to_next, distance / next_len)));
    }
    output
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MultiTrim {
    Simultaneous,
    Individual,
}

pub fn path_length(vertices: &[[f64; 2]], closed: bool) -> f64 {
    edge_lengths(vertices, closed).into_iter().sum()
}

pub fn trim_polyline(
    vertices: &[[f64; 2]],
    closed: bool,
    start: f64,
    end: f64,
    offset: f64,
    first_vertex: usize,
) -> Vec<[f64; 2]> {
    if vertices.len() < 2 {
        return vertices.to_vec();
    }
    let rotated = rotate_start(vertices, closed, first_vertex);
    let length = path_length(&rotated, closed);
    if length == 0.0 {
        return rotated;
    }
    let from = (start + offset).clamp(0.0, length);
    let to = (end + offset).clamp(0.0, length);
    if to <= from {
        return vec![point_along(&rotated, closed, from)];
    }
    subpolyline(&rotated, closed, from, to)
}

pub fn trim_many(
    paths: &[Vec<[f64; 2]>],
    start: f64,
    end: f64,
    mode: MultiTrim,
) -> Vec<Vec<[f64; 2]>> {
    match mode {
        MultiTrim::Simultaneous => paths
            .iter()
            .map(|path| trim_polyline(path, false, start, end, 0.0, 0))
            .collect(),
        MultiTrim::Individual => {
            let mut lengths = Vec::new();
            let mut cursor = 0.0;
            for path in paths {
                let length = path_length(path, false);
                lengths.push((cursor, cursor + length));
                cursor += length;
            }
            paths
                .iter()
                .zip(lengths)
                .map(|(path, (from, to))| {
                    let local_start = (start - from).clamp(0.0, to - from);
                    let local_end = (end - from).clamp(0.0, to - from);
                    trim_polyline(path, false, local_start, local_end, 0.0, 0)
                })
                .collect()
        }
    }
}

pub fn twist_points(vertices: &[[f64; 2]], amount_degrees: f64) -> Vec<[f64; 2]> {
    let Some(center) = bbox_center(vertices) else {
        return Vec::new();
    };
    let max_distance = vertices
        .iter()
        .map(|vertex| len2(sub2(*vertex, center)))
        .fold(0.0_f64, f64::max);
    if max_distance == 0.0 {
        return vertices.to_vec();
    }
    vertices
        .iter()
        .map(|vertex| {
            let offset = sub2(*vertex, center);
            let distance = len2(offset);
            let factor = 1.0 - distance / max_distance;
            let radians = (amount_degrees * factor).to_radians();
            let (sin, cos) = radians.sin_cos();
            let x = cos * offset[0] + sin * offset[1];
            let y = -sin * offset[0] + cos * offset[1];
            add2(center, [x, y])
        })
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ZigZagSpacing {
    Absolute(f64),
    Relative(f64),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ZigZagPoints {
    Smooth,
    Corner,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ZigZag {
    pub spacing: ZigZagSpacing,
    pub ridges_per_segment: u32,
    pub points: ZigZagPoints,
}

pub fn apply_zigzag(source: &ShapePath, zigzag: ZigZag) -> ShapePath {
    let mut vertices = Vec::new();
    let mut outgoing = Vec::new();
    let count = source.vertices.len();
    if count == 0 {
        return ShapePath::new();
    }
    let segments = if source.closed {
        count
    } else {
        count.saturating_sub(1)
    };
    for index in 0..segments {
        let start = source.vertices[index];
        let end = source.vertices[(index + 1) % count];
        vertices.push(start);
        outgoing.push([0.0, 0.0]);
        let chord = sub2(end, start);
        let length = len2(chord);
        if length == 0.0 || zigzag.ridges_per_segment == 0 {
            continue;
        }
        let step = match zigzag.spacing {
            ZigZagSpacing::Absolute(pixels) => pixels.abs().max(1e-6),
            ZigZagSpacing::Relative(fraction) => (length * fraction.abs()).max(1e-6),
        };
        let normal = scale2(left_normal(chord), 1.0 / length);
        for ridge in 0..zigzag.ridges_per_segment {
            let distance = step * f64::from(ridge + 1);
            if distance >= length {
                break;
            }
            let sign = if ridge % 2 == 0 { 1.0 } else { -1.0 };
            vertices.push(add2(
                lerp2(start, end, distance / length),
                scale2(normal, sign * step),
            ));
            outgoing.push(match zigzag.points {
                ZigZagPoints::Smooth => scale2(chord, 0.1),
                ZigZagPoints::Corner => [0.0, 0.0],
            });
        }
    }
    if !source.closed {
        vertices.push(*source.vertices.last().unwrap_or(&start_or_zero(source)));
        outgoing.push([0.0, 0.0]);
    }
    let mut path = ShapePath::polygon(vertices, source.closed);
    path.out_tangents = fit_vec(&outgoing, path.vertices.len());
    path
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WiggleSize {
    Absolute(f64),
    Relative(f64),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WigglePaths {
    pub size: WiggleSize,
    pub detail: f64,
    pub points: ZigZagPoints,
    pub correlation_percent: f64,
    pub wiggles_per_second: f64,
    pub temporal_phase: f64,
}

pub fn apply_wiggle_paths(source: &ShapePath, wiggle: WigglePaths, time: f64) -> ShapePath {
    let phase = wiggle.temporal_phase + wiggle.wiggles_per_second * time;
    let count = source.vertices.len();
    let mut vertices = Vec::with_capacity(count);
    for index in 0..count {
        let own = (f64::from(index as u32) * wiggle.detail + phase).sin();
        let shared = phase.sin();
        let independence = 1.0 - wiggle.correlation_percent / 100.0;
        let noise = shared + (own - shared) * independence;
        let previous = source.vertices[neighbor(count, index, -1, source.closed)];
        let next = source.vertices[neighbor(count, index, 1, source.closed)];
        let tangent = sub2(next, previous);
        let length = len2(tangent).max(1e-6);
        let normal = scale2(left_normal(tangent), 1.0 / length);
        let magnitude = match wiggle.size {
            WiggleSize::Absolute(pixels) => pixels,
            WiggleSize::Relative(fraction) => fraction * length,
        };
        vertices.push(add2(
            source.vertices[index],
            scale2(normal, noise * magnitude),
        ));
    }
    let mut path = ShapePath::polygon(vertices, source.closed);
    if wiggle.points == ZigZagPoints::Smooth {
        path.out_tangents = vec![[0.25, 0.0]; count];
    }
    path
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WiggleTransform {
    pub position: [f64; 2],
    pub anchor: [f64; 2],
    pub scale: [f64; 2],
    pub rotation_degrees: f64,
    pub correlation_percent: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WiggleSample {
    pub position: [f64; 2],
    pub anchor: [f64; 2],
    pub scale: [f64; 2],
    pub rotation_degrees: f64,
}

pub fn sample_wiggle_transform(
    wiggle: WiggleTransform,
    copy_index: u32,
    time: f64,
    repeater_precedes: bool,
) -> WiggleSample {
    let independence = if repeater_precedes {
        1.0 - wiggle.correlation_percent / 100.0
    } else {
        0.0
    };
    let wave = (time + f64::from(copy_index) * independence).sin();
    WiggleSample {
        position: [wiggle.position[0] * wave, wiggle.position[1] * wave],
        anchor: [wiggle.anchor[0] * wave, wiggle.anchor[1] * wave],
        scale: [wiggle.scale[0] * wave, wiggle.scale[1] * wave],
        rotation_degrees: wiggle.rotation_degrees * wave,
    }
}

/// グループの変換。タイムライン名が確認できたのはアンカーポイントと位置だけ。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroupTransform {
    pub anchor: [f64; 2],
    pub position: [f64; 2],
}

impl GroupTransform {
    pub fn origin() -> Self {
        Self {
            anchor: [0.0, 0.0],
            position: [0.0, 0.0],
        }
    }
}

pub fn group_transform_channels() -> &'static [&'static str] {
    &["anchor", "position"]
}

pub fn path_point_world(vertex: [f64; 2], transform: GroupTransform) -> [f64; 2] {
    [
        transform.position[0] + vertex[0] - transform.anchor[0],
        transform.position[1] + vertex[1] - transform.anchor[1],
    ]
}

pub struct ParametricRect {
    pub center: [f64; 2],
    pub size: [f64; 2],
    pub position: [f64; 2],
}

pub fn parametric_rect_to_bezier(source: ParametricRect) -> (ShapePath, GroupTransform) {
    let half_width = source.size[0] * 0.5;
    let half_height = source.size[1] * 0.5;
    let path = ShapePath::polygon(
        vec![
            [-half_width, -half_height],
            [half_width, -half_height],
            [half_width, half_height],
            [-half_width, half_height],
        ],
        true,
    );
    let transform = GroupTransform {
        anchor: [0.0, 0.0],
        position: source.center,
    };
    let _ = source.position;
    (path, transform)
}

/// シェイプコンテンツの種類。数値は子プロパティに置く。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShapeKind {
    Group,
    Path,
    Fill,
    Stroke,
    GradientFill,
    GradientStroke,
    MergePaths(MergeMode),
    OffsetPaths,
    PuckerAndBloat,
    Repeater,
    RoundCorners,
    TrimPaths,
    Twist,
    ZigZag,
    WigglePaths,
    WiggleTransform,
    Transform,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ContentClass {
    Group,
    Path,
    Paint,
    Operation,
    Repeater,
    Transform,
}

fn content_class(kind: ShapeKind) -> ContentClass {
    match kind {
        ShapeKind::Group => ContentClass::Group,
        ShapeKind::Path => ContentClass::Path,
        ShapeKind::Fill
        | ShapeKind::Stroke
        | ShapeKind::GradientFill
        | ShapeKind::GradientStroke => ContentClass::Paint,
        ShapeKind::Repeater => ContentClass::Repeater,
        ShapeKind::Transform => ContentClass::Transform,
        ShapeKind::MergePaths(_)
        | ShapeKind::OffsetPaths
        | ShapeKind::PuckerAndBloat
        | ShapeKind::RoundCorners
        | ShapeKind::TrimPaths
        | ShapeKind::Twist
        | ShapeKind::ZigZag
        | ShapeKind::WigglePaths
        | ShapeKind::WiggleTransform => ContentClass::Operation,
    }
}

/// シェイプレイヤーのコンテンツ木。
#[derive(Clone, Debug, PartialEq)]
pub struct ShapeContents {
    pub nodes: Vec<ShapeNode>,
}

impl ShapeContents {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }
}

impl Default for ShapeContents {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ShapePayload {
    Empty,
    Transform(GroupTransform),
    Path(ShapePath),
    Fill(SolidPaint),
    Stroke(StrokeStyle),
    GradientFill(GradientFill),
    GradientStroke(GradientStroke),
    Repeater(Repeater),
    Offset(OffsetPaths),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShapeNode {
    pub id: PropertyId,
    pub name: String,
    pub kind: ShapeKind,
    pub properties: Vec<Property>,
    pub children: Vec<ShapeNode>,
    pub excluded_from_drawing: bool,
    pub payload: ShapePayload,
}

impl ShapeNode {
    pub fn new(id: PropertyId, name: impl Into<String>, kind: ShapeKind) -> Self {
        Self {
            id,
            name: name.into(),
            kind,
            properties: Vec::new(),
            children: Vec::new(),
            excluded_from_drawing: false,
            payload: ShapePayload::Empty,
        }
    }

    pub fn group(id: PropertyId, transform_id: PropertyId, name: impl Into<String>) -> Self {
        let mut node = Self::new(id, name, ShapeKind::Group);
        let mut transform = Self::new(transform_id, "Transform", ShapeKind::Transform);
        transform.payload = ShapePayload::Transform(GroupTransform::origin());
        node.children.push(transform);
        node
    }

    pub fn insert_content(&mut self, child: ShapeNode, alt: bool) -> bool {
        if self.kind != ShapeKind::Group {
            return false;
        }
        let class = content_class(child.kind);
        let is_merge = matches!(child.kind, ShapeKind::MergePaths(_));
        let index = insertion_index(&self.children, class, alt);
        self.children.insert(index, child);
        if is_merge {
            for earlier in &mut self.children[..index] {
                if earlier.kind == ShapeKind::Path {
                    earlier.excluded_from_drawing = true;
                }
            }
            let paint_after = self.children[index + 1..]
                .iter()
                .any(|node| content_class(node.kind) == ContentClass::Paint);
            if !paint_after {
                let mut fill = ShapeNode::new(PropertyId::new(0), "Fill", ShapeKind::Fill);
                fill.payload = ShapePayload::Fill(SolidPaint::new());
                let mut stroke = ShapeNode::new(PropertyId::new(0), "Stroke", ShapeKind::Stroke);
                stroke.payload = ShapePayload::Stroke(StrokeStyle::new());
                self.children.insert(index + 1, stroke);
                self.children.insert(index + 1, fill);
            }
        }
        true
    }

    pub fn move_child(&mut self, from: usize, to: usize) -> bool {
        if self.kind != ShapeKind::Group || from >= self.children.len() || to >= self.children.len()
        {
            return false;
        }
        let child = self.children.remove(from);
        self.children.insert(to, child);
        true
    }

    pub fn child_kinds(&self) -> Vec<ShapeKind> {
        self.children.iter().map(|child| child.kind).collect()
    }
}

pub fn group_selection(
    id: PropertyId,
    transform_id: PropertyId,
    name: impl Into<String>,
    members: Vec<ShapeNode>,
) -> ShapeNode {
    let mut group = ShapeNode::group(id, transform_id, name);
    for member in members {
        let _ = group.insert_content(member, false);
    }
    group
}

pub fn ungroup(group: ShapeNode) -> Vec<ShapeNode> {
    group
        .children
        .into_iter()
        .filter(|child| child.kind != ShapeKind::Transform)
        .collect()
}

pub fn duplicate_group(
    group: &ShapeNode,
    translation: [f64; 2],
    rotation_clockwise_deg: f64,
    scale: [f64; 2],
) -> ShapeNode {
    let mut copy = group.clone();
    for child in &mut copy.children {
        if let ShapePayload::Transform(transform) = &mut child.payload {
            transform.position = add2(transform.position, translation);
        }
        if let ShapePayload::Path(path) = &mut child.payload {
            path.vertices = transform_points(
                &path.vertices,
                [0.0, 0.0],
                [0.0, 0.0],
                rotation_clockwise_deg,
                scale,
            );
        }
    }
    copy
}

/// 複数パスの文字から作るシェイプの初期計算は複合パス。
pub fn shape_from_character_paths(
    group_id: PropertyId,
    transform_id: PropertyId,
    paths: Vec<ShapePath>,
) -> ShapeNode {
    let mut group = ShapeNode::group(group_id, transform_id, "Character");
    for (index, path) in paths.into_iter().enumerate() {
        let mut node = ShapeNode::new(PropertyId::new(index as u64 + 1), "Path", ShapeKind::Path);
        node.payload = ShapePayload::Path(path);
        let _ = group.insert_content(node, false);
    }
    if group
        .children
        .iter()
        .filter(|child| child.kind == ShapeKind::Path)
        .count()
        > 1
    {
        let merge = ShapeNode::new(
            PropertyId::new(10_000),
            "Merge Paths",
            ShapeKind::MergePaths(MergeMode::Merge),
        );
        let _ = group.insert_content(merge, false);
    }
    group
}

fn insertion_index(children: &[ShapeNode], class: ContentClass, alt: bool) -> usize {
    let limit = if matches!(
        children.last().map(|child| child.kind),
        Some(ShapeKind::Transform)
    ) {
        children.len() - 1
    } else {
        children.len()
    };
    if class == ContentClass::Repeater || class == ContentClass::Transform {
        return limit;
    }
    let mut repeater_at = limit;
    while repeater_at > 0 && children[repeater_at - 1].kind == ShapeKind::Repeater {
        repeater_at -= 1;
    }
    if alt {
        return repeater_at;
    }
    let region = &children[..repeater_at];
    match class {
        ContentClass::Path | ContentClass::Group => region
            .iter()
            .rposition(|child| {
                matches!(
                    content_class(child.kind),
                    ContentClass::Path | ContentClass::Group
                )
            })
            .map(|index| index + 1)
            .unwrap_or(0),
        ContentClass::Paint => region
            .iter()
            .position(|child| content_class(child.kind) == ContentClass::Paint)
            .unwrap_or_else(|| {
                region
                    .iter()
                    .rposition(|child| {
                        matches!(
                            content_class(child.kind),
                            ContentClass::Path | ContentClass::Group
                        )
                    })
                    .map(|index| index + 1)
                    .unwrap_or(0)
            }),
        ContentClass::Operation => region
            .iter()
            .rposition(|child| content_class(child.kind) == ContentClass::Operation)
            .map(|index| index + 1)
            .unwrap_or_else(|| {
                region
                    .iter()
                    .rposition(|child| {
                        matches!(
                            content_class(child.kind),
                            ContentClass::Path | ContentClass::Group
                        )
                    })
                    .map(|index| index + 1)
                    .unwrap_or(0)
            }),
        ContentClass::Repeater | ContentClass::Transform => limit,
    }
}

fn start_or_zero(path: &ShapePath) -> [f64; 2] {
    path.vertices.first().copied().unwrap_or([0.0, 0.0])
}

fn push_from(center: [f64; 2], point: [f64; 2], distance: f64) -> [f64; 2] {
    let offset = sub2(point, center);
    let length = len2(offset);
    if length == 0.0 {
        point
    } else {
        add2(point, scale2(offset, distance / length))
    }
}

fn rotate_start(vertices: &[[f64; 2]], closed: bool, first: usize) -> Vec<[f64; 2]> {
    if vertices.is_empty() || !closed {
        return vertices.to_vec();
    }
    let first = first % vertices.len();
    vertices[first..]
        .iter()
        .chain(vertices[..first].iter())
        .copied()
        .collect()
}

fn edge_lengths(vertices: &[[f64; 2]], closed: bool) -> Vec<f64> {
    let count = vertices.len();
    if count < 2 {
        return Vec::new();
    }
    let edges = if closed { count } else { count - 1 };
    (0..edges)
        .map(|index| len2(sub2(vertices[(index + 1) % count], vertices[index])))
        .collect()
}

fn point_along(vertices: &[[f64; 2]], closed: bool, distance: f64) -> [f64; 2] {
    let mut remaining = distance;
    let count = vertices.len();
    let edges = if closed { count } else { count - 1 };
    for index in 0..edges {
        let start = vertices[index];
        let end = vertices[(index + 1) % count];
        let length = len2(sub2(end, start));
        if remaining <= length || index + 1 == edges {
            let t = if length == 0.0 {
                0.0
            } else {
                (remaining / length).clamp(0.0, 1.0)
            };
            return lerp2(start, end, t);
        }
        remaining -= length;
    }
    vertices.last().copied().unwrap_or([0.0, 0.0])
}

fn subpolyline(vertices: &[[f64; 2]], closed: bool, from: f64, to: f64) -> Vec<[f64; 2]> {
    let mut output = vec![point_along(vertices, closed, from)];
    let mut walked = 0.0;
    let count = vertices.len();
    let edges = if closed { count } else { count - 1 };
    for index in 0..edges {
        let start = vertices[index];
        let end = vertices[(index + 1) % count];
        let length = len2(sub2(end, start));
        let next = walked + length;
        if next > from && walked < to && next < to {
            output.push(end);
        }
        walked = next;
    }
    output.push(point_along(vertices, closed, to));
    output
}

fn sample_stops(stops: &[ColorStop], t: f64) -> [f64; 4] {
    if stops.is_empty() {
        return [0.0, 0.0, 0.0, 0.0];
    }
    if stops.len() == 1 || t <= stops[0].offset {
        return clip_color(stops[0].color);
    }
    for pair in stops.windows(2) {
        let left = &pair[0];
        let right = &pair[1];
        if t <= right.offset {
            let span = right.offset - left.offset;
            let local = if span == 0.0 {
                0.0
            } else {
                (t - left.offset) / span
            };
            return clip_color([
                left.color[0] + (right.color[0] - left.color[0]) * local,
                left.color[1] + (right.color[1] - left.color[1]) * local,
                left.color[2] + (right.color[2] - left.color[2]) * local,
                left.color[3] + (right.color[3] - left.color[3]) * local,
            ]);
        }
    }
    clip_color(stops[stops.len() - 1].color)
}

fn neighbor(count: usize, index: usize, step: isize, closed: bool) -> usize {
    if count == 0 {
        return 0;
    }
    if closed {
        (index as isize + step).rem_euclid(count as isize) as usize
    } else {
        (index as isize + step).clamp(0, count as isize - 1) as usize
    }
}

fn insert_pad(values: &mut Vec<[f64; 2]>, previous_len: usize, at: usize) {
    if values.len() == previous_len {
        values.insert(at, [0.0, 0.0]);
    }
}

fn remove_pad(values: &mut Vec<[f64; 2]>, index: usize) {
    if index < values.len() {
        values.remove(index);
    }
}

fn fit_vec(values: &[[f64; 2]], count: usize) -> Vec<[f64; 2]> {
    let mut fitted = values.to_vec();
    fitted.resize(count, [0.0, 0.0]);
    fitted
}

fn winding_number(vertices: &[[f64; 2]], closed: bool, point: [f64; 2]) -> i32 {
    if !closed || vertices.len() < 3 {
        return 0;
    }
    let mut winding = 0;
    for index in 0..vertices.len() {
        let start = vertices[index];
        let end = vertices[(index + 1) % vertices.len()];
        if start[1] <= point[1] {
            if end[1] > point[1] && is_left(start, end, point) > 0.0 {
                winding += 1;
            }
        } else if end[1] <= point[1] && is_left(start, end, point) < 0.0 {
            winding -= 1;
        }
    }
    winding
}

fn ray_crossings(vertices: &[[f64; 2]], closed: bool, point: [f64; 2]) -> u32 {
    if !closed || vertices.len() < 3 {
        return 0;
    }
    let mut crossings = 0;
    for index in 0..vertices.len() {
        let start = vertices[index];
        let end = vertices[(index + 1) % vertices.len()];
        if (start[1] > point[1]) == (end[1] > point[1]) {
            continue;
        }
        let x = start[0] + (end[0] - start[0]) * (point[1] - start[1]) / (end[1] - start[1]);
        if x > point[0] {
            crossings += 1;
        }
    }
    crossings
}

fn is_left(start: [f64; 2], end: [f64; 2], point: [f64; 2]) -> f64 {
    (end[0] - start[0]) * (point[1] - start[1]) - (point[0] - start[0]) * (end[1] - start[1])
}

fn polyline_distance(vertices: &[[f64; 2]], closed: bool, point: [f64; 2]) -> f64 {
    if vertices.is_empty() {
        return 0.0;
    }
    if vertices.len() == 1 {
        return len2(sub2(point, vertices[0]));
    }
    let edges = if closed {
        vertices.len()
    } else {
        vertices.len() - 1
    };
    let mut best = f64::MAX;
    for index in 0..edges {
        let distance = segment_distance(
            vertices[index],
            vertices[(index + 1) % vertices.len()],
            point,
        );
        best = best.min(distance);
    }
    best
}

fn segment_distance(start: [f64; 2], end: [f64; 2], point: [f64; 2]) -> f64 {
    let chord = sub2(end, start);
    let length_sq = dot2(chord, chord);
    if length_sq == 0.0 {
        return len2(sub2(point, start));
    }
    let t = (dot2(sub2(point, start), chord) / length_sq).clamp(0.0, 1.0);
    len2(sub2(point, add2(start, scale2(chord, t))))
}

fn left_normal(vector: [f64; 2]) -> [f64; 2] {
    [-vector[1], vector[0]]
}

fn right_normal(vector: [f64; 2]) -> [f64; 2] {
    [vector[1], -vector[0]]
}

fn add2(left: [f64; 2], right: [f64; 2]) -> [f64; 2] {
    [left[0] + right[0], left[1] + right[1]]
}

fn sub2(left: [f64; 2], right: [f64; 2]) -> [f64; 2] {
    [left[0] - right[0], left[1] - right[1]]
}

fn scale2(vector: [f64; 2], factor: f64) -> [f64; 2] {
    [vector[0] * factor, vector[1] * factor]
}

fn dot2(left: [f64; 2], right: [f64; 2]) -> f64 {
    left[0] * right[0] + left[1] * right[1]
}

fn len2(vector: [f64; 2]) -> f64 {
    dot2(vector, vector).sqrt()
}

fn lerp2(start: [f64; 2], end: [f64; 2], t: f64) -> [f64; 2] {
    [
        start[0] + (end[0] - start[0]) * t,
        start[1] + (end[1] - start[1]) * t,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(left: f64, right: f64) -> bool {
        (left - right).abs() < 1e-6
    }

    fn near2(left: [f64; 2], right: [f64; 2]) -> bool {
        near(left[0], right[0]) && near(left[1], right[1])
    }

    fn rect(min: [f64; 2], max: [f64; 2]) -> Vec<[f64; 2]> {
        vec![
            [min[0], min[1]],
            [max[0], min[1]],
            [max[0], max[1]],
            [min[0], max[1]],
        ]
    }

    fn kind_names(group: &ShapeNode) -> Vec<&'static str> {
        group
            .children
            .iter()
            .map(|child| match child.kind {
                ShapeKind::Group => "group",
                ShapeKind::Path => "path",
                ShapeKind::Fill => "fill",
                ShapeKind::Stroke => "stroke",
                ShapeKind::GradientFill => "gradient-fill",
                ShapeKind::GradientStroke => "gradient-stroke",
                ShapeKind::MergePaths(_) => "merge",
                ShapeKind::OffsetPaths => "offset",
                ShapeKind::PuckerAndBloat => "pucker",
                ShapeKind::Repeater => "repeater",
                ShapeKind::RoundCorners => "round",
                ShapeKind::TrimPaths => "trim",
                ShapeKind::Twist => "twist",
                ShapeKind::ZigZag => "zigzag",
                ShapeKind::WigglePaths => "wiggle-paths",
                ShapeKind::WiggleTransform => "wiggle-transform",
                ShapeKind::Transform => "transform",
            })
            .collect()
    }

    #[test]
    fn group_insertion_alt_and_repeater() {
        let mut group = ShapeNode::group(PropertyId::new(1), PropertyId::new(2), "Group");
        assert!(group.insert_content(
            ShapeNode::new(PropertyId::new(3), "P1", ShapeKind::Path),
            false
        ));
        assert!(group.insert_content(
            ShapeNode::new(PropertyId::new(4), "Fill", ShapeKind::Fill),
            false
        ));
        assert!(group.insert_content(
            ShapeNode::new(PropertyId::new(5), "Stroke", ShapeKind::Stroke),
            false
        ));
        assert!(group.insert_content(
            ShapeNode::new(PropertyId::new(6), "P2", ShapeKind::Path),
            false
        ));
        assert!(group.insert_content(
            ShapeNode::new(PropertyId::new(7), "Offset", ShapeKind::OffsetPaths),
            false
        ));
        assert!(group.insert_content(
            ShapeNode::new(PropertyId::new(8), "Repeater", ShapeKind::Repeater),
            false
        ));
        assert!(group.insert_content(
            ShapeNode::new(PropertyId::new(9), "Fill2", ShapeKind::Fill),
            true
        ));
        assert_eq!(
            kind_names(&group),
            vec![
                "path",
                "path",
                "offset",
                "stroke",
                "fill",
                "fill",
                "repeater",
                "transform"
            ]
        );
        let released = ungroup(group_selection(
            PropertyId::new(1),
            PropertyId::new(2),
            "G",
            vec![
                ShapeNode::new(PropertyId::new(3), "A", ShapeKind::Path),
                ShapeNode::new(PropertyId::new(4), "B", ShapeKind::Path),
            ],
        ));
        assert_eq!(released.len(), 2);
        assert_eq!(
            swatch(&[[0.0, 0.0, 0.0, 1.0], [1.0, 0.0, 0.0, 1.0]]),
            Swatch::Mixed
        );
        let mut colors = [[0.2, 0.2, 0.2, 1.0], [0.8, 0.1, 0.1, 1.0]];
        for color in &mut colors {
            *color = [0.0, 1.0, 0.0, 1.0];
        }
        assert_eq!(swatch(&colors), Swatch::Value([0.0, 1.0, 0.0, 1.0]));
    }

    #[test]
    fn transform_is_anchor_relative_and_duplicates() {
        assert_eq!(group_transform_channels(), &["anchor", "position"]);
        let transform = GroupTransform {
            anchor: [1.0, 0.0],
            position: [0.0, 0.0],
        };
        assert!(near2(path_point_world([5.0, 0.0], transform), [4.0, 0.0]));
        let (path, converted) = parametric_rect_to_bezier(ParametricRect {
            center: [10.0, 20.0],
            size: [4.0, 6.0],
            position: [0.0, 0.0],
        });
        assert!(near2(converted.position, [10.0, 20.0]));
        assert!(near2(path.vertices[0], [-2.0, -3.0]));
        let mut group = ShapeNode::group(PropertyId::new(1), PropertyId::new(2), "G");
        let mut path_node = ShapeNode::new(PropertyId::new(3), "P", ShapeKind::Path);
        path_node.payload = ShapePayload::Path(ShapePath::polygon(vec![[2.0, 0.0]], false));
        assert!(group.insert_content(path_node, false));
        let copy = duplicate_group(&group, [5.0, 0.0], 90.0, [1.0, 1.0]);
        let copied = copy.children.iter().find_map(|child| {
            if let ShapePayload::Path(path) = &child.payload {
                Some(path.vertices[0])
            } else {
                None
            }
        });
        assert!(near2(copied.unwrap(), [0.0, -2.0]));
        let moved = copy.children.iter().find_map(|child| {
            if let ShapePayload::Transform(transform) = &child.payload {
                Some(transform.position)
            } else {
                None
            }
        });
        assert!(near2(moved.unwrap(), [5.0, 0.0]));
    }

    #[test]
    fn fill_clips_color_and_switches_order() {
        let mut paint = SolidPaint::new();
        assert_eq!(paint.rule, FillRule::NonZero);
        paint.set_color([-0.4, 0.25, 1.5, 2.0]);
        assert_eq!(paint.color, Some([0.0, 0.25, 1.0, 1.0]));
        assert_eq!(
            front_paint(&[PaintComposite::BottomToTop, PaintComposite::BottomToTop]),
            Some(0)
        );
        assert_eq!(
            front_paint(&[
                PaintComposite::BottomToTop,
                PaintComposite::AbovePreviousInSameGroup
            ]),
            Some(1)
        );
        let outer = rect([0.0, 0.0], [4.0, 4.0]);
        let inner = rect([1.0, 1.0], [3.0, 3.0]);
        assert!(fill_contains(
            FillRule::NonZero,
            &[outer.clone(), inner.clone()],
            [2.0, 2.0]
        ));
        assert!(!fill_contains(
            FillRule::EvenOdd,
            &[outer, inner],
            [2.0, 2.0]
        ));
        assert_eq!(
            ToolPaintKind::None.cycle().cycle().cycle().cycle(),
            ToolPaintKind::None
        );
    }

    #[test]
    fn stroke_dash_cap_miter_and_taper() {
        let mut stroke = StrokeStyle::new();
        assert!(stroke.try_push_dash(1.0, 1.0).is_ok());
        assert!(stroke.try_push_dash(1.0, 1.0).is_ok());
        assert!(stroke.try_push_dash(1.0, 1.0).is_ok());
        assert!(stroke.try_push_dash(1.0, 1.0).is_err());
        assert!(near(cap_extension(LineCap::Round, 25.0), 25.0));
        assert!(near(cap_extension(LineCap::Projecting, 25.0), 25.0));
        assert!(near(cap_extension(LineCap::Butt, 25.0), 0.0));
        let right = std::f64::consts::FRAC_PI_2;
        assert!(miter_becomes_bevel(LineJoin::Miter, 1.0, 10.0, right));
        assert!(!miter_becomes_bevel(LineJoin::Miter, 4.0, 10.0, right));
        assert!(miter_becomes_bevel(LineJoin::Miter, 4.0, 10.0, 0.2));
        assert_eq!(taper_profile(0.0), TaperProfile::Straight);
        assert_eq!(taper_profile(-1.0), TaperProfile::Sharp);
        assert_eq!(taper_profile(1.0), TaperProfile::Round);
        assert!(near(taper_straight_width(2.0, 8.0, 0.5), 5.0));
        assert!(near(
            taper_length_along(LengthUnit::Percent, 50.0, 80.0),
            40.0
        ));
        assert_eq!(wave_count(WaveUnit::Cycles, 1.0, 100.0), Some(1.0));
        stroke.arm_offset_stopwatch();
        assert!(stroke.offset_stopwatch);
    }

    #[test]
    fn gradient_fill_follows_axis_and_accepts_color_keys() {
        let mut fill = GradientFill::at_layer_center([40.0, 50.0]);
        assert!(near2(fill.geometry.start, [40.0, 50.0]));
        assert!(near2(fill.geometry.end, [40.0, 50.0]));
        fill.geometry.start = [0.0, 0.0];
        fill.geometry.end = [10.0, 0.0];
        fill.geometry.color_stops = vec![
            ColorStop {
                offset: 0.0,
                color: [1.0, 0.0, 0.0, 1.0],
            },
            ColorStop {
                offset: 1.0,
                color: [0.0, 0.0, 1.0, 1.0],
            },
        ];
        assert_eq!(sample_linear(&fill.geometry, [0.0, 0.0])[0], 1.0);
        assert_eq!(sample_linear(&fill.geometry, [10.0, 0.0])[2], 1.0);
        assert_eq!(sample_linear(&fill.geometry, [5.0, 99.0])[0], 0.5);
        assert_eq!(sample_radial(&fill.geometry, [0.0, 0.0])[0], 1.0);
        assert_eq!(sample_radial(&fill.geometry, [10.0, 0.0])[2], 1.0);
        fill.geometry
            .push_color_key(0.0, fill.geometry.color_stops.clone());
        fill.geometry.push_color_key(
            1.0,
            vec![ColorStop {
                offset: 0.0,
                color: [0.0, 1.0, 0.0, 1.0],
            }],
        );
        assert_eq!(fill.geometry.color_keys.len(), 2);
        assert_eq!(clip_color([2.0, -1.0, 0.3, 1.2]), [1.0, 0.0, 0.3, 1.0]);
    }

    #[test]
    fn gradient_stroke_stays_inside_width_and_keys_offset() {
        let square = rect([0.0, 0.0], [10.0, 10.0]);
        let mut stroke = GradientStroke::at_layer_center([0.0, 0.0]);
        stroke.geometry.start = [0.0, 0.0];
        stroke.geometry.end = [10.0, 0.0];
        stroke.geometry.color_stops = vec![
            ColorStop {
                offset: 0.0,
                color: [1.0, 0.0, 0.0, 1.0],
            },
            ColorStop {
                offset: 1.0,
                color: [0.0, 0.0, 1.0, 1.0],
            },
        ];
        stroke.stroke.width = Some(4.0);
        assert!(stroke_covers(&square, true, 4.0, [5.0, 0.0]));
        assert!(!stroke_covers(&square, true, 4.0, [5.0, 5.0]));
        let on_edge = sample_linear(&stroke.geometry, [0.0, 0.0]);
        assert_eq!(on_edge[0], 1.0);
        stroke.stroke.arm_offset_stopwatch();
        assert!(stroke.stroke.offset_stopwatch);
    }

    #[test]
    fn merge_compound_keeps_inputs_out_of_drawing() {
        let shape = shape_from_character_paths(
            PropertyId::new(1),
            PropertyId::new(2),
            vec![
                ShapePath::polygon(rect([0.0, 0.0], [4.0, 4.0]), true),
                ShapePath::polygon(vec![[1.0, 1.0], [1.0, 3.0], [3.0, 3.0], [3.0, 1.0]], true),
            ],
        );
        assert!(shape
            .children
            .iter()
            .any(|child| child.kind == ShapeKind::MergePaths(MergeMode::Merge)));
        assert!(shape
            .children
            .iter()
            .filter(|child| child.kind == ShapeKind::Path)
            .all(|child| child.excluded_from_drawing));
        assert_eq!(
            shape
                .children
                .iter()
                .filter(|child| child.kind == ShapeKind::Path)
                .count(),
            2
        );
        let outer = rect([0.0, 0.0], [4.0, 4.0]);
        let hole = vec![[1.0, 1.0], [1.0, 3.0], [3.0, 3.0], [3.0, 1.0]];
        assert!(!merge_contains(
            MergeMode::Merge,
            &[outer.clone(), hole],
            [2.0, 2.0]
        ));
        assert!(merge_contains(MergeMode::Merge, &[outer], [0.5, 2.0]));
    }

    #[test]
    fn merge_add_drops_internal_boundary() {
        let polygons = [rect([0.0, 0.0], [2.0, 2.0]), rect([1.0, 0.0], [3.0, 2.0])];
        assert!(merge_contains(MergeMode::Add, &polygons, [0.5, 1.0]));
        assert!(merge_contains(MergeMode::Add, &polygons, [1.5, 1.0]));
        assert!(merge_contains(MergeMode::Add, &polygons, [2.5, 1.0]));
        assert!(merge_contains(MergeMode::Add, &polygons, [0.9, 1.0]));
        assert!(merge_contains(MergeMode::Add, &polygons, [1.1, 1.0]));
        assert!(!merge_contains(MergeMode::Add, &polygons, [-0.1, 1.0]));
    }

    #[test]
    fn merge_subtract_keeps_top_outside_overlap() {
        let polygons = [rect([0.0, 0.0], [4.0, 4.0]), rect([1.0, 1.0], [3.0, 3.0])];
        assert!(merge_contains(MergeMode::Subtract, &polygons, [0.5, 2.0]));
        assert!(!merge_contains(MergeMode::Subtract, &polygons, [2.0, 2.0]));
    }

    #[test]
    fn merge_intersect_keeps_only_shared_region() {
        let polygons = [rect([0.0, 0.0], [2.0, 2.0]), rect([1.0, 0.0], [3.0, 2.0])];
        assert!(merge_contains(MergeMode::Intersect, &polygons, [1.5, 1.0]));
        assert!(!merge_contains(MergeMode::Intersect, &polygons, [0.5, 1.0]));
        assert!(!merge_contains(MergeMode::Intersect, &polygons, [2.5, 1.0]));
    }

    #[test]
    fn merge_exclude_removes_full_overlap() {
        let polygons = [rect([0.0, 0.0], [2.0, 2.0]), rect([1.0, 0.0], [3.0, 2.0])];
        assert!(merge_contains(
            MergeMode::ExcludeIntersections,
            &polygons,
            [0.5, 1.0]
        ));
        assert!(!merge_contains(
            MergeMode::ExcludeIntersections,
            &polygons,
            [1.5, 1.0]
        ));
        assert!(merge_contains(
            MergeMode::ExcludeIntersections,
            &polygons,
            [2.5, 1.0]
        ));
    }

    #[test]
    fn offset_paths_expand_and_arm_copy_offset() {
        let source = rect([0.0, 0.0], [2.0, 2.0]);
        let same = offset_closed(&source, 0.0);
        assert!(near2(same[0], source[0]));
        let grown = offset_closed(&source, 1.0);
        let grown_span = (grown[2][0] - grown[0][0]) * (grown[2][1] - grown[0][1]);
        let source_span = 4.0;
        assert!(grown_span > source_span);
        let shrunk = offset_closed(&source, -0.25);
        let shrunk_span = (shrunk[2][0] - shrunk[0][0]) * (shrunk[2][1] - shrunk[0][1]);
        assert!(shrunk_span < source_span);
        let mut offset = OffsetPaths::new(1.0);
        assert_eq!(offset.copies, 1);
        offset.copies = 2;
        offset.copy_offset = 1.0;
        let copies = offset.polygons(&source);
        assert_eq!(copies.len(), 2);
        assert!(near2(
            bbox_center(&copies[0]).unwrap(),
            bbox_center(&copies[1]).unwrap()
        ));
        let outer = (copies[1][2][0] - copies[1][0][0]).abs();
        let inner = (copies[0][2][0] - copies[0][0][0]).abs();
        assert!(outer > inner);
        offset.arm_copy_offset_stopwatch();
        assert!(offset.copy_offset_stopwatch);
        assert!(miter_becomes_bevel(
            LineJoin::Miter,
            1.0,
            8.0,
            std::f64::consts::FRAC_PI_2
        ));
    }

    #[test]
    fn pucker_and_bloat_switch_without_a_sign() {
        let square = rect([0.0, 0.0], [2.0, 2.0]);
        let pucker = apply_pucker_bloat(&square, PuckerBloat::VerticesOutSegmentsIn, 0.2);
        let bloat = apply_pucker_bloat(&square, PuckerBloat::VerticesOutSegmentsIn.switched(), 0.2);
        let center = [1.0, 1.0];
        let vertex_out = len2(sub2(pucker[0], center)) > len2(sub2(square[0], center));
        let segment_in =
            len2(sub2(pucker[1], center)) < len2(sub2(lerp2(square[0], square[1], 0.5), center));
        assert!(vertex_out && segment_in);
        let vertex_in = len2(sub2(bloat[0], center)) < len2(sub2(square[0], center));
        let segment_out =
            len2(sub2(bloat[1], center)) > len2(sub2(lerp2(square[0], square[1], 0.5), center));
        assert!(vertex_in && segment_out);
    }

    #[test]
    fn repeater_places_copies_and_stays_virtual() {
        assert!(near2(
            repeater_copy_position(0, 0.0, [0.0, 8.0]),
            [0.0, 0.0]
        ));
        assert!(near2(
            repeater_copy_position(1, 0.0, [0.0, 8.0]),
            [0.0, 8.0]
        ));
        assert!(near2(
            repeater_copy_position(9, 0.0, [0.0, 8.0]),
            [0.0, 72.0]
        ));
        assert!(near2(
            repeater_copy_position(0, 3.0, [0.0, 8.0]),
            [0.0, 24.0]
        ));
        assert!(near2(
            repeater_copy_position(9, 3.0, [0.0, 8.0]),
            [0.0, 96.0]
        ));
        assert!(near(repeater_copy_opacity(0, 10, 0.2, 1.0), 0.2));
        assert!(near(repeater_copy_opacity(9, 10, 0.2, 1.0), 1.0));
        let mut group = ShapeNode::group(PropertyId::new(1), PropertyId::new(2), "G");
        let mut repeater = ShapeNode::new(PropertyId::new(3), "Repeater", ShapeKind::Repeater);
        repeater.payload = ShapePayload::Repeater(Repeater::new(10));
        assert!(group.insert_content(
            ShapeNode::new(PropertyId::new(4), "Path", ShapeKind::Path),
            false
        ));
        assert!(group.insert_content(repeater, false));
        assert_eq!(
            group
                .children
                .iter()
                .filter(|child| child.kind == ShapeKind::Repeater)
                .count(),
            1
        );
        assert!(repeater_paints_as_compound(&[
            ShapeKind::Repeater,
            ShapeKind::Fill
        ]));
        assert!(!repeater_paints_as_compound(&[
            ShapeKind::Fill,
            ShapeKind::Repeater
        ]));
    }

    #[test]
    fn round_corners_grow_with_radius_and_zero_keeps_corner() {
        let square = rect([0.0, 0.0], [2.0, 2.0]);
        let sharp = round_corners(&square, true, 0.0);
        assert!(sharp.contains(&square[0]));
        let rounded = round_corners(&square, true, 0.5);
        assert!(!rounded.contains(&square[0]));
        let gentle = round_corners(&square, true, 0.2);
        let cut_large = len2(sub2(rounded[0], square[0]));
        let cut_small = len2(sub2(gentle[0], square[0]));
        assert!(cut_large > cut_small);
    }

    #[test]
    fn trim_changes_span_per_path_and_with_first_vertex() {
        let long = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]];
        let short = trim_polyline(&long, false, 0.0, 4.0, 0.0, 0);
        let longer = trim_polyline(&long, false, 0.0, 8.0, 0.0, 0);
        assert!(path_length(&short, false) < path_length(&longer, false));
        let first = vec![[0.0, 0.0], [4.0, 0.0], [4.0, 4.0], [0.0, 4.0]];
        let from_zero = trim_polyline(&first, true, 0.0, 4.0, 0.0, 0);
        let from_one = trim_polyline(&first, true, 0.0, 4.0, 0.0, 1);
        assert!(!near2(from_zero[0], from_one[0]));
        let paths = vec![vec![[0.0, 0.0], [10.0, 0.0]], vec![[0.0, 1.0], [10.0, 1.0]]];
        let together = trim_many(&paths, 0.0, 10.0, MultiTrim::Simultaneous);
        let apart = trim_many(&paths, 0.0, 10.0, MultiTrim::Individual);
        assert!(path_length(&together[1], false) > path_length(&apart[1], false));
    }

    #[test]
    fn twist_sign_reverses_and_center_moves_more() {
        let points = vec![
            [0.1, 0.0],
            [1.0, 1.0],
            [-1.0, 1.0],
            [-1.0, -1.0],
            [1.0, -1.0],
        ];
        let clockwise = twist_points(&points, 30.0);
        let counter = twist_points(&points, -30.0);
        let center = bbox_center(&points).unwrap();
        let inner_clockwise = angle_of(sub2(clockwise[0], center));
        let inner_counter = angle_of(sub2(counter[0], center));
        let outer_clockwise = angle_of(sub2(clockwise[1], center));
        let outer_source = angle_of(sub2(points[1], center));
        assert!(inner_clockwise < 0.0);
        assert!(inner_counter > 0.0);
        assert!(inner_clockwise.abs() > (outer_clockwise - outer_source).abs());
    }

    #[test]
    fn zigzag_parameters_are_independent_and_source_remains() {
        let mut source = ShapePath::polygon(vec![[0.0, 0.0], [100.0, 0.0]], false);
        let base = ZigZag {
            spacing: ZigZagSpacing::Absolute(10.0),
            ridges_per_segment: 2,
            points: ZigZagPoints::Corner,
        };
        let spaced = apply_zigzag(&source, base);
        let more = apply_zigzag(
            &source,
            ZigZag {
                ridges_per_segment: 4,
                ..base
            },
        );
        let wider = apply_zigzag(
            &source,
            ZigZag {
                spacing: ZigZagSpacing::Absolute(30.0),
                ..base
            },
        );
        let smooth = apply_zigzag(
            &source,
            ZigZag {
                points: ZigZagPoints::Smooth,
                ..base
            },
        );
        assert!(more.vertices.len() > spaced.vertices.len());
        assert_ne!(wider.vertices, spaced.vertices);
        assert_ne!(smooth.out_tangents, spaced.out_tangents);
        source.vertices[0] = [0.0, 0.0];
        assert_eq!(source.vertices[0], [0.0, 0.0]);
        assert_eq!(source.vertices.len(), 2);
    }

    #[test]
    fn wiggle_paths_follow_time_and_phase_when_rate_is_zero() {
        let source = ShapePath::polygon(vec![[0.0, 0.0], [1.0, 0.0], [2.0, 0.0]], false);
        let moving = WigglePaths {
            size: WiggleSize::Absolute(1.0),
            detail: 1.0,
            points: ZigZagPoints::Corner,
            correlation_percent: 0.0,
            wiggles_per_second: 1.0,
            temporal_phase: 0.0,
        };
        assert_ne!(
            apply_wiggle_paths(&source, moving, 0.0).vertices,
            apply_wiggle_paths(&source, moving, 1.0).vertices
        );
        let held = WigglePaths {
            wiggles_per_second: 0.0,
            ..moving
        };
        assert_eq!(
            apply_wiggle_paths(&source, held, 0.0).vertices,
            apply_wiggle_paths(&source, held, 4.0).vertices
        );
        let phased = WigglePaths {
            temporal_phase: 1.3,
            ..held
        };
        assert_ne!(
            apply_wiggle_paths(&source, held, 0.0).vertices,
            apply_wiggle_paths(&source, phased, 0.0).vertices
        );
        assert_eq!(source.vertices.len(), 3);
    }

    #[test]
    fn wiggle_transform_depends_on_repeater_order_and_correlation() {
        let wiggle = WiggleTransform {
            position: [4.0, 0.0],
            anchor: [0.0, 0.0],
            scale: [0.0, 0.0],
            rotation_degrees: 0.0,
            correlation_percent: 0.0,
        };
        let after_a = sample_wiggle_transform(wiggle, 0, 1.0, true);
        let after_b = sample_wiggle_transform(wiggle, 1, 1.0, true);
        assert_ne!(after_a.position, after_b.position);
        let before_a = sample_wiggle_transform(wiggle, 0, 1.0, false);
        let before_b = sample_wiggle_transform(wiggle, 1, 1.0, false);
        assert_eq!(before_a.position, before_b.position);
        let locked = WiggleTransform {
            correlation_percent: 100.0,
            ..wiggle
        };
        assert_eq!(
            sample_wiggle_transform(locked, 0, 1.0, true).position,
            sample_wiggle_transform(locked, 2, 1.0, true).position
        );
        assert_ne!(
            sample_wiggle_transform(wiggle, 0, 0.0, true).position,
            sample_wiggle_transform(wiggle, 0, 1.0, true).position
        );
    }

    fn angle_of(vector: [f64; 2]) -> f64 {
        vector[1].atan2(vector[0])
    }
}
