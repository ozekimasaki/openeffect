use effect_api::{CpuApply, CpuInput, Frame, Frame32, Param, ParamValues};
use model::{
    BitsPerChannel, Ids, Layer, LayerId, LayerKind, PropertyBody, PropertyValue, Rgba, Time,
};

use crate::catalog::{
    effect_api_params, effects, ButtonAct, Category, EffectId, Keyframes, Kind, Unit,
};
use crate::cx::RenderCx;
use crate::hosted::AppliedEffect;
use crate::image::{Audio, Image};
use crate::stack::{add_effect, menu_effects};
use crate::values::{EffectValues, Slot};
use crate::{render, Rendered};

pub fn check_stack(index: usize) {
    let def = &effects()[index];
    let id = EffectId(index);
    let menus = menu_effects();
    match &def.menu {
        Some(menu) => assert!(
            menus
                .iter()
                .any(|(menu_id, path)| menu_id.0 == index && *path == menu),
            "{} がメニューに無い",
            def.name
        ),
        None => assert!(
            menus.iter().all(|(menu_id, _)| menu_id.0 != index),
            "{} のメニュー位置は要確認",
            def.name
        ),
    }

    let mut ids = Ids::new();
    let mut layer = Layer::new(ids.layer(), "layer", LayerKind::Null);
    let property_id = add_effect(&mut layer, &mut ids, id).expect("効果を追加できる");
    let group = layer
        .properties
        .iter()
        .find(|property| property.id == property_id)
        .expect("スタックに載る");
    assert_eq!(group.name, def.name);
    let PropertyBody::Group { children } = &group.body else {
        panic!("{} はグループ", def.name);
    };
    assert_eq!(children.len(), def.params.len(), "{}", def.name);
    let initial = EffectValues::initial(id);
    for (param_index, param) in def.params.iter().enumerate() {
        let child = &children[param_index];
        assert_eq!(child.name, param.name, "{}", def.name);
        match (&param.kind, &child.body) {
            (Kind::Group, PropertyBody::Group { children }) => assert!(children.is_empty()),
            (_, PropertyBody::Value { value, .. }) => {
                assert_eq!(
                    value,
                    &slot_property(initial.get(param_index).unwrap()),
                    "{} / {}",
                    def.name,
                    param.name
                );
            }
            (kind, _) => panic!("{} / {} の型が一致しない: {kind:?}", def.name, param.name),
        }
    }

    let api = effect_api_params(id);
    let specified: Vec<_> = def
        .params
        .iter()
        .filter(|param| fully_specified(param))
        .collect();
    assert_eq!(api.len(), specified.len(), "{}", def.name);
    for ((_, api_param), param) in api.iter().zip(specified) {
        assert_eq!(api_param.name(), param.name, "{}", def.name);
        assert_api_matches(api_param, param);
    }

    let frame = Frame::Bpc32(Frame32::new(2, 2, 4, 0.25));
    let mut output = frame.clone();
    let hosted = AppliedEffect {
        id,
        values: EffectValues::initial(id),
    };
    hosted
        .apply_cpu(
            &CpuInput {
                time: Time::ZERO,
                bits: BitsPerChannel::Bpc32,
                params: &ParamValues::new(),
                frame: &frame,
            },
            &mut output,
        )
        .expect("CPU で適用できる");
    assert_eq!(output.width(), 2);
}

pub fn check_param(effect_index: usize, param_index: usize) {
    let def = &effects()[effect_index];
    let param = &def.params[param_index];
    let id = EffectId(effect_index);
    assert_bounds_reject(id, param_index, param);
    if param.name == "Speed" && def.name == "Timewarp" {
        assert_eq!(param.keyframes, Keyframes::Yes);
    }

    match expectation(def, param) {
        Expect::Meta => {
            let mut values = EffectValues::empty(id);
            assert!(
                values.set_number(param_index, 1.0).is_err(),
                "{} / {} は未確定のまま",
                def.name,
                param.name
            );
        }
        Expect::Expose => {
            let scene = scene();
            let a = render_scene(
                id,
                &fill(id, Some((param_index, false))),
                &scene,
                false,
                None,
            );
            let b = render_scene(
                id,
                &fill(id, Some((param_index, true))),
                &scene,
                false,
                None,
            );
            assert!(
                !image_changed(&a.image, &b.image),
                "{} / {} は映像を変えない",
                def.name,
                param.name
            );
            assert_ne!(a.exposed, b.exposed, "{} / {}", def.name, param.name);
        }
        Expect::Audio => {
            let scene = scene();
            let a = render_scene(
                id,
                &fill(id, Some((param_index, false))),
                &scene,
                false,
                None,
            );
            let b = render_scene(
                id,
                &fill(id, Some((param_index, true))),
                &scene,
                false,
                None,
            );
            assert!(
                audio_changed(a.audio.as_ref(), b.audio.as_ref()),
                "{} / {} で音が変わらない",
                def.name,
                param.name
            );
        }
        Expect::Image => {
            let scene = scene();
            let a = render_scene(
                id,
                &fill(id, Some((param_index, false))),
                &scene,
                false,
                None,
            );
            let b = render_scene(
                id,
                &fill(id, Some((param_index, true))),
                &scene,
                false,
                None,
            );
            assert!(
                image_changed(&a.image, &b.image),
                "{} / {} で絵が変わらない {:?} {:?}",
                def.name,
                param.name,
                &a.image.px[..4.min(a.image.px.len())],
                &b.image.px[..4.min(b.image.px.len())]
            );
            if param.name == "Columns" {
                assert_columns_follow_rows(id, &scene);
            }
            if param.name == "Transition Completion" || param.name == "Completion" {
                assert_full_completion_clears(id, param_index, &scene);
            }
        }
        Expect::Clear => {
            let scene = scene();
            let values = fill(id, None);
            let kept = render_scene(id, &values, &scene, false, None);
            let cleared = render_scene(id, &values, &scene, true, None);
            assert!(
                image_changed(&kept.image, &cleared.image),
                "{} / Clear で絵が変わらない",
                def.name
            );
        }
        Expect::Lut => {
            let scene = scene();
            let values = fill(id, None);
            let a = render_scene(id, &values, &scene, false, Some(&scene.lut_a));
            let b = render_scene(id, &values, &scene, false, Some(&scene.lut_b));
            assert!(
                image_changed(&a.image, &b.image),
                "{} / Choose LUT で絵が変わらない",
                def.name
            );
        }
    }
}

enum Expect {
    Meta,
    Image,
    Audio,
    Expose,
    Clear,
    Lut,
}

fn expectation(effect: &crate::catalog::EffectDef, param: &crate::catalog::ParamDef) -> Expect {
    match &param.kind {
        Kind::Button {
            act: ButtonAct::Clear,
        } => Expect::Clear,
        Kind::Button {
            act: ButtonAct::ChooseLut,
        } => Expect::Lut,
        Kind::Button {
            act: ButtonAct::Dialog,
        }
        | Kind::Group
        | Kind::Dialog
        | Kind::Direction
        | Kind::Ambiguous
        | Kind::Unconfirmed => Expect::Meta,
        Kind::Menu { choices, .. } if choices.len() < 2 => Expect::Meta,
        Kind::Number { min, max, .. } if number_probes(*min, *max).is_none() => Expect::Meta,
        _ => match effect.category {
            Category::Expression => Expect::Expose,
            Category::Audio => Expect::Audio,
            Category::Simulation
            | Category::Channel3d
            | Category::Time
            | Category::Transition
            | Category::Utility
            | Category::Obsolete => Expect::Image,
        },
    }
}

fn fill(id: EffectId, vary: Option<(usize, bool)>) -> EffectValues {
    let mut values = EffectValues::empty(id);
    let effect = &effects()[id.0];
    for (index, param) in effect.params.iter().enumerate() {
        let side_b = vary.is_some_and(|(vary_index, side_b)| vary_index == index && side_b);
        match &param.kind {
            Kind::Number { min, max, unit, .. } => {
                if let Some((low, high)) = number_probes(*min, *max) {
                    let mut value = if side_b { high } else { low };
                    if matches!(unit, Unit::Integer) {
                        value = value.round();
                    }
                    values.set_number(index, value).expect(param.name.as_str());
                }
            }
            Kind::Angle { .. } => {
                values
                    .set_number(index, if side_b { 70.0 } else { 20.0 })
                    .expect(param.name.as_str());
            }
            Kind::Checkbox { .. } => {
                values.set_bool(index, side_b).expect(param.name.as_str());
            }
            Kind::Color { .. } => {
                let color = if side_b {
                    Rgba::new(0.1, 0.2, 0.9, 1.0)
                } else {
                    Rgba::new(0.9, 0.15, 0.1, 1.0)
                };
                values.set_color(index, color).expect(param.name.as_str());
            }
            Kind::Point { .. } => {
                let point = if side_b { [5.5, 4.0] } else { [1.5, 1.5] };
                values.set_point(index, point).expect(param.name.as_str());
            }
            Kind::Point3 { .. } => {
                let point = if side_b {
                    [0.8, 0.2, 1.4]
                } else {
                    [0.2, 0.4, 0.3]
                };
                values.set_point3(index, point).expect(param.name.as_str());
            }
            Kind::Menu { choices, .. } if choices.len() >= 2 => {
                values
                    .set_index(index, if side_b { 1 } else { 0 })
                    .expect(param.name.as_str());
            }
            Kind::Layer => {
                let layer = if side_b {
                    LayerId::new(22)
                } else {
                    LayerId::new(11)
                };
                values
                    .set_layer(index, Some(layer))
                    .expect(param.name.as_str());
            }
            Kind::Text => {
                values
                    .set_text(index, if side_b { "South" } else { "North" })
                    .expect(param.name.as_str());
            }
            Kind::MaskPath => {
                let path = if side_b {
                    vec![[6.0, 1.0], [1.0, 6.0]]
                } else {
                    vec![[1.0, 1.0], [4.0, 2.0], [2.0, 5.0]]
                };
                values.set_path(index, path).expect(param.name.as_str());
            }
            Kind::NumberSet => {
                let numbers = if side_b {
                    vec![4.0, 1.0, 2.0, 8.0]
                } else {
                    vec![1.0, 0.0, 1.0, 0.0]
                };
                values
                    .set_numbers(index, numbers)
                    .expect(param.name.as_str());
            }
            Kind::Menu { .. }
            | Kind::Button { .. }
            | Kind::Group
            | Kind::Dialog
            | Kind::Direction
            | Kind::Ambiguous
            | Kind::Unconfirmed => {}
        }
    }
    values
}

fn number_probes(min: Option<f64>, max: Option<f64>) -> Option<(f64, f64)> {
    let inside = |value: f64| {
        min.is_none_or(|min| value + 1e-9 >= min) && max.is_none_or(|max| value - 1e-9 <= max)
    };
    if inside(2.0) && inside(7.0) {
        return Some((2.0, 7.0));
    }
    match (min, max) {
        (Some(min), Some(max)) if (max - min).abs() > 1e-9 => Some((min, max)),
        (None, Some(max)) => Some((max - 5.0, max)),
        (Some(min), None) => Some((min, min + 5.0)),
        _ => None,
    }
}

fn assert_bounds_reject(id: EffectId, index: usize, param: &crate::catalog::ParamDef) {
    let Kind::Number { min, max, .. } = &param.kind else {
        return;
    };
    let mut values = EffectValues::empty(id);
    if let Some(max) = max {
        assert!(
            values.set_number(index, max + 1.0).is_err(),
            "{} の上限",
            param.name
        );
    }
    if let Some(min) = min {
        assert!(
            values.set_number(index, min - 1.0).is_err(),
            "{} の下限",
            param.name
        );
    }
}

fn assert_columns_follow_rows(id: EffectId, scene: &Scene) {
    let effect = &effects()[id.0];
    let mode = effect
        .params
        .iter()
        .position(|param| param.name == "Rows & Columns")
        .expect("Rows & Columns");
    let rows = effect
        .params
        .iter()
        .position(|param| param.name == "Rows")
        .expect("Rows");
    let cols = effect
        .params
        .iter()
        .position(|param| param.name == "Columns")
        .expect("Columns");
    let mut narrow = fill(id, None);
    let mut wide = fill(id, None);
    narrow.set_index(mode, 1).unwrap();
    wide.set_index(mode, 1).unwrap();
    narrow.set_number(rows, 4.0).unwrap();
    wide.set_number(rows, 4.0).unwrap();
    narrow.set_number(cols, 2.0).unwrap();
    wide.set_number(cols, 9.0).unwrap();
    let a = render_scene(id, &narrow, scene, false, None);
    let b = render_scene(id, &wide, scene, false, None);
    assert!(
        !image_changed(&a.image, &b.image),
        "Columns Follows Rows のときは Columns が絵を変えない"
    );
}

fn assert_full_completion_clears(id: EffectId, index: usize, scene: &Scene) {
    let mut values = EffectValues::empty(id);
    values.set_number(index, 100.0).unwrap();
    let rendered = render_scene(id, &values, scene, false, None);
    let leftover = rendered
        .image
        .px
        .iter()
        .fold(0.0f32, |max, sample| max.max(sample.abs()));
    assert!(
        leftover < 1e-3,
        "{} の 100% でレイヤーが残る",
        effects()[id.0].name
    );
}

struct Scene {
    src: Image,
    layers: Vec<(LayerId, Image)>,
    history: Vec<(f64, Image)>,
    audio: Audio,
    depth: Image,
    object_id: Image,
    material_id: Image,
    accum: Image,
    lut_a: Vec<f32>,
    lut_b: Vec<f32>,
}

fn scene() -> Scene {
    let mut src = Image::new(8, 8, [0.0; 4]);
    for y in 0..8 {
        for x in 0..8 {
            src.put(
                x,
                y,
                [
                    x as f32 / 7.0,
                    y as f32 / 7.0,
                    ((x + y) % 8) as f32 / 7.0,
                    1.0,
                ],
            );
        }
    }
    src.put(0, 0, [1.8, 0.2, 0.2, 1.0]);
    let mut red = Image::new(8, 8, [0.0; 4]);
    let mut green = Image::new(8, 8, [0.0; 4]);
    for y in 0..8 {
        for x in 0..8 {
            red.put(x, y, [0.2 + x as f32 / 10.0, 0.05, 0.05, 1.0]);
            green.put(x, y, [0.05, if y % 2 == 0 { 0.8 } else { 0.1 }, 0.2, 1.0]);
        }
    }
    let mut depth = Image::new(8, 8, [0.0; 4]);
    let mut object_id = Image::new(8, 8, [0.0; 4]);
    let mut material_id = Image::new(8, 8, [0.0; 4]);
    for y in 0..8 {
        for x in 0..8 {
            depth.put(x, y, [x as f32 / 7.0, 0.0, 0.0, 1.0]);
            let id = if x < 4 { 0.2 } else { 0.8 };
            object_id.put(x, y, [id, 0.0, 0.0, 1.0]);
            material_id.put(x, y, [if y < 4 { 0.1 } else { 0.9 }, 0.0, 0.0, 1.0]);
        }
    }
    let mut history = Vec::new();
    for step in 0..33 {
        let time = -8.0 + step as f64 * 0.5;
        let mut frame = Image::new(8, 8, [0.0; 4]);
        let color = [
            (step % 5) as f32 / 4.0,
            (step % 7) as f32 / 6.0,
            (step % 3) as f32 / 2.0,
            1.0,
        ];
        for y in 0..8 {
            for x in 0..8 {
                frame.put(x, y, color);
            }
        }
        history.push((time, frame));
    }
    let mut samples = Vec::with_capacity(256 * 2);
    for frame in 0..256 {
        let time = frame as f32 / 16_000.0;
        let env = if frame < 30 {
            0.08
        } else if frame < 140 {
            0.85
        } else {
            0.12
        };
        let low = (time * 120.0 * std::f32::consts::TAU).sin();
        let high = (time * 5_500.0 * std::f32::consts::TAU).sin();
        samples.push(env * (0.55 * low + 0.45 * high));
        samples.push(env * (0.25 * low + 0.75 * high));
    }
    Scene {
        src,
        layers: vec![(LayerId::new(11), red), (LayerId::new(22), green)],
        history,
        audio: Audio {
            rate: 16_000,
            channels: 2,
            samples,
        },
        depth,
        object_id,
        material_id,
        accum: Image::new(8, 8, [0.15, 0.05, 0.55, 1.0]),
        lut_a: vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
        lut_b: vec![1.0, 0.0, 0.2, 0.0, 0.1, 1.0],
    }
}

fn render_scene(
    id: EffectId,
    values: &EffectValues,
    scene: &Scene,
    clear: bool,
    lut: Option<&[f32]>,
) -> Rendered {
    let layers: Vec<(LayerId, &Image)> = scene
        .layers
        .iter()
        .map(|(id, image)| (*id, image))
        .collect();
    let history: Vec<(f64, &Image)> = scene
        .history
        .iter()
        .map(|(time, image)| (*time, image))
        .collect();
    let cx = RenderCx {
        time: 0.8,
        frame_rate: 24.0,
        bits: BitsPerChannel::Bpc32,
        layers: &layers,
        history: &history,
        audio: Some(&scene.audio),
        depth: Some(&scene.depth),
        object_id: Some(&scene.object_id),
        material_id: Some(&scene.material_id),
        accum: Some(&scene.accum),
        lut,
        clear,
    };
    render(id, &scene.src, values, &cx).expect("適用")
}

fn image_changed(a: &Image, b: &Image) -> bool {
    a.width != b.width
        || a.height != b.height
        || a.px.len() != b.px.len()
        || a.px
            .iter()
            .zip(&b.px)
            .any(|(left, right)| (left - right).abs() > 1.0e-3)
}

fn audio_changed(a: Option<&Audio>, b: Option<&Audio>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => {
            a.samples.len() != b.samples.len()
                || a.samples
                    .iter()
                    .zip(&b.samples)
                    .any(|(left, right)| (left - right).abs() > 1.0e-4)
        }
        _ => false,
    }
}

fn slot_property(slot: &Slot) -> PropertyValue {
    match slot {
        Slot::Empty | Slot::Numbers(_) => PropertyValue::Empty,
        Slot::Number(value) => PropertyValue::Scalar(*value),
        Slot::Bool(value) => PropertyValue::Boolean(*value),
        Slot::Color(value) => PropertyValue::Color(*value),
        Slot::Point(value) => PropertyValue::TwoD(*value),
        Slot::Point3(value) => PropertyValue::ThreeD(*value),
        Slot::Index(value) => PropertyValue::Index(*value as i32),
        Slot::Layer(value) => PropertyValue::Layer(*value),
        Slot::Text(value) => PropertyValue::Text(value.clone()),
        Slot::Path(vertices) => PropertyValue::path(crate::values::path_of(vertices)),
    }
}

fn fully_specified(param: &crate::catalog::ParamDef) -> bool {
    match &param.kind {
        Kind::Number {
            default: Some(_),
            min: Some(_),
            max: Some(_),
            ..
        }
        | Kind::Angle { default: Some(_) }
        | Kind::Checkbox { default: Some(_) } => true,
        Kind::Menu {
            choices,
            complete: true,
            default_index: Some(index),
        } => !choices.is_empty() && (*index as usize) < choices.len(),
        _ => false,
    }
}

fn assert_api_matches(api: &Param, param: &crate::catalog::ParamDef) {
    match (&param.kind, api) {
        (
            Kind::Number {
                default: Some(default),
                min: Some(min),
                max: Some(max),
                ..
            },
            Param::Float {
                default: api_default,
                min: api_min,
                max: api_max,
                ..
            },
        ) => {
            assert_eq!(api_default, default);
            assert_eq!(api_min, min);
            assert_eq!(api_max, max);
        }
        (
            Kind::Checkbox {
                default: Some(default),
            },
            Param::Checkbox {
                default: api_default,
                ..
            },
        ) => {
            assert_eq!(api_default, default);
        }
        (
            Kind::Menu {
                choices,
                default_index: Some(index),
                ..
            },
            Param::Popup {
                choices: api_choices,
                default_index,
                ..
            },
        ) => {
            assert_eq!(api_choices, choices);
            assert_eq!(default_index, index);
        }
        (kind, api) => panic!("{kind:?} を {:?} にした", api.name()),
    }
}
