use std::sync::OnceLock;

use effect_api::ParamList;
use model::Rgba;

#[path = "catalog_data.rs"]
mod catalog_data;

/// 効果メニューの区分。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Category {
    Simulation,
    Channel3d,
    Audio,
    Time,
    Transition,
    Utility,
    Expression,
    Obsolete,
}

/// キーフレーム可否。出典が要確認のときは未確定。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Keyframes {
    Unconfirmed,
    Yes,
}

/// 数値の単位。スライダー範囲とは別。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Unit {
    Scalar,
    Percent,
    Slider,
    Decibel,
    Hertz,
    Milliseconds,
    Seconds,
    Ratio,
    Opacity,
    Pixels,
    Frames,
    Integer,
}

/// ボタンが絵に及ぼす動作。ダイアログは設定一覧が要確認。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ButtonAct {
    Clear,
    ChooseLut,
    Dialog,
}

/// パラメータの型。数値が None の欄は出典が要確認。
#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    Number {
        default: Option<f64>,
        min: Option<f64>,
        max: Option<f64>,
        unit: Unit,
    },
    Angle {
        default: Option<f64>,
    },
    Checkbox {
        default: Option<bool>,
    },
    Color {
        default: Option<Rgba>,
    },
    Point {
        default: Option<[f64; 2]>,
    },
    Point3 {
        default: Option<[f64; 3]>,
    },
    Menu {
        choices: Vec<String>,
        complete: bool,
        default_index: Option<u32>,
    },
    Layer,
    Button {
        act: ButtonAct,
    },
    Group,
    Dialog,
    Text,
    MaskPath,
    NumberSet,
    Direction,
    Ambiguous,
    Unconfirmed,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParamDef {
    pub name: String,
    pub kind: Kind,
    pub keyframes: Keyframes,
    pub index_origin: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EffectDef {
    pub name: String,
    pub category: Category,
    pub menu: Option<String>,
    pub params: Vec<ParamDef>,
}

/// カタログ上の番号。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EffectId(pub usize);

pub fn effects() -> &'static [EffectDef] {
    static EFFECTS: OnceLock<Vec<EffectDef>> = OnceLock::new();
    EFFECTS.get_or_init(catalog_data::build_effects)
}

pub fn effect(id: EffectId) -> Option<&'static EffectDef> {
    effects().get(id.0)
}

pub fn effect_index(name: &str) -> Option<EffectId> {
    effects()
        .iter()
        .position(|effect| effect.name == name)
        .map(EffectId)
}

/// 初期値・範囲・選択肢がすべて確定しているパラメータだけを effect-api に出す。
pub fn effect_api_params(id: EffectId) -> ParamList {
    let mut list = ParamList::new();
    let Some(effect) = effect(id) else {
        return list;
    };
    for param in &effect.params {
        push_api(&mut list, param);
    }
    list
}

fn push_api(list: &mut ParamList, param: &ParamDef) {
    match &param.kind {
        Kind::Number {
            default: Some(default),
            min: Some(min),
            max: Some(max),
            ..
        } => {
            list.define_float(param.name.clone(), *default, *min, *max);
        }
        Kind::Angle {
            default: Some(default),
        } => {
            list.define_angle(param.name.clone(), *default);
        }
        Kind::Checkbox {
            default: Some(default),
        } => {
            list.define_checkbox(param.name.clone(), *default);
        }
        Kind::Color {
            default: Some(default),
        } => {
            list.define_color(param.name.clone(), *default);
        }
        Kind::Point {
            default: Some(default),
        } => {
            list.define_point(param.name.clone(), *default);
        }
        Kind::Menu {
            choices,
            complete: true,
            default_index: Some(index),
        } if !choices.is_empty() && (*index as usize) < choices.len() => {
            list.define_popup(param.name.clone(), choices.clone(), *index);
        }
        Kind::Number { .. }
        | Kind::Angle { .. }
        | Kind::Checkbox { .. }
        | Kind::Color { .. }
        | Kind::Point { .. }
        | Kind::Point3 { .. }
        | Kind::Menu { .. }
        | Kind::Layer
        | Kind::Button { .. }
        | Kind::Group
        | Kind::Dialog
        | Kind::Text
        | Kind::MaskPath
        | Kind::NumberSet
        | Kind::Direction
        | Kind::Ambiguous
        | Kind::Unconfirmed => {}
    }
}
