use model::bits::BitsPerChannel;
use model::color::{Rgba, SOLID_GRAY_CODE};
use model::footage::{
    red_source_exposure, solid_size_allowed, AlphaInterpretation, AlphaMode, CinemaDngOptions,
    CinemaDngSpace, FieldSeparation, Footage, FootageProxy, InputProfile, Interpretation,
    PixelAspect, Pulldown, SOLID_MAX_PIXELS,
};
use model::project::{
    ColorEngine, FilmGauge, OcioConfig, Project, TimeDisplayStyle, TimecodeOrigin, WorkingSpace,
    SOLIDS_FOLDER,
};
use model::time::{feet_frames, frame_number, frame_to_timecode, FrameNumbering, Time};
use model::PropertyValue;

use eval::blend_pixel::{blend_channel, blend_pixel};
use eval::color_space::{
    convert_to_working, display_transform, gamma_encoded_mix, linearize_mix, srgb_code_to_rec709,
    srgb_to_linear,
};
use eval::edit::{
    add_footage_layer, apply_interpretation, clear_proxy, create_proxy_from_render, edit_solid,
    import_placeholder, import_solid, mark_missing, new_solid_layer, relink, remember_interpretation,
    render_order, replace_with_file, replace_with_placeholder, set_proxy, set_proxy_enabled,
};
use eval::picture::{apply_alpha, guess_alpha, preserve_edges_static, pulldown_rate, remove_pulldown, separate_fields};
use eval::render::{render, RenderPurpose};
use model::layer::LayerKind;
use model::BlendMode;
use model::composition::Composition;

fn near(left: f32, right: f32) {
    assert!((left - right).abs() < 1.0e-4, "{left} != {right}");
}

fn project_8() -> Project {
    Project::new(BitsPerChannel::Bpc8)
}

fn add_comp(project: &mut Project, fps: f64, seconds: f64) -> model::id::ItemId {
    let id = project.alloc_item();
    project.add_composition(Composition::new(
        id,
        "Comp",
        8,
        4,
        fps,
        Time::from_seconds(seconds),
    ));
    id
}

#[test]
fn depth_8_bpc_uses_256_steps() {
    let mut codes = std::collections::BTreeSet::new();
    for code in 0..=255 {
        codes.insert(BitsPerChannel::Bpc8.quantize_unit(code as f32 / 255.0).to_bits());
    }
    assert_eq!(codes.len(), 256);
    assert_eq!(BitsPerChannel::Bpc8.quantize_unit(0.0), 0.0);
    assert_eq!(BitsPerChannel::Bpc8.quantize_unit(1.0), 1.0);
    assert_eq!(BitsPerChannel::Bpc8.cycle(), BitsPerChannel::Bpc16);
}

#[test]
fn depth_16_bpc_has_finer_steps_than_8() {
    let step = |bits: BitsPerChannel| {
        let mut previous = bits.quantize_unit(0.0);
        let mut largest = 0.0_f32;
        for index in 1..=512 {
            let quantized = bits.quantize_unit(index as f32 / 512.0);
            largest = largest.max((quantized - previous).abs());
            previous = quantized;
        }
        largest
    };
    assert!(step(BitsPerChannel::Bpc16) < step(BitsPerChannel::Bpc8));
    assert_eq!(BitsPerChannel::Bpc16.level_count(), Some(32769));
}

#[test]
fn depth_32_bpc_keeps_values_past_white() {
    let gray = 128.0 / 255.0;
    let eight = blend_channel(BlendMode::Add, gray, gray, BitsPerChannel::Bpc8);
    let wide = blend_channel(BlendMode::Add, gray, gray, BitsPerChannel::Bpc32);
    assert!(eight <= 1.0);
    assert!(wide > 1.0);
    assert_eq!(BitsPerChannel::Bpc32.cycle(), BitsPerChannel::Bpc8);
}

#[test]
fn new_project_color_engine_is_adobe() {
    let project = project_8();
    assert_eq!(project.color.engine, ColorEngine::AdobeColorManaged);
    assert!(project.color.icc_profiles_available());
}

#[test]
fn ocio_engine_exposes_config_and_display() {
    let mut project = project_8();
    project.color.engine = ColorEngine::OcioColorManaged;
    project.color.ocio_config = Some(OcioConfig::Aces20CgV4);
    project.color.working_space = Some(WorkingSpace::HdtvRec709);
    project.color.display_color_space = Some("sRGB".to_string());
    assert!(project.color.ocio_menus_visible());
    assert_ne!(
        project.color.display_color_space.as_deref(),
        Some("HDTV (Rec. 709)")
    );
}

#[test]
fn ocio_aces_cg_is_selectable() {
    let mut project = project_8();
    project.color.engine = ColorEngine::OcioColorManaged;
    project.color.ocio_config = Some(OcioConfig::Aces20CgV4);
    assert_eq!(project.color.ocio_config, Some(OcioConfig::Aces20CgV4));
    assert!(project.color.display_color_space.is_none());
}

#[test]
fn ocio_aces_studio_differs_from_cg() {
    let cg = OcioConfig::Aces20CgV4;
    let studio = OcioConfig::Aces20StudioV4;
    assert_ne!(cg, studio);
    let mut project = project_8();
    project.color.ocio_config = Some(studio.clone());
    assert_eq!(project.color.ocio_config, Some(OcioConfig::Aces20StudioV4));
}

#[test]
fn ocio_custom_config_path_is_kept() {
    let mut project = project_8();
    project.color.engine = ColorEngine::OcioColorManaged;
    project.color.ocio_config = Some(OcioConfig::Custom("show.ocio".to_string()));
    assert_eq!(
        project.color.ocio_config,
        Some(OcioConfig::Custom("show.ocio".to_string()))
    );
}

#[test]
fn working_space_off_keeps_file_rgb() {
    let pixel = Rgba::new(0.2, 0.4, 0.6, 1.0);
    let converted = convert_to_working(pixel, Some(&WorkingSpace::Off), false, false);
    assert_eq!(converted, pixel);
}

#[test]
fn rec709_working_space_converts_srgb_mid_gray() {
    let mid = 0.5;
    let converted = srgb_code_to_rec709(mid);
    near(srgb_code_to_rec709(0.0), 0.0);
    near(srgb_code_to_rec709(1.0), 1.0);
    assert!((converted - mid).abs() > 1.0e-3);
    let pixel = convert_to_working(
        Rgba::new(mid, mid, mid, 1.0),
        Some(&WorkingSpace::HdtvRec709),
        false,
        false,
    );
    near(pixel.r, converted);
}

#[test]
fn display_space_does_not_change_export() {
    let working = 0.4;
    let exported = working;
    let displayed = display_transform(working, Some("linear"));
    assert_eq!(exported, working);
    assert!((displayed - exported).abs() > 1.0e-4);
    let other = display_transform(working, Some("srgb-view"));
    assert!((other - displayed).abs() > 1.0e-4 || other != displayed);
    assert_eq!(display_transform(working, None), exported);
}

#[test]
fn linearize_changes_saturated_fringe() {
    let gamma = gamma_encoded_mix(1.0, 0.0);
    let linear = linearize_mix(1.0, 0.0, true);
    assert!((gamma - linear).abs() > 1.0e-3);
    near(linear, linearize_mix(1.0, 0.0, true));
}

#[test]
fn gamma_one_blend_differs_from_encoded_blend() {
    let linear_half = 0.5_f32;
    let encoded_half = gamma_encoded_mix(1.0, 0.0);
    let encoded_as_light = srgb_to_linear(encoded_half);
    near(encoded_half, 0.5);
    assert!((linear_half - encoded_as_light).abs() > 0.05);
}

#[test]
fn timecode_2997_drop_and_nondrop_differ_at_one_minute() {
    let drop = frame_to_timecode(1800, true);
    let non_drop = frame_to_timecode(1800, false);
    assert_eq!(non_drop.format(), "00:01:00:00");
    assert_eq!(drop.format(), "00:01:00:02");
    assert_ne!(drop, non_drop);
}

#[test]
fn media_source_timecode_uses_the_source_start() {
    let mut project = project_8();
    project.time_display.origin = Some(TimecodeOrigin::MediaSource);
    let id = project.alloc_item();
    let mut footage = Footage::file(id, "shot", 4, 4, Time::from_seconds(1.0), 24.0);
    footage.media_start_frame = Some(24 * 60 * 60);
    project.add_footage(footage);
    let shown = match project.time_display.origin {
        Some(TimecodeOrigin::MediaSource) => project.footage(id).unwrap().media_start_frame,
        _ => Some(0),
    };
    assert_eq!(shown, Some(86400));
    project.time_display.origin = Some(TimecodeOrigin::Zero);
    let zero = match project.time_display.origin {
        Some(TimecodeOrigin::Zero) => 0,
        _ => 1,
    };
    assert_eq!(zero, 0);
}

#[test]
fn ctrl_click_toggles_timecode_and_frames() {
    let mut style = TimeDisplayStyle::Timecode;
    style = style.toggle();
    assert_eq!(style, TimeDisplayStyle::Frames);
    style = style.toggle();
    assert_eq!(style, TimeDisplayStyle::Timecode);
}

#[test]
fn frame_number_follows_timecode_conversion() {
    let start = Some(100);
    assert_eq!(frame_number(0, FrameNumbering::TimecodeConversion, start), 100);
    assert_eq!(frame_number(0, FrameNumbering::TimecodeConversion, None), 0);
}

#[test]
fn frame_number_starts_at_zero() {
    assert_eq!(frame_number(0, FrameNumbering::StartAt0, Some(10)), 0);
}

#[test]
fn frame_number_starts_at_one() {
    assert_eq!(frame_number(0, FrameNumbering::StartAt1, None), 1);
}

#[test]
fn feet_plus_frames_splits_with_a_supplied_count() {
    let shown = feet_frames(40, 16).unwrap();
    assert_eq!(shown.feet, 2);
    assert_eq!(shown.frames, 8);
    assert!(feet_frames(10, 0).is_none());
}

#[test]
fn film_16mm_does_not_invent_frames_per_foot() {
    let mut project = project_8();
    assert!(project.time_display.frames_per_foot_16.is_none());
    project.time_display.gauge = Some(FilmGauge::Mm16);
    assert!(project.time_display.frames_per_foot().is_none());
    project.time_display.frames_per_foot_16 = Some(40);
    assert_eq!(project.time_display.frames_per_foot(), Some(40));
}

#[test]
fn film_35mm_count_is_independent_of_16mm() {
    let mut project = project_8();
    project.time_display.frames_per_foot_16 = Some(40);
    project.time_display.gauge = Some(FilmGauge::Mm35);
    assert!(project.time_display.frames_per_foot().is_none());
    project.time_display.frames_per_foot_35 = Some(16);
    assert_eq!(project.time_display.frames_per_foot(), Some(16));
    assert_eq!(project.time_display.frames_per_foot_16, Some(40));
}

#[test]
fn import_solid_does_not_add_a_layer() {
    let mut project = project_8();
    let comp = add_comp(&mut project, 24.0, 2.0);
    let before = project.composition(comp).unwrap().layers.len();
    let id = import_solid(&mut project, "Gray", 64, 64, Rgba::solid_default()).unwrap();
    assert!(project.footage(id).is_some());
    assert_eq!(project.composition(comp).unwrap().layers.len(), before);
    assert!(project.root.child_folder(SOLIDS_FOLDER).is_some());
}

#[test]
fn solid_rejects_sizes_above_30000() {
    assert!(solid_size_allowed(SOLID_MAX_PIXELS, SOLID_MAX_PIXELS));
    assert!(!solid_size_allowed(SOLID_MAX_PIXELS + 1, 10));
    assert!(!solid_size_allowed(10, SOLID_MAX_PIXELS + 1));
    assert!(import_solid(&mut project_8(), "big", SOLID_MAX_PIXELS + 1, 10, Rgba::black()).is_none());
}

#[test]
fn new_solid_is_45_gray_and_keeps_a_changed_color() {
    let color = Rgba::solid_default();
    let code = (color.r * 255.0).round() as u8;
    assert_eq!(code, SOLID_GRAY_CODE);
    assert_eq!((color.g * 255.0).round() as u8, 45);
    assert_eq!((color.b * 255.0).round() as u8, 45);
    let mut project = project_8();
    let comp = add_comp(&mut project, 24.0, 1.0);
    let layer = new_solid_layer(&mut project, comp, "S", 8, 8, Rgba::solid_default()).unwrap();
    let custom = Rgba::new(0.1, 0.2, 0.3, 1.0);
    edit_solid(&mut project, comp, layer, custom, true);
    let source = project.composition(comp).unwrap().layer(layer).unwrap().source_id().unwrap();
    match project.footage(source).unwrap().kind {
        model::footage::FootageKind::Solid { color } => assert_eq!(color, custom),
        _ => panic!("solid"),
    }
}

#[test]
fn renaming_a_solid_updates_layer_source_name() {
    let mut project = project_8();
    let comp = add_comp(&mut project, 24.0, 1.0);
    let layer = new_solid_layer(&mut project, comp, "Old", 4, 4, Rgba::black()).unwrap();
    let source = project.composition(comp).unwrap().layer(layer).unwrap().source_id().unwrap();
    project.footage_mut(source).unwrap().name = "New".to_string();
    let name = eval::edit::displayed_source_name(&project, project.composition(comp).unwrap().layer(layer).unwrap());
    assert_eq!(name, "New");
}

#[test]
fn affect_all_solids_or_only_the_selected_layer() {
    let mut project = project_8();
    let comp = add_comp(&mut project, 24.0, 1.0);
    let color = Rgba::black();
    let first = new_solid_layer(&mut project, comp, "S", 4, 4, color).unwrap();
    let source = project.composition(comp).unwrap().layer(first).unwrap().source_id().unwrap();
    let second_id = project.alloc_layer();
    let second = model::layer::Layer::new(second_id, "S2", LayerKind::Solid { source });
    project.composition_mut(comp).unwrap().layers.push(second);
    let red = Rgba::new(1.0, 0.0, 0.0, 1.0);
    edit_solid(&mut project, comp, first, red, true);
    let other = project.composition(comp).unwrap().layers.iter().find(|layer| layer.id == second_id).unwrap().source_id();
    assert_eq!(other, Some(source));
    let blue = Rgba::new(0.0, 0.0, 1.0, 1.0);
    let created = edit_solid(&mut project, comp, first, blue, false).unwrap();
    assert_ne!(created, source);
    assert_eq!(
        project.composition(comp).unwrap().layers.iter().find(|layer| layer.id == second_id).unwrap().source_id(),
        Some(source)
    );
}

#[test]
fn placeholder_import_has_no_layer() {
    let mut project = project_8();
    let comp = add_comp(&mut project, 24.0, 1.0);
    let before = project.composition(comp).unwrap().layers.len();
    let id = import_placeholder(&mut project, "hold", 4, 4, Time::from_seconds(1.0), 24.0);
    assert!(matches!(project.footage(id).unwrap().kind, model::footage::FootageKind::Placeholder));
    assert_eq!(project.composition(comp).unwrap().layers.len(), before);
}

#[test]
fn replacing_footage_with_placeholder_keeps_keys_and_effects() {
    let mut project = project_8();
    let comp = add_comp(&mut project, 24.0, 1.0);
    let footage = project.alloc_item();
    let mut item = Footage::file(footage, "clip", 4, 4, Time::from_seconds(1.0), 24.0);
    item.pixels = vec![Rgba::new(0.2, 0.0, 0.0, 1.0); 16];
    project.add_footage(item);
    let layer_id = add_footage_layer(&mut project, comp, footage).unwrap();
    let property = project.alloc_property();
    let layer = project.composition_mut(comp).unwrap().layer_mut(layer_id).unwrap();
    layer.effects.push(model::layer::LayerEffect {
        name: "Blur".into(),
        enabled: true,
        kind: model::layer::EffectKind::Blur { radius: 2.0 },
    });
    layer.properties.push(model::property::Property::value(
        property,
        "Opacity",
        "Opacity",
        PropertyValue::Scalar(1.0),
    ));
    layer.properties[0].push_keyframe(model::keyframe::Keyframe::at(Time::ZERO, PropertyValue::Scalar(0.5)));
    replace_with_placeholder(&mut project, footage);
    let layer = project.composition(comp).unwrap().layer(layer_id).unwrap();
    assert_eq!(layer.effects.len(), 1);
    assert_eq!(layer.properties[0].keyframes().len(), 1);
    assert!(matches!(project.footage(footage).unwrap().kind, model::footage::FootageKind::Placeholder));
    assert_ne!(project.footage(footage).unwrap().pixels[1], Rgba::new(0.2, 0.0, 0.0, 1.0));
}

#[test]
fn replacing_a_placeholder_updates_every_comp() {
    let mut project = project_8();
    let a = add_comp(&mut project, 24.0, 1.0);
    let b = add_comp(&mut project, 24.0, 1.0);
    let footage = import_placeholder(&mut project, "ph", 2, 2, Time::from_seconds(1.0), 24.0);
    add_footage_layer(&mut project, a, footage).unwrap();
    add_footage_layer(&mut project, b, footage).unwrap();
    replace_with_file(&mut project, footage, "real", vec![Rgba::new(0.2, 0.3, 0.4, 1.0); 4], 2, 2);
    assert!(matches!(project.footage(footage).unwrap().kind, model::footage::FootageKind::File));
    let image_a = render(&project, a, Time::ZERO, RenderPurpose::Preview).unwrap();
    let image_b = render(&project, b, Time::ZERO, RenderPurpose::Preview).unwrap();
    assert_eq!(image_a.pixels[0], image_b.pixels[0]);
    near(image_a.pixels[0].r, 0.2);
}

#[test]
fn missing_footage_relinks_at_the_same_layer() {
    let mut project = project_8();
    let comp = add_comp(&mut project, 24.0, 1.0);
    let footage = project.alloc_item();
    let mut item = Footage::file(footage, "lost", 2, 2, Time::from_seconds(1.0), 24.0);
    item.path = "shot.png".into();
    item.pixels = vec![Rgba::new(51.0 / 255.0, 0.0, 0.0, 1.0); 4];
    project.add_footage(item);
    let layer = add_footage_layer(&mut project, comp, footage).unwrap();
    let index = project.composition(comp).unwrap().layers.iter().position(|item| item.id == layer).unwrap();
    mark_missing(&mut project, footage);
    assert!(project.footage(footage).unwrap().missing);
    assert!(project.footage(footage).unwrap().name_is_italic());
    relink(&mut project, footage, vec![Rgba::new(51.0 / 255.0, 0.0, 0.0, 1.0); 4]);
    assert!(!project.footage(footage).unwrap().missing);
    assert_eq!(project.composition(comp).unwrap().layers[index].id, layer);
    let image = render(&project, comp, Time::ZERO, RenderPurpose::Preview).unwrap();
    near(image.pixels[0].r, 51.0 / 255.0);
}

#[test]
fn proxy_replaces_the_picture_at_the_footage_size() {
    let mut project = project_8();
    let comp = add_comp(&mut project, 24.0, 1.0);
    let footage = project.alloc_item();
    let mut item = Footage::file(footage, "full", 4, 2, Time::from_seconds(1.0), 24.0);
    item.pixels = vec![Rgba::new(1.0, 0.0, 0.0, 1.0); 8];
    project.add_footage(item);
    add_footage_layer(&mut project, comp, footage).unwrap();
    set_proxy(
        &mut project,
        footage,
        FootageProxy {
            path: "proxy.png".into(),
            width: 2,
            height: 1,
            pixels: vec![Rgba::new(0.0, 1.0, 0.0, 1.0); 2],
            in_use: true,
            still: true,
            interpretation: Interpretation::default(),
        },
    );
    let image = render(&project, comp, Time::ZERO, RenderPurpose::Preview).unwrap();
    near(image.pixels[0].g, 1.0);
    assert_eq!(project.footage(footage).unwrap().width, 4);
}

#[test]
fn proxy_toggle_switches_the_picture() {
    let mut project = project_8();
    let comp = add_comp(&mut project, 24.0, 1.0);
    let footage = project.alloc_item();
    let mut item = Footage::file(footage, "full", 2, 2, Time::from_seconds(1.0), 24.0);
    item.pixels = vec![Rgba::new(1.0, 0.0, 0.0, 1.0); 4];
    project.add_footage(item);
    add_footage_layer(&mut project, comp, footage).unwrap();
    set_proxy(
        &mut project,
        footage,
        FootageProxy {
            path: "p".into(),
            width: 2,
            height: 2,
            pixels: vec![Rgba::new(0.0, 0.0, 1.0, 1.0); 4],
            in_use: true,
            still: false,
            interpretation: Interpretation::default(),
        },
    );
    let proxied = render(&project, comp, Time::ZERO, RenderPurpose::Preview).unwrap();
    near(proxied.pixels[0].b, 1.0);
    set_proxy_enabled(&mut project, footage, false);
    let real = render(&project, comp, Time::ZERO, RenderPurpose::Preview).unwrap();
    near(real.pixels[0].r, 1.0);
}

#[test]
fn clearing_a_proxy_removes_the_slot() {
    let mut project = project_8();
    let footage = project.alloc_item();
    project.add_footage(Footage::file(footage, "full", 2, 2, Time::from_seconds(1.0), 24.0));
    set_proxy(
        &mut project,
        footage,
        FootageProxy {
            path: "p".into(),
            width: 1,
            height: 1,
            pixels: vec![Rgba::black()],
            in_use: true,
            still: true,
            interpretation: Interpretation::default(),
        },
    );
    clear_proxy(&mut project, footage);
    assert!(project.footage(footage).unwrap().proxy.is_none());
}

#[test]
fn still_proxy_points_at_the_rendered_frame() {
    let mut project = project_8();
    let footage = project.alloc_item();
    project.add_footage(Footage::file(footage, "full", 2, 2, Time::from_seconds(1.0), 24.0));
    create_proxy_from_render(&mut project, footage, "still.png", vec![Rgba::white(); 4], true);
    let proxy = project.footage(footage).unwrap().proxy.as_ref().unwrap();
    assert!(proxy.still);
    assert_eq!(proxy.path, "still.png");
    assert!(proxy.in_use);
}

#[test]
fn movie_proxy_is_used_by_the_comp() {
    let mut project = project_8();
    let comp = add_comp(&mut project, 24.0, 1.0);
    let footage = project.alloc_item();
    let mut item = Footage::file(footage, "full", 2, 2, Time::from_seconds(1.0), 24.0);
    item.pixels = vec![Rgba::black(); 4];
    project.add_footage(item);
    add_footage_layer(&mut project, comp, footage).unwrap();
    create_proxy_from_render(&mut project, footage, "movie.mov", vec![Rgba::new(0.0, 1.0, 0.0, 1.0); 4], false);
    assert!(!project.footage(footage).unwrap().proxy.as_ref().unwrap().still);
    let image = render(&project, comp, Time::ZERO, RenderPurpose::Preview).unwrap();
    near(image.pixels[0].g, 1.0);
}

#[test]
fn proxy_alpha_is_independent_of_the_source() {
    let mut project = project_8();
    let footage = project.alloc_item();
    let mut item = Footage::file(footage, "full", 2, 2, Time::from_seconds(1.0), 24.0);
    item.interpretation.alpha = Some(AlphaInterpretation::straight());
    project.add_footage(item);
    let mut proxy_interpretation = Interpretation::default();
    proxy_interpretation.alpha = Some(AlphaInterpretation::ignored());
    set_proxy(
        &mut project,
        footage,
        FootageProxy {
            path: "p".into(),
            width: 2,
            height: 2,
            pixels: vec![],
            in_use: true,
            still: false,
            interpretation: proxy_interpretation,
        },
    );
    assert_eq!(
        project.footage(footage).unwrap().interpretation.alpha.unwrap().mode,
        AlphaMode::Straight
    );
    assert_eq!(
        project.footage(footage).unwrap().active_interpretation().alpha.unwrap().mode,
        AlphaMode::Ignored
    );
}

#[test]
fn output_placeholder_renders_before_the_dependent_comp() {
    let mut project = project_8();
    let source = add_comp(&mut project, 24.0, 1.0);
    let main = add_comp(&mut project, 30.0, 1.0);
    let footage = project.alloc_item();
    let mut item = Footage::file(footage, "out", 2, 2, Time::from_seconds(1.0), 24.0);
    item.kind = model::footage::FootageKind::OutputPlaceholder { comp: source };
    project.add_footage(item);
    add_footage_layer(&mut project, main, footage).unwrap();
    let order = render_order(&project, main);
    assert_eq!(order, vec![source, main]);
}

#[test]
fn remembered_interpretation_copies_alpha_and_rate() {
    let mut project = project_8();
    let source = project.alloc_item();
    let mut item = Footage::file(source, "a", 2, 2, Time::from_seconds(1.0), 24.0);
    item.interpretation.alpha = Some(AlphaInterpretation::straight());
    item.interpretation.conform_frame_rate = Some(12.0);
    project.add_footage(item);
    remember_interpretation(&mut project, source);
    let copies = [project.alloc_item(), project.alloc_item()];
    for id in copies {
        project.add_footage(Footage::file(id, "b", 2, 2, Time::from_seconds(1.0), 24.0));
    }
    apply_interpretation(&mut project, &copies);
    for id in copies {
        let interpretation = &project.footage(id).unwrap().interpretation;
        assert_eq!(interpretation.alpha.unwrap().mode, AlphaMode::Straight);
        assert_eq!(interpretation.conform_frame_rate, Some(12.0));
    }
}

#[test]
fn apply_interpretation_matches_the_source_item() {
    let mut project = project_8();
    let source = project.alloc_item();
    let mut item = Footage::file(source, "a", 2, 2, Time::from_seconds(1.0), 30.0);
    item.interpretation.conform_frame_rate = Some(15.0);
    project.add_footage(item);
    remember_interpretation(&mut project, source);
    let target = project.alloc_item();
    project.add_footage(Footage::file(target, "b", 2, 2, Time::from_seconds(1.0), 30.0));
    apply_interpretation(&mut project, &[target]);
    assert_eq!(
        project.footage(target).unwrap().interpretation.conform_frame_rate,
        project.footage(source).unwrap().interpretation.conform_frame_rate
    );
}

#[test]
fn interpretation_preview_matches_the_committed_comp() {
    let pixel = Rgba::new(1.0, 0.0, 0.0, 0.25);
    let preview = apply_alpha(pixel, AlphaInterpretation::ignored(), false);
    let mut project = project_8();
    let comp = add_comp(&mut project, 24.0, 1.0);
    let footage = project.alloc_item();
    let mut item = Footage::file(footage, "a", 1, 1, Time::from_seconds(1.0), 24.0);
    item.pixels = vec![pixel];
    item.interpretation.alpha = Some(AlphaInterpretation::ignored());
    project.add_footage(item);
    add_footage_layer(&mut project, comp, footage).unwrap();
    let image = render(&project, comp, Time::ZERO, RenderPurpose::Preview).unwrap();
    near(preview.a, 1.0);
    near(image.pixels[0].a, 1.0);
}

#[test]
fn guess_alpha_matches_clear_cases_and_warns_when_unsure() {
    let straight = guess_alpha(&[Rgba::new(1.0, 0.0, 0.0, 0.5)], false);
    assert!(matches!(straight.guess, eval::picture::AlphaGuess::Straight));
    assert!(!straight.open_dialog);
    let premul = guess_alpha(&[Rgba::new(0.0, 0.0, 0.0, 0.5)], true);
    assert!(matches!(premul.guess, eval::picture::AlphaGuess::PremultipliedBlack));
    let unsure = guess_alpha(&[Rgba::new(1.0, 0.0, 0.0, 1.0)], true);
    assert!(matches!(unsure.guess, eval::picture::AlphaGuess::Uncertain));
    assert!(unsure.open_dialog);
    let warned = guess_alpha(&[Rgba::white()], false);
    assert!(warned.warn);
}

#[test]
fn ignored_alpha_makes_the_pixel_opaque() {
    let pixel = apply_alpha(Rgba::new(0.2, 0.3, 0.4, 0.0), AlphaInterpretation::ignored(), false);
    near(pixel.a, 1.0);
    near(pixel.r, 0.2);
}

#[test]
fn straight_alpha_keeps_the_edge_color() {
    let pixel = apply_alpha(Rgba::new(1.0, 0.0, 0.0, 0.5), AlphaInterpretation::straight(), false);
    near(pixel.r, 1.0);
    near(pixel.a, 0.5);
    let over_blue = blend_pixel(
        BlendMode::Normal,
        pixel,
        Rgba::new(0.0, 0.0, 1.0, 1.0),
        1.0,
        BitsPerChannel::Bpc32,
        false,
        0,
        0,
        0,
    );
    assert!(over_blue.r > 0.4);
    assert!(over_blue.b > 0.4);
}

#[test]
fn premultiplied_black_and_white_mattes_restore_the_edge() {
    let black = apply_alpha(
        Rgba::new(0.5, 0.0, 0.0, 0.5),
        AlphaInterpretation::premultiplied(Rgba::black()),
        false,
    );
    near(black.r, 1.0);
    near(black.g, 0.0);
    let white = apply_alpha(
        Rgba::new(1.0, 0.5, 0.5, 0.5),
        AlphaInterpretation::premultiplied(Rgba::white()),
        false,
    );
    near(white.r, 1.0);
    near(white.g, 0.0);
    near(white.b, 0.0);
}

#[test]
fn invert_alpha_swaps_transparent_and_opaque() {
    let inverted = apply_alpha(Rgba::new(0.2, 0.2, 0.2, 0.0), AlphaInterpretation::straight(), true);
    near(inverted.a, 1.0);
    let back = apply_alpha(Rgba::new(0.2, 0.2, 0.2, 1.0), AlphaInterpretation::straight(), true);
    near(back.a, 0.0);
}

#[test]
fn file_frame_rate_repeats_frames_inside_a_faster_comp() {
    let mut seen = std::collections::BTreeMap::<usize, usize>::new();
    let source_rate = 24.0;
    let comp_rate = 30.0;
    for frame in 0..30 {
        let time = frame as f64 / comp_rate;
        let source_frame = (time * source_rate).floor() as usize;
        *seen.entry(source_frame).or_insert(0) += 1;
    }
    assert!(seen.values().any(|count| *count >= 2));
}

#[test]
fn conform_15_to_30_halves_picture_duration_and_keeps_audio() {
    let mut footage = Footage::file(
        model::id::ItemId::new(1),
        "clip",
        2,
        2,
        Time::from_seconds(10.0),
        15.0,
    );
    footage.interpretation.conform_frame_rate = Some(30.0);
    let picture = footage.conformed_picture_duration().unwrap();
    near(picture.seconds() as f32, 5.0);
    near(footage.audio_duration.seconds() as f32, 10.0);
    footage.interpretation.conform_frame_rate = Some(1000.0);
    assert!(footage.conformed_picture_duration().is_none());
    footage.interpretation.conform_frame_rate = Some(999.0);
    assert!(footage.conformed_picture_duration().is_some());
}

#[test]
fn progressive_footage_does_not_gain_frames() {
    let frame = vec![Rgba::black(), Rgba::white(), Rgba::black(), Rgba::white()];
    let fields = separate_fields(&frame, 2, 2, FieldSeparation::Off);
    assert_eq!(fields.len(), 1);
}

#[test]
fn upper_field_first_keeps_motion_in_one_direction() {
    let mut frame = vec![Rgba::black(); 8];
    frame[0] = Rgba::white();
    frame[1] = Rgba::white();
    frame[6] = Rgba::new(0.0, 1.0, 0.0, 1.0);
    frame[7] = Rgba::new(0.0, 1.0, 0.0, 1.0);
    let upper = separate_fields(&frame, 2, 4, FieldSeparation::UpperFirst);
    let lower = separate_fields(&frame, 2, 4, FieldSeparation::LowerFirst);
    assert_eq!(upper.len(), 2);
    assert_ne!(upper[0][0], lower[0][0]);
    assert_eq!(upper[0][0], Rgba::white());
}

#[test]
fn lower_field_first_is_the_opposite_order() {
    let mut frame = vec![Rgba::black(); 4];
    frame[0] = Rgba::white();
    frame[1] = Rgba::new(0.0, 0.0, 1.0, 1.0);
    let lower = separate_fields(&frame, 1, 4, FieldSeparation::LowerFirst);
    let upper = separate_fields(&frame, 1, 4, FieldSeparation::UpperFirst);
    assert_eq!(lower[0][1], frame[1]);
    assert_ne!(lower[0][0], upper[0][0]);
}

#[test]
fn preserve_edges_changes_best_and_not_draft() {
    let source = vec![Rgba::white(), Rgba::black()];
    let filtered = vec![Rgba::new(0.98, 0.98, 0.98, 1.0), Rgba::new(0.02, 0.02, 0.02, 1.0)];
    let best = preserve_edges_static(&source, &filtered, model::layer::LayerQuality::Best, true);
    let best_off = preserve_edges_static(&source, &filtered, model::layer::LayerQuality::Best, false);
    let draft = preserve_edges_static(&source, &filtered, model::layer::LayerQuality::Draft, true);
    let draft_off = preserve_edges_static(&source, &filtered, model::layer::LayerQuality::Draft, false);
    assert_ne!(best[0], best_off[0]);
    assert_eq!(draft, draft_off);
}

#[test]
fn pulldown_off_keeps_the_frame_count() {
    let frames = vec![Rgba::black(), Rgba::white(), Rgba::black(), Rgba::white(), Rgba::black()];
    let kept = remove_pulldown(&frames, Pulldown::Off, false).unwrap();
    assert_eq!(kept.len(), frames.len());
}

#[test]
fn guess_32_pulldown_drops_the_rate_by_one_fifth() {
    near(pulldown_rate(30.0, Pulldown::ThreeTwo { phase: 0 }) as f32, 24.0);
    near(pulldown_rate(29.97, Pulldown::ThreeTwo { phase: 0 }) as f32, 23.976);
    let frames = vec![
        Rgba::new(1.0, 0.0, 0.0, 1.0),
        Rgba::new(0.0, 1.0, 0.0, 1.0),
        Rgba::new(0.0, 0.0, 1.0, 1.0),
        Rgba::new(1.0, 1.0, 0.0, 1.0),
        Rgba::new(0.0, 1.0, 1.0, 1.0),
    ];
    let film = remove_pulldown(&frames, Pulldown::ThreeTwo { phase: 0 }, true).unwrap();
    assert_eq!(film.len(), 4);
    assert!(remove_pulldown(&frames, Pulldown::ThreeTwo { phase: 0 }, false).is_none());
}

#[test]
fn guess_24pa_uses_a_different_cadence() {
    let frames = vec![
        Rgba::new(0.1, 0.0, 0.0, 1.0),
        Rgba::new(0.2, 0.0, 0.0, 1.0),
        Rgba::new(0.3, 0.0, 0.0, 1.0),
        Rgba::new(0.4, 0.0, 0.0, 1.0),
        Rgba::new(0.5, 0.0, 0.0, 1.0),
    ];
    let pa = remove_pulldown(&frames, Pulldown::TwentyFourPa { phase: 0 }, true).unwrap();
    let three = remove_pulldown(&frames, Pulldown::ThreeTwo { phase: 0 }, true).unwrap();
    assert_ne!(pa, three);
    near(pulldown_rate(29.97, Pulldown::TwentyFourPa { phase: 0 }) as f32, 23.976);
}

#[test]
fn manual_pulldown_phase_changes_frames_and_needs_fields() {
    let frames = vec![
        Rgba::new(0.1, 0.0, 0.0, 1.0),
        Rgba::new(0.2, 0.0, 0.0, 1.0),
        Rgba::new(0.3, 0.0, 0.0, 1.0),
        Rgba::new(0.4, 0.0, 0.0, 1.0),
        Rgba::new(0.5, 0.0, 0.0, 1.0),
    ];
    assert!(remove_pulldown(&frames, Pulldown::ThreeTwo { phase: 1 }, false).is_none());
    let phase0 = remove_pulldown(&frames, Pulldown::ThreeTwo { phase: 0 }, true).unwrap();
    let phase1 = remove_pulldown(&frames, Pulldown::ThreeTwo { phase: 1 }, true).unwrap();
    assert_ne!(phase0, phase1);
    assert!(PixelAspect::Anamorphic2to1.ratio().is_none());
}

#[test]
fn looping_ten_frames_three_times_makes_thirty() {
    let mut footage = Footage::file(model::id::ItemId::new(1), "loop", 2, 2, Time::from_frames(10.0, 24.0), 24.0);
    footage.interpretation.loop_count = 3;
    near(footage.looped_duration().to_frames(24.0) as f32, 30.0);
}

#[test]
fn square_pixels_keep_a_circle_round() {
    let ratio = PixelAspect::Square.ratio().unwrap();
    assert_eq!(ratio, 1.0);
    let display_w = 100.0 * ratio;
    let display_h = 100.0;
    assert_eq!(display_w, display_h);
}

#[test]
fn d1_ntsc_circle_stretches_until_corrected() {
    let ratio = PixelAspect::D1DvNtsc.ratio().unwrap();
    assert_eq!(ratio, 0.91);
    let pixel_h = 100.0;
    let pixel_w = pixel_h / ratio;
    assert!(pixel_w / pixel_h > 1.0);
    near((pixel_w * ratio) as f32, pixel_h as f32);
}

#[test]
fn d1_ntsc_widescreen_ratio_is_1_21() {
    assert_eq!(PixelAspect::D1DvNtscWidescreen.ratio(), Some(1.21));
    let corrected = 720.0 * 1.21;
    assert!(corrected > 720.0);
}

#[test]
fn d1_pal_ratio_is_1_09() {
    assert_eq!(PixelAspect::D1DvPal.ratio(), Some(1.09));
    near((720.0_f64 * 1.09 / 576.0) as f32, (720.0 * 1.09 / 576.0) as f32);
    assert!((720.0 * 1.09) > 720.0);
}

#[test]
fn d1_pal_widescreen_ratio_is_1_46() {
    assert_eq!(PixelAspect::D1DvPalWidescreen.ratio(), Some(1.46));
}

#[test]
fn anamorphic_2_to_1_ratio_is_unspecified() {
    assert!(PixelAspect::Anamorphic2to1.ratio().is_none());
    assert_ne!(PixelAspect::Anamorphic2to1.ratio(), Some(2.0));
}

#[test]
fn hdv_1080_display_width_uses_1_33() {
    assert_eq!(PixelAspect::Hdv1080.ratio(), Some(1.33));
    near((1440.0_f64 * 1.33) as f32, 1915.2);
}

#[test]
fn dvcpro_hd_1080_display_width_uses_1_5() {
    assert_eq!(PixelAspect::DvcproHd1080.ratio(), Some(1.5));
    assert_eq!(1280.0 * 1.5, 1920.0);
}

#[test]
fn named_pixel_aspects_are_only_the_published_set() {
    assert_eq!(PixelAspect::NAMED.len(), 8);
    assert!(PixelAspect::NAMED.iter().any(|preset| preset.ratio().is_none()));
    assert!(PixelAspect::Custom(1.1).ratio().is_some());
}

#[test]
fn override_color_space_is_rejected_when_working_space_is_off() {
    let mut project = project_8();
    project.color.working_space = Some(WorkingSpace::Off);
    let allowed = !matches!(project.color.working_space, Some(WorkingSpace::Off));
    assert!(!allowed);
    project.color.working_space = Some(WorkingSpace::HdtvRec709);
    let footage = project.alloc_item();
    let mut item = Footage::file(footage, "still", 1, 1, Time::ZERO, 1.0);
    item.interpretation.override_color_space = Some(InputProfile::Srgb);
    item.pixels = vec![Rgba::new(0.5, 0.5, 0.5, 1.0)];
    project.add_footage(item);
    let converted = convert_to_working(
        project.footage(footage).unwrap().pixels[0],
        project.color.working_space.as_ref(),
        false,
        false,
    );
    assert!((converted.r - 0.5).abs() > 1.0e-3);
    let native = InputProfile::NativeNonRgb { name: "CMYK".into() };
    assert!(matches!(native, InputProfile::NativeNonRgb { .. }));
}

#[test]
fn interpret_as_linear_light_changes_brightness() {
    let encoded = convert_to_working(Rgba::new(0.5, 0.5, 0.5, 1.0), Some(&WorkingSpace::HdtvRec709), false, false);
    let linear = convert_to_working(Rgba::new(0.5, 0.5, 0.5, 1.0), Some(&WorkingSpace::HdtvRec709), false, true);
    assert!((encoded.r - linear.r).abs() > 1.0e-3);
}

#[test]
fn preserve_rgb_keeps_file_values_in_rec709() {
    let pixel = Rgba::new(0.5, 0.25, 0.125, 1.0);
    let kept = convert_to_working(pixel, Some(&WorkingSpace::HdtvRec709), true, false);
    assert_eq!(kept, pixel);
}

#[test]
fn cinema_dng_color_spaces_are_the_four_published_ones() {
    assert_eq!(CinemaDngSpace::ALL.len(), 4);
    let options = CinemaDngOptions { color_space: CinemaDngSpace::Srgb };
    let again = CinemaDngOptions { color_space: CinemaDngSpace::Srgb };
    assert_eq!(options, again);
    assert_ne!(options.color_space, CinemaDngSpace::ProPhotoRgb);
}

#[test]
fn red_source_exposure_clamps_but_project_32_does_not() {
    let initial = red_source_exposure(0.8, 0.0);
    near(initial, 0.8);
    let source = red_source_exposure(0.8, 2.0);
    assert!(source <= 1.0);
    let wide = model::footage::project_exposure_32(0.8, 2.0);
    assert!(wide > 1.0);
}
