use effects_key_stylize::{
    defaults_from, effect, frame8, instantiate, pixel8, render, render_at, solid8, threshold_at,
    BitsPerChannel, CpuApply, CpuInput, EffectStack, Frame, Frame16, KnownDefault, Param,
    ParamIndex, ParamValue, ParamValues, Rgba, Time, EFFECTS,
};
use model::{Keyframe, Property, PropertyId, PropertyValue};

fn run(
    id: &str,
    source: &Frame,
    setup: impl FnOnce(&mut effects_key_stylize::EffectInstance),
) -> Frame {
    render(id, source, setup).unwrap()
}

fn at(frame: &Frame, x: u32, y: u32) -> [u8; 4] {
    pixel8(frame, x, y)
}

#[test]
fn catalog_lists_each_effect_and_named_parameter() {
    assert_eq!(EFFECTS.len(), 54);
    let params: usize = EFFECTS.iter().map(|item| item.params.len()).sum();
    assert_eq!(params, 464);
    let mut ids = std::collections::BTreeSet::new();
    for item in EFFECTS {
        assert!(ids.insert(item.id), "{}", item.id);
        assert!(item.menu.starts_with("Effect > "));
        assert!(item.menu.ends_with(item.name));
        let mut param_ids = std::collections::BTreeSet::new();
        for param in item.params {
            assert!(param_ids.insert(param.id), "{} {}", item.id, param.id);
        }
    }
}

#[test]
fn published_controls_are_only_fully_confirmed_defaults() {
    let mut published = Vec::new();
    for item in EFFECTS {
        for param in item.params.iter().filter(|param| param.is_published()) {
            published.push((item.id, param.id));
            assert!(param.formal_name, "{}", param.id);
            assert!(!matches!(param.default, KnownDefault::Unknown));
        }
        for param in item.params {
            if matches!(param.default, KnownDefault::Unknown) {
                assert!(!param.is_published(), "{} を推測して公開している", param.id);
            }
        }
    }
    published.sort_unstable();
    assert_eq!(
        published,
        vec![
            ("add-grain", "invert-match"),
            ("add-grain", "matching-color"),
            ("add-grain", "viewing-mode"),
            ("brush-strokes", "paint-surface"),
            ("cc-hex-tile", "lock-center-tile"),
            ("cc-kaleida", "floating-center"),
            ("cc-mr-smoothie", "property"),
            ("cc-threshold", "invert"),
            ("cc-threshold-rgb", "invert-blue"),
            ("cc-threshold-rgb", "invert-green"),
            ("cc-threshold-rgb", "invert-red"),
            ("color-difference-key", "key-color"),
            ("match-grain", "matching-color"),
            ("match-grain", "sample-box-color"),
            ("match-grain", "viewing-mode"),
            ("remove-grain", "matching-color"),
            ("remove-grain", "sample-box-color"),
        ]
    );
}

#[test]
fn confirmed_defaults_match_the_document() {
    let key = effect("color-difference-key")
        .unwrap()
        .param("key-color")
        .unwrap();
    assert_eq!(
        key.default,
        KnownDefault::Color(Rgba::new(0.0, 0.0, 1.0, 1.0))
    );
    assert_eq!(
        effect("color-difference-key")
            .unwrap()
            .param("gamma-a")
            .unwrap()
            .default,
        KnownDefault::Float(1.0)
    );
    assert_eq!(
        effect("cc-simple-wire-removal")
            .unwrap()
            .param("mirror-blend")
            .unwrap()
            .default,
        KnownDefault::Float(25.0)
    );
    assert_eq!(
        effect("add-grain")
            .unwrap()
            .param("midpoint")
            .unwrap()
            .default,
        KnownDefault::Float(0.5)
    );
    assert_eq!(
        effect("add-grain")
            .unwrap()
            .param("animation-speed")
            .unwrap()
            .default,
        KnownDefault::Float(1.0)
    );
    assert_eq!(
        effect("add-grain")
            .unwrap()
            .param("blend-amount")
            .unwrap()
            .default,
        KnownDefault::Float(0.0)
    );
    assert_eq!(
        effect("add-grain")
            .unwrap()
            .param("monochromatic")
            .unwrap()
            .default,
        KnownDefault::Checkbox(false)
    );
    assert_eq!(
        effect("add-grain")
            .unwrap()
            .param("invert-match")
            .unwrap()
            .default,
        KnownDefault::Checkbox(false)
    );
    assert_eq!(
        effect("fractal-noise")
            .unwrap()
            .param("contrast")
            .unwrap()
            .default,
        KnownDefault::Float(100.0)
    );
    assert_eq!(
        effect("curl-noise")
            .unwrap()
            .param("turbulence-speed")
            .unwrap()
            .default,
        KnownDefault::Float(10.0)
    );
    assert_eq!(
        effect("linear-color-key")
            .unwrap()
            .param("match-colors")
            .unwrap()
            .default,
        KnownDefault::Choice("RGB")
    );
    assert_eq!(
        effect("cc-hex-tile")
            .unwrap()
            .param("lock-center-tile")
            .unwrap()
            .default,
        KnownDefault::Checkbox(false)
    );
    assert_eq!(
        effect("cc-threshold-rgb")
            .unwrap()
            .param("invert-red")
            .unwrap()
            .default,
        KnownDefault::Checkbox(false)
    );
    let list = effect("add-grain").unwrap().param_list();
    let mode = list
        .iter()
        .find(|(_, param)| param.name() == "Viewing Mode")
        .unwrap()
        .1;
    match mode {
        Param::Popup {
            default_index,
            choices,
            ..
        } => {
            assert_eq!(choices[*default_index as usize], "Preview");
            assert_eq!(
                choices,
                &[
                    "Preview".to_string(),
                    "Blending Matte".to_string(),
                    "Final Output".to_string()
                ]
            );
        }
        _ => panic!("Viewing Mode はメニュー"),
    }
}

#[test]
fn known_ranges_reject_values_outside_the_document() {
    let mut poster = instantiate("posterize").unwrap();
    assert!(poster.set_float("level", 1.0).is_err());
    assert!(poster.set_float("level", 2.0).is_ok());
    assert!(poster.set_float("level", 255.0).is_ok());
    let mut curl = instantiate("curl-noise").unwrap();
    assert!(curl.set_float("scale", 10001.0).is_err());
    assert!(curl.set_float("scale", 10000.0).is_ok());
    assert!(curl.set_float("turbulence-speed", 501.0).is_err());
    assert!(curl.set_float("swirl", 361.0).is_err());
    assert!(curl.set_float("sample-count", 2.0).is_err());
    assert!(curl.set_float("sample-radius", 201.0).is_err());
}

#[test]
fn keyframable_parameters_accept_a_keyframe() {
    let mut count = 0;
    for item in EFFECTS {
        for param in item.params {
            if param.keyframable == Some(true) {
                count += 1;
                let mut property = Property::value(
                    PropertyId::new(1),
                    param.name,
                    param.id,
                    PropertyValue::Scalar(0.0),
                );
                assert!(
                    property.push_keyframe(Keyframe::at(Time::ZERO, PropertyValue::Scalar(1.0)))
                );
                assert_eq!(property.keyframes().len(), 1);
            }
        }
    }
    assert!(count > 0);
}

#[test]
fn every_effect_can_be_added_to_a_stack() {
    let mut stack = EffectStack::new();
    for item in EFFECTS {
        stack.add(item.id).unwrap();
    }
    assert_eq!(stack.len(), 54);
}

#[test]
fn color_range_rejects_16_bpc() {
    let source = Frame::Bpc16(Frame16::new(2, 2, 4, 0));
    let effect = instantiate("color-range").unwrap();
    let values = ParamValues::new();
    let input = CpuInput {
        time: Time::ZERO,
        bits: BitsPerChannel::Bpc16,
        params: &values,
        frame: &source,
    };
    let mut output = source.clone();
    let error = effect.apply_cpu(&input, &mut output).unwrap_err();
    assert!(error.message.contains("16"));
}

#[test]
fn advanced_spill_suppressor_reduces_the_dominant_spill() {
    let source = solid8(1, 1, [20, 220, 20, 255]);
    let standard = run("advanced-spill-suppressor", &source, |fx| {
        fx.set_choice("method", "Standard").unwrap();
    });
    let ultra = run("advanced-spill-suppressor", &source, |fx| {
        fx.set_choice("method", "Ultra").unwrap();
    });
    assert!(at(&standard, 0, 0)[1] < 220);
    assert!(at(&ultra, 0, 0)[1] < 220);
    let red = run(
        "advanced-spill-suppressor",
        &solid8(1, 1, [220, 20, 20, 255]),
        |fx| {
            fx.set_choice("method", "Standard").unwrap();
        },
    );
    assert_eq!(at(&red, 0, 0)[0], 220);
}

#[test]
fn cc_simple_wire_removal_fills_the_line_from_the_sides() {
    let mut pixels = [[80, 80, 80, 255]; 16];
    pixels[4] = [255, 255, 255, 255];
    pixels[5] = [255, 255, 255, 255];
    let source = frame8(4, 4, &pixels);
    let out = run("cc-simple-wire-removal", &source, |fx| {
        fx.set_point("point-a", [0.0, 1.0]).unwrap();
        fx.set_point("point-b", [3.0, 1.0]).unwrap();
        fx.set_choice("removal-style", "Fade").unwrap();
        fx.set_float("thickness", 0.6).unwrap();
        fx.set_float("slope", 100.0).unwrap();
        fx.set_float("mirror-blend", 50.0).unwrap();
    });
    assert!(at(&out, 1, 1)[0] < 255);
}

#[test]
fn color_difference_key_drops_blue_and_keeps_red() {
    let source = frame8(2, 1, &[[0, 0, 255, 255], [255, 0, 0, 255]]);
    let out = run("color-difference-key", &source, |fx| {
        fx.set_choice("color-matching-accuracy", "More Accurate")
            .unwrap();
        fx.set_choice("view", "Final Output").unwrap();
        fx.set_float("black-a", 0.0).unwrap();
        fx.set_float("white-a", 1.0).unwrap();
        fx.set_float("black-b", 0.0).unwrap();
        fx.set_float("white-b", 1.0).unwrap();
        fx.set_float("black-final", 0.0).unwrap();
        fx.set_float("white-final", 1.0).unwrap();
    });
    assert_eq!(at(&out, 0, 0)[3], 0);
    assert_eq!(at(&out, 1, 0)[3], 255);
}

#[test]
fn color_range_clears_pixels_inside_the_rgb_box() {
    let source = frame8(2, 1, &[[255, 0, 0, 255], [0, 255, 0, 255]]);
    let out = run("color-range", &source, |fx| {
        fx.set_choice("color-space", "RGB").unwrap();
        fx.set_float("fuzziness", 0.0).unwrap();
        fx.set_float("min-1", 0.8).unwrap();
        fx.set_float("max-1", 1.0).unwrap();
        fx.set_float("min-2", 0.0).unwrap();
        fx.set_float("max-2", 0.2).unwrap();
        fx.set_float("min-3", 0.0).unwrap();
        fx.set_float("max-3", 0.2).unwrap();
    });
    assert_eq!(at(&out, 0, 0)[3], 0);
    assert_eq!(at(&out, 1, 0)[3], 255);
}

#[test]
fn difference_matte_clears_matching_pixels_only() {
    let source = frame8(2, 1, &[[10, 20, 30, 255], [200, 10, 10, 255]]);
    let other = frame8(2, 1, &[[10, 20, 30, 255], [0, 0, 0, 255]]);
    let out = run("difference-matte", &source, |fx| {
        fx.set_layer_frame("difference-layer", &other).unwrap();
        fx.set_choice("if-layer-sizes-differ", "Center").unwrap();
        fx.set_float("matching-tolerance", 0.0).unwrap();
        fx.set_float("matching-softness", 0.0).unwrap();
        fx.set_float("blur-before-difference", 0.0).unwrap();
        fx.set_choice("view", "Final Output").unwrap();
    });
    assert_eq!(at(&out, 0, 0)[3], 0);
    assert_eq!(at(&out, 1, 0)[3], 255);
}

#[test]
fn extract_clears_luminance_outside_the_bar() {
    let source = frame8(2, 1, &[[10, 10, 10, 255], [250, 250, 250, 255]]);
    let out = run("extract", &source, |fx| {
        fx.set_choice("channel", "Luminance").unwrap();
        fx.set_float("black-point", 40.0).unwrap();
        fx.set_float("white-point", 200.0).unwrap();
        fx.set_float("black-softness", 0.0).unwrap();
        fx.set_float("white-softness", 0.0).unwrap();
    });
    assert_eq!(at(&out, 0, 0)[3], 0);
    assert_eq!(at(&out, 1, 0)[3], 0);
}

#[test]
fn inner_outer_key_keeps_the_foreground_mask() {
    let source = solid8(4, 2, [180, 40, 40, 255]);
    let mut mask = [[0, 0, 0, 0]; 8];
    for y in 0..2 {
        for x in 0..2 {
            mask[y * 4 + x] = [255, 255, 255, 255];
        }
    }
    let foreground = frame8(4, 2, &mask);
    let out = run("inner-outer-key", &source, |fx| {
        fx.set_mask_frame("foreground", &foreground).unwrap();
        fx.set_float("single-mask-highlight-radius", 0.0).unwrap();
        fx.set_float("edge-thin", 0.0).unwrap();
        fx.set_float("edge-feather", 0.0).unwrap();
        fx.set_float("edge-threshold", 0.0).unwrap();
        fx.set_checkbox("invert-extraction", false).unwrap();
        fx.set_float("blend-with-original", 0.0).unwrap();
        fx.set_float("brush-radius", 0.0).unwrap();
        fx.set_float("brush-pressure", 0.0).unwrap();
    });
    assert_eq!(at(&out, 0, 0)[3], 255);
    assert_eq!(at(&out, 2, 0)[3], 0);
}

#[test]
fn key_cleaner_restores_alpha_where_rgb_detail_meets_the_matte() {
    let mut pixels = [[20, 20, 20, 255]; 9];
    pixels[5] = [255, 255, 255, 0];
    pixels[4] = [20, 20, 20, 255];
    let source = frame8(3, 3, &pixels);
    let out = run("key-cleaner", &source, |_| {});
    assert!(at(&out, 2, 1)[3] > 0);
}

#[test]
fn keylight_makes_the_screen_color_transparent() {
    let source = frame8(2, 1, &[[0, 255, 0, 255], [255, 0, 0, 255]]);
    let out = run("keylight", &source, |fx| {
        fx.set_color("screen-colour", Rgba::new(0.0, 1.0, 0.0, 1.0))
            .unwrap();
    });
    assert_eq!(at(&out, 0, 0)[3], 0);
    assert!(at(&out, 1, 0)[3] > at(&out, 0, 0)[3]);
    assert_eq!(at(&out, 1, 0)[0], 255);
}

#[test]
fn linear_color_key_endpoints_are_opaque_and_clear() {
    let source = frame8(2, 1, &[[0, 255, 0, 255], [255, 0, 0, 255]]);
    let setup = |fx: &mut effects_key_stylize::EffectInstance, tolerance: f64| {
        fx.set_choice("key-operation", "Key Colors").unwrap();
        fx.set_choice("view", "Final Output").unwrap();
        fx.set_color("key-color", Rgba::new(0.0, 1.0, 0.0, 1.0))
            .unwrap();
        fx.set_float("matching-tolerance", tolerance).unwrap();
        fx.set_float("matching-softness", 0.0).unwrap();
    };
    let none = run("linear-color-key", &source, |fx| setup(fx, 0.0));
    let all = run("linear-color-key", &source, |fx| setup(fx, 100.0));
    let mid = run("linear-color-key", &source, |fx| setup(fx, 40.0));
    assert_eq!(at(&none, 0, 0)[3], 255);
    assert_eq!(at(&all, 1, 0)[3], 0);
    assert!(at(&mid, 0, 0)[3] < at(&mid, 1, 0)[3]);
}

#[test]
fn unmult_clears_black_and_can_unpremultiply() {
    let source = frame8(2, 1, &[[0, 0, 0, 255], [255, 255, 255, 255]]);
    let out = run("unmult", &source, |fx| {
        fx.set_choice("background-color", "Black").unwrap();
        fx.set_float("black-level", 0.0).unwrap();
        fx.set_float("white-level", 1.0).unwrap();
        fx.set_float("softness", 0.0).unwrap();
        fx.set_checkbox("remove-color-matting", false).unwrap();
        fx.set_checkbox("clip-hdr-results", false).unwrap();
    });
    assert_eq!(at(&out, 0, 0)[3], 0);
    assert_eq!(at(&out, 1, 0)[3], 255);
}

#[test]
fn matte_choker_closes_a_hole() {
    let mut pixels = [[0, 0, 0, 0]; 49];
    for y in 1..6 {
        for x in 1..6 {
            pixels[y * 7 + x] = [255, 0, 0, 255];
        }
    }
    pixels[3 * 7 + 3] = [255, 0, 0, 0];
    let source = frame8(7, 7, &pixels);
    let out = run("matte-choker", &source, |fx| {
        fx.set_float("geometric-softness-1", 1.0).unwrap();
        fx.set_float("choke-1", -1.0).unwrap();
        fx.set_float("gray-level-softness-1", 100.0).unwrap();
        fx.set_float("geometric-softness-2", 1.0).unwrap();
        fx.set_float("choke-2", 1.0).unwrap();
        fx.set_float("gray-level-softness-2", 100.0).unwrap();
        fx.set_float("iterations", 1.0).unwrap();
    });
    assert_eq!(at(&out, 3, 3)[3], 255);
}

#[test]
fn unspecified_matte_effects_do_not_invent_a_look() {
    let source = solid8(2, 2, [10, 20, 30, 255]);
    for id in ["mocha-shape", "refine-hard-matte"] {
        let out = run(id, &source, |_| {});
        assert_eq!(at(&out, 0, 0), [10, 20, 30, 255]);
        assert!(effect(id).unwrap().params.is_empty());
    }
}

#[test]
fn simple_choker_shrinks_when_positive() {
    let source = solid8(4, 4, [255, 0, 0, 255]);
    let out = run("simple-choker", &source, |fx| {
        fx.set_choice("view", "Final Output").unwrap();
        fx.set_float("choke-matte", 1.0).unwrap();
    });
    assert_eq!(at(&out, 0, 0)[3], 0);
    assert_eq!(at(&out, 1, 1)[3], 255);
}

#[test]
fn refine_soft_matte_positive_shift_grows_alpha() {
    let mut pixels = [[0, 0, 0, 0]; 9];
    pixels[4] = [255, 0, 0, 255];
    let source = frame8(3, 3, &pixels);
    let setup = |fx: &mut effects_key_stylize::EffectInstance, shift: f64| {
        fx.set_checkbox("calculate-edge-details", false).unwrap();
        fx.set_float("additional-edge-radius", 0.0).unwrap();
        fx.set_checkbox("view-edge-region", false).unwrap();
        fx.set_float("smooth", 0.0).unwrap();
        fx.set_float("feather", 0.0).unwrap();
        fx.set_float("contrast", 0.0).unwrap();
        fx.set_float("shift-edge", shift).unwrap();
        fx.set_float("chatter-reduction", 0.0).unwrap();
        fx.set_checkbox("more-motion-blur", false).unwrap();
        fx.set_checkbox("decontaminate-edge-colors", false).unwrap();
        fx.set_float("decontaminate-amount", 0.0).unwrap();
        fx.set_float("extend-where-smoothed", 0.0).unwrap();
        fx.set_float("increase-decontaminate-radius", 0.0).unwrap();
        fx.set_checkbox("view-map", false).unwrap();
    };
    let grown = run("refine-soft-matte", &source, |fx| setup(fx, 1.0));
    assert_eq!(at(&grown, 0, 1)[3], 255);
}

#[test]
fn add_grain_is_stable_when_animation_speed_is_zero() {
    let source = solid8(4, 4, [40, 40, 40, 255]);
    let setup = |fx: &mut effects_key_stylize::EffectInstance| {
        fx.set_choice("viewing-mode", "Final Output").unwrap();
        fx.set_float("intensity", 1.0).unwrap();
        fx.set_float("size", 1.0).unwrap();
        fx.set_float("softness", 0.0).unwrap();
        fx.set_float("aspect-ratio", 1.0).unwrap();
        fx.set_checkbox("monochromatic", false).unwrap();
        fx.set_choice("blending-mode", "Add").unwrap();
        fx.set_float("random-seed", 3.0).unwrap();
        fx.set_float("animation-speed", 0.0).unwrap();
    };
    let a = render_at("add-grain", &source, Time::from_seconds(0.0), setup).unwrap();
    let b = render_at("add-grain", &source, Time::from_seconds(2.0), setup).unwrap();
    assert_eq!(at(&a, 1, 1), at(&b, 1, 1));
    assert_ne!(at(&a, 1, 1), at(&source, 1, 1));
}

#[test]
fn dust_and_scratches_replaces_an_outlier() {
    let mut pixels = [[10, 10, 10, 255]; 9];
    pixels[4] = [250, 250, 250, 255];
    let source = frame8(3, 3, &pixels);
    let out = run("dust-scratches", &source, |fx| {
        fx.set_float("radius", 1.0).unwrap();
        fx.set_float("threshold", 0.2).unwrap();
    });
    assert!(at(&out, 1, 1)[0] < 250);
}

#[test]
fn fractal_noise_opacity_zero_keeps_the_source() {
    let source = solid8(4, 4, [30, 60, 90, 255]);
    let out = run("fractal-noise", &source, |fx| {
        fx.set_float("opacity", 0.0).unwrap();
        fx.set_float("complexity", 2.0).unwrap();
        fx.set_float("sub-influence", 50.0).unwrap();
        fx.set_angle("evolution", 10.0).unwrap();
    });
    assert_eq!(at(&out, 1, 1), [30, 60, 90, 255]);
}

#[test]
fn median_replaces_a_pixel_with_the_neighborhood_median() {
    let mut pixels = [[0, 0, 0, 255]; 9];
    pixels[4] = [255, 0, 0, 255];
    let source = frame8(3, 3, &pixels);
    let out = run("median", &source, |fx| {
        fx.set_float("radius", 1.0).unwrap();
    });
    assert_eq!(at(&out, 1, 1)[0], 0);
}

#[test]
fn noise_amount_zero_is_unchanged_and_clipping_limits_eight_bpc() {
    let source = solid8(2, 2, [128, 128, 128, 255]);
    let same = run("noise", &source, |fx| {
        fx.set_float("amount-of-noise", 0.0).unwrap();
        fx.set_choice("noise-type", "Use Color Noise").unwrap();
        fx.set_checkbox("clipping", true).unwrap();
    });
    assert_eq!(at(&same, 0, 0), [128, 128, 128, 255]);
    let noisy = run("noise", &solid8(2, 2, [250, 10, 10, 255]), |fx| {
        fx.set_float("amount-of-noise", 1.0).unwrap();
        fx.set_choice("noise-type", "Use Color Noise").unwrap();
        fx.set_checkbox("clipping", true).unwrap();
    });
    assert_ne!(at(&noisy, 0, 0), [250, 10, 10, 255]);
}

#[test]
fn noise_alpha_leaves_rgb_alone() {
    let source = solid8(2, 2, [20, 40, 60, 200]);
    let out = run("noise-alpha", &source, |fx| {
        fx.set_choice("noise", "Unique Random").unwrap();
        fx.set_float("amount", 0.5).unwrap();
        fx.set_choice("original-alpha", "Add").unwrap();
        fx.set_choice("overflow", "Clip").unwrap();
        fx.set_float("random-seed", 2.0).unwrap();
        fx.set_float("noise-phase", 0.0).unwrap();
    });
    let pixel = at(&out, 0, 0);
    assert_eq!(pixel[0], 20);
    assert_eq!(pixel[1], 40);
    assert_eq!(pixel[2], 60);
    assert_ne!(pixel[3], 200);
}

#[test]
fn noise_hls_zero_amounts_keep_color() {
    let source = solid8(2, 2, [200, 20, 20, 255]);
    let out = run("noise-hls", &source, |fx| {
        fx.set_choice("noise", "Uniform").unwrap();
        fx.set_float("hue", 0.0).unwrap();
        fx.set_float("lightness", 0.0).unwrap();
        fx.set_float("saturation", 0.0).unwrap();
        fx.set_float("noise-phase", 0.0).unwrap();
    });
    assert_eq!(at(&out, 0, 0)[0], 200);
}

#[test]
fn noise_hls_auto_speed_zero_does_not_move() {
    let source = solid8(3, 3, [80, 120, 40, 255]);
    let setup = |fx: &mut effects_key_stylize::EffectInstance| {
        fx.set_choice("noise", "Uniform").unwrap();
        fx.set_float("hue", 40.0).unwrap();
        fx.set_float("lightness", 0.1).unwrap();
        fx.set_float("saturation", 0.0).unwrap();
        fx.set_float("noise-animation-speed", 0.0).unwrap();
    };
    let a = render_at("noise-hls-auto", &source, Time::from_seconds(0.0), setup).unwrap();
    let b = render_at("noise-hls-auto", &source, Time::from_seconds(3.0), setup).unwrap();
    assert_eq!(at(&a, 1, 1), at(&b, 1, 1));
}

#[test]
fn remove_grain_lowers_neighbor_difference() {
    let mut pixels = [[0, 0, 0, 255]; 16];
    for (index, pixel) in pixels.iter_mut().enumerate() {
        let v = if index % 2 == 0 { 20 } else { 220 };
        *pixel = [v, v, v, 255];
    }
    let source = frame8(4, 4, &pixels);
    let out = run("remove-grain", &source, |fx| {
        fx.set_float("noise-reduction", 100.0).unwrap();
        fx.set_float("passes", 1.0).unwrap();
        fx.set_float("texture", 0.0).unwrap();
    });
    let before = (at(&source, 0, 0)[0] as i16 - at(&source, 1, 0)[0] as i16).unsigned_abs();
    let after = (at(&out, 0, 0)[0] as i16 - at(&out, 1, 0)[0] as i16).unsigned_abs();
    assert!(after < before);
}

#[test]
fn posterize_uses_the_requested_level_count() {
    let source = frame8(2, 1, &[[40, 40, 40, 255], [220, 220, 220, 255]]);
    let out = run("posterize", &source, |fx| {
        fx.set_float("level", 2.0).unwrap();
    });
    assert_eq!(at(&out, 0, 0)[0], 0);
    assert_eq!(at(&out, 1, 0)[0], 255);
}

#[test]
fn curl_noise_sample_radius_zero_skips_flow_lines() {
    let source = frame8(
        4,
        2,
        &[
            [255, 0, 0, 255],
            [0, 0, 255, 255],
            [255, 0, 0, 255],
            [0, 0, 255, 255],
            [0, 255, 0, 255],
            [0, 255, 0, 255],
            [0, 255, 0, 255],
            [0, 255, 0, 255],
        ],
    );
    let still = run("curl-noise", &source, |fx| {
        fx.set_choice("input-noise", "Internal").unwrap();
        fx.set_angle("evolution", 0.0).unwrap();
        fx.set_float("swirl", 0.0).unwrap();
        fx.set_float("sample-radius", 0.0).unwrap();
        fx.set_choice("view", "Final Render").unwrap();
    });
    let flowing = run("curl-noise", &source, |fx| {
        fx.set_choice("input-noise", "Internal").unwrap();
        fx.set_angle("evolution", 0.0).unwrap();
        fx.set_float("swirl", 90.0).unwrap();
        fx.set_float("sample-radius", 8.0).unwrap();
        fx.set_choice("view", "Final Render").unwrap();
    });
    assert_ne!(at(&still, 0, 0), at(&flowing, 0, 0));
}

#[test]
fn brush_strokes_length_zero_does_not_smear_a_flat_color() {
    let source = solid8(4, 4, [90, 40, 20, 255]);
    let out = run("brush-strokes", &source, |fx| {
        fx.set_angle("stroke-angle", 0.0).unwrap();
        fx.set_float("brush-size", 1.0).unwrap();
        fx.set_float("stroke-length", 0.0).unwrap();
        fx.set_float("stroke-density", 1.0).unwrap();
        fx.set_float("stroke-randomness", 0.0).unwrap();
        fx.set_float("blend-with-original", 0.0).unwrap();
    });
    assert_eq!(at(&out, 1, 1)[0], 90);
}

#[test]
fn cartoon_fill_quantizes_shading() {
    let source = frame8(2, 1, &[[40, 40, 40, 255], [200, 200, 200, 255]]);
    let out = run("cartoon", &source, |fx| {
        fx.set_choice("render", "Fill").unwrap();
        fx.set_float("detail-radius", 0.0).unwrap();
        fx.set_float("detail-threshold", 0.0).unwrap();
        fx.set_float("shading-steps", 2.0).unwrap();
        fx.set_float("shading-smoothness", 0.0).unwrap();
        fx.set_float("edge-threshold", 1.0).unwrap();
        fx.set_float("edge-width", 1.0).unwrap();
        fx.set_float("edge-softness", 0.0).unwrap();
        fx.set_float("edge-opacity", 100.0).unwrap();
    });
    assert_eq!(at(&out, 0, 0)[0], 0);
    assert_ne!(at(&out, 1, 0)[0], 200);
}

#[test]
fn cc_block_load_starts_clear_and_ends_full() {
    let source = solid8(4, 4, [10, 20, 30, 255]);
    let cleared = run("cc-block-load", &source, |fx| {
        fx.set_float("completion", 0.0).unwrap();
        fx.set_float("scans", 2.0).unwrap();
        fx.set_checkbox("start-cleared", true).unwrap();
        fx.set_checkbox("bilinear", false).unwrap();
    });
    let done = run("cc-block-load", &source, |fx| {
        fx.set_float("completion", 100.0).unwrap();
        fx.set_float("scans", 4.0).unwrap();
        fx.set_checkbox("start-cleared", true).unwrap();
        fx.set_checkbox("bilinear", false).unwrap();
    });
    assert_eq!(at(&cleared, 0, 0)[3], 0);
    assert_eq!(at(&done, 1, 1), [10, 20, 30, 255]);
}

#[test]
fn cc_burn_film_zero_keeps_the_picture_and_high_opens_the_center() {
    let source = solid8(5, 5, [80, 80, 80, 255]);
    let same = run("cc-burn-film", &source, |fx| {
        fx.set_float("burn", 0.0).unwrap();
        fx.set_point("center", [2.0, 2.0]).unwrap();
    });
    let burnt = run("cc-burn-film", &source, |fx| {
        fx.set_float("burn", 1.0).unwrap();
        fx.set_point("center", [2.0, 2.0]).unwrap();
    });
    assert_eq!(at(&same, 2, 2)[3], 255);
    assert!(at(&burnt, 2, 2)[3] < at(&burnt, 0, 0)[3]);
}

#[test]
fn cc_threshold_low_is_white_and_high_is_black() {
    let source = solid8(2, 2, [120, 120, 120, 255]);
    let white = run("cc-threshold", &source, |fx| {
        fx.set_float("threshold", 0.0).unwrap();
        fx.set_choice("channel", "Luminance").unwrap();
        fx.set_float("blend-with-original", 0.0).unwrap();
    });
    let black = run("cc-threshold", &source, |fx| {
        fx.set_float("threshold", 2.0).unwrap();
        fx.set_choice("channel", "Luminance").unwrap();
        fx.set_float("blend-with-original", 0.0).unwrap();
    });
    assert_eq!(at(&white, 0, 0)[0], 255);
    assert_eq!(at(&black, 0, 0)[0], 0);
}

#[test]
fn cc_threshold_rgb_splits_channels() {
    let source = solid8(1, 1, [200, 10, 200, 255]);
    let out = run("cc-threshold-rgb", &source, |fx| {
        fx.set_float("red-threshold", 0.5).unwrap();
        fx.set_float("green-threshold", 0.5).unwrap();
        fx.set_float("blue-threshold", 0.5).unwrap();
        fx.set_float("blend-with-original", 0.0).unwrap();
    });
    assert_eq!(at(&out, 0, 0), [255, 0, 255, 255]);
}

#[test]
fn cc_vignette_positive_amount_darkens_the_corner() {
    let source = solid8(5, 5, [200, 200, 200, 255]);
    let out = run("cc-vignette", &source, |fx| {
        fx.set_float("amount", 1.0).unwrap();
        fx.set_angle("angle-of-view", 60.0).unwrap();
        fx.set_point("center", [2.0, 2.0]).unwrap();
    });
    assert!(at(&out, 0, 0)[0] < at(&out, 2, 2)[0]);
}

#[test]
fn emboss_blend_full_matches_the_source_and_zero_is_gray() {
    let source = frame8(3, 1, &[[0, 0, 0, 255], [255, 0, 0, 255], [0, 0, 0, 255]]);
    let same = run("emboss", &source, |fx| {
        fx.set_angle("direction", 0.0).unwrap();
        fx.set_float("relief", 1.0).unwrap();
        fx.set_float("contrast", 100.0).unwrap();
        fx.set_float("blend-with-original", 100.0).unwrap();
    });
    let gray = run("emboss", &source, |fx| {
        fx.set_angle("direction", 0.0).unwrap();
        fx.set_float("relief", 1.0).unwrap();
        fx.set_float("contrast", 100.0).unwrap();
        fx.set_float("blend-with-original", 0.0).unwrap();
    });
    assert_eq!(at(&same, 1, 0), [255, 0, 0, 255]);
    let pixel = at(&gray, 1, 0);
    assert_eq!(pixel[0], pixel[1]);
    assert_eq!(pixel[1], pixel[2]);
}

#[test]
fn color_emboss_keeps_the_red() {
    let source = frame8(3, 1, &[[0, 0, 0, 255], [200, 20, 20, 255], [0, 0, 0, 255]]);
    let out = run("color-emboss", &source, |_| {});
    let pixel = at(&out, 1, 0);
    assert!(pixel[0] > pixel[1]);
}

#[test]
fn find_edges_invert_flips_the_background() {
    let source = frame8(
        5,
        1,
        &[
            [0, 0, 0, 255],
            [0, 0, 0, 255],
            [0, 0, 0, 255],
            [255, 255, 255, 255],
            [255, 255, 255, 255],
        ],
    );
    let off = run("find-edges", &source, |fx| {
        fx.set_checkbox("invert", false).unwrap();
        fx.set_float("blend-with-original", 0.0).unwrap();
    });
    let on = run("find-edges", &source, |fx| {
        fx.set_checkbox("invert", true).unwrap();
        fx.set_float("blend-with-original", 0.0).unwrap();
    });
    assert!(at(&off, 0, 0)[0] > at(&on, 0, 0)[0]);
}

#[test]
fn glow_spreads_a_bright_pixel() {
    let mut pixels = [[0, 0, 0, 255]; 9];
    pixels[4] = [255, 255, 255, 255];
    let source = frame8(3, 3, &pixels);
    let out = run("glow", &source, |fx| {
        fx.set_float("glow-threshold", 0.0).unwrap();
        fx.set_float("glow-radius", 1.0).unwrap();
        fx.set_float("glow-intensity", 2.0).unwrap();
    });
    assert!(at(&out, 0, 1)[0] > 0);
}

#[test]
fn mosaic_one_block_uses_the_average_when_sharp_is_off() {
    let source = frame8(
        2,
        2,
        &[
            [0, 0, 0, 255],
            [255, 255, 255, 255],
            [0, 0, 0, 255],
            [255, 255, 255, 255],
        ],
    );
    let out = run("mosaic", &source, |fx| {
        fx.set_float("horizontal-blocks", 1.0).unwrap();
        fx.set_float("vertical-blocks", 1.0).unwrap();
        fx.set_checkbox("sharp-colors", false).unwrap();
    });
    assert_eq!(at(&out, 0, 0)[0], at(&out, 1, 1)[0]);
}

#[test]
fn motion_tile_output_wider_than_the_source_repeats() {
    let source = frame8(2, 1, &[[255, 0, 0, 255], [0, 0, 255, 255]]);
    let out = run("motion-tile", &source, |fx| {
        fx.set_point("tile-center", [0.0, 0.0]).unwrap();
        fx.set_float("tile-width", 100.0).unwrap();
        fx.set_float("tile-height", 100.0).unwrap();
        fx.set_float("output-width", 200.0).unwrap();
        fx.set_float("output-height", 100.0).unwrap();
        fx.set_checkbox("mirror-edges", false).unwrap();
        fx.set_float("phase", 0.0).unwrap();
        fx.set_checkbox("horizontal-phase-shift", true).unwrap();
    });
    assert_eq!(out.width(), 4);
}

#[test]
fn roughen_edges_zero_influence_keeps_alpha() {
    let source = frame8(3, 1, &[[0, 0, 0, 0], [255, 0, 0, 255], [0, 0, 0, 0]]);
    let out = run("roughen-edges", &source, |fx| {
        fx.set_float("fractal-influence", 0.0).unwrap();
    });
    assert_eq!(at(&out, 1, 0)[3], 255);
    assert_eq!(at(&out, 0, 0)[3], 0);
}

#[test]
fn scatter_keeps_colors_and_horizontal_grain_keeps_the_row() {
    let source = frame8(
        3,
        2,
        &[
            [255, 0, 0, 255],
            [0, 0, 0, 255],
            [0, 0, 0, 255],
            [0, 255, 0, 255],
            [0, 0, 0, 255],
            [0, 0, 0, 255],
        ],
    );
    let out = run("scatter", &source, |fx| {
        fx.set_choice("grain", "水平").unwrap();
    });
    let mut reds = 0;
    for x in 0..3 {
        if at(&out, x, 0)[0] == 255 {
            reds += 1;
        }
        assert_ne!(at(&out, x, 1)[0], 255);
    }
    assert_eq!(reds, 1);
}

#[test]
fn strobe_light_makes_the_layer_transparent_during_the_pulse() {
    let source = solid8(2, 2, [10, 20, 30, 255]);
    let on = render_at("strobe-light", &source, Time::ZERO, |fx| {
        fx.set_color("strobe-color", Rgba::white()).unwrap();
        fx.set_float("blend-with-original", 0.0).unwrap();
        fx.set_float("strobe-duration", 0.2).unwrap();
        fx.set_float("strobe-period", 1.0).unwrap();
        fx.set_float("random-strobe-probability", 0.0).unwrap();
        fx.set_choice("strobe", "Makes Layer Transparent").unwrap();
    })
    .unwrap();
    let off = render_at("strobe-light", &source, Time::from_seconds(0.5), |fx| {
        fx.set_color("strobe-color", Rgba::white()).unwrap();
        fx.set_float("blend-with-original", 0.0).unwrap();
        fx.set_float("strobe-duration", 0.2).unwrap();
        fx.set_float("strobe-period", 1.0).unwrap();
        fx.set_float("random-strobe-probability", 0.0).unwrap();
        fx.set_choice("strobe", "Makes Layer Transparent").unwrap();
    })
    .unwrap();
    assert_eq!(at(&on, 0, 0)[3], 0);
    assert_eq!(at(&off, 0, 0)[3], 255);
}

#[test]
fn texturize_uses_the_texture_luminance() {
    let source = solid8(3, 1, [100, 100, 100, 255]);
    let texture = frame8(
        3,
        1,
        &[[0, 0, 0, 255], [255, 255, 255, 255], [0, 0, 0, 255]],
    );
    let out = run("texturize", &source, |fx| {
        fx.set_layer_frame("texture-layer", &texture).unwrap();
        fx.set_angle("light-direction", 0.0).unwrap();
        fx.set_float("texture-contrast", 1.0).unwrap();
        fx.set_choice("texture-placement", "Center Texture")
            .unwrap();
    });
    assert_ne!(at(&out, 0, 0)[0], at(&out, 1, 0)[0]);
}

#[test]
fn threshold_does_not_invent_a_cutoff_and_the_operation_uses_explicit_levels() {
    let source = solid8(1, 1, [100, 100, 100, 255]);
    let error = render("threshold", &source, |_| {}).unwrap_err();
    assert!(error.message.contains("未確認"));
    let low = threshold_at(&source, 44.0 / 255.0).unwrap();
    let mid = threshold_at(&source, 70.0 / 255.0).unwrap();
    let high = threshold_at(&source, 200.0 / 255.0).unwrap();
    assert_eq!(at(&low, 0, 0)[0], 255);
    assert_eq!(at(&mid, 0, 0)[0], 255);
    assert_eq!(at(&high, 0, 0)[0], 0);
}

#[test]
fn match_grain_adds_variation_from_a_noisy_source() {
    let target = solid8(4, 4, [80, 80, 80, 255]);
    let mut pixels = [[80, 80, 80, 255]; 16];
    for (index, pixel) in pixels.iter_mut().enumerate() {
        let v = if index % 2 == 0 { 20 } else { 200 };
        *pixel = [v, v, v, 255];
    }
    let noisy = frame8(4, 4, &pixels);
    let out = run("match-grain", &target, |fx| {
        fx.set_layer_frame("noise-source-layer", &noisy).unwrap();
        fx.set_float("compensate-for-existing-noise", 0.0).unwrap();
        fx.set_choice("viewing-mode", "Final Output").unwrap();
        fx.set_float("intensity", 1.0).unwrap();
        fx.set_float("size", 1.0).unwrap();
        fx.set_float("softness", 0.0).unwrap();
        fx.set_float("aspect-ratio", 1.0).unwrap();
        fx.set_checkbox("monochromatic", true).unwrap();
        fx.set_choice("blending-mode", "Add").unwrap();
        fx.set_float("random-seed", 1.0).unwrap();
        fx.set_float("animation-speed", 0.0).unwrap();
    });
    assert_ne!(at(&out, 0, 0), at(&target, 0, 0));
}

#[test]
fn turbulent_noise_can_replace_the_picture() {
    let source = solid8(4, 4, [40, 40, 40, 255]);
    let out = run("turbulent-noise", &source, |fx| {
        fx.set_float("opacity", 100.0).unwrap();
        fx.set_float("complexity", 2.0).unwrap();
        fx.set_float("sub-influence", 80.0).unwrap();
        fx.set_angle("evolution", 12.0).unwrap();
        fx.set_float("turbulence-factor", 0.0).unwrap();
    });
    assert_ne!(at(&out, 1, 1), [40, 40, 40, 255]);
}

#[test]
fn median_legacy_can_include_alpha() {
    let mut pixels = [[10, 10, 10, 255]; 9];
    pixels[4] = [10, 10, 10, 0];
    let source = frame8(3, 3, &pixels);
    let out = run("median-legacy", &source, |fx| {
        fx.set_float("radius", 1.0).unwrap();
        fx.set_checkbox("operate-on-alpha-channel", true).unwrap();
    });
    assert_eq!(at(&out, 1, 1)[3], 255);
    assert_eq!(at(&out, 1, 1)[0], 10);
}

#[test]
fn cc_glass_shades_a_slope() {
    let mut pixels = [[0; 4]; 12];
    for x in 0..4 {
        let v = (x * 80) as u8;
        for y in 0..3 {
            pixels[y * 4 + x] = [v, v, v, 255];
        }
    }
    let source = frame8(4, 3, &pixels);
    let out = run("cc-glass", &source, |fx| {
        fx.set_choice("property", "Luminance").unwrap();
        fx.set_float("height", 4.0).unwrap();
        fx.set_choice("light-type", "Distant Light").unwrap();
        fx.set_angle("light-direction", 180.0).unwrap();
    });
    assert_ne!(at(&out, 1, 1)[0], at(&source, 1, 1)[0]);
}

#[test]
fn cc_hex_tile_small_radius_changes_the_repeat() {
    let source = frame8(
        4,
        4,
        &[
            [255, 0, 0, 255],
            [0, 0, 255, 255],
            [255, 0, 0, 255],
            [0, 0, 255, 255],
            [0, 255, 0, 255],
            [0, 255, 0, 255],
            [0, 255, 0, 255],
            [0, 255, 0, 255],
            [255, 0, 0, 255],
            [0, 0, 255, 255],
            [255, 0, 0, 255],
            [0, 0, 255, 255],
            [0, 255, 0, 255],
            [0, 255, 0, 255],
            [0, 255, 0, 255],
            [0, 255, 0, 255],
        ],
    );
    let out = run("cc-hex-tile", &source, |fx| {
        fx.set_float("radius", 2.0).unwrap();
        fx.set_point("center", [1.0, 1.0]).unwrap();
        fx.set_angle("rotate", 0.0).unwrap();
    });
    assert_ne!(at(&out, 3, 3), at(&source, 3, 3));
}

#[test]
fn cc_kaleida_mirrors_an_asymmetric_source() {
    let mut pixels = [[0, 0, 0, 255]; 16];
    pixels[2 * 4 + 3] = [255, 0, 0, 255];
    let source = frame8(4, 4, &pixels);
    let out = run("cc-kaleida", &source, |fx| {
        fx.set_point("center", [2.0, 2.0]).unwrap();
        fx.set_float("size", 4.0).unwrap();
        fx.set_choice("mirroring", "Wheel").unwrap();
        fx.set_angle("rotation", 0.0).unwrap();
    });
    let mut reds = 0;
    for y in 0..4 {
        for x in 0..4 {
            if at(&out, x, y)[0] > 10 {
                reds += 1;
            }
        }
    }
    assert!(reds >= 2, "万華鏡が赤を繰り返さない");
}

#[test]
fn cc_mr_smoothie_maps_between_sample_colors() {
    let source = frame8(2, 1, &[[255, 0, 0, 255], [0, 0, 255, 255]]);
    let flow = frame8(2, 1, &[[0, 0, 0, 255], [255, 255, 255, 255]]);
    let out = run("cc-mr-smoothie", &source, |fx| {
        fx.set_layer_frame("flow-layer", &flow).unwrap();
        fx.set_point("sample-a", [0.0, 0.0]).unwrap();
        fx.set_point("sample-b", [1.0, 0.0]).unwrap();
        fx.set_angle("phase", 0.0).unwrap();
        fx.set_choice("color-loop", "AB").unwrap();
    });
    assert_eq!(at(&out, 0, 0)[0], 255);
    assert_eq!(at(&out, 1, 0)[2], 255);
}

#[test]
fn cc_plastic_obeys_the_cut_range() {
    let source = solid8(3, 3, [180, 180, 180, 255]);
    let out = run("cc-plastic", &source, |fx| {
        fx.set_choice("property", "Luminance").unwrap();
        fx.set_float("height", 2.0).unwrap();
        fx.set_float("cut-min", 0.0).unwrap();
        fx.set_float("cut-max", 0.1).unwrap();
        fx.set_choice("light-type", "Distant Light").unwrap();
        fx.set_angle("light-direction", 0.0).unwrap();
    });
    assert_eq!(at(&out, 1, 1), [180, 180, 180, 255]);
}

#[test]
fn cc_repeatile_grows_by_the_expand_amounts() {
    let source = frame8(
        2,
        2,
        &[
            [255, 0, 0, 255],
            [0, 0, 255, 255],
            [0, 255, 0, 255],
            [255, 255, 0, 255],
        ],
    );
    let out = run("cc-repeatile", &source, |fx| {
        fx.set_float("expand-right", 2.0).unwrap();
        fx.set_float("expand-left", 0.0).unwrap();
        fx.set_float("expand-down", 0.0).unwrap();
        fx.set_float("expand-up", 0.0).unwrap();
        fx.set_choice("tiling", "Repeat").unwrap();
    });
    assert_eq!(out.width(), 4);
    assert_eq!(at(&out, 2, 0), at(&out, 0, 0));
}

#[test]
fn published_popup_default_round_trips_through_param_values() {
    let list = effect("cc-mr-smoothie").unwrap().param_list();
    let values = defaults_from(&list);
    match values.get(ParamIndex::new(0)) {
        Some(ParamValue::Popup(index)) => {
            let Param::Popup { choices, .. } = list.get(ParamIndex::new(0)).unwrap() else {
                panic!("Property");
            };
            assert_eq!(choices[*index as usize], "Luminance");
        }
        other => panic!("unexpected {other:?}"),
    }
}
