use std::collections::BTreeMap;

use model::{ItemId, LayerId, PropertyId};

use crate::ast::Stmt;

/// 式の値。V8 でも同じ列挙を公開する。
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Number(f64),
    Bool(bool),
    String(String),
    Array(Vec<Value>),
    Null,
    Comp(ItemId),
    Layer {
        comp: ItemId,
        layer: LayerId,
    },
    Footage(ItemId),
    Property(PropRef),
    Group(PropRef),
    MaskRef {
        comp: ItemId,
        layer: LayerId,
        index: usize,
    },
    Path(PathData),
    MarkerProp {
        comp: ItemId,
        layer: Option<LayerId>,
    },
    MarkerKey {
        comp: ItemId,
        layer: Option<LayerId>,
        index: usize,
    },
    KeyRef {
        prop: PropRef,
        index: usize,
    },
    Rect {
        top: f64,
        left: f64,
        width: f64,
        height: f64,
    },
    Content {
        comp: ItemId,
        layer: LayerId,
        names: Vec<String>,
    },
    SourceText {
        comp: ItemId,
        layer: LayerId,
    },
    TextObject {
        comp: ItemId,
        layer: LayerId,
    },
    Map(BTreeMap<String, Value>),
    Dollar,
    Project,
    GlobalObject,
    This,
    Function(Function),
}

#[derive(Clone, Debug, PartialEq)]
pub struct PropRef {
    pub comp: ItemId,
    pub layer: LayerId,
    pub id: PropertyId,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PathData {
    pub points: Vec<[f64; 2]>,
    pub in_tangents: Vec<[f64; 2]>,
    pub out_tangents: Vec<[f64; 2]>,
    pub closed: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Function {
    pub params: Vec<String>,
    pub body: Vec<Stmt>,
}

impl Value {
    pub fn number(&self) -> Option<f64> {
        match self {
            Self::Number(value) => Some(*value),
            Self::Bool(true) => Some(1.0),
            Self::Bool(false) => Some(0.0),
            _ => None,
        }
    }

    pub fn text(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }

    pub fn bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }

    pub fn array(&self) -> Option<&[Value]> {
        match self {
            Self::Array(value) => Some(value),
            _ => None,
        }
    }

    pub fn truthy(&self) -> bool {
        match self {
            Self::Bool(value) => *value,
            Self::Number(value) => *value != 0.0 && !value.is_nan(),
            Self::String(value) => !value.is_empty(),
            Self::Null => false,
            _ => true,
        }
    }
}

pub fn numbers_of(value: &Value) -> Option<Vec<f64>> {
    let array = value.array()?;
    array.iter().map(Value::number).collect()
}

pub fn array_of(values: impl IntoIterator<Item = f64>) -> Value {
    Value::Array(values.into_iter().map(Value::Number).collect())
}
