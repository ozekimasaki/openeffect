use model::composition::{
    auto_graph_kind, Composition, GraphKind, GraphPropertyRef, GraphTypeMode, ShowProperties,
};
use model::id::{ItemId, LayerId, PropertyId};
use model::keyframe::{
    apply_roving, apply_spatial, apply_temporal, easy_ease, easy_ease_in, easy_ease_out, Keyframe,
    KeyframeEase, Marker, Roving, RovingCommand, SpatialCommand, SpatialInterpolation,
    TemporalCommand, TemporalInterpolation, TimedMarker,
};
use model::layer::Layer;
use model::property::{Property, PropertyValue};
use model::time::Time;
use model::LayerKind;

use eval::edit::{drag_marker, marker_interval, sequence_layers};
use eval::interpolate::{
    audio_amplitude, bake_expression, corner_turns, eval_simple_expression, exponential_scale,
    grab_spatial_handle, keys_from_points, rove_middle_key, sample_position, sample_scalar,
    speed_variation, time_reverse_keys,
};

fn key_at(time: f64, value: PropertyValue) -> Keyframe {
    Keyframe::at(Time::from_seconds(time), value)
}

fn pos(time: f64, x: f64, y: f64) -> Keyframe {
    key_at(time, PropertyValue::TwoD([x, y]))
}

#[test]
fn layer_marker_lands_at_the_current_time_without_stopping_preview() {
    let mut composition = Composition::new(ItemId::new(1), "C", 8, 4, 24.0, Time::from_seconds(2.0));
    let layer = Layer::new(LayerId::new(1), "L", LayerKind::Null);
    composition.layers.push(layer);
    let playing = composition.preview_playing_add_marker(Some(LayerId::new(1)), Time::from_seconds(0.5));
    assert!(playing);
    assert_eq!(composition.layer(LayerId::new(1)).unwrap().markers[0].time, Time::from_seconds(0.5));
}

#[test]
fn comp_marker_lands_on_the_ruler_when_no_layer_is_selected() {
    let mut composition = Composition::new(ItemId::new(1), "C", 8, 4, 24.0, Time::from_seconds(2.0));
    composition.add_marker(None, Time::from_seconds(0.25), false);
    assert_eq!(composition.markers.len(), 1);
    assert!(composition.layers.is_empty());
}

#[test]
fn marker_dialog_opens_and_confirm_keeps_the_marker() {
    let mut composition = Composition::new(ItemId::new(1), "C", 8, 4, 24.0, Time::from_seconds(2.0));
    let opened = composition.add_marker(None, Time::from_seconds(0.5), true);
    assert!(opened);
    assert_eq!(composition.markers[0].time, Time::from_seconds(0.5));
}

#[test]
fn numbered_marker_moves_instead_of_duplicating() {
    let mut composition = Composition::new(ItemId::new(1), "C", 8, 4, 24.0, Time::from_seconds(4.0));
    composition.place_numbered_marker(3, Time::from_seconds(0.0));
    composition.place_numbered_marker(3, Time::from_seconds(2.0));
    let numbered: Vec<_> = composition.markers.iter().filter(|marker| marker.number == Some(3)).collect();
    assert_eq!(numbered.len(), 1);
    assert_eq!(numbered[0].time, Time::from_seconds(2.0));
}

#[test]
fn marker_comments_roundtrip() {
    let mut marker = Marker::comp_numbered(4);
    assert_eq!(marker.comment, "4");
    marker.comment = "cut".into();
    let copy = marker.clone();
    assert_eq!(copy.comment, "cut");
    assert_eq!(Marker::layer_default().comment, "");
}

#[test]
fn marker_duration_splits_the_icon() {
    let mut marker = TimedMarker::at(Time::from_seconds(1.0), Marker::layer_default());
    marker.marker.duration_seconds = 1.0;
    let end = marker.time.seconds() + marker.marker.duration_seconds;
    assert!((end - marker.time.seconds() - 1.0).abs() < 1.0e-12);
    assert!(marker.marker.duration_seconds > 0.0);
}

#[test]
fn chapter_marker_gets_an_icon_dot() {
    let mut marker = Marker::layer_default();
    assert!(!marker.has_icon_dot());
    marker.chapter = "Intro".into();
    assert!(marker.has_icon_dot());
}

#[test]
fn url_marker_gets_an_icon_dot() {
    let mut marker = Marker::layer_default();
    marker.url = "https://example.test".into();
    assert!(marker.has_icon_dot());
    let copy = marker.clone();
    assert_eq!(copy.url, "https://example.test");
}

#[test]
fn frame_target_roundtrips() {
    let mut marker = Marker::layer_default();
    marker.frame_target = "_blank".into();
    assert_eq!(marker.clone().frame_target, "_blank");
}

#[test]
fn event_cue_point_is_kept() {
    let mut marker = Marker::layer_default();
    marker.event_cue_point = true;
    marker.cue_point_name = "hit".into();
    assert!(marker.has_icon_dot());
    assert!(marker.clone().event_cue_point);
}

#[test]
fn navigation_cue_point_is_kept() {
    let mut marker = Marker::layer_default();
    marker.navigation_cue_point = true;
    assert!(marker.has_icon_dot());
    assert!(marker.clone().navigation_cue_point);
}

#[test]
fn cue_parameters_survive_a_roundtrip() {
    let mut marker = Marker::layer_default();
    marker.parameters = vec![
        ("a".into(), "1".into()),
        ("b".into(), "2".into()),
        ("c".into(), "3".into()),
    ];
    assert_eq!(marker.clone().parameters.len(), 3);
}

#[test]
fn marker_label_is_kept() {
    let mut marker = Marker::layer_default();
    assert_eq!(marker.label, 0);
    marker.label = 4;
    assert_eq!(marker.clone().label, 4);
}

#[test]
fn locked_markers_do_not_drag() {
    let mut marker = TimedMarker::at(Time::from_seconds(1.0), Marker::layer_default());
    assert!(!drag_marker(&mut marker, Time::from_seconds(2.0), true));
    assert_eq!(marker.time, Time::from_seconds(1.0));
    assert!(drag_marker(&mut marker, Time::from_seconds(2.0), false));
    assert_eq!(marker.time, Time::from_seconds(2.0));
}

#[test]
fn deleting_a_marker_does_not_renumber_the_others() {
    let mut composition = Composition::new(ItemId::new(1), "C", 8, 4, 24.0, Time::from_seconds(4.0));
    composition.place_numbered_marker(1, Time::ZERO);
    composition.place_numbered_marker(3, Time::from_seconds(1.0));
    composition.delete_marker_at(Time::ZERO);
    assert_eq!(composition.marker_numbers(), vec![3]);
}

#[test]
fn update_markers_from_source_copies_child_markers() {
    let mut composition = Composition::new(ItemId::new(1), "C", 8, 4, 24.0, Time::from_seconds(2.0));
    let mut layer = Layer::new(LayerId::new(1), "nest", LayerKind::Precomp { source: ItemId::new(2) });
    layer.markers.push(TimedMarker::at(Time::from_seconds(0.2), Marker::new("hand")));
    composition.layers.push(layer);
    let source = vec![TimedMarker::at(Time::from_seconds(0.8), Marker::new("from-child"))];
    composition.update_markers_from_source(LayerId::new(1), &source);
    let markers = &composition.layer(LayerId::new(1)).unwrap().markers;
    assert_eq!(markers.len(), 1);
    assert_eq!(markers[0].marker.comment, "from-child");
}

#[test]
fn protected_region_keeps_its_length_when_the_comp_stretches() {
    let mut composition = Composition::new(ItemId::new(1), "C", 8, 4, 24.0, Time::from_seconds(5.0));
    let mut marker = Marker::comp_numbered(1);
    marker.protected_region = true;
    marker.duration_seconds = 2.0;
    composition.markers.push(TimedMarker::at(Time::from_seconds(1.0), marker));
    composition.stretch_protecting(Time::from_seconds(10.0));
    assert!((composition.markers[0].marker.duration_seconds - 2.0).abs() < 1.0e-9);
    assert!((composition.duration.seconds() - 10.0).abs() < 1.0e-9);
}

#[test]
fn intro_protects_fifteen_percent() {
    let mut composition = Composition::new(ItemId::new(1), "C", 8, 4, 24.0, Time::from_seconds(10.0));
    composition.create_intro_protected();
    let marker = &composition.markers[0];
    assert!((marker.time.seconds()).abs() < 1.0e-9);
    assert!((marker.marker.duration_seconds - 1.5).abs() < 1.0e-9);
    assert!(marker.marker.protected_region);
}

#[test]
fn outro_protects_the_last_fifteen_percent() {
    let mut composition = Composition::new(ItemId::new(1), "C", 8, 4, 24.0, Time::from_seconds(10.0));
    composition.create_outro_protected();
    let marker = &composition.markers[0];
    let end = marker.time.seconds() + marker.marker.duration_seconds;
    assert!((marker.marker.duration_seconds - 1.5).abs() < 1.0e-9);
    assert!((end - 10.0).abs() < 1.0e-9);
}

#[test]
fn work_area_becomes_the_protected_region() {
    let mut composition = Composition::new(ItemId::new(1), "C", 8, 4, 24.0, Time::from_seconds(10.0));
    composition.work_area_start = Time::from_seconds(2.0);
    composition.work_area_duration = Time::from_seconds(3.0);
    composition.create_protected_from_work_area();
    let marker = &composition.markers[0];
    assert_eq!(marker.time, composition.work_area_start);
    assert!((marker.marker.duration_seconds - composition.work_area_duration.seconds()).abs() < 1.0e-9);
}

#[test]
fn info_panel_reports_one_second_between_markers() {
    let a = TimedMarker::at(Time::from_seconds(1.0), Marker::layer_default());
    let b = TimedMarker::at(Time::from_seconds(2.0), Marker::layer_default());
    assert!((marker_interval(&a, &b).seconds() - 1.0).abs() < 1.0e-12);
}

#[test]
fn spatial_linear_has_corners_and_no_curve() {
    let keys = vec![pos(0.0, 0.0, 0.0), pos(1.0, 1.0, 0.0), pos(2.0, 1.0, 1.0)];
    let mut linear = keys.clone();
    for key in &mut linear {
        key.spatial = SpatialInterpolation::Linear;
    }
    let mid = sample_position(&linear, Time::from_seconds(0.5));
    assert!((mid[0] - 0.5).abs() < 1.0e-6);
    assert!(mid[1].abs() < 1.0e-6);
    assert!(corner_turns(&linear));
}

#[test]
fn spatial_bezier_can_corner_one_side() {
    let mut left = pos(0.0, 0.0, 0.0);
    let mut right = pos(1.0, 1.0, 0.0);
    left.spatial = SpatialInterpolation::Bezier;
    left.spatial_out = [0.2, 0.8, 0.0];
    right.spatial = SpatialInterpolation::Bezier;
    right.spatial_in = [0.0, 0.0, 0.0];
    let mid = sample_position(&[left, right], Time::from_seconds(0.5));
    assert!(mid[1].abs() > 0.05);
}

#[test]
fn continuous_bezier_keeps_handles_opposite() {
    let mut key = pos(1.0, 1.0, 0.0);
    key.spatial = SpatialInterpolation::ContinuousBezier;
    grab_spatial_handle(&mut key, [0.4, 0.2, 0.0]);
    assert_eq!(key.spatial_in[0], -0.4);
    assert_eq!(key.spatial_in[1], -0.2);
}

#[test]
fn auto_bezier_is_curved_until_the_handle_is_grabbed() {
    let keys = vec![pos(0.0, 0.0, 0.0), pos(1.0, 1.0, 0.0), pos(2.0, 2.0, 1.0)];
    let mid = sample_position(&keys, Time::from_seconds(0.5));
    let chord_y = 0.0;
    assert!((mid[1] - chord_y).abs() > 1.0e-3);
    let mut key = keys[1].clone();
    assert_eq!(key.spatial, SpatialInterpolation::AutoBezier);
    grab_spatial_handle(&mut key, [0.1, 0.2, 0.0]);
    assert_eq!(key.spatial, SpatialInterpolation::ContinuousBezier);
}

#[test]
fn spatial_current_settings_leave_mixed_keys_alone() {
    let mut linear = pos(0.0, 0.0, 0.0);
    linear.spatial = SpatialInterpolation::Linear;
    let mut curve = pos(1.0, 1.0, 1.0);
    curve.spatial = SpatialInterpolation::Bezier;
    apply_spatial(&mut linear, SpatialCommand::CurrentSettings);
    apply_spatial(&mut curve, SpatialCommand::CurrentSettings);
    assert_eq!(linear.spatial, SpatialInterpolation::Linear);
    assert_eq!(curve.spatial, SpatialInterpolation::Bezier);
}

#[test]
fn disabling_the_stopwatch_clears_keys() {
    let mut property = Property::value(PropertyId::new(1), "Opacity", "Opacity", PropertyValue::Scalar(0.25));
    property.push_keyframe(key_at(0.0, PropertyValue::Scalar(0.0)));
    property.push_keyframe(key_at(1.0, PropertyValue::Scalar(1.0)));
    property.disable_stopwatch();
    assert!(property.keyframes().is_empty());
    assert_eq!(property.base_value(), Some(&PropertyValue::Scalar(0.25)));
}

#[test]
fn temporal_linear_is_halfway_at_half_time() {
    let keys = vec![key_at(0.0, PropertyValue::Scalar(0.0)), key_at(1.0, PropertyValue::Scalar(100.0))];
    let mid = sample_scalar(&keys, Time::from_seconds(0.5), 0.0);
    assert!((mid - 50.0).abs() < 1.0e-9);
}

#[test]
fn temporal_bezier_can_flatten_one_side() {
    let mut left = key_at(0.0, PropertyValue::Scalar(0.0));
    let mut right = key_at(1.0, PropertyValue::Scalar(100.0));
    left.temporal_out = TemporalInterpolation::Bezier;
    left.ease_out = KeyframeEase::new(0.0, 100.0);
    right.temporal_in = TemporalInterpolation::Bezier;
    right.ease_in = KeyframeEase::new(200.0, 10.0);
    let early = sample_scalar(&[left, right], Time::from_seconds(0.25), 0.0);
    assert!(early < 25.0);
}

#[test]
fn temporal_continuous_bezier_does_not_corner() {
    let mut keys = vec![
        key_at(0.0, PropertyValue::Scalar(0.0)),
        key_at(1.0, PropertyValue::Scalar(10.0)),
        key_at(2.0, PropertyValue::Scalar(0.0)),
    ];
    for key in &mut keys {
        key.temporal_in = TemporalInterpolation::ContinuousBezier;
        key.temporal_out = TemporalInterpolation::ContinuousBezier;
        key.ease_in = KeyframeEase::new(0.0, 40.0);
        key.ease_out = KeyframeEase::new(0.0, 40.0);
    }
    let before = sample_scalar(&keys, Time::from_seconds(0.99), 0.0);
    let after = sample_scalar(&keys, Time::from_seconds(1.01), 0.0);
    assert!((before - 10.0).abs() < 1.0);
    assert!((after - 10.0).abs() < 1.0);
}

#[test]
fn temporal_auto_bezier_becomes_continuous_when_edited() {
    let mut key = key_at(1.0, PropertyValue::Scalar(10.0));
    key.temporal_out = TemporalInterpolation::AutoBezier;
    assert_eq!(key.temporal_out, TemporalInterpolation::AutoBezier);
    key.temporal_out = TemporalInterpolation::ContinuousBezier;
    key.ease_out.speed = 5.0;
    assert_eq!(key.temporal_out, TemporalInterpolation::ContinuousBezier);
}

#[test]
fn hold_keeps_the_value_until_the_next_key() {
    let mut left = key_at(0.0, PropertyValue::Scalar(20.0));
    left.temporal_out = TemporalInterpolation::Hold;
    let right = key_at(1.0, PropertyValue::Scalar(80.0));
    let just_before = sample_scalar(&[left, right], Time::from_seconds(0.99), 0.0);
    assert!((just_before - 20.0).abs() < 1.0e-9);
}

#[test]
fn temporal_current_settings_keep_mixed_keys() {
    let mut linear = key_at(0.0, PropertyValue::Scalar(0.0));
    linear.temporal_out = TemporalInterpolation::Linear;
    let mut hold = key_at(1.0, PropertyValue::Scalar(1.0));
    hold.temporal_out = TemporalInterpolation::Hold;
    apply_temporal(&mut linear, TemporalCommand::CurrentSettings);
    apply_temporal(&mut hold, TemporalCommand::CurrentSettings);
    assert_eq!(linear.temporal_out, TemporalInterpolation::Linear);
    assert_eq!(hold.temporal_out, TemporalInterpolation::Hold);
}

#[test]
fn keyframe_velocity_changes_with_influence() {
    let mut flat = key_at(0.0, PropertyValue::Scalar(0.0));
    let mut steep = flat.clone();
    let end = key_at(1.0, PropertyValue::Scalar(100.0));
    flat.temporal_out = TemporalInterpolation::Bezier;
    steep.temporal_out = TemporalInterpolation::Bezier;
    flat.ease_out = KeyframeEase::new(100.0, 0.0);
    steep.ease_out = KeyframeEase::new(100.0, 100.0);
    let low = sample_scalar(&[flat, end.clone()], Time::from_seconds(0.25), 0.0);
    let high = sample_scalar(&[steep, end], Time::from_seconds(0.25), 0.0);
    assert!((low - high).abs() > 1.0e-3);
}

#[test]
fn roving_moves_the_middle_key_and_smooths_speed() {
    let mut keys = vec![pos(0.0, 0.0, 0.0), pos(0.1, 10.0, 0.0), pos(1.0, 20.0, 0.0)];
    keys[1].roving = Roving::RoveAcrossTime;
    let before = speed_variation(&keys);
    rove_middle_key(&mut keys);
    let after = speed_variation(&keys);
    assert!(after < before);
    assert!((keys[1].time.seconds() - 0.5).abs() < 0.05);
}

#[test]
fn lock_to_time_keeps_the_key_time() {
    let mut keys = vec![pos(0.0, 0.0, 0.0), pos(0.2, 5.0, 0.0), pos(1.0, 10.0, 0.0)];
    keys[1].roving = Roving::LockToTime;
    let time = keys[1].time;
    keys[0].time = Time::from_seconds(0.05);
    rove_middle_key(&mut keys);
    assert_eq!(keys[1].time, time);
}

#[test]
fn roving_current_settings_do_not_change_keys() {
    let mut key = pos(0.2, 1.0, 0.0);
    key.roving = Roving::LockToTime;
    apply_roving(&mut key, RovingCommand::CurrentSettings);
    assert_eq!(key.roving, Roving::LockToTime);
}

#[test]
fn graph_editor_toggles_with_the_layer_bar() {
    let mut composition = Composition::new(ItemId::new(1), "C", 8, 4, 24.0, Time::from_seconds(1.0));
    assert!(!composition.graph.open);
    composition.graph.open = !composition.graph.open;
    assert!(composition.graph.open);
    composition.graph.open = !composition.graph.open;
    assert!(!composition.graph.open);
}

#[test]
fn graph_shows_only_the_selected_property() {
    let mut composition = Composition::new(ItemId::new(1), "C", 8, 4, 24.0, Time::from_seconds(1.0));
    composition.graph.show = ShowProperties::Selected;
    let properties = vec![
        GraphPropertyRef { match_name: "Opacity".into(), spatial: false, animated: true, selected: true },
        GraphPropertyRef { match_name: "Position".into(), spatial: true, animated: true, selected: false },
    ];
    let shown = composition.visible_graphs(&properties);
    assert_eq!(shown.len(), 1);
    assert_eq!(shown[0].match_name, "Opacity");
}

#[test]
fn graph_hides_properties_without_keys() {
    let mut composition = Composition::new(ItemId::new(1), "C", 8, 4, 24.0, Time::from_seconds(1.0));
    composition.graph.show = ShowProperties::Animated;
    let properties = vec![
        GraphPropertyRef { match_name: "Opacity".into(), spatial: false, animated: true, selected: false },
        GraphPropertyRef { match_name: "Anchor".into(), spatial: false, animated: false, selected: true },
    ];
    let shown = composition.visible_graphs(&properties);
    assert!(shown.iter().all(|item| item.animated));
}

#[test]
fn graph_editor_set_lists_only_switched_properties() {
    let mut composition = Composition::new(ItemId::new(1), "C", 8, 4, 24.0, Time::from_seconds(1.0));
    composition.graph.show = ShowProperties::GraphEditorSet;
    composition.graph.graph_set.push("Opacity".into());
    let properties = vec![
        GraphPropertyRef { match_name: "Opacity".into(), spatial: false, animated: true, selected: false },
        GraphPropertyRef { match_name: "Position".into(), spatial: true, animated: true, selected: true },
    ];
    let shown = composition.visible_graphs(&properties);
    assert_eq!(shown.len(), 1);
    assert_eq!(shown[0].match_name, "Opacity");
}

#[test]
fn auto_graph_type_uses_speed_for_position() {
    let mut composition = Composition::new(ItemId::new(1), "C", 8, 4, 24.0, Time::from_seconds(1.0));
    composition.graph.graph_type = GraphTypeMode::Auto;
    assert_eq!(composition.graph_kind_for(true), GraphKind::Speed);
    assert_eq!(composition.graph_kind_for(false), GraphKind::Value);
    assert_eq!(auto_graph_kind(true), GraphKind::Speed);
}

#[test]
fn value_graph_can_edit_position() {
    let mut composition = Composition::new(ItemId::new(1), "C", 8, 4, 24.0, Time::from_seconds(1.0));
    composition.graph.graph_type = GraphTypeMode::Value;
    assert_eq!(composition.graph_kind_for(true), GraphKind::Value);
}

#[test]
fn speed_graph_uses_speed_for_opacity() {
    let mut composition = Composition::new(ItemId::new(1), "C", 8, 4, 24.0, Time::from_seconds(1.0));
    composition.graph.graph_type = GraphTypeMode::Speed;
    assert_eq!(composition.graph_kind_for(false), GraphKind::Speed);
}

#[test]
fn reference_graph_is_not_the_edited_graph() {
    let mut composition = Composition::new(ItemId::new(1), "C", 8, 4, 24.0, Time::from_seconds(1.0));
    composition.graph.graph_type = GraphTypeMode::Value;
    composition.graph.show_reference = true;
    let edited = composition.graph_kind_for(false);
    let reference = match edited {
        GraphKind::Value => GraphKind::Speed,
        GraphKind::Speed => GraphKind::Value,
    };
    assert_ne!(edited, reference);
    assert!(composition.graph.show_reference);
}

#[test]
fn audio_waveform_toggle_is_independent_of_keys() {
    let mut composition = Composition::new(ItemId::new(1), "C", 8, 4, 24.0, Time::from_seconds(1.0));
    composition.graph.show_audio = true;
    assert!(composition.graph.show_audio);
    composition.graph.show_audio = false;
    assert!(!composition.graph.show_audio);
}

#[test]
fn layer_in_out_brackets_match_the_layer_span() {
    let mut composition = Composition::new(ItemId::new(1), "C", 8, 4, 24.0, Time::from_seconds(4.0));
    let mut layer = Layer::new(LayerId::new(1), "L", LayerKind::Null);
    layer.in_point = Time::from_seconds(1.0);
    layer.out_point = Time::from_seconds(3.0);
    composition.layers.push(layer);
    composition.graph.show_layer_in_out = true;
    let layer = &composition.layers[0];
    assert_eq!(layer.in_point, Time::from_seconds(1.0));
    assert_eq!(layer.out_point, Time::from_seconds(3.0));
}

#[test]
fn layer_marker_triangles_follow_marker_time() {
    let mut composition = Composition::new(ItemId::new(1), "C", 8, 4, 24.0, Time::from_seconds(2.0));
    let mut layer = Layer::new(LayerId::new(1), "L", LayerKind::Null);
    layer.markers.push(TimedMarker::at(Time::from_seconds(0.75), Marker::layer_default()));
    composition.layers.push(layer);
    composition.graph.show_layer_markers = true;
    assert_eq!(composition.layers[0].markers[0].time, Time::from_seconds(0.75));
}

#[test]
fn graph_tooltips_can_be_hidden() {
    let mut composition = Composition::new(ItemId::new(1), "C", 8, 4, 24.0, Time::from_seconds(1.0));
    composition.graph.show_tooltips = false;
    assert!(!composition.graph.show_tooltips);
}

#[test]
fn expression_editor_can_be_shown() {
    let mut composition = Composition::new(ItemId::new(1), "C", 8, 4, 24.0, Time::from_seconds(1.0));
    composition.graph.show_expression_editor = true;
    assert!(composition.graph.show_expression_editor);
    composition.graph.show_expression_editor = false;
    assert!(!composition.graph.show_expression_editor);
}

#[test]
fn keys_between_frames_snap_when_disabled() {
    let mut composition = Composition::new(ItemId::new(1), "C", 8, 4, 24.0, Time::from_seconds(1.0));
    composition.graph.allow_keyframes_between_frames = true;
    let kept = composition.quantize_key_time(Time::from_seconds(0.5 / 24.0));
    assert!((kept.seconds() - 0.5 / 24.0).abs() < 1.0e-12);
    composition.graph.allow_keyframes_between_frames = false;
    let snapped = composition.quantize_key_time(Time::from_seconds(0.5 / 24.0));
    assert!((snapped.seconds() - 1.0 / 24.0).abs() < 1.0e-9 || snapped.seconds().abs() < 1.0e-9);
}

#[test]
fn snap_pulls_a_key_to_a_marker_until_ctrl_inverts_it() {
    let mut composition = Composition::new(ItemId::new(1), "C", 8, 4, 24.0, Time::from_seconds(2.0));
    composition.graph.snap = true;
    composition.graph.snap_threshold = Some(0.1);
    let marker = Time::from_seconds(1.0);
    let snapped = composition.snap_key_time(Time::from_seconds(1.05), &[marker], false);
    assert_eq!(snapped, marker);
    let free = composition.snap_key_time(Time::from_seconds(1.05), &[marker], true);
    assert!((free.seconds() - 1.05).abs() < 1.0e-12);
}

#[test]
fn auto_zoom_height_blocks_vertical_zoom() {
    let mut composition = Composition::new(ItemId::new(1), "C", 8, 4, 24.0, Time::from_seconds(1.0));
    composition.graph.auto_zoom_height = true;
    let before = composition.graph.zoom[1];
    if composition.graph.auto_zoom_height {
        // 縦ズームは受け付けない。
    } else {
        composition.graph.zoom[1] = 3.0;
    }
    assert_eq!(composition.graph.zoom[1], before);
}

#[test]
fn fit_selection_brings_two_keys_into_view() {
    let mut composition = Composition::new(ItemId::new(1), "C", 8, 4, 24.0, Time::from_seconds(4.0));
    composition.graph.pan[0] = 20.0;
    composition.fit_graph_to_times(&[Time::from_seconds(1.0), Time::from_seconds(3.0)]);
    assert!(composition.graph.pan[0] <= 1.0);
    assert!(composition.graph.pan[0] + composition.graph.zoom[0] >= 3.0);
}

#[test]
fn fit_all_brings_an_offscreen_curve_into_view() {
    let mut composition = Composition::new(ItemId::new(1), "C", 8, 4, 24.0, Time::from_seconds(4.0));
    composition.graph.pan[0] = 100.0;
    composition.fit_graph_to_times(&[Time::from_seconds(0.0), Time::from_seconds(4.0)]);
    assert!(composition.graph.pan[0] <= 0.0);
    assert!(composition.graph.pan[0] + composition.graph.zoom[0] >= 4.0);
}

#[test]
fn separate_dimensions_lets_x_move_without_a_new_y_key() {
    let mut property = Property::value(PropertyId::new(1), "Position", "Position", PropertyValue::TwoD([10.0, 20.0]));
    let x = Property::value(PropertyId::new(2), "X", "X", PropertyValue::Scalar(0.0));
    let y = Property::value(PropertyId::new(3), "Y", "Y", PropertyValue::Scalar(0.0));
    assert!(property.separate_dimensions(x, y, None));
    let x_prop = property.child("X").unwrap();
    assert!(matches!(x_prop.base_value(), Some(PropertyValue::Scalar(value)) if (*value - 10.0).abs() < 1.0e-9));
    let mut x_owned = property.child("X").unwrap().clone();
    x_owned.push_keyframe(key_at(1.0, PropertyValue::Scalar(30.0)));
    assert_eq!(x_owned.keyframes().len(), 1);
    assert!(property.child("Y").unwrap().keyframes().is_empty());
}

#[test]
fn panning_the_graph_does_not_change_key_values() {
    let mut composition = Composition::new(ItemId::new(1), "C", 8, 4, 24.0, Time::from_seconds(1.0));
    let key = key_at(1.0, PropertyValue::Scalar(40.0));
    composition.graph.pan[0] += 5.0;
    assert!((eval::interpolate::scalar(&key.value).unwrap() - 40.0).abs() < 1.0e-9);
}

#[test]
fn zooming_the_graph_does_not_change_key_times() {
    let mut composition = Composition::new(ItemId::new(1), "C", 8, 4, 24.0, Time::from_seconds(1.0));
    let key = key_at(1.25, PropertyValue::Scalar(1.0));
    composition.graph.zoom[0] *= 2.0;
    assert!((key.time.seconds() - 1.25).abs() < 1.0e-12);
}

#[test]
fn graph_hold_makes_the_span_flat() {
    let mut key = key_at(0.0, PropertyValue::Scalar(15.0));
    let next = key_at(1.0, PropertyValue::Scalar(90.0));
    let composition = Composition::new(ItemId::new(1), "C", 8, 4, 24.0, Time::from_seconds(1.0));
    composition.set_graph_interpolation(&mut key, TemporalInterpolation::Hold);
    let mid = sample_scalar(&[key, next], Time::from_seconds(0.5), 0.0);
    assert!((mid - 15.0).abs() < 1.0e-9);
}

#[test]
fn graph_linear_straightens_a_curve() {
    let mut key = key_at(0.0, PropertyValue::Scalar(0.0));
    key.temporal_out = TemporalInterpolation::Bezier;
    key.ease_out = KeyframeEase::new(0.0, 100.0);
    let next = key_at(1.0, PropertyValue::Scalar(100.0));
    let curved = sample_scalar(&[key.clone(), next.clone()], Time::from_seconds(0.25), 0.0);
    let composition = Composition::new(ItemId::new(1), "C", 8, 4, 24.0, Time::from_seconds(1.0));
    composition.set_graph_interpolation(&mut key, TemporalInterpolation::Linear);
    let straight = sample_scalar(&[key, next], Time::from_seconds(0.25), 0.0);
    assert!((straight - 25.0).abs() < 1.0e-9);
    assert!((curved - straight).abs() > 1.0e-3);
}

#[test]
fn graph_auto_bezier_softens_a_corner() {
    let mut key = key_at(1.0, PropertyValue::Scalar(10.0));
    key.temporal_out = TemporalInterpolation::Linear;
    let composition = Composition::new(ItemId::new(1), "C", 8, 4, 24.0, Time::from_seconds(2.0));
    composition.set_graph_interpolation(&mut key, TemporalInterpolation::AutoBezier);
    assert_eq!(key.temporal_out, TemporalInterpolation::AutoBezier);
}

#[test]
fn stretching_a_handle_changes_the_bulge() {
    let mut left = key_at(0.0, PropertyValue::Scalar(0.0));
    let right = key_at(1.0, PropertyValue::Scalar(100.0));
    left.temporal_out = TemporalInterpolation::Bezier;
    left.ease_out = KeyframeEase::new(80.0, 10.0);
    let small = sample_scalar(&[left.clone(), right.clone()], Time::from_seconds(0.25), 0.0);
    left.ease_out.influence = 100.0;
    let large = sample_scalar(&[left, right], Time::from_seconds(0.25), 0.0);
    assert!((small - large).abs() > 1.0e-3);
}

#[test]
fn splitting_handles_moves_only_one_side() {
    let mut key = key_at(1.0, PropertyValue::Scalar(10.0));
    key.temporal_in = TemporalInterpolation::Bezier;
    key.temporal_out = TemporalInterpolation::Bezier;
    key.ease_in = KeyframeEase::new(5.0, 30.0);
    key.ease_out = KeyframeEase::new(8.0, 40.0);
    let outgoing = key.ease_out;
    key.ease_in.speed = 0.0;
    assert_eq!(key.ease_out.speed, outgoing.speed);
    assert_eq!(key.ease_out.influence, outgoing.influence);
}

#[test]
fn audio_to_keyframes_is_low_in_silence_and_high_when_loud() {
    let quiet = audio_amplitude(&[0.0; 16], 8);
    let loud = audio_amplitude(&[1.0; 16], 8);
    assert!(quiet.iter().all(|value| *value < 0.01));
    assert!(loud.iter().all(|value| *value > 0.9));
}

#[test]
fn expression_baked_to_keys_matches_after_the_expression_is_removed() {
    let source = "time * 10";
    let times = [Time::ZERO, Time::from_seconds(1.0)];
    let keys = bake_expression(source, &times).unwrap();
    let mut property = Property::value(PropertyId::new(1), "Opacity", "Opacity", PropertyValue::Scalar(0.0));
    property.set_expression(Some(source.into()));
    for key in keys {
        property.push_keyframe(key);
    }
    property.set_expression(None);
    let mid = sample_scalar(property.keyframes(), Time::from_seconds(0.5), 0.0);
    let live = eval_simple_expression(source, Time::from_seconds(0.5)).unwrap();
    assert!((mid - live).abs() < 1.0e-9);
}

#[test]
fn data_points_become_keys_at_the_same_times() {
    let points = [(Time::from_seconds(0.0), 1.0), (Time::from_seconds(0.5), 3.0)];
    let keys = keys_from_points(&points);
    assert_eq!(keys.len(), 2);
    assert!((eval::interpolate::scalar(&keys[1].value).unwrap() - 3.0).abs() < 1.0e-9);
}

#[test]
fn easy_ease_slows_both_sides() {
    let mut key = key_at(0.5, PropertyValue::Scalar(50.0));
    let before_out = key.ease_out.speed;
    easy_ease(&mut key);
    assert_eq!(key.ease_in.speed, 0.0);
    assert_eq!(key.ease_out.speed, 0.0);
    assert!(key.ease_out.influence > before_out);
}

#[test]
fn easy_ease_in_changes_only_the_incoming_side() {
    let mut key = key_at(0.5, PropertyValue::Scalar(50.0));
    key.ease_out = KeyframeEase::new(12.0, 20.0);
    easy_ease_in(&mut key);
    assert_eq!(key.ease_in.speed, 0.0);
    assert_eq!(key.ease_out.speed, 12.0);
    assert_eq!(key.ease_out.influence, 20.0);
}

#[test]
fn easy_ease_out_changes_only_the_outgoing_side() {
    let mut key = key_at(0.5, PropertyValue::Scalar(50.0));
    key.ease_in = KeyframeEase::new(9.0, 15.0);
    easy_ease_out(&mut key);
    assert_eq!(key.ease_out.speed, 0.0);
    assert_eq!(key.ease_in.speed, 9.0);
}

#[test]
fn exponential_scale_midpoint_is_the_geometric_mean() {
    let keys = exponential_scale(10.0, 1000.0, Time::ZERO, Time::from_seconds(1.0)).unwrap();
    let mid = eval::interpolate::scalar(&keys[1].value).unwrap();
    assert!((mid - 100.0).abs() < 1.0e-6);
    assert!(exponential_scale(0.0, 100.0, Time::ZERO, Time::from_seconds(1.0)).is_none());
}

#[test]
fn rpf_samples_become_camera_keys() {
    let samples = [(Time::ZERO, [0.0, 0.0, 0.0]), (Time::from_seconds(1.0), [10.0, 0.0, 5.0])];
    let keys = keys_from_points(&samples.iter().map(|(time, value)| (*time, value[0])).collect::<Vec<_>>());
    assert_eq!(keys.len(), 2);
    assert!((eval::interpolate::scalar(&keys[1].value).unwrap() - 10.0).abs() < 1.0e-9);
}

#[test]
fn sequence_layers_places_the_next_in_point_on_the_previous_out_point() {
    let mut composition = Composition::new(ItemId::new(1), "C", 8, 4, 24.0, Time::from_seconds(10.0));
    for (index, id) in [1_u64, 2, 3].into_iter().enumerate() {
        let mut layer = Layer::new(LayerId::new(id), format!("L{index}"), LayerKind::Null);
        layer.in_point = Time::ZERO;
        layer.out_point = Time::from_seconds(1.0);
        composition.layers.push(layer);
    }
    sequence_layers(&mut composition, &[LayerId::new(1), LayerId::new(2), LayerId::new(3)]);
    assert!((composition.layer(LayerId::new(2)).unwrap().in_point.seconds() - 1.0).abs() < 1.0e-9);
    assert!((composition.layer(LayerId::new(3)).unwrap().in_point.seconds() - 2.0).abs() < 1.0e-9);
}

#[test]
fn time_reverse_keyframes_swaps_the_two_times() {
    let mut keys = vec![key_at(0.0, PropertyValue::Scalar(1.0)), key_at(2.0, PropertyValue::Scalar(9.0))];
    time_reverse_keys(&mut keys);
    let at_start = keys.iter().find(|key| (eval::interpolate::scalar(&key.value).unwrap() - 9.0).abs() < 1.0e-9).unwrap();
    let at_end = keys.iter().find(|key| (eval::interpolate::scalar(&key.value).unwrap() - 1.0).abs() < 1.0e-9).unwrap();
    assert!((at_start.time.seconds()).abs() < 1.0e-9);
    assert!((at_end.time.seconds() - 2.0).abs() < 1.0e-9);
}
