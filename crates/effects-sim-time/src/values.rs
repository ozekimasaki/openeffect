use model::{LayerId, Rgba, ShapePath};

use crate::catalog::{effects, EffectId, Kind, Unit};

/// ホストが置いた値。未設定は Empty で、要確認の初期値ではない。
#[derive(Clone, Debug, PartialEq)]
pub enum Slot {
    Empty,
    Number(f64),
    Bool(bool),
    Color(Rgba),
    Point([f64; 2]),
    Point3([f64; 3]),
    Index(u32),
    Layer(Option<LayerId>),
    Text(String),
    Path(Vec<[f64; 2]>),
    Numbers(Vec<f64>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct EffectValues {
    pub effect: EffectId,
    slots: Vec<Slot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SetError {
    MissingEffect,
    Index,
    Unconfirmed,
    WrongType,
    OutOfRange,
}

impl EffectValues {
    pub fn empty(effect: EffectId) -> Self {
        let len = effects()
            .get(effect.0)
            .map(|item| item.params.len())
            .unwrap_or(0);
        Self {
            effect,
            slots: vec![Slot::Empty; len],
        }
    }

    /// 出典で確定した初期値だけを入れる。
    pub fn initial(effect: EffectId) -> Self {
        let mut values = Self::empty(effect);
        let Some(def) = effects().get(effect.0) else {
            return values;
        };
        for (index, param) in def.params.iter().enumerate() {
            values.slots[index] = match &param.kind {
                Kind::Number {
                    default: Some(value),
                    ..
                }
                | Kind::Angle {
                    default: Some(value),
                } => Slot::Number(*value),
                Kind::Checkbox {
                    default: Some(value),
                } => Slot::Bool(*value),
                Kind::Color {
                    default: Some(value),
                } => Slot::Color(*value),
                Kind::Point {
                    default: Some(value),
                } => Slot::Point(*value),
                Kind::Point3 {
                    default: Some(value),
                } => Slot::Point3(*value),
                Kind::Menu {
                    default_index: Some(index),
                    ..
                } => Slot::Index(*index),
                Kind::Number { default: None, .. }
                | Kind::Angle { default: None }
                | Kind::Checkbox { default: None }
                | Kind::Color { default: None }
                | Kind::Point { default: None }
                | Kind::Point3 { default: None }
                | Kind::Menu {
                    default_index: None,
                    ..
                }
                | Kind::Layer
                | Kind::Button { .. }
                | Kind::Group
                | Kind::Dialog
                | Kind::Text
                | Kind::MaskPath
                | Kind::NumberSet
                | Kind::Direction
                | Kind::Ambiguous
                | Kind::Unconfirmed => Slot::Empty,
            };
        }
        values
    }

    pub fn get(&self, index: usize) -> Option<&Slot> {
        self.slots.get(index)
    }

    pub fn slots(&self) -> &[Slot] {
        &self.slots
    }

    pub fn set_number(&mut self, index: usize, value: f64) -> Result<(), SetError> {
        if !value.is_finite() {
            return Err(SetError::OutOfRange);
        }
        let kind = kind_at(self.effect, index)?;
        match kind {
            Kind::Number { min, max, unit, .. } => {
                if matches!(unit, Unit::Integer) && value.fract() != 0.0 {
                    return Err(SetError::WrongType);
                }
                in_range(value, *min, *max)?;
                self.slots[index] = Slot::Number(value);
                Ok(())
            }
            Kind::Angle { .. } => {
                self.slots[index] = Slot::Number(value);
                Ok(())
            }
            Kind::Unconfirmed | Kind::Ambiguous | Kind::Direction => Err(SetError::Unconfirmed),
            _ => Err(SetError::WrongType),
        }
    }

    pub fn set_bool(&mut self, index: usize, value: bool) -> Result<(), SetError> {
        let kind = kind_at(self.effect, index)?;
        match kind {
            Kind::Checkbox { .. } => {
                self.slots[index] = Slot::Bool(value);
                Ok(())
            }
            Kind::Unconfirmed | Kind::Ambiguous => Err(SetError::Unconfirmed),
            _ => Err(SetError::WrongType),
        }
    }

    pub fn set_color(&mut self, index: usize, value: Rgba) -> Result<(), SetError> {
        let kind = kind_at(self.effect, index)?;
        match kind {
            Kind::Color { .. } => {
                self.slots[index] = Slot::Color(value);
                Ok(())
            }
            Kind::Unconfirmed | Kind::Ambiguous => Err(SetError::Unconfirmed),
            _ => Err(SetError::WrongType),
        }
    }

    pub fn set_point(&mut self, index: usize, value: [f64; 2]) -> Result<(), SetError> {
        let kind = kind_at(self.effect, index)?;
        match kind {
            Kind::Point { .. } => {
                self.slots[index] = Slot::Point(value);
                Ok(())
            }
            Kind::Unconfirmed | Kind::Ambiguous => Err(SetError::Unconfirmed),
            _ => Err(SetError::WrongType),
        }
    }

    pub fn set_point3(&mut self, index: usize, value: [f64; 3]) -> Result<(), SetError> {
        let kind = kind_at(self.effect, index)?;
        match kind {
            Kind::Point3 { .. } => {
                self.slots[index] = Slot::Point3(value);
                Ok(())
            }
            Kind::Unconfirmed | Kind::Ambiguous => Err(SetError::Unconfirmed),
            _ => Err(SetError::WrongType),
        }
    }

    pub fn set_index(&mut self, index: usize, value: u32) -> Result<(), SetError> {
        let kind = kind_at(self.effect, index)?;
        match kind {
            Kind::Menu { choices, .. } => {
                if choices.is_empty() || value as usize >= choices.len() {
                    return Err(SetError::Unconfirmed);
                }
                self.slots[index] = Slot::Index(value);
                Ok(())
            }
            Kind::Unconfirmed | Kind::Ambiguous | Kind::Direction => Err(SetError::Unconfirmed),
            _ => Err(SetError::WrongType),
        }
    }

    pub fn set_layer(&mut self, index: usize, value: Option<LayerId>) -> Result<(), SetError> {
        let kind = kind_at(self.effect, index)?;
        match kind {
            Kind::Layer => {
                self.slots[index] = Slot::Layer(value);
                Ok(())
            }
            Kind::Unconfirmed | Kind::Ambiguous => Err(SetError::Unconfirmed),
            _ => Err(SetError::WrongType),
        }
    }

    pub fn set_text(&mut self, index: usize, value: impl Into<String>) -> Result<(), SetError> {
        let kind = kind_at(self.effect, index)?;
        match kind {
            Kind::Text => {
                self.slots[index] = Slot::Text(value.into());
                Ok(())
            }
            Kind::Unconfirmed | Kind::Ambiguous => Err(SetError::Unconfirmed),
            _ => Err(SetError::WrongType),
        }
    }

    pub fn set_path(&mut self, index: usize, vertices: Vec<[f64; 2]>) -> Result<(), SetError> {
        let kind = kind_at(self.effect, index)?;
        match kind {
            Kind::MaskPath => {
                self.slots[index] = Slot::Path(vertices);
                Ok(())
            }
            Kind::Unconfirmed | Kind::Ambiguous => Err(SetError::Unconfirmed),
            _ => Err(SetError::WrongType),
        }
    }

    pub fn set_numbers(&mut self, index: usize, values: Vec<f64>) -> Result<(), SetError> {
        let kind = kind_at(self.effect, index)?;
        match kind {
            Kind::NumberSet => {
                self.slots[index] = Slot::Numbers(values);
                Ok(())
            }
            Kind::Unconfirmed | Kind::Ambiguous => Err(SetError::Unconfirmed),
            _ => Err(SetError::WrongType),
        }
    }
}

pub fn path_of(vertices: &[[f64; 2]]) -> ShapePath {
    ShapePath {
        vertices: vertices.to_vec(),
        ..ShapePath::new()
    }
}

fn kind_at(effect: EffectId, index: usize) -> Result<&'static Kind, SetError> {
    let effect = effects().get(effect.0).ok_or(SetError::MissingEffect)?;
    effect
        .params
        .get(index)
        .map(|param| &param.kind)
        .ok_or(SetError::Index)
}

fn in_range(value: f64, min: Option<f64>, max: Option<f64>) -> Result<(), SetError> {
    if min.is_some_and(|min| value < min) || max.is_some_and(|max| value > max) {
        Err(SetError::OutOfRange)
    } else {
        Ok(())
    }
}
