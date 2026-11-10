use crate::id::PropertyId;
use crate::property::Property;

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
}

impl ShapePath {
    pub fn new() -> Self {
        Self::default()
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

/// 塗りの内側判定。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FillRule {
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
pub struct ShapeNode {
    pub id: PropertyId,
    pub name: String,
    pub kind: ShapeKind,
    pub properties: Vec<Property>,
    pub children: Vec<ShapeNode>,
}

impl ShapeNode {
    pub fn new(id: PropertyId, name: impl Into<String>, kind: ShapeKind) -> Self {
        Self {
            id,
            name: name.into(),
            kind,
            properties: Vec::new(),
            children: Vec::new(),
        }
    }
}
