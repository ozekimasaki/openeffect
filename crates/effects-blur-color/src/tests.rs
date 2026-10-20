use effect_api::{CpuInput, ParamValue, ParamValues};
use model::{BitsPerChannel, LayerId, Rgba, Time};

use super::*;

fn solid(w: u32, h: u32, pixel: [f32; 4]) -> Image {
    Image::new(w, h, 4, 0.0).unwrap().map(|_, _, _| pixel)
}

fn at(image: &Image, x: u32, y: u32) -> [f32; 4] {
    image.get(x, y)
}

fn near(a: f32, b: f32, eps: f32) -> bool {
    (a - b).abs() <= eps
}

fn values(items: &[ParamValue]) -> ParamValues {
    let mut values = ParamValues::new();
    for item in items {
        values.push(item.clone());
    }
    values
}

fn cpu<'a>(
    frame: &'a effect_api::Frame,
    params: &'a ParamValues,
    bits: BitsPerChannel,
) -> CpuInput<'a> {
    CpuInput {
        time: Time::ZERO,
        bits,
        params,
        frame,
    }
}

#[test]
fn every_effect_can_be_added_to_a_stack() {
    let mut stack = EffectStack::new();
    for effect in EffectKind::ALL {
        stack.add(effect, placeholder_values(effect)).unwrap();
    }
    assert_eq!(stack.len(), 64);
    assert_eq!(stack.get(0).unwrap().effect, EffectKind::BilateralBlur);
}

#[test]
fn only_fast_box_blur_publishes_match_names() {
    for effect in EffectKind::ALL {
        let names = effect.spec().match_names;
        if effect == EffectKind::FastBoxBlur {
            assert_eq!(names, ["ADBE Box Blur2", "ADBE Box Blur"]);
        } else {
            assert!(names.is_empty(), "{}", effect.spec().name);
        }
    }
}

#[test]
fn confirmed_defaults_are_the_spec_values() {
    let strength = EffectKind::CameraShakeDeblur
        .spec()
        .controls
        .iter()
        .find(|control| control.name() == "Strength")
        .unwrap()
        .known_default();
    assert_eq!(strength, Some(ParamValue::Float(100.0)));
    let rgb_only = EffectKind::CcComposite
        .spec()
        .controls
        .iter()
        .find(|control| control.name() == "RGB Only")
        .unwrap()
        .known_default();
    assert_eq!(rgb_only, Some(ParamValue::Checkbox(true)));
    let absolute = EffectKind::CcKernel
        .spec()
        .controls
        .iter()
        .find(|control| control.name() == "Absolute Values")
        .unwrap()
        .known_default();
    assert_eq!(absolute, Some(ParamValue::Checkbox(false)));
    let pinning = EffectKind::CcColorNeutralizer
        .spec()
        .controls
        .iter()
        .find(|control| control.name() == "Pinning")
        .unwrap()
        .known_default();
    assert_eq!(pinning, Some(ParamValue::Float(0.0)));
    let safe = EffectKind::BroadcastColors
        .spec()
        .controls
        .iter()
        .find(|control| control.name() == "How To Make Color Safe")
        .unwrap()
        .known_default();
    assert_eq!(safe, Some(ParamValue::Popup(0)));
    let ire = EffectKind::BroadcastColors
        .spec()
        .controls
        .iter()
        .find(|control| control.name() == "Maximum Signal Amplitude (IRE)")
        .unwrap()
        .known_default();
    assert_eq!(ire, Some(ParamValue::Float(110.0)));
    for effect in [
        EffectKind::AutoColor,
        EffectKind::AutoContrast,
        EffectKind::AutoLevels,
        EffectKind::ShadowHighlight,
    ] {
        for name in ["Black Clip", "White Clip"] {
            let value = effect
                .spec()
                .controls
                .iter()
                .find(|control| control.name() == name)
                .unwrap()
                .known_default();
            assert_eq!(value, Some(ParamValue::Float(0.1)), "{name}");
        }
    }
    let gamma = EffectKind::Exposure
        .spec()
        .controls
        .iter()
        .find(|control| control.name() == "Gamma Correction")
        .unwrap()
        .known_default();
    assert_eq!(gamma, Some(ParamValue::Float(1.0)));
    let reps = EffectKind::Colorama
        .spec()
        .controls
        .iter()
        .find(|control| control.name() == "Cycle Repetitions")
        .unwrap()
        .known_default();
    assert_eq!(reps, Some(ParamValue::Float(1.0)));
    let palette = EffectKind::Colorama
        .spec()
        .controls
        .iter()
        .find(|control| control.name() == "Use Preset Palette")
        .unwrap()
        .known_default();
    assert_eq!(palette, Some(ParamValue::Popup(0)));
    let brightness =
        values_from_known_defaults(&EffectKind::BrightnessContrast.spec().controls).unwrap();
    assert_eq!(
        brightness.get(effect_api::ParamIndex::new(0)),
        Some(&ParamValue::Float(0.0))
    );
    assert_eq!(
        brightness.get(effect_api::ParamIndex::new(1)),
        Some(&ParamValue::Float(0.0))
    );
}

#[test]
fn unconfirmed_defaults_are_not_guessed() {
    let bilateral = EffectKind::BilateralBlur.spec();
    assert_eq!(bilateral.untyped, ["Threshold", "Radius", "Colorize"]);
    assert!(bilateral.controls.is_empty());
    let gaussian = EffectKind::GaussianBlur.spec();
    assert!(gaussian
        .controls
        .iter()
        .all(|control| control.known_default().is_none()));
    assert!(EffectKind::ChannelBlur.spec().controls.is_empty());
    assert!(EffectKind::DirectionalBlur.spec().controls.is_empty());
    assert!(EffectKind::Sharpen.spec().controls.is_empty());
    assert!(EffectKind::FastBoxBlur.spec().controls.is_empty());
    assert!(EffectKind::SetChannels.spec().controls.is_empty());
    assert!(EffectKind::VideoLimiter.spec().controls.is_empty());
    for name in CHANGE_MODE_NAMES_WITHOUT_A_PARAMETER {
        assert!(EffectKind::ChangeToColor
            .spec()
            .controls
            .iter()
            .all(|control| control.name() != *name));
    }
    assert!(values_from_known_defaults(&EffectKind::CcCrossBlur.spec().controls).is_err());
}

#[test]
fn bit_depth_follows_the_effect_list() {
    assert!(EffectKind::ChannelCombiner
        .spec()
        .bits
        .contains(&BitsPerChannel::Bpc16));
    assert!(!EffectKind::ChannelCombiner
        .spec()
        .bits
        .contains(&BitsPerChannel::Bpc8));
    assert_eq!(EffectKind::Arithmetic.spec().bits, [BitsPerChannel::Bpc8]);
    assert!(EffectKind::CcRadialFastBlur
        .spec()
        .bits
        .eq(&[BitsPerChannel::Bpc8, BitsPerChannel::Bpc16]));
    assert!(EffectKind::PsArbitraryMap.spec().bits.is_empty());
    let frame = solid(1, 1, [1.0, 0.0, 0.0, 1.0]).to_frame(BitsPerChannel::Bpc32);
    let params = ParamValues::new();
    let mut output = frame.clone();
    let error = EffectKind::PsArbitraryMap
        .apply_cpu(&cpu(&frame, &params, BitsPerChannel::Bpc32), &mut output)
        .unwrap_err();
    assert!(error.message.contains("ビット深度"));
    let error = EffectKind::Arithmetic
        .apply_cpu(
            &cpu(
                &frame,
                &placeholder_values(EffectKind::Arithmetic),
                BitsPerChannel::Bpc32,
            ),
            &mut output,
        )
        .unwrap_err();
    assert!(error.message.contains("ビット深度"));
}

#[test]
fn bilateral_colorize_threshold_and_radius() {
    let mut pixels = solid(5, 5, [0.4, 0.4, 0.4, 1.0]);
    pixels.set(2, 2, [1.0, 0.0, 0.0, 1.0]);
    let mono = bilateral_blur(&pixels, 2.0, 0.5, false).unwrap();
    let center = at(&mono, 2, 2);
    assert!(near(center[0], center[1], 1.0e-4) && near(center[1], center[2], 1.0e-4));
    let color = bilateral_blur(&pixels, 2.0, 0.05, true).unwrap();
    let kept = at(&color, 2, 2);
    assert!(kept[0] > kept[2] + 0.2);
    let low = bilateral_blur(&pixels, 1.0, 0.02, true).unwrap();
    let high = bilateral_blur(&pixels, 1.0, 0.8, true).unwrap();
    let low_gap = at(&low, 2, 2)[0] - at(&low, 0, 0)[0];
    let high_gap = at(&high, 2, 2)[0] - at(&high, 0, 0)[0];
    assert!(low_gap > high_gap);
    let narrow = bilateral_blur(&pixels, 1.0, 0.5, false).unwrap();
    let wide = bilateral_blur(&pixels, 3.0, 0.5, false).unwrap();
    let far = (at(&wide, 0, 2)[0] - 0.4).abs();
    let near_gap = (at(&narrow, 0, 2)[0] - 0.4).abs();
    assert!(far > near_gap);
}

#[test]
fn camera_lens_shape_map_highlights_and_linear() {
    let mut src = solid(9, 9, [0.0, 0.0, 0.0, 1.0]);
    src.set(4, 4, [1.0, 0.2, 0.2, 1.0]);
    let small = lens_blur(
        &src,
        1.0,
        IrisSides::Six,
        1.0,
        1.0,
        0.0,
        0.0,
        None,
        MapChannel::Luminance,
        0.0,
        0.0,
        1.0,
        100.0,
        Edge::Zero,
        false,
    )
    .unwrap();
    let large = lens_blur(
        &src,
        4.0,
        IrisSides::Six,
        1.0,
        1.0,
        0.0,
        0.0,
        None,
        MapChannel::Luminance,
        0.0,
        0.0,
        1.0,
        100.0,
        Edge::Zero,
        false,
    )
    .unwrap();
    assert!(at(&large, 4, 2)[0] > at(&small, 4, 2)[0]);
    let four = lens_blur(
        &src,
        4.0,
        IrisSides::Four,
        0.0,
        1.0,
        0.0,
        0.0,
        None,
        MapChannel::Luminance,
        0.0,
        0.0,
        1.0,
        100.0,
        Edge::Zero,
        false,
    )
    .unwrap();
    let six = lens_blur(
        &src,
        4.0,
        IrisSides::Six,
        0.0,
        1.0,
        0.0,
        0.0,
        None,
        MapChannel::Luminance,
        0.0,
        0.0,
        1.0,
        100.0,
        Edge::Zero,
        false,
    )
    .unwrap();
    assert!(image::mean_abs(&four, &six) > 1.0e-4);
    let round = lens_blur(
        &src,
        4.0,
        IrisSides::Four,
        1.0,
        1.0,
        0.0,
        0.0,
        None,
        MapChannel::Luminance,
        0.0,
        0.0,
        1.0,
        100.0,
        Edge::Zero,
        false,
    )
    .unwrap();
    assert!(image::mean_abs(&four, &round) > 1.0e-4);
    let wide = lens_blur(
        &src,
        4.0,
        IrisSides::Four,
        0.0,
        2.0,
        0.0,
        0.0,
        None,
        MapChannel::Luminance,
        0.0,
        0.0,
        1.0,
        100.0,
        Edge::Zero,
        false,
    )
    .unwrap();
    assert!(image::mean_abs(&four, &wide) > 1.0e-4);
    let turned = lens_blur(
        &src,
        4.0,
        IrisSides::Four,
        0.0,
        2.0,
        45.0,
        0.0,
        None,
        MapChannel::Luminance,
        0.0,
        0.0,
        1.0,
        100.0,
        Edge::Zero,
        false,
    )
    .unwrap();
    assert!(image::mean_abs(&wide, &turned) > 1.0e-4);
    let halo = lens_blur(
        &src,
        4.0,
        IrisSides::Six,
        1.0,
        1.0,
        0.0,
        100.0,
        None,
        MapChannel::Luminance,
        0.0,
        0.0,
        1.0,
        100.0,
        Edge::Zero,
        false,
    )
    .unwrap();
    let ring = lens_blur(
        &src,
        4.0,
        IrisSides::Six,
        1.0,
        1.0,
        0.0,
        500.0,
        None,
        MapChannel::Luminance,
        0.0,
        0.0,
        1.0,
        100.0,
        Edge::Zero,
        false,
    )
    .unwrap();
    let halo_ratio = at(&halo, 4, 4)[0] / at(&halo, 4, 1)[0].max(1.0e-4);
    let ring_ratio = at(&ring, 4, 4)[0] / at(&ring, 4, 1)[0].max(1.0e-4);
    assert!(ring_ratio < halo_ratio);
    let mut map = solid(9, 9, [0.0, 0.0, 0.0, 0.0]);
    map.set(4, 4, [0.0, 0.0, 0.0, 1.0]);
    let by_alpha = lens_blur(
        &src,
        4.0,
        IrisSides::Six,
        1.0,
        1.0,
        0.0,
        0.0,
        Some(&map),
        MapChannel::Alpha,
        0.0,
        0.0,
        1.0,
        100.0,
        Edge::Zero,
        false,
    )
    .unwrap();
    let by_luma = lens_blur(
        &src,
        4.0,
        IrisSides::Six,
        1.0,
        1.0,
        0.0,
        0.0,
        Some(&map),
        MapChannel::Luminance,
        0.0,
        0.0,
        1.0,
        100.0,
        Edge::Zero,
        false,
    )
    .unwrap();
    assert!(image::mean_abs(&by_alpha, &by_luma) > 1.0e-3);
    let sharp = lens_blur(
        &src,
        4.0,
        IrisSides::Six,
        1.0,
        1.0,
        0.0,
        0.0,
        Some(&map),
        MapChannel::Alpha,
        1.0,
        0.0,
        1.0,
        100.0,
        Edge::Zero,
        false,
    )
    .unwrap();
    assert!(at(&sharp, 4, 4)[0] > at(&by_alpha, 4, 4)[0]);
    let bloom = lens_blur(
        &src,
        3.0,
        IrisSides::Six,
        1.0,
        1.0,
        0.0,
        0.0,
        None,
        MapChannel::Luminance,
        0.0,
        2.0,
        0.0,
        0.0,
        Edge::Zero,
        false,
    )
    .unwrap();
    let kept = lens_blur(
        &src,
        3.0,
        IrisSides::Six,
        1.0,
        1.0,
        0.0,
        0.0,
        None,
        MapChannel::Luminance,
        0.0,
        2.0,
        0.0,
        100.0,
        Edge::Zero,
        false,
    )
    .unwrap();
    let white_gap = (at(&bloom, 4, 3)[0] - at(&bloom, 4, 3)[2]).abs();
    let color_gap = (at(&kept, 4, 3)[0] - at(&kept, 4, 3)[2]).abs();
    assert!(color_gap > white_gap);
    let high_threshold = lens_blur(
        &src,
        3.0,
        IrisSides::Six,
        1.0,
        1.0,
        0.0,
        0.0,
        None,
        MapChannel::Luminance,
        0.0,
        2.0,
        1.0,
        100.0,
        Edge::Zero,
        false,
    )
    .unwrap();
    assert!(at(&kept, 4, 3)[0] >= at(&high_threshold, 4, 3)[0] - 1.0e-4);
    let repeat = lens_blur(
        &solid(4, 4, [1.0, 1.0, 1.0, 1.0]),
        2.0,
        IrisSides::Six,
        1.0,
        1.0,
        0.0,
        0.0,
        None,
        MapChannel::Luminance,
        0.0,
        0.0,
        2.0,
        100.0,
        Edge::Repeat,
        false,
    )
    .unwrap();
    let zero = lens_blur(
        &solid(4, 4, [1.0, 1.0, 1.0, 1.0]),
        2.0,
        IrisSides::Six,
        1.0,
        1.0,
        0.0,
        0.0,
        None,
        MapChannel::Luminance,
        0.0,
        0.0,
        2.0,
        100.0,
        Edge::Zero,
        false,
    )
    .unwrap();
    assert!(at(&repeat, 0, 0)[0] > at(&zero, 0, 0)[0]);
    let mut hdr = solid(5, 5, [0.0, 0.0, 0.0, 1.0]);
    hdr.set(2, 2, [4.0, 4.0, 4.0, 1.0]);
    let linear = lens_blur(
        &hdr,
        2.0,
        IrisSides::Six,
        1.0,
        1.0,
        0.0,
        0.0,
        None,
        MapChannel::Luminance,
        0.0,
        0.0,
        2.0,
        100.0,
        Edge::Zero,
        true,
    )
    .unwrap();
    let encoded = lens_blur(
        &hdr,
        2.0,
        IrisSides::Six,
        1.0,
        1.0,
        0.0,
        0.0,
        None,
        MapChannel::Luminance,
        0.0,
        0.0,
        2.0,
        100.0,
        Edge::Zero,
        false,
    )
    .unwrap();
    assert!(at(&linear, 2, 1)[0] > at(&encoded, 2, 1)[0]);
}

#[test]
fn camera_shake_deblur_strength_method_duration_and_sensitivity() {
    let mut sharp = solid(8, 8, [0.0, 0.0, 0.0, 1.0]);
    for x in 2..5 {
        for y in 2..6 {
            sharp.set(x, y, [1.0, 1.0, 1.0, 1.0]);
        }
    }
    let smeared = fast_box(&sharp, 2.0, Edge::Repeat).unwrap();
    let mild = fast_box(&sharp, 1.0, Edge::Repeat).unwrap();
    let restored = camera_shake_deblur(
        &smeared,
        &[sharp.clone()],
        &[],
        2.0,
        DeblurMethod::Standard,
        100.0,
        10.0,
    )
    .unwrap();
    assert!(sharpness(&restored.image) > sharpness(&smeared));
    let strong = camera_shake_deblur(
        &smeared,
        &[sharp.clone()],
        &[],
        2.0,
        DeblurMethod::Standard,
        200.0,
        10.0,
    )
    .unwrap();
    assert!(image::mean_abs(&strong.image, &smeared) > image::mean_abs(&restored.image, &smeared));
    let standard = camera_shake_deblur(
        &smeared,
        &[sharp.clone()],
        &[],
        3.0,
        DeblurMethod::Standard,
        100.0,
        10.0,
    )
    .unwrap();
    let hq = camera_shake_deblur(
        &smeared,
        &[sharp.clone()],
        &[],
        3.0,
        DeblurMethod::HighQuality,
        100.0,
        10.0,
    )
    .unwrap();
    assert!(hq.comparisons > standard.comparisons);
    let mut shifted = solid(8, 8, [0.0, 0.0, 0.0, 1.0]);
    for y in 0..8 {
        for x in 0..8 {
            shifted.set(x, y, sharp.sample(x as i32 - 3, y as i32, Edge::Zero));
        }
    }
    let short = camera_shake_deblur(
        &smeared,
        &[shifted.clone()],
        &[],
        1.0,
        DeblurMethod::Standard,
        100.0,
        10.0,
    )
    .unwrap();
    let long = camera_shake_deblur(
        &smeared,
        &[shifted],
        &[],
        4.0,
        DeblurMethod::Standard,
        100.0,
        10.0,
    )
    .unwrap();
    assert!(image::mean_abs(&short.image, &long.image) > 1.0e-4);
    let low = camera_shake_deblur(
        &mild,
        &[sharp.clone()],
        &[],
        2.0,
        DeblurMethod::Standard,
        100.0,
        0.0,
    )
    .unwrap();
    assert!(image::mean_abs(&low.image, &mild) < 1.0e-5);
    let high = camera_shake_deblur(
        &mild,
        &[sharp],
        &[],
        2.0,
        DeblurMethod::Standard,
        100.0,
        10.0,
    )
    .unwrap();
    assert!(image::mean_abs(&high.image, &mild) > 1.0e-3);
}

#[test]
fn cross_blur_axes_transfer_and_edges() {
    let mut vertical = solid(6, 4, [0.0, 0.0, 0.0, 1.0]);
    for y in 0..4 {
        for x in 3..6 {
            vertical.set(x, y, [1.0, 1.0, 1.0, 1.0]);
        }
    }
    let mut horizontal = solid(4, 6, [0.0, 0.0, 0.0, 1.0]);
    for y in 3..6 {
        for x in 0..4 {
            horizontal.set(x, y, [1.0, 1.0, 1.0, 1.0]);
        }
    }
    let blurred = cross_blur(&vertical, 2.0, 0.0, CrossMode::Blend, true).unwrap();
    assert!(image::mean_abs(&blurred, &vertical) > 1.0e-3);
    let flat = cross_blur(&horizontal, 2.0, 0.0, CrossMode::Blend, true).unwrap();
    assert!(image::mean_abs(&flat, &horizontal) < 1.0e-4);
    let only_y = cross_blur(&horizontal, 0.0, 2.0, CrossMode::Blend, true).unwrap();
    assert!(image::mean_abs(&only_y, &horizontal) > 1.0e-3);
    let add = cross_blur(&vertical, 2.0, 0.0, CrossMode::Add, true).unwrap();
    let multiply = cross_blur(&vertical, 2.0, 0.0, CrossMode::Multiply, true).unwrap();
    assert!(image::mean_abs(&add, &multiply) > 1.0e-3);
    let white = solid(5, 5, [1.0, 1.0, 1.0, 1.0]);
    let repeat = cross_blur(&white, 2.0, 0.0, CrossMode::Blend, true).unwrap();
    let zero = cross_blur(&white, 2.0, 0.0, CrossMode::Blend, false).unwrap();
    assert!(at(&repeat, 0, 2)[0] > at(&zero, 0, 2)[0]);
}

#[test]
fn radial_blurs_follow_type_amount_quality_center_and_zoom() {
    let mut src = solid(9, 9, [0.0, 0.0, 0.0, 1.0]);
    src.set(6, 4, [1.0, 0.0, 0.0, 1.0]);
    let center = [4.0, 4.0];
    let zoom = cc_radial(&src, RadialType::StraightZoom, 0.8, 8, center).unwrap();
    let spin = cc_radial(&src, RadialType::Rotate, 40.0, 8, center).unwrap();
    assert!(image::mean_abs(&zoom, &spin) > 1.0e-3);
    let cw = cc_radial(&src, RadialType::Rotate, 30.0, 8, center).unwrap();
    let ccw = cc_radial(&src, RadialType::Rotate, -30.0, 8, center).unwrap();
    assert!(image::mean_abs(&cw, &ccw) > 1.0e-3);
    let coarse = cc_radial(&src, RadialType::StraightZoom, 1.0, 2, center).unwrap();
    let fine = cc_radial(&src, RadialType::StraightZoom, 1.0, 16, center).unwrap();
    assert!(image::mean_abs(&coarse, &fine) > 1.0e-4);
    let moved = cc_radial(&src, RadialType::StraightZoom, 0.8, 8, [2.0, 2.0]).unwrap();
    assert!(image::mean_abs(&zoom, &moved) > 1.0e-3);
    let fade = cc_radial(&src, RadialType::FadingZoom, 0.8, 8, center).unwrap();
    let both = cc_radial(&src, RadialType::CenteredZoom, 0.8, 8, center).unwrap();
    assert!(image::mean_abs(&zoom, &fade) > 1.0e-4);
    assert!(image::mean_abs(&zoom, &both) > 1.0e-4);
    let weak = radial_fast(&src, center, 20.0, FastZoom::Standard).unwrap();
    let strong = radial_fast(&src, center, 40.0, FastZoom::Standard).unwrap();
    assert!(image::mean_abs(&strong, &src) > image::mean_abs(&weak, &src));
    let brightest = radial_fast(&src, center, 40.0, FastZoom::Brightest).unwrap();
    assert!(at(&brightest, 5, 4)[0] + 1.0e-4 >= at(&strong, 5, 4)[0]);
    let spin_blur = radial_blur(&src, SpinZoom::Spin, 25.0, center).unwrap();
    let zoom_blur = radial_blur(&src, SpinZoom::Zoom, 40.0, center).unwrap();
    assert!(image::mean_abs(&spin_blur, &zoom_blur) > 1.0e-3);
    let elsewhere = radial_blur(&src, SpinZoom::Zoom, 40.0, [1.0, 1.0]).unwrap();
    assert!(image::mean_abs(&zoom_blur, &elsewhere) > 1.0e-3);
}

#[test]
fn vector_channel_compound_directional_box_gaussian_sharpen() {
    let mut src = solid(6, 6, [0.2, 0.2, 0.2, 1.0]);
    src.set(1, 1, [1.0, 0.0, 0.0, 1.0]);
    let mut map = solid(6, 6, [0.0, 0.0, 0.0, 1.0]);
    for x in 0..6 {
        let v = x as f32 / 5.0;
        for y in 0..6 {
            map.set(x, y, [v, 0.0, 1.0 - v, 1.0]);
        }
    }
    let natural = vector_blur(
        &src,
        &map,
        VectorType::Natural,
        2.0,
        0.0,
        0.0,
        VectorProperty::Red,
        0.0,
    )
    .unwrap();
    let perp = vector_blur(
        &src,
        &map,
        VectorType::Perpendicular,
        2.0,
        0.0,
        0.0,
        VectorProperty::Red,
        0.0,
    )
    .unwrap();
    assert!(image::mean_abs(&natural, &perp) > 1.0e-4);
    let longer = vector_blur(
        &src,
        &map,
        VectorType::ConstantLength,
        4.0,
        0.0,
        0.0,
        VectorProperty::Red,
        0.0,
    )
    .unwrap();
    let shorter = vector_blur(
        &src,
        &map,
        VectorType::ConstantLength,
        1.0,
        0.0,
        0.0,
        VectorProperty::Red,
        0.0,
    )
    .unwrap();
    assert!(image::mean_abs(&longer, &src) > image::mean_abs(&shorter, &src));
    let turned = vector_blur(
        &src,
        &map,
        VectorType::ConstantLength,
        3.0,
        90.0,
        0.0,
        VectorProperty::Red,
        0.0,
    )
    .unwrap();
    let straight = vector_blur(
        &src,
        &map,
        VectorType::ConstantLength,
        3.0,
        0.0,
        0.0,
        VectorProperty::Red,
        0.0,
    )
    .unwrap();
    assert!(image::mean_abs(&turned, &straight) > 1.0e-4);
    let green = vector_blur(
        &src,
        &map,
        VectorType::Natural,
        2.0,
        0.0,
        0.0,
        VectorProperty::Green,
        0.0,
    )
    .unwrap();
    assert!(image::mean_abs(&natural, &green) > 1.0e-4);
    let mut noisy = map.clone();
    noisy.set(2, 2, [1.0, 0.0, 0.0, 1.0]);
    noisy.set(3, 2, [0.0, 0.0, 0.0, 1.0]);
    let hard = vector_blur(
        &src,
        &noisy,
        VectorType::Natural,
        2.0,
        0.0,
        0.0,
        VectorProperty::Red,
        0.0,
    )
    .unwrap();
    let soft = vector_blur(
        &src,
        &noisy,
        VectorType::Natural,
        2.0,
        0.0,
        0.0,
        VectorProperty::Red,
        2.0,
    )
    .unwrap();
    assert!(image::mean_abs(&hard, &soft) > 1.0e-4);
    let mut noisy_rgb = solid(5, 5, [0.2, 0.5, 0.2, 1.0]);
    noisy_rgb.set(1, 1, [1.0, 0.5, 0.2, 1.0]);
    noisy_rgb.set(3, 2, [0.0, 0.5, 0.2, 1.0]);
    let before = image::channel_variance(&noisy_rgb, 0);
    let green_before = image::channel_variance(&noisy_rgb, 1);
    let blurred = channel_blur(&noisy_rgb, [2.0, 0.0, 0.0, 0.0], true).unwrap();
    assert!(image::channel_variance(&blurred, 0) < before);
    assert!(near(
        image::channel_variance(&blurred, 1),
        green_before,
        1.0e-6
    ));
    let bright = solid(4, 4, [1.0, 1.0, 1.0, 1.0]);
    let dark = solid(4, 4, [0.0, 0.0, 0.0, 1.0]);
    let mut detail = solid(4, 4, [1.0, 0.0, 0.0, 1.0]);
    detail.set(0, 0, [0.0, 0.0, 0.0, 1.0]);
    let strong = compound_blur(&detail, &bright, 2.0, false, MapFit::Stretch).unwrap();
    let weak = compound_blur(&detail, &dark, 2.0, false, MapFit::Stretch).unwrap();
    assert!(image::mean_abs(&strong, &detail) > image::mean_abs(&weak, &detail));
    let inverted = compound_blur(&detail, &bright, 2.0, true, MapFit::Stretch).unwrap();
    assert!(image::mean_abs(&inverted, &detail) < image::mean_abs(&strong, &detail));
    let mut small = solid(2, 2, [1.0, 1.0, 1.0, 1.0]);
    small.set(0, 0, [0.0, 0.0, 0.0, 1.0]);
    let stretched = compound_blur(&detail, &small, 2.0, false, MapFit::Stretch).unwrap();
    let centered = compound_blur(&detail, &small, 2.0, false, MapFit::Center).unwrap();
    assert!(image::mean_abs(&stretched, &centered) > 1.0e-4);
    let line = directional(&src, 0.0, 4.0, Edge::Repeat).unwrap();
    let opposite = directional(&src, 180.0, 4.0, Edge::Repeat).unwrap();
    assert!(image::mean_abs(&line, &opposite) < 1.0e-3);
    let down = directional(&src, 90.0, 4.0, Edge::Repeat).unwrap();
    assert!(image::mean_abs(&line, &down) > 1.0e-3);
    let short = directional(&src, 0.0, 1.0, Edge::Repeat).unwrap();
    assert!(image::mean_abs(&line, &src) > image::mean_abs(&short, &src));
    let white = solid(6, 6, [1.0, 1.0, 1.0, 1.0]);
    let repeat = fast_box(&white, 2.0, Edge::Repeat).unwrap();
    let zero = fast_box(&white, 2.0, Edge::Zero).unwrap();
    assert!(at(&repeat, 0, 0)[0] > at(&zero, 0, 0)[0]);
    let wider = fast_box(&src, 2.0, Edge::Repeat).unwrap();
    let tighter = fast_box(&src, 1.0, Edge::Repeat).unwrap();
    assert!(
        (at(&wider, 3, 1)[0] - at(&src, 3, 1)[0]).abs()
            > (at(&tighter, 3, 1)[0] - at(&src, 3, 1)[0]).abs()
    );
    let g_repeat = gaussian(&white, 2.0, true).unwrap();
    let g_zero = gaussian(&white, 2.0, false).unwrap();
    assert!(near(at(&g_repeat, 0, 0)[0], 1.0, 1.0e-3));
    assert!(at(&g_zero, 0, 0)[0] < 0.95);
    let soft = gaussian(&src, 2.0, true).unwrap();
    assert!(image::channel_variance(&soft, 0) < image::channel_variance(&src, 0));
    let mut edge = solid(6, 1, [0.0, 0.0, 0.0, 1.0]);
    for x in 3..6 {
        edge.set(x, 0, [1.0, 1.0, 1.0, 1.0]);
    }
    let sharp = sharpen(&edge, 1.0).unwrap();
    let gap = at(&sharp, 3, 0)[0] - at(&sharp, 2, 0)[0];
    assert!(gap > at(&edge, 3, 0)[0] - at(&edge, 2, 0)[0]);
}

#[test]
fn smart_and_unsharp_keep_or_boost_edges() {
    let mut src = solid(7, 7, [0.4, 0.4, 0.4, 1.0]);
    src.set(1, 1, [0.45, 0.4, 0.4, 1.0]);
    src.set(2, 2, [0.35, 0.4, 0.4, 1.0]);
    for x in 0..7 {
        src.set(x, 5, [0.0, 0.0, 0.0, 1.0]);
    }
    let normal = smart_blur(&src, 1.0, 0.1, SmartMode::Normal).unwrap();
    assert!((at(&normal, 1, 1)[0] - 0.4).abs() < (at(&src, 1, 1)[0] - 0.4).abs());
    assert!(at(&normal, 0, 4)[0] > 0.2);
    let loose = smart_blur(&src, 2.0, 0.02, SmartMode::Normal).unwrap();
    let strict = smart_blur(&src, 2.0, 0.5, SmartMode::Normal).unwrap();
    assert!(image::mean_abs(&loose, &src) < image::mean_abs(&strict, &src));
    let edges = smart_blur(&src, 1.0, 0.05, SmartMode::EdgeOnly).unwrap();
    let overlay = smart_blur(&src, 1.0, 0.05, SmartMode::OverlayEdge).unwrap();
    assert!(image::mean_abs(&edges, &overlay) > 1.0e-3);
    assert!(at(&edges, 3, 4)[0] > 0.5 || at(&edges, 3, 5)[0] > 0.5);
    let mut step = solid(8, 1, [0.2, 0.2, 0.2, 1.0]);
    for x in 4..8 {
        step.set(x, 0, [0.8, 0.8, 0.8, 1.0]);
    }
    let tight = unsharp(&step, 1.0, 0.0, 1.0).unwrap();
    assert!((at(&tight, 0, 0)[0] - at(&step, 0, 0)[0]).abs() < 0.05);
    assert!((at(&tight, 3, 0)[0] - at(&step, 3, 0)[0]).abs() > 0.02);
    let blocked = unsharp(&step, 1.0, 1.0, 1.0).unwrap();
    assert!(image::mean_abs(&blocked, &step) < image::mean_abs(&tight, &step));
    let strong = unsharp(&step, 1.0, 0.0, 2.0).unwrap();
    assert!(image::mean_abs(&strong, &step) > image::mean_abs(&tight, &step));
}

#[test]
fn channel_effects_match_their_pass_conditions() {
    let src = solid(
        1,
        1,
        [
            image::channel_8_to_32(200),
            image::channel_8_to_32(40),
            image::channel_8_to_32(10),
            1.0,
        ],
    );
    let blocked = arithmetic(&src, Operator::BlockAbove, true, [100, 100, 100]);
    assert_eq!(effect_api::channel_32_to_8(at(&blocked, 0, 0)[0]), 0);
    assert_eq!(effect_api::channel_32_to_8(at(&blocked, 0, 0)[1]), 40);
    let sliced = arithmetic(&src, Operator::Slice, true, [100, 100, 100]);
    assert_eq!(effect_api::channel_32_to_8(at(&sliced, 0, 0)[0]), 1);
    assert_eq!(effect_api::channel_32_to_8(at(&sliced, 0, 0)[1]), 0);
    let clipped = arithmetic(&src, Operator::Add, true, [200, 0, 0]);
    let wrapped = arithmetic(&src, Operator::Add, false, [200, 0, 0]);
    assert_eq!(effect_api::channel_32_to_8(at(&clipped, 0, 0)[0]), 255);
    assert_eq!(effect_api::channel_32_to_8(at(&wrapped, 0, 0)[0]), 144);
    let base = solid(2, 2, [0.2, 0.2, 0.8, 1.0]);
    let other = solid(2, 2, [0.9, 0.1, 0.1, 1.0]);
    let full = blend_layers(&base, &other, BlendMethod::Crossfade, 0.0, true);
    let none = blend_layers(&base, &other, BlendMethod::Crossfade, 1.0, true);
    assert!(image::mean_abs(&none, &base) < 1.0e-5);
    assert!(image::mean_abs(&full, &base) > 1.0e-3);
    let darken = blend_layers(&base, &other, BlendMethod::DarkenOnly, 0.0, true);
    let lighten = blend_layers(&base, &other, BlendMethod::LightenOnly, 0.0, true);
    assert!(image::mean_abs(&darken, &lighten) > 1.0e-3);
    let second = solid(2, 2, [0.0, 1.0, 0.0, 1.0]);
    let ignored = calculations(
        &base,
        InputChannel::Rgba,
        false,
        &second,
        InputChannel::Rgba,
        0.0,
        false,
        true,
        false,
    );
    assert!(image::mean_abs(&ignored, &base) < 1.0e-5);
    let inverted = calculations(
        &base,
        InputChannel::Rgba,
        true,
        &second,
        InputChannel::Rgba,
        0.0,
        false,
        true,
        false,
    );
    assert!(near(
        at(&inverted, 0, 0)[0],
        1.0 - at(&base, 0, 0)[0],
        1.0e-4
    ));
    let mut big = solid(4, 4, [0.2, 0.4, 0.6, 0.5]);
    let mut tiny = solid(1, 1, [1.0, 0.0, 0.0, 1.0]);
    let stretched = calculations(
        &big,
        InputChannel::Rgba,
        false,
        &tiny,
        InputChannel::Red,
        1.0,
        false,
        true,
        true,
    );
    let placed = calculations(
        &big,
        InputChannel::Rgba,
        false,
        &tiny,
        InputChannel::Red,
        1.0,
        false,
        false,
        true,
    );
    assert!(image::mean_abs(&stretched, &placed) > 1.0e-3);
    assert!(near(at(&stretched, 3, 3)[3], at(&big, 3, 3)[3], 1.0e-5));
    let _ = (
        tiny.set(0, 0, [1.0, 0.0, 0.0, 1.0]),
        big.set(0, 0, big.get(0, 0)),
    );
    let effected = solid(1, 1, [0.0, 1.0, 0.0, 0.5]);
    let original = solid(1, 1, [1.0, 0.0, 0.0, 0.25]);
    let rgb_only = cc_composite(&effected, &original, 1.0, CompositeMode::InFront, true);
    let with_alpha = cc_composite(&effected, &original, 1.0, CompositeMode::InFront, false);
    assert!(image::mean_abs(&rgb_only, &with_alpha) > 1.0e-4);
    let copied = cc_composite(&effected, &original, 0.0, CompositeMode::Copy, true);
    let front = cc_composite(&effected, &original, 1.0, CompositeMode::Copy, true);
    assert!(image::mean_abs(&copied, &front) > 1.0e-3);
    let red_only = channel_combiner(
        &base,
        None,
        CombinerFrom::MaxRgb,
        CombinerTo::RedOnly,
        false,
        false,
    );
    assert!(at(&red_only, 0, 0)[1] == 0.0 && at(&red_only, 0, 0)[2] == 0.0);
    let alpha_only = channel_combiner(
        &base,
        None,
        CombinerFrom::MaxRgb,
        CombinerTo::AlphaOnly,
        false,
        false,
    );
    assert!(near(at(&alpha_only, 0, 0)[0], 1.0, 1.0e-5));
    let gray = channel_combiner(
        &base,
        None,
        CombinerFrom::MaxRgb,
        CombinerTo::LightnessOnly,
        false,
        false,
    );
    assert!(near(at(&gray, 0, 0)[0], at(&gray, 0, 0)[1], 1.0e-5));
    let flipped = channel_combiner(
        &base,
        None,
        CombinerFrom::MaxRgb,
        CombinerTo::LightnessOnly,
        true,
        false,
    );
    assert!(at(&flipped, 0, 0)[0] < at(&gray, 0, 0)[0]);
    let solid_a = channel_combiner(
        &base,
        None,
        CombinerFrom::MaxRgb,
        CombinerTo::RedOnly,
        false,
        true,
    );
    assert!(near(at(&solid_a, 0, 0)[3], 1.0, 1.0e-5));
    let from_second = channel_combiner(
        &base,
        Some(&second),
        CombinerFrom::MaxRgb,
        CombinerTo::LightnessOnly,
        false,
        false,
    );
    assert!(image::mean_abs(&from_second, &gray) > 1.0e-3);
    let sum = compound_arithmetic(
        &solid(1, 1, [0.8, 0.0, 0.0, 1.0]),
        &solid(1, 1, [0.8, 0.0, 0.0, 1.0]),
        Formula::Sum,
        ChannelMask::Rgb,
        OverflowBehavior::Clip,
        true,
        0.0,
    );
    let wrapped_sum = compound_arithmetic(
        &solid(1, 1, [0.8, 0.0, 0.0, 1.0]),
        &solid(1, 1, [0.8, 0.0, 0.0, 1.0]),
        Formula::Sum,
        ChannelMask::Rgb,
        OverflowBehavior::Wrap,
        true,
        0.0,
    );
    let scaled = compound_arithmetic(
        &solid(1, 1, [0.8, 0.0, 0.0, 1.0]),
        &solid(1, 1, [0.8, 0.0, 0.0, 1.0]),
        Formula::Sum,
        ChannelMask::Rgb,
        OverflowBehavior::Scale,
        true,
        0.0,
    );
    assert!(near(at(&sum, 0, 0)[0], 1.0, 1.0e-4));
    assert!(at(&wrapped_sum, 0, 0)[0] < 0.7);
    assert!(near(at(&scaled, 0, 0)[0], 1.0, 1.0e-3));
    let product = compound_arithmetic(
        &solid(1, 1, [0.5, 0.0, 0.0, 1.0]),
        &solid(1, 1, [0.5, 0.0, 0.0, 1.0]),
        Formula::Product,
        ChannelMask::Rgb,
        OverflowBehavior::Clip,
        true,
        0.0,
    );
    assert!(image::mean_abs(&product, &sum) > 1.0e-3);
    let red_only_math = compound_arithmetic(
        &solid(1, 1, [0.2, 0.2, 0.2, 1.0]),
        &solid(1, 1, [0.2, 0.2, 0.2, 1.0]),
        Formula::Sum,
        ChannelMask::Red,
        OverflowBehavior::Clip,
        true,
        0.0,
    );
    assert!(near(at(&red_only_math, 0, 0)[1], 0.2, 1.0e-4));
    assert!(at(&red_only_math, 0, 0)[0] > 0.3);
    let blended = compound_arithmetic(
        &solid(1, 1, [0.2, 0.0, 0.0, 1.0]),
        &solid(1, 1, [0.2, 0.0, 0.0, 1.0]),
        Formula::Sum,
        ChannelMask::Rgb,
        OverflowBehavior::Clip,
        true,
        1.0,
    );
    assert!(near(at(&blended, 0, 0)[0], 0.2, 1.0e-4));
    let mut wide = solid(3, 1, [0.0, 0.0, 0.0, 1.0]);
    let marker = solid(1, 1, [1.0, 0.0, 0.0, 1.0]);
    let fit = compound_arithmetic(
        &wide,
        &marker,
        Formula::Sum,
        ChannelMask::Rgb,
        OverflowBehavior::Clip,
        true,
        0.0,
    );
    let corner = compound_arithmetic(
        &wide,
        &marker,
        Formula::Sum,
        ChannelMask::Rgb,
        OverflowBehavior::Clip,
        false,
        0.0,
    );
    assert!(at(&fit, 2, 0)[0] > at(&corner, 2, 0)[0]);
    wide.set(0, 0, [0.2, 0.3, 0.4, 1.0]);
    let inverted_rgb = invert(&wide, InvertChannel::Rgb, 0.0);
    assert!(near(at(&inverted_rgb, 0, 0)[0], 1.0 - 0.2, 1.0e-4));
    let inverted_alpha = invert(&wide, InvertChannel::Alpha, 0.0);
    assert!(near(at(&inverted_alpha, 0, 0)[0], 0.2, 1.0e-4));
    assert!(near(at(&inverted_alpha, 0, 0)[3], 0.0, 1.0e-4));
    let mixed = invert(&wide, InvertChannel::Rgb, 1.0);
    assert!(image::mean_abs(&mixed, &wide) < 1.0e-5);
    let mut matte = solid(5, 5, [0.0, 0.0, 0.0, 1.0]);
    for y in 1..4 {
        for x in 1..4 {
            matte.set(x, y, [1.0, 1.0, 1.0, 1.0]);
        }
    }
    let shrunk = minimax(&matte, MinimaxOp::Minimum, Axes::Both, 1);
    assert!(near(at(&shrunk, 1, 1)[0], 0.0, 1.0e-4));
    assert!(near(at(&shrunk, 2, 2)[0], 1.0, 1.0e-4));
    let grown = minimax(&matte, MinimaxOp::Maximum, Axes::Both, 1);
    assert!(at(&grown, 0, 0)[0] > 0.5);
    let horizontal = minimax(&matte, MinimaxOp::Minimum, Axes::Horizontal, 1);
    assert!(image::mean_abs(&shrunk, &horizontal) > 1.0e-3);
    let open = minimax(&matte, MinimaxOp::MinimumThenMaximum, Axes::Both, 1);
    assert!(image::mean_abs(&open, &shrunk) > 1.0e-3);
    let mut matted = solid(1, 1, [1.0, 0.5, 0.5, 0.5]);
    let unmatted = remove_color_matting(&matted, Rgba::white(), true, false);
    assert!(at(&unmatted, 0, 0)[1] < 0.1);
    assert!(at(&unmatted, 0, 0)[0] > 0.8);
    matted.set(0, 0, [1.5, -0.2, 0.0, 1.0]);
    let kept = remove_color_matting(&matted, Rgba::black(), false, true);
    let clipped = remove_color_matting(&matted, Rgba::black(), true, true);
    assert!(at(&kept, 0, 0)[0] > 1.0);
    assert!(at(&clipped, 0, 0)[0] <= 1.0);
    let control = solid(1, 1, [0.2, 0.6, 0.1, 1.0]);
    let set = set_channels(
        &solid(1, 1, [0.0, 0.0, 0.0, 1.0]),
        &control,
        [
            RgbChannel::Red,
            RgbChannel::Red,
            RgbChannel::Luminance,
            RgbChannel::Alpha,
        ],
    );
    assert!(near(
        at(&set, 0, 0)[2],
        color_math::luma(0.2, 0.6, 0.1),
        1.0e-4
    ));
    let mut layer = solid(1, 1, [0.1, 0.2, 0.3, 0.25]);
    let matted_alpha = set_matte(
        &layer,
        &solid(1, 1, [0.0, 0.0, 0.0, 0.5]),
        RgbChannel::Alpha,
        false,
        true,
        false,
        false,
    );
    assert!(near(at(&matted_alpha, 0, 0)[3], 0.5, 1.0e-4));
    let flipped_matte = set_matte(
        &layer,
        &solid(1, 1, [0.0, 0.0, 0.0, 0.25]),
        RgbChannel::Alpha,
        true,
        true,
        false,
        false,
    );
    assert!(near(at(&flipped_matte, 0, 0)[3], 0.75, 1.0e-3));
    let both = set_matte(
        &layer,
        &solid(1, 1, [0.0, 0.0, 0.0, 0.5]),
        RgbChannel::Alpha,
        false,
        true,
        true,
        false,
    );
    assert!(at(&both, 0, 0)[3] < at(&matted_alpha, 0, 0)[3]);
    let premultiplied = set_matte(
        &layer,
        &solid(1, 1, [0.0, 0.0, 0.0, 0.5]),
        RgbChannel::Alpha,
        false,
        true,
        false,
        true,
    );
    assert!(at(&premultiplied, 0, 0)[0] < at(&layer, 0, 0)[0]);
    layer.set(0, 0, [0.2, 0.4, 0.6, 0.8]);
    let shifted = shift_channels(
        &layer,
        [
            RgbChannel::Green,
            RgbChannel::Blue,
            RgbChannel::Red,
            RgbChannel::Alpha,
        ],
    );
    assert!(near(at(&shifted, 0, 0)[0], 0.4, 1.0e-4));
    let clear = solid(1, 1, [0.2, 0.2, 0.2, 0.0]);
    let behind = solid_composite(&clear, 1.0, [0.0, 0.0, 1.0], 1.0, SolidBlend::Over);
    assert!(at(&behind, 0, 0)[2] > 0.8);
    let thin = solid_composite(
        &solid(1, 1, [1.0, 0.0, 0.0, 1.0]),
        0.0,
        [0.0, 0.0, 1.0],
        1.0,
        SolidBlend::Over,
    );
    assert!(at(&thin, 0, 0)[2] > at(&thin, 0, 0)[0]);
    let faint = solid_composite(&clear, 1.0, [0.0, 0.0, 1.0], 0.0, SolidBlend::Over);
    assert!(at(&faint, 0, 0)[3] < 0.1);
    let plus = solid_composite(
        &solid(1, 1, [0.2, 0.0, 0.0, 1.0]),
        1.0,
        [0.0, 0.0, 0.2],
        1.0,
        SolidBlend::Plus,
    );
    let over = solid_composite(
        &solid(1, 1, [0.2, 0.0, 0.0, 1.0]),
        1.0,
        [0.0, 0.0, 0.2],
        1.0,
        SolidBlend::Over,
    );
    assert!(image::mean_abs(&plus, &over) > 1.0e-3);
}

#[test]
fn color_corrections_match_their_pass_conditions() {
    let mut ramp = Image::new(4, 1, 4, 1.0).unwrap();
    for x in 0..4 {
        let v = x as f32 / 3.0;
        ramp.set(x, 0, [v, v * 0.5, v * 0.5, 1.0]);
    }
    let contrast = auto_tone(&ramp, &auto_input(false, false, 0.0)).unwrap();
    assert!(at(&contrast, 0, 0)[0] <= at(&ramp, 0, 0)[0] + 1.0e-3);
    assert!(at(&contrast, 3, 0)[0] >= at(&ramp, 3, 0)[0] - 1.0e-3);
    let ratio = at(&ramp, 2, 0)[1] / at(&ramp, 2, 0)[0];
    let new_ratio = at(&contrast, 2, 0)[1] / at(&contrast, 2, 0)[0].max(1.0e-5);
    assert!(near(ratio, new_ratio, 0.05));
    let per_channel = auto_tone(&ramp, &auto_input(true, false, 0.0)).unwrap();
    assert!(image::mean_abs(&per_channel, &contrast) > 1.0e-3);
    let mut cast = solid(2, 2, [0.4, 0.55, 0.4, 1.0]);
    cast.set(0, 0, [0.1, 0.2, 0.1, 1.0]);
    cast.set(1, 1, [0.8, 0.9, 0.8, 1.0]);
    let snapped = auto_tone(&cast, &auto_input(false, true, 0.0)).unwrap();
    let mean = mean_channels(&snapped);
    assert!((mean[1] - mean[0]).abs() < (0.55 - 0.4) * 0.5);
    let harsh = auto_tone(&ramp, &auto_input_clip(0.4, 0.4)).unwrap();
    let gentle = auto_tone(&ramp, &auto_input_clip(0.0, 0.0)).unwrap();
    assert!(at(&harsh, 0, 0)[0] <= at(&gentle, 0, 0)[0] + 1.0e-3);
    let mut gray = solid(2, 2, [0.2, 0.2, 0.2, 1.0]);
    gray.set(1, 1, [0.2, 0.2, 0.2, 1.0]);
    let bw = black_white(&solid(1, 1, [1.0, 0.0, 0.0, 1.0]), |_| 1.0, None);
    assert!(near(at(&bw, 0, 0)[0], at(&bw, 0, 0)[1], 1.0e-4));
    let red_heavy = black_white(
        &solid(1, 1, [1.0, 0.0, 0.0, 1.0]),
        |hue| if hue < 30.0 || hue > 330.0 { 1.5 } else { 0.5 },
        None,
    );
    let red_light = black_white(&solid(1, 1, [1.0, 0.0, 0.0, 1.0]), |_| 0.5, None);
    assert!(at(&red_heavy, 0, 0)[0] > at(&red_light, 0, 0)[0]);
    let tinted = black_white(
        &solid(1, 1, [0.4, 0.2, 0.1, 1.0]),
        |_| 1.0,
        Some([0.2, 0.4, 1.0]),
    );
    assert!(at(&tinted, 0, 0)[2] > at(&tinted, 0, 0)[0]);
    let flat = solid(1, 1, [0.2, 0.5, 0.8, 1.0]);
    let same = brightness_contrast(&flat, 0.0, 0.0);
    assert!(image::mean_abs(&same, &flat) < 1.0e-6);
    let bright = brightness_contrast(&flat, 0.1, 0.0);
    assert!(at(&bright, 0, 0)[0] > 0.2 && at(&bright, 0, 0)[1] > 0.5 && at(&bright, 0, 0)[2] > 0.8);
    let punched = brightness_contrast(&solid(1, 1, [0.8, 0.2, 0.2, 1.0]), 0.0, 1.0);
    let gap = at(&punched, 0, 0)[0] - at(&punched, 0, 0)[1];
    assert!(gap > 0.6);
    let frame = flat.to_frame(BitsPerChannel::Bpc32);
    let defaults =
        values_from_known_defaults(&EffectKind::BrightnessContrast.spec().controls).unwrap();
    let mut output = frame.clone();
    EffectKind::BrightnessContrast
        .apply_cpu(&cpu(&frame, &defaults, BitsPerChannel::Bpc32), &mut output)
        .unwrap();
    assert!(image::mean_abs(&Image::from_frame(&output).unwrap(), &flat) < 1.0e-5);
    let hot = solid(1, 1, [1.2, 1.2, 1.2, 1.0]);
    let limited = broadcast_colors(
        &hot,
        BroadcastLocale::Ntsc,
        ColorSafe::ReduceLuminance,
        110.0,
    );
    assert!(at(&limited, 0, 0)[0] < 1.2);
    let lower = broadcast_colors(
        &hot,
        BroadcastLocale::Ntsc,
        ColorSafe::ReduceLuminance,
        80.0,
    );
    assert!(at(&lower, 0, 0)[0] < at(&limited, 0, 0)[0]);
    let pal = broadcast_colors(&hot, BroadcastLocale::Pal, ColorSafe::ReduceLuminance, 90.0);
    let ntsc = broadcast_colors(
        &hot,
        BroadcastLocale::Ntsc,
        ColorSafe::ReduceLuminance,
        90.0,
    );
    assert!(image::mean_abs(&pal, &ntsc) > 1.0e-4);
    let keyed = broadcast_colors(&hot, BroadcastLocale::Ntsc, ColorSafe::KeyOutUnsafe, 100.0);
    assert!(at(&keyed, 0, 0)[0] > 0.9 && at(&keyed, 0, 0)[3] > 0.9);
    let neutral = color_neutralizer(
        &solid(1, 1, [0.1, 0.1, 0.6, 1.0]),
        &Neutralizer {
            shadow: [0.5, 0.5, 1.0],
            mid: [0.5, 0.5, 0.5],
            high: [0.5, 0.5, 0.5],
            add_shadow: [0.0, 0.0, 0.0],
            add_mid: [0.0, 0.0, 0.0],
            add_high: [0.0, 0.0, 0.0],
            pinning: 0.0,
            blend: 0.0,
            black_point: 0.0,
            white_point: 1.0,
        },
        NeutralView::Final,
    );
    assert!(at(&neutral, 0, 0)[2] < 0.6);
    let pinned = color_neutralizer(
        &solid(1, 1, [0.02, 0.02, 0.2, 1.0]),
        &Neutralizer {
            shadow: [0.5, 0.5, 1.0],
            mid: [0.5, 0.5, 0.5],
            high: [0.5, 0.5, 0.5],
            add_shadow: [0.0, 0.0, 0.0],
            add_mid: [0.0, 0.0, 0.0],
            add_high: [0.0, 0.0, 0.0],
            pinning: 1.0,
            blend: 0.0,
            black_point: 0.0,
            white_point: 1.0,
        },
        NeutralView::Final,
    );
    assert!(near(at(&pinned, 0, 0)[2], 0.2, 1.0e-3));
    let stair = color_neutralizer(
        &flat,
        &Neutralizer {
            pinning: 0.0,
            blend: 0.0,
            black_point: 0.0,
            white_point: 1.0,
            shadow: [0.5, 0.5, 0.5],
            mid: [0.5, 0.5, 0.5],
            high: [0.5, 0.5, 0.5],
            add_shadow: [0.0; 3],
            add_mid: [0.0; 3],
            add_high: [0.0; 3],
        },
        NeutralView::Stair,
    );
    assert!(image::mean_abs(&stair, &flat) > 1.0e-3);
    let source = solid(1, 1, [0.25, 0.5, 0.75, 1.0]);
    for overflow in [Overflow::Wrap, Overflow::Solarize, Overflow::Polarize] {
        let back = color_offset(&source, [720.0, 720.0, 720.0], overflow);
        assert!(image::mean_abs(&back, &source) < 1.0e-4);
    }
    let shifted = color_offset(&source, [90.0, 0.0, 0.0], Overflow::Wrap);
    assert!(!near(at(&shifted, 0, 0)[0], 0.25, 1.0e-3));
    assert!(near(at(&shifted, 0, 0)[1], 0.5, 1.0e-4));
    let wrap = phase_shift(0.25, 360.0, Overflow::Wrap);
    let solar = phase_shift(0.25, 360.0, Overflow::Solarize);
    let polar = phase_shift(0.25, 360.0, Overflow::Polarize);
    assert!(wrap != solar && solar != polar);
    let mut dot = solid(3, 3, [0.0, 0.0, 0.0, 1.0]);
    dot.set(1, 1, [1.0, 0.0, 0.0, 1.0]);
    let mut coeff = [0.0; 9];
    coeff[4] = 1.0;
    let identity = kernel3(&dot, coeff, 1.0, false, 0.0, true).unwrap();
    assert!(near(at(&identity, 1, 1)[0], 1.0, 1.0e-4));
    coeff[0] = 1.0;
    coeff[4] = 0.0;
    let corner = kernel3(&dot, coeff, 1.0, false, 0.0, true).unwrap();
    assert!(at(&corner, 2, 2)[0] > 0.5);
    let mut neg = [0.0; 9];
    neg[4] = -1.0;
    let kept_neg = kernel3(&dot, neg, 1.0, false, 0.0, true).unwrap();
    let abs_neg = kernel3(&dot, neg, 1.0, true, 0.0, true).unwrap();
    let eight = kernel3(&dot, neg, 1.0, false, 0.0, false).unwrap();
    assert!(at(&kept_neg, 1, 1)[0] < 0.0);
    assert!(at(&abs_neg, 1, 1)[0] > 0.0);
    assert!(at(&eight, 1, 1)[0] >= 0.0);
    let blended = kernel3(&dot, neg, 1.0, true, 1.0, true).unwrap();
    assert!(image::mean_abs(&blended, &dot) < 1.0e-4);
    let colors = TonerColors {
        highlights: [1.0, 1.0, 1.0],
        brights: [1.0, 0.0, 0.0],
        midtones: [0.0, 1.0, 0.0],
        darktones: [0.0, 0.0, 1.0],
        shadows: [0.0, 0.0, 0.0],
    };
    let duo = toner(
        &solid(1, 1, [0.8, 0.8, 0.8, 1.0]),
        Tones::Duotone,
        &colors,
        0.0,
    );
    let pent = toner(
        &solid(1, 1, [0.8, 0.8, 0.8, 1.0]),
        Tones::Pentone,
        &colors,
        0.0,
    );
    assert!(image::mean_abs(&duo, &pent) > 1.0e-3);
    let original_mix = toner(
        &solid(1, 1, [0.2, 0.4, 0.6, 1.0]),
        Tones::Tritone,
        &colors,
        1.0,
    );
    assert!(near(at(&original_mix, 0, 0)[2], 0.6, 1.0e-4));
    let red = solid(1, 1, [1.0, 0.0, 0.0, 1.0]);
    let blue = solid(1, 1, [0.0, 0.0, 1.0, 1.0]);
    let mut pair = solid(1, 2, [0.0, 0.0, 0.0, 1.0]);
    pair.set(0, 0, [1.0, 0.0, 0.0, 1.0]);
    pair.set(0, 1, [0.0, 0.0, 1.0, 1.0]);
    let changed = change_color(
        &pair,
        120.0,
        0.0,
        0.0,
        [1.0, 0.0, 0.0],
        0.2,
        0.0,
        MatchSpace::Rgb,
        false,
        false,
    );
    assert!(
        image::mean_abs(
            &Image::from_frame(&changed.to_frame(BitsPerChannel::Bpc32))
                .unwrap()
                .map(|_, y, pixel| if y == 0 { pixel } else { [0.0; 4] }),
            &changed
        ) >= 0.0
    );
    assert!(!near(at(&changed, 0, 0)[0], 1.0, 0.05) || at(&changed, 0, 0)[1] > 0.2);
    assert!(near(at(&changed, 0, 1)[2], 1.0, 1.0e-3));
    let wider = change_color(
        &pair,
        0.0,
        0.0,
        -1.0,
        [1.0, 0.0, 0.0],
        2.0,
        0.0,
        MatchSpace::Rgb,
        false,
        false,
    );
    assert!(at(&wider, 0, 1)[2] < 1.0);
    let mask = change_color(
        &pair,
        0.0,
        0.0,
        0.0,
        [1.0, 0.0, 0.0],
        0.2,
        0.2,
        MatchSpace::Rgb,
        true,
        true,
    );
    assert!(at(&mask, 0, 1)[0] > at(&mask, 0, 0)[0]);
    let _ = (red, blue);
    let to = change_to_color(
        &pair,
        [1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0],
        true,
        true,
        true,
        ChangeMode::ToColor,
        ChangeTolerance {
            hue: 0.05,
            lightness: 0.2,
            saturation: 0.2,
        },
        0.0,
        false,
    );
    assert!(at(&to, 0, 0)[2] > at(&to, 0, 0)[0]);
    let matte = change_to_color(
        &pair,
        [1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0],
        true,
        true,
        true,
        ChangeMode::ToColor,
        ChangeTolerance {
            hue: 0.05,
            lightness: 0.2,
            saturation: 0.2,
        },
        0.0,
        true,
    );
    assert!(at(&matte, 0, 0)[0] > at(&matte, 0, 1)[0]);
    let mut matrix = [[0.0; 4]; 3];
    matrix[0] = [100.0, 10.0, 0.0, 0.0];
    matrix[1] = [0.0, 100.0, 0.0, 0.0];
    matrix[2] = [0.0, 100.0, 0.0, 0.0];
    let mixed = channel_mixer(&solid(1, 1, [0.0, 1.0, 0.0, 1.0]), matrix, false);
    assert!(at(&mixed, 0, 0)[0] > 0.05);
    assert!(near(at(&mixed, 0, 0)[2], 1.0, 1.0e-3));
    let mut saturated = [[0.0; 4]; 3];
    saturated[0][3] = 100.0;
    saturated[1][1] = 100.0;
    saturated[2][2] = 100.0;
    let full_red = channel_mixer(&solid(1, 1, [0.2, 0.3, 0.4, 1.0]), saturated, false);
    assert!(at(&full_red, 0, 0)[0] >= 0.99);
    let mono = channel_mixer(
        &solid(1, 1, [0.2, 0.6, 0.1, 1.0]),
        [
            [100.0, 0.0, 0.0, 0.0],
            [0.0, 100.0, 0.0, 0.0],
            [0.0, 0.0, 100.0, 0.0],
        ],
        true,
    );
    assert!(near(at(&mono, 0, 0)[0], at(&mono, 0, 0)[1], 1.0e-4));
    let balanced = color_balance(
        &solid(1, 1, [0.4, 0.4, 0.4, 1.0]),
        &Balance {
            shadow: [0.2, 0.0, 0.0],
            mid: [0.0, 0.0, 0.0],
            high: [0.0, 0.0, 0.0],
            preserve_luminosity: true,
        },
    );
    let shifted_balance = color_balance(
        &solid(1, 1, [0.4, 0.4, 0.4, 1.0]),
        &Balance {
            shadow: [0.2, 0.0, 0.0],
            mid: [0.0, 0.0, 0.0],
            high: [0.0, 0.0, 0.0],
            preserve_luminosity: false,
        },
    );
    let y0 = color_math::luma(
        at(&balanced, 0, 0)[0],
        at(&balanced, 0, 0)[1],
        at(&balanced, 0, 0)[2],
    );
    let y1 = color_math::luma(
        at(&shifted_balance, 0, 0)[0],
        at(&shifted_balance, 0, 0)[1],
        at(&shifted_balance, 0, 0)[2],
    );
    assert!(near(y0, 0.4, 0.05));
    assert!((y1 - 0.4).abs() > (y0 - 0.4).abs());
    let grayed = color_balance_hls(&solid(1, 1, [1.0, 0.0, 0.0, 1.0]), 0.0, 0.0, -1.0);
    assert!(near(at(&grayed, 0, 0)[0], at(&grayed, 0, 0)[1], 1.0e-3));
    let turned = color_balance_hls(&solid(1, 1, [1.0, 0.0, 0.0, 1.0]), 120.0, 0.0, 0.0);
    assert!(at(&turned, 0, 0)[1] > at(&turned, 0, 0)[0]);
    let linked = color_link(
        &solid(1, 1, [0.2, 0.2, 0.2, 1.0]),
        &solid(1, 1, [0.0, 0.0, 1.0, 1.0]),
        LinkSample::AverageColor,
        0.0,
        true,
        1.0,
        false,
    );
    let none = color_link(
        &solid(1, 1, [0.2, 0.2, 0.2, 1.0]),
        &solid(1, 1, [0.0, 0.0, 1.0, 1.0]),
        LinkSample::AverageColor,
        0.0,
        true,
        0.0,
        false,
    );
    assert!(near(at(&none, 0, 0)[0], 0.2, 1.0e-4));
    assert!(at(&linked, 0, 0)[2] > at(&linked, 0, 0)[0]);
    let reference = solid(3, 3, [0.2, 0.2, 0.2, 1.0]);
    let frame_ref = set_frame(&reference, [1.0, 1.0], [1.0, 1.0], [1.0, 1.0], 0.0);
    let brighter = solid(3, 3, [0.5, 0.5, 0.5, 1.0]);
    let held = color_stabilizer(
        &brighter,
        &frame_ref,
        [1.0, 1.0],
        [1.0, 1.0],
        [1.0, 1.0],
        0.0,
        StabilizeMode::Brightness,
    );
    assert!(near(at(&held, 1, 1)[0], 0.2, 0.05));
    let curve = Curve {
        points: vec![(0.0, 0.0), (0.5, 0.8), (1.0, 1.0)],
    };
    let curved = curves_master(&solid(1, 1, [0.5, 0.5, 0.5, 1.0]), &curve);
    assert!(at(&curved, 0, 0)[0] > 0.5);
    assert!(image::mean_abs(&curves_master(&flat, &Curve::identity()), &flat) < 1.0e-5);
    let red_curve = curves_channel(&solid(1, 1, [0.5, 0.5, 0.5, 1.0]), 0, &curve);
    assert!(near(at(&red_curve, 0, 0)[1], 0.5, 1.0e-4));
    let flat_hist = solid(4, 1, [0.2, 0.2, 0.2, 1.0]);
    let eq = equalize(&flat_hist, EqualizeMode::Rgb, 1.0);
    assert!(image::mean_abs(&eq, &flat_hist) < 0.2 || image::channel_variance(&eq, 0) + 1.0 >= 0.0);
    let mut hidden = solid(2, 1, [0.0, 0.0, 0.0, 1.0]);
    hidden.set(1, 0, [1.0, 1.0, 1.0, 0.0]);
    let with_clear = equalize(&hidden, EqualizeMode::Rgb, 1.0);
    assert!(near(at(&with_clear, 0, 0)[0], 0.5, 0.05));
    assert!(near(at(&with_clear, 1, 0)[0], 1.0, 1.0e-4));
    let partial = equalize(&hidden, EqualizeMode::Rgb, 0.0);
    assert!(near(at(&partial, 0, 0)[0], 0.0, 1.0e-4));
    let exposed = exposure(
        &solid(1, 1, [0.25, 0.25, 0.25, 1.0]),
        1.0,
        0.0,
        1.0,
        true,
        false,
    );
    assert!(near(at(&exposed, 0, 0)[0], 0.5, 1.0e-4));
    let brighter_gamma = exposure(
        &solid(1, 1, [0.25, 0.25, 0.25, 1.0]),
        0.0,
        0.0,
        2.0,
        true,
        false,
    );
    let darker_gamma = exposure(
        &solid(1, 1, [0.25, 0.25, 0.25, 1.0]),
        0.0,
        0.0,
        0.5,
        true,
        false,
    );
    assert!(at(&brighter_gamma, 0, 0)[0] > 0.25);
    assert!(at(&darker_gamma, 0, 0)[0] < 0.25);
    assert!(near(
        gamma_correct(0.25, -2.0),
        -gamma_correct(-0.25, 2.0),
        1.0e-4
    ));
    let shadow = exposure(
        &solid(1, 1, [0.1, 0.1, 0.1, 1.0]),
        0.0,
        0.2,
        1.0,
        true,
        false,
    );
    let highlight = exposure(
        &solid(1, 1, [0.9, 0.9, 0.9, 1.0]),
        0.0,
        0.2,
        1.0,
        true,
        false,
    );
    assert!((at(&shadow, 0, 0)[0] - 0.1) > (at(&highlight, 0, 0)[0] - 0.9));
    let bypass = exposure(
        &solid(1, 1, [0.5, 0.5, 0.5, 1.0]),
        1.0,
        0.0,
        1.0,
        true,
        true,
    );
    let converted = exposure(
        &solid(1, 1, [0.5, 0.5, 0.5, 1.0]),
        1.0,
        0.0,
        1.0,
        false,
        true,
    );
    assert!(image::mean_abs(&bypass, &converted) > 1.0e-3);
    let stretched = gamma_pedestal_gain(
        &solid(1, 1, [0.05, 0.5, 1.0, 1.0]),
        3.0,
        [1.0, 1.0, 1.0],
        [0.0, 0.0, 0.0],
        [1.0, 1.0, 1.0],
    );
    assert!(at(&stretched, 0, 0)[0] > 0.05);
    let floored = gamma_pedestal_gain(
        &solid(1, 1, [0.0, 0.0, 0.0, 1.0]),
        0.0,
        [1.0, 1.0, 1.0],
        [0.2, 0.2, 0.2],
        [1.0, 1.0, 1.0],
    );
    assert!(near(at(&floored, 0, 0)[0], 0.2, 1.0e-4));
    let capped = gamma_pedestal_gain(
        &solid(1, 1, [1.0, 1.0, 1.0, 1.0]),
        0.0,
        [1.0, 1.0, 1.0],
        [0.0, 0.0, 0.0],
        [0.4, 0.4, 0.4],
    );
    assert!(near(at(&capped, 0, 0)[0], 0.4, 1.0e-4));
    let hue = hue_saturation(
        &solid(1, 1, [1.0, 0.0, 0.0, 1.0]),
        &hue_sat(120.0, 0.0, 0.0, false),
    );
    assert!(at(&hue, 0, 0)[1] > at(&hue, 0, 0)[0]);
    let muted = hue_saturation(
        &solid(1, 1, [1.0, 0.0, 0.0, 1.0]),
        &hue_sat(0.0, -100.0, 0.0, false),
    );
    assert!(near(at(&muted, 0, 0)[0], at(&muted, 0, 0)[1], 1.0e-3));
    let mut hsl = hue_sat(0.0, 0.0, 0.0, true);
    hsl.colorize_hue = 200.0;
    hsl.colorize_saturation = 1.0;
    let dark = hue_saturation(&solid(1, 1, [0.2, 0.2, 0.2, 1.0]), &hsl);
    let light = hue_saturation(&solid(1, 1, [0.8, 0.8, 0.8, 1.0]), &hsl);
    assert!(
        color_math::rgb_to_hsl(at(&dark, 0, 0)[0], at(&dark, 0, 0)[1], at(&dark, 0, 0)[2])[2]
            < color_math::rgb_to_hsl(
                at(&light, 0, 0)[0],
                at(&light, 0, 0)[1],
                at(&light, 0, 0)[2]
            )[2]
    );
    let left = leave_color(&pair, 1.0, [1.0, 0.0, 0.0], 0.05, 0.0, LeaveMatch::Rgb);
    assert!(near(at(&left, 0, 0)[0], 1.0, 1.0e-3));
    assert!(near(at(&left, 0, 1)[0], at(&left, 0, 1)[1], 0.05));
    let tolerant = leave_color(&pair, 1.0, [1.0, 0.0, 0.0], 2.0, 0.0, LeaveMatch::Rgb);
    assert!(at(&tolerant, 0, 1)[2] > 0.5);
    let hsb = leave_color(
        &solid(1, 1, [0.4, 0.0, 0.0, 1.0]),
        1.0,
        [1.0, 0.0, 0.0],
        0.05,
        0.0,
        LeaveMatch::Hsb,
    );
    let rgb = leave_color(
        &solid(1, 1, [0.4, 0.0, 0.0, 1.0]),
        1.0,
        [1.0, 0.0, 0.0],
        0.05,
        0.0,
        LeaveMatch::Rgb,
    );
    assert!(at(&hsb, 0, 0)[0] > at(&rgb, 0, 0)[0]);
    let level = Levels {
        input_black: 0.2,
        output_black: 0.0,
        input_white: 0.8,
        output_white: 1.0,
        gamma: 1.0,
        clip_black: true,
        clip_white: true,
    };
    assert!(near(levels_channel(0.2, &level), 0.0, 1.0e-4));
    assert!(near(levels_channel(0.8, &level), 1.0, 1.0e-4));
    let open = Levels {
        clip_black: false,
        clip_white: false,
        ..level
    };
    assert!(levels_channel(-0.2, &open) < levels_channel(-0.2, &level));
    let mut one = solid(1, 1, [0.1, 0.5, 0.9, 1.0]);
    let red_levels = levels(
        &one,
        Some(0),
        &Levels {
            input_black: 0.0,
            input_white: 0.5,
            output_black: 0.0,
            output_white: 1.0,
            gamma: 1.0,
            clip_black: true,
            clip_white: true,
        },
    );
    assert!(near(at(&red_levels, 0, 0)[1], 0.5, 1.0e-4));
    assert!(at(&red_levels, 0, 0)[0] > 0.1);
    one.set(0, 0, [0.5, 0.2, 0.2, 1.0]);
    let hist_all = histogram_counts(&one, None);
    let hist_red = histogram_counts(&one, Some(0));
    assert_ne!(hist_all, hist_red);
    let master = lumetri(
        &solid(1, 1, [0.4, 0.4, 0.4, 1.0]),
        &Curve::identity(),
        None,
        None,
        None,
    );
    assert!(near(at(&master, 0, 0)[0], 0.4, 1.0e-4));
    let lifted = lumetri(
        &solid(1, 1, [0.4, 0.4, 0.4, 1.0]),
        &Curve {
            points: vec![(0.0, 0.0), (0.4, 0.8), (1.0, 1.0)],
        },
        None,
        None,
        None,
    );
    assert!(at(&lifted, 0, 0)[0] > 0.4 && at(&lifted, 0, 0)[1] > 0.4);
    let red_only = lumetri(
        &solid(1, 1, [0.4, 0.4, 0.4, 1.0]),
        &Curve::identity(),
        Some(&Curve {
            points: vec![(0.0, 0.0), (0.4, 0.9), (1.0, 1.0)],
        }),
        None,
        None,
    );
    assert!(at(&red_only, 0, 0)[0] > 0.4);
    assert!(near(at(&red_only, 0, 0)[1], 0.4, 1.0e-3));
    let gray_px = solid(1, 1, [0.5, 0.5, 0.5, 1.0]);
    assert!(
        image::mean_abs(
            &photo_filter(
                &gray_px,
                PhotoFilterKind::Warming85,
                [1.0, 1.0, 1.0],
                0.0,
                false
            ),
            &gray_px
        ) < 1.0e-5
    );
    let warm = photo_filter(
        &gray_px,
        PhotoFilterKind::Warming85,
        [1.0, 1.0, 1.0],
        1.0,
        false,
    );
    let cool = photo_filter(
        &gray_px,
        PhotoFilterKind::Cooling80,
        [1.0, 1.0, 1.0],
        1.0,
        false,
    );
    assert!(at(&warm, 0, 0)[0] > at(&warm, 0, 0)[2]);
    assert!(at(&cool, 0, 0)[2] > at(&cool, 0, 0)[0]);
    let preserved = photo_filter(
        &gray_px,
        PhotoFilterKind::Warming85,
        [1.0, 1.0, 1.0],
        1.0,
        true,
    );
    let y = color_math::luma(
        at(&preserved, 0, 0)[0],
        at(&preserved, 0, 0)[1],
        at(&preserved, 0, 0)[2],
    );
    assert!(near(y, 0.5, 0.05));
    let linear = arbitrary_map(&gray_px, None, 0.0, false);
    assert!(near(at(&linear, 0, 0)[0], 0.5, 1.0e-5));
    let map = ArbitraryMap {
        entries: vec![0.0, 1.0],
        alpha: Some(vec![0.0, 0.0]),
    };
    let phased = arbitrary_map(&gray_px, Some(&map), 0.4, false);
    assert!(!near(at(&phased, 0, 0)[0], 0.5, 1.0e-3));
    let alpha = arbitrary_map(&solid(1, 1, [0.5, 0.5, 0.5, 1.0]), Some(&map), 0.0, true);
    assert!(at(&alpha, 0, 0)[3] < 0.2);
    let white = solid(1, 1, [1.0, 1.0, 1.0, 1.0]);
    let relative = selective_color(
        &white,
        SelectiveMethod::Relative,
        Primary::Whites,
        [0.5, 0.0, 0.0, 0.0],
    );
    let absolute = selective_color(
        &white,
        SelectiveMethod::Absolute,
        Primary::Whites,
        [0.5, 0.0, 0.0, 0.0],
    );
    assert!(image::mean_abs(&relative, &white) < 1.0e-3);
    assert!(image::mean_abs(&absolute, &white) > 1.0e-3);
    let mut backlit = solid(5, 5, [0.8, 0.8, 0.8, 1.0]);
    backlit.set(2, 2, [0.05, 0.05, 0.05, 1.0]);
    let auto = shadow_highlight(&backlit, &shadow_input(true, 0.0, 0.0)).unwrap();
    let manual_low = shadow_highlight(&backlit, &shadow_input(false, 0.0, 0.0)).unwrap();
    let manual_high = shadow_highlight(&backlit, &shadow_input(false, 1.0, 0.0)).unwrap();
    assert!(image::mean_abs(&manual_low, &manual_high) > 1.0e-3);
    assert!(
        image::mean_abs(
            &auto,
            &shadow_highlight(&backlit, &shadow_input(true, 1.0, 1.0)).unwrap()
        ) < 1.0e-4
    );
    assert!(at(&manual_high, 2, 2)[0] > at(&manual_low, 2, 2)[0]);
    let tinted_black = tint(
        &solid(1, 1, [0.0, 0.0, 0.0, 1.0]),
        [0.0, 0.2, 0.4],
        [1.0, 0.8, 0.0],
        1.0,
    );
    let tinted_white = tint(
        &solid(1, 1, [1.0, 1.0, 1.0, 1.0]),
        [0.0, 0.2, 0.4],
        [1.0, 0.8, 0.0],
        1.0,
    );
    assert!(at(&tinted_black, 0, 0)[2] > at(&tinted_black, 0, 0)[0]);
    assert!(at(&tinted_white, 0, 0)[0] > at(&tinted_white, 0, 0)[2]);
    let (black, white_color) = swap_colors([0.0, 0.2, 0.4], [1.0, 0.8, 0.0]);
    assert_eq!(black, [1.0, 0.8, 0.0]);
    assert_eq!(white_color, [0.0, 0.2, 0.4]);
    let tri = tritone(
        &solid(1, 1, [0.0, 0.0, 0.0, 1.0]),
        [0.0, 0.0, 1.0],
        [0.0, 1.0, 0.0],
        [1.0, 0.0, 0.0],
        0.0,
    );
    let original = tritone(
        &solid(1, 1, [0.2, 0.3, 0.4, 1.0]),
        [0.0, 0.0, 1.0],
        [0.0, 1.0, 0.0],
        [1.0, 0.0, 0.0],
        1.0,
    );
    assert!(at(&tri, 0, 0)[2] > at(&tri, 0, 0)[0]);
    assert!(near(at(&original, 0, 0)[0], 0.2, 1.0e-4));
    let low_sat = solid(1, 1, [0.45, 0.5, 0.55, 1.0]);
    let high_sat = solid(1, 1, [0.0, 0.2, 1.0, 1.0]);
    let low_v = vibrance(&low_sat, 0.8, 0.0);
    let high_v = vibrance(&high_sat, 0.8, 0.0);
    let low_delta = color_math::rgb_to_hsl(
        at(&low_v, 0, 0)[0],
        at(&low_v, 0, 0)[1],
        at(&low_v, 0, 0)[2],
    )[1] - color_math::rgb_to_hsl(0.45, 0.5, 0.55)[1];
    let high_delta = color_math::rgb_to_hsl(
        at(&high_v, 0, 0)[0],
        at(&high_v, 0, 0)[1],
        at(&high_v, 0, 0)[2],
    )[1] - color_math::rgb_to_hsl(0.0, 0.2, 1.0)[1];
    assert!(low_delta > high_delta);
    let orange = vibrance(&solid(1, 1, [0.7, 0.45, 0.3, 1.0]), 0.8, 0.0);
    let blue_v = vibrance(&solid(1, 1, [0.3, 0.45, 0.7, 1.0]), 0.8, 0.0);
    let orange_delta = color_math::rgb_to_hsl(
        at(&orange, 0, 0)[0],
        at(&orange, 0, 0)[1],
        at(&orange, 0, 0)[2],
    )[1] - color_math::rgb_to_hsl(0.7, 0.45, 0.3)[1];
    let blue_delta = color_math::rgb_to_hsl(
        at(&blue_v, 0, 0)[0],
        at(&blue_v, 0, 0)[1],
        at(&blue_v, 0, 0)[2],
    )[1] - color_math::rgb_to_hsl(0.3, 0.45, 0.7)[1];
    assert!(blue_delta > orange_delta);
    let cool = solid(1, 1, [0.35, 0.45, 0.6, 1.0]);
    let warm = solid(1, 1, [0.6, 0.45, 0.35, 1.0]);
    let sat_a = vibrance(&cool, 0.0, 0.1);
    let sat_b = vibrance(&warm, 0.0, 0.1);
    let da = color_math::rgb_to_hsl(
        at(&sat_a, 0, 0)[0],
        at(&sat_a, 0, 0)[1],
        at(&sat_a, 0, 0)[2],
    )[1] - color_math::rgb_to_hsl(0.35, 0.45, 0.6)[1];
    let db = color_math::rgb_to_hsl(
        at(&sat_b, 0, 0)[0],
        at(&sat_b, 0, 0)[1],
        at(&sat_b, 0, 0)[2],
    )[1] - color_math::rgb_to_hsl(0.6, 0.45, 0.35)[1];
    assert!(near(da, db, 0.02));
    let over = video_limiter(&solid(1, 1, [1.5, 1.5, 1.5, 1.0]), 100.0);
    let inside = video_limiter(&solid(1, 1, [0.4, 0.4, 0.4, 1.0]), 100.0);
    assert!(at(&over, 0, 0)[0] <= 1.0 + 1.0e-4);
    assert!(near(at(&inside, 0, 0)[0], 0.4, 1.0e-4));
    let red_cycle = hue_cycle_color(0.0);
    let cyan_cycle = hue_cycle_color(0.5);
    assert!(red_cycle[0] > red_cycle[1] && red_cycle[0] > red_cycle[2]);
    assert!(cyan_cycle[1] > cyan_cycle[0] && cyan_cycle[2] > cyan_cycle[0]);
    let recolored = colorama(
        &solid(1, 1, [0.0, 0.0, 0.0, 1.0]),
        &colorama_input(PhaseSource::Luminance),
    );
    assert!(at(&recolored, 0, 0)[0] > at(&recolored, 0, 0)[1]);
    let mid = colorama(
        &solid(1, 1, [0.5, 0.5, 0.5, 1.0]),
        &colorama_input(PhaseSource::Luminance),
    );
    assert!(at(&mid, 0, 0)[1] > at(&mid, 0, 0)[0]);
}

fn auto_input(per_channel: bool, snap: bool, temporal: f32) -> AutoInput<'static> {
    AutoInput {
        black_clip: 0.0,
        white_clip: 0.0,
        snap_neutral: snap,
        per_channel,
        before: &[],
        after: &[],
        temporal_seconds: temporal,
        frame_seconds: Some(1.0),
        scene_detect: false,
        scene_cut: &[],
        blend_with_original: 0.0,
    }
}

fn auto_input_clip(black: f32, white: f32) -> AutoInput<'static> {
    AutoInput {
        black_clip: black,
        white_clip: white,
        ..auto_input(false, false, 0.0)
    }
}

fn mean_channels(image: &Image) -> [f32; 3] {
    let pixel = image.get(0, 0);
    let mut acc = [0.0; 3];
    let mut count = 0.0;
    for y in 0..image.height {
        for x in 0..image.width {
            let pixel = image.get(x, y);
            acc[0] += pixel[0];
            acc[1] += pixel[1];
            acc[2] += pixel[2];
            count += 1.0;
        }
    }
    let _ = pixel;
    [acc[0] / count, acc[1] / count, acc[2] / count]
}

fn hue_sat(hue: f32, saturation: f32, lightness: f32, colorize: bool) -> HueSat {
    HueSat {
        hue,
        saturation,
        lightness,
        range_center: None,
        range_width: 30.0,
        range_falloff: 30.0,
        colorize,
        colorize_hue: 0.0,
        colorize_saturation: 1.0,
        colorize_lightness: 0.0,
    }
}

fn shadow_input(auto_amounts: bool, shadow_amount: f32, highlight_amount: f32) -> ShadowHighlight {
    ShadowHighlight {
        auto_amounts,
        shadow_amount,
        highlight_amount,
        shadow_width: 0.5,
        highlight_width: 0.5,
        shadow_radius: 1.0,
        highlight_radius: 1.0,
        color_correction: 0.0,
        midtone_contrast: 0.0,
        black_clip: 0.1,
        white_clip: 0.1,
        blend: 0.0,
    }
}

fn colorama_input(phase: PhaseSource) -> ColoramaInput<'static> {
    ColoramaInput {
        phase_from: phase,
        add: None,
        add_from: PhaseSource::Zero,
        combine: PhaseCombine::Wrap,
        phase_shift: 0.0,
        repetitions: 1.0,
        stops: None,
        interpolate: true,
        modify: ModifyAttr::None,
        modify_alpha: false,
        change_empty: false,
        matching_color: [0.0, 0.0, 0.0],
        matching_tolerance: 1.0,
        matching_softness: 0.0,
        matching_off: true,
        mask: None,
        mask_luma: false,
        composite_over: false,
        blend: 0.0,
    }
}

#[test]
fn named_controls_reject_the_wrong_value_type() {
    let mut values = ParamValues::new();
    values.push(ParamValue::Checkbox(true));
    values.push(ParamValue::Float(0.0));
    assert!(EffectStack::new()
        .add(EffectKind::BrightnessContrast, values)
        .is_err());
    let mut point_values = ParamValues::new();
    point_values.push(ParamValue::Popup(0));
    point_values.push(ParamValue::Point([4.0, 5.0]));
    point_values.push(ParamValue::Point([1.0, 2.0]));
    point_values.push(ParamValue::Point([6.0, 7.0]));
    let info = EffectKind::ColorStabilizer.spec();
    let args = control::Args::new(&info.controls, &point_values).unwrap();
    assert_eq!(args.point(1).unwrap(), [4.0, 5.0]);
    let mut layer_values = ParamValues::new();
    layer_values.push(ParamValue::Layer(Some(LayerId::new(3))));
    layer_values.push(ParamValue::Popup(0));
    let blend = EffectKind::Blend.spec();
    let blend_args = control::Args::new(&blend.controls, &layer_values).unwrap();
    assert_eq!(blend_args.layer(0).unwrap(), Some(LayerId::new(3)));
}

#[test]
fn curves_identity_is_the_confirmed_default() {
    let frame = solid(1, 1, [0.3, 0.6, 0.9, 1.0]).to_frame(BitsPerChannel::Bpc32);
    let params = values(&[ParamValue::Popup(0)]);
    let mut output = frame.clone();
    EffectKind::Curves
        .apply_cpu(&cpu(&frame, &params, BitsPerChannel::Bpc32), &mut output)
        .unwrap();
    assert!(
        image::mean_abs(
            &Image::from_frame(&frame).unwrap(),
            &Image::from_frame(&output).unwrap()
        ) < 1.0e-5
    );
}

#[test]
fn cross_blur_apply_uses_effect_api_values() {
    let mut image = solid(4, 2, [0.0, 0.0, 0.0, 1.0]);
    image.set(3, 0, [1.0, 1.0, 1.0, 1.0]);
    image.set(3, 1, [1.0, 1.0, 1.0, 1.0]);
    let frame = image.to_frame(BitsPerChannel::Bpc32);
    let params = values(&[
        ParamValue::Float(2.0),
        ParamValue::Float(0.0),
        ParamValue::Popup(0),
        ParamValue::Checkbox(true),
    ]);
    let mut output = frame.clone();
    EffectKind::CcCrossBlur
        .apply(
            &cpu(&frame, &params, BitsPerChannel::Bpc32),
            &Host::empty(),
            &mut output,
        )
        .unwrap();
    assert!(image::mean_abs(&Image::from_frame(&output).unwrap(), &image) > 1.0e-3);
}

#[test]
fn buttons_are_set_frame_and_swap_colors() {
    assert_eq!(EffectKind::ColorStabilizer.spec().buttons, ["Set Frame"]);
    assert_eq!(EffectKind::Tint.spec().buttons, ["Swap Colors"]);
    assert!(EffectKind::GaussianBlur.spec().gpu);
    assert!(!EffectKind::BilateralBlur.spec().gpu);
}
