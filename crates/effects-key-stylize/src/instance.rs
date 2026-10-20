use std::collections::BTreeMap;

use effect_api::{
    copy_frame, defaults_from, ApplyError, ApplyResult, CpuApply, CpuInput, Frame, ParamIndex,
    ParamValue, ParamValues,
};
use model::Rgba;

use crate::catalog::effect;
use crate::image::Image;
use crate::spec::{ApplyCtx, EffectDef, KnownDefault, ParamKind, StoredValue};

pub struct EffectInstance {
    def: &'static EffectDef,
    values: BTreeMap<&'static str, StoredValue>,
    layers: BTreeMap<&'static str, Image>,
    masks: BTreeMap<&'static str, Image>,
    sample: Option<Image>,
}

impl EffectInstance {
    pub fn new(def: &'static EffectDef) -> Self {
        let mut values = BTreeMap::new();
        for param in def.params {
            if let Some(value) = stored_default(param.default) {
                values.insert(param.id, value);
            }
        }
        Self {
            def,
            values,
            layers: BTreeMap::new(),
            masks: BTreeMap::new(),
            sample: None,
        }
    }

    pub fn spec(&self) -> &'static EffectDef {
        self.def
    }

    pub fn set_float(&mut self, id: &str, value: f64) -> ApplyResult<()> {
        let spec = self.param(id)?;
        match spec.kind {
            ParamKind::Slider | ParamKind::Unknown | ParamKind::Angle => {}
            _ => return Err(ApplyError::new(format!("{} は数値にできない", spec.name))),
        }
        if let Some(min) = spec.min {
            if value < min {
                return Err(ApplyError::new(format!(
                    "{} は {min} 未満にできない",
                    spec.name
                )));
            }
        }
        if let Some(max) = spec.max {
            if value > max {
                return Err(ApplyError::new(format!(
                    "{} は {max} を超えられない",
                    spec.name
                )));
            }
        }
        let stored = if spec.kind == ParamKind::Angle {
            StoredValue::Angle(value)
        } else {
            StoredValue::Float(value)
        };
        self.values.insert(spec.id, stored);
        Ok(())
    }

    pub fn set_angle(&mut self, id: &str, degrees: f64) -> ApplyResult<()> {
        let spec = self.param(id)?;
        match spec.kind {
            ParamKind::Angle | ParamKind::Unknown => {}
            _ => return Err(ApplyError::new(format!("{} は角度にできない", spec.name))),
        }
        self.values.insert(spec.id, StoredValue::Angle(degrees));
        Ok(())
    }

    pub fn set_checkbox(&mut self, id: &str, value: bool) -> ApplyResult<()> {
        let spec = self.param(id)?;
        match spec.kind {
            ParamKind::Checkbox | ParamKind::Unknown => {}
            _ => {
                return Err(ApplyError::new(format!(
                    "{} はチェックにできない",
                    spec.name
                )))
            }
        }
        self.values.insert(spec.id, StoredValue::Checkbox(value));
        Ok(())
    }

    pub fn set_color(&mut self, id: &str, value: Rgba) -> ApplyResult<()> {
        let spec = self.param(id)?;
        match spec.kind {
            ParamKind::Color | ParamKind::Unknown => {}
            _ => return Err(ApplyError::new(format!("{} は色にできない", spec.name))),
        }
        self.values.insert(spec.id, StoredValue::Color(value));
        Ok(())
    }

    pub fn set_point(&mut self, id: &str, value: [f64; 2]) -> ApplyResult<()> {
        let spec = self.param(id)?;
        match spec.kind {
            ParamKind::Point | ParamKind::Unknown => {}
            _ => {
                return Err(ApplyError::new(format!(
                    "{} はポイントにできない",
                    spec.name
                )))
            }
        }
        self.values.insert(spec.id, StoredValue::Point(value));
        Ok(())
    }

    pub fn set_choice(&mut self, id: &str, choice: &str) -> ApplyResult<()> {
        let spec = self.param(id)?;
        match spec.kind {
            ParamKind::Popup | ParamKind::Unknown => {}
            _ => {
                return Err(ApplyError::new(format!(
                    "{} はメニューにできない",
                    spec.name
                )))
            }
        }
        if spec.choices_complete && !spec.choices.contains(&choice) {
            return Err(ApplyError::new(format!(
                "{} の選択肢に {choice} は無い",
                spec.name
            )));
        }
        self.values
            .insert(spec.id, StoredValue::Choice(choice.to_string()));
        Ok(())
    }

    pub fn set_layer_frame(&mut self, id: &str, frame: &Frame) -> ApplyResult<()> {
        let spec = self.param(id)?;
        match spec.kind {
            ParamKind::Layer | ParamKind::Unknown => {}
            _ => {
                return Err(ApplyError::new(format!(
                    "{} はレイヤーにできない",
                    spec.name
                )))
            }
        }
        self.layers.insert(spec.id, Image::from_frame(frame)?);
        Ok(())
    }

    pub fn set_mask_frame(&mut self, id: &str, frame: &Frame) -> ApplyResult<()> {
        let spec = self.param(id)?;
        match spec.kind {
            ParamKind::Mask | ParamKind::Unknown => {}
            _ => return Err(ApplyError::new(format!("{} はマスクにできない", spec.name))),
        }
        self.masks.insert(spec.id, Image::from_frame(frame)?);
        Ok(())
    }

    pub fn set_sample_frame(&mut self, frame: &Frame) -> ApplyResult<()> {
        self.sample = Some(Image::from_frame(frame)?);
        Ok(())
    }

    fn param(&self, id: &str) -> ApplyResult<&'static crate::spec::ParamSpec> {
        self.def
            .param(id)
            .ok_or_else(|| ApplyError::new(format!("未知のパラメータ {id}")))
    }
}

fn stored_default(default: KnownDefault) -> Option<StoredValue> {
    match default {
        KnownDefault::Float(value) => Some(StoredValue::Float(value)),
        KnownDefault::Angle(value) => Some(StoredValue::Angle(value)),
        KnownDefault::Checkbox(value) => Some(StoredValue::Checkbox(value)),
        KnownDefault::Color(value) => Some(StoredValue::Color(value)),
        KnownDefault::Point(value) => Some(StoredValue::Point(value)),
        KnownDefault::Choice(value) => Some(StoredValue::Choice(value.to_string())),
        KnownDefault::Unknown
        | KnownDefault::NoLayer
        | KnownDefault::CurrentLayer
        | KnownDefault::NoMask
        | KnownDefault::FirstFrame => None,
    }
}

impl CpuApply for EffectInstance {
    fn apply_cpu(&self, input: &CpuInput<'_>, output: &mut Frame) -> ApplyResult<()> {
        if !self.def.allows(input.bits) {
            return Err(ApplyError::new(format!(
                "{} は {} bpc では動かない",
                self.def.name,
                input.bits.bits()
            )));
        }
        let source = Image::from_frame(input.frame)?;
        let mut values = self.values.clone();
        for (index, spec) in self
            .def
            .params
            .iter()
            .filter(|param| param.is_published())
            .enumerate()
        {
            if let Some(value) = input.params.get(ParamIndex::new(index as u32)) {
                if let Some(stored) = stored_from_value(value) {
                    values.insert(spec.id, stored);
                }
            }
        }
        let ctx = ApplyCtx {
            time: input.time,
            bits: input.bits,
            source: &source,
            values: &values,
            layers: &self.layers,
            masks: &self.masks,
            sample: self.sample.as_ref(),
            def: self.def,
        };
        let image = (self.def.apply)(&ctx)?;
        *output = image.to_frame();
        Ok(())
    }
}

fn stored_from_value(value: &ParamValue) -> Option<StoredValue> {
    match value {
        ParamValue::Float(value) => Some(StoredValue::Float(*value)),
        ParamValue::Angle(value) => Some(StoredValue::Angle(*value)),
        ParamValue::Checkbox(value) => Some(StoredValue::Checkbox(*value)),
        ParamValue::Color(value) => Some(StoredValue::Color(*value)),
        ParamValue::Point(value) => Some(StoredValue::Point(*value)),
        ParamValue::Popup(_) | ParamValue::Layer(_) => None,
    }
}

pub struct EffectStack {
    items: Vec<EffectInstance>,
}

impl EffectStack {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn add(&mut self, id: &str) -> ApplyResult<()> {
        let def = effect(id).ok_or_else(|| ApplyError::new(format!("未知の効果 {id}")))?;
        self.items.push(EffectInstance::new(def));
        Ok(())
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut EffectInstance> {
        self.items.get_mut(index)
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl Default for EffectStack {
    fn default() -> Self {
        Self::new()
    }
}

impl CpuApply for EffectStack {
    fn apply_cpu(&self, input: &CpuInput<'_>, output: &mut Frame) -> ApplyResult<()> {
        if self.items.is_empty() {
            copy_frame(input.frame, output);
            return Ok(());
        }
        let mut current = input.frame.clone();
        let empty = ParamValues::new();
        for item in &self.items {
            let mut next = current.clone();
            let nested = CpuInput {
                time: input.time,
                bits: current.bits(),
                params: &empty,
                frame: &current,
            };
            item.apply_cpu(&nested, &mut next)?;
            current = next;
        }
        *output = current;
        Ok(())
    }
}

pub fn published_defaults(def: &EffectDef) -> ParamValues {
    defaults_from(&def.param_list())
}
