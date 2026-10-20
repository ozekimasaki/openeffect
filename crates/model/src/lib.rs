//! プロジェクトの共有型。評価と描画は持たない。

#![forbid(unsafe_code)]

pub mod bits;
pub mod blend;
pub mod color;
pub mod composition;
pub mod folder;
pub mod footage;
pub mod id;
pub mod keyframe;
pub mod layer;
pub mod mask;
pub mod project;
pub mod property;
pub mod shape;
pub mod text;
pub mod time;
pub mod track_matte;
pub mod transform;

pub use bits::BitsPerChannel;
pub use blend::BlendMode;
pub use color::Rgba;
pub use composition::{Composition, MotionBlurSettings};
pub use folder::{Folder, FolderChild};
pub use footage::{Footage, FootageKind};
pub use id::{Ids, ItemId, LayerId, PropertyId};
pub use keyframe::{
    Keyframe, KeyframeEase, Marker, Roving, SpatialInterpolation, TemporalInterpolation,
};
pub use layer::{CameraNodes, Layer, LayerKind, LightKind};
pub use mask::{Mask, MaskMode};
pub use project::Project;
pub use property::{Property, PropertyBody, PropertyValue};
pub use shape::{FillRule, LineCap, LineJoin, MergeMode, ShapeContents, ShapeKind, ShapePath};
pub use text::{ParagraphAlignment, Text, TextAnimator, TextLayout, TextOrientation, TextStyle};
pub use time::Time;
pub use track_matte::{TrackMatte, TrackMatteMode};
pub use transform::{Transform2d, Transform3d};
