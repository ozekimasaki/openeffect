use model::{Keyframe, Property, PropertyBody, PropertyValue, Time};

/// 時刻から値を取る。
pub trait Sampler {
    type Value;

    fn sample(&self, time: Time) -> Self::Value;
}

/// 時刻に依らない値。
#[derive(Clone, Debug, PartialEq)]
pub struct Constant<T> {
    value: T,
}

impl<T> Constant<T> {
    pub const fn new(value: T) -> Self {
        Self { value }
    }

    pub const fn value(&self) -> &T {
        &self.value
    }
}

impl<T: Clone> Sampler for Constant<T> {
    type Value = T;

    fn sample(&self, _time: Time) -> T {
        self.value.clone()
    }
}

/// プロパティの静的値か、その時刻以前で最も新しいキー。
#[derive(Clone, Copy, Debug)]
pub struct PropertySampler<'a> {
    property: &'a Property,
}

impl<'a> PropertySampler<'a> {
    pub const fn new(property: &'a Property) -> Self {
        Self { property }
    }
}

impl Sampler for PropertySampler<'_> {
    type Value = PropertyValue;

    fn sample(&self, time: Time) -> PropertyValue {
        sample_property(self.property, time)
    }
}

pub fn sample_property(property: &Property, time: Time) -> PropertyValue {
    match &property.body {
        PropertyBody::Group { .. } => PropertyValue::Empty,
        PropertyBody::Value {
            value, keyframes, ..
        } => sample_keyframes(value, keyframes, time),
    }
}

/// キーが無いときは静的値。時刻が最初のキーより前なら最初のキー。それ以外は時刻以前で最も新しいキー。
pub fn sample_keyframes(base: &PropertyValue, keys: &[Keyframe], time: Time) -> PropertyValue {
    if keys.is_empty() {
        return base.clone();
    }
    let mut earliest = &keys[0];
    let mut latest_at_or_before: Option<&Keyframe> = None;
    for key in keys {
        if key.time < earliest.time {
            earliest = key;
        }
        if key.time <= time {
            match latest_at_or_before {
                Some(chosen) if key.time < chosen.time => {}
                _ => latest_at_or_before = Some(key),
            }
        }
    }
    match latest_at_or_before {
        Some(key) => key.value.clone(),
        None => earliest.value.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::{sample_keyframes, Constant, Sampler};
    use model::{Keyframe, PropertyValue, Time};

    #[test]
    fn constant_ignores_time() {
        let sampler = Constant::new(PropertyValue::Scalar(3.0));
        assert_eq!(
            sampler.sample(Time::from_seconds(10.0)),
            PropertyValue::Scalar(3.0)
        );
    }

    #[test]
    fn steps_to_previous_key() {
        let base = PropertyValue::Scalar(0.0);
        let keys = vec![
            Keyframe::at(Time::from_seconds(1.0), PropertyValue::Scalar(10.0)),
            Keyframe::at(Time::from_seconds(3.0), PropertyValue::Scalar(30.0)),
        ];
        assert_eq!(
            sample_keyframes(&base, &keys, Time::from_seconds(0.0)),
            PropertyValue::Scalar(10.0)
        );
        assert_eq!(
            sample_keyframes(&base, &keys, Time::from_seconds(2.0)),
            PropertyValue::Scalar(10.0)
        );
        assert_eq!(
            sample_keyframes(&base, &keys, Time::from_seconds(3.0)),
            PropertyValue::Scalar(30.0)
        );
    }
}
