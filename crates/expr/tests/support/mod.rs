#![allow(dead_code)]

use expr::{Scene, Target, Value};
use model::{
    BitsPerChannel, ItemId, Keyframe, LayerId, Marker, PropertyId, PropertyValue, Rgba, ShapePath,
    Time,
};

pub struct Env {
    pub scene: Scene,
    pub comp: ItemId,
    pub layer: LayerId,
    pub prop: PropertyId,
    pub time: f64,
}

impl Env {
    pub fn new(name: &str) -> Self {
        let mut scene = Scene::new(BitsPerChannel::Bpc8);
        let comp = scene.add_comp(name, 1920, 1080, 25.0, 10.0);
        let layer = scene.add_solid(comp, "Layer 1", 1920, 1080);
        let prop = scene.add_value(
            comp,
            layer,
            None,
            "Opacity",
            "opacity",
            PropertyValue::Scalar(100.0),
        );
        Self {
            scene,
            comp,
            layer,
            prop,
            time: 0.0,
        }
    }

    pub fn target(&self) -> Target {
        Target {
            comp: self.comp,
            layer: self.layer,
            property: self.prop,
            time: self.time,
        }
    }

    pub fn eval(&mut self, source: &str) -> Result<Value, expr::ExprFailure> {
        let target = self.target();
        self.scene.eval(target, source)
    }

    pub fn num(&mut self, source: &str) -> f64 {
        number(
            self.eval(source)
                .unwrap_or_else(|error| panic!("{source}: {error:?}")),
        )
    }

    pub fn text(&mut self, source: &str) -> String {
        match self
            .eval(source)
            .unwrap_or_else(|error| panic!("{source}: {error:?}"))
        {
            Value::String(text) => text,
            other => panic!("{source}: {other:?}"),
        }
    }

    pub fn flag(&mut self, source: &str) -> bool {
        match self
            .eval(source)
            .unwrap_or_else(|error| panic!("{source}: {error:?}"))
        {
            Value::Bool(value) => value,
            other => panic!("{source}: {other:?}"),
        }
    }

    pub fn vec(&mut self, source: &str) -> Vec<f64> {
        coords(
            self.eval(source)
                .unwrap_or_else(|error| panic!("{source}: {error:?}")),
        )
    }
}

pub fn number(value: Value) -> f64 {
    value.number().unwrap_or_else(|| panic!("{value:?}"))
}

pub fn coords(value: Value) -> Vec<f64> {
    value
        .array()
        .unwrap_or_else(|| panic!("{value:?}"))
        .iter()
        .map(|item| item.number().unwrap())
        .collect()
}

pub fn close(left: f64, right: f64) {
    assert!((left - right).abs() < 1e-6, "{left} != {right}");
}

pub fn close_vec(left: &[f64], right: &[f64]) {
    assert_eq!(left.len(), right.len());
    for (left, right) in left.iter().zip(right) {
        close(*left, *right);
    }
}

pub fn scalar_keys(
    scene: &mut Scene,
    comp: ItemId,
    layer: LayerId,
    prop: PropertyId,
    keys: &[(f64, f64)],
) {
    for (time, value) in keys {
        scene.add_key(comp, layer, prop, *time, PropertyValue::Scalar(*value));
    }
}

pub fn add_position(
    scene: &mut Scene,
    comp: ItemId,
    layer: LayerId,
    value: [f64; 2],
) -> PropertyId {
    let group = scene.add_group(comp, layer, None, "Transform", "transform");
    scene.add_value(
        comp,
        layer,
        Some(group),
        "Position",
        "position",
        PropertyValue::TwoD(value),
    )
}

pub fn marker_at(comment: &str, time: f64) -> (f64, Marker) {
    (time, Marker::new(comment))
}

pub fn path_line() -> ShapePath {
    let mut path = ShapePath::new();
    path.vertices = vec![[0.0, 0.0], [100.0, 0.0]];
    path.closed = false;
    path
}

pub fn key(time: f64, value: PropertyValue) -> Keyframe {
    Keyframe::at(Time::from_seconds(time), value)
}

pub fn white() -> Rgba {
    Rgba::white()
}
