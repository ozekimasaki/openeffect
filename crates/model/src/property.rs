use crate::color::Rgba;
use crate::id::{LayerId, PropertyId};
use crate::keyframe::{Keyframe, Marker};
use crate::shape::ShapePath;

/// プロパティの静的値、またはキーの値。
#[derive(Clone, Debug, PartialEq)]
pub enum PropertyValue {
    Empty,
    Scalar(f64),
    TwoD([f64; 2]),
    ThreeD([f64; 3]),
    Color(Rgba),
    Boolean(bool),
    Index(i32),
    Text(String),
    Path(Box<ShapePath>),
    Marker(Box<Marker>),
    Layer(Option<LayerId>),
}

impl PropertyValue {
    pub fn path(path: ShapePath) -> Self {
        Self::Path(Box::new(path))
    }

    pub fn marker(marker: Marker) -> Self {
        Self::Marker(Box::new(marker))
    }
}

impl Default for PropertyValue {
    fn default() -> Self {
        Self::Empty
    }
}

/// グループか、値を持つプロパティか。
#[derive(Clone, Debug, PartialEq)]
pub enum PropertyBody {
    Group {
        children: Vec<Property>,
    },
    Value {
        value: PropertyValue,
        keyframes: Vec<Keyframe>,
        expression: Option<String>,
        expression_enabled: bool,
    },
}

/// タイムライン上のプロパティ。`match_name` は後から項目を結ぶ安定名。
#[derive(Clone, Debug, PartialEq)]
pub struct Property {
    pub id: PropertyId,
    pub name: String,
    pub match_name: String,
    pub body: PropertyBody,
}

impl Property {
    pub fn group(id: PropertyId, name: impl Into<String>, match_name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            match_name: match_name.into(),
            body: PropertyBody::Group {
                children: Vec::new(),
            },
        }
    }

    pub fn value(
        id: PropertyId,
        name: impl Into<String>,
        match_name: impl Into<String>,
        value: PropertyValue,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            match_name: match_name.into(),
            body: PropertyBody::Value {
                value,
                keyframes: Vec::new(),
                expression: None,
                expression_enabled: false,
            },
        }
    }

    pub fn push_child(&mut self, child: Property) -> bool {
        match &mut self.body {
            PropertyBody::Group { children } => {
                children.push(child);
                true
            }
            PropertyBody::Value { .. } => false,
        }
    }

    pub fn child(&self, match_name: &str) -> Option<&Property> {
        let PropertyBody::Group { children } = &self.body else {
            return None;
        };
        for child in children {
            if child.match_name == match_name {
                return Some(child);
            }
            if let Some(found) = child.child(match_name) {
                return Some(found);
            }
        }
        None
    }

    pub fn child_mut(&mut self, match_name: &str) -> Option<&mut Property> {
        match &mut self.body {
            PropertyBody::Group { children } => {
                if let Some(index) = children
                    .iter()
                    .position(|child| child.match_name == match_name)
                {
                    return children.get_mut(index);
                }
                for child in children.iter_mut() {
                    if child.child(match_name).is_some() {
                        return child.child_mut(match_name);
                    }
                }
                None
            }
            PropertyBody::Value { .. } => None,
        }
    }

    pub fn base_value(&self) -> Option<&PropertyValue> {
        match &self.body {
            PropertyBody::Value { value, .. } => Some(value),
            PropertyBody::Group { .. } => None,
        }
    }

    pub fn keyframes(&self) -> &[Keyframe] {
        match &self.body {
            PropertyBody::Value { keyframes, .. } => keyframes,
            PropertyBody::Group { .. } => &[],
        }
    }

    pub fn push_keyframe(&mut self, keyframe: Keyframe) -> bool {
        match &mut self.body {
            PropertyBody::Value { keyframes, .. } => {
                keyframes.push(keyframe);
                true
            }
            PropertyBody::Group { .. } => false,
        }
    }
}
