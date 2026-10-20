use model::{Ids, Layer, Property, PropertyValue};

use crate::catalog::{effects, EffectDef, EffectId, Kind};
use crate::values::{path_of, EffectValues, Slot};

/// メニュー位置が確定している効果。
pub fn menu_effects() -> Vec<(EffectId, &'static str)> {
    effects()
        .iter()
        .enumerate()
        .filter_map(|(index, effect)| effect.menu.as_deref().map(|menu| (EffectId(index), menu)))
        .collect()
}

/// レイヤーの効果スタックへ足す。初期値は確定したものだけ。
pub fn add_effect(layer: &mut Layer, ids: &mut Ids, effect: EffectId) -> Option<model::PropertyId> {
    let def = effects().get(effect.0)?;
    let values = EffectValues::initial(effect);
    let mut group = Property::group(ids.property(), def.name.clone(), effect_match(layer, def));
    for (index, param) in def.params.iter().enumerate() {
        group.push_child(param_property(
            ids,
            def,
            param,
            values.get(index).unwrap_or(&Slot::Empty),
        ));
    }
    let id = group.id;
    layer.properties.push(group);
    Some(id)
}

fn effect_match(layer: &Layer, effect: &EffectDef) -> String {
    let base = format!("effect:{}", effect.name);
    let taken = layer.property(&base).is_some();
    if taken {
        format!("{base}#{}", layer.properties.len())
    } else {
        base
    }
}

fn param_property(
    ids: &mut Ids,
    effect: &EffectDef,
    param: &crate::catalog::ParamDef,
    slot: &Slot,
) -> Property {
    let match_name = format!("param:{}:{}", effect.name, param.name);
    if matches!(param.kind, Kind::Group) {
        return Property::group(ids.property(), param.name.clone(), match_name);
    }
    Property::value(
        ids.property(),
        param.name.clone(),
        match_name,
        slot_value(slot),
    )
}

fn slot_value(slot: &Slot) -> PropertyValue {
    match slot {
        Slot::Empty => PropertyValue::Empty,
        Slot::Number(value) => PropertyValue::Scalar(*value),
        Slot::Bool(value) => PropertyValue::Boolean(*value),
        Slot::Color(value) => PropertyValue::Color(*value),
        Slot::Point(value) => PropertyValue::TwoD(*value),
        Slot::Point3(value) => PropertyValue::ThreeD(*value),
        Slot::Index(value) => PropertyValue::Index(*value as i32),
        Slot::Layer(value) => PropertyValue::Layer(*value),
        Slot::Text(value) => PropertyValue::Text(value.clone()),
        Slot::Path(vertices) => PropertyValue::path(path_of(vertices)),
        Slot::Numbers(_) => PropertyValue::Empty,
    }
}
