use model::{LayerId, Rgba};

/// 定義順の番号。0 始まり。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ParamIndex(u32);

impl ParamIndex {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Param {
    Float {
        name: String,
        default: f64,
        min: f64,
        max: f64,
    },
    Angle {
        name: String,
        default_degrees: f64,
    },
    Checkbox {
        name: String,
        default: bool,
    },
    Color {
        name: String,
        default: Rgba,
    },
    Point {
        name: String,
        default: [f64; 2],
    },
    Popup {
        name: String,
        choices: Vec<String>,
        default_index: u32,
    },
    Layer {
        name: String,
    },
}

impl Param {
    pub fn name(&self) -> &str {
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
}

/// 評価後のパラメータ値。
#[derive(Clone, Debug, PartialEq)]
pub enum ParamValue {
    Float(f64),
    Angle(f64),
    Checkbox(bool),
    Color(Rgba),
    Point([f64; 2]),
    Popup(u32),
    Layer(Option<LayerId>),
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ParamList {
    params: Vec<Param>,
}

impl ParamList {
    pub fn new() -> Self {
        Self { params: Vec::new() }
    }

    pub fn len(&self) -> usize {
        self.params.len()
    }

    pub fn is_empty(&self) -> bool {
        self.params.is_empty()
    }

    pub fn get(&self, index: ParamIndex) -> Option<&Param> {
        self.params.get(index.get() as usize)
    }

    pub fn iter(&self) -> impl Iterator<Item = (ParamIndex, &Param)> {
        self.params
            .iter()
            .enumerate()
            .map(|(index, param)| (ParamIndex::new(index as u32), param))
    }

    pub fn define_float(
        &mut self,
        name: impl Into<String>,
        default: f64,
        min: f64,
        max: f64,
    ) -> ParamIndex {
        self.push(Param::Float {
            name: name.into(),
            default,
            min,
            max,
        })
    }

    pub fn define_angle(&mut self, name: impl Into<String>, default_degrees: f64) -> ParamIndex {
        self.push(Param::Angle {
            name: name.into(),
            default_degrees,
        })
    }

    pub fn define_checkbox(&mut self, name: impl Into<String>, default: bool) -> ParamIndex {
        self.push(Param::Checkbox {
            name: name.into(),
            default,
        })
    }

    pub fn define_color(&mut self, name: impl Into<String>, default: Rgba) -> ParamIndex {
        self.push(Param::Color {
            name: name.into(),
            default,
        })
    }

    pub fn define_point(&mut self, name: impl Into<String>, default: [f64; 2]) -> ParamIndex {
        self.push(Param::Point {
            name: name.into(),
            default,
        })
    }

    pub fn define_popup(
        &mut self,
        name: impl Into<String>,
        choices: Vec<String>,
        default_index: u32,
    ) -> ParamIndex {
        self.push(Param::Popup {
            name: name.into(),
            choices,
            default_index,
        })
    }

    pub fn define_layer(&mut self, name: impl Into<String>) -> ParamIndex {
        self.push(Param::Layer { name: name.into() })
    }

    fn push(&mut self, param: Param) -> ParamIndex {
        let index = ParamIndex::new(self.params.len() as u32);
        self.params.push(param);
        index
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ParamValues {
    values: Vec<ParamValue>,
}

impl ParamValues {
    pub fn new() -> Self {
        Self { values: Vec::new() }
    }

    pub fn push(&mut self, value: ParamValue) {
        self.values.push(value);
    }

    pub fn get(&self, index: ParamIndex) -> Option<&ParamValue> {
        self.values.get(index.get() as usize)
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

/// 定義の既定値で値リストを作る。
pub fn defaults_from(list: &ParamList) -> ParamValues {
    let mut values = ParamValues::new();
    for (_, param) in list.iter() {
        values.push(match param {
            Param::Float { default, .. } => ParamValue::Float(*default),
            Param::Angle {
                default_degrees, ..
            } => ParamValue::Angle(*default_degrees),
            Param::Checkbox { default, .. } => ParamValue::Checkbox(*default),
            Param::Color { default, .. } => ParamValue::Color(*default),
            Param::Point { default, .. } => ParamValue::Point(*default),
            Param::Popup { default_index, .. } => ParamValue::Popup(*default_index),
            Param::Layer { .. } => ParamValue::Layer(None),
        });
    }
    values
}
