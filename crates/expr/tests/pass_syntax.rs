#[path = "support/mod.rs"]
mod support;

use expr::{ComponentDrop, EngineMode, Panel};
use model::{Keyframe, Marker, PropertyValue, Time};
use support::Env;

fn env() -> Env {
    let mut env = Env::new("Main");
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
    env.scene.add_value(
        env.comp,
        env.layer,
        Some(group),
        "Position",
        "position",
        PropertyValue::TwoD([1.0, 2.0]),
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
        None,
        "Audio Levels",
        "audioLevels",
        PropertyValue::TwoD([0.0, 0.0]),
    );
    env.scene.add_value(
        env.comp,
        env.layer,
        None,
        "Time Remap",
        "timeRemap",
        PropertyValue::Scalar(0.0),
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
        "pointOfInterest",
        "depthOfField",
        "focusDistance",
        "blurLevel",
        "coneAngle",
        "coneFeather",
        "shadowDarkness",
        "shadowDiffusion",
    ] {
        env.scene.add_value(
            env.comp,
            env.layer,
            None,
            name,
            name,
            PropertyValue::Scalar(1.0),
        );
    }
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
        PropertyValue::Scalar(10.0),
    );
    let camera = env.scene.add_camera(env.comp, "Cam");
    env.scene.layer_attr_mut(camera).unwrap().enabled = true;
    env.scene.set_3d(env.comp, env.layer, true);
    env
}

fn legacy_fails(bad: &str, good: &str) {
    let mut env = env();
    let bad_prop = env.scene.add_value(
        env.comp,
        env.layer,
        None,
        "Bad",
        "bad",
        PropertyValue::Scalar(0.0),
    );
    let good_prop = env.scene.add_value(
        env.comp,
        env.layer,
        None,
        "Good",
        "good",
        PropertyValue::Scalar(0.0),
    );
    env.prop = bad_prop;
    assert!(env.eval(bad).is_err(), "{bad} should fail");
    assert!(
        !env.scene.errors_for(env.layer).is_empty(),
        "{bad} should stay on the layer"
    );
    env.scene.add_key(
        env.comp,
        env.layer,
        good_prop,
        0.0,
        PropertyValue::Scalar(0.0),
    );
    env.scene.add_key(
        env.comp,
        env.layer,
        good_prop,
        1.0,
        PropertyValue::Scalar(1.0),
    );
    env.prop = good_prop;
    assert!(
        env.eval(good).is_ok(),
        "{good} should succeed: {:?}",
        env.eval(good)
    );
}

macro_rules! legacy {
    ($name:ident, $bad:literal, $good:literal) => {
        #[test]
        fn $name() {
            legacy_fails($bad, $good);
        }
    };
}

legacy!(legacy_this_comp, "this_comp", "thisComp.name");
legacy!(legacy_this_layer, "this_layer", "thisLayer.name");
legacy!(legacy_this_property, "this_property", "thisProperty.name");
legacy!(legacy_color_depth, "color_depth", "colorDepth");
legacy!(legacy_has_parent, "has_parent", "hasParent");
legacy!(legacy_in_point, "in_point", "inPoint");
legacy!(legacy_out_point, "out_point", "outPoint");
legacy!(legacy_start_time, "start_time", "startTime");
legacy!(legacy_has_video, "has_video", "hasVideo");
legacy!(legacy_has_audio, "has_audio", "hasAudio");
legacy!(legacy_audio_active, "audio_active", "audioActive");
legacy!(legacy_anchor_point, "anchor_point", "anchorPoint");
legacy!(legacy_audio_levels, "audio_levels", "audioLevels");
legacy!(legacy_time_remap, "time_remap", "timeRemap");
legacy!(legacy_casts_shadows, "casts_shadows", "castsShadows");
legacy!(
    legacy_light_transmission,
    "light_transmission",
    "lightTransmission"
);
legacy!(legacy_accepts_shadows, "accepts_shadows", "acceptsShadows");
legacy!(legacy_accepts_lights, "accepts_lights", "acceptsLights");
legacy!(legacy_frame_duration, "frame_duration", "frameDuration");
legacy!(legacy_shutter_angle, "shutter_angle", "shutterAngle");
legacy!(legacy_shutter_phase, "shutter_phase", "shutterPhase");
legacy!(legacy_num_layers, "num_layers", "numLayers");
legacy!(legacy_pixel_aspect, "pixel_aspect", "pixelAspect");
legacy!(
    legacy_point_of_interest,
    "point_of_interest",
    "pointOfInterest"
);
legacy!(legacy_depth_of_field, "depth_of_field", "depthOfField");
legacy!(legacy_focus_distance, "focus_distance", "focusDistance");
legacy!(legacy_blur_level, "blur_level", "blurLevel");
legacy!(legacy_cone_angle, "cone_angle", "coneAngle");
legacy!(legacy_cone_feather, "cone_feather", "coneFeather");
legacy!(legacy_shadow_darkness, "shadow_darkness", "shadowDarkness");
legacy!(
    legacy_shadow_diffusion,
    "shadow_diffusion",
    "shadowDiffusion"
);
legacy!(legacy_active_camera, "active_camera", "activeCamera.name");
legacy!(legacy_value_at_time, "value_at_time", "valueAtTime(0)");
legacy!(
    legacy_velocity_at_time,
    "velocity_at_time",
    "velocityAtTime(0)"
);
legacy!(legacy_speed_at_time, "speed_at_time", "speedAtTime(0)");
legacy!(legacy_nearest_key, "nearest_key", "nearestKey(0)");
legacy!(
    legacy_posterize_time,
    "posterize_time",
    "posterizeTime(1); 1"
);
legacy!(legacy_look_at, "look_at", "lookAt([0,0,0],[0,0,1])");
legacy!(legacy_seed_random, "seed_random", "seedRandom(1, true); 1");
legacy!(legacy_gauss_random, "gauss_random", "gaussRandom()");
legacy!(legacy_ease_in, "ease_in", "easeIn(0, 0, 1)");
legacy!(legacy_ease_out, "ease_out", "easeOut(0, 0, 1)");
legacy!(legacy_rgb_to_hsl, "rgb_to_hsl", "rgbToHsl([1,0,0,1])");
legacy!(legacy_hsl_to_rgb, "hsl_to_rgb", "hslToRgb([0,1,0.5,1])");
legacy!(
    legacy_degrees_to_radians,
    "degrees_to_radians",
    "degreesToRadians(0)"
);
legacy!(
    legacy_radians_to_degrees,
    "radians_to_degrees",
    "radiansToDegrees(0)"
);
legacy!(
    legacy_from_comp_to_surface,
    "from_comp_to_surface",
    "fromCompToSurface([0,0,0])"
);
legacy!(legacy_to_comp_vec, "to_comp_vec", "toCompVec([1,0])");
legacy!(legacy_from_comp_vec, "from_comp_vec", "fromCompVec([1,0])");
legacy!(legacy_to_world_vec, "to_world_vec", "toWorldVec([1,0])");
legacy!(
    legacy_from_world_vec,
    "from_world_vec",
    "fromWorldVec([1,0])"
);
legacy!(legacy_to_comp, "to_comp", "toComp([0,0])");
legacy!(legacy_from_comp, "from_comp", "fromComp([0,0])");
legacy!(legacy_to_world, "to_world", "toWorld([0,0])");
legacy!(legacy_from_world, "from_world", "fromWorld([0,0])");
legacy!(
    legacy_temporal_wiggle,
    "temporal_wiggle",
    "temporalWiggle(1, 0.1)"
);
legacy!(
    legacy_loop_in_duration,
    "loop_in_duration",
    "loopInDuration(\"cycle\", 1)"
);
legacy!(
    legacy_loop_out_duration,
    "loop_out_duration",
    "loopOutDuration(\"cycle\", 1)"
);
legacy!(legacy_loop_in, "loop_in", "loopIn(\"cycle\")");
legacy!(legacy_loop_out, "loop_out", "loopOut(\"cycle\")");

fn dollar_fails(source: &str) {
    let mut env = env();
    assert!(env.eval(source).is_err(), "{source}");
    assert!(!env.scene.errors_for(env.layer).is_empty());
}

macro_rules! dollar {
    ($name:ident, $source:literal) => {
        #[test]
        fn $name() {
            dollar_fails($source);
        }
    };
}

dollar!(dollar_file_name, "$.fileName");
dollar!(dollar_hi_res_times, "$.hiResTimes");
dollar!(dollar_stack, "$.stack");
dollar!(dollar_eval_file, "$.evalFile()");
dollar!(dollar_list, "$.list()");
dollar!(dollar_setenv, "$.setenv()");
dollar!(dollar_getenv, "$.getenv()");
dollar!(dollar_app_encoding, "$.appEncoding");
dollar!(dollar_build_date, "$.buildDate");
dollar!(dollar_decimal_point, "$.decimalPoint");
dollar!(dollar_dictionary, "$.dictionary");
dollar!(dollar_error, "$.error");
dollar!(dollar_flags, "$.flags");
dollar!(dollar_include_path, "$.includePath");
dollar!(dollar_level, "$.level");
dollar!(dollar_line, "$.line");
dollar!(dollar_locale, "$.locale");
dollar!(dollar_localize, "$.localize");
dollar!(dollar_mem_cache, "$.memCache");
dollar!(dollar_os, "$.os");
dollar!(dollar_screens, "$.screens");
dollar!(dollar_strict, "$.strict");
dollar!(dollar_version, "$.version");

#[test]
fn dollar_supported_members_do_not_fail() {
    let mut env = env();
    assert!(env.eval("$.build").is_ok());
    assert!(env.eval("$.engineName").is_ok());
    assert!(env.eval("$.global").is_ok());
}

#[test]
fn reflect_and_to_source_fail() {
    let mut env = env();
    assert!(env.eval("reflect.properties").is_err());
    assert!(!env.scene.errors_for(env.layer).is_empty());
    env.scene.clear_error(env.prop);
    assert!(env.eval("reflect.methods").is_err());
    env.scene.clear_error(env.prop);
    assert!(env.eval("[1, 2].toSource()").is_err());
    assert!(!env.scene.errors_for(env.layer).is_empty());
}

#[test]
fn jsxbin_eval_fails_and_prefixed_eval_runs() {
    let mut env = env();
    assert!(env.eval("eval(\"@JSXBIN@binary\")").is_err());
    assert!(!env.scene.errors_for(env.layer).is_empty());
    env.scene.clear_error(env.prop);
    assert_eq!(env.num("eval(\"thisLayer.add([1, 2], [3, 4])\")[0]"), 4.0);
}

#[test]
fn trailing_if_without_else_keeps_the_documented_error() {
    let mut env = env();
    let error = env
        .eval("var x = 50; if (time > 1) { x = 100 }")
        .unwrap_err();
    assert_eq!(
        error.message,
        "Undefined value used in expression (could be an out of range array subscript?)"
    );
    assert!(!env.scene.errors_for(env.layer).is_empty());
    env.time = 0.0;
    assert_eq!(
        env.num("var x = 50; if (time > 1) { x = 100 } else { x = 50 }\nx"),
        50.0
    );
    env.time = 2.0;
    assert_eq!(
        env.num("var x = 50; if (time > 1) { x = 100 } else { x = 50 }\nx"),
        100.0
    );
}

#[test]
fn single_line_if_else_fails_but_split_forms_work() {
    let mut env = env();
    assert!(env.eval("if ( time > 1 ) 100 else 50;").is_err());
    assert!(!env.scene.errors_for(env.layer).is_empty());
    env.time = 0.0;
    assert_eq!(env.num("if (time > 1)\n100\nelse\n50"), 50.0);
    env.time = 2.0;
    assert_eq!(env.num("if (time > 1) { 100 } else { 50 }"), 100.0);
    env.time = 0.0;
    assert_eq!(env.num("time > 1 ? 100 : 50"), 50.0);
    env.time = 2.0;
    assert_eq!(env.num("time > 1 ? 100 : 50"), 100.0);
}

#[test]
fn function_declaration_must_not_be_last() {
    let mut env = env();
    env.prop = env.scene.add_value(
        env.comp,
        env.layer,
        None,
        "Ten",
        "ten",
        PropertyValue::Scalar(10.0),
    );
    let error = env
        .eval("timesTen(value); function timesTen(val) { return val * 10 }")
        .unwrap_err();
    assert_eq!(
        error.message,
        "Object of type found where a Number, Array, or Property is needed"
    );
    assert_eq!(
        env.num("function timesTen(val) { return val * 10 }\ntimesTen(value)"),
        100.0
    );
}

#[test]
fn pickwhip_writes_compact_or_indexed_paths() {
    let mut env = env();
    let group = env
        .scene
        .layer(env.comp, env.layer)
        .unwrap()
        .properties
        .iter()
        .find(|p| p.match_name == "transform")
        .unwrap()
        .id;
    let position = env
        .scene
        .layer(env.comp, env.layer)
        .unwrap()
        .property("position")
        .unwrap()
        .id;
    assert!(env.scene.compact_english);
    let compact = env
        .scene
        .pickwhip(env.comp, env.layer, &[group, position], None);
    assert_eq!(compact, "thisComp.layer(\"Layer 1\").transform.position");
    env.scene.compact_english = false;
    let indexed = env
        .scene
        .pickwhip(env.comp, env.layer, &[group, position], None);
    assert_eq!(
        indexed,
        "thisComp.layer(\"Layer 1\")(\"Transform\")(\"Position\")"
    );
    assert_eq!(env.vec(&compact), env.vec(&indexed));
    env.scene.compact_english = true;
    let scale = env.scene.add_value(
        env.comp,
        env.layer,
        Some(group),
        "Scale",
        "scale",
        PropertyValue::TwoD([0.0, 0.0]),
    );
    let component = env.scene.pickwhip(
        env.comp,
        env.layer,
        &[group, position],
        Some(ComponentDrop {
            index: 1,
            dimensions: 2,
        }),
    );
    env.prop = scale;
    let written = env.vec(&component);
    assert_eq!(written[0], written[1]);
    assert_eq!(written[0], 2.0);
}

#[test]
fn pickwhip_renames_duplicate_layers() {
    let mut env = env();
    env.scene.layer_mut(env.comp, env.layer).unwrap().name = "Solid 1".into();
    let other = env.scene.add_solid(env.comp, "Solid 1", 10, 10);
    let group = env
        .scene
        .add_group(env.comp, other, None, "Transform", "transform");
    let position = env.scene.add_value(
        env.comp,
        other,
        Some(group),
        "Position",
        "position",
        PropertyValue::TwoD([8.0, 9.0]),
    );
    let text = env
        .scene
        .pickwhip(env.comp, other, &[group, position], None);
    let renamed = env.scene.layer(env.comp, other).unwrap().name.clone();
    assert_ne!(renamed, "Solid 1");
    assert!(renamed.chars().last().unwrap().is_ascii_digit());
    assert!(text.contains(&renamed));
    close_vec_local(&env.vec(&text), &[8.0, 9.0]);
}

fn close_vec_local(left: &[f64], right: &[f64]) {
    assert_eq!(left, right);
}

#[test]
fn pickwhip_inserts_or_replaces() {
    let existing = "wiggle(3, )";
    let caret = existing.find(')').unwrap();
    let inserted =
        expr::Scene::insert_reference(existing, Some(caret), "effect(\"Slider\")(\"Slider\")");
    assert!(inserted.starts_with("wiggle(3, "));
    assert!(inserted.ends_with(')'));
    assert!(inserted.contains("effect(\"Slider\")(\"Slider\")"));
    let replaced = expr::Scene::insert_reference(existing, None, "effect(\"Slider\")(\"Slider\")");
    assert_eq!(replaced, "effect(\"Slider\")(\"Slider\")");
}

#[test]
fn rename_follows_unique_names_and_keeps_broken_text() {
    let mut env = env();
    env.scene.set_expression(
        env.comp,
        env.layer,
        env.prop,
        Some("thisComp.layer(\"Hero\").transform.position".into()),
        true,
    );
    let hero = env.scene.add_solid(env.comp, "Hero", 10, 10);
    let group = env
        .scene
        .add_group(env.comp, hero, None, "Transform", "transform");
    env.scene.add_value(
        env.comp,
        hero,
        Some(group),
        "Position",
        "position",
        PropertyValue::TwoD([4.0, 5.0]),
    );
    env.scene.rename_layer(env.comp, hero, "Heroic");
    let source = env
        .scene
        .expression_of(env.comp, env.layer, env.prop)
        .unwrap()
        .0;
    assert!(source.contains("Heroic"));
    assert!(!source.contains("layer(\"Hero\")"));
    close_vec_local(&env.vec(&source), &[4.0, 5.0]);
    let other = env.scene.add_solid(env.comp, "Taken", 10, 10);
    env.scene.set_expression(
        env.comp,
        env.layer,
        env.prop,
        Some("thisComp.layer(\"Solo\").transform.position".into()),
        true,
    );
    let solo = env.scene.add_solid(env.comp, "Solo", 10, 10);
    env.scene.rename_layer(env.comp, solo, "Taken");
    let kept = env
        .scene
        .expression_of(env.comp, env.layer, env.prop)
        .unwrap()
        .0;
    assert!(kept.contains("layer(\"Solo\")"));
    assert!(env.eval(&kept).is_err());
    assert!(!env.scene.banners(Panel::Timeline, 80).is_empty());
    let _ = other;
}

#[test]
fn precompose_breaks_this_comp_reference_and_can_reveal_the_new_home() {
    let mut env = env();
    let alpha = env.scene.add_solid(env.comp, "A", 10, 10);
    let beta = env.scene.add_solid(env.comp, "B", 10, 10);
    let group = env
        .scene
        .add_group(env.comp, alpha, None, "Transform", "transform");
    env.scene.add_value(
        env.comp,
        alpha,
        Some(group),
        "Position",
        "position",
        PropertyValue::TwoD([1.0, 1.0]),
    );
    env.scene.set_expression(
        env.comp,
        env.layer,
        env.prop,
        Some("thisComp.layer(\"A\").transform.position".into()),
        true,
    );
    let new_comp = env.scene.precompose(env.comp, &[alpha, beta], "Pre");
    let source = env
        .scene
        .expression_of(env.comp, env.layer, env.prop)
        .unwrap()
        .0;
    assert!(source.contains("layer(\"A\")"));
    assert!(!env.scene.errors_for(env.layer).is_empty());
    let reveal = env.scene.reveal_fix(0).unwrap();
    assert_eq!(reveal.comp, new_comp);
    assert_eq!(env.scene.open_comp, Some(new_comp));
}

#[test]
fn banners_navigate_truncate_and_follow_the_enable_switch() {
    let mut env = env();
    let long_name = "N".repeat(80);
    env.scene.set_expression(
        env.comp,
        env.layer,
        env.prop,
        Some(format!("thisComp.layer(\"{long_name}\")")),
        true,
    );
    env.eval(&format!("thisComp.layer(\"{long_name}\")"))
        .unwrap_err();
    let timeline = env.scene.banners(Panel::Timeline, 12);
    let composition = env.scene.banners(Panel::Composition, 12);
    assert_eq!(timeline.len(), 1);
    assert_eq!(composition.len(), 1);
    assert_eq!(timeline[0].line.chars().count(), 12);
    assert!(!timeline[0].line.contains('\n'));
    assert!(timeline[0].full_text.chars().count() > 12);
    env.eval("1").unwrap();
    assert!(env.scene.banners(Panel::Timeline, 40).is_empty());
    let other = env.scene.add_value(
        env.comp,
        env.layer,
        None,
        "Other",
        "other",
        PropertyValue::Scalar(0.0),
    );
    env.prop = env
        .scene
        .layer(env.comp, env.layer)
        .unwrap()
        .property("opacity")
        .unwrap()
        .id;
    env.eval("thisComp.layer(\"missing-a\")").unwrap_err();
    env.prop = other;
    env.eval("thisComp.layer(\"missing-b\")").unwrap_err();
    assert_eq!(
        env.scene
            .current_banner(Panel::Timeline, 40)
            .unwrap()
            .full_text
            .contains("missing-a"),
        true
    );
    env.scene.step_banner(1);
    assert!(env
        .scene
        .current_banner(Panel::Timeline, 40)
        .unwrap()
        .full_text
        .contains("missing-b"));
    env.scene.step_banner(-1);
    assert!(env
        .scene
        .current_banner(Panel::Timeline, 40)
        .unwrap()
        .full_text
        .contains("missing-a"));
    env.scene.banner_open = false;
    assert!(env
        .scene
        .current_banner(Panel::Timeline, 40)
        .unwrap()
        .line
        .is_empty());
    env.scene.banner_open = true;
    assert!(!env
        .scene
        .current_banner(Panel::Timeline, 40)
        .unwrap()
        .line
        .is_empty());
    let dialog = env.scene.current_banner(Panel::Timeline, 8).unwrap();
    assert!(dialog.full_text.len() > dialog.line.len());
    assert!(env.scene.has_warning(env.prop));
    let shown = env.scene.show_expressions_with_errors(env.layer);
    assert!(shown.contains(&env.prop));
    env.scene.set_expression(
        env.comp,
        env.layer,
        env.prop,
        Some("thisComp.layer(\"missing-b\")".into()),
        true,
    );
    env.scene.set_expression(
        env.comp,
        env.layer,
        env.prop,
        Some("thisComp.layer(\"missing-b\")".into()),
        false,
    );
    assert!(env
        .scene
        .expression_of(env.comp, env.layer, env.prop)
        .unwrap()
        .0
        .contains("missing-b"));
    assert!(env
        .scene
        .banners(Panel::Timeline, 40)
        .iter()
        .all(|banner| banner.property != env.prop));
    env.scene.set_expression(
        env.comp,
        env.layer,
        env.prop,
        Some("thisComp.layer(\"missing-b\")".into()),
        true,
    );
    assert!(env
        .scene
        .banners(Panel::Timeline, 40)
        .iter()
        .any(|banner| banner.property == env.prop));
    env.scene.show_banners = false;
    assert!(env.scene.banners(Panel::Timeline, 40).is_empty());
    env.scene.show_banners = true;
    assert!(!env.scene.banners(Panel::Timeline, 40).is_empty());
    let reveal = env.scene.reveal_expression(0).unwrap();
    assert_eq!(env.scene.open_comp, Some(reveal.comp));
    assert!(env.scene.revealed.unwrap().property.is_some());
}

#[test]
fn failure_keeps_the_error_on_the_layer() {
    let mut env = env();
    env.scene.set_expression(
        env.comp,
        env.layer,
        env.prop,
        Some("thisComp.layer(\"nope\")".into()),
        true,
    );
    env.eval("thisComp.layer(\"nope\")").unwrap_err();
    assert!(!env.scene.errors_for(env.layer).is_empty());
    assert!(
        env.scene
            .expression_of(env.comp, env.layer, env.prop)
            .unwrap()
            .1
    );
}

#[test]
fn legacy_engine_mode_still_rejects_legacy_syntax() {
    let mut env = env();
    env.scene.engine_mode = EngineMode::Legacy;
    assert_eq!(env.num("const x = 1; x"), 1.0);
    assert!(env.eval("this_layer").is_err());
    let _ = Keyframe::at(Time::from_seconds(0.0), PropertyValue::Scalar(0.0));
    let _ = Marker::new("x");
}
