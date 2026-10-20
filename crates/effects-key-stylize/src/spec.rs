//! 効果とパラメータの定義。初期値が文書で確定していないものは `Unknown` のままにする。

use std::collections::BTreeMap;

use effect_api::{ApplyError, ApplyResult, ParamList};
use model::{BitsPerChannel, Rgba, Time};

use crate::image::Image;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParamKind {
    Unknown,
    Slider,
    Angle,
    Checkbox,
    Color,
    Point,
    Popup,
    Layer,
    Mask,
    Button,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum KnownDefault {
    Unknown,
    Float(f64),
    Angle(f64),
    Checkbox(bool),
    Color(Rgba),
    Point([f64; 2]),
    Choice(&'static str),
    NoLayer,
    CurrentLayer,
    NoMask,
    /// レイヤーの先頭フレーム。
    FirstFrame,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParamSpec {
    pub id: &'static str,
    pub name: &'static str,
    /// 文書が正式名を確定しているときだけ Effect Controls へ出せる。
    pub formal_name: bool,
    pub kind: ParamKind,
    pub default: KnownDefault,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub choices: &'static [&'static str],
    pub choices_complete: bool,
    pub keyframable: Option<bool>,
}

impl ParamSpec {
    pub const fn new(id: &'static str, name: &'static str) -> Self {
        Self {
            id,
            name,
            formal_name: true,
            kind: ParamKind::Unknown,
            default: KnownDefault::Unknown,
            min: None,
            max: None,
            choices: &[],
            choices_complete: false,
            keyframable: None,
        }
    }

    pub const fn informal(self) -> Self {
        Self {
            formal_name: false,
            ..self
        }
    }

    pub const fn kind(self, kind: ParamKind) -> Self {
        Self { kind, ..self }
    }

    pub const fn slider(self) -> Self {
        self.kind(ParamKind::Slider)
    }

    pub const fn angle(self) -> Self {
        self.kind(ParamKind::Angle)
    }

    pub const fn check(self) -> Self {
        self.kind(ParamKind::Checkbox)
    }

    pub const fn color(self) -> Self {
        self.kind(ParamKind::Color)
    }

    pub const fn point(self) -> Self {
        self.kind(ParamKind::Point)
    }

    pub const fn layer(self) -> Self {
        self.kind(ParamKind::Layer)
    }

    pub const fn mask(self) -> Self {
        self.kind(ParamKind::Mask)
    }

    pub const fn button(self) -> Self {
        self.kind(ParamKind::Button)
    }

    pub const fn popup(self, choices: &'static [&'static str], complete: bool) -> Self {
        Self {
            kind: ParamKind::Popup,
            choices,
            choices_complete: complete,
            ..self
        }
    }

    pub const fn def_f(self, value: f64) -> Self {
        Self {
            default: KnownDefault::Float(value),
            ..self
        }
    }

    pub const fn def_bool(self, value: bool) -> Self {
        Self {
            default: KnownDefault::Checkbox(value),
            ..self
        }
    }

    pub const fn def_color(self, value: Rgba) -> Self {
        Self {
            default: KnownDefault::Color(value),
            ..self
        }
    }

    pub const fn def_choice(self, value: &'static str) -> Self {
        Self {
            default: KnownDefault::Choice(value),
            ..self
        }
    }

    pub const fn def_current_layer(self) -> Self {
        Self {
            default: KnownDefault::CurrentLayer,
            ..self
        }
    }

    pub const fn def_no_mask(self) -> Self {
        Self {
            default: KnownDefault::NoMask,
            ..self
        }
    }

    pub const fn def_first_frame(self) -> Self {
        Self {
            default: KnownDefault::FirstFrame,
            ..self
        }
    }

    pub const fn range(self, min: f64, max: f64) -> Self {
        Self {
            min: Some(min),
            max: Some(max),
            ..self
        }
    }

    pub const fn max_only(self, max: f64) -> Self {
        Self {
            max: Some(max),
            ..self
        }
    }

    pub const fn key_yes(self) -> Self {
        Self {
            keyframable: Some(true),
            ..self
        }
    }

    /// 型・正式名・初期値・範囲または完全な選択肢が揃ったものだけを共有 API へ出す。
    pub fn is_published(&self) -> bool {
        if !self.formal_name {
            return false;
        }
        match self.kind {
            ParamKind::Slider => {
                matches!(self.default, KnownDefault::Float(_))
                    && self.min.is_some()
                    && self.max.is_some()
            }
            ParamKind::Angle => matches!(self.default, KnownDefault::Angle(_)),
            ParamKind::Checkbox => matches!(self.default, KnownDefault::Checkbox(_)),
            ParamKind::Color => matches!(self.default, KnownDefault::Color(_)),
            ParamKind::Point => matches!(self.default, KnownDefault::Point(_)),
            ParamKind::Popup => match self.default {
                KnownDefault::Choice(name) => self.choices_complete && self.choices.contains(&name),
                _ => false,
            },
            ParamKind::Layer => matches!(self.default, KnownDefault::NoLayer),
            ParamKind::Unknown | ParamKind::Mask | ParamKind::Button => false,
        }
    }
}

pub const fn p(id: &'static str, name: &'static str) -> ParamSpec {
    ParamSpec::new(id, name)
}

#[derive(Clone, Copy, Debug)]
pub struct EffectDef {
    pub id: &'static str,
    pub name: &'static str,
    pub menu: &'static str,
    /// `None` は文書が深度を書いていない。制限として扱わない。
    pub bits: Option<&'static [BitsPerChannel]>,
    pub params: &'static [ParamSpec],
    pub apply: fn(&ApplyCtx<'_>) -> ApplyResult<Image>,
}

impl EffectDef {
    pub fn param(&self, id: &str) -> Option<&ParamSpec> {
        self.params.iter().find(|param| param.id == id)
    }

    pub fn param_list(&self) -> ParamList {
        let mut list = ParamList::new();
        for param in self.params.iter().filter(|param| param.is_published()) {
            push_published(&mut list, param);
        }
        list
    }

    pub fn allows(&self, bits: BitsPerChannel) -> bool {
        self.bits
            .map(|supported| supported.contains(&bits))
            .unwrap_or(true)
    }
}

fn push_published(list: &mut ParamList, param: &ParamSpec) {
    match param.kind {
        ParamKind::Slider => {
            if let KnownDefault::Float(default) = param.default {
                list.define_float(
                    param.name,
                    default,
                    param.min.unwrap_or(default),
                    param.max.unwrap_or(default),
                );
            }
        }
        ParamKind::Angle => {
            if let KnownDefault::Angle(default) = param.default {
                list.define_angle(param.name, default);
            }
        }
        ParamKind::Checkbox => {
            if let KnownDefault::Checkbox(default) = param.default {
                list.define_checkbox(param.name, default);
            }
        }
        ParamKind::Color => {
            if let KnownDefault::Color(default) = param.default {
                list.define_color(param.name, default);
            }
        }
        ParamKind::Point => {
            if let KnownDefault::Point(default) = param.default {
                list.define_point(param.name, default);
            }
        }
        ParamKind::Popup => {
            if let KnownDefault::Choice(name) = param.default {
                if let Some(index) = param.choices.iter().position(|choice| *choice == name) {
                    list.define_popup(
                        param.name,
                        param
                            .choices
                            .iter()
                            .map(|choice| (*choice).to_string())
                            .collect(),
                        index as u32,
                    );
                }
            }
        }
        ParamKind::Layer => {
            list.define_layer(param.name);
        }
        ParamKind::Unknown | ParamKind::Mask | ParamKind::Button => {}
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum StoredValue {
    Float(f64),
    Angle(f64),
    Checkbox(bool),
    Color(Rgba),
    Point([f64; 2]),
    Choice(String),
}

pub struct ApplyCtx<'a> {
    pub time: Time,
    pub bits: BitsPerChannel,
    pub source: &'a Image,
    pub values: &'a BTreeMap<&'static str, StoredValue>,
    pub layers: &'a BTreeMap<&'static str, Image>,
    pub masks: &'a BTreeMap<&'static str, Image>,
    pub sample: Option<&'a Image>,
    pub def: &'static EffectDef,
}

impl ApplyCtx<'_> {
    fn spec(&self, id: &str) -> ApplyResult<&ParamSpec> {
        self.def
            .param(id)
            .ok_or_else(|| ApplyError::new(format!("未知のパラメータ {id}")))
    }

    pub fn f64(&self, id: &str) -> ApplyResult<f64> {
        let spec = self.spec(id)?;
        match self.values.get(spec.id) {
            Some(StoredValue::Float(value)) => Ok(*value),
            Some(_) => Err(ApplyError::new(format!("{} の型が数値ではない", spec.name))),
            None => match spec.default {
                KnownDefault::Float(value) => Ok(value),
                KnownDefault::FirstFrame => Ok(0.0),
                _ => Err(ApplyError::new(format!("{} の初期値は未確認", spec.name))),
            },
        }
    }

    pub fn angle(&self, id: &str) -> ApplyResult<f64> {
        let spec = self.spec(id)?;
        match self.values.get(spec.id) {
            Some(StoredValue::Angle(value)) | Some(StoredValue::Float(value)) => Ok(*value),
            Some(_) => Err(ApplyError::new(format!("{} の型が角度ではない", spec.name))),
            None => match spec.default {
                KnownDefault::Angle(value) | KnownDefault::Float(value) => Ok(value),
                _ => Err(ApplyError::new(format!("{} の初期値は未確認", spec.name))),
            },
        }
    }

    pub fn bool(&self, id: &str) -> ApplyResult<bool> {
        let spec = self.spec(id)?;
        match self.values.get(spec.id) {
            Some(StoredValue::Checkbox(value)) => Ok(*value),
            Some(_) => Err(ApplyError::new(format!(
                "{} の型がチェックではない",
                spec.name
            ))),
            None => match spec.default {
                KnownDefault::Checkbox(value) => Ok(value),
                _ => Err(ApplyError::new(format!("{} の初期値は未確認", spec.name))),
            },
        }
    }

    pub fn color(&self, id: &str) -> ApplyResult<Rgba> {
        let spec = self.spec(id)?;
        match self.values.get(spec.id) {
            Some(StoredValue::Color(value)) => Ok(*value),
            Some(_) => Err(ApplyError::new(format!("{} の型が色ではない", spec.name))),
            None => match spec.default {
                KnownDefault::Color(value) => Ok(value),
                _ => Err(ApplyError::new(format!("{} の初期値は未確認", spec.name))),
            },
        }
    }

    pub fn point(&self, id: &str) -> ApplyResult<[f64; 2]> {
        let spec = self.spec(id)?;
        match self.values.get(spec.id) {
            Some(StoredValue::Point(value)) => Ok(*value),
            Some(_) => Err(ApplyError::new(format!(
                "{} の型がポイントではない",
                spec.name
            ))),
            None => match spec.default {
                KnownDefault::Point(value) => Ok(value),
                _ => Err(ApplyError::new(format!("{} の初期値は未確認", spec.name))),
            },
        }
    }

    pub fn choice(&self, id: &str) -> ApplyResult<String> {
        let spec = self.spec(id)?;
        match self.values.get(spec.id) {
            Some(StoredValue::Choice(value)) => Ok(value.clone()),
            Some(_) => Err(ApplyError::new(format!(
                "{} の型がメニューではない",
                spec.name
            ))),
            None => match spec.default {
                KnownDefault::Choice(value) => Ok(value.to_string()),
                _ => Err(ApplyError::new(format!("{} の初期値は未確認", spec.name))),
            },
        }
    }

    pub fn f64_opt(&self, id: &str) -> ApplyResult<Option<f64>> {
        if self.def.param(id).is_none() {
            return Ok(None);
        }
        let result = self.f64(id);
        self.optional(id, result)
    }

    pub fn bool_opt(&self, id: &str) -> ApplyResult<Option<bool>> {
        if self.def.param(id).is_none() {
            return Ok(None);
        }
        let result = self.bool(id);
        self.optional(id, result)
    }

    pub fn color_opt(&self, id: &str) -> ApplyResult<Option<Rgba>> {
        if self.def.param(id).is_none() {
            return Ok(None);
        }
        let result = self.color(id);
        self.optional(id, result)
    }

    fn optional<T>(&self, id: &str, result: ApplyResult<T>) -> ApplyResult<Option<T>> {
        match result {
            Ok(value) => Ok(Some(value)),
            Err(_) => {
                let spec = self.spec(id)?;
                if !self.values.contains_key(spec.id)
                    && matches!(spec.default, KnownDefault::Unknown)
                {
                    Ok(None)
                } else {
                    Err(ApplyError::new(format!("{} の初期値は未確認", spec.name)))
                }
            }
        }
    }

    pub fn choice_opt(&self, id: &str) -> ApplyResult<Option<String>> {
        if self.def.param(id).is_none() {
            return Ok(None);
        }
        match self.choice(id) {
            Ok(value) => Ok(Some(value)),
            Err(_) => {
                let spec = self.spec(id)?;
                if matches!(spec.default, KnownDefault::Unknown)
                    && !self.values.contains_key(spec.id)
                {
                    Ok(None)
                } else {
                    self.choice(id).map(Some)
                }
            }
        }
    }

    pub fn layer(&self, id: &str) -> ApplyResult<&Image> {
        let spec = self.spec(id)?;
        if let Some(image) = self.layers.get(spec.id) {
            return Ok(image);
        }
        if matches!(spec.default, KnownDefault::CurrentLayer) {
            return Ok(self.source);
        }
        Err(ApplyError::new(format!("{} のレイヤーが無い", spec.name)))
    }

    pub fn mask(&self, id: &str) -> ApplyResult<Option<&Image>> {
        let spec = self.spec(id)?;
        if let Some(image) = self.masks.get(spec.id) {
            return Ok(Some(image));
        }
        if matches!(spec.default, KnownDefault::NoMask) {
            return Ok(None);
        }
        Err(ApplyError::new(format!("{} のマスクが無い", spec.name)))
    }

    /// 未設定かつ初期値未確認のマスクは、その項目を適用しない。
    pub fn mask_if_present(&self, id: &str) -> ApplyResult<Option<&Image>> {
        let spec = self.spec(id)?;
        if let Some(image) = self.masks.get(spec.id) {
            return Ok(Some(image));
        }
        if matches!(spec.default, KnownDefault::NoMask | KnownDefault::Unknown) {
            return Ok(None);
        }
        Err(ApplyError::new(format!("{} のマスクが無い", spec.name)))
    }
}
