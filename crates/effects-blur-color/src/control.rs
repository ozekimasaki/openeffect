//! 確認できた名前だけをコントロールにする。初期値が要確認のものは空のままにする。

use effect_api::{ApplyError, ApplyResult, ParamIndex, ParamList, ParamValue, ParamValues};
use model::Rgba;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Category {
    BlurSharpen,
    Channel,
    ColorCorrection,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Control {
    Float {
        name: &'static str,
        default: Option<f64>,
        min: Option<f64>,
        max: Option<f64>,
    },
    Angle {
        name: &'static str,
        default_degrees: Option<f64>,
    },
    Checkbox {
        name: &'static str,
        default: Option<bool>,
    },
    Color {
        name: &'static str,
        default: Option<Rgba>,
    },
    Point {
        name: &'static str,
        default: Option<[f64; 2]>,
    },
    Popup {
        name: &'static str,
        choices: &'static [&'static str],
        default_index: Option<u32>,
        choices_complete: bool,
    },
    Layer {
        name: &'static str,
    },
}

impl Control {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Float { name, .. }
            | Self::Angle { name, .. }
            | Self::Checkbox { name, .. }
            | Self::Color { name, .. }
            | Self::Point { name, .. }
            | Self::Popup { name, .. }
            | Self::Layer { name } => name,
        }
    }

    /// 仕様に数値が書いてある初期値だけを返す。
    pub fn known_default(self) -> Option<ParamValue> {
        match self {
            Self::Float {
                default: Some(value),
                ..
            } => Some(ParamValue::Float(value)),
            Self::Angle {
                default_degrees: Some(value),
                ..
            } => Some(ParamValue::Angle(value)),
            Self::Checkbox {
                default: Some(value),
                ..
            } => Some(ParamValue::Checkbox(value)),
            Self::Color {
                default: Some(value),
                ..
            } => Some(ParamValue::Color(value)),
            Self::Point {
                default: Some(value),
                ..
            } => Some(ParamValue::Point(value)),
            Self::Popup {
                default_index: Some(value),
                ..
            } => Some(ParamValue::Popup(value)),
            Self::Layer { .. }
            | Self::Float { default: None, .. }
            | Self::Angle {
                default_degrees: None,
                ..
            }
            | Self::Checkbox { default: None, .. }
            | Self::Color { default: None, .. }
            | Self::Point { default: None, .. }
            | Self::Popup {
                default_index: None,
                ..
            } => None,
        }
    }

    fn accepts(self, value: &ParamValue) -> bool {
        match (self, value) {
            (Self::Float { .. }, ParamValue::Float(_))
            | (Self::Angle { .. }, ParamValue::Angle(_))
            | (Self::Checkbox { .. }, ParamValue::Checkbox(_))
            | (Self::Color { .. }, ParamValue::Color(_))
            | (Self::Point { .. }, ParamValue::Point(_))
            | (Self::Layer { .. }, ParamValue::Layer(_)) => true,
            (
                Self::Popup {
                    choices,
                    choices_complete: _,
                    ..
                },
                ParamValue::Popup(index),
            ) => choices.is_empty() || (*index as usize) < choices.len(),
            _ => false,
        }
    }
}

pub fn check_values(controls: &[Control], values: &ParamValues) -> ApplyResult<()> {
    if values.len() != controls.len() {
        return Err(ApplyError::new("パラメータ数が定義と一致しない"));
    }
    for (index, control) in controls.iter().copied().enumerate() {
        let Some(value) = values.get(ParamIndex::new(index as u32)) else {
            return Err(ApplyError::new("パラメータが足りない"));
        };
        if !control.accepts(value) {
            return Err(ApplyError::new(format!(
                "{} の値が定義と一致しない",
                control.name()
            )));
        }
    }
    Ok(())
}

/// 初期値がすべて確認できているときだけ値リストを作る。
pub fn values_from_known_defaults(controls: &[Control]) -> ApplyResult<ParamValues> {
    let mut values = ParamValues::new();
    for control in controls {
        let Some(value) = control.known_default() else {
            return Err(ApplyError::new(format!(
                "{} の初期値は要確認",
                control.name()
            )));
        };
        values.push(value);
    }
    Ok(values)
}

/// 既定・範囲が全部確認できたコントロールだけを effect-api の定義へ写す。
pub fn confirmed_param_list(controls: &[Control]) -> ParamList {
    let mut list = ParamList::new();
    for control in controls {
        match *control {
            Control::Float {
                name,
                default: Some(default),
                min: Some(min),
                max: Some(max),
            } => {
                list.define_float(name, default, min, max);
            }
            Control::Angle {
                name,
                default_degrees: Some(default),
            } => {
                list.define_angle(name, default);
            }
            Control::Checkbox {
                name,
                default: Some(default),
            } => {
                list.define_checkbox(name, default);
            }
            Control::Color {
                name,
                default: Some(default),
            } => {
                list.define_color(name, default);
            }
            Control::Point {
                name,
                default: Some(default),
            } => {
                list.define_point(name, default);
            }
            Control::Popup {
                name,
                choices,
                default_index: Some(default),
                choices_complete: true,
            } => {
                list.define_popup(
                    name,
                    choices.iter().copied().map(str::to_string).collect(),
                    default,
                );
            }
            Control::Layer { .. }
            | Control::Float { .. }
            | Control::Angle { .. }
            | Control::Checkbox { .. }
            | Control::Color { .. }
            | Control::Point { .. }
            | Control::Popup { .. } => {}
        }
    }
    list
}

pub struct Args<'a> {
    values: &'a ParamValues,
}

impl<'a> Args<'a> {
    pub fn new(controls: &[Control], values: &'a ParamValues) -> ApplyResult<Self> {
        check_values(controls, values)?;
        Ok(Self { values })
    }

    fn get(&self, index: usize) -> ApplyResult<&ParamValue> {
        self.values
            .get(ParamIndex::new(index as u32))
            .ok_or_else(|| ApplyError::new("パラメータが足りない"))
    }

    pub fn float(&self, index: usize) -> ApplyResult<f64> {
        match self.get(index)? {
            ParamValue::Float(value) => Ok(*value),
            _ => Err(ApplyError::new("数値ではない")),
        }
    }

    pub fn angle(&self, index: usize) -> ApplyResult<f64> {
        match self.get(index)? {
            ParamValue::Angle(value) => Ok(*value),
            _ => Err(ApplyError::new("角度ではない")),
        }
    }

    pub fn checkbox(&self, index: usize) -> ApplyResult<bool> {
        match self.get(index)? {
            ParamValue::Checkbox(value) => Ok(*value),
            _ => Err(ApplyError::new("チェックではない")),
        }
    }

    pub fn color(&self, index: usize) -> ApplyResult<Rgba> {
        match self.get(index)? {
            ParamValue::Color(value) => Ok(*value),
            _ => Err(ApplyError::new("色ではない")),
        }
    }

    pub fn point(&self, index: usize) -> ApplyResult<[f64; 2]> {
        match self.get(index)? {
            ParamValue::Point(value) => Ok(*value),
            _ => Err(ApplyError::new("点ではない")),
        }
    }

    pub fn popup(&self, index: usize) -> ApplyResult<u32> {
        match self.get(index)? {
            ParamValue::Popup(value) => Ok(*value),
            _ => Err(ApplyError::new("メニューではない")),
        }
    }

    pub fn layer(&self, index: usize) -> ApplyResult<Option<model::LayerId>> {
        match self.get(index)? {
            ParamValue::Layer(value) => Ok(*value),
            _ => Err(ApplyError::new("レイヤーではない")),
        }
    }
}
