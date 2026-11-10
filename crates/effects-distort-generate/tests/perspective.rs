#[path = "common/mod.rs"]
mod common;

use effects_distort_generate::{EffectKind, Instance, SourceLayer, TrackObs};
use model::{BitsPerChannel, LayerId, Rgba};

use common::{at, diff, paint, run, solid};

fn fx(kind: EffectKind) -> Instance {
    Instance::new(kind)
}

#[test]
fn camera_tracker_analyze_and_create() {
    let frame = solid(16, 12, BitsPerChannel::Bpc8, [0.2, 0.2, 0.2, 1.0]);
    let mut g = fx(EffectKind::CameraTracker);
    g.host.tracks = vec![
        TrackObs {
            time: 0.0,
            id: 1,
            xy: [2.0, 2.0],
        },
        TrackObs {
            time: 1.0,
            id: 1,
            xy: [5.0, 2.0],
        },
        TrackObs {
            time: 0.0,
            id: 2,
            xy: [8.0, 6.0],
        },
        TrackObs {
            time: 1.0,
            id: 2,
            xy: [11.0, 6.0],
        },
    ];
    assert!(g.press("Create Camera").is_err());
    g.press("Analyze").unwrap();
    g.finish_analysis();
    assert!(g.runtime.analysis.camera_ready);
    assert!(g.runtime.analysis.camera_path[1][0] < g.runtime.analysis.camera_path[0][0]);
    g.press("Create Camera").unwrap();
    assert!(g.runtime.analysis.camera_created);
    g.set_choice("Shot Type", "Variable Zoom").unwrap();
    g.set_number("Horizontal Angle of View", 40.0).unwrap();
    g.finish_analysis();
    assert!((g.runtime.analysis.camera_zoom.unwrap() - 40.0).abs() < 1e-6);
    g.set_choice("Shot Type", "Fixed Angle of View").unwrap();
    g.set_bool("Render Track Points", true).unwrap();
    g.set_number("Track Point Size", 1.0).unwrap();
    let drawn = run(&g, &frame, 0.0);
    assert!(at(&drawn, 2, 2)[0] > 0.8);
}

#[test]
fn glasses_views_swap_and_balance() {
    let base = solid(8, 8, BitsPerChannel::Bpc8, [0.0, 0.0, 0.0, 1.0]);
    let mut left = solid(8, 8, BitsPerChannel::Bpc8, [1.0, 0.0, 0.0, 1.0]);
    left = paint(&left, 1, 1, [0.0, 1.0, 0.0, 1.0]);
    let right = solid(8, 8, BitsPerChannel::Bpc8, [0.0, 0.0, 1.0, 1.0]);
    let mut g = fx(EffectKind::Glasses3d);
    let left_id = LayerId::new(1);
    let right_id = LayerId::new(2);
    g.host.layers.push(SourceLayer {
        id: left_id,
        frame: left,
        masks: vec![],
        audio: None,
        precomposed: false,
    });
    g.host.layers.push(SourceLayer {
        id: right_id,
        frame: right,
        masks: vec![],
        audio: None,
        precomposed: false,
    });
    g.set_layer("Left View", Some(left_id)).unwrap();
    g.set_layer("Right View", Some(right_id)).unwrap();
    g.set_choice("3D View", "Stereo Pair (Side by Side)")
        .unwrap();
    assert!(!g.enabled("Convergence Offset"));
    let pair = run(&g, &base, 0.0);
    assert!(at(&pair, 1, 4)[0] > at(&pair, 6, 4)[0]);
    g.set_bool("Swap Left-Right", true).unwrap();
    let swapped = run(&g, &base, 0.0);
    assert!(diff(&pair, &swapped) > 0.2);
    g.set_bool("Swap Left-Right", false).unwrap();
    g.set_choice("3D View", "Over Under").unwrap();
    let over = run(&g, &base, 0.0);
    assert!(diff(&pair, &over) > 0.1);
    g.set_choice("3D View", "Red Blue LR").unwrap();
    assert!(g.enabled("Convergence Offset"));
    g.set_number("Convergence Offset", 1.0).unwrap();
    g.set_number("Vertical Alignment", 0.0).unwrap();
    g.set_choice("Units", "Pixels").unwrap();
    let anaglyph = run(&g, &base, 0.0);
    g.set_choice("Units", "% of Source").unwrap();
    g.set_number("Convergence Offset", 20.0).unwrap();
    let percent = run(&g, &base, 0.0);
    assert!(diff(&anaglyph, &percent) > 0.0);
    g.set_choice("3D View", "Balanced Colored Red Blue")
        .unwrap();
    assert!(g.enabled("Balance"));
    g.set_number("Balance", 0.0).unwrap();
    let flat = run(&g, &base, 0.0);
    g.set_number("Balance", 1.5).unwrap();
    let hot = run(&g, &base, 0.0);
    assert!(diff(&flat, &hot) > 0.05);
    g.set_choice("3D View", "Interlace Upper L Lower R")
        .unwrap();
    let interlace = run(&g, &base, 0.0);
    assert!(diff(&flat, &interlace) > 0.0);
}

#[test]
fn bevels_are_softer_on_alpha_than_edges() {
    let mut frame = solid(12, 12, BitsPerChannel::Bpc8, [0.0, 0.0, 0.0, 0.0]);
    for y in 3..9 {
        for x in 3..9 {
            frame = paint(&frame, x, y, [0.6, 0.6, 0.6, 1.0]);
        }
    }
    let alpha = run(&fx(EffectKind::BevelAlpha), &frame, 0.0);
    let edges = run(&fx(EffectKind::BevelEdges), &frame, 0.0);
    assert!(diff(&frame, &alpha) > 0.05);
    assert!(diff(&frame, &edges) > 0.05);
    assert!(effects_distort_generate::parameters(EffectKind::BevelAlpha).is_empty());
    assert!(effects_distort_generate::parameters(EffectKind::BevelEdges).is_empty());
    let changed = |image: &effect_api::Frame| view_count(image, &frame);
    assert!(changed(&alpha) >= changed(&edges));
}

fn view_count(image: &effect_api::Frame, original: &effect_api::Frame) -> usize {
    common::view(image)
        .px
        .iter()
        .zip(common::view(original).px.iter())
        .filter(|(a, b)| (a[0] - b[0]).abs() > 0.02)
        .count()
}

#[test]
fn drop_shadow_and_radial_shadow() {
    let mut frame = solid(16, 16, BitsPerChannel::Bpc8, [0.0, 0.0, 0.0, 0.0]);
    for y in 2..6 {
        for x in 2..6 {
            frame = paint(&frame, x, y, [1.0, 0.2, 0.2, 1.0]);
        }
    }
    let mut g = fx(EffectKind::DropShadow);
    g.set_bool("Shadow Only", false).unwrap();
    let both = run(&g, &frame, 0.0);
    assert!(at(&both, 6, 6)[3] > 0.1);
    g.set_bool("Shadow Only", true).unwrap();
    let only = run(&g, &frame, 0.0);
    assert!(at(&only, 3, 3)[0] < at(&both, 3, 3)[0]);
    assert!(at(&only, 6, 6)[3] > 0.05);
    let mut radial = fx(EffectKind::RadialShadow);
    radial
        .set_color("Shadow Color", Rgba::new(0.0, 0.0, 0.0, 1.0))
        .unwrap();
    radial.set_number("Opacity", 1.0).unwrap();
    radial.set_point("Light Source", 0.0, 0.0).unwrap();
    radial.set_number("Projection Distance", 6.0).unwrap();
    radial.set_number("Softness", 0.0).unwrap();
    radial.set_choice("Render", "Regular").unwrap();
    radial.set_bool("Shadow Only", true).unwrap();
    radial.set_bool("Resize Layer", false).unwrap();
    let down = run(&radial, &frame, 0.0);
    radial.set_point("Light Source", 16.0, 16.0).unwrap();
    let up = run(&radial, &frame, 0.0);
    assert!(diff(&down, &up) > 0.05);
    radial.set_choice("Render", "Glass Edge").unwrap();
    radial.set_number("Color Influence", 100.0).unwrap();
    let glass = run(&radial, &frame, 0.0);
    radial.set_number("Color Influence", 0.0).unwrap();
    let plain = run(&radial, &frame, 0.0);
    assert!(diff(&glass, &plain) > 0.0);
    radial.set_bool("Resize Layer", true).unwrap();
    radial.set_number("Projection Distance", 8.0).unwrap();
    let grown = run(&radial, &frame, 0.0);
    assert!(grown.width() > frame.width());
    radial.set_number("Softness", 3.0).unwrap();
    let soft = run(&radial, &frame, 0.0);
    assert!(diff(&grown, &soft) > 0.0);
}
