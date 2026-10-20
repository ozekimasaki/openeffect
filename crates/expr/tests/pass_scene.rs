#[path = "support/mod.rs"]
mod support;

use expr::{Image, Value};
use model::{BitsPerChannel, LayerKind, LightKind, Marker, PropertyValue, ShapePath, Text};
use support::{close, close_vec, Env};

fn env() -> Env {
    Env::new("Main")
}

#[test]
fn layer_index_name_and_relative() {
    let mut env = env();
    let second = env.scene.add_solid(env.comp, "Solid 1", 10, 10);
    let third = env.scene.add_solid(env.comp, "Third", 10, 10);
    env.scene.layer_mut(env.comp, third).unwrap().name = "N".into();
    assert_eq!(env.text("thisComp.layer(3).name"), "N");
    let duplicate = env.scene.add_solid(env.comp, "Solid 1", 10, 10);
    let _ = duplicate;
    assert_eq!(env.text("thisComp.layer(\"Solid 1\").name"), "Solid 1");
    env.scene.layer_attr_mut(second).unwrap().enabled = true;
    env.time = 1.0;
    assert!(env.flag("thisComp.layer(thisLayer, 1).active"));
}

#[test]
fn comp_markers_and_counts() {
    let mut env = env();
    assert_eq!(env.num("thisComp.marker.numKeys"), 0.0);
    env.scene.comp_attr_mut(env.comp).unwrap().markers = vec![
        (2.0, Marker::new("late")),
        (0.5, Marker::new("0")),
        (4.0, Marker::new("far")),
    ];
    assert_eq!(env.num("thisComp.marker.numKeys"), 3.0);
    assert_eq!(env.num("thisComp.marker.key(1).time"), 0.5);
    assert_eq!(env.num("thisComp.marker.key(\"0\").time"), 0.5);
    assert_eq!(env.num("thisComp.marker.nearestKey(1).time"), 0.5);
    env.time = 3.5;
    assert_eq!(env.num("thisComp.marker.nearestKey(time).time"), 4.0);
    env.scene.layer_attr_mut(env.layer).unwrap().comment = "hero".into();
    assert_eq!(
        env.text("thisComp.layerByComment(\"hero\").name"),
        "Layer 1"
    );
    assert_eq!(env.num("thisComp.numLayers"), 1.0);
}

#[test]
fn comp_attributes() {
    let mut env = env();
    let camera = env.scene.add_camera(env.comp, "Cam");
    env.scene.layer_attr_mut(camera).unwrap().in_point = 0.0;
    env.scene.layer_attr_mut(camera).unwrap().out_point = 10.0;
    env.scene.layer_attr_mut(camera).unwrap().enabled = true;
    env.time = 1.0;
    assert_eq!(env.text("thisComp.activeCamera.name"), "Cam");
    assert_eq!(env.num("thisComp.width"), 1920.0);
    assert_eq!(env.num("thisComp.height"), 1080.0);
    assert_eq!(env.num("thisComp.duration"), 10.0);
    env.scene.comp_attr_mut(env.comp).unwrap().ntsc_drop_frame = true;
    assert!(env.flag("thisComp.ntscDropFrame"));
    env.scene.comp_attr_mut(env.comp).unwrap().ntsc_drop_frame = false;
    assert!(!env.flag("thisComp.ntscDropFrame"));
    env.scene
        .comp_attr_mut(env.comp)
        .unwrap()
        .display_start_time = 3600.0;
    assert_eq!(env.num("thisComp.displayStartTime"), 3600.0);
    close(env.num("thisComp.frameDuration"), 1.0 / 25.0);
    env.scene
        .project
        .composition_mut(env.comp)
        .unwrap()
        .motion_blur
        .shutter_angle_degrees = 180.0;
    env.scene
        .project
        .composition_mut(env.comp)
        .unwrap()
        .motion_blur
        .shutter_phase_degrees = -90.0;
    assert_eq!(env.num("thisComp.shutterAngle"), 180.0);
    assert_eq!(env.num("thisComp.shutterPhase"), -90.0);
    let color = env.vec("thisComp.bgColor");
    assert_eq!(color.len(), 4);
    assert!(color[0] > 0.0 && color[0] == color[1] && color[1] == color[2]);
    env.scene
        .project
        .composition_mut(env.comp)
        .unwrap()
        .pixel_aspect = 1.2;
    assert_eq!(env.num("thisComp.pixelAspect"), 1.2);
    assert_eq!(env.text("thisComp.name"), "Main");
}

#[test]
fn source_footage_and_precomp() {
    let mut env = env();
    let footage = env.scene.add_footage("Clip", 320, 240);
    let footage_layer = env.scene.add_layer(
        env.comp,
        "Clip Layer",
        LayerKind::Footage { source: footage },
    );
    env.layer = footage_layer;
    assert_eq!(env.text("source.name"), "Clip");
    assert_eq!(env.num("width"), 320.0);
    assert_eq!(env.num("source.width"), 320.0);
    assert_eq!(env.num("height"), 240.0);
    let inner = env.scene.add_comp("Nested", 640, 360, 25.0, 4.0);
    let pre = env
        .scene
        .add_layer(env.comp, "Pre", LayerKind::Precomp { source: inner });
    env.layer = pre;
    assert_eq!(env.text("source.name"), "Nested");
}

#[test]
fn source_time_and_rect() {
    let mut env = env();
    env.time = 2.0;
    assert_eq!(env.num("sourceTime(time)"), 2.0);
    env.scene
        .layer_attr_mut(env.layer)
        .unwrap()
        .source_rect
        .width = 80.0;
    env.scene.layer_attr_mut(env.layer).unwrap().extents = 6.0;
    assert_eq!(env.num("sourceRectAtTime().width"), 80.0);
    assert_eq!(env.num("sourceRectAtTime(time, false).width"), 80.0);
    assert_eq!(env.num("sourceRectAtTime(time, true).width"), 86.0);
    let text = env.scene.add_text_layer(env.comp, "Title", "Hello");
    env.scene.layer_attr_mut(text).unwrap().source_rect.width = 42.0;
    env.layer = text;
    assert_eq!(env.num("sourceRectAtTime().width"), 42.0);
}

#[test]
fn effects_masks_and_controls() {
    let mut env = env();
    let effects = env
        .scene
        .add_group(env.comp, env.layer, None, "Effects", "effects");
    let slider = env.scene.add_group(
        env.comp,
        env.layer,
        Some(effects),
        "Slider Control",
        "slider",
    );
    env.scene.add_value(
        env.comp,
        env.layer,
        Some(slider),
        "Slider",
        "Slider",
        PropertyValue::Scalar(12.0),
    );
    let other = env.scene.add_group(
        env.comp,
        env.layer,
        Some(effects),
        "Slider Control",
        "slider2",
    );
    env.scene.add_value(
        env.comp,
        env.layer,
        Some(other),
        "Slider",
        "Slider",
        PropertyValue::Scalar(99.0),
    );
    assert_eq!(env.num("effect(\"Slider Control\")(\"Slider\")"), 12.0);
    assert_eq!(env.num("effect(1)(\"Slider\")"), 12.0);
    assert_eq!(
        env.num("effect(\"Slider Control\").param(\"Slider\")"),
        12.0
    );
    assert_eq!(env.num("effect(\"Slider Control\").param(1)"), 12.0);
    let null_layer = env.scene.add_layer(env.comp, "Null 1", LayerKind::Null);
    let null_fx = env
        .scene
        .add_group(env.comp, null_layer, None, "Effects", "effects");
    let null_slider = env.scene.add_group(
        env.comp,
        null_layer,
        Some(null_fx),
        "Slider Control",
        "slider",
    );
    env.scene.add_value(
        env.comp,
        null_layer,
        Some(null_slider),
        "Slider",
        "Slider",
        PropertyValue::Scalar(7.0),
    );
    env.layer = null_layer;
    assert_eq!(
        env.num("thisComp.layer(\"Null 1\").effect(\"Slider Control\")(\"Slider\")"),
        7.0
    );
    if let Some(effect) = env
        .scene
        .layer(env.comp, null_layer)
        .unwrap()
        .properties
        .iter()
        .find(|p| p.match_name == "effects")
    {
        if let model::PropertyBody::Group { children } = &effect.body {
            let id = children[0].id;
            env.scene
                .property_mut(env.comp, null_layer, id)
                .unwrap()
                .name = "Amount".into();
        }
    }
    assert_eq!(env.num("effect(\"Amount\")(\"Slider\")"), 7.0);
    let angle = env
        .scene
        .add_group(env.comp, env.layer, Some(null_fx), "Angle Control", "angle");
    env.scene.add_value(
        env.comp,
        env.layer,
        Some(angle),
        "Angle",
        "Angle",
        PropertyValue::Scalar(35.0),
    );
    assert_eq!(env.num("effect(\"Angle Control\")(\"Angle\")"), 35.0);
    let check = env.scene.add_group(
        env.comp,
        env.layer,
        Some(null_fx),
        "Checkbox Control",
        "check",
    );
    let check_prop = env.scene.add_value(
        env.comp,
        env.layer,
        Some(check),
        "Checkbox",
        "Checkbox",
        PropertyValue::Scalar(0.0),
    );
    let off = env.num("effect(\"Checkbox Control\")(\"Checkbox\")");
    assert!(off == 0.0 || !env.flag("effect(\"Checkbox Control\")(\"Checkbox\")"));
    env.scene
        .property_mut(env.comp, env.layer, check_prop)
        .unwrap();
    if let model::PropertyBody::Value { value, .. } = &mut env
        .scene
        .property_mut(env.comp, env.layer, check_prop)
        .unwrap()
        .body
    {
        *value = PropertyValue::Scalar(1.0);
    }
    let on = env
        .eval("effect(\"Checkbox Control\")(\"Checkbox\")")
        .unwrap();
    assert!(matches!(on, Value::Number(n) if n == 1.0) || matches!(on, Value::Bool(true)));
    let color = env
        .scene
        .add_group(env.comp, env.layer, Some(null_fx), "Color Control", "color");
    env.scene.add_value(
        env.comp,
        env.layer,
        Some(color),
        "Color",
        "Color",
        PropertyValue::Color(model::Rgba::new(0.2, 0.3, 0.4, 1.0)),
    );
    close_vec(
        &env.vec("effect(\"Color Control\")(\"Color\")"),
        &[0.2, 0.3, 0.4, 1.0],
    );
    let pointed = env.scene.add_solid(env.comp, "Picked", 10, 10);
    let layer_ctl =
        env.scene
            .add_group(env.comp, env.layer, Some(null_fx), "Layer Control", "layer");
    env.scene.add_value(
        env.comp,
        env.layer,
        Some(layer_ctl),
        "Layer",
        "Layer",
        PropertyValue::Layer(Some(pointed)),
    );
    assert_eq!(
        env.text("effect(\"Layer Control\")(\"Layer\").name"),
        "Picked"
    );
    let point = env
        .scene
        .add_group(env.comp, env.layer, Some(null_fx), "Point Control", "point");
    env.scene.add_value(
        env.comp,
        env.layer,
        Some(point),
        "Point",
        "Point",
        PropertyValue::TwoD([3.0, 4.0]),
    );
    close_vec(
        &env.vec("effect(\"Point Control\")(\"Point\")"),
        &[3.0, 4.0],
    );
    let point3 = env.scene.add_group(
        env.comp,
        env.layer,
        Some(null_fx),
        "3D Point Control",
        "point3",
    );
    env.scene.add_value(
        env.comp,
        env.layer,
        Some(point3),
        "3D Point",
        "3D Point",
        PropertyValue::ThreeD([1.0, 2.0, 3.0]),
    );
    close_vec(
        &env.vec("effect(\"3D Point Control\")(\"3D Point\")"),
        &[1.0, 2.0, 3.0],
    );
    let menu = env.scene.add_group(
        env.comp,
        env.layer,
        Some(null_fx),
        "Dropdown Menu Control",
        "menu",
    );
    let menu_prop = env.scene.add_value(
        env.comp,
        env.layer,
        Some(menu),
        "Menu",
        "Menu",
        PropertyValue::Index(3),
    );
    env.scene.prop_extra_mut(menu_prop).dropdown_items = vec!["A".into(), "B".into(), "C".into()];
    assert_eq!(env.num("effect(\"Dropdown Menu Control\")(\"Menu\")"), 3.0);
    assert_eq!(
        env.text("effect(\"Dropdown Menu Control\")(\"Menu\").items[2]"),
        "C"
    );
    assert_eq!(
        env.text("effect(\"Dropdown Menu Control\")(\"Menu\").text"),
        "C"
    );
    env.scene.prop_extra_mut(menu_prop).dropdown_items = vec!["A".into(), "(-".into(), "B".into()];
    assert_eq!(
        env.text("effect(\"Dropdown Menu Control\")(\"Menu\").items[0]"),
        "A"
    );
    assert_eq!(
        env.text("effect(\"Dropdown Menu Control\")(\"Menu\").items[1]"),
        "(-"
    );
    env.scene
        .add_key(env.comp, env.layer, menu_prop, 0.0, PropertyValue::Index(1));
    env.scene
        .add_key(env.comp, env.layer, menu_prop, 2.0, PropertyValue::Index(3));
    env.time = 2.0;
    assert_eq!(
        env.text("effect(\"Dropdown Menu Control\")(\"Menu\").textAtTime(time - 1)"),
        "A"
    );
    env.scene.prop_extra_mut(slider).enabled = false;
    env.layer = env.scene.project.composition(env.comp).unwrap().layers[0].id;
    // The first layer still holds the original slider group `slider`.
    assert!(!env.flag("effect(\"Slider Control\").enabled"));
}

#[test]
fn masks_and_paths() {
    let mut env = env();
    let mut path = ShapePath::new();
    path.vertices = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]];
    path.closed = true;
    env.scene
        .add_mask(env.comp, env.layer, "Mask 1", path.clone());
    let open = {
        let mut path = ShapePath::new();
        path.vertices = vec![[0.0, 0.0], [100.0, 0.0]];
        path.closed = false;
        path
    };
    env.scene.add_mask(env.comp, env.layer, "Open", open);
    assert!(env.flag("mask(\"Mask 1\").path.isClosed()"));
    assert!(!env.flag("mask(\"Open\").path.isClosed()"));
    close_vec(&env.vec("mask(\"Mask 1\").path.points(0)[0]"), &[0.0, 0.0]);
    close_vec(
        &env.vec("mask(\"Mask 1\").maskPath.points()[0]"),
        &[0.0, 0.0],
    );
    let points = env.eval("mask(\"Open\").path.points()").unwrap();
    let tangents = env.eval("mask(\"Open\").path.inTangents()").unwrap();
    assert_eq!(
        points.array().unwrap().len(),
        tangents.array().unwrap().len()
    );
    for tangent in tangents.array().unwrap() {
        close_vec(&support::coords(tangent.clone()), &[0.0, 0.0]);
    }
    for tangent in env
        .eval("mask(\"Open\").path.outTangents()")
        .unwrap()
        .array()
        .unwrap()
    {
        close_vec(&support::coords(tangent.clone()), &[0.0, 0.0]);
    }
    close_vec(&env.vec("mask(\"Open\").path.pointOnPath(0)"), &[0.0, 0.0]);
    close_vec(
        &env.vec("mask(\"Open\").path.pointOnPath(1)"),
        &[100.0, 0.0],
    );
    close_vec(
        &env.vec("mask(\"Open\").path.pointOnPath(0.5)"),
        &[50.0, 0.0],
    );
    close_vec(&env.vec("mask(\"Open\").path.pointOnPath(-1)"), &[0.0, 0.0]);
    close_vec(
        &env.vec("mask(\"Open\").path.pointOnPath(2)"),
        &[100.0, 0.0],
    );
    close(
        env.num("length(mask(\"Open\").path.tangentOnPath(0.5))"),
        1.0,
    );
    close(
        env.num("length(mask(\"Open\").path.normalOnPath(0.5))"),
        1.0,
    );
    close(
        env.num(
            "dot(mask(\"Open\").path.tangentOnPath(0.5), mask(\"Open\").path.normalOnPath(0.5))",
        ),
        0.0,
    );
    let created = env.eval("createPath().points()").unwrap();
    assert_eq!(created.array().unwrap().len(), 4);
    assert!(env.flag("mask(\"Mask 1\").path.createPath(mask(\"Mask 1\").path.points()).isClosed()"));
    assert!(!env.flag("createPath(mask(\"Open\").path.points(), [], [], false).isClosed()"));
    assert!(env.eval("pointsOnPath(0.5)").is_err());
    let shape = env.scene.add_shape_layer(env.comp, "Shape");
    let mut shape_path = ShapePath::new();
    shape_path.vertices = vec![[1.0, 2.0], [3.0, 4.0]];
    shape_path.closed = false;
    env.scene
        .set_shape_path(env.comp, shape, "Shape 1", "Path 1", shape_path);
    env.layer = shape;
    close_vec(
        &env.vec("content(\"Shape 1\").content(\"Path 1\").path.points(0)[0]"),
        &[1.0, 2.0],
    );
}

#[test]
fn layer_switches_parent_and_transform() {
    let mut env = env();
    assert!(!env.flag("hasParent"));
    let parent = env.scene.add_solid(env.comp, "Parent", 111, 80);
    env.scene.layer_attr_mut(parent).unwrap().width = 111.0;
    env.scene.set_parent(env.comp, env.layer, Some(parent));
    assert!(env.flag("hasParent"));
    assert_eq!(env.num("parent.width"), 111.0);
    env.scene.layer_attr_mut(env.layer).unwrap().in_point = 1.0;
    env.scene.layer_attr_mut(env.layer).unwrap().out_point = 3.0;
    env.scene.layer_attr_mut(env.layer).unwrap().start_time = 2.0;
    assert_eq!(env.num("inPoint"), 1.0);
    assert_eq!(env.num("outPoint"), 3.0);
    assert_eq!(env.num("startTime"), 2.0);
    assert!(env.flag("hasVideo"));
    assert!(!env.flag("hasAudio"));
    env.scene.layer_attr_mut(env.layer).unwrap().has_video = false;
    assert!(!env.flag("hasVideo"));
    env.scene.layer_attr_mut(env.layer).unwrap().has_video = true;
    env.scene.layer_attr_mut(env.layer).unwrap().has_audio = true;
    assert!(env.flag("hasAudio"));
    env.time = 2.0;
    assert!(env.flag("active"));
    env.scene.layer_attr_mut(env.layer).unwrap().enabled = false;
    assert!(!env.flag("active"));
    assert!(!env.flag("enabled"));
    env.scene.layer_attr_mut(env.layer).unwrap().enabled = true;
    env.time = 0.0;
    assert!(env.flag("enabled"));
    assert!(!env.flag("active"));
    env.time = 2.0;
    env.scene.layer_attr_mut(env.layer).unwrap().audio_enabled = true;
    assert!(env.flag("audioActive"));
    env.scene.layer_attr_mut(env.layer).unwrap().audio_enabled = false;
    assert!(!env.flag("audioActive"));
    assert_eq!(env.num("index"), 1.0);
    assert_eq!(env.text("name"), "Layer 1");
    let group = env
        .scene
        .add_group(env.comp, env.layer, None, "Transform", "transform");
    env.scene.add_value(
        env.comp,
        env.layer,
        Some(group),
        "Anchor Point",
        "anchorPoint",
        PropertyValue::TwoD([0.0, 0.0]),
    );
    let position = env.scene.add_value(
        env.comp,
        env.layer,
        Some(group),
        "Position",
        "position",
        PropertyValue::TwoD([10.0, 20.0]),
    );
    env.scene.add_value(
        env.comp,
        env.layer,
        Some(group),
        "Scale",
        "scale",
        PropertyValue::TwoD([100.0, 100.0]),
    );
    let rotation = env.scene.add_value(
        env.comp,
        env.layer,
        Some(group),
        "Rotation",
        "rotation",
        PropertyValue::Scalar(45.0),
    );
    if let model::PropertyBody::Value { value, .. } = &mut env
        .scene
        .property_mut(env.comp, env.layer, env.prop)
        .unwrap()
        .body
    {
        *value = PropertyValue::Scalar(50.0);
    }
    close_vec(&env.vec("anchorPoint"), &[0.0, 0.0]);
    close_vec(&env.vec("position"), &[10.0, 20.0]);
    close_vec(&env.vec("thisLayer.transform.position"), &[10.0, 20.0]);
    close_vec(&env.vec("scale"), &[100.0, 100.0]);
    assert_eq!(env.num("rotation"), 45.0);
    assert_eq!(env.num("opacity"), 50.0);
    env.scene.add_value(
        env.comp,
        env.layer,
        None,
        "Audio Levels",
        "audioLevels",
        PropertyValue::TwoD([0.0, 0.0]),
    );
    close_vec(&env.vec("audioLevels"), &[0.0, 0.0]);
    env.scene.add_value(
        env.comp,
        env.layer,
        None,
        "Time Remap",
        "timeRemap",
        PropertyValue::Scalar(2.0),
    );
    assert_eq!(env.num("timeRemap"), 2.0);
    env.scene.set_3d(env.comp, env.layer, true);
    env.scene.add_value(
        env.comp,
        env.layer,
        Some(group),
        "Orientation",
        "orientation",
        PropertyValue::ThreeD([0.0, 0.0, 0.0]),
    );
    env.scene.add_value(
        env.comp,
        env.layer,
        Some(group),
        "X Rotation",
        "rotationX",
        PropertyValue::Scalar(30.0),
    );
    env.scene.add_value(
        env.comp,
        env.layer,
        Some(group),
        "Y Rotation",
        "rotationY",
        PropertyValue::Scalar(30.0),
    );
    env.scene.add_value(
        env.comp,
        env.layer,
        Some(group),
        "Z Rotation",
        "rotationZ",
        PropertyValue::Scalar(30.0),
    );
    close_vec(&env.vec("orientation"), &[0.0, 0.0, 0.0]);
    assert_eq!(env.num("rotationX"), 30.0);
    assert_eq!(env.num("rotationY"), 30.0);
    assert_eq!(env.num("rotationZ"), 30.0);
    for (name, value) in [
        ("lightTransmission", 0.4),
        ("castsShadows", 1.0),
        ("acceptsShadows", 1.0),
        ("acceptsLights", 1.0),
        ("ambient", 0.2),
        ("diffuse", 0.3),
        ("specularIntensity", 0.5),
        ("specularShininess", 16.0),
        ("metal", 0.7),
    ] {
        env.scene.add_value(
            env.comp,
            env.layer,
            None,
            name,
            name,
            PropertyValue::Scalar(value),
        );
        assert_eq!(env.num(name), value);
    }
    assert!(env.eval("specular").is_err());
    assert!(env.eval("shininess").is_err());
    env.prop = rotation;
    assert_eq!(env.text("thisProperty.name"), "Rotation");
    assert_eq!(env.num("propertyIndex"), 4.0);
    assert!(env.flag("thisProperty.active"));
    assert_eq!(env.num("thisProperty.propertyGroup(1).numProperties"), 8.0);
    env.prop = position;
    close_vec(
        &env.vec("thisComp.layer(1).transform.position.value"),
        &[10.0, 20.0],
    );
    close_vec(&env.vec("position"), &[10.0, 20.0]);
    let _ = BitsPerChannel::Bpc8;
}

#[test]
fn sample_image_averages_rect() {
    let mut env = env();
    let mut pixels = vec![[0.0, 0.0, 0.0, 1.0]; 200 * 200];
    for y in 98..101 {
        for x in 98..102 {
            pixels[y * 200 + x] = [0.0, 1.0, 0.0, 1.0];
        }
    }
    env.scene.layer_attr_mut(env.layer).unwrap().image = Some(Image {
        width: 200,
        height: 200,
        pixels: pixels.clone(),
    });
    close_vec(
        &env.vec("sampleImage([100, 100], [2, 1.5])"),
        &[0.0, 1.0, 0.0, 1.0],
    );
    pixels.fill([0.2, 0.4, 0.6, 1.0]);
    env.scene.layer_attr_mut(env.layer).unwrap().image = Some(Image {
        width: 200,
        height: 200,
        pixels,
    });
    close_vec(
        &env.vec("sampleImage([10, 10])"),
        &env.vec("sampleImage([10, 10], [.5, .5])"),
    );
    assert_eq!(env.vec("sampleImage([10, 10])").len(), 4);
}

#[test]
fn space_roundtrip_ignores_camera_for_world() {
    let mut env = env();
    let group = env
        .scene
        .add_group(env.comp, env.layer, None, "Transform", "transform");
    env.scene.add_value(
        env.comp,
        env.layer,
        Some(group),
        "Anchor Point",
        "anchorPoint",
        PropertyValue::TwoD([5.0, 6.0]),
    );
    env.scene.add_value(
        env.comp,
        env.layer,
        Some(group),
        "Position",
        "position",
        PropertyValue::TwoD([10.0, 20.0]),
    );
    env.scene.add_value(
        env.comp,
        env.layer,
        Some(group),
        "Scale",
        "scale",
        PropertyValue::TwoD([100.0, 100.0]),
    );
    env.scene.add_value(
        env.comp,
        env.layer,
        Some(group),
        "Rotation",
        "rotation",
        PropertyValue::Scalar(0.0),
    );
    close_vec(&env.vec("toComp(anchorPoint)"), &[10.0, 20.0]);
    close_vec(&env.vec("fromComp(toComp(anchorPoint))"), &[5.0, 6.0]);
    close_vec(
        &env.vec("toWorld(anchorPoint)"),
        &env.vec("toComp(anchorPoint)"),
    );
    close_vec(&env.vec("fromWorld(toWorld(anchorPoint))"), &[5.0, 6.0]);
    close(env.num("length(toCompVec([1, 0]))"), 1.0);
    close_vec(&env.vec("fromCompVec(toCompVec([1, 0]))"), &[1.0, 0.0]);
    close_vec(
        &env.vec("toWorldVec([1, 0])"),
        &env.vec("toCompVec([1, 0])"),
    );
    close_vec(&env.vec("fromWorldVec(toWorldVec([1, 0]))"), &[1.0, 0.0]);
    env.scene.set_3d(env.comp, env.layer, true);
    let camera_a = env.scene.add_camera(env.comp, "A");
    let camera_b = env.scene.add_camera(env.comp, "B");
    for (camera, x) in [(camera_a, 0.0), (camera_b, 50.0)] {
        let group = env
            .scene
            .add_group(env.comp, camera, None, "Transform", "transform");
        env.scene.add_value(
            env.comp,
            camera,
            Some(group),
            "Position",
            "position",
            PropertyValue::ThreeD([x, 0.0, 0.0]),
        );
        env.scene.layer_attr_mut(camera).unwrap().enabled = true;
    }
    env.scene.layer_attr_mut(camera_b).unwrap().enabled = false;
    env.time = 1.0;
    let first = env.vec("toWorld([1, 2, 3])");
    env.scene.layer_attr_mut(camera_a).unwrap().enabled = false;
    env.scene.layer_attr_mut(camera_b).unwrap().enabled = true;
    let second = env.vec("toWorld([1, 2, 3])");
    close_vec(&first, &second);
    assert_eq!(env.vec("fromCompToSurface([0, 0, 0])").len(), 2);
}

#[test]
fn property_time_loops_and_wiggle() {
    let mut env = env();
    env.scene.add_key(
        env.comp,
        env.layer,
        env.prop,
        0.0,
        PropertyValue::Scalar(0.0),
    );
    env.scene.add_key(
        env.comp,
        env.layer,
        env.prop,
        1.0,
        PropertyValue::Scalar(100.0),
    );
    env.time = 0.5;
    assert_eq!(env.num("valueAtTime(0.5)"), 50.0);
    assert_eq!(env.num("velocity"), 100.0);
    assert_eq!(env.num("velocityAtTime(0.5)"), 100.0);
    let group = env
        .scene
        .add_group(env.comp, env.layer, None, "Transform", "transform");
    let position = env.scene.add_value(
        env.comp,
        env.layer,
        Some(group),
        "Position",
        "position",
        PropertyValue::TwoD([0.0, 0.0]),
    );
    env.scene.add_key(
        env.comp,
        env.layer,
        position,
        0.0,
        PropertyValue::TwoD([0.0, 0.0]),
    );
    env.scene.add_key(
        env.comp,
        env.layer,
        position,
        1.0,
        PropertyValue::TwoD([100.0, 0.0]),
    );
    env.prop = position;
    assert_eq!(env.num("speed"), 100.0);
    assert_eq!(env.num("speedAtTime(0.5)"), 100.0);
    env.time = 0.0;
    let a = env.vec("seedRandom(1, true); wiggle(5, 20)");
    env.time = 1.0;
    let b = env.vec("seedRandom(1, true); wiggle(5, 20)");
    assert_ne!(a, b);
    let c = env.vec("seedRandom(2, true); wiggle(5, 20)");
    assert_ne!(b, c);
    let paired = env.vec("v = wiggle(5, 10); [v[0], v[0]]");
    assert_eq!(paired[0], paired[1]);
    let original_x = env.vec("value")[0];
    assert_eq!(env.vec("[value[0], wiggle(3, 50)[1]]")[0], original_x);
    env.prop = env.scene.add_value(
        env.comp,
        env.layer,
        None,
        "Flat",
        "flat",
        PropertyValue::Scalar(4.0),
    );
    assert_eq!(env.num("temporalWiggle(5, 0.2)"), 4.0);
    let scale = env.scene.add_value(
        env.comp,
        env.layer,
        Some(group),
        "Scale",
        "scale",
        PropertyValue::TwoD([0.0, 0.0]),
    );
    env.scene.add_key(
        env.comp,
        env.layer,
        scale,
        0.0,
        PropertyValue::TwoD([0.0, 0.0]),
    );
    env.scene.add_key(
        env.comp,
        env.layer,
        scale,
        10.0,
        PropertyValue::TwoD([100.0, 100.0]),
    );
    env.prop = scale;
    env.time = 5.0;
    let wiggled = env.vec("temporalWiggle(5, 0.2)");
    assert!(wiggled[0] >= 48.0 && wiggled[0] <= 52.0, "{wiggled:?}");
    env.prop = env.scene.add_value(
        env.comp,
        env.layer,
        None,
        "Hold",
        "hold",
        PropertyValue::Scalar(8.0),
    );
    assert_eq!(env.num("smooth()"), 8.0);
    let spike = env.prop;
    env.scene
        .add_key(env.comp, env.layer, spike, 0.9, PropertyValue::Scalar(0.0));
    env.scene.add_key(
        env.comp,
        env.layer,
        spike,
        1.0,
        PropertyValue::Scalar(100.0),
    );
    env.scene
        .add_key(env.comp, env.layer, spike, 1.1, PropertyValue::Scalar(0.0));
    env.time = 1.0;
    assert!(env.num("smooth(0.1, 5)") < 100.0);
    let looped = env.scene.add_value(
        env.comp,
        env.layer,
        None,
        "Loop",
        "loop",
        PropertyValue::Scalar(0.0),
    );
    env.scene
        .add_key(env.comp, env.layer, looped, 0.0, PropertyValue::Scalar(0.0));
    env.scene.add_key(
        env.comp,
        env.layer,
        looped,
        1.0,
        PropertyValue::Scalar(10.0),
    );
    env.prop = looped;
    env.time = 1.25;
    close(env.num("loopOut(\"cycle\")"), 2.5);
    close(env.num("loopOut(\"offset\")"), 12.5);
    close(env.num("loopOut(\"continue\")"), 12.5);
    env.time = -0.5;
    close(env.num("loopIn(\"pingpong\")"), 5.0);
    close(env.num("loopIn(\"offset\")"), -5.0);
    let longer = env.scene.add_value(
        env.comp,
        env.layer,
        None,
        "Long",
        "long",
        PropertyValue::Scalar(0.0),
    );
    env.scene
        .add_key(env.comp, env.layer, longer, 0.0, PropertyValue::Scalar(0.0));
    env.scene.add_key(
        env.comp,
        env.layer,
        longer,
        1.0,
        PropertyValue::Scalar(10.0),
    );
    env.scene.add_key(
        env.comp,
        env.layer,
        longer,
        2.0,
        PropertyValue::Scalar(100.0),
    );
    env.prop = longer;
    env.time = -1.0;
    close(env.num("loopIn(\"continue\", 1)"), -10.0);
    env.scene.add_key(
        env.comp,
        env.layer,
        looped,
        2.0,
        PropertyValue::Scalar(110.0),
    );
    env.prop = looped;
    env.time = 2.5;
    close(env.num("loopOut(\"cycle\", 1)"), 60.0);
    let dur = env.scene.add_value(
        env.comp,
        env.layer,
        None,
        "Dur",
        "dur",
        PropertyValue::Scalar(0.0),
    );
    env.scene
        .add_key(env.comp, env.layer, dur, 0.0, PropertyValue::Scalar(0.0));
    env.scene
        .add_key(env.comp, env.layer, dur, 1.0, PropertyValue::Scalar(10.0));
    env.scene
        .add_key(env.comp, env.layer, dur, 3.0, PropertyValue::Scalar(0.0));
    env.prop = dur;
    env.time = -0.5;
    close(env.num("loopInDuration(\"cycle\", 1)"), 5.0);
    env.scene.layer_attr_mut(env.layer).unwrap().out_point = 4.0;
    let omitted = env.scene.add_value(
        env.comp,
        env.layer,
        None,
        "OmitIn",
        "omitIn",
        PropertyValue::Scalar(0.0),
    );
    env.scene.add_key(
        env.comp,
        env.layer,
        omitted,
        1.0,
        PropertyValue::Scalar(0.0),
    );
    env.scene.add_key(
        env.comp,
        env.layer,
        omitted,
        3.0,
        PropertyValue::Scalar(10.0),
    );
    env.scene.add_key(
        env.comp,
        env.layer,
        omitted,
        6.0,
        PropertyValue::Scalar(0.0),
    );
    env.prop = omitted;
    env.time = 0.0;
    close(env.num("loopInDuration(\"cycle\")"), 10.0);
    let out_dur = env.scene.add_value(
        env.comp,
        env.layer,
        None,
        "OutDur",
        "outDur",
        PropertyValue::Scalar(0.0),
    );
    env.scene.add_key(
        env.comp,
        env.layer,
        out_dur,
        0.0,
        PropertyValue::Scalar(100.0),
    );
    env.scene.add_key(
        env.comp,
        env.layer,
        out_dur,
        4.0,
        PropertyValue::Scalar(10.0),
    );
    env.scene.add_key(
        env.comp,
        env.layer,
        out_dur,
        5.0,
        PropertyValue::Scalar(0.0),
    );
    env.prop = out_dur;
    env.time = 5.5;
    close(env.num("loopOutDuration(\"cycle\", 1)"), 5.0);
    env.scene.layer_attr_mut(env.layer).unwrap().in_point = 1.0;
    let out_omit = env.scene.add_value(
        env.comp,
        env.layer,
        None,
        "OutOmit",
        "outOmit",
        PropertyValue::Scalar(0.0),
    );
    env.scene.add_key(
        env.comp,
        env.layer,
        out_omit,
        0.0,
        PropertyValue::Scalar(0.0),
    );
    env.scene.add_key(
        env.comp,
        env.layer,
        out_omit,
        1.0,
        PropertyValue::Scalar(0.0),
    );
    env.scene.add_key(
        env.comp,
        env.layer,
        out_omit,
        3.0,
        PropertyValue::Scalar(10.0),
    );
    env.prop = out_omit;
    env.time = 3.5;
    close(env.num("loopOutDuration(\"cycle\")"), 2.5);
    env.prop = position;
    env.scene.add_key(
        env.comp,
        env.layer,
        position,
        2.0,
        PropertyValue::TwoD([1.0, 2.0]),
    );
    assert_eq!(env.vec("position.key(3).value"), vec![1.0, 2.0]);
    assert_eq!(env.num("key(1).time"), 0.0);
    assert_eq!(env.num("key(1).index"), 1.0);
    env.time = 1.0;
    assert_eq!(env.num("nearestKey(0.4).time"), 0.0);
    assert_eq!(env.num("thisProperty.nextKey(time).time - time"), 1.0);
    env.time = 3.0;
    assert_eq!(env.num("thisProperty.nextKey(time).time"), 2.0);
    env.time = 1.0;
    assert_eq!(env.num("time - thisProperty.previousKey(time).time"), 1.0);
    env.time = -1.0;
    assert_eq!(env.num("thisProperty.previousKey(time).time"), 0.0);
    assert_eq!(env.num("numKeys"), 3.0);
    let empty = env.scene.add_value(
        env.comp,
        env.layer,
        None,
        "Empty",
        "empty",
        PropertyValue::Scalar(0.0),
    );
    env.prop = empty;
    assert_eq!(env.num("numKeys"), 0.0);
    env.scene.set_3d(env.comp, env.layer, false);
    let rx = env.scene.add_value(
        env.comp,
        env.layer,
        Some(group),
        "X Rotation",
        "rotationX",
        PropertyValue::Scalar(1.0),
    );
    env.prop = rx;
    assert!(!env.flag("thisProperty.active"));
    let fx = env
        .scene
        .add_group(env.comp, env.layer, None, "Effects", "effects");
    let blur = env
        .scene
        .add_group(env.comp, env.layer, Some(fx), "Blur", "blur");
    env.scene.prop_extra_mut(blur).enabled = false;
    env.prop = empty;
    assert!(!env.flag("effect(\"Blur\").enabled"));
}

#[test]
fn layer_markers_and_marker_fields() {
    let mut env = env();
    let marker = env.scene.add_value(
        env.comp,
        env.layer,
        None,
        "Marker",
        "marker",
        PropertyValue::Scalar(0.0),
    );
    let mut early = Marker::new("Start");
    early.duration_seconds = 1.0;
    early.chapter = "C".into();
    early.url = "U".into();
    early.frame_target = "F".into();
    early.event_cue_point = true;
    early.cue_point_name = "Q".into();
    early.parameters = vec![("background color".into(), "V".into())];
    early.protected_region = false;
    let mut late = Marker::new("End");
    late.event_cue_point = false;
    env.scene.add_key(
        env.comp,
        env.layer,
        marker,
        1.5,
        PropertyValue::marker(early),
    );
    env.scene.add_key(
        env.comp,
        env.layer,
        marker,
        3.0,
        PropertyValue::marker(late),
    );
    assert_eq!(env.num("marker.numKeys"), 2.0);
    assert_eq!(env.num("thisLayer.marker.numKeys"), 2.0);
    assert_eq!(env.num("marker.key(1).time"), 1.5);
    env.time = 0.0;
    assert_eq!(
        env.num("linear(time, marker.key(\"Start\").time, marker.key(\"End\").time, 0, 100)"),
        0.0
    );
    env.time = 1.0;
    assert_eq!(env.num("marker.nearestKey(1).time"), 1.5);
    env.time = 1.5;
    assert_eq!(env.num("marker.nearestKey(time).duration"), 1.0);
    assert_eq!(env.text("marker.nearestKey(time).comment"), "Start");
    assert_eq!(env.text("marker.nearestKey(time).chapter"), "C");
    assert_eq!(env.text("marker.nearestKey(time).url"), "U");
    assert_eq!(env.text("marker.nearestKey(time).frameTarget"), "F");
    assert!(env.flag("marker.nearestKey(time).eventCuePoint"));
    assert_eq!(env.text("marker.nearestKey(time).cuePointName"), "Q");
    assert_eq!(
        env.text("marker.nearestKey(time).parameters[\"background color\"]"),
        "V"
    );
    assert_eq!(env.num("marker.nearestKey(time).index"), 1.0);
    env.scene.comp_attr_mut(env.comp).unwrap().markers = vec![(0.25, {
        let mut marker = Marker::new("comp");
        marker.protected_region = true;
        marker
    })];
    assert_eq!(env.num("thisComp.marker.key(1).time"), 0.25);
    assert!(env.flag("thisComp.marker.key(1).protectedRegion"));
    env.scene.comp_attr_mut(env.comp).unwrap().markers[0]
        .1
        .protected_region = false;
    assert!(!env.flag("thisComp.marker.key(1).protectedRegion"));
    env.prop = marker;
    assert_eq!(
        env.num("key(\"Start\").time"),
        env.num("marker.key(\"Start\").time")
    );
}

#[test]
fn camera_and_light_shared_and_missing() {
    let mut scene = expr::Scene::new(BitsPerChannel::Bpc8);
    let solid_comp = scene.add_comp("SolidComp", 200, 100, 25.0, 8.0);
    let cam_comp = scene.add_comp("CamComp", 200, 100, 25.0, 8.0);
    let solid = scene.add_solid(solid_comp, "Twin", 200, 100);
    let camera = scene.add_camera(cam_comp, "Twin");
    equip(&mut scene, solid_comp, solid);
    equip(&mut scene, cam_comp, camera);
    let solid_prop = host_prop(&scene, solid_comp, solid);
    let cam_prop = host_prop(&scene, cam_comp, camera);
    for expr_src in [
        "sourceTime(0)",
        "sourceRectAtTime().width",
        "index",
        "parent.name",
        "hasParent",
        "inPoint",
        "outPoint",
        "startTime",
        "hasVideo",
        "hasAudio",
        "enabled",
        "audioActive",
        "sampleImage([0,0])",
        "transform.position",
        "position",
        "rotation",
        "marker.numKeys",
        "marker.key(1).time",
        "marker.key(\"1\").time",
        "marker.nearestKey(time).time",
        "name",
        "orientation",
        "rotationX",
        "rotationY",
        "rotationZ",
        "toComp([0,0])",
        "fromComp(toComp([0,0]))",
        "toWorld([0,0])",
        "fromWorld(toWorld([0,0]))",
        "toCompVec([1,0])",
        "fromCompVec([1,0])",
        "toWorldVec([1,0])",
        "fromWorldVec([1,0])",
        "fromCompToSurface([0,0,0])",
    ] {
        let left = scene
            .eval(target(solid_comp, solid, solid_prop), expr_src)
            .unwrap_or_else(|e| panic!("{expr_src} solid {e:?}"));
        let right = scene
            .eval(target(cam_comp, camera, cam_prop), expr_src)
            .unwrap_or_else(|e| panic!("{expr_src} camera {e:?}"));
        assert_eq!(format!("{left:?}"), format!("{right:?}"), "{expr_src}");
    }
    let light_comp = scene.add_comp("LightComp", 200, 100, 25.0, 8.0);
    let light = scene.add_light(light_comp, "Twin", LightKind::Spot);
    equip(&mut scene, light_comp, light);
    let light_prop = host_prop(&scene, light_comp, light);
    for expr_src in [
        "active",
        "sourceTime(0)",
        "index",
        "name",
        "position",
        "toComp([0,0])",
        "marker.numKeys",
    ] {
        let left = scene
            .eval(target(solid_comp, solid, solid_prop), expr_src)
            .unwrap();
        let right = scene
            .eval(target(light_comp, light, light_prop), expr_src)
            .unwrap();
        assert_eq!(format!("{left:?}"), format!("{right:?}"), "{expr_src}");
    }
    for missing in [
        "source",
        "effect(1)",
        "mask(1)",
        "width",
        "height",
        "anchorPoint",
        "scale",
        "opacity",
        "audioLevels",
        "timeRemap",
        "lightTransmission",
        "castsShadows",
        "acceptsShadows",
        "acceptsLights",
        "ambient",
        "diffuse",
        "specularIntensity",
        "specularShininess",
        "metal",
    ] {
        assert!(
            scene
                .eval(target(cam_comp, camera, cam_prop), missing)
                .is_err(),
            "{missing}"
        );
        assert!(
            scene
                .eval(target(light_comp, light, light_prop), missing)
                .is_err(),
            "{missing}"
        );
        assert!(
            scene
                .eval(target(solid_comp, solid, solid_prop), missing)
                .is_ok(),
            "{missing}"
        );
    }
}

fn equip(scene: &mut expr::Scene, comp: model::ItemId, layer: model::LayerId) {
    scene.add_value(
        comp,
        layer,
        None,
        "Host",
        "host",
        PropertyValue::Scalar(0.0),
    );
    let parent = scene.add_solid(comp, "Parent", 40, 40);
    scene.layer_attr_mut(parent).unwrap().width = 40.0;
    scene.set_parent(comp, layer, Some(parent));
    scene.set_3d(comp, layer, true);
    let attr = scene.layer_attr_mut(layer).unwrap();
    attr.in_point = 1.0;
    attr.out_point = 6.0;
    attr.start_time = 1.0;
    attr.enabled = true;
    attr.audio_enabled = true;
    attr.has_audio = true;
    attr.has_video = true;
    attr.width = 20.0;
    attr.height = 10.0;
    attr.source_rect.width = 20.0;
    attr.image = Some(Image {
        width: 2,
        height: 2,
        pixels: vec![[0.1, 0.2, 0.3, 1.0]; 4],
    });
    let group = scene.add_group(comp, layer, None, "Transform", "transform");
    scene.add_value(
        comp,
        layer,
        Some(group),
        "Position",
        "position",
        PropertyValue::ThreeD([4.0, 5.0, 6.0]),
    );
    scene.add_value(
        comp,
        layer,
        Some(group),
        "Rotation",
        "rotation",
        PropertyValue::Scalar(15.0),
    );
    scene.add_value(
        comp,
        layer,
        Some(group),
        "X Rotation",
        "rotationX",
        PropertyValue::Scalar(1.0),
    );
    scene.add_value(
        comp,
        layer,
        Some(group),
        "Y Rotation",
        "rotationY",
        PropertyValue::Scalar(2.0),
    );
    scene.add_value(
        comp,
        layer,
        Some(group),
        "Z Rotation",
        "rotationZ",
        PropertyValue::Scalar(15.0),
    );
    scene.add_value(
        comp,
        layer,
        Some(group),
        "Orientation",
        "orientation",
        PropertyValue::ThreeD([0.0, 0.0, 0.0]),
    );
    scene.add_value(
        comp,
        layer,
        Some(group),
        "Anchor Point",
        "anchorPoint",
        PropertyValue::ThreeD([0.0, 0.0, 0.0]),
    );
    scene.add_value(
        comp,
        layer,
        Some(group),
        "Scale",
        "scale",
        PropertyValue::ThreeD([100.0, 100.0, 100.0]),
    );
    scene.add_value(
        comp,
        layer,
        Some(group),
        "Opacity",
        "opacity",
        PropertyValue::Scalar(100.0),
    );
    scene.add_value(
        comp,
        layer,
        None,
        "Audio Levels",
        "audioLevels",
        PropertyValue::TwoD([0.0, 0.0]),
    );
    scene.add_value(
        comp,
        layer,
        None,
        "Time Remap",
        "timeRemap",
        PropertyValue::Scalar(1.0),
    );
    for name in [
        "lightTransmission",
        "castsShadows",
        "acceptsShadows",
        "acceptsLights",
        "ambient",
        "diffuse",
        "specularIntensity",
        "specularShininess",
        "metal",
    ] {
        scene.add_value(comp, layer, None, name, name, PropertyValue::Scalar(1.0));
    }
    let marker = scene.add_value(
        comp,
        layer,
        None,
        "Marker",
        "marker",
        PropertyValue::Scalar(0.0),
    );
    scene.add_key(
        comp,
        layer,
        marker,
        1.5,
        PropertyValue::marker(Marker::new("1")),
    );
    let footage = scene.add_footage("Clip", 320, 240);
    if let Some(found) = scene.layer_mut(comp, layer) {
        if matches!(
            found.kind,
            LayerKind::Solid { .. } | LayerKind::Footage { .. }
        ) {
            found.kind = LayerKind::Footage { source: footage };
        }
    }
    let effects = scene.add_group(comp, layer, None, "Effects", "effects");
    let slider = scene.add_group(comp, layer, Some(effects), "Slider Control", "slider");
    scene.add_value(
        comp,
        layer,
        Some(slider),
        "Slider",
        "Slider",
        PropertyValue::Scalar(1.0),
    );
    scene.add_mask(comp, layer, "Mask 1", ShapePath::new());
}

fn host_prop(scene: &expr::Scene, comp: model::ItemId, layer: model::LayerId) -> model::PropertyId {
    scene
        .layer(comp, layer)
        .unwrap()
        .properties
        .iter()
        .find(|property| property.match_name == "host")
        .unwrap()
        .id
}

fn target(comp: model::ItemId, layer: model::LayerId, property: model::PropertyId) -> expr::Target {
    expr::Target {
        comp,
        layer,
        property,
        time: 2.0,
    }
}

#[test]
fn camera_and_light_options() {
    let mut env = env();
    let camera = env.scene.add_camera(env.comp, "Cam");
    let group = env
        .scene
        .add_group(env.comp, camera, None, "Transform", "transform");
    env.scene.add_value(
        env.comp,
        camera,
        Some(group),
        "Point of Interest",
        "pointOfInterest",
        PropertyValue::ThreeD([0.0, 0.0, 0.0]),
    );
    env.scene.add_value(
        env.comp,
        camera,
        None,
        "Zoom",
        "zoom",
        PropertyValue::Scalar(1000.0),
    );
    env.scene.add_value(
        env.comp,
        camera,
        None,
        "Depth of Field",
        "depthOfField",
        PropertyValue::Scalar(1.0),
    );
    env.scene.add_value(
        env.comp,
        camera,
        None,
        "Focus Distance",
        "focusDistance",
        PropertyValue::Scalar(200.0),
    );
    env.scene.add_value(
        env.comp,
        camera,
        None,
        "Aperture",
        "aperture",
        PropertyValue::Scalar(8.0),
    );
    env.scene.add_value(
        env.comp,
        camera,
        None,
        "Blur Level",
        "blurLevel",
        PropertyValue::Scalar(0.5),
    );
    env.scene.add_value(
        env.comp,
        camera,
        None,
        "Iris Shape",
        "irisShape",
        PropertyValue::Scalar(4.0),
    );
    env.scene.add_value(
        env.comp,
        camera,
        None,
        "Iris Rotation",
        "irisRotation",
        PropertyValue::Scalar(12.0),
    );
    env.scene.add_value(
        env.comp,
        camera,
        None,
        "Iris Roundness",
        "irisRoundness",
        PropertyValue::Scalar(20.0),
    );
    env.scene.add_value(
        env.comp,
        camera,
        None,
        "Iris Aspect Ratio",
        "irisAspectRatio",
        PropertyValue::Scalar(30.0),
    );
    env.scene.add_value(
        env.comp,
        camera,
        None,
        "Iris Diffraction Fringe",
        "irisDiffractionFringe",
        PropertyValue::Scalar(40.0),
    );
    env.scene.add_value(
        env.comp,
        camera,
        None,
        "Highlight Gain",
        "highlightGain",
        PropertyValue::Scalar(50.0),
    );
    env.scene.project.bits_per_channel = BitsPerChannel::Bpc8;
    env.scene.add_value(
        env.comp,
        camera,
        None,
        "Highlight Threshold",
        "highlightThreshold",
        PropertyValue::Scalar(40.0),
    );
    env.scene.add_value(
        env.comp,
        camera,
        None,
        "Highlight Saturation",
        "highlightSaturation",
        PropertyValue::Scalar(60.0),
    );
    env.layer = camera;
    env.time = 1.0;
    close_vec(&env.vec("pointOfInterest"), &[0.0, 0.0, 0.0]);
    assert_eq!(env.num("thisComp.activeCamera.zoom"), 1000.0);
    assert_eq!(env.num("depthOfField"), 1.0);
    assert_eq!(env.num("focusDistance"), 200.0);
    assert_eq!(env.num("aperture"), 8.0);
    assert_eq!(env.num("blurLevel"), 0.5);
    let shape = env.num("irisShape");
    assert_eq!(shape, 4.0);
    assert!((1.0..=10.0).contains(&shape));
    assert_eq!(env.num("irisRotation"), 12.0);
    assert_eq!(env.num("irisRoundness"), 20.0);
    assert_eq!(env.num("irisAspectRatio"), 30.0);
    assert_eq!(env.num("irisDiffractionFringe"), 40.0);
    assert_eq!(env.num("highlightGain"), 50.0);
    let threshold = env.num("highlightThreshold");
    assert_eq!(threshold, 40.0);
    assert!((0.0..=100.0).contains(&threshold));
    assert_eq!(env.num("highlightSaturation"), 60.0);
    assert!(env.flag("active"));
    let lower = env.scene.add_camera(env.comp, "Lower");
    env.layer = lower;
    assert!(!env.flag("active"));
    env.scene.layer_attr_mut(camera).unwrap().enabled = false;
    env.layer = camera;
    assert!(!env.flag("active"));
    assert!(env.eval("aperature").is_err());
    assert!(env.eval("cameraOption.zoom").is_err());
    env.scene.project.bits_per_channel = BitsPerChannel::Bpc16;
    env.scene.add_value(
        env.comp,
        camera,
        None,
        "Highlight Threshold 16",
        "highlightThreshold16",
        PropertyValue::Scalar(1000.0),
    );
    assert_eq!(env.num("highlightThreshold16"), 1000.0);
    assert!(env.num("highlightThreshold16") <= 32768.0);
    env.scene.project.bits_per_channel = BitsPerChannel::Bpc32;
    env.scene.add_value(
        env.comp,
        camera,
        None,
        "Highlight Threshold 32",
        "highlightThreshold32",
        PropertyValue::Scalar(0.5),
    );
    assert_eq!(env.num("highlightThreshold32"), 0.5);
    assert!(env.num("highlightThreshold32") <= 1.0);
    let light = env.scene.add_light(env.comp, "Spot", LightKind::Spot);
    let light_group = env
        .scene
        .add_group(env.comp, light, None, "Transform", "transform");
    env.scene.add_value(
        env.comp,
        light,
        Some(light_group),
        "Point of Interest",
        "pointOfInterest",
        PropertyValue::ThreeD([0.0, 0.0, 0.0]),
    );
    env.scene.add_value(
        env.comp,
        light,
        None,
        "Intensity",
        "intensity",
        PropertyValue::Scalar(75.0),
    );
    env.scene.add_value(
        env.comp,
        light,
        None,
        "Color",
        "color",
        PropertyValue::Color(model::Rgba::white()),
    );
    env.scene.add_value(
        env.comp,
        light,
        None,
        "Cone Angle",
        "coneAngle",
        PropertyValue::Scalar(40.0),
    );
    env.scene.add_value(
        env.comp,
        light,
        None,
        "Cone Feather",
        "coneFeather",
        PropertyValue::Scalar(15.0),
    );
    env.scene.add_value(
        env.comp,
        light,
        None,
        "Shadow Darkness",
        "shadowDarkness",
        PropertyValue::Scalar(0.6),
    );
    env.scene.add_value(
        env.comp,
        light,
        None,
        "Shadow Diffusion",
        "shadowDiffusion",
        PropertyValue::Scalar(5.0),
    );
    env.layer = light;
    close_vec(&env.vec("pointOfInterest"), &[0.0, 0.0, 0.0]);
    assert_eq!(env.num("intensity"), 75.0);
    let color = env.vec("color");
    assert_eq!(color.len(), 4);
    assert!(color[0] == color[1] && color[1] == color[2] && color[0] > 0.0);
    assert_eq!(env.num("coneAngle"), 40.0);
    assert_eq!(env.num("coneFeather"), 15.0);
    assert_eq!(env.num("shadowDarkness"), 0.6);
    assert_eq!(env.num("shadowDiffusion"), 5.0);
    assert!(env.eval("lightOption.intensity").is_err());
}

#[test]
fn text_source_and_legacy_calls() {
    let mut env = env();
    let text = env.scene.add_text_layer(env.comp, "Title", "AB");
    if let Some(layer) = env.scene.layer_mut(env.comp, text) {
        layer.text = Some(Text::point("AB"));
    }
    env.layer = text;
    assert!(env.eval("text.sourceText[0]").is_err());
    assert!(!env.scene.errors_for(text).is_empty());
    env.scene.clear_error(env.prop);
    assert_eq!(env.text("text.sourceText.value[0]"), "A");
    for name in ["P0", "P1", "P2", "P3"] {
        env.scene
            .add_value(env.comp, text, None, name, name, PropertyValue::Scalar(0.0));
    }
    let group = env.scene.add_group(env.comp, text, None, "Group", "group");
    env.scene.add_value(
        env.comp,
        text,
        Some(group),
        "One",
        "one",
        PropertyValue::Scalar(1.0),
    );
    env.scene.add_value(
        env.comp,
        text,
        Some(group),
        "Two",
        "two",
        PropertyValue::Scalar(9.0),
    );
    env.eval("thisLayer(5)(2).value").unwrap();
    assert!(env.eval("this(5)(2).value").is_err());
    assert_eq!(
        env.eval("this(5)(2).value").unwrap_err().message,
        "this is not a function"
    );
}
