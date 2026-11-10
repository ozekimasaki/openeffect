#[path = "support/mod.rs"]
mod support;

use model::{BitsPerChannel, PropertyValue};
use support::{close, close_vec, Env};

fn env() -> Env {
    Env::new("Main")
}

#[test]
fn javascript_and_legacy_both_run_const() {
    let mut env = env();
    assert_eq!(env.num("const x = 1; x"), 1.0);
    env.scene.engine_mode = expr::EngineMode::Legacy;
    assert_eq!(env.num("const x = 1; x"), 1.0);
}

#[test]
fn comp_by_name() {
    let mut env = env();
    assert_eq!(env.text("comp(\"Main\").name"), "Main");
}

#[test]
fn footage_by_name() {
    let mut env = env();
    let id = env.scene.add_footage("Clip", 10, 10);
    let value = env.eval("footage(\"Clip\")").unwrap();
    match value {
        expr::Value::Footage(found) => assert_eq!(found, id),
        other => panic!("{other:?}"),
    }
}

#[test]
fn this_comp_name() {
    let mut env = env();
    assert_eq!(env.text("thisComp.name"), "Main");
}

#[test]
fn this_project_bits() {
    let mut env = env();
    env.scene.project.bits_per_channel = BitsPerChannel::Bpc16;
    assert_eq!(env.num("thisProject.bitsPerChannel"), 16.0);
}

#[test]
fn this_layer_name_and_width() {
    let mut env = env();
    assert_eq!(env.text("thisLayer.name"), "Layer 1");
    assert_eq!(env.num("thisLayer.width"), env.num("width"));
}

#[test]
fn this_property_name() {
    let mut env = env();
    assert_eq!(env.text("thisProperty.name"), "Opacity");
}

#[test]
fn time_ignores_display_start() {
    let mut env = env();
    env.scene
        .comp_attr_mut(env.comp)
        .unwrap()
        .display_start_time = 3600.0;
    env.time = 2.0;
    assert_eq!(env.num("time"), 2.0);
}

#[test]
fn color_depth_matches_project() {
    for (bits, expected) in [
        (BitsPerChannel::Bpc8, 8.0),
        (BitsPerChannel::Bpc16, 16.0),
        (BitsPerChannel::Bpc32, 32.0),
    ] {
        let mut env = env();
        env.scene.project.bits_per_channel = bits;
        assert_eq!(env.num("colorDepth"), expected);
    }
}

#[test]
fn posterize_time_holds_random_and_zero_freezes_time() {
    let mut env = env();
    env.time = 0.2;
    let early = env.num("posterizeTime(1); random()");
    env.time = 0.8;
    let late = env.num("posterizeTime(1); random()");
    assert_eq!(early, late);
    assert!((0.0..=1.0).contains(&early));
    env.time = 4.0;
    assert_eq!(env.num("posterizeTime(0); time"), 0.0);
}

#[test]
fn value_is_pre_expression_number() {
    let mut env = env();
    let prop = env.scene.add_value(
        env.comp,
        env.layer,
        None,
        "Opacity",
        "opacityValue",
        PropertyValue::Scalar(40.0),
    );
    env.prop = prop;
    assert_eq!(env.num("value"), 40.0);
}

#[test]
fn full_path_empty_until_saved() {
    let mut env = env();
    assert_eq!(env.text("thisProject.fullPath"), "");
    env.scene.full_path = "/tmp/project.aep".into();
    assert_eq!(env.text("thisProject.fullPath"), "/tmp/project.aep");
}

#[test]
fn bits_per_channel_values() {
    for (bits, expected) in [
        (BitsPerChannel::Bpc8, 8.0),
        (BitsPerChannel::Bpc16, 16.0),
        (BitsPerChannel::Bpc32, 32.0),
    ] {
        let mut env = env();
        env.scene.project.bits_per_channel = bits;
        assert_eq!(env.num("thisProject.bitsPerChannel"), expected);
    }
}

#[test]
fn linear_blending_flag() {
    let mut env = env();
    env.scene.linear_blending = true;
    assert!(env.flag("thisProject.linearBlending"));
    env.scene.linear_blending = false;
    assert!(!env.flag("thisProject.linearBlending"));
}

#[test]
fn time_to_frames_defaults_and_rounding() {
    let mut env = env();
    let frame = 1.0 / 25.0;
    env.time = 0.0;
    assert_eq!(env.num("timeToFrames()"), 0.0);
    let expr = format!("timeToFrames({frame}, 1/{frame}, false)");
    assert_eq!(env.num(&expr), 1.0);
    assert_eq!(env.num("timeToFrames(-0.2, 1, false)"), -1.0);
    assert_eq!(env.num("timeToFrames(1.9, 1, false)"), 1.0);
    assert_eq!(env.num("timeToFrames(1.5, 1, true)"), 2.0);
    assert_eq!(env.num("timeToFrames(-1.5, 1, true)"), -2.0);
}

#[test]
fn frames_to_time_matches_frame_duration() {
    let mut env = env();
    close(
        env.num("framesToTime(1)"),
        env.num("thisComp.frameDuration"),
    );
}

fn zero_stamp(text: &str) -> bool {
    text.chars().any(|ch| ch.is_ascii_digit())
        && text.chars().all(|ch| !ch.is_ascii_digit() || ch == '0')
}

#[test]
fn time_to_timecode_at_zero() {
    let mut env = env();
    let text = env.text("timeToTimecode()");
    assert!(zero_stamp(&text), "{text}");
}

#[test]
fn ntsc_timecode_drop_differs_at_zero() {
    let mut env = env();
    let nondrop = env.text("timeToNTSCTimecode()");
    let drop = env.text("timeToNTSCTimecode(0, true, true)");
    assert!(zero_stamp(&nondrop), "{nondrop}");
    assert!(zero_stamp(&drop), "{drop}");
    assert_ne!(nondrop, drop);
}

#[test]
fn feet_and_frames_at_zero() {
    let mut env = env();
    let text = env.text("timeToFeetAndFrames()");
    assert!(zero_stamp(&text), "{text}");
}

#[test]
fn time_to_current_format_follows_project_display() {
    let mut env = env();
    env.time = 2.0;
    env.scene.time_display = expr::TimeDisplay::Timecode;
    assert_eq!(
        env.text("timeToCurrentFormat()"),
        env.text("timeToTimecode()")
    );
    env.scene.time_display = expr::TimeDisplay::FeetAndFrames;
    assert_eq!(
        env.text("timeToCurrentFormat()"),
        env.text("timeToFeetAndFrames()")
    );
    env.scene.time_display = expr::TimeDisplay::Ntsc;
    env.scene.comp_attr_mut(env.comp).unwrap().ntsc_drop_frame = false;
    assert_eq!(
        env.text("timeToCurrentFormat()"),
        env.text("timeToNTSCTimecode()")
    );
    env.scene.comp_attr_mut(env.comp).unwrap().ntsc_drop_frame = true;
    assert_eq!(
        env.text("timeToCurrentFormat()"),
        env.text("timeToNTSCTimecode(time + thisComp.displayStartTime, true, false)")
    );
}

#[test]
fn vector_add_sub_mul_div_dot_cross_normalize_length() {
    let mut env = env();
    close_vec(&env.vec("add([10, 20], [1, 2, 3])"), &[11.0, 22.0, 3.0]);
    close_vec(&env.vec("sub([11, 22, 3], [1, 2, 3])"), &[10.0, 20.0, 0.0]);
    close_vec(&env.vec("mul([1, 2, 3], 2)"), &[2.0, 4.0, 6.0]);
    close_vec(&env.vec("div([2, 4], 2)"), &[1.0, 2.0]);
    assert_eq!(env.num("dot([1, 0], [0, 1])"), 0.0);
    assert_eq!(env.num("dot([1, 2], [3, 4])"), 11.0);
    let crossed = env.vec("cross([1, 0, 0], [0, 1, 0])");
    close(crossed[0], 0.0);
    close(crossed[1], 0.0);
    close(crossed[2].abs(), 1.0);
    close(env.num("length(normalize([3, 4]))"), 1.0);
    assert_eq!(env.num("length([3, 4])"), 5.0);
    assert_eq!(env.num("length([0, 0], [3, 4])"), 5.0);
    assert_eq!(env.num("clamp(0.5, 0, 1)"), 0.5);
    let edge = env.num("clamp(5, 0, 1)");
    assert!(edge == 0.0 || edge == 1.0);
}

#[test]
fn look_at_points_z_at_target() {
    let mut env = env();
    let group = env
        .scene
        .add_group(env.comp, env.layer, None, "Transform", "transform");
    env.scene.add_value(
        env.comp,
        env.layer,
        Some(group),
        "Position",
        "position",
        PropertyValue::ThreeD([80.0, 10.0, 40.0]),
    );
    let light = env
        .scene
        .add_light(env.comp, "Spot", model::LightKind::Spot);
    let light_group = env
        .scene
        .add_group(env.comp, light, None, "Transform", "transform");
    env.scene.add_value(
        env.comp,
        light,
        Some(light_group),
        "Position",
        "position",
        PropertyValue::ThreeD([0.0, 0.0, 0.0]),
    );
    env.layer = light;
    env.scene.layer_attr_mut(light).unwrap().auto_orient = false;
    let value = env.vec("lookAt(position, thisComp.layer(1).position)");
    assert!(!env.scene.layer_attr(light).unwrap().auto_orient);
    let forward = expr::orientation_z_axis([value[0], value[1], value[2]]);
    let dir = [80.0_f64, 10.0, 40.0];
    let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
    let dot = forward[0] * dir[0] / len + forward[1] * dir[1] / len + forward[2] * dir[2] / len;
    assert!(dot > 0.999, "forward {forward:?} value {value:?}");
    close_vec(&expr::orientation_z_axis([0.0, 0.0, 0.0]), &[0.0, 0.0, 1.0]);
}

#[test]
fn array_operators_and_eval_prefix() {
    let mut env = env();
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
    env.prop = position;
    close_vec(&env.vec("position + [100, 100]"), &[100.0, 100.0]);
    close_vec(&env.vec("[5, 6] - [1, 2]"), &[4.0, 4.0]);
    assert!(env.eval("[2, 4] * 2").unwrap().array().unwrap().len() == 2);
    assert!(env.eval("[2, 4] / 2").unwrap().array().unwrap().len() == 2);
    assert!(env.eval("eval(\"position + [100, 100]\")").is_err());
    close_vec(
        &env.vec("eval(\"thisLayer.add(position, [100, 100])\")"),
        &[100.0, 100.0],
    );
    assert!(env.eval("eval(\"thisLayer.sub([5, 6], [1, 2])\")").is_ok());
    assert!(env.eval("eval(\"thisLayer.mul([2, 4], 2)\")").is_ok());
    assert!(env.eval("eval(\"thisLayer.div([2, 4], 2)\")").is_ok());
}

#[test]
fn seed_random_timeless_and_ranges() {
    let mut env = env();
    env.time = 0.25;
    let first = env.num("seedRandom(123456, true); random()*100");
    env.time = 3.5;
    let second = env.num("seedRandom(123456, true); random()*100");
    assert_eq!(first, second);
    assert!((0.0..=100.0).contains(&first));
    env.time = 1.0;
    let seeded = env.num("seedRandom(4, true); random()");
    let again = env.num("seedRandom(4, true); random()");
    assert_eq!(seeded, again);
    assert!((0.0..=1.0).contains(&seeded));
    let sample = env.num("random(10)");
    assert!((0.0..=10.0).contains(&sample));
    let pair = env.vec("random([10, 20])");
    assert_eq!(pair.len(), 2);
    assert!((0.0..=10.0).contains(&pair[0]));
    assert!((0.0..=20.0).contains(&pair[1]));
    let ranged = env.vec("random([100, 200], [300, 400])");
    assert!((100.0..=300.0).contains(&ranged[0]));
    assert!((200.0..=400.0).contains(&ranged[1]));
}

#[test]
fn gauss_random_is_mostly_inside_and_keeps_dimensions() {
    let mut env = env();
    let mut inside = 0;
    for seed in 0..400 {
        let value = env.num(&format!("seedRandom({seed}, true); gaussRandom()"));
        if (0.0..=1.0).contains(&value) {
            inside += 1;
        }
    }
    assert!((320..390).contains(&inside), "inside {inside}");
    let mut inside_scaled = 0;
    for seed in 0..400 {
        let value = env.num(&format!("seedRandom({seed}, true); gaussRandom(10)"));
        if (0.0..=10.0).contains(&value) {
            inside_scaled += 1;
        }
    }
    assert!(
        (320..390).contains(&inside_scaled),
        "scaled {inside_scaled}"
    );
    assert_eq!(env.vec("gaussRandom([10, 20])").len(), 2);
    let mut inside_span = 0;
    for seed in 0..400 {
        let value = env.num(&format!("seedRandom({seed}, true); gaussRandom(0, 10)"));
        if (0.0..=10.0).contains(&value) {
            inside_span += 1;
        }
    }
    assert!((320..390).contains(&inside_span), "span {inside_span}");
}

#[test]
fn noise_is_bounded_and_locally_smooth() {
    let mut env = env();
    let at_zero = env.num("noise(0)");
    assert!((-1.0..=1.0).contains(&at_zero));
    let near = env.num("noise(0.01)");
    let far = env.num("noise(10)");
    assert!((at_zero - near).abs() < (at_zero - far).abs());
}

#[test]
fn linear_and_ease_endpoints() {
    let mut env = env();
    assert_eq!(env.num("linear(0, 0, 6, 20, 80)"), 20.0);
    assert_eq!(env.num("linear(6, 0, 6, 20, 80)"), 80.0);
    assert_eq!(env.num("linear(3, 0, 6, 20, 80)"), 50.0);
    assert_eq!(env.num("linear(-1, 0, 6, 20, 80)"), 20.0);
    assert_eq!(env.num("linear(0, 20, 80)"), 20.0);
    assert_eq!(env.num("linear(1, 20, 80)"), 80.0);
    assert_eq!(env.num("linear(0.5, 20, 80)"), 50.0);
    for name in ["ease", "easeIn", "easeOut"] {
        assert_eq!(env.num(&format!("{name}(0, 20, 80)")), 20.0);
        assert_eq!(env.num(&format!("{name}(1, 20, 80)")), 80.0);
        assert_eq!(env.num(&format!("{name}(0, 0, 6, 20, 80)")), 20.0);
        assert_eq!(env.num(&format!("{name}(6, 0, 6, 20, 80)")), 80.0);
    }
    let slope = |source: &str| {
        let mut left = Env::new("Main");
        let y0 = left.num(&source.replace("T", "0"));
        let y1 = left.num(&source.replace("T", "0.001"));
        (y1 - y0) / 0.001
    };
    assert!(slope("ease(T, 20, 80)").abs() < 1.0);
    let end = {
        let mut left = Env::new("Main");
        let y0 = left.num("ease(0.999, 20, 80)");
        let y1 = left.num("ease(1, 20, 80)");
        (y1 - y0) / 0.001
    };
    assert!(end.abs() < 1.0);
    assert!(slope("easeIn(T, 20, 80)").abs() < 1.0);
    assert!(slope("easeIn(T, 0, 6, 20, 80)").abs() < 1.0);
    let ease_out_end = {
        let mut left = Env::new("Main");
        let y0 = left.num("easeOut(0.999, 20, 80)");
        let y1 = left.num("easeOut(1, 20, 80)");
        (y1 - y0) / 0.001
    };
    assert!(ease_out_end.abs() < 1.0);
    let ease_out_range_end = {
        let mut left = Env::new("Main");
        let y0 = left.num("easeOut(5.999, 0, 6, 20, 80)");
        let y1 = left.num("easeOut(6, 0, 6, 20, 80)");
        (y1 - y0) / 0.001
    };
    assert!(ease_out_range_end.abs() < 1.0);
}

#[test]
fn color_and_angle_conversion() {
    let mut env = env();
    let hsl = env.vec("rgbToHsl([1, 0, 0, 1])");
    close(hsl[1], 1.0);
    close(hsl[3], 1.0);
    close_vec(
        &env.vec("hslToRgb(rgbToHsl([0.2, 0.4, 0.6, 0.8]))"),
        &[0.2, 0.4, 0.6, 0.8],
    );
    for source in [
        "hexToRgb(\"FF00FF\")",
        "hexToRgb(\"#FF00FF\")",
        "hexToRgb(\"0xFF00FF\")",
        "hexToRgb(\"0XFF00FFFF\")",
    ] {
        close_vec(&env.vec(source), &[1.0, 0.0, 1.0, 1.0]);
    }
    assert_eq!(env.num("degreesToRadians(0)"), 0.0);
    close(env.num("degreesToRadians(180)"), std::f64::consts::PI);
    assert_eq!(env.num("radiansToDegrees(0)"), 0.0);
    close(env.num("radiansToDegrees(degreesToRadians(180))"), 180.0);
}
