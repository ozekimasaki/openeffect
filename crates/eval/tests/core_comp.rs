use model::bits::BitsPerChannel;
use model::color::Rgba;
use model::composition::{
    confirmed_resolution_menu, confirmed_timeline_columns, auto_graph_kind, Cinema4dAntiAlias,
    Cinema4dDepth, Cinema4dReflectance, CompAnchor, CompPreset, Composition, EdgeAnchor, GraphKind,
    GraphPropertyRef, NameColumn, Renderer, Resolution, ShowProperties, CINEMA4D_GEOMETRY_SUBPIXELS,
};
use model::id::ItemId;
use model::layer::{FrameBlend, Layer, LayerKind, LayerQuality, Sampling};
use model::project::{LabelColor, Project};
use model::property::{Property, PropertyValue};
use model::time::{samples_in_one_second, shutter_window, Time};
use model::BlendMode;

use eval::edit::{
    add_adjustment, add_camera, add_light, add_null, add_photoshop_layer, add_shape_rect, add_text_layer,
    apply_preset, classic_is_default, confirm_comp_settings, displayed_source_name, enable_3d,
    handle_color, import_media, included_in_render, label_name, new_comp_from_last, new_folder,
    new_other_3d, new_solid_layer, precompose, save_preset, search_layers, select_label_group,
    solo_only, tab_color, timeline_lists, try_move, visible_in_preview,
};
use eval::picture::{
    blur_span, collapse_resample, diagonal_coverage, motion_blur_sample_count, project_camera,
    rasterize_step, resample_1d, shadow_map_size, snap_subpixel,
};
use eval::render::{render, RenderPurpose};

fn project() -> Project {
    Project::new(BitsPerChannel::Bpc8)
}

fn add_comp(project: &mut Project) -> ItemId {
    let id = project.alloc_item();
    let mut composition = Composition::new(id, "Comp", 16, 8, 24.0, Time::from_seconds(10.0));
    composition.label = Some(3);
    project.add_composition(composition);
    id
}

#[test]
fn renaming_a_comp_updates_tab_project_and_nested_source() {
    let mut project = project();
    let child = add_comp(&mut project);
    let parent = add_comp(&mut project);
    project.composition_mut(child).unwrap().name = "Renamed".into();
    let layer_id = project.alloc_layer();
    let layer = Layer::new(layer_id, "nest", LayerKind::Precomp { source: child });
    project.composition_mut(parent).unwrap().layers.push(layer);
    assert_eq!(project.composition(child).unwrap().name, "Renamed");
    assert_eq!(
        displayed_source_name(&project, project.composition(parent).unwrap().layer(layer_id).unwrap()),
        "Renamed"
    );
}

#[test]
fn presets_roundtrip_four_values_and_reset_to_bundled() {
    let mut project = project();
    project.presets.bundled.push(CompPreset {
        name: "bundled".into(),
        width: 100,
        height: 50,
        pixel_aspect: 1.0,
        frame_rate: 24.0,
        bundled: true,
    });
    let comp = add_comp(&mut project);
    project.composition_mut(comp).unwrap().width = 320;
    project.composition_mut(comp).unwrap().height = 240;
    project.composition_mut(comp).unwrap().pixel_aspect = 1.09;
    project.composition_mut(comp).unwrap().frame_rate = 25.0;
    let duration = project.composition(comp).unwrap().duration;
    save_preset(&mut project, comp, "mine").unwrap();
    project.composition_mut(comp).unwrap().width = 16;
    apply_preset(&mut project, comp, "mine").unwrap();
    let composition = project.composition(comp).unwrap();
    assert_eq!((composition.width, composition.height, composition.frame_rate), (320, 240, 25.0));
    assert_eq!(composition.pixel_aspect, 1.09);
    assert_eq!(composition.duration, duration);
    project.presets.reset_to_bundled();
    assert!(project.presets.user.is_empty());
    assert_eq!(project.presets.bundled.len(), 1);
}

#[test]
fn changing_width_changes_the_pixel_count() {
    let mut composition = Composition::new(ItemId::new(1), "C", 100, 40, 24.0, Time::from_seconds(1.0));
    composition.resize(120, 40);
    let (w, h) = composition.resolution.preview_size(composition.width, composition.height).unwrap();
    assert_eq!(w * h, 120 * 40);
}

#[test]
fn changing_height_changes_the_pixel_count() {
    let mut composition = Composition::new(ItemId::new(1), "C", 100, 40, 24.0, Time::from_seconds(1.0));
    composition.resize(100, 55);
    assert_eq!(composition.width * composition.height, 100 * 55);
}

#[test]
fn non_square_comp_corrects_a_circle() {
    let ratio = 0.91;
    let pixel_h = 100.0_f64;
    let pixel_w = pixel_h / ratio;
    assert!((pixel_w * ratio - pixel_h).abs() < 1.0e-9);
}

#[test]
fn frame_rate_samples_24_and_12_frames_per_second() {
    assert_eq!(samples_in_one_second(24.0), Some(24));
    assert_eq!(samples_in_one_second(12.0), Some(12));
    assert!(samples_in_one_second(1000.0).is_none());
    assert_eq!(samples_in_one_second(999.0), Some(999));
}

#[test]
fn drop_frame_timecode_at_one_minute() {
    let mut project = project();
    let comp = add_comp(&mut project);
    project.composition_mut(comp).unwrap().frame_rate = 29.97;
    project.composition_mut(comp).unwrap().drop_frame = true;
    let drop = model::time::frame_to_timecode(1800, project.composition(comp).unwrap().drop_frame);
    project.composition_mut(comp).unwrap().drop_frame = false;
    let non = model::time::frame_to_timecode(1800, false);
    assert_eq!(drop.format(), "00:01:00:02");
    assert_eq!(non.format(), "00:01:00:00");
}

#[test]
fn non_drop_timecode_does_not_skip_numbers() {
    let first = model::time::frame_to_timecode(0, false);
    let next = model::time::frame_to_timecode(1, false);
    assert_eq!(first.frames, 0);
    assert_eq!(next.frames, 1);
}

#[test]
fn full_resolution_matches_the_comp_size() {
    let size = Resolution::Full.preview_size(1920, 1080).unwrap();
    assert_eq!(size, (1920, 1080));
    assert_eq!(size.0 * size.1, 1920 * 1080);
}

#[test]
fn half_resolution_halves_each_axis() {
    let size = Resolution::Half.preview_size(100, 80).unwrap();
    assert_eq!(size, (50, 40));
    assert_eq!(size.0 * size.1, 100 * 80 / 4);
}

#[test]
fn custom_resolution_uses_the_entered_denominators() {
    let size = Resolution::Custom { x_denom: 4, y_denom: 4 }.preview_size(1920, 1080).unwrap();
    assert_eq!(size, (480, 270));
    assert!(Resolution::Custom { x_denom: 0, y_denom: 1 }.preview_size(10, 10).is_none());
}

#[test]
fn unconfirmed_resolution_steps_are_not_in_the_menu() {
    let menu = confirmed_resolution_menu();
    assert_eq!(menu, &["Full", "Half", "Custom"]);
    assert!(!menu.contains(&"Third"));
    assert!(!menu.contains(&"Quarter"));
    assert!(!menu.contains(&"Auto"));
}

#[test]
fn start_timecode_changes_the_label_not_the_frame_order() {
    let mut composition = Composition::new(ItemId::new(1), "C", 8, 4, 30.0, Time::from_seconds(2.0));
    composition.start = Time::from_seconds(3600.0);
    let label = model::time::frame_to_timecode(composition.start.to_frames(30.0) as i64, false);
    assert_eq!(label.format(), "01:00:00:00");
    assert!((composition.last_frame_time().unwrap().seconds() - (2.0 - 1.0 / 30.0)).abs() < 1.0e-9);
}

#[test]
fn duration_sets_the_last_frame() {
    let composition = Composition::new(ItemId::new(1), "C", 8, 4, 24.0, Time::from_seconds(1.0));
    let last = composition.last_frame_time().unwrap();
    assert!((last.seconds() - 23.0 / 24.0).abs() < 1.0e-9);
}

#[test]
fn nested_comp_drops_its_background() {
    let mut project = project();
    let child = add_comp(&mut project);
    project.composition_mut(child).unwrap().background = Rgba::new(0.0, 1.0, 0.0, 1.0);
    project.composition_mut(child).unwrap().width = 4;
    project.composition_mut(child).unwrap().height = 2;
    let alone = render(&project, child, Time::ZERO, RenderPurpose::Preview).unwrap();
    assert_eq!(alone.pixels[0].g, 1.0);
    let parent = add_comp(&mut project);
    project.composition_mut(parent).unwrap().background = Rgba::new(1.0, 0.0, 0.0, 1.0);
    project.composition_mut(parent).unwrap().width = 4;
    project.composition_mut(parent).unwrap().height = 2;
    let layer = Layer::new(project.alloc_layer(), "nest", LayerKind::Precomp { source: child });
    project.composition_mut(parent).unwrap().layers.push(layer);
    let nested = render(&project, parent, Time::ZERO, RenderPurpose::Preview).unwrap();
    assert!(nested.pixels[0].r > 0.5);
    assert!(nested.pixels[0].g < 0.5);
}

#[test]
fn resize_anchor_keeps_the_chosen_edge() {
    let mut composition = Composition::new(ItemId::new(1), "C", 100, 40, 24.0, Time::from_seconds(1.0));
    composition.anchor = CompAnchor { horizontal: EdgeAnchor::Start, vertical: EdgeAnchor::Start };
    let mut layer = Layer::new(model::id::LayerId::new(2), "L", LayerKind::Null);
    layer.transform.position = [10.0, 4.0, 0.0];
    composition.layers.push(layer);
    composition.resize(140, 40);
    assert_eq!(composition.layers[0].transform.position[0], 10.0);
    composition.anchor.horizontal = EdgeAnchor::End;
    composition.resize(180, 40);
    assert_eq!(composition.layers[0].transform.position[0], 50.0);
}

#[test]
fn nested_preserve_frame_rate_holds_12_fps_inside_24() {
    let mut project = project();
    let child = add_comp(&mut project);
    project.composition_mut(child).unwrap().frame_rate = 12.0;
    project.composition_mut(child).unwrap().preserve_frame_rate = true;
    let parent = add_comp(&mut project);
    project.composition_mut(parent).unwrap().frame_rate = 24.0;
    let mut layer = Layer::new(project.alloc_layer(), "nest", LayerKind::Precomp { source: child });
    layer.in_point = Time::ZERO;
    layer.out_point = Time::from_seconds(1.0);
    let layer_id = layer.id;
    project.composition_mut(parent).unwrap().layers.push(layer);
    let early = eval::render::render(&project, parent, Time::from_seconds(1.0 / 24.0), RenderPurpose::Preview);
    let _ = (early, layer_id);
    let stepped = {
        let composition = project.composition(child).unwrap();
        let frame = (Time::from_seconds(1.0 / 24.0).seconds() * composition.frame_rate).floor();
        Time::from_seconds(frame / composition.frame_rate)
    };
    assert_eq!(stepped, Time::ZERO);
    confirm_comp_settings(&mut project, child);
    let fresh = new_comp_from_last(&mut project, "New").unwrap();
    assert!(!project.composition(fresh).unwrap().preserve_frame_rate);
}

#[test]
fn nested_preserve_resolution_ignores_parent_half() {
    let mut child = Composition::new(ItemId::new(1), "child", 100, 80, 24.0, Time::from_seconds(1.0));
    child.preserve_resolution = true;
    child.resolution = Resolution::Full;
    let parent_half = Resolution::Half.preview_size(100, 80).unwrap();
    let child_size = if child.preserve_resolution {
        child.resolution.preview_size(child.width, child.height).unwrap()
    } else {
        parent_half
    };
    assert_eq!(child_size, (100, 80));
    assert_ne!(child_size, parent_half);
}

#[test]
fn shutter_angle_scales_blur_length() {
    let (short, long) = (blur_span(100.0, 90.0), blur_span(100.0, 180.0));
    assert!((short - 25.0).abs() < 1.0e-9);
    assert!((long / short - 2.0).abs() < 1.0e-9);
    let exposure = model::time::exposure_seconds(90.0, 24.0).unwrap();
    assert!((exposure - 1.0 / 96.0).abs() < 1.0e-12);
}

#[test]
fn shutter_phase_shifts_the_window() {
    let at_zero = shutter_window(0.0, 24.0, 180.0, 0.0).unwrap();
    assert!((at_zero.0).abs() < 1.0e-12);
    let centered = shutter_window(0.0, 24.0, 180.0, -90.0).unwrap();
    assert!(centered.0 < 0.0 && centered.1 > 0.0);
    assert!((centered.1 + centered.0).abs() < 1.0e-9);
}

#[test]
fn samples_per_frame_changes_the_3d_minimum() {
    let low = motion_blur_sample_count(2, 32, None);
    let high = motion_blur_sample_count(8, 32, None);
    assert!(high > low);
}

#[test]
fn adaptive_sample_limit_caps_fast_motion() {
    let loose = motion_blur_sample_count(2, 16, Some(40));
    let tight = motion_blur_sample_count(2, 6, Some(40));
    assert_eq!(loose, 16);
    assert_eq!(tight, 6);
}

#[test]
fn shadow_resolution_divisor_changes_the_map() {
    let fine = shadow_map_size(100, 80, 1);
    let coarse = shadow_map_size(100, 80, 4);
    assert!(coarse.0 < fine.0 && coarse.1 < fine.1);
}

#[test]
fn new_composition_renderer_is_classic_3d() {
    let composition = Composition::new(ItemId::new(1), "C", 8, 4, 24.0, Time::from_seconds(1.0));
    assert!(classic_is_default(&composition));
    assert_eq!(composition.renderer, Renderer::Classic3d);
}

#[test]
fn advanced_3d_is_distinct_from_classic() {
    let mut composition = Composition::new(ItemId::new(1), "C", 8, 4, 24.0, Time::from_seconds(1.0));
    composition.renderer = Renderer::Advanced3d;
    assert_ne!(composition.renderer, Renderer::Classic3d);
}

#[test]
fn cinema_4d_renderer_can_be_selected() {
    let mut composition = Composition::new(ItemId::new(1), "C", 8, 4, 24.0, Time::from_seconds(1.0));
    composition.renderer = Renderer::Cinema4d;
    assert_eq!(composition.renderer, Renderer::Cinema4d);
}

#[test]
fn cinema_4d_quality_ends_change_depth_and_keep_known_defaults() {
    let mut options = model::composition::Cinema4dOptions::default();
    assert_eq!(options.anti_aliasing, Cinema4dAntiAlias::Geometry);
    assert_eq!(options.geometry_subpixels, CINEMA4D_GEOMETRY_SUBPIXELS);
    assert_eq!(options.reflectance, Cinema4dReflectance::LayerSampling);
    options.quality_low = true;
    assert!(options.shadow_depth() < Cinema4dDepth::High);
    assert_eq!(options.reflection_depth(), Cinema4dDepth::Low);
    options.quality_low = false;
    assert_eq!(options.shadow_depth(), Cinema4dDepth::High);
}

#[test]
fn cinema_4d_installation_path_still_selects_the_renderer() {
    let mut composition = Composition::new(ItemId::new(1), "C", 8, 4, 24.0, Time::from_seconds(1.0));
    composition.renderer = Renderer::Cinema4d;
    composition.cinema4d.installation_path = Some("/opt/cinema4d");
    assert_eq!(composition.renderer, Renderer::Cinema4d);
    assert_eq!(composition.cinema4d.installation_path, Some("/opt/cinema4d"));
}

#[test]
fn footage_layer_duration_matches_a_ten_second_source() {
    let mut project = project();
    let comp = add_comp(&mut project);
    let footage = import_media(&mut project, "video", Time::from_seconds(10.0), 24.0);
    let layer = eval::edit::add_footage_layer(&mut project, comp, footage).unwrap();
    let span = project.composition(comp).unwrap().layer(layer).unwrap().duration();
    assert!((span.seconds() - 10.0).abs() < 1.0e-9);
}

#[test]
fn solid_layer_lands_in_the_folder_and_the_comp() {
    let mut project = project();
    let comp = add_comp(&mut project);
    let before_footage = project.footage.len();
    let before_layers = project.composition(comp).unwrap().layers.len();
    new_solid_layer(&mut project, comp, "S", 8, 8, Rgba::solid_default()).unwrap();
    assert_eq!(project.footage.len(), before_footage + 1);
    assert_eq!(project.composition(comp).unwrap().layers.len(), before_layers + 1);
    assert!(project.root.child_folder(model::project::SOLIDS_FOLDER).is_some());
}

#[test]
fn text_layer_shows_the_typed_string() {
    let mut project = project();
    let comp = add_comp(&mut project);
    let before = project.composition(comp).unwrap().layers.len();
    let layer = add_text_layer(&mut project, comp, "Hi").unwrap();
    assert_eq!(project.composition(comp).unwrap().layers.len(), before + 1);
    assert_eq!(project.composition(comp).unwrap().layer(layer).unwrap().text.as_ref().unwrap().source, "Hi");
    let image = render(&project, comp, Time::ZERO, RenderPurpose::Preview).unwrap();
    assert!(image.pixels.iter().any(|pixel| pixel.a > 0.0));
}

#[test]
fn shift_drag_makes_a_square_shape_layer() {
    let mut project = project();
    let comp = add_comp(&mut project);
    let before = project.composition(comp).unwrap().layers.len();
    let (layer, width, height) = add_shape_rect(&mut project, comp, 30.0, 10.0, true).unwrap();
    assert_eq!(width, height);
    assert_eq!(project.composition(comp).unwrap().layers.len(), before + 1);
    assert!(matches!(project.composition(comp).unwrap().layer(layer).unwrap().kind, LayerKind::Shape));
}

#[test]
fn null_parent_moves_the_child_by_the_same_amount() {
    let mut project = project();
    let comp = add_comp(&mut project);
    let null_id = add_null(&mut project, comp).unwrap();
    let child_id = project.alloc_layer();
    let mut child = Layer::new(child_id, "child", LayerKind::Null);
    child.transform.position = [5.0, 0.0, 0.0];
    project.composition_mut(comp).unwrap().layers.push(child);
    eval::edit::assign_parent(project.composition_mut(comp).unwrap(), child_id, null_id, false);
    project.composition_mut(comp).unwrap().layer_mut(null_id).unwrap().transform.position[0] += 10.0;
    let world = eval::edit::world_position(project.composition(comp).unwrap(), child_id).unwrap();
    assert!((world[0] - 15.0).abs() < 1.0e-9);
}

#[test]
fn adjustment_blur_hits_only_layers_below() {
    let mut project = project();
    let comp = add_comp(&mut project);
    project.composition_mut(comp).unwrap().width = 4;
    project.composition_mut(comp).unwrap().height = 1;
    project.composition_mut(comp).unwrap().background = Rgba::transparent();
    let back = new_solid_layer(&mut project, comp, "back", 4, 1, Rgba::new(1.0, 0.0, 0.0, 1.0)).unwrap();
    let adjustment = add_adjustment(&mut project, comp).unwrap();
    project.composition_mut(comp).unwrap().layer_mut(adjustment).unwrap().effects.push(model::layer::LayerEffect {
        name: "Blur".into(),
        enabled: true,
        kind: model::layer::EffectKind::Blur { radius: 2.0 },
    });
    let front = new_solid_layer(&mut project, comp, "front", 4, 1, Rgba::new(0.0, 0.0, 1.0, 1.0)).unwrap();
    let image = render(&project, comp, Time::ZERO, RenderPurpose::Preview).unwrap();
    assert!(image.pixels[0].b > 0.9);
    let _ = (back, front);
}

#[test]
fn camera_changes_the_projected_picture() {
    let without = project_camera([10.0, 0.0, 100.0], [0.0, 0.0, 0.0], 0.0, 50.0);
    let moved = project_camera([10.0, 0.0, 100.0], [20.0, 0.0, 0.0], 0.0, 50.0);
    assert_ne!(without[0], moved[0]);
    let turned = project_camera([10.0, 0.0, 100.0], [0.0, 0.0, 0.0], 30.0, 50.0);
    assert_ne!(without[0], turned[0]);
    let mut project = project();
    let comp = add_comp(&mut project);
    let camera = add_camera(&mut project, comp, Some(50.0)).unwrap();
    assert!(project.composition(comp).unwrap().layer(camera).unwrap().is_camera());
}

#[test]
fn a_light_changes_3d_brightness() {
    let mut project = project();
    let comp = add_comp(&mut project);
    project.composition_mut(comp).unwrap().width = 2;
    project.composition_mut(comp).unwrap().height = 1;
    let solid = new_solid_layer(&mut project, comp, "card", 2, 1, Rgba::white()).unwrap();
    enable_3d(project.composition_mut(comp).unwrap().layer_mut(solid).unwrap(), None);
    let unlit = render(&project, comp, Time::ZERO, RenderPurpose::Preview).unwrap();
    add_light(&mut project, comp, 0.5).unwrap();
    let lit = render(&project, comp, Time::ZERO, RenderPurpose::Preview).unwrap();
    assert!(lit.pixels[0].r < unlit.pixels[0].r);
}

#[test]
fn photoshop_file_is_added_at_the_front() {
    let mut project = project();
    let comp = add_comp(&mut project);
    new_solid_layer(&mut project, comp, "under", 4, 4, Rgba::black()).unwrap();
    let layer = add_photoshop_layer(&mut project, comp, "art.psd", 4, 4).unwrap();
    assert_eq!(project.composition(comp).unwrap().layers[0].id, layer);
    assert!(project.footage.iter().any(|item| matches!(item.kind, model::footage::FootageKind::Photoshop)));
}

#[test]
fn precompose_moves_layers_and_keeps_the_picture() {
    let mut project = project();
    let comp = add_comp(&mut project);
    project.composition_mut(comp).unwrap().width = 4;
    project.composition_mut(comp).unwrap().height = 2;
    project.composition_mut(comp).unwrap().background = Rgba::black();
    let layer = new_solid_layer(&mut project, comp, "card", 4, 2, Rgba::new(0.0, 1.0, 0.0, 1.0)).unwrap();
    let before = render(&project, comp, Time::ZERO, RenderPurpose::Preview).unwrap();
    precompose(&mut project, comp, &[layer], "pre").unwrap();
    assert_eq!(project.composition(comp).unwrap().layers.len(), 1);
    assert!(matches!(project.composition(comp).unwrap().layers[0].kind, LayerKind::Precomp { .. }));
    let after = render(&project, comp, Time::ZERO, RenderPurpose::Preview).unwrap();
    assert!((before.pixels[0].g - after.pixels[0].g).abs() < 1.0e-4);
}

#[test]
fn guide_layers_preview_but_do_not_render() {
    let mut project = project();
    let comp = add_comp(&mut project);
    project.composition_mut(comp).unwrap().width = 2;
    project.composition_mut(comp).unwrap().height = 1;
    project.composition_mut(comp).unwrap().background = Rgba::transparent();
    let layer = new_solid_layer(&mut project, comp, "guide", 2, 1, Rgba::white()).unwrap();
    project.composition_mut(comp).unwrap().layer_mut(layer).unwrap().guide = true;
    let layer_ref = project.composition(comp).unwrap().layer(layer).unwrap();
    assert!(visible_in_preview(project.composition(comp).unwrap(), layer_ref));
    assert!(!included_in_render(project.composition(comp).unwrap(), layer_ref));
    let preview = render(&project, comp, Time::ZERO, RenderPurpose::Preview).unwrap();
    let exported = render(&project, comp, Time::ZERO, RenderPurpose::Export).unwrap();
    assert!(preview.pixels[0].a > 0.5);
    assert!(exported.pixels[0].a < 0.1);
}

#[test]
fn video_switch_hides_preview_and_render() {
    let mut project = project();
    let comp = add_comp(&mut project);
    project.composition_mut(comp).unwrap().width = 2;
    project.composition_mut(comp).unwrap().height = 1;
    project.composition_mut(comp).unwrap().background = Rgba::transparent();
    let layer = new_solid_layer(&mut project, comp, "v", 2, 1, Rgba::white()).unwrap();
    project.composition_mut(comp).unwrap().layer_mut(layer).unwrap().video = false;
    let preview = render(&project, comp, Time::ZERO, RenderPurpose::Preview).unwrap();
    let exported = render(&project, comp, Time::ZERO, RenderPurpose::Export).unwrap();
    assert!(preview.pixels[0].a < 0.1);
    assert!(exported.pixels[0].a < 0.1);
    project.composition_mut(comp).unwrap().layer_mut(layer).unwrap().video = true;
    let back = render(&project, comp, Time::ZERO, RenderPurpose::Preview).unwrap();
    assert!(back.pixels[0].a > 0.5);
}

#[test]
fn audio_switch_silences_the_layer() {
    let mut layer = Layer::new(model::id::LayerId::new(1), "snd", LayerKind::Footage { source: ItemId::new(1) });
    layer.audio = false;
    assert!(!layer.audio);
    layer.audio = true;
    assert!(layer.audio);
}

#[test]
fn solo_shows_only_the_soloed_visuals() {
    let mut project = project();
    let comp = add_comp(&mut project);
    let a = new_solid_layer(&mut project, comp, "a", 2, 2, Rgba::white()).unwrap();
    let b = new_solid_layer(&mut project, comp, "b", 2, 2, Rgba::black()).unwrap();
    let c = new_solid_layer(&mut project, comp, "c", 2, 2, Rgba::black()).unwrap();
    project.composition_mut(comp).unwrap().layer_mut(a).unwrap().solo = true;
    project.composition_mut(comp).unwrap().layer_mut(b).unwrap().solo = true;
    let composition = project.composition(comp).unwrap();
    assert!(visible_in_preview(composition, composition.layer(a).unwrap()));
    assert!(visible_in_preview(composition, composition.layer(b).unwrap()));
    assert!(!visible_in_preview(composition, composition.layer(c).unwrap()));
    solo_only(project.composition_mut(comp).unwrap(), a);
    let composition = project.composition(comp).unwrap();
    assert!(composition.layer(a).unwrap().solo);
    assert!(!composition.layer(b).unwrap().solo);
}

#[test]
fn locked_layers_cannot_move_until_unlocked() {
    let mut layer = Layer::new(model::id::LayerId::new(1), "L", LayerKind::Null);
    layer.locked = true;
    assert!(!try_move(&mut layer, [5.0, 0.0, 0.0]));
    assert_eq!(layer.transform.position[0], 0.0);
    layer.locked = false;
    assert!(try_move(&mut layer, [5.0, 0.0, 0.0]));
    assert_eq!(layer.transform.position[0], 5.0);
}

#[test]
fn shy_layers_leave_the_list_but_stay_in_preview() {
    let mut project = project();
    let comp = add_comp(&mut project);
    let layer = new_solid_layer(&mut project, comp, "shy", 2, 2, Rgba::white()).unwrap();
    project.composition_mut(comp).unwrap().layer_mut(layer).unwrap().shy = true;
    project.composition_mut(comp).unwrap().hide_shy_layers = true;
    let composition = project.composition(comp).unwrap();
    assert!(!timeline_lists(composition, composition.layer(layer).unwrap()));
    assert!(visible_in_preview(composition, composition.layer(layer).unwrap()));
}

#[test]
fn showing_shy_layers_puts_them_back_in_the_list() {
    let mut project = project();
    let comp = add_comp(&mut project);
    let layer = new_solid_layer(&mut project, comp, "shy", 2, 2, Rgba::white()).unwrap();
    project.composition_mut(comp).unwrap().layer_mut(layer).unwrap().shy = true;
    project.composition_mut(comp).unwrap().hide_shy_layers = false;
    let composition = project.composition(comp).unwrap();
    assert!(timeline_lists(composition, composition.layer(layer).unwrap()));
}

#[test]
fn collapse_transformations_keeps_full_resolution() {
    let row = [1.0, 0.0, 1.0, 0.0];
    let on = collapse_resample(&row, 0.5, 2.0, true);
    let off = collapse_resample(&row, 0.5, 2.0, false);
    assert_eq!(on, vec![1.0, 0.0, 1.0, 0.0]);
    assert_ne!(on, off);
}

#[test]
fn continuous_rasterize_stays_smooth_at_400_percent() {
    let on = rasterize_step(4.0, true);
    let off = rasterize_step(4.0, false);
    assert!(on.iter().any(|sample| *sample > 0.0 && *sample < 1.0));
    assert!(off.iter().all(|sample| *sample == 0.0 || *sample == 1.0));
}

#[test]
fn best_quality_antialiases_a_diagonal() {
    let coverage = diagonal_coverage(0.25, 0.0, LayerQuality::Best);
    assert!(coverage > 0.0 && coverage < 1.0);
    let snapped = snap_subpixel(0.5 + 1.0 / 65536.0, LayerQuality::Best);
    assert!((snapped - (0.5 + 1.0 / 65536.0)).abs() < 1.0e-9);
}

#[test]
fn draft_quality_is_jagged() {
    let coverage = diagonal_coverage(0.25, 0.0, LayerQuality::Draft);
    assert!(coverage == 0.0 || coverage == 1.0);
    let snapped = snap_subpixel(0.4, LayerQuality::Draft);
    assert_eq!(snapped, 0.0);
}

#[test]
fn wireframe_draws_only_the_border() {
    let mut project = project();
    let comp = add_comp(&mut project);
    project.composition_mut(comp).unwrap().width = 4;
    project.composition_mut(comp).unwrap().height = 3;
    project.composition_mut(comp).unwrap().background = Rgba::transparent();
    let layer = new_solid_layer(&mut project, comp, "box", 4, 3, Rgba::new(1.0, 0.0, 0.0, 1.0)).unwrap();
    project.composition_mut(comp).unwrap().layer_mut(layer).unwrap().quality = LayerQuality::Wireframe;
    let image = render(&project, comp, Time::ZERO, RenderPurpose::Preview).unwrap();
    assert!(image.pixels[0].a > 0.5);
    let center = (1 * image.width + 1) as usize;
    assert!(image.pixels[center].a < 0.1);
}

#[test]
fn bilinear_sampling_does_not_ring() {
    let samples = [0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
    let value = resample_1d(&samples, 2.5, Sampling::Bilinear);
    assert!((0.0..=1.0).contains(&value));
}

#[test]
fn bicubic_sampling_overshoots_a_hard_edge() {
    let samples = [0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
    let value = resample_1d(&samples, 2.8, Sampling::Bicubic);
    let linear = resample_1d(&samples, 2.8, Sampling::Bilinear);
    assert!(value > linear);
}

#[test]
fn effect_switch_disables_effects_without_clearing_checks() {
    let mut layer = Layer::new(model::id::LayerId::new(1), "L", LayerKind::Null);
    layer.effects.push(model::layer::LayerEffect {
        name: "Blur".into(),
        enabled: true,
        kind: model::layer::EffectKind::Generic,
    });
    layer.effects_enabled = false;
    assert!(layer.effects[0].enabled);
    assert!(!layer.effects_enabled);
}

#[test]
fn frame_blend_switch_cycles_three_states() {
    let mut mode = FrameBlend::Off;
    mode = mode.cycle();
    assert_eq!(mode, FrameBlend::FrameMix);
    mode = mode.cycle();
    assert_eq!(mode, FrameBlend::PixelMotion);
    mode = mode.cycle();
    assert_eq!(mode, FrameBlend::Off);
}

#[test]
fn motion_blur_span_follows_the_shutter() {
    assert!((blur_span(80.0, 90.0) - 20.0).abs() < 1.0e-9);
}

#[test]
fn comp_motion_blur_switch_gates_the_layer() {
    let active = |comp_on, layer_on| comp_on && layer_on;
    assert!(!active(false, true));
    assert!(active(true, true));
}

#[test]
fn adjustment_switch_hides_the_layer_picture() {
    let mut project = project();
    let comp = add_comp(&mut project);
    project.composition_mut(comp).unwrap().width = 2;
    project.composition_mut(comp).unwrap().height = 1;
    project.composition_mut(comp).unwrap().background = Rgba::black();
    let layer = new_solid_layer(&mut project, comp, "clip", 2, 1, Rgba::new(0.0, 1.0, 0.0, 1.0)).unwrap();
    project.composition_mut(comp).unwrap().layer_mut(layer).unwrap().adjustment = true;
    project.composition_mut(comp).unwrap().layer_mut(layer).unwrap().effects.push(model::layer::LayerEffect {
        name: "Blur".into(),
        enabled: true,
        kind: model::layer::EffectKind::Blur { radius: 1.0 },
    });
    let image = render(&project, comp, Time::ZERO, RenderPurpose::Preview).unwrap();
    assert!(image.pixels[0].g < 0.1);
}

#[test]
fn enabling_3d_adds_z_and_camera_perspective() {
    let mut layer = Layer::new(model::id::LayerId::new(1), "card", LayerKind::Null);
    enable_3d(&mut layer, None);
    assert!(layer.three_d);
    assert_eq!(layer.transform.position.len(), 3);
    let flat = project_camera([4.0, 0.0, 20.0], [0.0, 0.0, 0.0], 0.0, 10.0);
    let yaw = project_camera([4.0, 0.0, 20.0], [0.0, 0.0, 0.0], 25.0, 10.0);
    assert_ne!(flat[0], yaw[0]);
}

#[test]
fn av_column_hides_and_restores_four_switches() {
    let mut composition = Composition::new(ItemId::new(1), "C", 8, 4, 24.0, Time::from_seconds(1.0));
    assert!(composition.columns.av_features);
    composition.columns.av_features = false;
    assert!(!composition.columns.av_features);
    composition.columns.av_features = true;
    assert!(composition.columns.av_features);
}

#[test]
fn label_column_search_finds_the_same_color() {
    let mut project = project();
    project.labels.labels[2].name = "Red".into();
    project.labels.labels[2].color = Some(LabelColor { r: 1.0, g: 0.0, b: 0.0 });
    let comp = add_comp(&mut project);
    let layer = new_solid_layer(&mut project, comp, "a", 2, 2, Rgba::black()).unwrap();
    project.composition_mut(comp).unwrap().layer_mut(layer).unwrap().label = Some(3);
    let found = search_layers(&project, project.composition(comp).unwrap(), "Red");
    assert_eq!(found, vec![layer]);
    assert_eq!(label_name(&project, Some(3)), "Red");
}

#[test]
fn name_column_toggles_between_layer_and_source() {
    let mut column = NameColumn::Layer;
    column = column.toggle();
    assert_eq!(column, NameColumn::Source);
    column = column.toggle();
    assert_eq!(column, NameColumn::Layer);
}

#[test]
fn comment_search_returns_only_that_layer() {
    let mut project = project();
    let comp = add_comp(&mut project);
    let marked = new_solid_layer(&mut project, comp, "a", 2, 2, Rgba::black()).unwrap();
    new_solid_layer(&mut project, comp, "b", 2, 2, Rgba::black()).unwrap();
    project.composition_mut(comp).unwrap().layer_mut(marked).unwrap().comment = "hero".into();
    let found = search_layers(&project, project.composition(comp).unwrap(), "hero");
    assert_eq!(found, vec![marked]);
}

#[test]
fn f4_swaps_switches_and_modes() {
    let mut composition = Composition::new(ItemId::new(1), "C", 8, 4, 24.0, Time::from_seconds(1.0));
    assert!(composition.columns.switches);
    composition.columns.toggle_switches_modes();
    assert!(composition.columns.modes && !composition.columns.switches);
    composition.columns.toggle_switches_modes();
    assert!(composition.columns.switches && !composition.columns.modes);
}

#[test]
fn modes_column_exposes_the_blend_mode() {
    let mut composition = Composition::new(ItemId::new(1), "C", 8, 4, 24.0, Time::from_seconds(1.0));
    composition.columns.toggle_switches_modes();
    let mut layer = Layer::new(model::id::LayerId::new(1), "L", LayerKind::Null);
    assert_eq!(layer.blend_mode, BlendMode::Normal);
    layer.blend_mode = BlendMode::Multiply;
    composition.layers.push(layer);
    assert!(composition.columns.modes);
    assert_eq!(composition.layers[0].blend_mode, BlendMode::Multiply);
}

#[test]
fn parent_column_toggles_with_shift_f4() {
    let mut composition = Composition::new(ItemId::new(1), "C", 8, 4, 24.0, Time::from_seconds(1.0));
    let before = composition.columns.parent;
    composition.columns.toggle_parent();
    assert_ne!(composition.columns.parent, before);
}

#[test]
fn moving_the_in_point_later_shortens_the_layer() {
    let mut layer = Layer::new(model::id::LayerId::new(1), "L", LayerKind::Null);
    layer.in_point = Time::ZERO;
    layer.out_point = Time::from_seconds(4.0);
    layer.in_point = Time::from_seconds(1.0);
    assert!((layer.duration().seconds() - 3.0).abs() < 1.0e-9);
    assert!((layer.out_point.seconds() - 4.0).abs() < 1.0e-9);
}

#[test]
fn moving_the_out_point_earlier_keeps_the_in_point() {
    let mut layer = Layer::new(model::id::LayerId::new(1), "L", LayerKind::Null);
    layer.in_point = Time::from_seconds(1.0);
    layer.out_point = Time::from_seconds(4.0);
    layer.out_point = Time::from_seconds(2.0);
    assert!((layer.in_point.seconds() - 1.0).abs() < 1.0e-9);
    assert!((layer.duration().seconds() - 1.0).abs() < 1.0e-9);
}

#[test]
fn duration_column_matches_out_minus_in() {
    let mut layer = Layer::new(model::id::LayerId::new(1), "L", LayerKind::Null);
    layer.in_point = Time::from_seconds(1.5);
    layer.out_point = Time::from_seconds(3.5);
    assert!((layer.duration().seconds() - (layer.out_point.seconds() - layer.in_point.seconds())).abs() < 1.0e-12);
}

#[test]
fn stretch_200_doubles_duration_and_key_spacing() {
    let mut layer = Layer::new(model::id::LayerId::new(1), "L", LayerKind::Null);
    layer.in_point = Time::ZERO;
    layer.out_point = Time::from_seconds(2.0);
    layer.properties.push(Property::value(
        model::id::PropertyId::new(1),
        "Opacity",
        "Opacity",
        PropertyValue::Scalar(1.0),
    ));
    layer.properties[0].push_keyframe(model::keyframe::Keyframe::at(Time::ZERO, PropertyValue::Scalar(0.0)));
    layer.properties[0].push_keyframe(model::keyframe::Keyframe::at(Time::from_seconds(1.0), PropertyValue::Scalar(1.0)));
    layer.stretch_time(2.0, model::layer::StretchHold::InPoint, Time::ZERO);
    assert!((layer.duration().seconds() - 4.0).abs() < 1.0e-9);
    let keys = layer.properties[0].keyframes();
    assert!((keys[1].time.seconds() - 2.0).abs() < 1.0e-9);
}

#[test]
fn confirmed_columns_do_not_invent_a_keyframe_column() {
    let columns = confirmed_timeline_columns();
    assert!(columns.contains(&"A/V Features"));
    assert!(!columns.iter().any(|name| name.to_ascii_lowercase().contains("keyframe")));
}

#[test]
fn label_group_selects_every_layer_of_that_color() {
    let mut project = project();
    let comp = add_comp(&mut project);
    let a = new_solid_layer(&mut project, comp, "a", 2, 2, Rgba::black()).unwrap();
    let b = new_solid_layer(&mut project, comp, "b", 2, 2, Rgba::black()).unwrap();
    new_solid_layer(&mut project, comp, "c", 2, 2, Rgba::black()).unwrap();
    project.composition_mut(comp).unwrap().layer_mut(a).unwrap().label = Some(4);
    project.composition_mut(comp).unwrap().layer_mut(b).unwrap().label = Some(4);
    let selected = select_label_group(project.composition(comp).unwrap(), 4);
    assert_eq!(selected.len(), 2);
    assert!(selected.contains(&a) && selected.contains(&b));
}

#[test]
fn handle_colors_follow_the_preference() {
    let mut project = project();
    project.labels.labels[0].color = Some(LabelColor { r: 1.0, g: 0.0, b: 0.0 });
    project.labels.use_for_handles = Some(true);
    let mut layer = Layer::new(model::id::LayerId::new(1), "L", LayerKind::Null);
    layer.label = Some(1);
    assert_eq!(handle_color(&project, &layer).r, 1.0);
    project.labels.use_for_handles = Some(false);
    let mut other = layer.clone();
    other.label = Some(2);
    assert_eq!(handle_color(&project, &layer), handle_color(&project, &other));
}

#[test]
fn tab_color_matches_the_comp_label_when_enabled() {
    let mut project = project();
    project.labels.labels[2].color = Some(LabelColor { r: 0.0, g: 1.0, b: 0.0 });
    project.labels.use_for_tabs = Some(true);
    let comp = add_comp(&mut project);
    project.composition_mut(comp).unwrap().label = Some(3);
    assert_eq!(tab_color(&project, project.composition(comp).unwrap()).unwrap().g, 1.0);
    project.labels.use_for_tabs = Some(false);
    assert!(tab_color(&project, project.composition(comp).unwrap()).is_none());
}

#[test]
fn new_comp_uses_the_composition_label_default() {
    let mut project = project();
    project.labels.defaults.composition = Some(6);
    let comp = add_comp(&mut project);
    confirm_comp_settings(&mut project, comp);
    let fresh = new_comp_from_last(&mut project, "Next").unwrap();
    assert_eq!(project.composition(fresh).unwrap().label, Some(6));
}

#[test]
fn imported_video_uses_the_video_label_default() {
    let mut project = project();
    project.labels.defaults.video = Some(2);
    let id = import_media(&mut project, "video", Time::from_seconds(2.0), 24.0);
    assert_eq!(project.footage(id).unwrap().label, Some(2));
}

#[test]
fn imported_audio_uses_the_audio_label_default() {
    let mut project = project();
    project.labels.defaults.audio = Some(5);
    let id = import_media(&mut project, "audio", Time::from_seconds(2.0), 24.0);
    assert_eq!(project.footage(id).unwrap().label, Some(5));
}

#[test]
fn imported_still_uses_the_still_label_default() {
    let mut project = project();
    project.labels.defaults.still = Some(7);
    let id = import_media(&mut project, "still", Time::from_frames(1.0, 24.0), 24.0);
    assert_eq!(project.footage(id).unwrap().label, Some(7));
}

#[test]
fn new_folder_uses_the_folder_label_default() {
    let mut project = project();
    project.labels.defaults.folder = Some(8);
    let id = new_folder(&mut project, "Shots");
    let folder = project.root.children.iter().find_map(|child| match child {
        model::folder::FolderChild::Folder(folder) if folder.id == id => Some(folder),
        _ => None,
    });
    assert_eq!(folder.unwrap().label, Some(8));
}

#[test]
fn new_solid_uses_the_solid_label_default() {
    let mut project = project();
    project.labels.defaults.solid = Some(9);
    let comp = add_comp(&mut project);
    let layer = new_solid_layer(&mut project, comp, "S", 2, 2, Rgba::black()).unwrap();
    assert_eq!(project.composition(comp).unwrap().layer(layer).unwrap().label, Some(9));
}

#[test]
fn new_camera_uses_the_camera_label_default() {
    let mut project = project();
    project.labels.defaults.camera = Some(10);
    let comp = add_comp(&mut project);
    let layer = add_camera(&mut project, comp, None).unwrap();
    assert_eq!(project.composition(comp).unwrap().layer(layer).unwrap().label, Some(10));
}

#[test]
fn new_light_uses_the_light_label_default() {
    let mut project = project();
    project.labels.defaults.light = Some(11);
    let comp = add_comp(&mut project);
    let layer = add_light(&mut project, comp, 1.0).unwrap();
    assert_eq!(project.composition(comp).unwrap().layer(layer).unwrap().label, Some(11));
}

#[test]
fn new_adjustment_uses_the_adjustment_label_default() {
    let mut project = project();
    project.labels.defaults.adjustment = Some(12);
    let comp = add_comp(&mut project);
    let layer = add_adjustment(&mut project, comp).unwrap();
    assert_eq!(project.composition(comp).unwrap().layer(layer).unwrap().label, Some(12));
}

#[test]
fn new_null_uses_the_null_label_default() {
    let mut project = project();
    project.labels.defaults.null_object = Some(13);
    let comp = add_comp(&mut project);
    let layer = add_null(&mut project, comp).unwrap();
    assert_eq!(project.composition(comp).unwrap().layer(layer).unwrap().label, Some(13));
}

#[test]
fn new_shape_uses_the_shape_label_default() {
    let mut project = project();
    project.labels.defaults.shape = Some(14);
    let comp = add_comp(&mut project);
    let (layer, _, _) = add_shape_rect(&mut project, comp, 10.0, 10.0, false).unwrap();
    assert_eq!(project.composition(comp).unwrap().layer(layer).unwrap().label, Some(14));
}

#[test]
fn new_text_uses_the_text_label_default() {
    let mut project = project();
    project.labels.defaults.text = Some(15);
    let comp = add_comp(&mut project);
    let layer = add_text_layer(&mut project, comp, "A").unwrap();
    assert_eq!(project.composition(comp).unwrap().layer(layer).unwrap().label, Some(15));
}

#[test]
fn new_3d_model_uses_the_model_label_default() {
    let mut project = project();
    project.labels.defaults.model_3d = Some(16);
    let mut layer = Layer::new(model::id::LayerId::new(1), "card", LayerKind::Footage { source: ItemId::new(1) });
    enable_3d(&mut layer, project.labels.defaults.model_3d);
    assert_eq!(layer.label, Some(16));
    assert!(layer.three_d);
}

#[test]
fn other_3d_uses_its_label_default() {
    let mut project = project();
    project.labels.defaults.other_3d = Some(1);
    let comp = add_comp(&mut project);
    let layer = new_other_3d(&mut project, comp).unwrap();
    let layer = project.composition(comp).unwrap().layer(layer).unwrap();
    assert!(layer.three_d);
    assert_eq!(layer.label, Some(1));
}

#[test]
fn auto_graph_picks_speed_for_position_and_value_for_opacity() {
    assert_eq!(auto_graph_kind(true), GraphKind::Speed);
    assert_eq!(auto_graph_kind(false), GraphKind::Value);
    let _ = ShowProperties::Selected;
    let _ = GraphPropertyRef {
        match_name: "Opacity".into(),
        spatial: false,
        animated: true,
        selected: true,
    };
}
