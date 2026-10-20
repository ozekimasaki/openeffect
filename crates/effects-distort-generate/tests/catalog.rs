#[path = "common/mod.rs"]
mod common;

use effect_api::defaults_from;
use effects_distort_generate::{
    api_params, parameters, unconfirmed_note, EffectKind, Keyframe, Stack,
};
use model::BitsPerChannel;

use common::{run_bits, solid};

#[test]
fn every_effect_is_on_the_menu_and_renders_at_listed_depths() {
    assert_eq!(EffectKind::ALL.len(), 73);
    let mut names = Vec::new();
    for kind in EffectKind::ALL {
        names.push(kind.menu());
        let mut stack = Stack::new();
        let added = stack.add(&kind.menu()).expect("menu");
        assert_eq!(added.kind, kind);
        let _ = added;
        let frame = solid(8, 6, BitsPerChannel::Bpc8, [0.2, 0.3, 0.4, 1.0]);
        let fx = effects_distort_generate::Instance::new(kind);
        for bits in kind.bits() {
            let out = run_bits(&fx, &frame, *bits, 0.0).expect(kind.name());
            assert_eq!(out.bits(), *bits);
        }
        for bits in [
            BitsPerChannel::Bpc8,
            BitsPerChannel::Bpc16,
            BitsPerChannel::Bpc32,
        ] {
            if !kind.bits().contains(&bits) {
                assert!(run_bits(&fx, &frame, bits, 0.0).is_err(), "{}", kind.name());
            }
        }
        if kind.parameters_withheld() {
            assert!(parameters(kind).is_empty(), "{}", kind.name());
            assert!(api_params(kind).is_empty(), "{}", kind.name());
        }
    }
    names.sort();
    names.dedup();
    assert_eq!(names.len(), EffectKind::ALL.len());
}

#[test]
fn unconfirmed_defaults_are_not_published() {
    let allowed_numbers = ["Size", "Miter Limit", "Synthesis Edge Cropping"];
    let allowed_index = [
        "Interpolation Method",
        "Result",
        "Method",
        "Framing",
        "Objective",
        "Rolling Shutter Ripple",
    ];
    for kind in EffectKind::ALL {
        for param in parameters(kind) {
            if param.default_number.is_some() {
                assert!(
                    allowed_numbers.contains(&param.name.as_str()),
                    "{} {}",
                    kind.name(),
                    param.name
                );
            }
            if param.default_index.is_some()
                && param.kind == effects_distort_generate::ParamKind::Popup
            {
                assert!(
                    allowed_index.contains(&param.name.as_str()),
                    "{} {}",
                    kind.name(),
                    param.name
                );
            }
            assert!(
                param.default_bool.is_none(),
                "{} {}",
                kind.name(),
                param.name
            );
        }
    }
}

#[test]
fn effect_api_list_uses_only_fully_confirmed_contracts() {
    let scribble = api_params(EffectKind::Scribble);
    let values = defaults_from(&scribble);
    assert_eq!(scribble.len(), 1);
    assert!(matches!(
        values.get(effect_api::ParamIndex::new(0)),
        Some(effect_api::ParamValue::Float(value)) if (*value - 4.0).abs() < 1e-9
    ));
    let reshape = parameters(EffectKind::Reshape);
    let interp = reshape
        .iter()
        .find(|param| param.name == "Interpolation Method")
        .unwrap();
    assert_eq!(interp.default_index, Some(1));
    assert_eq!(interp.choices[1], "Linear");
    let stab = api_params(EffectKind::WarpStabilizer);
    let names: Vec<_> = stab
        .iter()
        .map(|(_, param)| param.name().to_string())
        .collect();
    assert!(names.contains(&"Result".to_string()));
    assert!(names.contains(&"Method".to_string()));
    assert!(names.contains(&"Framing".to_string()));
    assert!(names.contains(&"Objective".to_string()));
    assert!(!names.iter().any(|name| name == "Rolling Shutter Ripple"));
    assert!(!names.iter().any(|name| name == "Smoothness"));
}

#[test]
fn buttons_are_not_keyframeable_and_notes_mark_unknown_names() {
    let upscale = parameters(EffectKind::DetailPreservingUpscale);
    for name in ["Fit to Comp Width", "Fit to Comp Height"] {
        let param = upscale.iter().find(|param| param.name == name).unwrap();
        assert_eq!(param.keyframe, Keyframe::No);
    }
    assert_eq!(
        parameters(EffectKind::WarpStabilizer)
            .iter()
            .find(|param| param.name == "Analyze")
            .unwrap()
            .keyframe,
        Keyframe::No
    );
    assert_eq!(
        parameters(EffectKind::CameraTracker)
            .iter()
            .find(|param| param.name == "Create Camera")
            .unwrap()
            .keyframe,
        Keyframe::No
    );
    assert!(unconfirmed_note(EffectKind::BezierWarp)
        .unwrap()
        .contains("高品質"));
    assert!(unconfirmed_note(EffectKind::DisplacementMap).is_some());
    assert!(unconfirmed_note(EffectKind::Twirl)
        .unwrap()
        .contains("中心"));
    assert!(unconfirmed_note(EffectKind::Transform)
        .unwrap()
        .contains("シャッター"));
    let mesh = parameters(EffectKind::MeshWarp);
    assert_eq!(
        mesh.iter()
            .find(|param| param.name == "Distortion Mesh")
            .unwrap()
            .keyframe,
        Keyframe::Yes
    );
    assert!(parameters(EffectKind::BezierWarp).is_empty());
    assert!(parameters(EffectKind::CornerPin).is_empty());
}

#[test]
fn gpu_flag_follows_the_effect_list() {
    for kind in EffectKind::ALL {
        let listed = matches!(
            kind,
            EffectKind::Offset
                | EffectKind::Transform
                | EffectKind::Fill
                | EffectKind::GradientRamp
                | EffectKind::DropShadow
        );
        assert_eq!(kind.gpu_listed(), listed, "{}", kind.name());
    }
}
