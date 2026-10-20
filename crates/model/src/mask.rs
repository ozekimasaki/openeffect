use crate::id::PropertyId;
use crate::shape::ShapePath;

/// 閉じたマスクの合成。新規は Add。開いたパスは None。
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

/// レイヤーのマスク。不透明度 1 が内側を不透明にする。
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
        }
    }
}
