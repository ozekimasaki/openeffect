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

    pub fn keyframes_mut(&mut self) -> Option<&mut Vec<Keyframe>> {
        match &mut self.body {
            PropertyBody::Value { keyframes, .. } => Some(keyframes),
            PropertyBody::Group { .. } => None,
        }
    }

    /// ストップウォッチを外す。キーは消え、値は尺を通して一定になる。
    pub fn disable_stopwatch(&mut self) {
        if let PropertyBody::Value {
            keyframes,
            expression,
            expression_enabled,
            ..
        } = &mut self.body
        {
            keyframes.clear();
            *expression = None;
            *expression_enabled = false;
        }
    }

    pub fn set_scalar(&mut self, value: f64) -> bool {
        match &mut self.body {
            PropertyBody::Value { value: slot, .. } => {
                *slot = PropertyValue::Scalar(value);
                true
            }
            PropertyBody::Group { .. } => false,
        }
    }

    pub fn expression(&self) -> Option<&str> {
        match &self.body {
            PropertyBody::Value { expression, .. } => expression.as_deref(),
            PropertyBody::Group { .. } => None,
        }
    }

    pub fn set_expression(&mut self, source: Option<String>) {
        if let PropertyBody::Value {
            expression,
            expression_enabled,
            ..
        } = &mut self.body
        {
            *expression_enabled = source.is_some();
            *expression = source;
        }
    }

    /// 位置を X と Y に分ける。Z は 3D のときだけ。
    pub fn separate_dimensions(&mut self, x: Property, y: Property, z: Option<Property>) -> bool {
        separate_from_value(self, x, y, z)
    }
}

fn separate_from_value(property: &mut Property, mut x: Property, mut y: Property, z: Option<Property>) -> bool {
    let current = property.base_value().cloned().unwrap_or(PropertyValue::Empty);
    let (xv, yv, zv) = match current {
        PropertyValue::TwoD([x, y]) => (x, y, 0.0),
        PropertyValue::ThreeD([x, y, z]) => (x, y, z),
        _ => return false,
    };
    let _ = x.set_scalar(xv);
    let _ = y.set_scalar(yv);
    let mut children = vec![x, y];
    if let Some(mut z_prop) = z {
        let _ = z_prop.set_scalar(zv);
        children.push(z_prop);
    }
    property.body = PropertyBody::Group { children };
    true
}
