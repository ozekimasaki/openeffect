use crate::blend::BlendMode;
use crate::id::{ItemId, LayerId};
use crate::mask::Mask;
use crate::property::Property;
use crate::shape::ShapeContents;
use crate::text::{Text, TextAnimator};
use crate::track_matte::TrackMatte;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CameraNodes {
    One,
    Two,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LightKind {
    Parallel,
    Spot,
    Point,
    Ambient,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LayerKind {
    Footage { source: ItemId },
    Solid { source: ItemId },
    Precomp { source: ItemId },
    Null,
    Adjustment,
    Camera { nodes: CameraNodes },
    Light { kind: LightKind },
    Text,
    Shape,
    Guide,
    Placeholder { source: ItemId },
}

/// レイヤー。ブレンドと親子はキーフレームを持たない。
#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    pub id: LayerId,
    pub name: String,
    pub kind: LayerKind,
    pub parent: Option<LayerId>,
    pub blend_mode: BlendMode,
    pub track_matte: TrackMatte,
    pub preserve_underlying_transparency: bool,
    pub collapse_transformations: bool,
    pub motion_blur: bool,
    pub three_d: bool,
    pub masks: Vec<Mask>,
    pub properties: Vec<Property>,
    pub text: Option<Text>,
    pub text_animators: Vec<TextAnimator>,
    pub shape: Option<ShapeContents>,
}

impl Layer {
    pub fn new(id: LayerId, name: impl Into<String>, kind: LayerKind) -> Self {
        Self {
            id,
            name: name.into(),
            kind,
            parent: None,
            blend_mode: BlendMode::Normal,
            track_matte: TrackMatte::none(),
            preserve_underlying_transparency: false,
            collapse_transformations: false,
            motion_blur: false,
            three_d: false,
            masks: Vec::new(),
            properties: Vec::new(),
            text: None,
            text_animators: Vec::new(),
            shape: None,
        }
    }

    pub fn property(&self, match_name: &str) -> Option<&Property> {
        self.properties.iter().find_map(|property| {
            if property.match_name == match_name {
                Some(property)
            } else {
                property.child(match_name)
            }
        })
    }

    pub fn property_mut(&mut self, match_name: &str) -> Option<&mut Property> {
        for property in &mut self.properties {
            if property.match_name == match_name {
                return Some(property);
            }
            if property.child(match_name).is_some() {
                return property.child_mut(match_name);
            }
        }
        None
    }
}
