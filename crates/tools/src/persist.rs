//! レイヤーのプロパティへツール結果を残す。

use model::{Ids, Keyframe, Layer, Property, PropertyValue, Time};

pub fn group(ids: &mut Ids, name: &str, match_name: &str) -> Property {
    Property::group(ids.property(), name, match_name)
}

pub fn scalar(ids: &mut Ids, name: &str, match_name: &str, value: f64) -> Property {
    Property::value(
        ids.property(),
        name,
        match_name,
        PropertyValue::Scalar(value),
    )
}

pub fn point(ids: &mut Ids, name: &str, match_name: &str, value: [f64; 2]) -> Property {
    Property::value(ids.property(), name, match_name, PropertyValue::TwoD(value))
}

pub fn text(ids: &mut Ids, name: &str, match_name: &str, value: &str) -> Property {
    Property::value(
        ids.property(),
        name,
        match_name,
        PropertyValue::Text(value.to_string()),
    )
}

pub fn boolean(ids: &mut Ids, name: &str, match_name: &str, value: bool) -> Property {
    Property::value(
        ids.property(),
        name,
        match_name,
        PropertyValue::Boolean(value),
    )
}

pub fn push_scalar_keys(property: &mut Property, keys: &[(i32, f64)], frame_rate: f64) {
    for (frame, value) in keys {
        property.push_keyframe(Keyframe::at(
            Time::from_frames(f64::from(*frame), frame_rate),
            PropertyValue::Scalar(*value),
        ));
    }
}

pub fn push_point_keys(property: &mut Property, keys: &[(f64, [f64; 2])], frame_rate: f64) {
    for (frame, value) in keys {
        property.push_keyframe(Keyframe::at(
            Time::from_frames(*frame, frame_rate),
            PropertyValue::TwoD(*value),
        ));
    }
}

pub fn ensure_group<'a>(
    layer: &'a mut Layer,
    ids: &mut Ids,
    name: &str,
    match_name: &str,
) -> &'a mut Property {
    if layer.property(match_name).is_none() {
        layer.properties.push(group(ids, name, match_name));
    }
    layer.property_mut(match_name).expect("group was inserted")
}

pub fn child_group<'a>(
    parent: &'a mut Property,
    ids: &mut Ids,
    name: &str,
    match_name: &str,
) -> &'a mut Property {
    if parent.child(match_name).is_none() {
        parent.push_child(group(ids, name, match_name));
    }
    parent.child_mut(match_name).expect("child was inserted")
}

#[cfg(test)]
pub fn key_count(layer: &Layer, match_name: &str) -> usize {
    layer
        .property(match_name)
        .map(|property| property.keyframes().len())
        .unwrap_or(0)
}

#[cfg(test)]
pub fn point_keys(layer: &Layer, match_name: &str) -> Vec<[f64; 2]> {
    layer
        .property(match_name)
        .map(|property| {
            property
                .keyframes()
                .iter()
                .filter_map(|key| match key.value {
                    PropertyValue::TwoD(value) => Some(value),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
pub fn scalar_keys(layer: &Layer, match_name: &str) -> Vec<f64> {
    layer
        .property(match_name)
        .map(|property| {
            property
                .keyframes()
                .iter()
                .filter_map(|key| match key.value {
                    PropertyValue::Scalar(value) => Some(value),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default()
}
