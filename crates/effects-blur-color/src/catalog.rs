//! 効果の一覧。初期値は仕様に数値が書いてあるものだけ。

use model::BitsPerChannel;

use crate::control::{Category, Control};

pub const BPC_ALL: &[BitsPerChannel] = &[
    BitsPerChannel::Bpc8,
    BitsPerChannel::Bpc16,
    BitsPerChannel::Bpc32,
];
pub const BPC_8: &[BitsPerChannel] = &[BitsPerChannel::Bpc8];
pub const BPC_8_16: &[BitsPerChannel] = &[BitsPerChannel::Bpc8, BitsPerChannel::Bpc16];
pub const BPC_16_32: &[BitsPerChannel] = &[BitsPerChannel::Bpc16, BitsPerChannel::Bpc32];

pub struct EffectSpec {
    pub name: &'static str,
    pub category: Category,
    pub bits: &'static [BitsPerChannel],
    pub controls: Vec<Control>,
    pub untyped: &'static [&'static str],
    pub match_names: &'static [&'static str],
    pub gpu: bool,
    pub buttons: &'static [&'static str],
}

pub const fn float_unknown(name: &'static str) -> Control {
    Control::Float {
        name,
        default: None,
        min: None,
        max: None,
    }
}

pub const fn checkbox_unknown(name: &'static str) -> Control {
    Control::Checkbox {
        name,
        default: None,
    }
}

pub const fn angle_unknown(name: &'static str) -> Control {
    Control::Angle {
        name,
        default_degrees: None,
    }
}

pub const fn color_unknown(name: &'static str) -> Control {
    Control::Color {
        name,
        default: None,
    }
}

pub const fn point_unknown(name: &'static str) -> Control {
    Control::Point {
        name,
        default: None,
    }
}

pub const fn layer(name: &'static str) -> Control {
    Control::Layer { name }
}

pub const fn popup(
    name: &'static str,
    choices: &'static [&'static str],
    default_index: Option<u32>,
    choices_complete: bool,
) -> Control {
    Control::Popup {
        name,
        choices,
        default_index,
        choices_complete,
    }
}
