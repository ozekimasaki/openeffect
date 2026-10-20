//! スクリプティングの値。ファイル、ガイド、読み込み、キーイーズ、シェイプ、マーカー。

use std::path::{Path, PathBuf};

use model::{KeyframeEase as ModelEase, Marker, ShapePath};

use crate::enums::{GuideOrientationType, GuidePositionType, ImportAsType};
use crate::error::{ScriptError, ScriptResult};

#[derive(Clone, Debug, PartialEq)]
pub struct ScriptFile {
    pub path: PathBuf,
}

impl ScriptFile {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn as_path(&self) -> &Path {
        &self.path
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScriptFolder {
    pub path: PathBuf,
}

impl ScriptFolder {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScriptEase {
    pub speed: f64,
    pub influence: f64,
}

impl ScriptEase {
    pub fn new(speed: f64, influence: f64) -> ScriptResult<Self> {
        if !(0.1..=100.0).contains(&influence) {
            return Err(ScriptError::range("KeyframeEase.influence"));
        }
        Ok(Self { speed, influence })
    }

    pub fn to_model(&self) -> ModelEase {
        ModelEase::new(self.speed, self.influence)
    }
}

impl Default for ScriptEase {
    fn default() -> Self {
        Self {
            speed: 0.0,
            influence: 16.666_666_667,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShapeValue {
    pub path: ShapePath,
}

impl ShapeValue {
    pub fn new() -> Self {
        Self {
            path: ShapePath::new(),
        }
    }

    pub fn vertices(&self) -> &[[f64; 2]] {
        &self.path.vertices
    }

    pub fn closed(&self) -> bool {
        self.path.closed
    }
}

impl Default for ShapeValue {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MarkerValue {
    pub marker: Marker,
}

impl MarkerValue {
    pub fn new(comment: impl Into<String>) -> Self {
        Self {
            marker: Marker::new(comment),
        }
    }

    pub fn get_parameters(&self) -> &[(String, String)] {
        &self.marker.parameters
    }

    pub fn set_parameters(&mut self, params: Vec<(String, String)>) {
        self.marker.parameters = params;
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Guide {
    pub orientation: i32,
    pub position: f64,
    pub position_type: GuidePositionType,
    pub color: [f64; 3],
    pub pinned: bool,
    pub pin_anchor: f64,
}

impl Guide {
    pub fn at(orientation: i32, position: f64) -> Self {
        Self {
            orientation,
            position,
            position_type: GuidePositionType::Pixels,
            color: [0.0, 1.0, 1.0],
            pinned: false,
            pin_anchor: position,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GuideOptions {
    pub orientation: Option<GuideOrientationType>,
    pub position: Option<f64>,
    pub position_type: Option<GuidePositionType>,
    pub color: Option<[f64; 3]>,
    pub pinned: Option<bool>,
}

impl GuideOptions {
    pub fn new() -> Self {
        Self {
            orientation: None,
            position: None,
            position_type: None,
            color: None,
            pinned: None,
        }
    }
}

impl Default for GuideOptions {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImportOptions {
    pub file: Option<PathBuf>,
    pub force_alphabetical: bool,
    pub import_as: ImportAsType,
    pub range_start: Option<i32>,
    pub range_end: Option<i32>,
    pub sequence: bool,
}

impl ImportOptions {
    pub fn new(file: Option<PathBuf>) -> Self {
        Self {
            file,
            force_alphabetical: false,
            import_as: ImportAsType::Footage,
            range_start: None,
            range_end: None,
            sequence: false,
        }
    }

    pub fn can_import_as(&self, kind: ImportAsType) -> bool {
        let Some(path) = &self.file else {
            return kind == ImportAsType::Footage;
        };
        let ext = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        match kind {
            ImportAsType::Footage => true,
            ImportAsType::Comp | ImportAsType::CompCroppedLayers => ext == "psd" || file_has_layers(path),
            ImportAsType::Project => ext == "aep" || ext == "oeproject",
        }
    }

    pub fn is_file_name_numbered(&self) -> NumberedName {
        let Some(path) = &self.file else {
            return NumberedName {
                numbered: false,
                number: 0,
            };
        };
        numbered_name(&path.display().to_string())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NumberedName {
    pub numbered: bool,
    pub number: i32,
}

pub fn numbered_name(name: &str) -> NumberedName {
    let file_name = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let stem = file_name.rsplit_once('.').map(|(stem, _)| stem).unwrap_or(file_name);
    let digits: String = stem
        .chars()
        .rev()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    if digits.is_empty() {
        NumberedName {
            numbered: false,
            number: 0,
        }
    } else {
        NumberedName {
            numbered: true,
            number: digits.parse().unwrap_or(0),
        }
    }
}

fn file_has_layers(path: &Path) -> bool {
    std::fs::read(path)
        .ok()
        .is_some_and(|bytes| bytes.windows(6).any(|window| window == b"layers"))
}

#[derive(Clone, Debug, PartialEq)]
pub struct Viewer {
    pub item: model::ItemId,
    pub layer: Option<model::LayerId>,
    pub kind: ViewerKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewerKind {
    Composition,
    Footage,
    Layer,
}

pub fn png_info(bytes: &[u8]) -> Option<(u32, u32, bool)> {
    if bytes.len() < 26 || &bytes[0..8] != b"\x89PNG\r\n\x1a\n" {
        return None;
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().ok()?);
    let height = u32::from_be_bytes(bytes[20..24].try_into().ok()?);
    let color_type = bytes[25];
    let alpha = color_type == 4 || color_type == 6;
    Some((width, height, alpha))
}
