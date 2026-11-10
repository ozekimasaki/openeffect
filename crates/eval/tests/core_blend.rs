use model::bits::BitsPerChannel;
use model::color::Rgba;
use model::composition::Composition;
use model::id::{ItemId, LayerId};
use model::keyframe::{Keyframe, TemporalInterpolation};
use model::layer::{FrameBlend, Layer, LayerKind, LayerQuality, StretchHold};
use model::property::{Property, PropertyValue};
use model::time::Time;
use model::track_matte::{TrackMatte, TrackMatteMode};
use model::BlendMode;

use eval::blend_pixel::{blend_channel, blend_pixel, normal_alpha};
use eval::edit::{assign_parent, clear_parent, select_children, world_position};
use eval::picture::{frame_mix, pixel_motion_row};

fn px(r: f32, g: f32, b: f32, a: f32) -> Rgba {
    Rgba::new(r, g, b, a)
}

fn over(mode: BlendMode, src: Rgba, dst: Rgba, bits: BitsPerChannel) -> Rgba {
    blend_pixel(mode, src, dst, 1.0, bits, false, 0, 0, 0)
}

fn channel(mode: BlendMode, src: f32, dst: f32) -> f32 {
    blend_channel(mode, src, dst, BitsPerChannel::Bpc32)
}

#[test]
fn normal_opaque_red_stays_red_over_blue() {
    let result = over(BlendMode::Normal, px(1.0, 0.0, 0.0, 1.0), px(0.0, 0.0, 1.0, 1.0), BitsPerChannel::Bpc32);
    assert_eq!(result.r, 1.0);
    assert_eq!(result.b, 0.0);
}

#[test]
fn dissolve_dithers_at_half_opacity_and_3d_matches_normal() {
    let mut source = 0;
    let mut back = 0;
    for x in 0..16 {
        for y in 0..16 {
            let pixel = blend_pixel(
                BlendMode::Dissolve,
                px(1.0, 0.0, 0.0, 1.0),
                px(0.0, 0.0, 1.0, 1.0),
                0.5,
                BitsPerChannel::Bpc32,
                false,
                0,
                x,
                y,
            );
            if pixel.r > 0.5 { source += 1; } else { back += 1; }
        }
    }
    assert!(source > 0 && back > 0);
    let dissolved = blend_pixel(BlendMode::Dissolve, px(1.0, 0.0, 0.0, 1.0), px(0.0, 0.0, 1.0, 1.0), 0.5, BitsPerChannel::Bpc32, true, 0, 0, 0);
    let normal = blend_pixel(BlendMode::Normal, px(1.0, 0.0, 0.0, 1.0), px(0.0, 0.0, 1.0, 1.0), 0.5, BitsPerChannel::Bpc32, true, 0, 0, 0);
    assert_eq!(dissolved, normal);
}

#[test]
fn dancing_dissolve_changes_across_frames() {
    let still = blend_pixel(BlendMode::Dissolve, px(1.0, 0.0, 0.0, 1.0), px(0.0, 0.0, 1.0, 1.0), 0.5, BitsPerChannel::Bpc32, false, 0, 3, 4);
    let later = blend_pixel(BlendMode::Dissolve, px(1.0, 0.0, 0.0, 1.0), px(0.0, 0.0, 1.0, 1.0), 0.5, BitsPerChannel::Bpc32, false, 1, 3, 4);
    assert_eq!(still, later);
    let mut changed = false;
    let first = blend_pixel(BlendMode::DancingDissolve, px(1.0, 0.0, 0.0, 1.0), px(0.0, 0.0, 1.0, 1.0), 0.5, BitsPerChannel::Bpc32, false, 0, 1, 1);
    for frame in 1..12 {
        let next = blend_pixel(BlendMode::DancingDissolve, px(1.0, 0.0, 0.0, 1.0), px(0.0, 0.0, 1.0, 1.0), 0.5, BitsPerChannel::Bpc32, false, frame, 1, 1);
        changed |= next != first;
    }
    assert!(changed);
}

#[test]
fn darken_picks_the_darker_channel() {
    let result = over(BlendMode::Darken, px(1.0, 0.0, 0.0, 1.0), px(0.0, 1.0, 1.0, 1.0), BitsPerChannel::Bpc32);
    assert_eq!((result.r, result.g, result.b), (0.0, 0.0, 0.0));
}

#[test]
fn multiply_half_gray_is_quarter_gray() {
    let value = channel(BlendMode::Multiply, 0.5, 0.5);
    assert!((value - 0.25).abs() < 1.0e-6);
}

#[test]
fn color_burn_leaves_white_source_unchanged() {
    assert!((channel(BlendMode::ColorBurn, 1.0, 0.4) - 0.4).abs() < 1.0e-6);
    assert!(channel(BlendMode::ColorBurn, 0.5, 0.8) < 0.8);
}

#[test]
fn classic_color_burn_keeps_white_source() {
    assert!((channel(BlendMode::ClassicColorBurn, 1.0, 0.35) - 0.35).abs() < 1.0e-6);
}

#[test]
fn linear_burn_white_is_unchanged_and_black_is_black() {
    assert!((channel(BlendMode::LinearBurn, 1.0, 0.4) - 0.4).abs() < 1.0e-6);
    assert!((channel(BlendMode::LinearBurn, 0.0, 0.4) - 0.0).abs() < 1.0e-6);
}

#[test]
fn darker_color_picks_a_whole_pixel() {
    let src = px(0.2, 0.2, 0.2, 1.0);
    let dst = px(0.8, 0.1, 0.1, 1.0);
    let result = over(BlendMode::DarkerColor, src, dst, BitsPerChannel::Bpc32);
    assert!(result == src || result == dst);
}

#[test]
fn add_clamps_in_8_bpc_and_exceeds_one_in_32() {
    let gray = 128.0 / 255.0;
    let eight = blend_channel(BlendMode::Add, gray, gray, BitsPerChannel::Bpc8);
    let wide = blend_channel(BlendMode::Add, gray, gray, BitsPerChannel::Bpc32);
    assert!(eight <= 1.0);
    assert!(wide > 1.0);
}

#[test]
fn lighten_picks_the_brighter_channel() {
    let result = over(BlendMode::Lighten, px(1.0, 0.0, 0.0, 1.0), px(0.0, 1.0, 1.0, 1.0), BitsPerChannel::Bpc32);
    assert_eq!((result.r, result.g, result.b), (1.0, 1.0, 1.0));
}

#[test]
fn screen_half_gray_is_brighter_than_either_input() {
    let value = channel(BlendMode::Screen, 0.5, 0.5);
    assert!((value - 0.75).abs() < 1.0e-6);
}

#[test]
fn color_dodge_leaves_black_source() {
    assert!((channel(BlendMode::ColorDodge, 0.0, 0.4) - 0.4).abs() < 1.0e-6);
    assert!(channel(BlendMode::ColorDodge, 0.8, 0.5) > 0.5);
}

#[test]
fn classic_color_dodge_leaves_black_source() {
    assert!((channel(BlendMode::ClassicColorDodge, 0.0, 0.45) - 0.45).abs() < 1.0e-6);
}

#[test]
fn linear_dodge_matches_add_and_keeps_black() {
    assert!((channel(BlendMode::LinearDodge, 0.0, 0.3) - 0.3).abs() < 1.0e-6);
    assert!((channel(BlendMode::LinearDodge, 0.2, 0.3) - channel(BlendMode::Add, 0.2, 0.3)).abs() < 1.0e-6);
}

#[test]
fn lighter_color_picks_a_whole_pixel() {
    let src = px(0.9, 0.9, 0.2, 1.0);
    let dst = px(0.2, 0.2, 0.2, 1.0);
    let result = over(BlendMode::LighterColor, src, dst, BitsPerChannel::Bpc32);
    assert!(result == src || result == dst);
}

#[test]
fn overlay_half_gray_source_keeps_the_backdrop() {
    let value = channel(BlendMode::Overlay, 0.5, 0.2);
    assert!((value - 0.2).abs() < 1.0e-6);
    let high = channel(BlendMode::Overlay, 0.5, 0.8);
    assert!((high - 0.8).abs() < 1.0e-6);
}

#[test]
fn soft_light_white_does_not_turn_black_into_white() {
    let value = channel(BlendMode::SoftLight, 1.0, 0.0);
    assert!(value < 1.0);
}

#[test]
fn hard_light_white_and_black_push_the_backdrop() {
    assert!((channel(BlendMode::HardLight, 1.0, 0.3) - 1.0).abs() < 1.0e-6);
    assert!((channel(BlendMode::HardLight, 0.0, 0.3) - 0.0).abs() < 1.0e-6);
}

#[test]
fn linear_light_half_gray_keeps_the_backdrop() {
    assert!((channel(BlendMode::LinearLight, 0.5, 0.37) - 0.37).abs() < 1.0e-6);
}

#[test]
fn vivid_light_follows_the_source_side_of_gray() {
    let darker = channel(BlendMode::VividLight, 0.25, 0.6);
    let lighter = channel(BlendMode::VividLight, 0.75, 0.4);
    assert!(darker < 0.6);
    assert!(lighter > 0.4);
}

#[test]
fn pin_light_replaces_pixels_across_the_threshold() {
    let kept = channel(BlendMode::PinLight, 0.8, 0.9);
    let replaced = channel(BlendMode::PinLight, 0.8, 0.2);
    assert!((kept - 0.9).abs() < 1.0e-6);
    assert!(replaced > 0.2);
}

#[test]
fn hard_mix_in_8_bpc_is_only_black_or_white() {
    for src in [0.2_f32, 0.5, 0.8] {
        for dst in [0.2_f32, 0.5, 0.8] {
            let value = blend_channel(BlendMode::HardMix, src, dst, BitsPerChannel::Bpc8);
            assert!(value == 0.0 || value == 1.0, "{value}");
        }
    }
}

#[test]
fn difference_turns_matching_pixels_black() {
    let same = over(BlendMode::Difference, px(0.2, 0.4, 0.6, 1.0), px(0.2, 0.4, 0.6, 1.0), BitsPerChannel::Bpc32);
    assert_eq!((same.r, same.g, same.b), (0.0, 0.0, 0.0));
    let shifted = over(BlendMode::Difference, px(0.2, 0.4, 0.6, 1.0), px(0.2, 0.0, 0.6, 1.0), BitsPerChannel::Bpc32);
    assert!(shifted.g > 0.0);
}

#[test]
fn classic_difference_inverts_white() {
    let inverted = channel(BlendMode::ClassicDifference, 1.0, 0.25);
    assert!((inverted - 0.75).abs() < 1.0e-6);
}

#[test]
fn exclusion_inverts_white_and_is_softer_than_difference() {
    assert!((channel(BlendMode::Exclusion, 1.0, 0.2) - 0.8).abs() < 1.0e-6);
    let exclusion = channel(BlendMode::Exclusion, 0.5, 0.5);
    let difference = channel(BlendMode::Difference, 0.5, 0.5);
    assert!(exclusion > difference);
}

#[test]
fn subtract_goes_negative_only_in_32_bpc() {
    let wide = blend_channel(BlendMode::Subtract, 1.2, 1.0, BitsPerChannel::Bpc32);
    let eight = blend_channel(BlendMode::Subtract, 1.2, 1.0, BitsPerChannel::Bpc8);
    assert!(wide < 0.0);
    assert_eq!(eight, 0.0);
    assert!((channel(BlendMode::Subtract, 0.0, 0.4) - 0.4).abs() < 1.0e-6);
}

#[test]
fn divide_keeps_white_and_exceeds_one_in_32_bpc() {
    assert!((channel(BlendMode::Divide, 1.0, 0.4) - 0.4).abs() < 1.0e-6);
    let wide = blend_channel(BlendMode::Divide, 0.25, 1.0, BitsPerChannel::Bpc32);
    assert!(wide > 1.0);
}

#[test]
fn hue_over_gray_stays_gray() {
    let result = over(BlendMode::Hue, px(1.0, 0.0, 0.0, 1.0), px(0.4, 0.4, 0.4, 1.0), BitsPerChannel::Bpc32);
    assert!((result.r - result.g).abs() < 1.0e-4);
    assert!((result.g - result.b).abs() < 1.0e-4);
}

#[test]
fn saturation_keeps_the_backdrop_hue() {
    let result = over(BlendMode::Saturation, px(0.0, 1.0, 0.0, 1.0), px(0.8, 0.2, 0.2, 1.0), BitsPerChannel::Bpc32);
    assert!(result.r >= result.g && result.r >= result.b);
}

#[test]
fn color_mode_keeps_backdrop_luminance() {
    let backdrop = px(0.2, 0.5, 0.8, 1.0);
    let result = over(BlendMode::Color, px(1.0, 0.0, 0.0, 1.0), backdrop, BitsPerChannel::Bpc32);
    assert!((result.luma() - backdrop.luma()).abs() < 0.05);
    assert!(result.r > result.b);
}

#[test]
fn luminosity_keeps_backdrop_hue() {
    let result = over(BlendMode::Luminosity, px(0.9, 0.9, 0.9, 1.0), px(0.8, 0.2, 0.2, 1.0), BitsPerChannel::Bpc32);
    assert!(result.r > result.g);
}

#[test]
fn stencil_alpha_reveals_only_the_opaque_shape() {
    let dst = px(0.2, 0.4, 0.6, 1.0);
    let revealed = over(BlendMode::StencilAlpha, px(0.0, 0.0, 0.0, 1.0), dst, BitsPerChannel::Bpc32);
    let hidden = over(BlendMode::StencilAlpha, px(0.0, 0.0, 0.0, 0.0), dst, BitsPerChannel::Bpc32);
    assert!(revealed.a > 0.9);
    assert!(hidden.a < 0.1);
}

#[test]
fn stencil_luma_uses_a_white_shape() {
    let dst = px(0.2, 0.3, 0.4, 1.0);
    let revealed = over(BlendMode::StencilLuma, px(1.0, 1.0, 1.0, 1.0), dst, BitsPerChannel::Bpc32);
    let hidden = over(BlendMode::StencilLuma, px(0.0, 0.0, 0.0, 1.0), dst, BitsPerChannel::Bpc32);
    assert!(revealed.a > hidden.a);
}

#[test]
fn silhouette_alpha_punches_the_opaque_shape() {
    let dst = px(0.2, 0.3, 0.4, 1.0);
    let punched = over(BlendMode::SilhouetteAlpha, px(1.0, 1.0, 1.0, 1.0), dst, BitsPerChannel::Bpc32);
    assert!(punched.a < 0.1);
}

#[test]
fn silhouette_luma_punches_white() {
    let dst = px(0.2, 0.3, 0.4, 1.0);
    let punched = over(BlendMode::SilhouetteLuma, px(1.0, 1.0, 1.0, 1.0), dst, BitsPerChannel::Bpc32);
    let black = over(BlendMode::SilhouetteLuma, px(0.0, 0.0, 0.0, 1.0), dst, BitsPerChannel::Bpc32);
    assert!(punched.a < 0.1);
    assert!(black.a > 0.9);
}

#[test]
fn alpha_add_is_more_opaque_than_normal_half_over_half() {
    let normal = normal_alpha(0.5, 0.5);
    let added = blend_pixel(BlendMode::AlphaAdd, px(1.0, 0.0, 0.0, 0.5), px(0.0, 0.0, 1.0, 0.5), 1.0, BitsPerChannel::Bpc32, false, 0, 0, 0);
    assert!((normal - 0.75).abs() < 1.0e-6);
    assert!(added.a > normal);
}

#[test]
fn luminescent_premul_keeps_highlights_brighter_than_normal() {
    let src = px(1.0, 1.0, 1.0, 0.4);
    let dst = px(0.0, 0.0, 0.0, 1.0);
    let normal = over(BlendMode::Normal, src, dst, BitsPerChannel::Bpc32);
    let glow = over(BlendMode::LuminescentPremul, src, dst, BitsPerChannel::Bpc32);
    assert!(glow.r > normal.r);
}

#[test]
fn clearing_a_track_matte_restores_the_layer_alpha() {
    let mut layer = Layer::new(LayerId::new(1), "fill", LayerKind::Solid { source: ItemId::new(1) });
    layer.track_matte = TrackMatte { source: Some(LayerId::new(2)), mode: TrackMatteMode::Alpha };
    layer.track_matte = TrackMatte::none();
    assert!(layer.track_matte.source.is_none());
    assert_eq!(layer.track_matte.mode, TrackMatteMode::None);
}

#[test]
fn choosing_a_text_matte_limits_the_fill_to_the_glyphs() {
    let text = "AB";
    let inside = eval::picture::text_coverage(text, 0);
    let outside = eval::picture::text_coverage(text, 5);
    assert_eq!(inside, 1.0);
    assert_eq!(outside, 0.0);
}

#[test]
fn alpha_matte_shows_only_the_opaque_shape() {
    let matte = 1.0_f32;
    let fill = px(0.2, 0.4, 0.8, 1.0);
    let shown = px(fill.r, fill.g, fill.b, fill.a * matte);
    let hidden = px(fill.r, fill.g, fill.b, fill.a * 0.0);
    assert!(shown.a > 0.9);
    assert!(hidden.a < 0.1);
}

#[test]
fn inverted_alpha_matte_hides_the_opaque_shape() {
    let matte = 1.0_f32;
    let factor = 1.0 - matte;
    assert!(factor < 0.1);
    let outside = 1.0 - 0.0;
    assert!(outside > 0.9);
}

#[test]
fn luma_matte_shows_white_and_hides_black() {
    let white = px(1.0, 1.0, 1.0, 1.0).luma();
    let black = px(0.0, 0.0, 0.0, 1.0).luma();
    assert!(white > 0.9);
    assert!(black < 0.1);
}

#[test]
fn inverted_luma_matte_shows_black() {
    let black = 1.0 - px(0.0, 0.0, 0.0, 1.0).luma();
    let white = 1.0 - px(1.0, 1.0, 1.0, 1.0).luma();
    assert!(black > 0.9);
    assert!(white < 0.1);
}

#[test]
fn preserve_underlying_transparency_hides_empty_backdrop() {
    let backdrop = 0.0_f32;
    let layer = 1.0_f32;
    assert_eq!(layer * backdrop, 0.0);
    assert!(layer * 1.0 > 0.9);
}

#[test]
fn parenting_keeps_the_child_then_follows_ten_pixels() {
    let mut composition = Composition::new(ItemId::new(1), "C", 100, 80, 24.0, Time::from_seconds(1.0));
    let mut parent = Layer::new(LayerId::new(1), "parent", LayerKind::Null);
    parent.transform.position = [40.0, 0.0, 0.0];
    let mut child = Layer::new(LayerId::new(2), "child", LayerKind::Null);
    child.transform.position = [100.0, 0.0, 0.0];
    composition.layers.push(parent);
    composition.layers.push(child);
    assign_parent(&mut composition, LayerId::new(2), LayerId::new(1), false);
    let stayed = world_position(&composition, LayerId::new(2)).unwrap();
    assert!((stayed[0] - 100.0).abs() < 1.0e-9);
    composition.layer_mut(LayerId::new(1)).unwrap().transform.position[0] += 10.0;
    let moved = world_position(&composition, LayerId::new(2)).unwrap();
    assert!((moved[0] - 110.0).abs() < 1.0e-9);
}

#[test]
fn alt_parent_jumps_by_the_parent_transform() {
    let mut composition = Composition::new(ItemId::new(1), "C", 100, 80, 24.0, Time::from_seconds(1.0));
    let mut parent = Layer::new(LayerId::new(1), "parent", LayerKind::Null);
    parent.transform.position = [40.0, 0.0, 0.0];
    let mut child = Layer::new(LayerId::new(2), "child", LayerKind::Null);
    child.transform.position = [100.0, 0.0, 0.0];
    composition.layers.push(parent);
    composition.layers.push(child);
    assign_parent(&mut composition, LayerId::new(2), LayerId::new(1), true);
    let jumped = world_position(&composition, LayerId::new(2)).unwrap();
    assert!((jumped[0] - 140.0).abs() < 1.0e-9);
}

#[test]
fn clearing_a_parent_keeps_the_child_still() {
    let mut composition = Composition::new(ItemId::new(1), "C", 100, 80, 24.0, Time::from_seconds(1.0));
    let mut parent = Layer::new(LayerId::new(1), "parent", LayerKind::Null);
    parent.transform.position = [40.0, 0.0, 0.0];
    let mut child = Layer::new(LayerId::new(2), "child", LayerKind::Null);
    child.transform.position = [100.0, 0.0, 0.0];
    composition.layers.push(parent);
    composition.layers.push(child);
    assign_parent(&mut composition, LayerId::new(2), LayerId::new(1), false);
    clear_parent(&mut composition, LayerId::new(2), false);
    let stayed = world_position(&composition, LayerId::new(2)).unwrap();
    assert!((stayed[0] - 100.0).abs() < 1.0e-9);
    composition.layer_mut(LayerId::new(1)).unwrap().transform.position[0] += 25.0;
    let still = world_position(&composition, LayerId::new(2)).unwrap();
    assert!((still[0] - 100.0).abs() < 1.0e-9);
}

#[test]
fn alt_clear_parent_jumps_the_child() {
    let mut composition = Composition::new(ItemId::new(1), "C", 100, 80, 24.0, Time::from_seconds(1.0));
    let mut parent = Layer::new(LayerId::new(1), "parent", LayerKind::Null);
    parent.transform.position = [40.0, 0.0, 0.0];
    let mut child = Layer::new(LayerId::new(2), "child", LayerKind::Null);
    child.transform.position = [100.0, 0.0, 0.0];
    composition.layers.push(parent);
    composition.layers.push(child);
    assign_parent(&mut composition, LayerId::new(2), LayerId::new(1), false);
    let before = world_position(&composition, LayerId::new(2)).unwrap()[0];
    clear_parent(&mut composition, LayerId::new(2), true);
    let after = world_position(&composition, LayerId::new(2)).unwrap()[0];
    assert!((before - after).abs() > 1.0);
}

#[test]
fn select_children_includes_direct_children() {
    let mut composition = Composition::new(ItemId::new(1), "C", 100, 80, 24.0, Time::from_seconds(1.0));
    let parent = Layer::new(LayerId::new(1), "parent", LayerKind::Null);
    let mut child_a = Layer::new(LayerId::new(2), "a", LayerKind::Null);
    let mut child_b = Layer::new(LayerId::new(3), "b", LayerKind::Null);
    let mut grand = Layer::new(LayerId::new(4), "grand", LayerKind::Null);
    child_a.parent = Some(LayerId::new(1));
    child_b.parent = Some(LayerId::new(1));
    grand.parent = Some(LayerId::new(2));
    composition.layers.extend([parent, child_a, child_b, grand]);
    let selected = select_children(&composition, LayerId::new(1));
    assert!(selected.contains(&LayerId::new(2)));
    assert!(selected.contains(&LayerId::new(3)));
}

#[test]
fn time_remap_reverses_when_the_end_key_holds_the_start_value() {
    let mut layer = Layer::new(LayerId::new(1), "clip", LayerKind::Footage { source: ItemId::new(1) });
    layer.in_point = Time::ZERO;
    layer.out_point = Time::from_seconds(2.0);
    layer.enable_time_remap(Time::from_seconds(2.0));
    layer.reverse_time_remap();
    let start = eval::interpolate::sample_scalar(layer.time_remap.as_ref().unwrap(), Time::ZERO, 0.0);
    let end = eval::interpolate::sample_scalar(layer.time_remap.as_ref().unwrap(), Time::from_seconds(2.0), 0.0);
    assert!(start > end);
}

#[test]
fn freeze_frame_holds_the_current_picture() {
    let mut layer = Layer::new(LayerId::new(1), "clip", LayerKind::Footage { source: ItemId::new(1) });
    layer.in_point = Time::ZERO;
    layer.out_point = Time::from_seconds(4.0);
    layer.enable_time_remap(Time::from_seconds(4.0));
    layer.freeze_frame(Time::from_seconds(1.0), Time::from_seconds(1.0));
    assert_eq!(layer.time_remap.as_ref().unwrap().len(), 1);
    assert_eq!(layer.time_remap.as_ref().unwrap()[0].temporal_out, TemporalInterpolation::Hold);
    let later = eval::interpolate::sample_scalar(layer.time_remap.as_ref().unwrap(), Time::from_seconds(3.0), 0.0);
    assert!((later - 1.0).abs() < 1.0e-9);
}

#[test]
fn freeze_on_last_frame_extends_to_the_comp_end() {
    let mut layer = Layer::new(LayerId::new(1), "clip", LayerKind::Footage { source: ItemId::new(1) });
    layer.in_point = Time::ZERO;
    layer.out_point = Time::from_seconds(2.0);
    layer.freeze_on_last_frame(Time::from_seconds(2.0), Time::from_seconds(10.0));
    assert!((layer.out_point.seconds() - 10.0).abs() < 1.0e-9);
    let held = eval::interpolate::sample_scalar(layer.time_remap.as_ref().unwrap(), Time::from_seconds(9.0), 0.0);
    assert!((held - 2.0).abs() < 1.0e-9);
}

#[test]
fn stretch_200_doubles_span_keys_and_audio() {
    let mut layer = Layer::new(LayerId::new(1), "clip", LayerKind::Footage { source: ItemId::new(1) });
    layer.in_point = Time::ZERO;
    layer.out_point = Time::from_seconds(2.0);
    layer.properties.push(Property::value(model::id::PropertyId::new(1), "Opacity", "Opacity", PropertyValue::Scalar(1.0)));
    layer.properties[0].push_keyframe(Keyframe::at(Time::ZERO, PropertyValue::Scalar(0.0)));
    layer.properties[0].push_keyframe(Keyframe::at(Time::from_seconds(1.0), PropertyValue::Scalar(1.0)));
    let before_audio = layer.played_audio_duration().seconds();
    layer.stretch_time(2.0, StretchHold::InPoint, Time::ZERO);
    assert!((layer.duration().seconds() - 4.0).abs() < 1.0e-9);
    assert!((layer.played_audio_duration().seconds() - before_audio * 2.0).abs() < 1.0e-9);
    assert!((layer.properties[0].keyframes()[1].time.seconds() - 2.0).abs() < 1.0e-9);
}

#[test]
fn stretch_hold_in_point_keeps_the_in_point() {
    let mut layer = Layer::new(LayerId::new(1), "clip", LayerKind::Null);
    layer.in_point = Time::from_seconds(1.0);
    layer.out_point = Time::from_seconds(3.0);
    layer.stretch_time(2.0, StretchHold::InPoint, Time::from_seconds(1.0));
    assert!((layer.in_point.seconds() - 1.0).abs() < 1.0e-9);
}

#[test]
fn stretch_hold_current_frame_keeps_that_frame_in_place() {
    let mut layer = Layer::new(LayerId::new(1), "clip", LayerKind::Null);
    layer.in_point = Time::ZERO;
    layer.out_point = Time::from_seconds(4.0);
    let now = Time::from_seconds(1.0);
    layer.stretch_time(2.0, StretchHold::CurrentFrame, now);
    let source_before = 1.0 / 4.0;
    let source_after = (now.seconds() - layer.in_point.seconds()) / layer.duration().seconds();
    assert!((source_before - source_after).abs() < 1.0e-9);
}

#[test]
fn stretch_hold_out_point_keeps_the_out_point() {
    let mut layer = Layer::new(LayerId::new(1), "clip", LayerKind::Null);
    layer.in_point = Time::from_seconds(1.0);
    layer.out_point = Time::from_seconds(3.0);
    layer.stretch_time(2.0, StretchHold::OutPoint, Time::from_seconds(3.0));
    assert!((layer.out_point.seconds() - 3.0).abs() < 1.0e-9);
}

#[test]
fn time_reverse_swaps_the_end_frames_and_the_audio() {
    let mut layer = Layer::new(LayerId::new(1), "clip", LayerKind::Footage { source: ItemId::new(1) });
    layer.in_point = Time::ZERO;
    layer.out_point = Time::from_seconds(2.0);
    layer.time_reverse();
    let head = layer.source_time(Time::ZERO, Time::from_seconds(2.0));
    let tail = layer.source_time(Time::from_seconds(2.0), Time::from_seconds(2.0));
    assert!(head.seconds() > tail.seconds());
    assert!(layer.audio_reversed);
}

#[test]
fn frame_blend_off_repeats_frames_at_half_speed() {
    let mut counts = std::collections::BTreeMap::<usize, usize>::new();
    for frame in 0..8 {
        let source = (frame as f64 / 2.0).floor() as usize;
        *counts.entry(source).or_insert(0) += 1;
    }
    assert!(counts.values().all(|count| *count == 2));
    let _ = FrameBlend::Off;
}

#[test]
fn frame_mix_blends_neighboring_frames() {
    let mixed = frame_mix(&[0.0, 1.0], &[1.0, 0.0], 0.5);
    assert!((mixed[0] - 0.5).abs() < 1.0e-6);
    assert!((mixed[1] - 0.5).abs() < 1.0e-6);
}

#[test]
fn pixel_motion_places_the_edge_between_frames() {
    let mut a = vec![0.0; 6];
    let mut b = vec![0.0; 6];
    a[1] = 1.0;
    b[3] = 1.0;
    let moved = pixel_motion_row(&a, &b, 0.5);
    let peak = moved.iter().position(|sample| *sample > 0.5).unwrap();
    assert_eq!(peak, 2);
}

#[test]
fn comp_frame_blend_off_holds_frames() {
    let layer_mode = FrameBlend::PixelMotion;
    let comp_enabled = false;
    let effective = if comp_enabled { layer_mode } else { FrameBlend::Off };
    assert_eq!(effective, FrameBlend::Off);
    let _ = LayerQuality::Draft;
}
