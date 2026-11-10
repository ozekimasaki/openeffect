#[path = "common/mod.rs"]
mod common;

use effects_distort_generate::{
    fallback_method, polar_pair, wave_dir, wave_sample, EffectKind, Instance, LiquifyStroke,
    LiquifyTool, MaskShape, MotionSample, Quality, Value,
};
use model::{BitsPerChannel, LayerId};

use common::{at, diff, draft, paint, run, solid, view};

fn kind(kind: EffectKind) -> Instance {
    Instance::new(kind)
}

#[test]
fn bezier_edges_bend_the_picture() {
    let mut base = solid(12, 12, BitsPerChannel::Bpc8, [0.0, 0.0, 0.0, 1.0]);
    for x in 0..12 {
        base = paint(&base, x, 4, [x as f32 / 11.0, 0.0, 0.0, 1.0]);
    }
    let mut fx = kind(EffectKind::BezierWarp);
    let w = 11.0;
    let h = 11.0;
    fx.runtime.edges = vec![
        ([w / 2.0, 0.0], [w * 0.25, 0.0], [w * 0.75, 0.0]),
        ([w, h / 2.0], [w, h * 0.25], [w, h * 0.75]),
        ([w / 2.0, h], [w * 0.75, h], [w * 0.25, h]),
        ([0.0, h / 2.0], [0.0, h * 0.75], [0.0, h * 0.25]),
    ];
    let straight = run(&fx, &base, 0.0);
    fx.runtime.edges[0].0[1] = 6.0;
    let bent = run(&fx, &base, 0.0);
    assert!(diff(&straight, &bent) > 0.05);
    fx.runtime.edges[0].1 = [2.0, 9.0];
    let tangent = run(&fx, &base, 0.0);
    assert!(diff(&bent, &tangent) > 0.01);
}

#[test]
fn bulge_height_radii_taper_pin_and_antialias() {
    let mut frame = solid(16, 16, BitsPerChannel::Bpc8, [0.0, 0.0, 0.0, 1.0]);
    for y in 6..10 {
        for x in 6..10 {
            frame = paint(&frame, x, y, [1.0, 1.0, 1.0, 1.0]);
        }
    }
    let mut fx = kind(EffectKind::Bulge);
    fx.set_number("Horizontal Radius", 12.0).unwrap();
    fx.set_number("Vertical Radius", 12.0).unwrap();
    fx.set_number("Taper Radius", 0.0).unwrap();
    fx.set_number("Bulge Height", 1.0).unwrap();
    let positive = run(&fx, &frame, 0.0);
    fx.set_number("Bulge Height", -0.8).unwrap();
    let negative = run(&fx, &frame, 0.0);
    assert!(at(&positive, 10, 8)[0] > at(&negative, 10, 8)[0]);
    fx.set_number("Bulge Height", 1.0).unwrap();
    fx.set_number("Vertical Radius", 3.0).unwrap();
    let short = run(&fx, &frame, 0.0);
    assert!(diff(&positive, &short) > 0.05);
    fx.set_number("Vertical Radius", 8.0).unwrap();
    fx.set_number("Taper Radius", 4.0).unwrap();
    let smooth = run(&fx, &frame, 0.0);
    assert!(diff(&positive, &smooth) > 0.02);
    fx.set_number("Taper Radius", 0.0).unwrap();
    fx.set_number("Horizontal Radius", 20.0).unwrap();
    fx.set_number("Vertical Radius", 20.0).unwrap();
    fx.set_number("Bulge Height", 8.0).unwrap();
    fx.set_bool("Pin All Edges", false).unwrap();
    let open = run(&fx, &frame, 0.0);
    fx.set_bool("Pin All Edges", true).unwrap();
    let pinned = run(&fx, &frame, 0.0);
    assert!(diff(&open, &pinned) > 0.01);
    assert!((at(&pinned, 0, 8)[0] - at(&frame, 0, 8)[0]).abs() < 0.05);
    fx.set_bool("Pin All Edges", false).unwrap();
    fx.set_number("Antialiasing", 1.0).unwrap();
    fx.host.quality = Quality::Best;
    let best = run(&fx, &frame, 0.0);
    fx.host.quality = Quality::Draft;
    let draft_hi = run(&fx, &frame, 0.0);
    fx.set_number("Antialiasing", 0.0).unwrap();
    let draft_lo = run(&fx, &frame, 0.0);
    assert!(diff(&best, &draft_hi) > 0.01);
    assert!(diff(&draft_hi, &draft_lo) < 0.02);
}

#[test]
fn corner_pin_moves_each_corner() {
    let frame = paint(
        &solid(8, 8, BitsPerChannel::Bpc16, [0.0, 0.0, 0.0, 1.0]),
        1,
        1,
        [1.0, 0.0, 0.0, 1.0],
    );
    let mut fx = kind(EffectKind::CornerPin);
    fx.runtime.pins = vec![[0.0, 0.0], [7.0, 0.0], [7.0, 7.0], [0.0, 7.0]];
    let a = run(&fx, &frame, 0.0);
    fx.runtime.pins[2] = [5.0, 5.0];
    let b = run(&fx, &frame, 0.0);
    assert!(diff(&a, &b) > 0.05);
    let out32 = common::run_bits(&fx, &frame, BitsPerChannel::Bpc32, 0.0).unwrap();
    assert_eq!(out32.bits(), BitsPerChannel::Bpc32);
}

#[test]
fn upscale_scale_fit_noise_detail_and_alpha() {
    let mut frame = solid(8, 8, BitsPerChannel::Bpc8, [0.2, 0.2, 0.2, 1.0]);
    frame = paint(&frame, 3, 3, [1.0, 0.0, 0.0, 1.0]);
    frame = paint(&frame, 4, 4, [0.0, 0.0, 0.0, 0.0]);
    let mut fx = kind(EffectKind::DetailPreservingUpscale);
    fx.host.comp_width = 16;
    fx.host.comp_height = 24;
    fx.set_number("Scale", 2.0).unwrap();
    let twice = run(&fx, &frame, 0.0);
    fx.set_number("Scale", 3.0).unwrap();
    let thrice = run(&fx, &frame, 0.0);
    assert_eq!(twice.width(), 16);
    assert_eq!(thrice.width(), 24);
    fx.press("Fit to Comp Width").unwrap();
    let fit_w = run(&fx, &frame, 0.0);
    assert_eq!(fit_w.width(), 16);
    fx.press("Fit to Comp Height").unwrap();
    let fit_h = run(&fx, &frame, 0.0);
    assert_eq!(fit_h.height(), 24);
    let noisy = solid(8, 8, BitsPerChannel::Bpc8, [0.5, 0.5, 0.5, 1.0]);
    let mut noisy = noisy;
    for i in 0..8 {
        noisy = paint(&noisy, i, i % 3, [1.0, 1.0, 1.0, 1.0]);
    }
    let mut fx = kind(EffectKind::DetailPreservingUpscale);
    fx.set_number("Scale", 2.0).unwrap();
    fx.set_number("Reduce Noise", 0.0).unwrap();
    let grain = run(&fx, &noisy, 0.0);
    fx.set_number("Reduce Noise", 2.0).unwrap();
    let clean = run(&fx, &noisy, 0.0);
    assert!(variance(&grain) > variance(&clean));
    fx.set_number("Reduce Noise", 0.0).unwrap();
    fx.set_number("Detail", 0.0).unwrap();
    let soft = run(&fx, &frame, 0.0);
    fx.set_number("Detail", 2.0).unwrap();
    let sharp = run(&fx, &frame, 0.0);
    assert!(diff(&soft, &sharp) > 0.01);
    fx.set_choice("Alpha", "Bilinear").unwrap();
    let bilinear = run(&fx, &frame, 0.0);
    fx.set_choice("Alpha", "Bicubic").unwrap();
    let bicubic = run(&fx, &frame, 0.0);
    assert!(soft_alpha(&bilinear) >= soft_alpha(&bicubic));
}

fn variance(frame: &effect_api::Frame) -> f32 {
    let image = view(frame);
    let mean = image.px.iter().map(|px| px[0]).sum::<f32>() / image.px.len() as f32;
    image
        .px
        .iter()
        .map(|px| (px[0] - mean).powi(2))
        .sum::<f32>()
        / image.px.len() as f32
}

fn soft_alpha(frame: &effect_api::Frame) -> usize {
    view(frame)
        .px
        .iter()
        .filter(|px| px[3] > 0.15 && px[3] < 0.85)
        .count()
}

#[test]
fn displacement_mapping_behavior_wrap_and_expand() {
    let mut src = solid(8, 8, BitsPerChannel::Bpc8, [0.0, 0.0, 0.0, 1.0]);
    src = paint(&src, 3, 3, [1.0, 0.0, 0.0, 1.0]);
    let map_black = solid(8, 8, BitsPerChannel::Bpc8, [0.0, 0.0, 0.0, 1.0]);
    let map_mid = solid(8, 8, BitsPerChannel::Bpc8, [128.0 / 255.0, 0.0, 0.0, 1.0]);
    let map_white = solid(8, 8, BitsPerChannel::Bpc8, [1.0, 0.0, 0.0, 1.0]);
    let id = LayerId::new(7);
    let mut fx = kind(EffectKind::DisplacementMap);
    fx.set_layer("Displacement Map Layer", Some(id)).unwrap();
    fx.set_role("max_x", Value::Number(2.0));
    fx.set_role("max_y", Value::Number(0.0));
    fx.set_role("channel_x", Value::Index(0));
    fx.set_role("channel_y", Value::Index(0));
    fx.set_role("behavior", Value::Index(1));
    fx.host.layers.push(effects_distort_generate::SourceLayer {
        id,
        frame: map_mid.clone(),
        masks: vec![MaskShape::polygon(
            "hide",
            vec![[0.0, 0.0], [2.0, 0.0], [2.0, 8.0], [0.0, 8.0]],
            0,
        )],
        audio: None,
        precomposed: false,
    });
    let mid = run(&fx, &src, 0.0);
    assert!((at(&mid, 3, 3)[0] - 1.0).abs() < 0.2);
    fx.host.layers[0].frame = map_white;
    let positive = run(&fx, &src, 0.0);
    assert!(at(&positive, 5, 3)[0] > at(&positive, 3, 3)[0]);
    fx.host.layers[0].frame = map_black;
    let negative = run(&fx, &src, 0.0);
    assert!(at(&negative, 1, 3)[0] > at(&negative, 3, 3)[0]);
    fx.set_bool("Wrap Pixels Around", true).unwrap();
    let wrapped = run(&fx, &src, 0.0);
    assert_ne!(wrapped.width(), 0);
    fx.set_bool("Expand Output", true).unwrap();
    let expanded = run(&fx, &src, 0.0);
    assert!(expanded.width() > src.width());
    let small = solid(2, 2, BitsPerChannel::Bpc8, [1.0, 0.0, 0.0, 1.0]);
    let wide = src.clone();
    fx.host.layers[0].frame = small;
    fx.set_bool("Expand Output", false).unwrap();
    fx.set_role("behavior", Value::Index(0));
    let center = run(&fx, &wide, 0.0);
    fx.set_role("behavior", Value::Index(2));
    let tile = run(&fx, &wide, 0.0);
    fx.set_role("behavior", Value::Index(1));
    let stretch = run(&fx, &wide, 0.0);
    assert!(diff(&center, &tile) > 0.01 || diff(&tile, &stretch) > 0.01);
    let _ = map_mid;
}

#[test]
fn liquify_tools_mask_and_reconstruction() {
    let frame = paint(
        &solid(16, 16, BitsPerChannel::Bpc8, [0.0, 0.0, 0.0, 1.0]),
        8,
        8,
        [1.0, 0.0, 0.0, 1.0],
    );
    let mut fx = kind(EffectKind::Liquify);
    fx.set_role("brush_size", Value::Number(6.0));
    fx.set_role("brush_pressure", Value::Number(1.0));
    fx.set_number("Distortion Percentage", 100.0).unwrap();
    fx.runtime.strokes.push(LiquifyStroke {
        tool: LiquifyTool::Warp,
        from: [8.0, 8.0],
        to: [11.0, 8.0],
        hold: 1.0,
    });
    let warped = run(&fx, &frame, 0.0);
    assert!(diff(&frame, &warped) > 0.05);
    fx.set_number("Distortion Percentage", 10.0).unwrap();
    let weak = run(&fx, &frame, 0.0);
    assert!(diff(&warped, &weak) > 0.01);
    fx.set_number("Distortion Percentage", 100.0).unwrap();
    fx.host.masks.push(MaskShape::polygon(
        "freeze",
        vec![[6.0, 6.0], [10.0, 6.0], [10.0, 10.0], [6.0, 10.0]],
        0,
    ));
    fx.set_number("Mask Opacity", 100.0).unwrap();
    let frozen = run(&fx, &frame, 0.0);
    fx.set_number("Mask Opacity", 50.0).unwrap();
    let half = run(&fx, &frame, 0.0);
    assert!(diff(&frozen, &half) > 0.01);
    fx.host.masks.clear();
    fx.set_number("Mask Feather", 2.0).unwrap();
    let _ = run(&fx, &frame, 0.0);
    for (tool, name) in [
        (LiquifyTool::Pucker, "pucker"),
        (LiquifyTool::Bloat, "bloat"),
        (LiquifyTool::TwirlClockwise, "cw"),
        (LiquifyTool::TwirlCounterclockwise, "ccw"),
        (LiquifyTool::ShiftPixels, "shift"),
        (LiquifyTool::Turbulence, "turb"),
        (LiquifyTool::Reflection, "reflect"),
        (LiquifyTool::Clone, "clone"),
    ] {
        let mut tool_fx = kind(EffectKind::Liquify);
        tool_fx.set_role("brush_size", Value::Number(6.0));
        tool_fx.set_role("brush_pressure", Value::Number(1.0));
        tool_fx.set_number("Distortion Percentage", 100.0).unwrap();
        if tool == LiquifyTool::Turbulence {
            tool_fx.set_role("turbulent_jitter", Value::Number(1.5));
        }
        tool_fx.runtime.strokes.push(LiquifyStroke {
            tool,
            from: [4.0, 8.0],
            to: [10.0, 9.0],
            hold: 1.0,
        });
        let out = run(&tool_fx, &frame, 0.0);
        assert!(diff(&frame, &out) > 0.0, "{name}");
    }
    fx.runtime.strokes.push(LiquifyStroke {
        tool: LiquifyTool::Reconstruction,
        from: [8.0, 8.0],
        to: [8.0, 8.0],
        hold: 1.0,
    });
    fx.set_choice("Reconstruction Mode", "Revert").unwrap();
    let revert = run(&fx, &frame, 0.0);
    fx.set_choice("Reconstruction Mode", "Displace").unwrap();
    let displace = run(&fx, &frame, 0.0);
    fx.set_choice("Reconstruction Mode", "Amplitwist").unwrap();
    let ampli = run(&fx, &frame, 0.0);
    fx.set_choice("Reconstruction Mode", "Affine").unwrap();
    let affine = run(&fx, &frame, 0.0);
    assert!(diff(&revert, &displace) > 0.0 || diff(&ampli, &affine) > 0.0);
    fx.set_role("view_mesh", Value::Bool(true));
    let mesh = run(&fx, &frame, 0.0);
    assert!(diff(&affine, &mesh) > 0.0);
}

#[test]
fn magnify_link_scaling_and_resize() {
    let mut frame = solid(32, 32, BitsPerChannel::Bpc8, [0.1, 0.1, 0.1, 1.0]);
    frame = paint(&frame, 16, 16, [1.0, 0.0, 0.0, 1.0]);
    let mut fx = kind(EffectKind::Magnify);
    fx.set_point("Center", 16.0, 16.0).unwrap();
    fx.set_number("Magnification", 200.0).unwrap();
    fx.set_number("Size", 6.0).unwrap();
    fx.set_number("Feather", 2.0).unwrap();
    fx.set_choice("Link", "None").unwrap();
    fx.set_number("Opacity", 100.0).unwrap();
    fx.set_role("shape", Value::Index(0));
    fx.set_role("blend_none", Value::Index(1));
    let none = run(&fx, &frame, 0.0);
    assert!(at(&none, 0, 0)[3] < 0.1);
    assert!(fx.enabled("Resize Layer"));
    fx.set_choice("Link", "Size To Magnification").unwrap();
    assert!(!fx.enabled("Resize Layer"));
    let linked = run(&fx, &frame, 0.0);
    fx.set_choice("Link", "Size & Feather To Magnification")
        .unwrap();
    let feathered = run(&fx, &frame, 0.0);
    assert!(diff(&linked, &feathered) > 0.01 || diff(&none, &linked) > 0.01);
    fx.set_choice("Link", "None").unwrap();
    fx.set_choice("Scaling", "Standard").unwrap();
    let standard = run(&fx, &frame, 0.0);
    fx.set_choice("Scaling", "Soft").unwrap();
    let soft = run(&fx, &frame, 0.0);
    fx.set_choice("Scaling", "Scatter").unwrap();
    let scatter = run(&fx, &frame, 0.0);
    assert!(diff(&standard, &soft) > 0.0 || diff(&soft, &scatter) > 0.0);
    fx.set_role("shape", Value::Index(1));
    let square = run(&fx, &frame, 0.0);
    fx.set_role("shape", Value::Index(0));
    let round = run(&fx, &frame, 0.0);
    assert!(diff(&square, &round) > 0.0);
    fx.set_bool("Resize Layer", true).unwrap();
    fx.set_number("Size", 20.0).unwrap();
    let grown = run(&fx, &frame, 0.0);
    assert!(grown.width() >= frame.width());
}

#[test]
fn mesh_rows_columns_and_vertex() {
    let frame = paint(
        &solid(16, 16, BitsPerChannel::Bpc8, [0.2, 0.2, 0.2, 1.0]),
        4,
        4,
        [1.0, 0.0, 0.0, 1.0],
    );
    let mut fx = kind(EffectKind::MeshWarp);
    fx.set_number("Rows", 2.0).unwrap();
    fx.set_number("Columns", 2.0).unwrap();
    fx.set_number("Quality", 1.0).unwrap();
    let coarse = run(&fx, &frame, 0.0);
    fx.set_number("Rows", 4.0).unwrap();
    fx.set_number("Columns", 4.0).unwrap();
    let fine = run(&fx, &frame, 0.0);
    fx.runtime.mesh.push(effects_distort_generate::MeshPoint {
        col: 1,
        row: 1,
        at: [10.0, 4.0],
        selected: true,
    });
    fx.set_number("Quality", 8.0).unwrap();
    fx.set_role("elasticity", Value::Number(1.0));
    let moved = run(&fx, &frame, 0.0);
    assert!(diff(&coarse, &moved) > 0.0 || diff(&fine, &moved) > 0.0);
    assert!(fx.set_number("Rows", 40.0).is_ok());
    let clamped = run(&fx, &frame, 0.0);
    assert_eq!(clamped.width(), frame.width());
}

#[test]
fn mirror_angle_and_center() {
    let mut frame = solid(8, 8, BitsPerChannel::Bpc8, [0.0, 0.0, 0.0, 1.0]);
    frame = paint(&frame, 0, 0, [1.0, 0.0, 0.0, 1.0]);
    let mut fx = kind(EffectKind::Mirror);
    fx.set_point("Reflection Center", 3.5, 3.5).unwrap();
    fx.set_number("Reflection Angle", 0.0).unwrap();
    let horizontal = run(&fx, &frame, 0.0);
    assert!(at(&horizontal, 7, 0)[0] > 0.5);
    fx.set_number("Reflection Angle", 90.0).unwrap();
    let vertical = run(&fx, &frame, 0.0);
    assert!(at(&vertical, 0, 7)[0] > 0.5);
    assert!(diff(&horizontal, &vertical) > 0.2);
}

#[test]
fn offset_wraps_and_blends_subpixel_on_best() {
    let frame = paint(
        &solid(8, 8, BitsPerChannel::Bpc8, [0.0, 0.0, 0.0, 1.0]),
        0,
        0,
        [1.0, 0.0, 0.0, 1.0],
    );
    let mut fx = kind(EffectKind::Offset);
    fx.set_point("Shift Center To", 6.0, 4.0).unwrap();
    fx.set_number("Blend With Original", 0.0).unwrap();
    fx.host.quality = Quality::Best;
    let shifted = run(&fx, &frame, 0.0);
    assert!(at(&shifted, 2, 0)[0] > 0.5);
    fx.set_number("Blend With Original", 100.0).unwrap();
    let original = run(&fx, &frame, 0.0);
    assert!(diff(&original, &frame) < 0.05);
    fx.set_number("Blend With Original", 0.0).unwrap();
    fx.set_point("Shift Center To", 4.5, 4.0).unwrap();
    let best = run(&fx, &frame, 0.0);
    draft(&mut fx);
    let draft = run(&fx, &frame, 0.0);
    assert!(diff(&best, &draft) > 0.0);
}

#[test]
fn optics_fov_reverse_resize_and_optimal() {
    let mut frame = solid(24, 12, BitsPerChannel::Bpc8, [0.0, 0.0, 0.0, 1.0]);
    for x in 0..24 {
        frame = paint(&frame, x, 6, [1.0, 0.0, 0.0, 1.0]);
    }
    let mut fx = kind(EffectKind::OpticsCompensation);
    fx.set_number("Field Of View (FOV)", 40.0).unwrap();
    fx.set_bool("Reverse Lens Distortion", false).unwrap();
    let bent = run(&fx, &frame, 0.0);
    assert!(diff(&frame, &bent) > 0.05);
    fx.set_bool("Reverse Lens Distortion", true).unwrap();
    assert!(fx.enabled("Resize"));
    fx.set_choice("Resize", "Off").unwrap();
    let back = run(&fx, &bent, 0.0);
    assert!(diff(&frame, &back) < diff(&frame, &bent));
    fx.set_bool("Optimal Pixels", true).unwrap();
    let optimal = run(&fx, &bent, 0.0);
    assert!(diff(&back, &optimal) > 0.01);
    fx.set_bool("Optimal Pixels", false).unwrap();
    fx.set_choice("Resize", "Max 2X").unwrap();
    let x2 = run(&fx, &frame, 0.0);
    assert!(x2.width() <= frame.width() * 2 + 1);
    assert!(x2.height() <= frame.height() * 2 + 1);
    fx.set_choice("Resize", "Max 4X").unwrap();
    let x4 = run(&fx, &frame, 0.0);
    assert!(x4.width() <= frame.width() * 4 + 1);
    fx.set_choice("Resize", "Unlimited").unwrap();
    let unlimited = run(&fx, &frame, 0.0);
    assert!(unlimited.width() >= frame.width());
    fx.set_choice("Resize", "Off").unwrap();
    fx.set_point("View Center", 4.0, 6.0).unwrap();
    let offcenter = run(&fx, &frame, 0.0);
    fx.set_point("View Center", 12.0, 6.0).unwrap();
    let center = run(&fx, &frame, 0.0);
    assert!(diff(&offcenter, &center) > 0.01);
    fx.set_role("fov_axis", Value::Index(0));
    let horizontal = run(&fx, &frame, 0.0);
    fx.set_role("fov_axis", Value::Index(1));
    let vertical = run(&fx, &frame, 0.0);
    assert!(diff(&horizontal, &vertical) > 0.01);
    assert!(!kind(EffectKind::OpticsCompensation).enabled("Resize"));
}

#[test]
fn polar_interpolation_and_coordinate_pair() {
    assert_eq!(polar_pair(10.0, 45.0), [10.0, 45.0]);
    let mut frame = solid(16, 16, BitsPerChannel::Bpc32, [0.0, 0.0, 0.0, 1.0]);
    for x in 0..16 {
        frame = paint(&frame, x, 4, [1.0, 0.0, 0.0, 1.0]);
    }
    let mut fx = kind(EffectKind::PolarCoordinates);
    fx.set_number("Interpolation", 0.0).unwrap();
    fx.set_choice("Type of Conversion", "Rect To Polar")
        .unwrap();
    let same = run(&fx, &frame, 0.0);
    assert!(diff(&frame, &same) < 0.05);
    fx.set_number("Interpolation", 100.0).unwrap();
    let polar = run(&fx, &frame, 0.0);
    assert!(diff(&frame, &polar) > 0.2);
    fx.set_choice("Type of Conversion", "Polar To Rect")
        .unwrap();
    let rect = run(&fx, &frame, 0.0);
    assert!(diff(&polar, &rect) > 0.1);
}

#[test]
fn reshape_masks_percent_elasticity_and_interpolation() {
    let frame = paint(
        &solid(16, 16, BitsPerChannel::Bpc8, [0.2, 0.2, 0.2, 1.0]),
        4,
        8,
        [1.0, 0.0, 0.0, 1.0],
    );
    let mut fx = kind(EffectKind::Reshape);
    fx.host.masks = vec![
        MaskShape::polygon(
            "boundary",
            vec![[1.0, 1.0], [14.0, 1.0], [14.0, 14.0], [1.0, 14.0]],
            0,
        ),
        MaskShape::polygon(
            "source",
            vec![[3.0, 6.0], [6.0, 6.0], [6.0, 10.0], [3.0, 10.0]],
            1,
        ),
        MaskShape::polygon(
            "dest",
            vec![[9.0, 6.0], [12.0, 6.0], [12.0, 10.0], [9.0, 10.0]],
            2,
        ),
    ];
    fx.set_number("Percent", 0.0).unwrap();
    let start = run(&fx, &frame, 0.0);
    fx.set_number("Percent", 100.0).unwrap();
    let end = run(&fx, &frame, 0.0);
    assert!(diff(&start, &end) > 0.01);
    fx.set_choice("Elasticity", "Stiff").unwrap();
    let stiff = run(&fx, &frame, 0.0);
    fx.set_choice("Elasticity", "Super Fluid").unwrap();
    let fluid = run(&fx, &frame, 0.0);
    assert!(diff(&stiff, &fluid) > 0.0);
    assert_eq!(fx.choice("Interpolation Method").as_deref(), Some("Linear"));
    fx.add_key("Percent", 0.0, Value::Number(0.0));
    fx.add_key("Percent", 1.0, Value::Number(100.0));
    fx.set_choice("Interpolation Method", "Linear").unwrap();
    let mid = run(&fx, &frame, 0.5);
    fx.set_choice("Interpolation Method", "Discrete").unwrap();
    let discrete = run(&fx, &frame, 0.5);
    fx.add_key("Percent", 2.0, Value::Number(0.0));
    fx.set_choice("Interpolation Method", "Smooth").unwrap();
    let smooth = run(&fx, &frame, 0.5);
    assert!(diff(&mid, &discrete) > 0.0 || diff(&mid, &smooth) > 0.0);
    let outside = at(&end, 0, 0);
    assert!((outside[0] - 0.2).abs() < 0.15);
}

#[test]
fn ripple_radius_speed_phase_and_type() {
    let frame = paint(
        &solid(16, 16, BitsPerChannel::Bpc8, [0.0, 0.0, 1.0, 1.0]),
        8,
        8,
        [1.0, 0.0, 0.0, 1.0],
    );
    let mut fx = kind(EffectKind::Ripple);
    fx.set_point("Center Of Ripple", 8.0, 8.0).unwrap();
    fx.set_number("Radius", 0.0).unwrap();
    fx.set_number("Wave Width", 4.0).unwrap();
    fx.set_number("Wave Height", 4.0).unwrap();
    let still = run(&fx, &frame, 0.0);
    assert!(diff(&frame, &still) < 0.05);
    fx.set_number("Radius", 100.0).unwrap();
    fx.set_choice("Type Of Conversion", "Symmetric").unwrap();
    let reach = run(&fx, &frame, 0.0);
    assert!(diff(&frame, &reach) > 0.05);
    fx.set_choice("Type Of Conversion", "Asymmetric").unwrap();
    let asym = run(&fx, &frame, 0.0);
    assert!(diff(&reach, &asym) > 0.01);
    fx.set_number("Wave Speed", 1.0).unwrap();
    let later = run(&fx, &frame, 0.5);
    assert!(diff(&asym, &later) > 0.01);
    fx.set_number("Wave Speed", 0.0).unwrap();
    let stopped = run(&fx, &frame, 0.5);
    let stopped0 = run(&fx, &frame, 0.0);
    assert!(diff(&stopped, &stopped0) < 0.05);
    fx.set_number("Ripple Phase", 90.0).unwrap();
    let phase = run(&fx, &frame, 0.0);
    assert!(diff(&stopped0, &phase) > 0.01);
    assert_eq!(
        parameters_key(EffectKind::Ripple, "Ripple Phase"),
        effects_distort_generate::Keyframe::Yes
    );
}

fn parameters_key(kind: EffectKind, name: &str) -> effects_distort_generate::Keyframe {
    effects_distort_generate::parameters(kind)
        .into_iter()
        .find(|param| param.name == name)
        .unwrap()
        .keyframe
}

#[test]
fn rolling_shutter_rate_direction_and_methods() {
    let mut frame = solid(16, 16, BitsPerChannel::Bpc8, [0.0, 0.0, 0.0, 1.0]);
    let rate = 100.0_f32;
    for y in 0..16 {
        let shift = (y as f32 / 16.0 - 0.5) * rate / 100.0 * 16.0 * 0.5;
        let x = (8.0 - shift).round().clamp(0.0, 15.0) as u32;
        frame = paint(&frame, x, y, [1.0, 1.0, 1.0, 1.0]);
    }
    let mut fx = kind(EffectKind::RollingShutterRepair);
    fx.set_number("Rolling Shutter Rate", 100.0).unwrap();
    fx.set_choice("Method", "Warp").unwrap();
    fx.set_role("scan", Value::Index(0));
    let fixed = run(&fx, &frame, 0.0);
    let column = (0..16).filter(|y| at(&fixed, 8, *y)[0] > 0.4).count();
    assert!(column > 8, "{column}");
    fx.set_role("scan", Value::Index(1));
    let other = run(&fx, &frame, 0.0);
    assert!(diff(&fixed, &other) > 0.05);
    fx.set_role("scan", Value::Index(0));
    fx.set_choice("Method", "Pixel Motion").unwrap();
    assert!(fx.enabled("Pixel Motion Detail"));
    assert!(!fx.enabled("Detailed Analysis"));
    fx.set_number("Pixel Motion Detail", 1.0).unwrap();
    let coarse = run(&fx, &frame, 0.0);
    fx.set_number("Pixel Motion Detail", 4.0).unwrap();
    let fine = run(&fx, &frame, 0.0);
    assert!(diff(&coarse, &fine) > 0.0);
    fx.set_choice("Method", "Warp").unwrap();
    assert!(fx.enabled("Detailed Analysis"));
    assert!(!fx.enabled("Pixel Motion Detail"));
    fx.set_bool("Detailed Analysis", true).unwrap();
    let detailed = run(&fx, &frame, 0.0);
    assert_eq!(detailed.width(), 16);
}

#[test]
fn smear_offset_percent_and_elasticity() {
    let frame = paint(
        &solid(16, 16, BitsPerChannel::Bpc8, [0.1, 0.1, 0.1, 1.0]),
        4,
        8,
        [1.0, 0.0, 0.0, 1.0],
    );
    let mut fx = kind(EffectKind::Smear);
    fx.host.masks = vec![
        MaskShape::polygon(
            "boundary",
            vec![[1.0, 1.0], [14.0, 1.0], [14.0, 14.0], [1.0, 14.0]],
            0,
        ),
        MaskShape::polygon(
            "source",
            vec![[3.0, 6.0], [7.0, 6.0], [7.0, 10.0], [3.0, 10.0]],
            1,
        ),
    ];
    fx.set_point("Mask Offset", 6.0, 0.0).unwrap();
    fx.set_number("Mask Rotation", 0.0).unwrap();
    fx.set_number("Mask Scale", 100.0).unwrap();
    fx.set_number("Percent", 100.0).unwrap();
    let full = run(&fx, &frame, 0.0);
    fx.set_number("Percent", 50.0).unwrap();
    let half = run(&fx, &frame, 0.0);
    assert!(diff(&full, &half) > 0.01);
    fx.set_number("Percent", 100.0).unwrap();
    fx.set_number("Mask Rotation", 90.0).unwrap();
    let turned = run(&fx, &frame, 0.0);
    fx.set_number("Mask Rotation", 360.0).unwrap();
    let full_turn = run(&fx, &frame, 0.0);
    fx.set_number("Mask Rotation", 0.0).unwrap();
    let zero = run(&fx, &frame, 0.0);
    assert!(diff(&turned, &zero) > 0.01);
    assert!(diff(&full_turn, &zero) < diff(&turned, &zero));
    fx.set_number("Mask Scale", 200.0).unwrap();
    let scaled = run(&fx, &frame, 0.0);
    assert!(diff(&zero, &scaled) > 0.0);
    fx.set_role("elasticity", Value::Index(0));
    let stiff = run(&fx, &frame, 0.0);
    fx.set_role("elasticity", Value::Index(1));
    let fluid = run(&fx, &frame, 0.0);
    assert!(diff(&stiff, &fluid) > 0.0);
    let edge = at(&full, 0, 8);
    assert!((edge[0] - 0.1).abs() < 0.2);
}

#[test]
fn spherize_best_and_draft_sample_differently() {
    let mut frame = solid(12, 12, BitsPerChannel::Bpc8, [0.0, 0.0, 0.0, 1.0]);
    for y in 0..12 {
        for x in 0..12 {
            if (x + y) % 2 == 0 {
                frame = paint(&frame, x, y, [1.0, 1.0, 1.0, 1.0]);
            }
        }
    }
    let mut fx = kind(EffectKind::Spherize);
    assert!(effects_distort_generate::parameters(EffectKind::Spherize).is_empty());
    fx.host.quality = Quality::Best;
    let best = run(&fx, &frame, 0.0);
    draft(&mut fx);
    let draft = run(&fx, &frame, 0.0);
    assert!(diff(&best, &draft) > 0.01);
}

#[test]
fn transform_anchor_rotation_opacity_skew_and_shutter() {
    let frame = paint(
        &solid(12, 12, BitsPerChannel::Bpc16, [0.0, 0.0, 0.0, 1.0]),
        4,
        0,
        [1.0, 0.0, 0.0, 1.0],
    );
    let mut fx = kind(EffectKind::Transform);
    fx.set_point("Anchor Point", 0.0, 0.0).unwrap();
    fx.set_point("Position", 0.0, 0.0).unwrap();
    fx.set_number("Rotation", 90.0).unwrap();
    fx.set_number("Opacity", 1.0).unwrap();
    let turned = run(&fx, &frame, 0.0);
    assert!(at(&turned, 0, 4)[0] > 0.4);
    fx.set_point("Position", 2.0, 1.0).unwrap();
    let moved = run(&fx, &frame, 0.0);
    assert!(diff(&turned, &moved) > 0.05);
    fx.set_point("Position", 0.0, 0.0).unwrap();
    fx.set_number("Scale", 2.0).unwrap();
    let scaled = run(&fx, &frame, 0.0);
    assert!(diff(&turned, &scaled) > 0.05);
    fx.set_number("Scale", 1.0).unwrap();
    fx.set_number("Opacity", 0.25).unwrap();
    let faded = run(&fx, &frame, 0.0);
    assert!(at(&faded, 0, 4)[3] < at(&turned, 0, 4)[3]);
    fx.set_number("Opacity", 1.0).unwrap();
    fx.set_number("Rotation", 0.0).unwrap();
    fx.set_number("Skew", 30.0).unwrap();
    fx.set_number("Skew Axis", 0.0).unwrap();
    let skew = run(&fx, &frame, 0.0);
    fx.set_number("Skew Axis", 90.0).unwrap();
    let axis = run(&fx, &frame, 0.0);
    assert!(diff(&frame, &skew) > 0.01);
    assert!(diff(&skew, &axis) > 0.01);
    fx.set_number("Rotation", 40.0).unwrap();
    fx.host.layer_motion_blur = true;
    fx.host.comp_motion_blur = true;
    fx.host.comp_shutter = 180.0;
    fx.host.shutter = None;
    let comp = run(&fx, &frame, 0.0);
    fx.host.shutter = Some(45.0);
    let custom = run(&fx, &frame, 0.0);
    assert!(diff(&comp, &custom) > 0.0);
    fx.host.layer_motion_blur = false;
    let off = run(&fx, &frame, 0.0);
    assert!(diff(&custom, &off) > 0.0);
}

#[test]
fn turbulent_modes_evolution_seed_and_pin() {
    let frame = paint(
        &solid(16, 16, BitsPerChannel::Bpc32, [0.0, 0.0, 0.0, 1.0]),
        8,
        8,
        [1.0, 1.0, 1.0, 1.0],
    );
    let mut fx = kind(EffectKind::TurbulentDisplace);
    fx.set_choice("Displacement", "Turbulent").unwrap();
    fx.set_number("Amount", 0.0).unwrap();
    fx.set_number("Size", 8.0).unwrap();
    fx.set_number("Complexity", 2.0).unwrap();
    let flat = run(&fx, &frame, 0.0);
    fx.set_number("Amount", 6.0).unwrap();
    let turbulent = run(&fx, &frame, 0.0);
    assert!(diff(&flat, &turbulent) > 0.05);
    fx.set_choice("Displacement", "Turbulent Smoother").unwrap();
    let smoother = run(&fx, &frame, 0.0);
    assert!(diff(&turbulent, &smoother) > 0.0);
    fx.set_choice("Displacement", "Vertical Displacement")
        .unwrap();
    let vertical = run(&fx, &frame, 0.0);
    fx.set_choice("Displacement", "Horizontal Displacement")
        .unwrap();
    let horizontal = run(&fx, &frame, 0.0);
    fx.set_choice("Displacement", "Cross Displacement").unwrap();
    let cross = run(&fx, &frame, 0.0);
    assert!(diff(&vertical, &horizontal) > 0.0);
    assert!(diff(&horizontal, &cross) > 0.0);
    fx.set_choice("Displacement", "Bulge").unwrap();
    let bulge = run(&fx, &frame, 0.0);
    fx.set_choice("Displacement", "Twist").unwrap();
    let twist = run(&fx, &frame, 0.0);
    assert!(diff(&bulge, &twist) > 0.0);
    fx.set_number("Random Seed", 1.0).unwrap();
    let seed_a = run(&fx, &frame, 0.0);
    fx.set_number("Random Seed", 9.0).unwrap();
    let seed_b = run(&fx, &frame, 0.0);
    assert!(diff(&seed_a, &seed_b) > 0.0);
    fx.set_number("Evolution", 0.0).unwrap();
    let e0 = run(&fx, &frame, 0.0);
    fx.set_number("Evolution", 1.5).unwrap();
    let e1 = run(&fx, &frame, 0.0);
    assert!(diff(&e0, &e1) > 0.0);
    fx.set_bool("Cycle Evolution", true).unwrap();
    fx.set_number("Cycle", 1.0).unwrap();
    fx.set_number("Evolution", 1.25).unwrap();
    let cycled = run(&fx, &frame, 0.0);
    fx.set_number("Evolution", 0.25).unwrap();
    let wrapped = run(&fx, &frame, 0.0);
    assert!(diff(&cycled, &wrapped) < 0.05);
    fx.set_role("pin", Value::Index(1));
    fx.set_bool("Cycle Evolution", false).unwrap();
    fx.set_number("Amount", 8.0).unwrap();
    let pinned = run(&fx, &frame, 0.0);
    assert!((at(&pinned, 8, 0)[0] - at(&frame, 8, 0)[0]).abs() < 0.2);
    fx.set_bool("Resize Layer", true).unwrap();
    let resized = run(&fx, &frame, 0.0);
    assert!(resized.width() > frame.width());
}

#[test]
fn twirl_direction_and_radius() {
    let frame = paint(
        &solid(32, 32, BitsPerChannel::Bpc8, [0.0, 0.0, 0.0, 1.0]),
        20,
        16,
        [1.0, 0.0, 0.0, 1.0],
    );
    let mut fx = kind(EffectKind::Twirl);
    fx.set_number("Angle", 90.0).unwrap();
    fx.set_number("Twirl Radius", 50.0).unwrap();
    let cw = run(&fx, &frame, 0.0);
    let (x, y) = reddest(&cw);
    assert!(y > 16, "clockwise moved down to {x},{y}");
    fx.set_number("Angle", -90.0).unwrap();
    let ccw = run(&fx, &frame, 0.0);
    let (_, y2) = reddest(&ccw);
    assert!(y2 < 16, "counterclockwise moved up to {y2}");
    fx.set_number("Angle", 180.0).unwrap();
    fx.set_number("Twirl Radius", 50.0).unwrap();
    let edge = run(&fx, &frame, 0.0);
    fx.set_number("Twirl Radius", 5.0).unwrap();
    let small = run(&fx, &frame, 0.0);
    assert!(diff(&edge, &small) > 0.05);
}

fn reddest(frame: &effect_api::Frame) -> (u32, u32) {
    let image = view(frame);
    let mut best = (0, 0, -1.0_f32);
    for y in 0..image.height {
        for x in 0..image.width {
            let v = image.at(x, y)[0];
            if v > best.2 {
                best = (x, y, v);
            }
        }
    }
    (best.0, best.1)
}

#[test]
fn warp_style_changes_shape_without_confirmed_params() {
    assert!(effects_distort_generate::parameters(EffectKind::Warp).is_empty());
    let frame = paint(
        &solid(16, 16, BitsPerChannel::Bpc8, [0.0, 0.0, 0.0, 1.0]),
        8,
        4,
        [1.0, 1.0, 1.0, 1.0],
    );
    let mut fx = kind(EffectKind::Warp);
    fx.set_role("style", Value::Index(0));
    let a = run(&fx, &frame, 0.0);
    fx.set_role("style", Value::Index(1));
    let b = run(&fx, &frame, 0.0);
    assert!(diff(&a, &b) > 0.05);
}

#[test]
fn warp_stabilizer_controls() {
    let frame = solid(8, 8, BitsPerChannel::Bpc8, [0.4, 0.4, 0.4, 1.0]);
    let mut fx = kind(EffectKind::WarpStabilizer);
    assert_eq!(fx.choice("Result").as_deref(), Some("Smooth Motion"));
    assert_eq!(fx.choice("Method").as_deref(), Some("Subspace Warp"));
    assert_eq!(
        fx.choice("Framing").as_deref(),
        Some("Stabilize, Crop, Auto-scale")
    );
    assert_eq!(fx.choice("Objective").as_deref(), Some("Stabilize"));
    assert_eq!(
        fx.choice("Rolling Shutter Ripple").as_deref(),
        Some("Automatic Reduction")
    );
    assert!((fx.num("Synthesis Edge Cropping").unwrap_or(-1.0)).abs() < 1e-9);
    assert!(fx.enabled("Smoothness"));
    fx.set_choice("Result", "No Motion").unwrap();
    assert!(!fx.enabled("Smoothness"));
    assert!(!fx.enabled("Crop Less <-> Smooth More"));
    fx.set_choice("Result", "Smooth Motion").unwrap();
    fx.set_choice("Framing", "Stabilize Only").unwrap();
    assert!(!fx.enabled("Crop Less <-> Smooth More"));
    assert!(!fx.enabled("Maximum Scale"));
    fx.set_choice("Framing", "Stabilize, Crop").unwrap();
    assert!(!fx.enabled("Maximum Scale"));
    fx.set_choice("Framing", "Stabilize, Crop, Auto-scale")
        .unwrap();
    assert!(fx.enabled("Maximum Scale"));
    fx.set_choice("Framing", "Stabilize, Synthesize Edges")
        .unwrap();
    assert!(fx.enabled("Synthesis Edge Feather"));
    assert!(
        !fx.enabled("Rolling Shutter Ripple")
            || fx.choice("Method").as_deref() == Some("Subspace Warp")
    );
    fx.set_choice("Method", "Position").unwrap();
    assert!(!fx.enabled("Rolling Shutter Ripple"));
    fx.set_choice("Method", "Subspace Warp").unwrap();
    assert!(fx.enabled("Rolling Shutter Ripple"));
    fx.set_choice("Objective", "Apply Motion to Target")
        .unwrap();
    assert!(fx.enabled("Target Layer"));
    fx.set_choice("Objective", "Stabilize").unwrap();
    assert!(!fx.enabled("Target Layer"));
    assert_eq!(fallback_method(3, 1), 0);
    assert_eq!(fallback_method(3, 2), 1);
    assert_eq!(fallback_method(3, 4), 2);
    assert_eq!(fallback_method(3, 8), 3);
    fx.host.motion = vec![
        MotionSample {
            time: 0.0,
            translation: [0.0, 0.0],
            points: 2,
            row_shift: vec![],
        },
        MotionSample {
            time: 1.0,
            translation: [6.0, 0.0],
            points: 2,
            row_shift: vec![],
        },
        MotionSample {
            time: 2.0,
            translation: [0.0, 0.0],
            points: 2,
            row_shift: vec![],
        },
    ];
    fx.set_number("Smoothness", 100.0).unwrap();
    fx.press("Analyze").unwrap();
    assert!(fx.runtime.analysis.running);
    fx.press("Cancel").unwrap();
    assert!(fx.runtime.analysis.cancelled);
    let cancelled = run(&fx, &frame, 1.0);
    assert!(diff(&cancelled, &frame) < 0.05);
    fx.runtime.analysis.cancelled = false;
    fx.press("Analyze").unwrap();
    fx.finish_analysis();
    assert_eq!(fx.runtime.analysis.stab_method, Some(1));
    let stabilized = run(&fx, &frame, 1.0);
    assert_eq!(stabilized.width(), frame.width());
    fx.set_bool("Hide Warning Banner", true).unwrap();
    assert!(fx.flag("Hide Warning Banner").unwrap());
    fx.set_bool("Auto-delete Points Across Time", true).unwrap();
    fx.runtime.analysis.deleted_track_ids.push(1);
    assert_eq!(fx.runtime.analysis.deleted_track_ids, vec![1]);
}

#[test]
fn wave_warp_phase_direction_speed_pin_and_antialias() {
    assert!((wave_sample(0.0, 100.0, 0.0, 0.0, 0.0)).abs() < 1e-4);
    assert!((wave_sample(0.0, 100.0, 90.0, 0.0, 0.0) + 1.0).abs() < 1e-4);
    let dir = wave_dir(225.0);
    assert!(dir.0 < 0.0 && dir.1 > 0.0, "{dir:?}");
    let frame = paint(
        &solid(16, 16, BitsPerChannel::Bpc8, [0.0, 0.0, 0.0, 1.0]),
        8,
        8,
        [1.0, 1.0, 1.0, 1.0],
    );
    let mut fx = kind(EffectKind::WaveWarp);
    fx.set_number("Wave Height", 8.0).unwrap();
    fx.set_number("Wave Width", 4.0).unwrap();
    fx.set_number("Direction", 225.0).unwrap();
    fx.set_number("Wave Speed", 0.0).unwrap();
    fx.set_number("Phase", 0.0).unwrap();
    fx.set_role("wave_type", Value::Index(2));
    let phase0 = run(&fx, &frame, 0.0);
    fx.set_number("Phase", 90.0).unwrap();
    let phase90 = run(&fx, &frame, 0.0);
    assert!(diff(&phase0, &phase90) > 0.05);
    fx.set_number("Wave Speed", 1.0).unwrap();
    let forward = run(&fx, &frame, 0.25);
    fx.set_number("Wave Speed", -1.0).unwrap();
    let backward = run(&fx, &frame, 0.25);
    fx.set_number("Wave Speed", 0.0).unwrap();
    let stopped = run(&fx, &frame, 0.25);
    let stopped0 = run(&fx, &frame, 0.0);
    assert!(diff(&forward, &backward) > 0.01);
    assert!(diff(&stopped, &stopped0) < 0.05);
    fx.set_role("pin", Value::Index(1));
    fx.set_number("Wave Width", 6.0).unwrap();
    let pinned = run(&fx, &frame, 0.0);
    let free = {
        fx.set_role("pin", Value::Index(0));
        run(&fx, &frame, 0.0)
    };
    assert!(
        (at(&pinned, 8, 0)[0] - at(&frame, 8, 0)[0]).abs() < 0.25 || diff(&pinned, &free) > 0.0
    );
    fx.set_number("Antialiasing", 1.0).unwrap();
    fx.host.quality = Quality::Draft;
    let draft_a = run(&fx, &frame, 0.0);
    fx.set_number("Antialiasing", 4.0).unwrap();
    let draft_b = run(&fx, &frame, 0.0);
    fx.host.quality = Quality::Best;
    let best = run(&fx, &frame, 0.0);
    assert!(diff(&draft_a, &draft_b) < 0.05);
    assert!(diff(&draft_b, &best) > 0.0);
    fx.set_role("wave_type", Value::Index(0));
    let square = run(&fx, &frame, 0.0);
    fx.set_role("wave_type", Value::Index(1));
    let circular = run(&fx, &frame, 0.0);
    assert!(diff(&square, &circular) > 0.0);
}
