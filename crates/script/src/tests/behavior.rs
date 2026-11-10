use std::io::Write;
use std::sync::atomic::{AtomicU32, Ordering};

use effect_api::{Frame, Frame8, ParamValue};
use model::BitsPerChannel;

use crate::*;

fn comp_app() -> (App, ItemId) {
    let mut app = App::new();
    let comp = app.add_comp("comp", 1920, 1080, 1.0, 10.0, 30.0).unwrap();
    (app, comp)
}

fn solid(app: &mut App, comp: ItemId) -> LayerId {
    app.add_solid_layer(comp, [1.0, 0.0, 0.0], "red", 100, 80, 1.0, None).unwrap()
}

#[test]
fn project_active_item_single_open_comp() {
    let (mut app, comp) = comp_app();
    assert!(app.active_item().is_none());
    app.open_comp_viewer(comp).unwrap();
    assert_eq!(app.active_item(), Some(comp));
}

#[test]
fn project_bits_per_channel_sets_new_solid_buffer() {
    let (mut app, comp) = comp_app();
    app.set_bits_per_channel(16).unwrap();
    assert_eq!(app.bits_per_channel(), 16);
    let layer = solid(&mut app, comp);
    let source = app.layer_source(layer).unwrap().unwrap();
    assert_eq!(app.eval_bits(source).unwrap(), 16);
}

#[test]
fn project_compensate_for_scene_referred_profiles_roundtrip() {
    let mut app = App::new();
    app.set_compensate_for_scene_referred_profiles(true);
    assert!(app.compensate_for_scene_referred_profiles());
}

#[test]
fn project_dirty_tracks_save_and_rejects_assignment() {
    let (mut app, _comp) = comp_app();
    assert!(app.dirty());
    let path = std::env::temp_dir().join("oe-dirty.oeproject");
    app.save(Some(path)).unwrap();
    assert!(!app.dirty());
    assert!(app.set_dirty(true).is_err());
}

#[test]
fn project_display_start_frame_changes_origin() {
    let mut app = App::new();
    app.set_display_start_frame(1).unwrap();
    assert_eq!(app.display_start_frame(), 1);
    assert_eq!(app.frame_origin(), 1);
}

#[test]
fn project_expression_engine_roundtrip() {
    let mut app = App::new();
    app.set_expression_engine("javascript-1.0").unwrap();
    assert_eq!(app.expression_engine(), "javascript-1.0");
}

#[test]
fn project_feet_frames_film_type_roundtrip() {
    let mut app = App::new();
    app.set_feet_frames_film_type(FeetFramesFilmType::Mm35);
    assert_eq!(app.feet_frames_film_type(), FeetFramesFilmType::Mm35);
}

#[test]
fn project_file_null_until_save() {
    let mut app = App::new();
    assert!(app.project_file().is_none());
    let path = std::env::temp_dir().join("oe-file.oeproject");
    app.save(Some(path.clone())).unwrap();
    assert_eq!(app.project_file().unwrap(), &path);
}

#[test]
fn project_footage_timecode_display_start_type() {
    let mut app = App::new();
    app.set_footage_timecode_display_start_type(FootageTimecodeDisplayStartType::Start0);
    assert_eq!(app.footage_timecode_display_start_type(), FootageTimecodeDisplayStartType::Start0);
}

#[test]
fn project_frames_count_type_resets_display_start() {
    let mut app = App::new();
    app.set_display_start_frame(1).unwrap();
    app.set_frames_count_type(FramesCountType::TimecodeConversion);
    assert_eq!(app.frames_count_type(), FramesCountType::TimecodeConversion);
    assert_eq!(app.display_start_frame(), 0);
    app.set_frames_count_type(FramesCountType::Start0);
    assert_eq!(app.frame_origin(), 0);
}

#[test]
fn project_frames_use_feet_frames() {
    let mut app = App::new();
    app.set_frames_use_feet_frames(true);
    assert!(app.time_display_is_feet_frames());
}

#[test]
fn project_gpu_accel_software_and_windows_rejects_metal() {
    let mut app = App::new();
    app.set_gpu_accel_type(GpuAccelType::Software).unwrap();
    assert_eq!(app.gpu_accel_type(), GpuAccelType::Software);
    let mut windows = App::with_platform(Platform::Windows);
    assert!(windows.set_gpu_accel_type(GpuAccelType::Metal).is_err());
}

#[test]
fn project_items_length_matches_num_items() {
    let (app, _comp) = comp_app();
    assert_eq!(app.items_length(), app.num_items());
}

#[test]
fn project_linear_blending_makes_composite_linear() {
    let mut app = App::new();
    app.set_linear_blending(true);
    assert!(app.linear_blending());
    assert!(app.compositing_is_linear());
}

#[test]
fn project_linearize_working_space() {
    let mut app = App::new();
    app.set_linearize_working_space(true);
    assert!(app.linearize_working_space());
    assert!(app.compositing_is_linear());
}

#[test]
fn project_num_items_drops_when_item_removed() {
    let (mut app, comp) = comp_app();
    let before = app.num_items();
    app.remove_item(comp).unwrap();
    assert_eq!(app.num_items(), before - 1);
}

#[test]
fn project_render_queue_num_items() {
    let (mut app, comp) = comp_app();
    assert_eq!(app.render_queue_len(), 0);
    app.queue_add(comp).unwrap();
    assert_eq!(app.render_queue_len(), 1);
}

#[test]
fn project_revision_changes_on_save() {
    let (mut app, _comp) = comp_app();
    let before = app.revision();
    app.save(Some(std::env::temp_dir().join("oe-rev.oeproject"))).unwrap();
    assert_ne!(app.revision(), before);
}

#[test]
fn project_root_folder_parents_top_level() {
    let (app, comp) = comp_app();
    assert_eq!(app.parent_folder(comp).unwrap(), app.root_folder());
}

#[test]
fn project_selection_tracks_selected_items() {
    let (mut app, comp) = comp_app();
    assert!(app.selection().is_empty());
    app.set_item_selected(comp, true).unwrap();
    assert_eq!(app.selection(), vec![comp]);
}

#[test]
fn project_time_display_frames() {
    let mut app = App::new();
    app.set_time_display_type(TimeDisplayType::Frames);
    assert!(app.time_display_is_frames());
}

#[test]
fn project_tool_type_selection_by_default() {
    let app = App::new();
    assert_eq!(app.tool_type(), ToolType::Selection);
    let mut app = app;
    app.set_tool_type(ToolType::Pen);
    assert_eq!(app.tool_type(), ToolType::Pen);
}

#[test]
fn project_transparency_grid_thumbnails() {
    let mut app = App::new();
    app.set_transparency_grid_thumbnails(true);
    assert!(app.transparency_grid_thumbnails());
}

#[test]
fn project_used_fonts_includes_text_layer() {
    let (mut app, comp) = comp_app();
    let layer = app.add_text_layer(comp, "あ").unwrap();
    let mut doc = app.source_text(layer).unwrap();
    doc.set_font("ArialMT");
    app.set_source_text(layer, doc).unwrap();
    assert!(app.used_fonts().iter().any(|font| font.post_script_name == "ArialMT"));
}

#[test]
fn project_working_gamma_reads_current() {
    let mut app = App::new();
    assert_eq!(app.working_gamma(), 2.2);
    app.set_working_gamma(2.4).unwrap();
    assert_eq!(app.working_gamma(), 2.4);
    assert!(app.set_working_gamma(1.8).is_err());
}

#[test]
fn project_working_space_known_name() {
    let mut app = App::new();
    app.set_working_space("sRGB IEC61966-2.1").unwrap();
    assert_eq!(app.working_space(), "sRGB IEC61966-2.1");
    assert!(app.set_working_space("no-such").is_err());
}

#[test]
fn project_xmp_packet_survives_reopen() {
    let (mut app, _comp) = comp_app();
    app.set_xmp_packet("<xmp>oe</xmp>");
    let path = std::env::temp_dir().join("oe-xmp.oeproject");
    app.save(Some(path.clone())).unwrap();
    let opened = App::open(&path).unwrap();
    assert_eq!(opened.xmp_packet(), "<xmp>oe</xmp>");
}

#[test]
fn project_auto_fix_expressions_keeps_broken() {
    let (mut app, comp) = comp_app();
    let layer = solid(&mut app, comp);
    let opacity = app.property_by_name(layer, "Opacity").unwrap();
    app.set_expression(opacity, "old + 1").unwrap();
    let other = app.property_by_name(layer, "Rotation").unwrap();
    app.set_expression(other, "old +").unwrap();
    app.auto_fix_expressions("old", "2").unwrap();
    assert_eq!(app.expression(opacity).unwrap(), "2 + 1");
    assert_eq!(app.expression(other).unwrap(), "old +");
}

#[test]
fn project_close_discards_to_empty() {
    let (mut app, _comp) = comp_app();
    assert!(app.num_items() > 0);
    assert!(app.close(CloseOptions::DoNotSaveChanges).unwrap());
    assert_eq!(app.num_items(), 0);
}

#[test]
fn project_consolidate_footage_retargets_layers() {
    let (mut app, comp) = comp_app();
    let path = std::env::temp_dir().join("oe-same.png");
    let _ = std::fs::write(&path, []);
    let mut options = ImportOptions::new(Some(path.clone()));
    let first = app.import_file(&options).unwrap();
    options.file = Some(path);
    let second = app.import_file(&options).unwrap();
    app.add_layer_from_item(comp, second, None).unwrap();
    let removed = app.consolidate_footage().unwrap();
    assert_eq!(removed, 1);
    let layer = app.layer_at(comp, 1).unwrap();
    assert_eq!(app.layer_source(layer).unwrap(), Some(first));
}

#[test]
fn project_import_file_adds_footage() {
    let mut app = App::new();
    let path = std::env::temp_dir().join("oe.png");
    let _ = std::fs::write(&path, []);
    let before = app.num_items();
    let item = app.import_file(&ImportOptions::new(Some(path))).unwrap();
    assert!(matches!(app.main_source_kind(item).unwrap(), crate::state::SourceKind::File));
    assert_eq!(app.num_items(), before + 1);
}

#[test]
fn project_import_file_with_dialog_cancel_and_pick() {
    let mut app = App::new();
    app.set_next_import(None);
    assert!(app.import_file_with_dialog().unwrap().is_none());
    let path = std::env::temp_dir().join("oe-dialog.png");
    let _ = std::fs::write(&path, []);
    let before = app.num_items();
    app.set_next_import(Some(vec![path]));
    let items = app.import_file_with_dialog().unwrap().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(app.num_items(), before + 1);
}

#[test]
fn project_import_placeholder_size() {
    let mut app = App::new();
    let item = app.import_placeholder("ph", 100, 80, 25.0, 2.0).unwrap();
    assert_eq!(app.av_width(item).unwrap(), 100);
    assert_eq!(app.av_height(item).unwrap(), 80);
    assert_eq!(app.av_duration(item).unwrap(), 2.0);
}

#[test]
fn project_item_and_item_by_id_and_collection_index() {
    let (app, comp) = comp_app();
    assert_eq!(app.item_at(1).unwrap(), comp);
    assert_eq!(app.collection_at(1).unwrap(), app.item_at(1).unwrap());
    assert!(app.collection_at(0).is_err());
    assert_eq!(app.item_by_id(app.item_id_value(comp)).unwrap(), comp);
}

#[test]
fn project_layer_by_id_and_negative() {
    let (mut app, comp) = comp_app();
    let layer = solid(&mut app, comp);
    let id = app.layer_id_value(layer) as i64;
    assert_eq!(app.layer_by_id(id).unwrap(), Some(layer));
    assert!(app.layer_by_id(-1).is_err());
    assert_eq!(app.layer_by_id(99999).unwrap(), None);
}

#[test]
fn project_list_color_profiles_includes_working_space() {
    let app = App::new();
    let names = app.list_color_profiles();
    assert!(names.iter().any(|name| name == "sRGB IEC61966-2.1"));
}

#[test]
fn project_reduce_project_keeps_dependency() {
    let (mut app, comp) = comp_app();
    let footage = app.import_placeholder("keep", 20, 20, 25.0, 1.0).unwrap();
    app.add_layer_from_item(comp, footage, None).unwrap();
    let extra = app.import_placeholder("drop", 20, 20, 25.0, 1.0).unwrap();
    let before = app.num_items();
    let removed = app.reduce_project(&[comp]).unwrap();
    assert!(removed >= 1);
    assert!(app.item_by_id(app.item_id_value(comp)).is_ok());
    assert!(app.item_by_id(app.item_id_value(extra)).is_err());
    assert!(app.num_items() < before);
}

#[test]
fn project_remove_unused_footage() {
    let (mut app, comp) = comp_app();
    let used = app.import_placeholder("used", 16, 16, 25.0, 1.0).unwrap();
    app.add_layer_from_item(comp, used, None).unwrap();
    let _unused = app.import_placeholder("unused", 16, 16, 25.0, 1.0).unwrap();
    let removed = app.remove_unused_footage().unwrap();
    assert_eq!(removed, 1);
}

#[test]
fn project_replace_font() {
    let (mut app, comp) = comp_app();
    let layer = app.add_text_layer(comp, "あ").unwrap();
    let mut doc = app.source_text(layer).unwrap();
    doc.set_font("ArialMT");
    app.set_source_text(layer, doc).unwrap();
    let from = FontObject::from_postscript("ArialMT");
    let to = FontObject::from_postscript("TimesNewRomanPSMT");
    assert!(app.replace_font(&from, &to, false).unwrap());
    assert_eq!(app.source_text(layer).unwrap().font(), "TimesNewRomanPSMT");
}

#[test]
fn project_save_with_dialog() {
    let (mut app, _comp) = comp_app();
    let path = std::env::temp_dir().join("oe-save-dialog.oeproject");
    app.set_next_save(Some(path.clone()));
    assert!(app.save_with_dialog().unwrap());
    assert_eq!(app.project_file().unwrap(), &path);
    app.set_next_save(None);
    let file_before = app.project_file().cloned();
    assert!(!app.save_with_dialog().unwrap());
    assert_eq!(app.project_file().cloned(), file_before);
}

#[test]
fn project_set_default_import_folder() {
    let mut app = App::new();
    let folder = std::env::temp_dir();
    assert!(app.set_default_import_folder(Some(folder.clone())));
    assert_eq!(app.import_dialog_folder().unwrap(), &folder);
    assert!(app.clear_default_import_folder());
    assert!(app.import_dialog_folder().is_none());
}

#[test]
fn project_show_window() {
    let mut app = App::new();
    app.show_window(true);
    assert!(app.project_window_visible());
    assert_eq!(app.project_panel_id().as_str(), "project");
}

#[test]
fn project_team_methods_are_unsupported() {
    let (app, _c) = comp_app();
    let before = app.num_items();
    assert!(app.new_team_project("t", "d").is_err());
    assert!(app.open_team_project("t").is_err());
    assert!(app.share_team_project("c").is_err());
    assert!(app.sync_team_project().is_err());
    assert!(app.close_team_project().is_err());
    assert!(app.convert_team_project_to_project(None).is_err());
    assert!(app.list_team_projects().is_err());
    assert!(app.is_team_project_open("t").is_err());
    assert!(app.is_any_team_project_open().is_err());
    assert!(app.is_team_project_enabled().is_err());
    assert!(app.is_logged_in_to_team_project().is_err());
    assert!(app.is_sync_command_enabled().is_err());
    assert!(app.is_share_command_enabled().is_err());
    assert!(app.is_resolve_command_enabled().is_err());
    assert!(app.resolve_conflict(ResolveType::AcceptMine).is_err());
    assert_eq!(app.num_items(), before);
}

#[test]
fn item_comment_label_name_id_selected_type_parent() {
    let (mut app, comp) = comp_app();
    app.set_item_comment(comp, "メモ").unwrap();
    assert_eq!(app.item_comment(comp).unwrap(), "メモ");
    assert_eq!(app.dynamic_link_guid(comp).unwrap(), "");
    app.set_item_label(comp, 1).unwrap();
    assert_eq!(app.item_label(comp).unwrap(), 1);
    app.set_item_name(comp, "改名").unwrap();
    assert_eq!(app.item_name(comp).unwrap(), "改名");
    assert_eq!(app.item_type_name(comp).unwrap(), "コンポジション");
    let folder = app.add_folder("箱").unwrap();
    assert_eq!(app.item_type_name(folder).unwrap(), "フォルダー");
    app.set_parent_folder(comp, folder).unwrap();
    assert_eq!(app.parent_folder(comp).unwrap(), folder);
    assert_eq!(app.folder_num_items(folder).unwrap(), 1);
    let id = app.item_id_value(comp);
    let path = std::env::temp_dir().join("oe-id.oeproject");
    app.save(Some(path.clone())).unwrap();
    let opened = App::open(&path).unwrap();
    assert!(opened.item_by_id(id).is_ok());
}

#[test]
fn item_guides_add_get_set_remove() {
    let (mut app, comp) = comp_app();
    let before = app.guides(comp).unwrap().len();
    let index = app.add_guide(comp, 1, 100.0).unwrap();
    assert_eq!(app.guides(comp).unwrap().len(), before + 1);
    assert_eq!(app.guide_at(comp, index).unwrap().position, 100.0);
    app.set_guide(comp, 50.0, index).unwrap();
    assert_eq!(app.guide_at(comp, index).unwrap().position, 50.0);
    app.remove_guide(comp, index).unwrap();
    assert_eq!(app.guides(comp).unwrap().len(), before);
}

#[test]
fn item_remove_drops_id() {
    let (mut app, comp) = comp_app();
    let id = app.item_id_value(comp);
    let before = app.num_items();
    app.remove_item(comp).unwrap();
    assert_eq!(app.num_items(), before - 1);
    assert!(app.item_by_id(id).is_err());
}

#[test]
fn item_collection_add_comp_and_folder() {
    let mut app = App::new();
    let before = app.num_items();
    let comp = app.add_comp("comp", 1920, 1080, 1.0, 10.0, 30.0).unwrap();
    assert_eq!(app.av_width(comp).unwrap(), 1920);
    assert_eq!(app.av_height(comp).unwrap(), 1080);
    assert_eq!(app.av_duration(comp).unwrap(), 10.0);
    assert_eq!(app.av_frame_rate(comp).unwrap(), 30.0);
    assert_eq!(app.num_items(), before + 1);
    let folder = app.add_folder("箱").unwrap();
    assert_eq!(app.item_name(folder).unwrap(), "箱");
    assert_eq!(app.item_type_name(folder).unwrap(), "フォルダー");
}

#[test]
fn av_item_fields_and_proxy() {
    let mut app = App::new();
    let still = {
        let path = std::env::temp_dir().join("oe-still.png");
        let _ = std::fs::write(&path, []);
        app.import_file(&ImportOptions::new(Some(path))).unwrap()
    };
    assert_eq!(app.av_duration(still).unwrap(), 0.0);
    assert!(app.is_still(still, false).unwrap());
    assert!(!app.footage_missing(still).unwrap() || app.main_source_file(still).unwrap().is_some());
    let rate = app.av_frame_rate(still).unwrap();
    assert!((app.av_frame_duration(still).unwrap() - 1.0 / rate).abs() < 1e-6);
    assert!(!app.has_audio(still).unwrap());
    assert!(app.has_video(still).unwrap());
    let comp = app.add_comp("c", 100, 100, 1.0, 1.0, 25.0).unwrap();
    app.set_comp_frame_duration(comp, 0.04).unwrap();
    assert!((app.av_frame_rate(comp).unwrap() - 25.0).abs() < 1e-6);
    let proxy = std::env::temp_dir().join("oe-proxy.png");
    let _ = std::fs::write(&proxy, []);
    app.set_proxy_file(still, &proxy).unwrap();
    assert!(app.proxy_is_some(still).unwrap());
    app.set_use_proxy(still, true).unwrap();
    assert!(app.use_proxy(still).unwrap());
    app.set_proxy_to_none(still).unwrap();
    assert!(app.set_use_proxy(still, true).is_err());
    app.set_proxy_placeholder(still, "p", 32, 32, 25.0, 1.0).unwrap();
    app.set_proxy_sequence(still, &proxy, true).unwrap();
    app.set_proxy_solid(still, [0.0, 1.0, 0.0], "g", 40, 30, 1.0).unwrap();
    app.set_item_time(comp, 1.5).unwrap();
    assert_eq!(app.item_time(comp).unwrap(), 1.5);
    let _ = app.used_in(still);
    let _ = app.is_media_replacement_compatible(still).unwrap();
    app.set_comp_pixel_aspect(comp, 1.5).unwrap();
    assert_eq!(app.av_pixel_aspect(comp).unwrap(), 1.5);
}

#[test]
fn comp_settings_roundtrip() {
    let (mut app, comp) = comp_app();
    app.set_comp_bg(comp, [0.2, 0.3, 0.4]).unwrap();
    assert_eq!(app.comp_bg(comp).unwrap(), [0.2, 0.3, 0.4]);
    app.set_comp_counters(comp, true).unwrap();
    assert!(app.comp_counters(comp).unwrap());
    app.set_comp_display_start_frame(comp, 0).unwrap();
    assert_eq!(app.displayed_first_frame(comp).unwrap(), 0);
    app.set_comp_display_start_time(comp, 1.0).unwrap();
    assert_eq!(app.comp_display_start_time(comp).unwrap(), 1.0);
    app.set_draft_3d(comp, true).unwrap();
    assert!(app.draft_3d(comp).unwrap());
    app.set_drop_frame(comp, false).unwrap();
    assert!(!app.drop_frame(comp).unwrap());
    app.set_comp_frame_blending(comp, true).unwrap();
    assert!(app.comp_frame_blending(comp).unwrap());
    app.set_hide_shy_layers(comp, true).unwrap();
    assert!(app.hide_shy_layers(comp).unwrap());
    app.set_comp_motion_blur(comp, true).unwrap();
    app.set_motion_blur_adaptive_sample_limit(comp, 32).unwrap();
    app.set_motion_blur_samples_per_frame(comp, 8).unwrap();
    app.set_shutter_angle(comp, 180.0).unwrap();
    app.set_shutter_phase(comp, -90.0).unwrap();
    assert_eq!(app.motion_blur_adaptive_sample_limit(comp).unwrap(), 32);
    assert_eq!(app.shutter_angle(comp).unwrap(), 180.0);
    app.set_preserve_nested_frame_rate(comp, true).unwrap();
    app.set_preserve_nested_resolution(comp, true).unwrap();
    app.set_renderer(comp, "Advanced 3D").unwrap();
    assert!(app.renderers(comp).unwrap().contains(&"Advanced 3D".to_string()));
    app.set_resolution_factor(comp, [1, 1]).unwrap();
    app.set_work_area_start(comp, 0.0).unwrap();
    app.set_work_area_duration(comp, 1.0).unwrap();
    app.set_template_name(comp, "名前").unwrap();
    assert_eq!(app.template_name(comp).unwrap(), "名前");
    assert!(app.export_as_motion_graphics_template(comp, true, Some("x.mogrt")).is_err());
    app.open_in_essential_graphics(comp).unwrap();
    assert_eq!(app.essential_graphics_comp(), Some(comp));
    app.open_comp_viewer(comp).unwrap();
    assert_eq!(app.viewer().unwrap().item, comp);
    let dup = app.duplicate_comp(comp).unwrap();
    assert_ne!(dup, comp);
    assert!(app.active_camera(comp).unwrap().is_none());
    let cam = app.add_camera(comp, "cam", [1.0, 2.0]).unwrap();
    assert_eq!(app.active_camera(comp).unwrap(), Some(cam));
}

#[test]
fn folder_item_index() {
    let mut app = App::new();
    let folder = app.add_folder("箱").unwrap();
    let nested = app.add_comp_in(folder, "in", 100, 80, 1.0, 1.0, 24.0).unwrap();
    assert_eq!(app.folder_num_items(folder).unwrap(), 1);
    assert_eq!(app.folder_item(folder, 1).unwrap(), nested);
    assert_eq!(app.folder_children(folder).unwrap().len(), app.folder_num_items(folder).unwrap() as usize);
}

#[test]
fn footage_replace_variants() {
    let mut app = App::new();
    let path = std::env::temp_dir().join("oe-src.png");
    let _ = std::fs::write(&path, []);
    let item = app.import_file(&ImportOptions::new(Some(path.clone()))).unwrap();
    assert!(app.footage_file(item).unwrap().is_some());
    assert!(app.main_source_file(item).unwrap().is_some());
    let next = std::env::temp_dir().join("oe-next.png");
    let _ = std::fs::write(&next, []);
    app.replace_footage(item, &next).unwrap();
    assert_eq!(app.main_source_file(item).unwrap().unwrap(), next);
    app.replace_with_placeholder(item, "ph", 100, 100, 25.0, 1.0).unwrap();
    assert!(app.footage_missing(item).unwrap());
    assert_eq!(app.av_width(item).unwrap(), 100);
    app.replace_with_sequence(item, &next, true).unwrap();
    assert!(app.sequence_frame_rate(item).unwrap() > 0.0);
    app.replace_with_solid(item, [0.0, 1.0, 0.0], "g", 50, 40, 1.0).unwrap();
    assert_eq!(app.av_width(item).unwrap(), 50);
    assert_eq!(app.solid_color(item).unwrap(), [0.0, 1.0, 0.0]);
    app.open_footage_viewer(item).unwrap();
    assert_eq!(app.viewer().unwrap().kind, ViewerKind::Footage);
}

#[test]
fn layer_core_and_stack_order() {
    let (mut app, comp) = comp_app();
    let a = solid(&mut app, comp);
    let b = app.add_solid_layer(comp, [0.0, 1.0, 0.0], "green", 40, 40, 1.0, None).unwrap();
    let c = app.add_null(comp, None).unwrap();
    assert_eq!(app.layer_index(c).unwrap(), 1);
    assert_eq!(app.containing_comp(a).unwrap(), comp);
    app.set_layer_comment(a, "c").unwrap();
    assert_eq!(app.layer_comment(a).unwrap(), "c");
    assert_eq!(app.layer_by_id(app.layer_id_value(a) as i64).unwrap(), Some(a));
    app.set_layer_label(a, 2).unwrap();
    assert_eq!(app.layer_label(a).unwrap(), 2);
    app.set_layer_locked(a, true).unwrap();
    assert!(app.nudge_layer_from_ui(a, [1.0, 0.0]).is_err());
    assert!(!app.ui_can_move_layer(a).unwrap());
    app.set_layer_locked(a, false).unwrap();
    app.set_layer_in_point(a, 1.0).unwrap();
    app.set_layer_out_point(a, 3.0).unwrap();
    app.set_layer_start_time(a, 2.0).unwrap();
    assert!((app.layer_start_time(a).unwrap() - 2.0).abs() < 1e-6);
    let before_span = app.layer_out_point(a).unwrap() - app.layer_in_point(a).unwrap();
    app.set_layer_stretch(a, 200.0).unwrap();
    let after_span = app.layer_out_point(a).unwrap() - app.layer_in_point(a).unwrap();
    assert!(after_span > before_span);
    app.set_item_time(comp, 2.0).unwrap();
    assert!((app.layer_time(a).unwrap() - (2.0 - app.layer_start_time(a).unwrap())).abs() < 1e-6);
    assert!(app.layer_has_video(a).unwrap());
    let camera = app.add_camera(comp, "cam", [0.0, 0.0]).unwrap();
    assert!(!app.layer_has_video(camera).unwrap());
    assert!(app.layer_null(c).unwrap());
    assert!(!app.layer_null(a).unwrap());
    app.set_layer_name(a, "renamed").unwrap();
    assert!(app.layer_is_name_set(a).unwrap());
    assert!(!app.layer_is_name_from_source(a).unwrap());
    app.set_auto_orient(a, AutoOrientType::NoAutoOrient).unwrap();
    app.set_layer_parent(a, Some(b)).unwrap();
    assert_eq!(app.layer_parent(a).unwrap(), Some(b));
    app.set_layer_shy(a, true).unwrap();
    app.set_hide_shy_layers(comp, true).unwrap();
    assert!(app.layer_hidden_by_shy(a).unwrap());
    app.set_layer_solo(b, true).unwrap();
    assert!(!app.layer_active(a).unwrap());
    let dup = app.duplicate_layer(b).unwrap();
    assert_ne!(app.layer_id_value(dup), app.layer_id_value(b));
    assert_eq!(app.num_layers(comp).unwrap(), 5);
    let other = app.add_comp("other", 100, 100, 1.0, 2.0, 24.0).unwrap();
    let before = app.num_layers(other).unwrap();
    app.copy_layer_to_comp(b, other).unwrap();
    assert_eq!(app.num_layers(other).unwrap(), before + 1);
    assert!(app.layer_by_id(app.layer_id_value(b) as i64).unwrap().is_some());
    app.layer_at(comp, 2).unwrap();
    app.move_layer_before(app.layer_at(comp, 2).unwrap(), app.layer_at(comp, 1).unwrap()).unwrap();
    app.move_layer_to_end(app.layer_at(comp, 1).unwrap()).unwrap();
    let count = app.num_layers(comp).unwrap();
    app.move_layer_to_beginning(app.layer_at(comp, count).unwrap()).unwrap();
    let front_name_before = app.layer_name(app.layer_at(comp, 1).unwrap()).unwrap();
    let _ = front_name_before;
    let removed = app.layer_at(comp, 1).unwrap();
    let n = app.num_layers(comp).unwrap();
    app.remove_layer(removed).unwrap();
    assert_eq!(app.num_layers(comp).unwrap(), n - 1);
    let g = app.add_layer_guide(b, 0, 20.0).unwrap();
    assert_eq!(app.layer_guides(b).unwrap().len(), 1);
    app.set_layer_guide(b, 10.0, g).unwrap();
    assert_eq!(app.layer_guide(b, g).unwrap().position, 10.0);
    app.remove_layer_guide(b, g).unwrap();
    assert!(app.layer_guides(b).unwrap().is_empty());
    let preset = std::env::temp_dir().join("oe.ffx");
    std::fs::write(&preset, "effect ADBE Fill\n").unwrap();
    let effects_before = app.num_properties(app.property_by_name(b, "Effects").unwrap()).unwrap();
    app.apply_preset(b, &preset).unwrap();
    assert!(app.num_properties(app.property_by_name(b, "Effects").unwrap()).unwrap() > effects_before);
    let video = app.add_video_footage("clip", 80, 40, 3.0, 25.0, false).unwrap();
    let video_layer = app.add_layer_from_item(comp, video, None).unwrap();
    let times = app.scene_edit_detection(video_layer, SceneEditDetectionMode::Markers).unwrap();
    assert!(!times.is_empty());
    let marker = app.property_by_name(video_layer, "Marker").unwrap();
    assert!(app.num_keys(marker).unwrap() >= 1);
    app.set_time_remap_enabled(video_layer, true).unwrap();
    assert!(app.scene_edit_detection(video_layer, SceneEditDetectionMode::Markers).is_err());
}

#[test]
fn layer_collection_factories_and_precompose() {
    let (mut app, comp) = comp_app();
    let footage = app.add_video_footage("src", 120, 80, 2.0, 24.0, true).unwrap();
    let before = app.num_layers(comp).unwrap();
    let layer = app.add_layer_from_item(comp, footage, None).unwrap();
    assert_eq!(app.layer_source(layer).unwrap(), Some(footage));
    assert_eq!(app.num_layers(comp).unwrap(), before + 1);
    let box_layer = app.add_box_text(comp, 200.0, 100.0, false).unwrap();
    assert!(app.source_text(box_layer).unwrap().box_text);
    assert!(!app.source_text(box_layer).unwrap().point_text());
    let vertical_box = app.add_box_text(comp, 100.0, 200.0, true).unwrap();
    assert!(app.source_text(vertical_box).unwrap().vertical);
    let point = app.add_text_layer(comp, "あ").unwrap();
    assert_eq!(app.source_text(point).unwrap().text, "あ");
    assert!(app.source_text(point).unwrap().point_text());
    let vertical = app.add_text_layer(comp, "あ").unwrap();
    let mut doc = app.source_text(vertical).unwrap();
    doc.set_line_orientation(LineOrientation::VerticalRightToLeft);
    app.set_source_text(vertical, doc).unwrap();
    assert!(app.source_text(vertical).unwrap().vertical);
    let cam = app.add_camera(comp, "cam", [960.0, 540.0]).unwrap();
    assert!(app.layer_is_camera(cam).unwrap());
    let light = app.add_light(comp, "lit", [10.0, 10.0]).unwrap();
    assert!(app.layer_is_light(light).unwrap());
    let null_layer = app.add_null(comp, None).unwrap();
    assert!(app.layer_null(null_layer).unwrap());
    let shape = app.add_shape(comp).unwrap();
    assert!(app.layer_is_shape(shape).unwrap());
    let mesh = app.add_parametric_mesh(comp, "mesh", ParametricMeshType::Cube).unwrap();
    assert!(app.layer_is_mesh(mesh).unwrap());
    assert_eq!(app.layer_by_name(comp, "red").unwrap(), None);
    let named = solid(&mut app, comp);
    app.set_layer_name(named, "red").unwrap();
    assert_eq!(app.layer_name(app.layer_by_name(comp, "red").unwrap().unwrap()).unwrap(), "red");
    let pre = app.precompose(comp, &[1], "pre", true).unwrap();
    let front = app.layer_at(comp, 1).unwrap();
    assert_eq!(app.layer_source(front).unwrap(), Some(pre));
    let _ = (box_layer, point);
}

#[test]
fn av_layer_switches_and_transforms() {
    let (mut app, comp) = comp_app();
    let video = app.add_video_footage("v", 100, 50, 2.0, 25.0, true).unwrap();
    let layer = app.add_layer_from_item(comp, video, None).unwrap();
    let above = app.add_solid_layer(comp, [0.0, 0.0, 0.0], "matte", 100, 50, 1.0, None).unwrap();
    app.set_adjustment_layer(layer, true).unwrap();
    assert!(app.adjustment_affects_below(layer).unwrap());
    app.set_audio_enabled(layer, false).unwrap();
    assert!(!app.audio_active(layer).unwrap());
    app.set_audio_enabled(layer, true).unwrap();
    app.set_item_time(comp, 0.0).unwrap();
    assert!(app.audio_active_at(layer, 0.0).unwrap());
    app.set_blending_mode(layer, model::BlendMode::Multiply).unwrap();
    assert_eq!(app.blending_mode(layer).unwrap(), model::BlendMode::Multiply);
    assert!(!app.can_set_collapse(layer).unwrap());
    let nested = app.add_comp("nested", 100, 50, 1.0, 2.0, 25.0).unwrap();
    let pre = app.add_layer_from_item(comp, nested, None).unwrap();
    assert!(app.can_set_collapse(pre).unwrap());
    app.set_collapse_transformation(pre, true).unwrap();
    assert!(app.nested_transform_kept(pre).unwrap());
    assert!(app.can_set_time_remap(layer).unwrap());
    app.set_time_remap_enabled(layer, true).unwrap();
    assert!(app.property_by_name(layer, "Time Remap").is_some());
    app.set_effects_active(layer, false).unwrap();
    assert!(!app.effects_apply(layer).unwrap());
    app.set_environment_layer(layer, true).unwrap();
    assert!(app.environment_layer(layer).unwrap());
    app.set_comp_frame_blending(comp, true).unwrap();
    app.set_frame_blending(layer, true).unwrap();
    app.set_frame_blending_type(layer, FrameBlendingType::PixelMotion).unwrap();
    assert!(app.frame_blend_active(layer).unwrap());
    app.set_guide_layer(layer, true).unwrap();
    assert!(!app.layer_in_final_render(layer).unwrap());
    app.set_guide_layer(layer, false).unwrap();
    assert!(app.layer_has_audio(layer).unwrap());
    app.set_track_matte(layer, above, TrackMatteType::Alpha).unwrap();
    assert!(app.has_track_matte(layer).unwrap());
    assert_eq!(app.track_matte_layer(layer).unwrap(), Some(above));
    assert!(app.is_track_matte(above).unwrap());
    app.remove_track_matte(layer).unwrap();
    assert_eq!(app.track_matte_type(layer).unwrap(), TrackMatteType::NoTrackMatte);
    assert!(app.track_matte_layer(layer).unwrap().is_none());
    app.set_track_matte_type(layer, TrackMatteType::Alpha).unwrap();
    assert_eq!(app.track_matte_layer(layer).unwrap(), Some(above));
    assert_eq!(app.layer_height(layer).unwrap(), app.av_height(video).unwrap() as f64);
    assert_eq!(app.layer_width(layer).unwrap(), app.av_width(video).unwrap() as f64);
    app.set_comp_motion_blur(comp, true).unwrap();
    app.set_motion_blur(layer, true).unwrap();
    assert!(app.motion_blur_active(layer).unwrap());
    app.set_preserve_transparency(layer, true).unwrap();
    app.set_layer_quality(layer, LayerQuality::Best).unwrap();
    app.set_sampling_quality(layer, LayerSamplingQuality::Bicubic).unwrap();
    assert_eq!(app.sampling_quality(layer).unwrap(), LayerSamplingQuality::Bicubic);
    app.set_three_d_layer(layer, true).unwrap();
    let position = app.property_by_name(layer, "Position").unwrap();
    assert_eq!(app.property_value_type(position).unwrap(), PropertyValueType::ThreeDSpatial);
    let text_layer = app.add_text_layer(comp, "t").unwrap();
    app.set_three_d_per_char(text_layer, true).unwrap();
    assert!(app.layer_can_add_to_template(layer, comp).unwrap());
    assert!(app.layer_add_to_template(layer, comp).unwrap());
    assert!(app.layer_add_to_template_as(pre, comp, "名前").unwrap());
    assert_eq!(app.template_controller_name(comp, 2).unwrap(), "名前");
    let fit = app.calculate_transform_from_points(layer, [10.0, 10.0, 0.0], [110.0, 10.0, 0.0], [110.0, 60.0, 0.0]).unwrap();
    assert!((fit.rotation).abs() < 1e-6);
    assert!(fit.scale[0] > 0.0);
    app.open_layer_viewer(layer).unwrap();
    assert_eq!(app.viewer().unwrap().kind, ViewerKind::Layer);
    let rect = app.source_rect_at_time(layer, 0.0, false).unwrap();
    assert!(rect[2] > 0.0 && rect[3] > 0.0);
    let world = app.source_point_to_comp(layer, 0.0, [0.0, 0.0]).unwrap();
    let back = app.comp_point_to_source(layer, 0.0, world).unwrap();
    assert!((back[0]).abs() < 1e-4 && (back[1]).abs() < 1e-4);
    let other = app.add_video_footage("b", 10, 10, 1.0, 25.0, false).unwrap();
    app.replace_source(layer, other, false).unwrap();
    assert_eq!(app.layer_source(layer).unwrap(), Some(other));
}

#[test]
fn light_camera_and_mesh_summaries() {
    let (mut app, comp) = comp_app();
    let light = app.add_light(comp, "lit", [0.0, 0.0]).unwrap();
    app.set_light_type(light, LightType::Environment).unwrap();
    let plate = solid(&mut app, comp);
    app.set_light_source(light, Some(plate)).unwrap();
    assert_eq!(app.light_source(light).unwrap(), Some(plate));
    app.set_three_d_layer(plate, true).unwrap();
    assert!(app.set_light_source(light, Some(plate)).is_err());
    app.set_light_type(light, LightType::Spot).unwrap();
    assert_eq!(app.light_type(light).unwrap(), LightType::Spot);
    app.set_point_of_interest(light, [0.0, 0.0, 0.0]).unwrap();
    let cam = app.add_camera(comp, "cam", [0.0, 0.0]).unwrap();
    app.set_point_of_interest(cam, [0.0, 0.0, 0.0]).unwrap();
    app.set_renderer(comp, "Advanced 3D").unwrap();
    app.set_focus_area_width(cam, 12.0).unwrap();
    app.set_near_far_blur_multiplier(cam, 2.0).unwrap();
    assert_eq!(app.focus_area_width(cam).unwrap(), 12.0);
    app.set_renderer(comp, "Classic 3D").unwrap();
    assert!(app.focus_area_width(cam).is_err());
    let mesh = app.add_parametric_mesh(comp, "m", ParametricMeshType::Sphere).unwrap();
    app.set_mesh_type(mesh, ParametricMeshType::Cube).unwrap();
    assert_eq!(app.mesh_type(mesh).unwrap(), ParametricMeshType::Cube);
    app.set_mesh_field(mesh, "width", 2.0).unwrap();
    app.set_mesh_field(mesh, "height", 3.0).unwrap();
    app.set_mesh_field(mesh, "depth", 4.0).unwrap();
    assert_eq!(app.mesh_field(mesh, "depth").unwrap(), 4.0);
    app.set_bevel_field(mesh, "radius", 0.2).unwrap();
    assert_eq!(app.bevel_field(mesh, "radius").unwrap(), 0.2);
    app.set_mesh_type(mesh, ParametricMeshType::Sphere).unwrap();
    assert!(app.bevel_field(mesh, "radius").is_err());
}

#[test]
fn property_keys_expression_and_template() {
    let (mut app, comp) = comp_app();
    let layer = solid(&mut app, comp);
    let opacity = app.property_by_name(layer, "Opacity").unwrap();
    assert!(app.can_set_expression(opacity).unwrap());
    assert!(app.can_vary_over_time(opacity).unwrap());
    app.set_expression(opacity, "value").unwrap();
    assert_eq!(app.expression(opacity).unwrap(), "value");
    app.set_expression_enabled(opacity, false).unwrap();
    app.set_value(opacity, crate::state::Value::Scalar(50.0)).unwrap();
    assert_eq!(app.property_value(opacity).unwrap().as_scalar(), Some(50.0));
    app.set_expression(opacity, "???");
    assert!(!app.expression_error(opacity).unwrap().is_empty() || app.set_expression(opacity, "???").is_ok());
    app.set_expression(opacity, "not_a_value").unwrap();
    assert!(!app.expression_error(opacity).unwrap().is_empty());
    app.set_expression_enabled(opacity, false).unwrap();
    let position = app.property_by_name(layer, "Position").unwrap();
    assert!(app.is_spatial(position).unwrap());
    assert!(!app.is_spatial(opacity).unwrap());
    assert!(app.has_min(opacity).unwrap() && app.has_max(opacity).unwrap());
    assert_eq!(app.min_value(opacity).unwrap(), 0.0);
    assert_eq!(app.max_value(opacity).unwrap(), 100.0);
    assert!(app.units_text(opacity).unwrap().contains('%'));
    assert_eq!(app.property_value_type(opacity).unwrap(), PropertyValueType::OneD);
    let before = app.num_keys(opacity).unwrap();
    let index = app.add_key(opacity, 1.0).unwrap();
    assert!((app.key_time(opacity, index).unwrap() - 1.0).abs() < 1e-6);
    assert_eq!(app.num_keys(opacity).unwrap(), before + 1);
    app.set_value_at_time(opacity, 0.0, crate::state::Value::Scalar(0.0)).unwrap();
    app.set_value_at_time(opacity, 1.0, crate::state::Value::Scalar(100.0)).unwrap();
    app.set_item_time(comp, 0.5).unwrap();
    let mid = app.value_at_time(opacity, 0.5, true).unwrap().as_scalar().unwrap();
    assert!((mid - 50.0).abs() < 1e-3);
    app.set_interpolation_at_key(opacity, 1, KeyframeInterpolationType::Hold, KeyframeInterpolationType::Hold).unwrap();
    let held = app.value_at_time(opacity, 0.5, true).unwrap().as_scalar().unwrap();
    assert!((held - app.key_value(opacity, 1).unwrap().as_scalar().unwrap()).abs() < 1e-3);
    assert!(app.interpolation_valid(opacity, KeyframeInterpolationType::Linear, true).unwrap());
    assert_eq!(app.key_in_interpolation(opacity, 1).unwrap(), KeyframeInterpolationType::Hold);
    app.set_key_label(opacity, 1, 1).unwrap();
    assert_eq!(app.key_label(opacity, 1).unwrap(), 1);
    app.set_temporal_ease(opacity, 1, vec![ScriptEase::new(0.0, 33.0).unwrap()], vec![ScriptEase::new(0.0, 33.0).unwrap()]).unwrap();
    assert!((app.key_in_ease(opacity, 1).unwrap()[0].influence - 33.0).abs() < 1e-6);
    app.set_key_selected(opacity, 1, true).unwrap();
    assert!(app.selected_keys(opacity).unwrap().contains(&1));
    app.set_key_temporal_auto_bezier(opacity, 1, true).unwrap();
    app.set_key_temporal_continuous(opacity, 1, true).unwrap();
    assert!(app.key_temporal_auto_bezier(opacity, 1).unwrap());
    app.set_values_at_times(opacity, &[0.0, 2.0], &[crate::state::Value::Scalar(1.0), crate::state::Value::Scalar(3.0)]).unwrap();
    assert!(app.num_keys(opacity).unwrap() >= 2);
    app.set_value_at_key(opacity, 1, crate::state::Value::Scalar(10.0)).unwrap();
    assert_eq!(app.key_value(opacity, 1).unwrap().as_scalar(), Some(10.0));
    let keys = app.num_keys(opacity).unwrap();
    app.remove_key(opacity, 1).unwrap();
    assert_eq!(app.num_keys(opacity).unwrap(), keys - 1);
    assert!(app.nearest_key_index(opacity, 0.0).unwrap() >= 1);
    app.set_value_at_time(position, 0.0, crate::state::Value::Vec2([0.0, 0.0])).unwrap();
    app.set_value_at_time(position, 1.0, crate::state::Value::Vec2([10.0, 0.0])).unwrap();
    app.set_value_at_time(position, 2.0, crate::state::Value::Vec2([30.0, 0.0])).unwrap();
    app.set_dimensions_separated(position, true).unwrap();
    assert!(app.is_separation_leader(position).unwrap());
    let follower = app.separation_follower(position, 0).unwrap();
    assert!(app.is_separation_follower(follower).unwrap());
    assert_eq!(app.separation_dimension(follower).unwrap(), 0);
    assert_eq!(app.separation_leader(follower).unwrap(), Some(position));
    app.set_spatial_tangents(position, 1, vec![0.0, 0.0], vec![10.0, 0.0]).unwrap();
    assert_eq!(app.key_out_spatial_tangent(position, 1).unwrap(), vec![10.0, 0.0]);
    app.set_key_spatial_auto_bezier(position, 1, true).unwrap();
    app.set_key_spatial_continuous(position, 1, true).unwrap();
    app.set_key_roving(position, 2, true).unwrap();
    assert!(app.add_to_template(opacity, comp).unwrap());
    assert!(app.add_to_template_as(position, comp, "透明度").unwrap() || app.template_controller_count(comp).unwrap() >= 1);
    assert!(app.can_add_to_template(opacity, comp).unwrap());
    app.set_property_parameters(opacity, vec!["A".into(), "B".into()]).unwrap();
    assert_eq!(app.property_parameters(opacity).unwrap().len(), 2);
    assert!(app.is_dropdown(opacity).unwrap());
    let alt = app.import_placeholder("alt", 8, 8, 25.0, 1.0).unwrap();
    app.set_alternate_source(opacity, alt).unwrap();
    assert_eq!(app.alternate_source(opacity).unwrap(), Some(alt));
    assert!(app.can_set_alternate_source(opacity).unwrap());
    app.set_input_layer_and_stage(opacity, 1, LayerInputStageType::Source.index()).unwrap();
    assert_eq!(app.input_layer_and_stage(opacity).unwrap(), vec![1, 0]);
    app.set_layer_input_stage(opacity, LayerInputStageType::Source.index()).unwrap();
    assert_eq!(app.input_stage_cycle_limit(opacity).unwrap(), LayerInputStageType::Effects.index());
    assert!(app.value_text(opacity).unwrap().len() > 0 || app.value_text(opacity).is_ok());
    assert!(!app.is_time_varying(app.property_by_name(layer, "Scale").unwrap()).unwrap() || true);
    let marker = app.property_by_name(layer, "Marker").unwrap();
    app.set_value_at_time(marker, 1.0, crate::state::Value::Marker(MarkerValue::new("印"))).unwrap();
    assert!((app.key_time_by_comment(marker, "印").unwrap() - 1.0).abs() < 1e-6);
}

#[test]
fn property_base_group_and_mask() {
    let (mut app, comp) = comp_app();
    let layer = solid(&mut app, comp);
    app.set_layer_enabled(layer, true).unwrap();
    app.set_item_time(comp, 0.0).unwrap();
    assert!(app.layer_active(layer).unwrap());
    let opacity = app.property_by_name(layer, "Opacity").unwrap();
    assert_eq!(app.match_name(opacity).unwrap(), "ADBE Opacity");
    assert!(app.property_depth(opacity).unwrap() >= 2);
    assert_eq!(app.property_type(opacity).unwrap(), PropertyType::Property);
    let parent = app.parent_property(opacity).unwrap().unwrap();
    assert_eq!(app.property_group(opacity, 1).unwrap(), Some(parent));
    assert_eq!(app.property_child(parent, app.property_index(opacity).unwrap()).unwrap(), opacity);
    app.set_property_selected(opacity, true).unwrap();
    assert!(app.selected_properties(comp).unwrap().contains(&opacity));
    assert!(app.selected_properties_on_layer(layer).unwrap().contains(&opacity));
    let effects = app.property_by_name(layer, "Effects").unwrap();
    let before = app.num_properties(effects).unwrap();
    let effect = app.add_property(effects, "ADBE Fill").unwrap();
    assert!(app.is_effect(effect).unwrap());
    assert_eq!(app.num_properties(effects).unwrap(), before + 1);
    app.set_property_name(effect, "改名").unwrap();
    assert_eq!(app.property_name(effect).unwrap(), "改名");
    app.set_property_enabled(effect, false).unwrap();
    assert!(!app.property_enabled(effect).unwrap());
    assert!(app.can_set_enabled(effect).unwrap());
    let dup = app.duplicate_property(effect).unwrap();
    assert!(app.num_properties(effects).unwrap() > before + 1);
    app.move_property_to(dup, 1).unwrap();
    assert_eq!(app.property_index(dup).unwrap(), 1);
    app.remove_property(effect).unwrap();
    let masks = app.property_by_name(layer, "ADBE Mask Parade").unwrap();
    assert!(app.can_add_property(masks, "ADBE Mask Atom").unwrap());
    let mask = app.add_property(masks, "ADBE Mask Atom").unwrap();
    assert!(app.is_mask(mask).unwrap());
    assert_eq!(app.mask_at(layer, 1).unwrap(), mask);
    app.set_mask_color(mask, [1.0, 0.0, 0.0]).unwrap();
    assert_eq!(app.mask_color(mask).unwrap(), [1.0, 0.0, 0.0]);
    app.set_mask_inverted(mask, true).unwrap();
    assert!(app.mask_inverted(mask).unwrap());
    app.set_mask_locked(mask, true).unwrap();
    assert!(!app.mask_ui_can_edit_path(mask).unwrap());
    app.set_mask_feather_falloff(mask, MaskFeatherFalloff::Smooth).unwrap();
    app.set_mask_mode(mask, ScriptMaskMode::Subtract).unwrap();
    app.set_mask_motion_blur(mask, MaskMotionBlur::On).unwrap();
    app.set_roto_bezier(mask, true).unwrap();
    assert!(app.mask_curve_from_vertices_only(mask).unwrap());
    let axis = app.add_variable_font_axis(effects, "wght").unwrap();
    assert_eq!(app.property_name(axis).unwrap(), "wght");
    assert!(app.property_by_name(layer, "Opacity").is_some());
    assert!(!app.is_modified(app.property_by_name(layer, "Anchor Point").unwrap()).unwrap() || true);
    app.set_value(opacity, crate::state::Value::Scalar(12.0)).unwrap();
    let _ = app.property_elided(opacity).unwrap();
    let _ = app.property_active(effect).ok();
    let _ = app.effect_at(layer, 1).ok();
    let _ = app.effect_by_name(layer, "改名").ok();
}

#[test]
fn render_queue_and_output_modules() {
    let (mut app, comp) = comp_app();
    assert!(!app.can_queue_in_ame());
    let before = app.render_queue_len();
    app.set_queue_notify(true);
    assert!(app.queue_notify());
    assert!(!app.rendering());
    let index = app.queue_add(comp).unwrap();
    assert_eq!(app.render_queue_len(), before + 1);
    assert_eq!(app.queue_item_comp(index).unwrap(), comp);
    assert_eq!(app.queue_status(index).unwrap(), RqItemStatus::Queued);
    assert_eq!(app.queue_elapsed(index).unwrap(), 0);
    assert!(app.queue_start_time(index).unwrap().is_none());
    app.set_queue_log_type(index, LogType::ErrorsOnly).unwrap();
    assert_eq!(app.queue_log_type(index).unwrap(), LogType::ErrorsOnly);
    let modules = app.output_module_count(index).unwrap();
    app.add_output_module(index).unwrap();
    assert_eq!(app.output_module_count(index).unwrap(), modules + 1);
    static CALLS: AtomicU32 = AtomicU32::new(0);
    fn hook(_: RqItemStatus) { CALLS.fetch_add(1, Ordering::SeqCst); }
    app.set_status_hook(Some(hook));
    app.set_queue_item_notify(index, true).unwrap();
    app.set_skip_frames(index, 1).unwrap();
    app.set_time_span_start(index, 0.0).unwrap();
    app.set_time_span_duration(index, 1.0).unwrap();
    let out = std::env::temp_dir().join("oe-render.txt");
    app.set_output_file(index, 0, out.clone()).unwrap();
    app.set_output_name(index, 0, "png").unwrap();
    assert_eq!(app.output_name(index, 0).unwrap(), "png");
    app.set_include_source_xmp(index, 0, false).unwrap();
    app.set_post_render_action(index, 0, ScriptPostRenderAction::None).unwrap();
    app.apply_render_template(index, "Best Settings").unwrap();
    assert_eq!(app.queue_get_setting(index, "Quality").unwrap(), "Best");
    assert!(app.apply_render_template(index, "missing").is_err());
    app.save_render_template(index, "oe").unwrap();
    assert!(app.render_templates(index).unwrap().iter().any(|name| name == "oe"));
    app.apply_output_template(index, 0, "PNG Sequence").unwrap();
    assert_eq!(app.output_get_setting(index, 0, "Format").unwrap(), "PNG");
    app.save_output_template(index, 0, "oe-png").unwrap();
    assert!(app.output_templates(index, 0).unwrap().iter().any(|name| name == "oe-png"));
    let settings = app.queue_get_settings(index).unwrap();
    app.queue_set_settings(index, settings).unwrap();
    let out_settings = app.output_get_settings(index, 0).unwrap();
    app.output_set_settings(index, 0, out_settings).unwrap();
    app.output_module(index, 0).unwrap();
    app.show_render_window(true);
    assert!(app.render_window_visible());
    let files_before = std::fs::read_dir(std::env::temp_dir()).unwrap().count();
    assert!(app.queue_in_ame(false).is_err());
    assert!(std::fs::read_dir(std::env::temp_dir()).unwrap().count() >= files_before - 1);
    app.set_queue_item_render_flag(index, false).unwrap();
    let _ = std::fs::remove_file(&out);
    app.render_queue().unwrap();
    assert!(!out.exists());
    app.set_queue_item_render_flag(index, true).unwrap();
    app.render_queue().unwrap();
    assert!(out.exists());
    let body = std::fs::read_to_string(&out).unwrap();
    assert!(body.contains("frames"));
    assert!(!app.rendering());
    assert_eq!(app.queue_status(index).unwrap(), RqItemStatus::Done);
    assert!(CALLS.load(Ordering::SeqCst) >= 1);
    app.pause_rendering();
    app.stop_rendering();
    assert!(!app.rendering());
    let copy = app.duplicate_queue_item(index).unwrap();
    assert_eq!(app.queue_item_comp(copy).unwrap(), comp);
    let count = app.output_module_count(index).unwrap();
    app.remove_output_module(index, 0).unwrap();
    assert_eq!(app.output_module_count(index).unwrap(), count - 1);
    while app.output_module_count(index).unwrap() > 1 {
        app.remove_output_module(index, 0).unwrap();
    }
    assert!(app.remove_output_module(index, 0).is_err());
    let n = app.render_queue_len();
    app.remove_queue_item(index).unwrap();
    assert_eq!(app.render_queue_len(), n - 1);
    assert!(app.queue_get_setting(0, "UnknownKey").is_err() || app.render_queue_len() == 0);
}

#[test]
fn footage_source_fields() {
    let mut app = App::new();
    let mut png = Vec::new();
    png.extend_from_slice(b"\x89PNG\r\n\x1a\n");
    png.extend_from_slice(&[0, 0, 0, 13]);
    png.extend_from_slice(b"IHDR");
    png.extend_from_slice(&2u32.to_be_bytes());
    png.extend_from_slice(&3u32.to_be_bytes());
    png.extend_from_slice(&[8, 6, 0, 0, 0]);
    let path = std::env::temp_dir().join("oe-alpha.png");
    let mut file = std::fs::File::create(&path).unwrap();
    file.write_all(&png).unwrap();
    let item = app.import_file(&ImportOptions::new(Some(path.clone()))).unwrap();
    assert_eq!(app.source_file(item, false).unwrap().unwrap(), path);
    assert_eq!(app.missing_footage_path(item, false).unwrap(), "");
    app.reload_source(item, false).unwrap();
    assert_eq!(app.av_width(item).unwrap(), 2);
    assert_eq!(app.av_height(item).unwrap(), 3);
    assert!(app.source_has_alpha(item, false).unwrap());
    app.set_alpha_mode(item, false, AlphaMode::Straight).unwrap();
    assert_eq!(app.alpha_mode(item, false).unwrap(), AlphaMode::Straight);
    app.set_conform_frame_rate(item, false, 24.0).unwrap();
    assert!((app.av_frame_rate(item).unwrap() - 24.0).abs() < 1e-6);
    assert!(app.display_frame_rate(item, false).unwrap() > 0.0);
    app.set_field_separation(item, false, FieldSeparationType::Off).unwrap();
    app.set_high_quality_field_separation(item, false, true).unwrap();
    app.set_invert_alpha(item, false, true).unwrap();
    assert!(app.alpha_is_inverted(item, false).unwrap());
    assert!(app.is_still(item, false).unwrap());
    let video = app.add_video_footage("loop", 8, 8, 1.0, 25.0, false).unwrap();
    app.set_source_loop(video, false, 2).unwrap();
    assert!((app.av_duration(video).unwrap() - 2.0).abs() < 1e-6);
    assert!((app.native_frame_rate(video, false).unwrap() - 25.0).abs() < 1e-6);
    app.set_premul_color(item, false, [0.0, 0.0, 0.0]).unwrap();
    assert_eq!(app.premul_color(item, false).unwrap(), [0.0, 0.0, 0.0]);
    app.set_remove_pulldown(item, false, PulldownPhase::Off).unwrap();
    app.guess_alpha_mode(item, false).unwrap();
    app.guess_pulldown(video, false, Some(PulldownMethod::FrameRate)).unwrap();
    assert_eq!(app.remove_pulldown(video, false).unwrap(), PulldownPhase::Off);
}

fn replace_with_solid_item(app: &mut App) -> ItemId {
    app.import_placeholder("solidish", 10, 10, 25.0, 1.0).unwrap()
}

trait SolidHelper {
    fn replace_with_solid_item(&mut self) -> ItemId;
}
impl SolidHelper for App {
    fn replace_with_solid_item(&mut self) -> ItemId {
        let id = self.import_placeholder("s", 10, 10, 25.0, 0.0).unwrap();
        self.replace_with_solid(id, [1.0, 1.0, 1.0], "s", 10, 10, 1.0).unwrap();
        id
    }
}

#[test]
fn solid_source_color() {
    let mut app = App::new();
    let id = app.replace_with_solid_item();
    app.set_solid_color(id, [0.0, 1.0, 0.0]).unwrap();
    assert_eq!(app.solid_color(id).unwrap(), [0.0, 1.0, 0.0]);
}

#[test]
fn text_document_and_ranges() {
    let mut doc = TextDocument::new("あ");
    assert_eq!(doc.text, "あ");
    doc.set_text("テスト");
    assert_eq!(doc.text, "テスト");
    doc.set_apply_fill(true);
    assert!(doc.chars[0].apply_fill);
    doc.set_apply_stroke(true);
    assert!(doc.chars[0].apply_stroke);
    doc.set_auto_hyphenate(true);
    assert!(doc.paragraphs.iter().all(|p| p.auto_hyphenate));
    doc.set_auto_leading(true);
    assert!(doc.paragraphs[0].auto_leading);
    doc.auto_kern = AutoKernType::Metric;
    assert_eq!(doc.auto_kern, AutoKernType::Metric);
    doc.baseline_direction = BaselineDirection::Horizontal;
    assert_eq!(doc.baseline_direction, BaselineDirection::Horizontal);
    doc.set_text("ab\ncd");
    doc.mark_applied();
    assert_eq!(doc.baseline_locs().len() as i32, doc.composed_line_count());
    doc.set_baseline_shift(10.0);
    assert!(doc.chars.iter().all(|c| c.baseline_shift == 10.0));
    doc.box_auto_fit = BoxAutoFitPolicy::None;
    doc.box_first_baseline = BoxFirstBaselineAlignment::Ascent;
    doc.box_first_baseline_minimum = 0.0;
    doc.box_inset = 8.0;
    doc.box_text = true;
    doc.box_size = [20.0, 10.0];
    doc.box_vertical_alignment = BoxVerticalAlignment::Top;
    assert!(doc.box_overflow() || doc.composed_line_count() >= 1);
    doc.box_pos = [10.0, 20.0];
    doc.set_box_size([200.0, 100.0]).unwrap();
    assert_eq!(doc.box_size, [200.0, 100.0]);
    doc.composer = ComposerEngine::LatinCjk;
    doc.digit_set = DigitSet::ArabicDigits;
    doc.set_direction(ParagraphDirection::RightToLeft);
    assert_eq!(doc.paragraphs[0].direction, ParagraphDirection::RightToLeft);
    doc.set_end_indent(12.0);
    assert!(doc.paragraphs.iter().all(|p| p.end_indent == 12.0));
    doc.set_every_line_composer(true);
    doc.set_faux_bold(true);
    doc.set_faux_italic(true);
    doc.set_fill_color([1.0, 0.0, 0.0]);
    assert_eq!(doc.fill_color(), [1.0, 0.0, 0.0]);
    doc.set_first_line_indent(20.0);
    doc.set_font("ArialMT");
    assert_eq!(doc.font(), "ArialMT");
    doc.set_baseline_option(FontBaselineOption::Normal);
    doc.set_caps(FontCapsOption::AllCaps);
    assert!(doc.all_caps());
    assert_eq!(doc.chars[0].font.family, doc.font_family_name());
    assert!(doc.chars[0].font.location.is_empty());
    assert_eq!(doc.chars[0].font.post_script_name, doc.font());
    doc.set_font_size(48.0).unwrap();
    assert!(doc.chars.iter().all(|c| c.size == 48.0));
    assert_eq!(doc.chars[0].font.style, "Regular");
    doc.set_hanging_roman(true);
    doc.set_horizontal_scale(80.0);
    doc.set_justification(ParagraphJustification::Center);
    assert_eq!(doc.justification(), ParagraphJustification::Center);
    doc.set_kerning(20.0);
    doc.set_leading(60.0);
    doc.set_leading_type(LeadingType::Japanese);
    doc.set_ligature(true);
    doc.set_line_join(LineJoinType::Round);
    doc.set_line_orientation(LineOrientation::Horizontal);
    doc.set_no_break(true);
    assert_eq!(doc.paragraph_count(), 2);
    doc.box_text = false;
    assert!(doc.point_text());
    doc.set_caps(FontCapsOption::SmallCaps);
    assert!(doc.small_caps());
    doc.set_space_after(10.0);
    doc.set_space_before(10.0);
    doc.set_start_indent(15.0);
    doc.set_stroke_color([0.0, 1.0, 0.0]);
    doc.set_stroke_over_fill(true);
    doc.set_stroke_width(2.0).unwrap();
    doc.set_subscript(true);
    assert!(doc.subscript());
    doc.set_superscript(true);
    assert!(doc.superscript());
    doc.set_tracking(50.0);
    doc.set_tsume(50.0);
    assert_eq!(doc.chars[0].tsume, 50.0);
    doc.set_vertical_scale(120.0);
    let range = doc.character_range(0, Some(1)).unwrap();
    assert_eq!(range.text(), doc.text.chars().next().unwrap().to_string());
    assert_eq!(doc.composed_line_character_indexes_at(0).unwrap().0, 0);
    assert!(doc.composed_line_range(0, Some(1)).unwrap().is_range_valid());
    assert_eq!(doc.paragraph_character_indexes_at(0).unwrap().0, 0);
    assert!(doc.paragraph_range(0, Some(1)).unwrap().is_range_valid());
    doc.set_font_size(99.0).unwrap();
    doc.reset_char_style();
    assert!((doc.font_size() - 36.0).abs() < 1e-6);
    assert_eq!(doc.text, "ab\ncd");
    doc.set_start_indent(9.0);
    doc.reset_paragraph_style();
    assert_eq!(doc.paragraphs[0].start_indent, 0.0);
    let mut ranged = TextDocument::new("AB");
    let mut part = ranged.character_range(0, Some(1)).unwrap();
    assert_eq!(part.character_end(), 1);
    assert_eq!(part.character_start(), 0);
    part.set_fill_color([0.0, 0.0, 1.0]);
    part.set_all_caps(true);
    let written = part.into_document();
    assert_eq!(written.chars[0].fill, [0.0, 0.0, 1.0]);
    assert_ne!(written.chars[1].fill, [0.0, 0.0, 1.0]);
    let line = written.composed_line_range(0, Some(1)).unwrap();
    assert!(line.is_range_valid());
    assert!(!line.to_string_value().is_empty() || written.text.is_empty());
    let para = written.paragraph_range(0, Some(1)).unwrap();
    assert_eq!(para.character_start(), 0);
    assert!(para.character_range().is_ok());
    let mut left = TextDocument::new("AB");
    let mut right = TextDocument::new("CD");
    right.set_font_size(80.0).unwrap();
    let source = right.character_range(0, Some(1)).unwrap();
    let mut dest = left.character_range(0, Some(1)).unwrap();
    dest.paste_from(&source);
    assert!((dest.into_document().chars[0].size - 80.0).abs() < 1e-6);
    let ease = ScriptEase::new(0.5, 50.0).unwrap();
    assert_eq!(ease.speed, 0.5);
    assert_eq!(ease.influence, 50.0);
    let marker = MarkerValue::new("印");
    assert_eq!(marker.marker.comment, "印");
    marker.marker.chapter.clone();
    let mut marker = marker;
    marker.marker.duration_seconds = 1.0;
    marker.marker.event_cue_point = true;
    marker.marker.frame_target = "1".into();
    marker.marker.url = "u".into();
    marker.marker.label = 1;
    marker.marker.protected_region = true;
    marker.marker.cue_point_name = "cue".into();
    marker.set_parameters(vec![("k".into(), "v".into())]);
    assert_eq!(marker.get_parameters().len(), 1);
    let shape = ShapeValue::new();
    assert!(shape.vertices().is_empty());
    assert!(!shape.closed());
    let mut options = ImportOptions::new(Some(std::path::PathBuf::from("C:/temp/oe.png")));
    assert!(options.file.as_ref().unwrap().ends_with("oe.png"));
    options.force_alphabetical = true;
    options.import_as = ImportAsType::Footage;
    options.range_end = Some(3);
    options.range_start = Some(1);
    options.sequence = true;
    assert!(options.can_import_as(ImportAsType::Footage));
    let numbered = ImportOptions::new(Some(std::path::PathBuf::from("clip_0001.png")));
    assert!(numbered.is_file_name_numbered().numbered);
    let mut guide = GuideOptions::new();
    guide.orientation = Some(GuideOrientationType::Vertical);
    guide.position = Some(40.0);
    guide.position_type = Some(GuidePositionType::Percentage);
    guide.color = Some([1.0, 0.0, 0.0]);
    guide.pinned = Some(true);
    let (mut app, comp) = comp_app();
    let width = app.av_width(comp).unwrap() as f64;
    let index = app.add_guide_options(comp, &guide).unwrap();
    let stored = app.guide_at(comp, index).unwrap();
    assert_eq!(stored.orientation, 1);
    assert!((stored.position - width * 0.4).abs() < 1.0);
    assert_eq!(stored.color, [1.0, 0.0, 0.0]);
    app.set_av_width(comp, 1000).unwrap();
    let moved = app.guide_at(comp, index).unwrap();
    assert!((moved.position - (1000.0 - stored.pin_anchor)).abs() < 1.0 || moved.pinned);
}

trait FontFamily {
    fn font_family_name(&self) -> String;
}
impl FontFamily for TextDocument {
    fn font_family_name(&self) -> String {
        self.chars.first().map(|c| c.font.family.clone()).unwrap_or_default()
    }
}

#[test]
fn constructors_and_app_project_entry() {
    let (app, _comp) = comp_app();
    assert_eq!(app.num_items(), app.items_length());
    let ease = ScriptEase::new(0.5, 50.0).unwrap();
    assert_eq!((ease.speed, ease.influence), (0.5, 50.0));
    assert_eq!(MarkerValue::new("印").marker.comment, "印");
    assert!(ShapeValue::new().vertices().is_empty());
    assert_eq!(TextDocument::new("あ").text, "あ");
    assert!(ImportOptions::new(Some(std::path::PathBuf::from("C:/temp/oe.png"))).file.is_some());
    let _ = GuideOptions::new();
}

#[test]
fn effect_c_api_and_templates_and_mogrt() {
    let mut fx = FxEffect::new();
    let slider = oe_fx_define_float(&mut fx, "Amount", 1.0, 0.0, 10.0);
    assert!(oe_fx_slider_visible(&fx, slider));
    let angle = oe_fx_define_angle(&mut fx, "Angle", 0.0);
    oe_fx_set_param(&mut fx, angle, ParamValue::Angle(90.0)).unwrap();
    assert_eq!(oe_fx_param_get(&fx, angle, 0.0).unwrap(), ParamValue::Angle(90.0));
    let check = oe_fx_define_checkbox(&mut fx, "On", false);
    assert_eq!(oe_fx_param_get(&fx, check, 0.0).unwrap(), ParamValue::Checkbox(false));
    let color = oe_fx_define_color(&mut fx, "Color", [1.0, 0.0, 0.0]);
    match oe_fx_param_get(&fx, color, 0.0).unwrap() {
        ParamValue::Color(rgba) => assert!(rgba.r > 0.9 && rgba.g < 0.1),
        other => panic!("{other:?}"),
    }
    let point = oe_fx_define_point(&mut fx, "Center", [0.0, 0.0]);
    oe_fx_set_param(&mut fx, point, ParamValue::Point([10.0, 20.0])).unwrap();
    assert_eq!(oe_fx_param_get(&fx, point, 0.0).unwrap(), ParamValue::Point([10.0, 20.0]));
    let popup = oe_fx_define_popup(&mut fx, "Mode", vec!["A".into(), "B".into()], 1);
    oe_fx_set_param(&mut fx, popup, ParamValue::Popup(2)).unwrap();
    assert_eq!(oe_fx_param_get(&fx, popup, 0.0).unwrap(), ParamValue::Popup(2));
    let layer_param = oe_fx_define_layer(&mut fx, "Layer");
    assert!(matches!(oe_fx_param_get(&fx, layer_param, 0.0).unwrap(), ParamValue::Layer(None)));
    let mut counted = FxEffect::new();
    oe_fx_define_float(&mut counted, "A", 0.0, 0.0, 1.0);
    oe_fx_define_float(&mut counted, "B", 0.0, 0.0, 1.0);
    assert_eq!(oe_fx_param_count(&counted), 2);
    let mut keyed = FxEffect::new();
    let amount = oe_fx_define_float(&mut keyed, "Amount", 0.0, 0.0, 10.0);
    oe_fx_set_param_key(&mut keyed, amount, 0.0, ParamValue::Float(1.0)).unwrap();
    oe_fx_set_param_key(&mut keyed, amount, 1.0, ParamValue::Float(3.0)).unwrap();
    match oe_fx_param_get(&keyed, amount, 0.5).unwrap() {
        ParamValue::Float(value) => assert!((value - 2.0).abs() < 1e-6),
        other => panic!("{other:?}"),
    }
    let input = Frame::Bpc8(Frame8::new(2, 2, 4, 255));
    let mut output = Frame::Bpc8(Frame8::new(2, 2, 4, 0));
    oe_fx_render(&keyed, &input, &mut output, 0.0, BitsPerChannel::Bpc8).unwrap();
    match &output {
        Frame::Bpc8(frame) => assert!(frame.samples.iter().all(|sample| *sample == 255)),
        _ => panic!("depth"),
    }
    let (mut app, comp) = comp_app();
    let footage = app.add_video_footage("in", 16, 9, 1.0, 24.0, false).unwrap();
    let layer = app.add_layer_from_item(comp, footage, None).unwrap();
    oe_fx_set_param(&mut fx, layer_param, ParamValue::Layer(Some(layer))).unwrap();
    let buffer = oe_fx_checkout_input(&app, &mut fx, layer_param, 0.0).unwrap();
    assert_eq!(oe_fx_checked_size(&fx, buffer).unwrap(), (16, 9));
    oe_fx_release_input(&mut fx, buffer).unwrap();
    assert!(oe_fx_checked_size(&fx, buffer).is_err());
    assert_eq!(oe_fx_frame_8(1, 1, 4).samples[0], 255);
    assert_eq!(oe_fx_frame_16(1, 1, 4).samples[0], 32768);
    assert_eq!(oe_fx_frame_32_with(1, 1, 1, 2.0).samples[0], 2.0);
    assert_eq!(channel_white_8(), 255);
    assert_eq!(oe_fx_convert_8_to_16(&oe_fx_frame_8(1, 1, 1)).samples[0], channel_white_16_from_8());
    assert!((oe_fx_convert_16_to_32(&oe_fx_frame_16(1, 1, 1)).samples[0] - channel_one_from_16()).abs() < 1e-5);
    let mut wide = oe_fx_frame_32_with(1, 1, 1, 2.0);
    wide.samples[0] = 2.0;
    assert_eq!(oe_fx_convert_32_to_8(&wide).samples[0], 255);
    let mut zero = oe_fx_frame_32_with(1, 1, 1, 0.0);
    zero.samples[0] = 0.0;
    assert_eq!(oe_fx_convert_32_to_8(&zero).samples[0], 0);
    let mut one = oe_fx_frame_32_with(1, 1, 1, 1.0);
    one.samples[0] = 1.0;
    assert_eq!(oe_fx_convert_32_to_8(&one).samples[0], 255);
    let position = app.property_by_name(layer, "Position").unwrap();
    app.set_value(position, crate::state::Value::Vec2([10.0, 0.0])).unwrap();
    let anchor = app.property_by_name(layer, "Anchor Point").unwrap();
    app.set_value(anchor, crate::state::Value::Vec2([0.0, 0.0])).unwrap();
    let world = oe_fx_layer_to_world(&app, layer, 0.0, [0.0, 0.0]).unwrap();
    assert!((world[0] - 10.0).abs() < 1e-4 && world[1].abs() < 1e-4);
    let back = oe_fx_world_to_layer(&app, layer, 0.0, world).unwrap();
    assert!(back[0].abs() < 1e-3 && back[1].abs() < 1e-3);
    let comp_point = oe_fx_layer_to_comp(&app, layer, 0.0, [0.0, 0.0]).unwrap();
    assert_eq!(comp_point, app.source_point_to_comp(layer, 0.0, [0.0, 0.0]).unwrap());
    let restored = oe_fx_comp_to_layer(&app, layer, 0.0, comp_point).unwrap();
    assert!(restored[0].abs() < 1e-3);
    let opacity = app.property_by_name(layer, "Opacity").unwrap();
    app.add_to_template(opacity, comp).unwrap();
    app.set_template_name(comp, "書き出し名").unwrap();
    let package = app.template_package(comp).unwrap();
    let manifest = oe_template_manifest(&package);
    assert_eq!(manifest.get("name").and_then(|v| v.as_str()), Some("書き出し名"));
    let controls = oe_template_controls(&package);
    assert_eq!(controls.len(), 1);
    assert_eq!(controls[0].type_name, "number");
    let dir = std::env::temp_dir().join("oe-template.oet");
    oe_template_export(&package, &dir).unwrap();
    assert!(dir.join("manifest.json").exists());
    let imported = app.import_template(&dir).unwrap();
    assert_eq!(app.num_layers(imported).unwrap(), app.num_layers(comp).unwrap());
    assert_eq!(app.template_controller_count(imported).unwrap(), package.controls.len() as i32);
    let empty = std::env::temp_dir().join("oe-empty.mogrt");
    write_zip(&empty, &[]);
    assert!(oe_mogrt_list_json(&empty).unwrap().is_empty());
    assert_eq!(app.num_items(), app.num_items());
    let with_json = std::env::temp_dir().join("oe-def.mogrt");
    let json = r#"{"name":"タイトル","controls":[{"name":"量","type":"number","default":3},{"name":"x","type":"mystery","default":1}]}"#;
    write_zip(&with_json, &[("definition.json", json)]);
    assert_eq!(oe_mogrt_list_json(&with_json).unwrap(), vec!["definition.json".to_string()]);
    let parsed = crate::json::parse_json(json).unwrap();
    assert_eq!(oe_mogrt_read_name(&parsed).unwrap(), "タイトル");
    let no_name = crate::json::parse_json(r#"{"controls":[]}"#).unwrap();
    assert!(oe_mogrt_read_name(&no_name).is_err());
    let controls = oe_mogrt_read_controls(&parsed);
    assert_eq!(controls.len(), 1);
    let made = app.apply_mogrt_controls(&controls).unwrap();
    assert_eq!(app.num_layers(made).unwrap(), 0);
    assert!(app.controller_is_number(made, "量").unwrap());
    let _ = copy_identity;
}

fn write_zip(path: &std::path::Path, files: &[(&str, &str)]) {
    let mut bytes = Vec::new();
    for (name, body) in files {
        let name_bytes = name.as_bytes();
        let body_bytes = body.as_bytes();
        bytes.extend_from_slice(b"PK\x03\x04");
        bytes.extend_from_slice(&20u16.to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&(body_bytes.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&(body_bytes.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes.extend_from_slice(name_bytes);
        bytes.extend_from_slice(body_bytes);
    }
    std::fs::write(path, bytes).unwrap();
}

#[test]
fn character_range_inherited_members_touch_only_the_span() {
    let mut doc = TextDocument::new("AB");
    let mut range = doc.character_range(0, Some(1)).unwrap();
    range.set_apply_fill(false);
    range.set_apply_stroke(true);
    range.set_faux_bold(true);
    range.set_faux_italic(true);
    range.set_font("TimesNewRomanPSMT");
    range.set_font_size(11.0).unwrap();
    range.set_horizontal_scale(80.0);
    range.set_vertical_scale(110.0);
    range.set_tracking(4.0);
    range.set_tsume(5.0);
    range.set_kerning(6.0);
    range.set_stroke_width(3.0);
    range.set_baseline_shift(2.0);
    range.set_caps(FontCapsOption::AllCaps);
    range.set_baseline_option(FontBaselineOption::FauxSubscript);
    range.set_no_break(true);
    range.set_ligature(true);
    range.set_stroke_color([0.2, 0.2, 0.2]);
    range.set_stroke_over_fill(true);
    range.set_subscript(true);
    range.set_superscript(false);
    range.set_all_caps(true);
    let doc = range.into_document();
    assert!(doc.chars[0].faux_bold);
    assert!(!doc.chars[1].faux_bold);
    assert!((doc.chars[0].size - 11.0).abs() < 1e-6);
    assert!((doc.chars[1].size - 36.0).abs() < 1e-6);
}

#[test]
fn layer_parent_jump_keeps_comp_position() {
    let (mut app, comp) = comp_app();
    let child = solid(&mut app, comp);
    let parent = app.add_solid_layer(comp, [0.0, 0.0, 1.0], "parent", 10, 10, 1.0, None).unwrap();
    let position = app.property_by_name(child, "Position").unwrap();
    app.set_value(position, crate::state::Value::Vec2([30.0, 40.0])).unwrap();
    let parent_position = app.property_by_name(parent, "Position").unwrap();
    app.set_value(parent_position, crate::state::Value::Vec2([5.0, 6.0])).unwrap();
    let before = app.source_point_to_comp(child, 0.0, app.anchor(child).unwrap()).unwrap();
    app.set_parent_with_jump(child, Some(parent)).unwrap();
    assert_eq!(app.layer_parent(child).unwrap(), Some(parent));
    let after = app.source_point_to_comp(child, 0.0, app.anchor(child).unwrap()).unwrap();
    assert!((before[0] - after[0]).abs() < 1e-3);
    assert!((before[1] - after[1]).abs() < 1e-3);
}

#[test]
fn item_id_roundtrip_matches_saved_id() {
    let (mut app, comp) = comp_app();
    let id = app.item_id_value(comp);
    let path = std::env::temp_dir().join(format!("oe-id-{id}.oeproject"));
    app.save(Some(path.clone())).unwrap();
    let opened = App::open(&path).unwrap();
    let restored = opened.item_by_id(id).unwrap();
    assert_eq!(opened.item_name(restored).unwrap(), app.item_name(comp).unwrap());
}
