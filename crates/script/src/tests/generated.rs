use crate::*;

#[test]
fn match_adbe_plane_options_group() {
    let mut app = App::new();
    let id = app.host_property("ADBE Plane Options Group").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Plane Options Group");
}

#[test]
fn match_adbe_plane_curvature() {
    let mut app = App::new();
    let id = app.host_property("ADBE Plane Curvature").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Plane Curvature");
}

#[test]
fn match_adbe_plane_subdivision() {
    let mut app = App::new();
    let id = app.host_property("ADBE Plane Subdivision").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Plane Subdivision");
}

#[test]
fn match_adbe_extrsn_options_group() {
    let mut app = App::new();
    let id = app.host_property("ADBE Extrsn Options Group").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Extrsn Options Group");
}

#[test]
fn match_adbe_bevel_depth() {
    let mut app = App::new();
    let id = app.host_property("ADBE Bevel Depth").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Bevel Depth");
}

#[test]
fn match_adbe_hole_bevel_depth() {
    let mut app = App::new();
    let id = app.host_property("ADBE Hole Bevel Depth").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Hole Bevel Depth");
}

#[test]
fn match_adbe_extrsn_depth() {
    let mut app = App::new();
    let id = app.host_property("ADBE Extrsn Depth").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Extrsn Depth");
}

#[test]
fn match_adbe_material_options_group() {
    let mut app = App::new();
    let id = app.host_property("ADBE Material Options Group").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Material Options Group");
}

#[test]
fn match_adbe_light_transmission() {
    let mut app = App::new();
    let id = app.host_property("ADBE Light Transmission").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Light Transmission");
}

#[test]
fn match_adbe_ambient_coefficient() {
    let mut app = App::new();
    let id = app.host_property("ADBE Ambient Coefficient").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Ambient Coefficient");
}

#[test]
fn match_adbe_diffuse_coefficient() {
    let mut app = App::new();
    let id = app.host_property("ADBE Diffuse Coefficient").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Diffuse Coefficient");
}

#[test]
fn match_adbe_specular_coefficient() {
    let mut app = App::new();
    let id = app.host_property("ADBE Specular Coefficient").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Specular Coefficient");
}

#[test]
fn match_adbe_shininess_coefficient() {
    let mut app = App::new();
    let id = app.host_property("ADBE Shininess Coefficient").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Shininess Coefficient");
}

#[test]
fn match_adbe_metal_coefficient() {
    let mut app = App::new();
    let id = app.host_property("ADBE Metal Coefficient").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Metal Coefficient");
}

#[test]
fn match_adbe_reflection_coefficient() {
    let mut app = App::new();
    let id = app.host_property("ADBE Reflection Coefficient").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Reflection Coefficient");
}

#[test]
fn match_adbe_glossiness_coefficient() {
    let mut app = App::new();
    let id = app.host_property("ADBE Glossiness Coefficient").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Glossiness Coefficient");
}

#[test]
fn match_adbe_fresnel_coefficient() {
    let mut app = App::new();
    let id = app.host_property("ADBE Fresnel Coefficient").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Fresnel Coefficient");
}

#[test]
fn match_adbe_transparency_coefficient() {
    let mut app = App::new();
    let id = app.host_property("ADBE Transparency Coefficient").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Transparency Coefficient");
}

#[test]
fn match_adbe_transp_rolloff() {
    let mut app = App::new();
    let id = app.host_property("ADBE Transp Rolloff").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Transp Rolloff");
}

#[test]
fn match_adbe_index_of_refraction() {
    let mut app = App::new();
    let id = app.host_property("ADBE Index of Refraction").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Index of Refraction");
}

#[test]
fn match_adbe_av_layer() {
    let mut app = App::new();
    let id = app.host_property("ADBE AV Layer").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE AV Layer");
}

#[test]
fn match_adbe_marker() {
    let mut app = App::new();
    let id = app.host_property("ADBE Marker").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Marker");
}

#[test]
fn match_adbe_time_remapping() {
    let mut app = App::new();
    let id = app.host_property("ADBE Time Remapping").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Time Remapping");
}

#[test]
fn match_adbe_mtrackers() {
    let mut app = App::new();
    let id = app.host_property("ADBE MTrackers").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE MTrackers");
}

#[test]
fn match_adbe_mask_parade() {
    let mut app = App::new();
    let id = app.host_property("ADBE Mask Parade").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Mask Parade");
}

#[test]
fn match_adbe_effect_parade() {
    let mut app = App::new();
    let id = app.host_property("ADBE Effect Parade").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Effect Parade");
}

#[test]
fn match_adbe_layer_overrides() {
    let mut app = App::new();
    let id = app.host_property("ADBE Layer Overrides").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Layer Overrides");
}

#[test]
fn match_adbe_transform_group() {
    let mut app = App::new();
    let id = app.host_property("ADBE Transform Group").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Transform Group");
}

#[test]
fn match_adbe_anchor_point() {
    let mut app = App::new();
    let id = app.host_property("ADBE Anchor Point").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Anchor Point");
}

#[test]
fn match_adbe_position() {
    let mut app = App::new();
    let id = app.host_property("ADBE Position").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Position");
}

#[test]
fn match_adbe_position_0() {
    let mut app = App::new();
    let id = app.host_property("ADBE Position_0").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Position_0");
}

#[test]
fn match_adbe_position_1() {
    let mut app = App::new();
    let id = app.host_property("ADBE Position_1").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Position_1");
}

#[test]
fn match_adbe_position_2() {
    let mut app = App::new();
    let id = app.host_property("ADBE Position_2").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Position_2");
}

#[test]
fn match_adbe_scale() {
    let mut app = App::new();
    let id = app.host_property("ADBE Scale").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Scale");
}

#[test]
fn match_adbe_orientation() {
    let mut app = App::new();
    let id = app.host_property("ADBE Orientation").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Orientation");
}

#[test]
fn match_adbe_rotate_x() {
    let mut app = App::new();
    let id = app.host_property("ADBE Rotate X").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Rotate X");
}

#[test]
fn match_adbe_rotate_y() {
    let mut app = App::new();
    let id = app.host_property("ADBE Rotate Y").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Rotate Y");
}

#[test]
fn match_adbe_rotate_z() {
    let mut app = App::new();
    let id = app.host_property("ADBE Rotate Z").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Rotate Z");
}

#[test]
fn match_adbe_opacity() {
    let mut app = App::new();
    let id = app.host_property("ADBE Opacity").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Opacity");
}

#[test]
fn match_adbe_audio_group() {
    let mut app = App::new();
    let id = app.host_property("ADBE Audio Group").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Audio Group");
}

#[test]
fn match_adbe_audio_levels() {
    let mut app = App::new();
    let id = app.host_property("ADBE Audio Levels").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Audio Levels");
}

#[test]
fn match_adbe_layer_source_alternate() {
    let mut app = App::new();
    let id = app.host_property("ADBE Layer Source Alternate").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Layer Source Alternate");
}

#[test]
fn match_adbe_camera_layer() {
    let mut app = App::new();
    let id = app.host_property("ADBE Camera Layer").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Camera Layer");
}

#[test]
fn match_adbe_camera_options_group() {
    let mut app = App::new();
    let id = app.host_property("ADBE Camera Options Group").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Camera Options Group");
}

#[test]
fn match_adbe_camera_zoom() {
    let mut app = App::new();
    let id = app.host_property("ADBE Camera Zoom").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Camera Zoom");
}

#[test]
fn match_adbe_camera_depth_of_field() {
    let mut app = App::new();
    let id = app.host_property("ADBE Camera Depth of Field").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Camera Depth of Field");
}

#[test]
fn match_adbe_camera_focus_distance() {
    let mut app = App::new();
    let id = app.host_property("ADBE Camera Focus Distance").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Camera Focus Distance");
}

#[test]
fn match_adbe_camera_aperture() {
    let mut app = App::new();
    let id = app.host_property("ADBE Camera Aperture").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Camera Aperture");
}

#[test]
fn match_adbe_camera_blur_level() {
    let mut app = App::new();
    let id = app.host_property("ADBE Camera Blur Level").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Camera Blur Level");
}

#[test]
fn match_adbe_iris_shape() {
    let mut app = App::new();
    let id = app.host_property("ADBE Iris Shape").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Iris Shape");
}

#[test]
fn match_adbe_iris_rotation() {
    let mut app = App::new();
    let id = app.host_property("ADBE Iris Rotation").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Iris Rotation");
}

#[test]
fn match_adbe_iris_roundness() {
    let mut app = App::new();
    let id = app.host_property("ADBE Iris Roundness").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Iris Roundness");
}

#[test]
fn match_adbe_iris_aspect_ratio() {
    let mut app = App::new();
    let id = app.host_property("ADBE Iris Aspect Ratio").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Iris Aspect Ratio");
}

#[test]
fn match_adbe_iris_diffraction_fringe() {
    let mut app = App::new();
    let id = app.host_property("ADBE Iris Diffraction Fringe").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Iris Diffraction Fringe");
}

#[test]
fn match_adbe_iris_highlight_gain() {
    let mut app = App::new();
    let id = app.host_property("ADBE Iris Highlight Gain").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Iris Highlight Gain");
}

#[test]
fn match_adbe_iris_highlight_threshold() {
    let mut app = App::new();
    let id = app.host_property("ADBE Iris Highlight Threshold").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Iris Highlight Threshold");
}

#[test]
fn match_adbe_iris_hightlight_saturation() {
    let mut app = App::new();
    let id = app.host_property("ADBE Iris Hightlight Saturation").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Iris Hightlight Saturation");
}

#[test]
fn match_adbe_blend_options_group() {
    let mut app = App::new();
    let id = app.host_property("ADBE Blend Options Group").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Blend Options Group");
}

#[test]
fn match_adbe_global_angle2() {
    let mut app = App::new();
    let id = app.host_property("ADBE Global Angle2").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Global Angle2");
}

#[test]
fn match_adbe_global_altitude2() {
    let mut app = App::new();
    let id = app.host_property("ADBE Global Altitude2").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Global Altitude2");
}

#[test]
fn match_adbe_adv_blend_group() {
    let mut app = App::new();
    let id = app.host_property("ADBE Adv Blend Group").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Adv Blend Group");
}

#[test]
fn match_adbe_layer_fill_opacity2() {
    let mut app = App::new();
    let id = app.host_property("ADBE Layer Fill Opacity2").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Layer Fill Opacity2");
}

#[test]
fn match_adbe_r_channel_blend() {
    let mut app = App::new();
    let id = app.host_property("ADBE R Channel Blend").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE R Channel Blend");
}

#[test]
fn match_adbe_g_channel_blend() {
    let mut app = App::new();
    let id = app.host_property("ADBE G Channel Blend").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE G Channel Blend");
}

#[test]
fn match_adbe_b_channel_blend() {
    let mut app = App::new();
    let id = app.host_property("ADBE B Channel Blend").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE B Channel Blend");
}

#[test]
fn match_adbe_blend_interior() {
    let mut app = App::new();
    let id = app.host_property("ADBE Blend Interior").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Blend Interior");
}

#[test]
fn match_adbe_blend_ranges() {
    let mut app = App::new();
    let id = app.host_property("ADBE Blend Ranges").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Blend Ranges");
}

#[test]
fn match_dropshadow_enabled() {
    let mut app = App::new();
    let id = app.host_property("dropShadow/enabled").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "dropShadow/enabled");
}

#[test]
fn match_dropshadow_mode2() {
    let mut app = App::new();
    let id = app.host_property("dropShadow/mode2").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "dropShadow/mode2");
}

#[test]
fn match_dropshadow_color() {
    let mut app = App::new();
    let id = app.host_property("dropShadow/color").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "dropShadow/color");
}

#[test]
fn match_dropshadow_opacity() {
    let mut app = App::new();
    let id = app.host_property("dropShadow/opacity").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "dropShadow/opacity");
}

#[test]
fn match_dropshadow_useglobalangle() {
    let mut app = App::new();
    let id = app.host_property("dropShadow/useGlobalAngle").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "dropShadow/useGlobalAngle");
}

#[test]
fn match_dropshadow_locallightingangle() {
    let mut app = App::new();
    let id = app.host_property("dropShadow/localLightingAngle").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "dropShadow/localLightingAngle");
}

#[test]
fn match_dropshadow_distance() {
    let mut app = App::new();
    let id = app.host_property("dropShadow/distance").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "dropShadow/distance");
}

#[test]
fn match_dropshadow_chokematte() {
    let mut app = App::new();
    let id = app.host_property("dropShadow/chokeMatte").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "dropShadow/chokeMatte");
}

#[test]
fn match_dropshadow_blur() {
    let mut app = App::new();
    let id = app.host_property("dropShadow/blur").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "dropShadow/blur");
}

#[test]
fn match_dropshadow_noise() {
    let mut app = App::new();
    let id = app.host_property("dropShadow/noise").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "dropShadow/noise");
}

#[test]
fn match_dropshadow_layerconceals() {
    let mut app = App::new();
    let id = app.host_property("dropShadow/layerConceals").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "dropShadow/layerConceals");
}

#[test]
fn match_innershadow_enabled() {
    let mut app = App::new();
    let id = app.host_property("innerShadow/enabled").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "innerShadow/enabled");
}

#[test]
fn match_innershadow_mode2() {
    let mut app = App::new();
    let id = app.host_property("innerShadow/mode2").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "innerShadow/mode2");
}

#[test]
fn match_innershadow_color() {
    let mut app = App::new();
    let id = app.host_property("innerShadow/color").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "innerShadow/color");
}

#[test]
fn match_innershadow_opacity() {
    let mut app = App::new();
    let id = app.host_property("innerShadow/opacity").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "innerShadow/opacity");
}

#[test]
fn match_innershadow_useglobalangle() {
    let mut app = App::new();
    let id = app.host_property("innerShadow/useGlobalAngle").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "innerShadow/useGlobalAngle");
}

#[test]
fn match_innershadow_locallightingangle() {
    let mut app = App::new();
    let id = app.host_property("innerShadow/localLightingAngle").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "innerShadow/localLightingAngle");
}

#[test]
fn match_innershadow_distance() {
    let mut app = App::new();
    let id = app.host_property("innerShadow/distance").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "innerShadow/distance");
}

#[test]
fn match_innershadow_chokematte() {
    let mut app = App::new();
    let id = app.host_property("innerShadow/chokeMatte").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "innerShadow/chokeMatte");
}

#[test]
fn match_innershadow_blur() {
    let mut app = App::new();
    let id = app.host_property("innerShadow/blur").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "innerShadow/blur");
}

#[test]
fn match_innershadow_noise() {
    let mut app = App::new();
    let id = app.host_property("innerShadow/noise").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "innerShadow/noise");
}

#[test]
fn match_outerglow_enabled() {
    let mut app = App::new();
    let id = app.host_property("outerGlow/enabled").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "outerGlow/enabled");
}

#[test]
fn match_outerglow_mode2() {
    let mut app = App::new();
    let id = app.host_property("outerGlow/mode2").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "outerGlow/mode2");
}

#[test]
fn match_outerglow_opacity() {
    let mut app = App::new();
    let id = app.host_property("outerGlow/opacity").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "outerGlow/opacity");
}

#[test]
fn match_outerglow_noise() {
    let mut app = App::new();
    let id = app.host_property("outerGlow/noise").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "outerGlow/noise");
}

#[test]
fn match_outerglow_aecolorchoice() {
    let mut app = App::new();
    let id = app.host_property("outerGlow/AEColorChoice").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "outerGlow/AEColorChoice");
}

#[test]
fn match_outerglow_color() {
    let mut app = App::new();
    let id = app.host_property("outerGlow/color").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "outerGlow/color");
}

#[test]
fn match_outerglow_gradient() {
    let mut app = App::new();
    let id = app.host_property("outerGlow/gradient").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "outerGlow/gradient");
}

#[test]
fn match_outerglow_gradientsmoothness() {
    let mut app = App::new();
    let id = app.host_property("outerGlow/gradientSmoothness").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "outerGlow/gradientSmoothness");
}

#[test]
fn match_outerglow_glowtechnique() {
    let mut app = App::new();
    let id = app.host_property("outerGlow/glowTechnique").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "outerGlow/glowTechnique");
}

#[test]
fn match_outerglow_chokematte() {
    let mut app = App::new();
    let id = app.host_property("outerGlow/chokeMatte").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "outerGlow/chokeMatte");
}

#[test]
fn match_outerglow_blur() {
    let mut app = App::new();
    let id = app.host_property("outerGlow/blur").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "outerGlow/blur");
}

#[test]
fn match_outerglow_inputrange() {
    let mut app = App::new();
    let id = app.host_property("outerGlow/inputRange").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "outerGlow/inputRange");
}

#[test]
fn match_outerglow_shadingnoise() {
    let mut app = App::new();
    let id = app.host_property("outerGlow/shadingNoise").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "outerGlow/shadingNoise");
}

#[test]
fn match_innerglow_enabled() {
    let mut app = App::new();
    let id = app.host_property("innerGlow/enabled").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "innerGlow/enabled");
}

#[test]
fn match_innerglow_mode2() {
    let mut app = App::new();
    let id = app.host_property("innerGlow/mode2").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "innerGlow/mode2");
}

#[test]
fn match_innerglow_opacity() {
    let mut app = App::new();
    let id = app.host_property("innerGlow/opacity").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "innerGlow/opacity");
}

#[test]
fn match_innerglow_noise() {
    let mut app = App::new();
    let id = app.host_property("innerGlow/noise").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "innerGlow/noise");
}

#[test]
fn match_innerglow_aecolorchoice() {
    let mut app = App::new();
    let id = app.host_property("innerGlow/AEColorChoice").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "innerGlow/AEColorChoice");
}

#[test]
fn match_innerglow_color() {
    let mut app = App::new();
    let id = app.host_property("innerGlow/color").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "innerGlow/color");
}

#[test]
fn match_innerglow_gradient() {
    let mut app = App::new();
    let id = app.host_property("innerGlow/gradient").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "innerGlow/gradient");
}

#[test]
fn match_innerglow_gradientsmoothness() {
    let mut app = App::new();
    let id = app.host_property("innerGlow/gradientSmoothness").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "innerGlow/gradientSmoothness");
}

#[test]
fn match_innerglow_glowtechnique() {
    let mut app = App::new();
    let id = app.host_property("innerGlow/glowTechnique").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "innerGlow/glowTechnique");
}

#[test]
fn match_innerglow_innerglowsource() {
    let mut app = App::new();
    let id = app.host_property("innerGlow/innerGlowSource").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "innerGlow/innerGlowSource");
}

#[test]
fn match_innerglow_chokematte() {
    let mut app = App::new();
    let id = app.host_property("innerGlow/chokeMatte").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "innerGlow/chokeMatte");
}

#[test]
fn match_innerglow_blur() {
    let mut app = App::new();
    let id = app.host_property("innerGlow/blur").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "innerGlow/blur");
}

#[test]
fn match_innerglow_inputrange() {
    let mut app = App::new();
    let id = app.host_property("innerGlow/inputRange").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "innerGlow/inputRange");
}

#[test]
fn match_innerglow_shadingnoise() {
    let mut app = App::new();
    let id = app.host_property("innerGlow/shadingNoise").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "innerGlow/shadingNoise");
}

#[test]
fn match_bevelemboss_enabled() {
    let mut app = App::new();
    let id = app.host_property("bevelEmboss/enabled").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "bevelEmboss/enabled");
}

#[test]
fn match_bevelemboss_bevelstyle() {
    let mut app = App::new();
    let id = app.host_property("bevelEmboss/bevelStyle").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "bevelEmboss/bevelStyle");
}

#[test]
fn match_bevelemboss_beveltechnique() {
    let mut app = App::new();
    let id = app.host_property("bevelEmboss/bevelTechnique").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "bevelEmboss/bevelTechnique");
}

#[test]
fn match_bevelemboss_strengthratio() {
    let mut app = App::new();
    let id = app.host_property("bevelEmboss/strengthRatio").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "bevelEmboss/strengthRatio");
}

#[test]
fn match_bevelemboss_beveldirection() {
    let mut app = App::new();
    let id = app.host_property("bevelEmboss/bevelDirection").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "bevelEmboss/bevelDirection");
}

#[test]
fn match_bevelemboss_blur() {
    let mut app = App::new();
    let id = app.host_property("bevelEmboss/blur").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "bevelEmboss/blur");
}

#[test]
fn match_bevelemboss_softness() {
    let mut app = App::new();
    let id = app.host_property("bevelEmboss/softness").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "bevelEmboss/softness");
}

#[test]
fn match_bevelemboss_useglobalangle() {
    let mut app = App::new();
    let id = app.host_property("bevelEmboss/useGlobalAngle").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "bevelEmboss/useGlobalAngle");
}

#[test]
fn match_bevelemboss_locallightingangle() {
    let mut app = App::new();
    let id = app.host_property("bevelEmboss/localLightingAngle").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "bevelEmboss/localLightingAngle");
}

#[test]
fn match_bevelemboss_locallightingaltitude() {
    let mut app = App::new();
    let id = app.host_property("bevelEmboss/localLightingAltitude").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "bevelEmboss/localLightingAltitude");
}

#[test]
fn match_bevelemboss_highlightmode() {
    let mut app = App::new();
    let id = app.host_property("bevelEmboss/highlightMode").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "bevelEmboss/highlightMode");
}

#[test]
fn match_bevelemboss_highlightcolor() {
    let mut app = App::new();
    let id = app.host_property("bevelEmboss/highlightColor").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "bevelEmboss/highlightColor");
}

#[test]
fn match_bevelemboss_highlightopacity() {
    let mut app = App::new();
    let id = app.host_property("bevelEmboss/highlightOpacity").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "bevelEmboss/highlightOpacity");
}

#[test]
fn match_bevelemboss_shadowmode() {
    let mut app = App::new();
    let id = app.host_property("bevelEmboss/shadowMode").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "bevelEmboss/shadowMode");
}

#[test]
fn match_bevelemboss_shadowcolor() {
    let mut app = App::new();
    let id = app.host_property("bevelEmboss/shadowColor").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "bevelEmboss/shadowColor");
}

#[test]
fn match_bevelemboss_shadowopacity() {
    let mut app = App::new();
    let id = app.host_property("bevelEmboss/shadowOpacity").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "bevelEmboss/shadowOpacity");
}

#[test]
fn match_chromefx_enabled() {
    let mut app = App::new();
    let id = app.host_property("chromeFX/enabled").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "chromeFX/enabled");
}

#[test]
fn match_chromefx_mode2() {
    let mut app = App::new();
    let id = app.host_property("chromeFX/mode2").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "chromeFX/mode2");
}

#[test]
fn match_chromefx_color() {
    let mut app = App::new();
    let id = app.host_property("chromeFX/color").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "chromeFX/color");
}

#[test]
fn match_chromefx_opacity() {
    let mut app = App::new();
    let id = app.host_property("chromeFX/opacity").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "chromeFX/opacity");
}

#[test]
fn match_chromefx_locallightingangle() {
    let mut app = App::new();
    let id = app.host_property("chromeFX/localLightingAngle").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "chromeFX/localLightingAngle");
}

#[test]
fn match_chromefx_distance() {
    let mut app = App::new();
    let id = app.host_property("chromeFX/distance").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "chromeFX/distance");
}

#[test]
fn match_chromefx_blur() {
    let mut app = App::new();
    let id = app.host_property("chromeFX/blur").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "chromeFX/blur");
}

#[test]
fn match_chromefx_invert() {
    let mut app = App::new();
    let id = app.host_property("chromeFX/invert").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "chromeFX/invert");
}

#[test]
fn match_solidfill_enabled() {
    let mut app = App::new();
    let id = app.host_property("solidFill/enabled").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "solidFill/enabled");
}

#[test]
fn match_solidfill_mode2() {
    let mut app = App::new();
    let id = app.host_property("solidFill/mode2").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "solidFill/mode2");
}

#[test]
fn match_solidfill_color() {
    let mut app = App::new();
    let id = app.host_property("solidFill/color").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "solidFill/color");
}

#[test]
fn match_solidfill_opacity() {
    let mut app = App::new();
    let id = app.host_property("solidFill/opacity").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "solidFill/opacity");
}

#[test]
fn match_gradientfill_enabled() {
    let mut app = App::new();
    let id = app.host_property("gradientFill/enabled").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "gradientFill/enabled");
}

#[test]
fn match_gradientfill_mode2() {
    let mut app = App::new();
    let id = app.host_property("gradientFill/mode2").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "gradientFill/mode2");
}

#[test]
fn match_gradientfill_opacity() {
    let mut app = App::new();
    let id = app.host_property("gradientFill/opacity").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "gradientFill/opacity");
}

#[test]
fn match_gradientfill_gradient() {
    let mut app = App::new();
    let id = app.host_property("gradientFill/gradient").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "gradientFill/gradient");
}

#[test]
fn match_gradientfill_gradientsmoothness() {
    let mut app = App::new();
    let id = app.host_property("gradientFill/gradientSmoothness").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "gradientFill/gradientSmoothness");
}

#[test]
fn match_gradientfill_angle() {
    let mut app = App::new();
    let id = app.host_property("gradientFill/angle").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "gradientFill/angle");
}

#[test]
fn match_gradientfill_type() {
    let mut app = App::new();
    let id = app.host_property("gradientFill/type").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "gradientFill/type");
}

#[test]
fn match_gradientfill_reverse() {
    let mut app = App::new();
    let id = app.host_property("gradientFill/reverse").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "gradientFill/reverse");
}

#[test]
fn match_gradientfill_align() {
    let mut app = App::new();
    let id = app.host_property("gradientFill/align").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "gradientFill/align");
}

#[test]
fn match_gradientfill_scale() {
    let mut app = App::new();
    let id = app.host_property("gradientFill/scale").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "gradientFill/scale");
}

#[test]
fn match_gradientfill_offset() {
    let mut app = App::new();
    let id = app.host_property("gradientFill/offset").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "gradientFill/offset");
}

#[test]
fn match_patternfill_enabled() {
    let mut app = App::new();
    let id = app.host_property("patternFill/enabled").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "patternFill/enabled");
}

#[test]
fn match_patternfill_mode2() {
    let mut app = App::new();
    let id = app.host_property("patternFill/mode2").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "patternFill/mode2");
}

#[test]
fn match_patternfill_opacity() {
    let mut app = App::new();
    let id = app.host_property("patternFill/opacity").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "patternFill/opacity");
}

#[test]
fn match_patternfill_align() {
    let mut app = App::new();
    let id = app.host_property("patternFill/align").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "patternFill/align");
}

#[test]
fn match_patternfill_scale() {
    let mut app = App::new();
    let id = app.host_property("patternFill/scale").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "patternFill/scale");
}

#[test]
fn match_patternfill_phase() {
    let mut app = App::new();
    let id = app.host_property("patternFill/phase").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "patternFill/phase");
}

#[test]
fn match_framefx_enabled() {
    let mut app = App::new();
    let id = app.host_property("frameFX/enabled").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "frameFX/enabled");
}

#[test]
fn match_framefx_mode2() {
    let mut app = App::new();
    let id = app.host_property("frameFX/mode2").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "frameFX/mode2");
}

#[test]
fn match_framefx_color() {
    let mut app = App::new();
    let id = app.host_property("frameFX/color").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "frameFX/color");
}

#[test]
fn match_framefx_size() {
    let mut app = App::new();
    let id = app.host_property("frameFX/size").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "frameFX/size");
}

#[test]
fn match_framefx_opacity() {
    let mut app = App::new();
    let id = app.host_property("frameFX/opacity").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "frameFX/opacity");
}

#[test]
fn match_framefx_style() {
    let mut app = App::new();
    let id = app.host_property("frameFX/style").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "frameFX/style");
}

#[test]
fn match_adbe_light_layer() {
    let mut app = App::new();
    let id = app.host_property("ADBE Light Layer").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Light Layer");
}

#[test]
fn match_adbe_light_options_group() {
    let mut app = App::new();
    let id = app.host_property("ADBE Light Options Group").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Light Options Group");
}

#[test]
fn match_adbe_light_intensity() {
    let mut app = App::new();
    let id = app.host_property("ADBE Light Intensity").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Light Intensity");
}

#[test]
fn match_adbe_light_color() {
    let mut app = App::new();
    let id = app.host_property("ADBE Light Color").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Light Color");
}

#[test]
fn match_adbe_light_cone_angle() {
    let mut app = App::new();
    let id = app.host_property("ADBE Light Cone Angle").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Light Cone Angle");
}

#[test]
fn match_adbe_light_cone_feather_2() {
    let mut app = App::new();
    let id = app.host_property("ADBE Light Cone Feather 2").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Light Cone Feather 2");
}

#[test]
fn match_adbe_light_falloff_type() {
    let mut app = App::new();
    let id = app.host_property("ADBE Light Falloff Type").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Light Falloff Type");
}

#[test]
fn match_adbe_light_falloff_start() {
    let mut app = App::new();
    let id = app.host_property("ADBE Light Falloff Start").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Light Falloff Start");
}

#[test]
fn match_adbe_light_falloff_distance() {
    let mut app = App::new();
    let id = app.host_property("ADBE Light Falloff Distance").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Light Falloff Distance");
}

#[test]
fn match_adbe_light_shadow_darkness() {
    let mut app = App::new();
    let id = app.host_property("ADBE Light Shadow Darkness").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Light Shadow Darkness");
}

#[test]
fn match_adbe_light_shadow_diffusion() {
    let mut app = App::new();
    let id = app.host_property("ADBE Light Shadow Diffusion").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Light Shadow Diffusion");
}

#[test]
fn match_adbe_vector_layer() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Layer").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Layer");
}

#[test]
fn match_adbe_root_vectors_group() {
    let mut app = App::new();
    let id = app.host_property("ADBE Root Vectors Group").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Root Vectors Group");
}

#[test]
fn match_adbe_vector_group() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Group").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Group");
}

#[test]
fn match_adbe_vector_blend_mode() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Blend Mode").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Blend Mode");
}

#[test]
fn match_adbe_vectors_group() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vectors Group").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vectors Group");
}

#[test]
fn match_adbe_vector_transform_group() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Transform Group").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Transform Group");
}

#[test]
fn match_adbe_vector_materials_group() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Materials Group").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Materials Group");
}

#[test]
fn match_adbe_vector_anchor() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Anchor").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Anchor");
}

#[test]
fn match_adbe_vector_position() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Position").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Position");
}

#[test]
fn match_adbe_vector_scale() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Scale").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Scale");
}

#[test]
fn match_adbe_vector_skew() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Skew").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Skew");
}

#[test]
fn match_adbe_vector_skew_axis() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Skew Axis").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Skew Axis");
}

#[test]
fn match_adbe_vector_rotation() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Rotation").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Rotation");
}

#[test]
fn match_adbe_vector_group_opacity() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Group Opacity").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Group Opacity");
}

#[test]
fn match_adbe_vector_shape_rect() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Shape - Rect").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Shape - Rect");
}

#[test]
fn match_adbe_vector_shape_direction() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Shape Direction").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Shape Direction");
}

#[test]
fn match_adbe_vector_rect_size() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Rect Size").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Rect Size");
}

#[test]
fn match_adbe_vector_rect_position() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Rect Position").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Rect Position");
}

#[test]
fn match_adbe_vector_rect_roundness() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Rect Roundness").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Rect Roundness");
}

#[test]
fn match_adbe_vector_shape_ellipse() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Shape - Ellipse").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Shape - Ellipse");
}

#[test]
fn match_adbe_vector_ellipse_size() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Ellipse Size").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Ellipse Size");
}

#[test]
fn match_adbe_vector_ellipse_position() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Ellipse Position").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Ellipse Position");
}

#[test]
fn match_adbe_vector_shape_star() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Shape - Star").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Shape - Star");
}

#[test]
fn match_adbe_vector_star_type() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Star Type").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Star Type");
}

#[test]
fn match_adbe_vector_star_points() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Star Points").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Star Points");
}

#[test]
fn match_adbe_vector_star_position() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Star Position").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Star Position");
}

#[test]
fn match_adbe_vector_star_rotation() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Star Rotation").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Star Rotation");
}

#[test]
fn match_adbe_vector_star_inner_radius() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Star Inner Radius").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Star Inner Radius");
}

#[test]
fn match_adbe_vector_star_outer_radius() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Star Outer Radius").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Star Outer Radius");
}

#[test]
fn match_adbe_vector_star_inner_roundess() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Star Inner Roundess").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Star Inner Roundess");
}

#[test]
fn match_adbe_vector_star_outer_roundess() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Star Outer Roundess").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Star Outer Roundess");
}

#[test]
fn match_adbe_vector_shape_group() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Shape - Group").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Shape - Group");
}

#[test]
fn match_adbe_vector_shape() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Shape").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Shape");
}

#[test]
fn match_adbe_vector_graphic_fill() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Graphic - Fill").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Graphic - Fill");
}

#[test]
fn match_adbe_vector_composite_order() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Composite Order").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Composite Order");
}

#[test]
fn match_adbe_vector_fill_rule() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Fill Rule").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Fill Rule");
}

#[test]
fn match_adbe_vector_fill_color() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Fill Color").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Fill Color");
}

#[test]
fn match_adbe_vector_fill_opacity() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Fill Opacity").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Fill Opacity");
}

#[test]
fn match_adbe_vector_graphic_stroke() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Graphic - Stroke").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Graphic - Stroke");
}

#[test]
fn match_adbe_vector_stroke_color() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Stroke Color").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Stroke Color");
}

#[test]
fn match_adbe_vector_stroke_opacity() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Stroke Opacity").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Stroke Opacity");
}

#[test]
fn match_adbe_vector_stroke_width() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Stroke Width").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Stroke Width");
}

#[test]
fn match_adbe_vector_stroke_line_cap() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Stroke Line Cap").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Stroke Line Cap");
}

#[test]
fn match_adbe_vector_stroke_line_join() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Stroke Line Join").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Stroke Line Join");
}

#[test]
fn match_adbe_vector_stroke_miter_limit() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Stroke Miter Limit").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Stroke Miter Limit");
}

#[test]
fn match_adbe_vector_stroke_dashes() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Stroke Dashes").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Stroke Dashes");
}

#[test]
fn match_adbe_vector_stroke_dash_1() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Stroke Dash 1").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Stroke Dash 1");
}

#[test]
fn match_adbe_vector_stroke_gap_1() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Stroke Gap 1").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Stroke Gap 1");
}

#[test]
fn match_adbe_vector_stroke_dash_2() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Stroke Dash 2").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Stroke Dash 2");
}

#[test]
fn match_adbe_vector_stroke_gap_2() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Stroke Gap 2").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Stroke Gap 2");
}

#[test]
fn match_adbe_vector_stroke_dash_3() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Stroke Dash 3").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Stroke Dash 3");
}

#[test]
fn match_adbe_vector_stroke_gap_3() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Stroke Gap 3").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Stroke Gap 3");
}

#[test]
fn match_adbe_vector_stroke_offset() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Stroke Offset").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Stroke Offset");
}

#[test]
fn match_adbe_vector_stroke_taper() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Stroke Taper").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Stroke Taper");
}

#[test]
fn match_adbe_vector_taper_start_width() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Taper Start Width").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Taper Start Width");
}

#[test]
fn match_adbe_vector_taper_length_units() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Taper Length Units").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Taper Length Units");
}

#[test]
fn match_adbe_vector_taper_end_width() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Taper End Width").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Taper End Width");
}

#[test]
fn match_adbe_vector_taper_end_ease() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Taper End Ease").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Taper End Ease");
}

#[test]
fn match_adbe_vector_taper_end_length() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Taper End Length").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Taper End Length");
}

#[test]
fn match_adbe_vector_taper_start_length() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Taper Start Length").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Taper Start Length");
}

#[test]
fn match_adbe_vector_taper_start_ease() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Taper Start Ease").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Taper Start Ease");
}

#[test]
fn match_adbe_vector_stroke_wave() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Stroke Wave").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Stroke Wave");
}

#[test]
fn match_adbe_vector_taper_wave_amount() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Taper Wave Amount").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Taper Wave Amount");
}

#[test]
fn match_adbe_vector_taper_wave_units() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Taper Wave Units").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Taper Wave Units");
}

#[test]
fn match_adbe_vector_taper_wave_phase() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Taper Wave Phase").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Taper Wave Phase");
}

#[test]
fn match_adbe_vector_taper_wavelength() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Taper Wavelength").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Taper Wavelength");
}

#[test]
fn match_adbe_vector_graphic_g_fill() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Graphic - G-Fill").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Graphic - G-Fill");
}

#[test]
fn match_adbe_vector_grad_type() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Grad Type").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Grad Type");
}

#[test]
fn match_adbe_vector_grad_start_pt() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Grad Start Pt").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Grad Start Pt");
}

#[test]
fn match_adbe_vector_grad_end_pt() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Grad End Pt").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Grad End Pt");
}

#[test]
fn match_adbe_vector_grad_hilite_length() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Grad HiLite Length").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Grad HiLite Length");
}

#[test]
fn match_adbe_vector_grad_hilite_angle() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Grad HiLite Angle").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Grad HiLite Angle");
}

#[test]
fn match_adbe_vector_grad_colors() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Grad Colors").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Grad Colors");
}

#[test]
fn match_adbe_vector_graphic_g_stroke() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Graphic - G-Stroke").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Graphic - G-Stroke");
}

#[test]
fn match_adbe_vector_filter_merge() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Filter - Merge").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Filter - Merge");
}

#[test]
fn match_adbe_vector_merge_type() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Merge Type").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Merge Type");
}

#[test]
fn match_adbe_vector_filter_offset() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Filter - Offset").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Filter - Offset");
}

#[test]
fn match_adbe_vector_offset_amount() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Offset Amount").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Offset Amount");
}

#[test]
fn match_adbe_vector_offset_line_join() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Offset Line Join").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Offset Line Join");
}

#[test]
fn match_adbe_vector_offset_miter_limit() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Offset Miter Limit").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Offset Miter Limit");
}

#[test]
fn match_adbe_vector_offset_copies() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Offset Copies").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Offset Copies");
}

#[test]
fn match_adbe_vector_offset_copy_offset() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Offset Copy Offset").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Offset Copy Offset");
}

#[test]
fn match_adbe_vector_filter_pb() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Filter - PB").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Filter - PB");
}

#[test]
fn match_adbe_vector_puckerbloat_amount() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector PuckerBloat Amount").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector PuckerBloat Amount");
}

#[test]
fn match_adbe_vector_filter_repeater() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Filter - Repeater").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Filter - Repeater");
}

#[test]
fn match_adbe_vector_repeater_copies() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Repeater Copies").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Repeater Copies");
}

#[test]
fn match_adbe_vector_repeater_offset() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Repeater Offset").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Repeater Offset");
}

#[test]
fn match_adbe_vector_repeater_order() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Repeater Order").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Repeater Order");
}

#[test]
fn match_adbe_vector_repeater_transform() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Repeater Transform").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Repeater Transform");
}

#[test]
fn match_adbe_vector_filter_rc() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Filter - RC").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Filter - RC");
}

#[test]
fn match_adbe_vector_roundcorner_radius() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector RoundCorner Radius").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector RoundCorner Radius");
}

#[test]
fn match_adbe_vector_filter_trim() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Filter - Trim").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Filter - Trim");
}

#[test]
fn match_adbe_vector_trim_start() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Trim Start").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Trim Start");
}

#[test]
fn match_adbe_vector_trim_end() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Trim End").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Trim End");
}

#[test]
fn match_adbe_vector_trim_offset() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Trim Offset").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Trim Offset");
}

#[test]
fn match_adbe_vector_trim_type() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Trim Type").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Trim Type");
}

#[test]
fn match_adbe_vector_filter_twist() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Filter - Twist").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Filter - Twist");
}

#[test]
fn match_adbe_vector_twist_angle() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Twist Angle").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Twist Angle");
}

#[test]
fn match_adbe_vector_twist_center() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Twist Center").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Twist Center");
}

#[test]
fn match_adbe_vector_filter_roughen() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Filter - Roughen").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Filter - Roughen");
}

#[test]
fn match_adbe_vector_roughen_size() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Roughen Size").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Roughen Size");
}

#[test]
fn match_adbe_vector_roughen_detail() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Roughen Detail").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Roughen Detail");
}

#[test]
fn match_adbe_vector_roughen_points() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Roughen Points").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Roughen Points");
}

#[test]
fn match_adbe_vector_temporal_freq() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Temporal Freq").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Temporal Freq");
}

#[test]
fn match_adbe_vector_correlation() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Correlation").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Correlation");
}

#[test]
fn match_adbe_vector_temporal_phase() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Temporal Phase").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Temporal Phase");
}

#[test]
fn match_adbe_vector_spatial_phase() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Spatial Phase").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Spatial Phase");
}

#[test]
fn match_adbe_vector_random_seed() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Random Seed").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Random Seed");
}

#[test]
fn match_adbe_vector_filter_wiggler() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Filter - Wiggler").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Filter - Wiggler");
}

#[test]
fn match_adbe_vector_xform_temporal_freq() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Xform Temporal Freq").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Xform Temporal Freq");
}

#[test]
fn match_adbe_vector_wiggler_transform() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Wiggler Transform").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Wiggler Transform");
}

#[test]
fn match_adbe_vector_filter_zigzag() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Filter - Zigzag").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Filter - Zigzag");
}

#[test]
fn match_adbe_vector_zigzag_size() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Zigzag Size").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Zigzag Size");
}

#[test]
fn match_adbe_vector_zigzag_detail() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Zigzag Detail").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Zigzag Detail");
}

#[test]
fn match_adbe_vector_zigzag_points() {
    let mut app = App::new();
    let id = app.host_property("ADBE Vector Zigzag Points").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Vector Zigzag Points");
}

#[test]
fn match_adbe_text_layer() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Layer").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Layer");
}

#[test]
fn match_adbe_text_properties() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Properties").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Properties");
}

#[test]
fn match_adbe_text_document() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Document").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Document");
}

#[test]
fn match_adbe_text_path_options() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Path Options").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Path Options");
}

#[test]
fn match_adbe_text_reverse_path() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Reverse Path").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Reverse Path");
}

#[test]
fn match_adbe_text_perpendicular_to_path() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Perpendicular To Path").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Perpendicular To Path");
}

#[test]
fn match_adbe_text_force_align_path() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Force Align Path").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Force Align Path");
}

#[test]
fn match_adbe_text_first_margin() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text First Margin").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text First Margin");
}

#[test]
fn match_adbe_text_last_margin() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Last Margin").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Last Margin");
}

#[test]
fn match_adbe_text_more_options() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text More Options").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text More Options");
}

#[test]
fn match_adbe_text_variable_font_spacing() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Variable Font Spacing").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Variable Font Spacing");
}

#[test]
fn match_adbe_text_anchor_point_align() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Anchor Point Align").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Anchor Point Align");
}

#[test]
fn match_adbe_text_animators() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Animators").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Animators");
}

#[test]
fn match_adbe_text_animator() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Animator").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Animator");
}

#[test]
fn match_adbe_text_selectors() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Selectors").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Selectors");
}

#[test]
fn match_adbe_text_selector() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Selector").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Selector");
}

#[test]
fn match_adbe_text_percent_start() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Percent Start").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Percent Start");
}

#[test]
fn match_adbe_text_percent_end() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Percent End").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Percent End");
}

#[test]
fn match_adbe_text_percent_offset() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Percent Offset").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Percent Offset");
}

#[test]
fn match_adbe_text_index_start() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Index Start").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Index Start");
}

#[test]
fn match_adbe_text_index_end() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Index End").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Index End");
}

#[test]
fn match_adbe_text_index_offset() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Index Offset").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Index Offset");
}

#[test]
fn match_adbe_text_range_advanced() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Range Advanced").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Range Advanced");
}

#[test]
fn match_adbe_text_range_units() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Range Units").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Range Units");
}

#[test]
fn match_adbe_text_selector_mode() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Selector Mode").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Selector Mode");
}

#[test]
fn match_adbe_text_selector_max_amount() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Selector Max Amount").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Selector Max Amount");
}

#[test]
fn match_adbe_text_selector_smoothness() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Selector Smoothness").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Selector Smoothness");
}

#[test]
fn match_adbe_text_levels_max_ease() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Levels Max Ease").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Levels Max Ease");
}

#[test]
fn match_adbe_text_levels_min_ease() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Levels Min Ease").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Levels Min Ease");
}

#[test]
fn match_adbe_text_random_seed() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Random Seed").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Random Seed");
}

#[test]
fn match_adbe_text_animator_properties() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Animator Properties").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Animator Properties");
}

#[test]
fn match_adbe_text_anchor_point_3d() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Anchor Point 3D").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Anchor Point 3D");
}

#[test]
fn match_adbe_text_position_3d() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Position 3D").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Position 3D");
}

#[test]
fn match_adbe_text_scale_3d() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Scale 3D").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Scale 3D");
}

#[test]
fn match_adbe_text_skew() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Skew").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Skew");
}

#[test]
fn match_adbe_text_skew_axis() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Skew Axis").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Skew Axis");
}

#[test]
fn match_adbe_text_rotation_x() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Rotation X").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Rotation X");
}

#[test]
fn match_adbe_text_rotation_y() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Rotation Y").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Rotation Y");
}

#[test]
fn match_adbe_text_rotation() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Rotation").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Rotation");
}

#[test]
fn match_adbe_text_opacity() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Opacity").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Opacity");
}

#[test]
fn match_adbe_text_fill_opacity() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Fill Opacity").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Fill Opacity");
}

#[test]
fn match_adbe_text_stroke_opacity() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Stroke Opacity").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Stroke Opacity");
}

#[test]
fn match_adbe_text_fill_color() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Fill Color").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Fill Color");
}

#[test]
fn match_adbe_text_stroke_color() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Stroke Color").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Stroke Color");
}

#[test]
fn match_adbe_text_fill_hue() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Fill Hue").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Fill Hue");
}

#[test]
fn match_adbe_text_stroke_hue() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Stroke Hue").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Stroke Hue");
}

#[test]
fn match_adbe_text_fill_saturation() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Fill Saturation").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Fill Saturation");
}

#[test]
fn match_adbe_text_stroke_saturation() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Stroke Saturation").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Stroke Saturation");
}

#[test]
fn match_adbe_text_fill_brightness() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Fill Brightness").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Fill Brightness");
}

#[test]
fn match_adbe_text_stroke_brightness() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Stroke Brightness").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Stroke Brightness");
}

#[test]
fn match_adbe_text_stroke_width() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Stroke Width").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Stroke Width");
}

#[test]
fn match_adbe_text_line_anchor() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Line Anchor").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Line Anchor");
}

#[test]
fn match_adbe_text_track_type() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Track Type").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Track Type");
}

#[test]
fn match_adbe_text_tracking_amount() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Tracking Amount").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Tracking Amount");
}

#[test]
fn match_adbe_text_character_replace() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Character Replace").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Character Replace");
}

#[test]
fn match_adbe_text_character_offset() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Character Offset").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Character Offset");
}

#[test]
fn match_adbe_text_line_spacing() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Line Spacing").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Line Spacing");
}

#[test]
fn match_adbe_text_blur() {
    let mut app = App::new();
    let id = app.host_property("ADBE Text Blur").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE Text Blur");
}

#[test]
fn match_adbe_3dtext_front_rgb() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Front RGB").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Front RGB");
}

#[test]
fn match_adbe_3dtext_front_hue() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Front Hue").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Front Hue");
}

#[test]
fn match_adbe_3dtext_front_sat() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Front Sat").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Front Sat");
}

#[test]
fn match_adbe_3dtext_front_bright() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Front Bright").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Front Bright");
}

#[test]
fn match_adbe_3dtext_front_opacity() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Front Opacity").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Front Opacity");
}

#[test]
fn match_adbe_3dtext_front_ambient() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Front Ambient").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Front Ambient");
}

#[test]
fn match_adbe_3dtext_front_diffuse() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Front Diffuse").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Front Diffuse");
}

#[test]
fn match_adbe_3dtext_front_specular() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Front Specular").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Front Specular");
}

#[test]
fn match_adbe_3dtext_front_shininess() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Front Shininess").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Front Shininess");
}

#[test]
fn match_adbe_3dtext_front_metal() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Front Metal").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Front Metal");
}

#[test]
fn match_adbe_3dtext_front_reflection() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Front Reflection").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Front Reflection");
}

#[test]
fn match_adbe_3dtext_front_gloss() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Front Gloss").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Front Gloss");
}

#[test]
fn match_adbe_3dtext_front_fresnel() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Front Fresnel").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Front Fresnel");
}

#[test]
fn match_adbe_3dtext_front_xparency() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Front Xparency").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Front Xparency");
}

#[test]
fn match_adbe_3dtext_front_xparroll() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Front XparRoll").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Front XparRoll");
}

#[test]
fn match_adbe_3dtext_front_ior() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Front IOR").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Front IOR");
}

#[test]
fn match_adbe_3dtext_bevel_rgb() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Bevel RGB").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Bevel RGB");
}

#[test]
fn match_adbe_3dtext_bevel_hue() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Bevel Hue").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Bevel Hue");
}

#[test]
fn match_adbe_3dtext_bevel_sat() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Bevel Sat").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Bevel Sat");
}

#[test]
fn match_adbe_3dtext_bevel_bright() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Bevel Bright").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Bevel Bright");
}

#[test]
fn match_adbe_3dtext_bevel_opacity() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Bevel Opacity").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Bevel Opacity");
}

#[test]
fn match_adbe_3dtext_bevel_ambient() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Bevel Ambient").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Bevel Ambient");
}

#[test]
fn match_adbe_3dtext_bevel_diffuse() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Bevel Diffuse").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Bevel Diffuse");
}

#[test]
fn match_adbe_3dtext_bevel_specular() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Bevel Specular").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Bevel Specular");
}

#[test]
fn match_adbe_3dtext_bevel_shininess() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Bevel Shininess").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Bevel Shininess");
}

#[test]
fn match_adbe_3dtext_bevel_metal() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Bevel Metal").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Bevel Metal");
}

#[test]
fn match_adbe_3dtext_bevel_reflection() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Bevel Reflection").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Bevel Reflection");
}

#[test]
fn match_adbe_3dtext_bevel_gloss() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Bevel Gloss").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Bevel Gloss");
}

#[test]
fn match_adbe_3dtext_bevel_fresnel() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Bevel Fresnel").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Bevel Fresnel");
}

#[test]
fn match_adbe_3dtext_bevel_xparency() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Bevel Xparency").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Bevel Xparency");
}

#[test]
fn match_adbe_3dtext_bevel_xparroll() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Bevel XparRoll").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Bevel XparRoll");
}

#[test]
fn match_adbe_3dtext_bevel_ior() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Bevel IOR").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Bevel IOR");
}

#[test]
fn match_adbe_3dtext_side_rgb() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Side RGB").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Side RGB");
}

#[test]
fn match_adbe_3dtext_side_hue() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Side Hue").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Side Hue");
}

#[test]
fn match_adbe_3dtext_side_sat() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Side Sat").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Side Sat");
}

#[test]
fn match_adbe_3dtext_side_bright() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Side Bright").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Side Bright");
}

#[test]
fn match_adbe_3dtext_side_opacity() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Side Opacity").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Side Opacity");
}

#[test]
fn match_adbe_3dtext_side_ambient() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Side Ambient").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Side Ambient");
}

#[test]
fn match_adbe_3dtext_side_diffuse() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Side Diffuse").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Side Diffuse");
}

#[test]
fn match_adbe_3dtext_side_specular() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Side Specular").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Side Specular");
}

#[test]
fn match_adbe_3dtext_side_shininess() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Side Shininess").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Side Shininess");
}

#[test]
fn match_adbe_3dtext_side_metal() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Side Metal").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Side Metal");
}

#[test]
fn match_adbe_3dtext_side_reflection() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Side Reflection").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Side Reflection");
}

#[test]
fn match_adbe_3dtext_side_gloss() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Side Gloss").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Side Gloss");
}

#[test]
fn match_adbe_3dtext_side_fresnel() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Side Fresnel").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Side Fresnel");
}

#[test]
fn match_adbe_3dtext_side_xparency() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Side Xparency").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Side Xparency");
}

#[test]
fn match_adbe_3dtext_side_xparroll() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Side XparRoll").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Side XparRoll");
}

#[test]
fn match_adbe_3dtext_side_ior() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Side IOR").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Side IOR");
}

#[test]
fn match_adbe_3dtext_back_rgb() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Back RGB").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Back RGB");
}

#[test]
fn match_adbe_3dtext_back_hue() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Back Hue").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Back Hue");
}

#[test]
fn match_adbe_3dtext_back_sat() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Back Sat").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Back Sat");
}

#[test]
fn match_adbe_3dtext_back_bright() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Back Bright").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Back Bright");
}

#[test]
fn match_adbe_3dtext_back_opacity() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Back Opacity").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Back Opacity");
}

#[test]
fn match_adbe_3dtext_back_ambient() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Back Ambient").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Back Ambient");
}

#[test]
fn match_adbe_3dtext_back_diffuse() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Back Diffuse").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Back Diffuse");
}

#[test]
fn match_adbe_3dtext_back_specular() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Back Specular").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Back Specular");
}

#[test]
fn match_adbe_3dtext_back_shininess() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Back Shininess").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Back Shininess");
}

#[test]
fn match_adbe_3dtext_back_metal() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Back Metal").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Back Metal");
}

#[test]
fn match_adbe_3dtext_back_reflection() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Back Reflection").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Back Reflection");
}

#[test]
fn match_adbe_3dtext_back_gloss() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Back Gloss").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Back Gloss");
}

#[test]
fn match_adbe_3dtext_back_fresnel() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Back Fresnel").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Back Fresnel");
}

#[test]
fn match_adbe_3dtext_back_xparency() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Back Xparency").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Back Xparency");
}

#[test]
fn match_adbe_3dtext_back_xparroll() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Back XparRoll").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Back XparRoll");
}

#[test]
fn match_adbe_3dtext_back_ior() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Back IOR").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Back IOR");
}

#[test]
fn match_adbe_3dtext_bevel_depth() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Bevel Depth").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Bevel Depth");
}

#[test]
fn match_adbe_3dtext_extrude_depth() {
    let mut app = App::new();
    let id = app.host_property("ADBE 3DText Extrude Depth").unwrap();
    assert_eq!(app.match_name(id).unwrap(), "ADBE 3DText Extrude Depth");
}

#[test]
fn alias_mask_parade() {
    let mut app = App::new();
    let comp = app.add_comp("c", 200, 100, 1.0, 2.0, 25.0).unwrap();
    let layer = app.add_solid_layer(comp, [1.0, 0.0, 0.0], "red", 100, 80, 1.0, None).unwrap();
    let a = app.property_by_name(layer, "ADBE Mask Parade").unwrap();
    let b = app.property_by_name(layer, "Masks").unwrap();
    assert_eq!(a, b);
}

#[test]
fn alias_effect_parade() {
    let mut app = App::new();
    let comp = app.add_comp("c", 200, 100, 1.0, 2.0, 25.0).unwrap();
    let layer = app.add_solid_layer(comp, [1.0, 0.0, 0.0], "red", 100, 80, 1.0, None).unwrap();
    let a = app.property_by_name(layer, "ADBE Effect Parade").unwrap();
    let b = app.property_by_name(layer, "Effects").unwrap();
    assert_eq!(a, b);
}

#[test]
fn alias_mtrackers() {
    let mut app = App::new();
    let comp = app.add_comp("c", 200, 100, 1.0, 2.0, 25.0).unwrap();
    let layer = app.add_solid_layer(comp, [1.0, 0.0, 0.0], "red", 100, 80, 1.0, None).unwrap();
    let a = app.property_by_name(layer, "ADBE MTrackers").unwrap();
    let b = app.property_by_name(layer, "Motion Trackers").unwrap();
    assert_eq!(a, b);
}

#[test]
fn alias_anchor() {
    let mut app = App::new();
    let comp = app.add_comp("c", 200, 100, 1.0, 2.0, 25.0).unwrap();
    let layer = app.add_solid_layer(comp, [1.0, 0.0, 0.0], "red", 100, 80, 1.0, None).unwrap();
    let a = app.property_by_name(layer, "Anchor Point").unwrap();
    let b = app.property_by_name(layer, "anchorPoint").unwrap();
    assert_eq!(a, b);
}

#[test]
fn alias_position() {
    let mut app = App::new();
    let comp = app.add_comp("c", 200, 100, 1.0, 2.0, 25.0).unwrap();
    let layer = app.add_solid_layer(comp, [1.0, 0.0, 0.0], "red", 100, 80, 1.0, None).unwrap();
    let a = app.property_by_name(layer, "Position").unwrap();
    let b = app.property_by_name(layer, "position").unwrap();
    assert_eq!(a, b);
}

#[test]
fn alias_scale() {
    let mut app = App::new();
    let comp = app.add_comp("c", 200, 100, 1.0, 2.0, 25.0).unwrap();
    let layer = app.add_solid_layer(comp, [1.0, 0.0, 0.0], "red", 100, 80, 1.0, None).unwrap();
    let a = app.property_by_name(layer, "Scale").unwrap();
    let b = app.property_by_name(layer, "scale").unwrap();
    assert_eq!(a, b);
}

#[test]
fn alias_rotation() {
    let mut app = App::new();
    let comp = app.add_comp("c", 200, 100, 1.0, 2.0, 25.0).unwrap();
    let layer = app.add_solid_layer(comp, [1.0, 0.0, 0.0], "red", 100, 80, 1.0, None).unwrap();
    let a = app.property_by_name(layer, "Rotation").unwrap();
    let b = app.property_by_name(layer, "rotation").unwrap();
    assert_eq!(a, b);
}

#[test]
fn alias_z_rotation() {
    let mut app = App::new();
    let comp = app.add_comp("c", 200, 100, 1.0, 2.0, 25.0).unwrap();
    let layer = app.add_solid_layer(comp, [1.0, 0.0, 0.0], "red", 100, 80, 1.0, None).unwrap();
    let a = app.property_by_name(layer, "Z Rotation").unwrap();
    let b = app.property_by_name(layer, "rotationZ").unwrap();
    assert_eq!(a, b);
}

#[test]
fn alias_opacity() {
    let mut app = App::new();
    let comp = app.add_comp("c", 200, 100, 1.0, 2.0, 25.0).unwrap();
    let layer = app.add_solid_layer(comp, [1.0, 0.0, 0.0], "red", 100, 80, 1.0, None).unwrap();
    let a = app.property_by_name(layer, "Opacity").unwrap();
    let b = app.property_by_name(layer, "opacity").unwrap();
    assert_eq!(a, b);
}

#[test]
fn alias_marker() {
    let mut app = App::new();
    let comp = app.add_comp("c", 200, 100, 1.0, 2.0, 25.0).unwrap();
    let layer = app.add_solid_layer(comp, [1.0, 0.0, 0.0], "red", 100, 80, 1.0, None).unwrap();
    let a = app.property_by_name(layer, "Marker").unwrap();
    let b = app.property_by_name(layer, "marker").unwrap();
    assert_eq!(a, b);
}

#[test]
fn alias_time_remap() {
    let mut app = App::new();
    let comp = app.add_comp("c", 200, 100, 1.0, 2.0, 25.0).unwrap();
    let src = app.add_video_footage("mov", 100, 80, 2.0, 25.0, false).unwrap();
    let layer = app.add_layer_from_item(comp, src, None).unwrap();
    let a = app.property_by_name(layer, "Time Remap").unwrap();
    let b = app.property_by_name(layer, "timeRemapEnabled").unwrap();
    assert_eq!(a, b);
}

#[test]
fn alias_audio() {
    let mut app = App::new();
    let comp = app.add_comp("c", 200, 100, 1.0, 2.0, 25.0).unwrap();
    let src = app.add_audio_footage("snd", 2.0, 25.0).unwrap();
    let layer = app.add_layer_from_item(comp, src, None).unwrap();
    let a = app.property_by_name(layer, "Audio Levels").unwrap();
    let b = app.property_by_name(layer, "audioLevels").unwrap();
    assert_eq!(a, b);
}

#[test]
fn alias_zoom() {
    let mut app = App::new();
    let comp = app.add_comp("c", 200, 100, 1.0, 2.0, 25.0).unwrap();
    let layer = app.add_camera(comp, "cam", [10.0, 20.0]).unwrap();
    let a = app.property_by_name(layer, "Zoom").unwrap();
    let b = app.property_by_name(layer, "zoom").unwrap();
    assert_eq!(a, b);
}

#[test]
fn alias_dof() {
    let mut app = App::new();
    let comp = app.add_comp("c", 200, 100, 1.0, 2.0, 25.0).unwrap();
    let layer = app.add_camera(comp, "cam", [10.0, 20.0]).unwrap();
    let a = app.property_by_name(layer, "Depth of Field").unwrap();
    let b = app.property_by_name(layer, "depthOfField").unwrap();
    assert_eq!(a, b);
}

#[test]
fn alias_focus() {
    let mut app = App::new();
    let comp = app.add_comp("c", 200, 100, 1.0, 2.0, 25.0).unwrap();
    let layer = app.add_camera(comp, "cam", [10.0, 20.0]).unwrap();
    let a = app.property_by_name(layer, "Focus Distance").unwrap();
    let b = app.property_by_name(layer, "focusDistance").unwrap();
    assert_eq!(a, b);
}

#[test]
fn alias_aperture() {
    let mut app = App::new();
    let comp = app.add_comp("c", 200, 100, 1.0, 2.0, 25.0).unwrap();
    let layer = app.add_camera(comp, "cam", [10.0, 20.0]).unwrap();
    let a = app.property_by_name(layer, "Aperture").unwrap();
    let b = app.property_by_name(layer, "aperture").unwrap();
    assert_eq!(a, b);
}

#[test]
fn alias_blur() {
    let mut app = App::new();
    let comp = app.add_comp("c", 200, 100, 1.0, 2.0, 25.0).unwrap();
    let layer = app.add_camera(comp, "cam", [10.0, 20.0]).unwrap();
    let a = app.property_by_name(layer, "Blur Level").unwrap();
    let b = app.property_by_name(layer, "blurLevel").unwrap();
    assert_eq!(a, b);
}

#[test]
fn alias_intensity() {
    let mut app = App::new();
    let comp = app.add_comp("c", 200, 100, 1.0, 2.0, 25.0).unwrap();
    let layer = app.add_light(comp, "lit", [10.0, 20.0]).unwrap();
    let a = app.property_by_name(layer, "Intensity").unwrap();
    let b = app.property_by_name(layer, "intensity").unwrap();
    assert_eq!(a, b);
}

#[test]
fn alias_color() {
    let mut app = App::new();
    let comp = app.add_comp("c", 200, 100, 1.0, 2.0, 25.0).unwrap();
    let layer = app.add_light(comp, "lit", [10.0, 20.0]).unwrap();
    let a = app.property_by_name(layer, "Color").unwrap();
    let b = app.property_by_name(layer, "color").unwrap();
    assert_eq!(a, b);
}

#[test]
fn alias_cone_angle() {
    let mut app = App::new();
    let comp = app.add_comp("c", 200, 100, 1.0, 2.0, 25.0).unwrap();
    let layer = app.add_light(comp, "lit", [10.0, 20.0]).unwrap();
    let a = app.property_by_name(layer, "Cone Angle").unwrap();
    let b = app.property_by_name(layer, "coneAngle").unwrap();
    assert_eq!(a, b);
}

#[test]
fn alias_cone_feather() {
    let mut app = App::new();
    let comp = app.add_comp("c", 200, 100, 1.0, 2.0, 25.0).unwrap();
    let layer = app.add_light(comp, "lit", [10.0, 20.0]).unwrap();
    let a = app.property_by_name(layer, "Cone Feather").unwrap();
    let b = app.property_by_name(layer, "coneFeather").unwrap();
    assert_eq!(a, b);
}

#[test]
fn alias_shadow_darkness() {
    let mut app = App::new();
    let comp = app.add_comp("c", 200, 100, 1.0, 2.0, 25.0).unwrap();
    let layer = app.add_light(comp, "lit", [10.0, 20.0]).unwrap();
    let a = app.property_by_name(layer, "Shadow Darkness").unwrap();
    let b = app.property_by_name(layer, "shadowDarkness").unwrap();
    assert_eq!(a, b);
}

#[test]
fn alias_shadow_diffusion() {
    let mut app = App::new();
    let comp = app.add_comp("c", 200, 100, 1.0, 2.0, 25.0).unwrap();
    let layer = app.add_light(comp, "lit", [10.0, 20.0]).unwrap();
    let a = app.property_by_name(layer, "Shadow Diffusion").unwrap();
    let b = app.property_by_name(layer, "shadowDiffusion").unwrap();
    assert_eq!(a, b);
}

#[test]
fn alias_light_casts() {
    let mut app = App::new();
    let comp = app.add_comp("c", 200, 100, 1.0, 2.0, 25.0).unwrap();
    let layer = app.add_light(comp, "lit", [10.0, 20.0]).unwrap();
    let a = app.property_by_name(layer, "Casts Shadows").unwrap();
    let b = app.property_by_name(layer, "castsShadows").unwrap();
    assert_eq!(a, b);
}

#[test]
fn alias_accepts_shadows() {
    let mut app = App::new();
    let comp = app.add_comp("c", 200, 100, 1.0, 2.0, 25.0).unwrap();
    let layer = app.add_solid_layer(comp, [1.0, 0.0, 0.0], "red", 100, 80, 1.0, None).unwrap();
    app.set_three_d_layer(layer, true).unwrap();
    let a = app.property_by_name(layer, "Accepts Shadows").unwrap();
    let b = app.property_by_name(layer, "acceptsShadows").unwrap();
    assert_eq!(a, b);
}

#[test]
fn alias_accepts_lights() {
    let mut app = App::new();
    let comp = app.add_comp("c", 200, 100, 1.0, 2.0, 25.0).unwrap();
    let layer = app.add_solid_layer(comp, [1.0, 0.0, 0.0], "red", 100, 80, 1.0, None).unwrap();
    app.set_three_d_layer(layer, true).unwrap();
    let a = app.property_by_name(layer, "Accepts Lights").unwrap();
    let b = app.property_by_name(layer, "acceptsLights").unwrap();
    assert_eq!(a, b);
}

#[test]
fn alias_ambient() {
    let mut app = App::new();
    let comp = app.add_comp("c", 200, 100, 1.0, 2.0, 25.0).unwrap();
    let layer = app.add_solid_layer(comp, [1.0, 0.0, 0.0], "red", 100, 80, 1.0, None).unwrap();
    app.set_three_d_layer(layer, true).unwrap();
    let a = app.property_by_name(layer, "Ambient").unwrap();
    let b = app.property_by_name(layer, "ambient").unwrap();
    assert_eq!(a, b);
}

#[test]
fn alias_diffuse() {
    let mut app = App::new();
    let comp = app.add_comp("c", 200, 100, 1.0, 2.0, 25.0).unwrap();
    let layer = app.add_solid_layer(comp, [1.0, 0.0, 0.0], "red", 100, 80, 1.0, None).unwrap();
    app.set_three_d_layer(layer, true).unwrap();
    let a = app.property_by_name(layer, "Diffuse").unwrap();
    let b = app.property_by_name(layer, "diffuse").unwrap();
    assert_eq!(a, b);
}

#[test]
fn alias_specular() {
    let mut app = App::new();
    let comp = app.add_comp("c", 200, 100, 1.0, 2.0, 25.0).unwrap();
    let layer = app.add_solid_layer(comp, [1.0, 0.0, 0.0], "red", 100, 80, 1.0, None).unwrap();
    app.set_three_d_layer(layer, true).unwrap();
    let a = app.property_by_name(layer, "Specular").unwrap();
    let b = app.property_by_name(layer, "specular").unwrap();
    assert_eq!(a, b);
}

#[test]
fn alias_shininess() {
    let mut app = App::new();
    let comp = app.add_comp("c", 200, 100, 1.0, 2.0, 25.0).unwrap();
    let layer = app.add_solid_layer(comp, [1.0, 0.0, 0.0], "red", 100, 80, 1.0, None).unwrap();
    app.set_three_d_layer(layer, true).unwrap();
    let a = app.property_by_name(layer, "Shininess").unwrap();
    let b = app.property_by_name(layer, "shininess").unwrap();
    assert_eq!(a, b);
}

#[test]
fn alias_casts_shadows_3d() {
    let mut app = App::new();
    let comp = app.add_comp("c", 200, 100, 1.0, 2.0, 25.0).unwrap();
    let layer = app.add_solid_layer(comp, [1.0, 0.0, 0.0], "red", 100, 80, 1.0, None).unwrap();
    app.set_three_d_layer(layer, true).unwrap();
    let a = app.property_by_name(layer, "Casts Shadows").unwrap();
    let b = app.property_by_name(layer, "castsShadows").unwrap();
    assert_eq!(a, b);
}

#[test]
fn alias_transmission() {
    let mut app = App::new();
    let comp = app.add_comp("c", 200, 100, 1.0, 2.0, 25.0).unwrap();
    let layer = app.add_solid_layer(comp, [1.0, 0.0, 0.0], "red", 100, 80, 1.0, None).unwrap();
    app.set_three_d_layer(layer, true).unwrap();
    let a = app.property_by_name(layer, "Light Transmission").unwrap();
    let b = app.property_by_name(layer, "lightTransmission").unwrap();
    assert_eq!(a, b);
}

#[test]
fn alias_metal() {
    let mut app = App::new();
    let comp = app.add_comp("c", 200, 100, 1.0, 2.0, 25.0).unwrap();
    let layer = app.add_solid_layer(comp, [1.0, 0.0, 0.0], "red", 100, 80, 1.0, None).unwrap();
    app.set_three_d_layer(layer, true).unwrap();
    let a = app.property_by_name(layer, "Metal").unwrap();
    let b = app.property_by_name(layer, "metal").unwrap();
    assert_eq!(a, b);
}

#[test]
fn alias_x_rotation() {
    let mut app = App::new();
    let comp = app.add_comp("c", 200, 100, 1.0, 2.0, 25.0).unwrap();
    let layer = app.add_camera(comp, "cam", [10.0, 20.0]).unwrap();
    let a = app.property_by_name(layer, "X Rotation").unwrap();
    let b = app.property_by_name(layer, "rotationX").unwrap();
    assert_eq!(a, b);
}

#[test]
fn alias_y_rotation() {
    let mut app = App::new();
    let comp = app.add_comp("c", 200, 100, 1.0, 2.0, 25.0).unwrap();
    let layer = app.add_camera(comp, "cam", [10.0, 20.0]).unwrap();
    let a = app.property_by_name(layer, "Y Rotation").unwrap();
    let b = app.property_by_name(layer, "rotationY").unwrap();
    assert_eq!(a, b);
}

#[test]
fn alias_orientation() {
    let mut app = App::new();
    let comp = app.add_comp("c", 200, 100, 1.0, 2.0, 25.0).unwrap();
    let layer = app.add_camera(comp, "cam", [10.0, 20.0]).unwrap();
    let a = app.property_by_name(layer, "Orientation").unwrap();
    let b = app.property_by_name(layer, "orientation").unwrap();
    assert_eq!(a, b);
}

#[test]
fn alias_source_text() {
    let mut app = App::new();
    let comp = app.add_comp("c", 200, 100, 1.0, 2.0, 25.0).unwrap();
    let layer = app.add_text_layer(comp, "あ").unwrap();
    let a = app.property_by_name(layer, "Source Text").unwrap();
    let b = app.property_by_name(layer, "text").unwrap();
    assert_eq!(a, b);
}

#[test]
fn alias_mask_atom() {
    let mut app = App::new();
    let comp = app.add_comp("c", 200, 100, 1.0, 2.0, 25.0).unwrap();
    let layer = app.add_solid_layer(comp, [1.0, 0.0, 0.0], "red", 100, 80, 1.0, None).unwrap();
    let parade = app.property_by_name(layer, "ADBE Mask Parade").unwrap();
    app.add_property(parade, "ADBE Mask Atom").unwrap();
    let a = app.property_by_name(layer, "ADBE Mask Atom").unwrap();
    let b = app.property_by_name(layer, "ADBE Mask Atom").unwrap();
    assert_eq!(a, b);
}

#[test]
fn alias_mask_shape() {
    let mut app = App::new();
    let comp = app.add_comp("c", 200, 100, 1.0, 2.0, 25.0).unwrap();
    let layer = app.add_solid_layer(comp, [1.0, 0.0, 0.0], "red", 100, 80, 1.0, None).unwrap();
    let parade = app.property_by_name(layer, "ADBE Mask Parade").unwrap();
    app.add_property(parade, "ADBE Mask Atom").unwrap();
    let a = app.property_by_name(layer, "ADBE Mask Shape").unwrap();
    let b = app.property_by_name(layer, "maskPath").unwrap();
    assert_eq!(a, b);
}

#[test]
fn alias_mask_feather() {
    let mut app = App::new();
    let comp = app.add_comp("c", 200, 100, 1.0, 2.0, 25.0).unwrap();
    let layer = app.add_solid_layer(comp, [1.0, 0.0, 0.0], "red", 100, 80, 1.0, None).unwrap();
    let parade = app.property_by_name(layer, "ADBE Mask Parade").unwrap();
    app.add_property(parade, "ADBE Mask Atom").unwrap();
    let a = app.property_by_name(layer, "ADBE Mask Feather").unwrap();
    let b = app.property_by_name(layer, "maskFeather").unwrap();
    assert_eq!(a, b);
}

#[test]
fn alias_mask_opacity() {
    let mut app = App::new();
    let comp = app.add_comp("c", 200, 100, 1.0, 2.0, 25.0).unwrap();
    let layer = app.add_solid_layer(comp, [1.0, 0.0, 0.0], "red", 100, 80, 1.0, None).unwrap();
    let parade = app.property_by_name(layer, "ADBE Mask Parade").unwrap();
    app.add_property(parade, "ADBE Mask Atom").unwrap();
    let a = app.property_by_name(layer, "ADBE Mask Opacity").unwrap();
    let b = app.property_by_name(layer, "maskOpacity").unwrap();
    assert_eq!(a, b);
}

#[test]
fn alias_mask_offset() {
    let mut app = App::new();
    let comp = app.add_comp("c", 200, 100, 1.0, 2.0, 25.0).unwrap();
    let layer = app.add_solid_layer(comp, [1.0, 0.0, 0.0], "red", 100, 80, 1.0, None).unwrap();
    let parade = app.property_by_name(layer, "ADBE Mask Parade").unwrap();
    app.add_property(parade, "ADBE Mask Atom").unwrap();
    let a = app.property_by_name(layer, "ADBE Mask Offset").unwrap();
    let b = app.property_by_name(layer, "maskOffset").unwrap();
    assert_eq!(a, b);
}

#[test]
fn mesh_cube_width() {
    let mut app = App::new();
    let comp = app.add_comp("c", 100, 100, 1.0, 1.0, 24.0).unwrap();
    let layer = app.add_parametric_mesh(comp, "m", ParametricMeshType::Cube).unwrap();
    app.set_mesh_field(layer, "width", 1.0).unwrap();
    assert_eq!(app.mesh_field(layer, "width").unwrap(), 1.0);
    app.set_mesh_type(layer, ParametricMeshType::Sphere).unwrap();
    assert!(app.mesh_field(layer, "width").is_err());
}

#[test]
fn mesh_cube_height() {
    let mut app = App::new();
    let comp = app.add_comp("c", 100, 100, 1.0, 1.0, 24.0).unwrap();
    let layer = app.add_parametric_mesh(comp, "m", ParametricMeshType::Cube).unwrap();
    app.set_mesh_field(layer, "height", 1.0).unwrap();
    assert_eq!(app.mesh_field(layer, "height").unwrap(), 1.0);
    app.set_mesh_type(layer, ParametricMeshType::Sphere).unwrap();
    assert!(app.mesh_field(layer, "height").is_err());
}

#[test]
fn mesh_cube_depth() {
    let mut app = App::new();
    let comp = app.add_comp("c", 100, 100, 1.0, 1.0, 24.0).unwrap();
    let layer = app.add_parametric_mesh(comp, "m", ParametricMeshType::Cube).unwrap();
    app.set_mesh_field(layer, "depth", 1.0).unwrap();
    assert_eq!(app.mesh_field(layer, "depth").unwrap(), 1.0);
    app.set_mesh_type(layer, ParametricMeshType::Sphere).unwrap();
    assert!(app.mesh_field(layer, "depth").is_err());
}

#[test]
fn mesh_cube_smoothingangle() {
    let mut app = App::new();
    let comp = app.add_comp("c", 100, 100, 1.0, 1.0, 24.0).unwrap();
    let layer = app.add_parametric_mesh(comp, "m", ParametricMeshType::Cube).unwrap();
    app.set_mesh_field(layer, "smoothingAngle", 1.0).unwrap();
    assert_eq!(app.mesh_field(layer, "smoothingAngle").unwrap(), 1.0);
    app.set_mesh_type(layer, ParametricMeshType::Plane).unwrap();
    assert!(app.mesh_field(layer, "smoothingAngle").is_err());
}

#[test]
fn mesh_sphere_radius() {
    let mut app = App::new();
    let comp = app.add_comp("c", 100, 100, 1.0, 1.0, 24.0).unwrap();
    let layer = app.add_parametric_mesh(comp, "m", ParametricMeshType::Sphere).unwrap();
    app.set_mesh_field(layer, "radius", 1.0).unwrap();
    assert_eq!(app.mesh_field(layer, "radius").unwrap(), 1.0);
    app.set_mesh_type(layer, ParametricMeshType::Plane).unwrap();
    assert!(app.mesh_field(layer, "radius").is_err());
}

#[test]
fn mesh_sphere_sides() {
    let mut app = App::new();
    let comp = app.add_comp("c", 100, 100, 1.0, 1.0, 24.0).unwrap();
    let layer = app.add_parametric_mesh(comp, "m", ParametricMeshType::Sphere).unwrap();
    app.set_mesh_field(layer, "sides", 1.0).unwrap();
    assert_eq!(app.mesh_field(layer, "sides").unwrap(), 1.0);
    app.set_mesh_type(layer, ParametricMeshType::Plane).unwrap();
    assert!(app.mesh_field(layer, "sides").is_err());
}

#[test]
fn mesh_sphere_slicecaps() {
    let mut app = App::new();
    let comp = app.add_comp("c", 100, 100, 1.0, 1.0, 24.0).unwrap();
    let layer = app.add_parametric_mesh(comp, "m", ParametricMeshType::Sphere).unwrap();
    app.set_mesh_field(layer, "sliceCaps", 1.0).unwrap();
    assert_eq!(app.mesh_field(layer, "sliceCaps").unwrap(), 1.0);
    app.set_mesh_type(layer, ParametricMeshType::Plane).unwrap();
    assert!(app.mesh_field(layer, "sliceCaps").is_err());
}

#[test]
fn mesh_sphere_slicestart() {
    let mut app = App::new();
    let comp = app.add_comp("c", 100, 100, 1.0, 1.0, 24.0).unwrap();
    let layer = app.add_parametric_mesh(comp, "m", ParametricMeshType::Sphere).unwrap();
    app.set_mesh_field(layer, "sliceStart", 1.0).unwrap();
    assert_eq!(app.mesh_field(layer, "sliceStart").unwrap(), 1.0);
    app.set_mesh_type(layer, ParametricMeshType::Plane).unwrap();
    assert!(app.mesh_field(layer, "sliceStart").is_err());
}

#[test]
fn mesh_sphere_sliceend() {
    let mut app = App::new();
    let comp = app.add_comp("c", 100, 100, 1.0, 1.0, 24.0).unwrap();
    let layer = app.add_parametric_mesh(comp, "m", ParametricMeshType::Sphere).unwrap();
    app.set_mesh_field(layer, "sliceEnd", 1.0).unwrap();
    assert_eq!(app.mesh_field(layer, "sliceEnd").unwrap(), 1.0);
    app.set_mesh_type(layer, ParametricMeshType::Plane).unwrap();
    assert!(app.mesh_field(layer, "sliceEnd").is_err());
}

#[test]
fn mesh_sphere_smoothingangle() {
    let mut app = App::new();
    let comp = app.add_comp("c", 100, 100, 1.0, 1.0, 24.0).unwrap();
    let layer = app.add_parametric_mesh(comp, "m", ParametricMeshType::Sphere).unwrap();
    app.set_mesh_field(layer, "smoothingAngle", 1.0).unwrap();
    assert_eq!(app.mesh_field(layer, "smoothingAngle").unwrap(), 1.0);
    app.set_mesh_type(layer, ParametricMeshType::Plane).unwrap();
    assert!(app.mesh_field(layer, "smoothingAngle").is_err());
}

#[test]
fn mesh_plane_width() {
    let mut app = App::new();
    let comp = app.add_comp("c", 100, 100, 1.0, 1.0, 24.0).unwrap();
    let layer = app.add_parametric_mesh(comp, "m", ParametricMeshType::Plane).unwrap();
    app.set_mesh_field(layer, "width", 1.0).unwrap();
    assert_eq!(app.mesh_field(layer, "width").unwrap(), 1.0);
    app.set_mesh_type(layer, ParametricMeshType::Sphere).unwrap();
    assert!(app.mesh_field(layer, "width").is_err());
}

#[test]
fn mesh_plane_length() {
    let mut app = App::new();
    let comp = app.add_comp("c", 100, 100, 1.0, 1.0, 24.0).unwrap();
    let layer = app.add_parametric_mesh(comp, "m", ParametricMeshType::Plane).unwrap();
    app.set_mesh_field(layer, "length", 1.0).unwrap();
    assert_eq!(app.mesh_field(layer, "length").unwrap(), 1.0);
    app.set_mesh_type(layer, ParametricMeshType::Sphere).unwrap();
    assert!(app.mesh_field(layer, "length").is_err());
}

#[test]
fn mesh_plane_cornerradius() {
    let mut app = App::new();
    let comp = app.add_comp("c", 100, 100, 1.0, 1.0, 24.0).unwrap();
    let layer = app.add_parametric_mesh(comp, "m", ParametricMeshType::Plane).unwrap();
    app.set_mesh_field(layer, "cornerRadius", 1.0).unwrap();
    assert_eq!(app.mesh_field(layer, "cornerRadius").unwrap(), 1.0);
    app.set_mesh_type(layer, ParametricMeshType::Sphere).unwrap();
    assert!(app.mesh_field(layer, "cornerRadius").is_err());
}

#[test]
fn mesh_plane_cornersides() {
    let mut app = App::new();
    let comp = app.add_comp("c", 100, 100, 1.0, 1.0, 24.0).unwrap();
    let layer = app.add_parametric_mesh(comp, "m", ParametricMeshType::Plane).unwrap();
    app.set_mesh_field(layer, "cornerSides", 1.0).unwrap();
    assert_eq!(app.mesh_field(layer, "cornerSides").unwrap(), 1.0);
    app.set_mesh_type(layer, ParametricMeshType::Sphere).unwrap();
    assert!(app.mesh_field(layer, "cornerSides").is_err());
}

#[test]
fn mesh_torus_ringradius() {
    let mut app = App::new();
    let comp = app.add_comp("c", 100, 100, 1.0, 1.0, 24.0).unwrap();
    let layer = app.add_parametric_mesh(comp, "m", ParametricMeshType::Torus).unwrap();
    app.set_mesh_field(layer, "ringRadius", 1.0).unwrap();
    assert_eq!(app.mesh_field(layer, "ringRadius").unwrap(), 1.0);
    app.set_mesh_type(layer, ParametricMeshType::Sphere).unwrap();
    assert!(app.mesh_field(layer, "ringRadius").is_err());
}

#[test]
fn mesh_torus_piperadius() {
    let mut app = App::new();
    let comp = app.add_comp("c", 100, 100, 1.0, 1.0, 24.0).unwrap();
    let layer = app.add_parametric_mesh(comp, "m", ParametricMeshType::Torus).unwrap();
    app.set_mesh_field(layer, "pipeRadius", 1.0).unwrap();
    assert_eq!(app.mesh_field(layer, "pipeRadius").unwrap(), 1.0);
    app.set_mesh_type(layer, ParametricMeshType::Sphere).unwrap();
    assert!(app.mesh_field(layer, "pipeRadius").is_err());
}

#[test]
fn mesh_torus_ringsides() {
    let mut app = App::new();
    let comp = app.add_comp("c", 100, 100, 1.0, 1.0, 24.0).unwrap();
    let layer = app.add_parametric_mesh(comp, "m", ParametricMeshType::Torus).unwrap();
    app.set_mesh_field(layer, "ringSides", 1.0).unwrap();
    assert_eq!(app.mesh_field(layer, "ringSides").unwrap(), 1.0);
    app.set_mesh_type(layer, ParametricMeshType::Sphere).unwrap();
    assert!(app.mesh_field(layer, "ringSides").is_err());
}

#[test]
fn mesh_torus_pipesides() {
    let mut app = App::new();
    let comp = app.add_comp("c", 100, 100, 1.0, 1.0, 24.0).unwrap();
    let layer = app.add_parametric_mesh(comp, "m", ParametricMeshType::Torus).unwrap();
    app.set_mesh_field(layer, "pipeSides", 1.0).unwrap();
    assert_eq!(app.mesh_field(layer, "pipeSides").unwrap(), 1.0);
    app.set_mesh_type(layer, ParametricMeshType::Sphere).unwrap();
    assert!(app.mesh_field(layer, "pipeSides").is_err());
}

#[test]
fn mesh_torus_caps() {
    let mut app = App::new();
    let comp = app.add_comp("c", 100, 100, 1.0, 1.0, 24.0).unwrap();
    let layer = app.add_parametric_mesh(comp, "m", ParametricMeshType::Torus).unwrap();
    app.set_mesh_field(layer, "caps", 1.0).unwrap();
    assert_eq!(app.mesh_field(layer, "caps").unwrap(), 1.0);
    app.set_mesh_type(layer, ParametricMeshType::Sphere).unwrap();
    assert!(app.mesh_field(layer, "caps").is_err());
}

#[test]
fn mesh_torus_slicestart() {
    let mut app = App::new();
    let comp = app.add_comp("c", 100, 100, 1.0, 1.0, 24.0).unwrap();
    let layer = app.add_parametric_mesh(comp, "m", ParametricMeshType::Torus).unwrap();
    app.set_mesh_field(layer, "sliceStart", 1.0).unwrap();
    assert_eq!(app.mesh_field(layer, "sliceStart").unwrap(), 1.0);
    app.set_mesh_type(layer, ParametricMeshType::Plane).unwrap();
    assert!(app.mesh_field(layer, "sliceStart").is_err());
}

#[test]
fn mesh_torus_sliceend() {
    let mut app = App::new();
    let comp = app.add_comp("c", 100, 100, 1.0, 1.0, 24.0).unwrap();
    let layer = app.add_parametric_mesh(comp, "m", ParametricMeshType::Torus).unwrap();
    app.set_mesh_field(layer, "sliceEnd", 1.0).unwrap();
    assert_eq!(app.mesh_field(layer, "sliceEnd").unwrap(), 1.0);
    app.set_mesh_type(layer, ParametricMeshType::Plane).unwrap();
    assert!(app.mesh_field(layer, "sliceEnd").is_err());
}

#[test]
fn mesh_torus_smoothingangle() {
    let mut app = App::new();
    let comp = app.add_comp("c", 100, 100, 1.0, 1.0, 24.0).unwrap();
    let layer = app.add_parametric_mesh(comp, "m", ParametricMeshType::Torus).unwrap();
    app.set_mesh_field(layer, "smoothingAngle", 1.0).unwrap();
    assert_eq!(app.mesh_field(layer, "smoothingAngle").unwrap(), 1.0);
    app.set_mesh_type(layer, ParametricMeshType::Plane).unwrap();
    assert!(app.mesh_field(layer, "smoothingAngle").is_err());
}

#[test]
fn mesh_cone_topradius() {
    let mut app = App::new();
    let comp = app.add_comp("c", 100, 100, 1.0, 1.0, 24.0).unwrap();
    let layer = app.add_parametric_mesh(comp, "m", ParametricMeshType::Cone).unwrap();
    app.set_mesh_field(layer, "topRadius", 1.0).unwrap();
    assert_eq!(app.mesh_field(layer, "topRadius").unwrap(), 1.0);
    app.set_mesh_type(layer, ParametricMeshType::Sphere).unwrap();
    assert!(app.mesh_field(layer, "topRadius").is_err());
}

#[test]
fn mesh_cone_bottomradius() {
    let mut app = App::new();
    let comp = app.add_comp("c", 100, 100, 1.0, 1.0, 24.0).unwrap();
    let layer = app.add_parametric_mesh(comp, "m", ParametricMeshType::Cone).unwrap();
    app.set_mesh_field(layer, "bottomRadius", 1.0).unwrap();
    assert_eq!(app.mesh_field(layer, "bottomRadius").unwrap(), 1.0);
    app.set_mesh_type(layer, ParametricMeshType::Sphere).unwrap();
    assert!(app.mesh_field(layer, "bottomRadius").is_err());
}

#[test]
fn mesh_cone_height() {
    let mut app = App::new();
    let comp = app.add_comp("c", 100, 100, 1.0, 1.0, 24.0).unwrap();
    let layer = app.add_parametric_mesh(comp, "m", ParametricMeshType::Cone).unwrap();
    app.set_mesh_field(layer, "height", 1.0).unwrap();
    assert_eq!(app.mesh_field(layer, "height").unwrap(), 1.0);
    app.set_mesh_type(layer, ParametricMeshType::Sphere).unwrap();
    assert!(app.mesh_field(layer, "height").is_err());
}

#[test]
fn mesh_cone_sides() {
    let mut app = App::new();
    let comp = app.add_comp("c", 100, 100, 1.0, 1.0, 24.0).unwrap();
    let layer = app.add_parametric_mesh(comp, "m", ParametricMeshType::Cone).unwrap();
    app.set_mesh_field(layer, "sides", 1.0).unwrap();
    assert_eq!(app.mesh_field(layer, "sides").unwrap(), 1.0);
    app.set_mesh_type(layer, ParametricMeshType::Plane).unwrap();
    assert!(app.mesh_field(layer, "sides").is_err());
}

#[test]
fn mesh_cone_topcap() {
    let mut app = App::new();
    let comp = app.add_comp("c", 100, 100, 1.0, 1.0, 24.0).unwrap();
    let layer = app.add_parametric_mesh(comp, "m", ParametricMeshType::Cone).unwrap();
    app.set_mesh_field(layer, "topCap", 1.0).unwrap();
    assert_eq!(app.mesh_field(layer, "topCap").unwrap(), 1.0);
    app.set_mesh_type(layer, ParametricMeshType::Sphere).unwrap();
    assert!(app.mesh_field(layer, "topCap").is_err());
}

#[test]
fn mesh_cone_bottomcap() {
    let mut app = App::new();
    let comp = app.add_comp("c", 100, 100, 1.0, 1.0, 24.0).unwrap();
    let layer = app.add_parametric_mesh(comp, "m", ParametricMeshType::Cone).unwrap();
    app.set_mesh_field(layer, "bottomCap", 1.0).unwrap();
    assert_eq!(app.mesh_field(layer, "bottomCap").unwrap(), 1.0);
    app.set_mesh_type(layer, ParametricMeshType::Sphere).unwrap();
    assert!(app.mesh_field(layer, "bottomCap").is_err());
}

#[test]
fn mesh_cone_slicecaps() {
    let mut app = App::new();
    let comp = app.add_comp("c", 100, 100, 1.0, 1.0, 24.0).unwrap();
    let layer = app.add_parametric_mesh(comp, "m", ParametricMeshType::Cone).unwrap();
    app.set_mesh_field(layer, "sliceCaps", 1.0).unwrap();
    assert_eq!(app.mesh_field(layer, "sliceCaps").unwrap(), 1.0);
    app.set_mesh_type(layer, ParametricMeshType::Plane).unwrap();
    assert!(app.mesh_field(layer, "sliceCaps").is_err());
}

#[test]
fn mesh_cone_slicestart() {
    let mut app = App::new();
    let comp = app.add_comp("c", 100, 100, 1.0, 1.0, 24.0).unwrap();
    let layer = app.add_parametric_mesh(comp, "m", ParametricMeshType::Cone).unwrap();
    app.set_mesh_field(layer, "sliceStart", 1.0).unwrap();
    assert_eq!(app.mesh_field(layer, "sliceStart").unwrap(), 1.0);
    app.set_mesh_type(layer, ParametricMeshType::Plane).unwrap();
    assert!(app.mesh_field(layer, "sliceStart").is_err());
}

#[test]
fn mesh_cone_sliceend() {
    let mut app = App::new();
    let comp = app.add_comp("c", 100, 100, 1.0, 1.0, 24.0).unwrap();
    let layer = app.add_parametric_mesh(comp, "m", ParametricMeshType::Cone).unwrap();
    app.set_mesh_field(layer, "sliceEnd", 1.0).unwrap();
    assert_eq!(app.mesh_field(layer, "sliceEnd").unwrap(), 1.0);
    app.set_mesh_type(layer, ParametricMeshType::Plane).unwrap();
    assert!(app.mesh_field(layer, "sliceEnd").is_err());
}

#[test]
fn mesh_cone_smoothingangle() {
    let mut app = App::new();
    let comp = app.add_comp("c", 100, 100, 1.0, 1.0, 24.0).unwrap();
    let layer = app.add_parametric_mesh(comp, "m", ParametricMeshType::Cone).unwrap();
    app.set_mesh_field(layer, "smoothingAngle", 1.0).unwrap();
    assert_eq!(app.mesh_field(layer, "smoothingAngle").unwrap(), 1.0);
    app.set_mesh_type(layer, ParametricMeshType::Plane).unwrap();
    assert!(app.mesh_field(layer, "smoothingAngle").is_err());
}

#[test]
fn mesh_cylinder_radius() {
    let mut app = App::new();
    let comp = app.add_comp("c", 100, 100, 1.0, 1.0, 24.0).unwrap();
    let layer = app.add_parametric_mesh(comp, "m", ParametricMeshType::Cylinder).unwrap();
    app.set_mesh_field(layer, "radius", 1.0).unwrap();
    assert_eq!(app.mesh_field(layer, "radius").unwrap(), 1.0);
    app.set_mesh_type(layer, ParametricMeshType::Plane).unwrap();
    assert!(app.mesh_field(layer, "radius").is_err());
}

#[test]
fn mesh_cylinder_height() {
    let mut app = App::new();
    let comp = app.add_comp("c", 100, 100, 1.0, 1.0, 24.0).unwrap();
    let layer = app.add_parametric_mesh(comp, "m", ParametricMeshType::Cylinder).unwrap();
    app.set_mesh_field(layer, "height", 1.0).unwrap();
    assert_eq!(app.mesh_field(layer, "height").unwrap(), 1.0);
    app.set_mesh_type(layer, ParametricMeshType::Sphere).unwrap();
    assert!(app.mesh_field(layer, "height").is_err());
}

#[test]
fn mesh_cylinder_sides() {
    let mut app = App::new();
    let comp = app.add_comp("c", 100, 100, 1.0, 1.0, 24.0).unwrap();
    let layer = app.add_parametric_mesh(comp, "m", ParametricMeshType::Cylinder).unwrap();
    app.set_mesh_field(layer, "sides", 1.0).unwrap();
    assert_eq!(app.mesh_field(layer, "sides").unwrap(), 1.0);
    app.set_mesh_type(layer, ParametricMeshType::Plane).unwrap();
    assert!(app.mesh_field(layer, "sides").is_err());
}

#[test]
fn mesh_cylinder_topcap() {
    let mut app = App::new();
    let comp = app.add_comp("c", 100, 100, 1.0, 1.0, 24.0).unwrap();
    let layer = app.add_parametric_mesh(comp, "m", ParametricMeshType::Cylinder).unwrap();
    app.set_mesh_field(layer, "topCap", 1.0).unwrap();
    assert_eq!(app.mesh_field(layer, "topCap").unwrap(), 1.0);
    app.set_mesh_type(layer, ParametricMeshType::Sphere).unwrap();
    assert!(app.mesh_field(layer, "topCap").is_err());
}

#[test]
fn mesh_cylinder_bottomcap() {
    let mut app = App::new();
    let comp = app.add_comp("c", 100, 100, 1.0, 1.0, 24.0).unwrap();
    let layer = app.add_parametric_mesh(comp, "m", ParametricMeshType::Cylinder).unwrap();
    app.set_mesh_field(layer, "bottomCap", 1.0).unwrap();
    assert_eq!(app.mesh_field(layer, "bottomCap").unwrap(), 1.0);
    app.set_mesh_type(layer, ParametricMeshType::Sphere).unwrap();
    assert!(app.mesh_field(layer, "bottomCap").is_err());
}

#[test]
fn mesh_cylinder_slicecaps() {
    let mut app = App::new();
    let comp = app.add_comp("c", 100, 100, 1.0, 1.0, 24.0).unwrap();
    let layer = app.add_parametric_mesh(comp, "m", ParametricMeshType::Cylinder).unwrap();
    app.set_mesh_field(layer, "sliceCaps", 1.0).unwrap();
    assert_eq!(app.mesh_field(layer, "sliceCaps").unwrap(), 1.0);
    app.set_mesh_type(layer, ParametricMeshType::Plane).unwrap();
    assert!(app.mesh_field(layer, "sliceCaps").is_err());
}

#[test]
fn mesh_cylinder_slicestart() {
    let mut app = App::new();
    let comp = app.add_comp("c", 100, 100, 1.0, 1.0, 24.0).unwrap();
    let layer = app.add_parametric_mesh(comp, "m", ParametricMeshType::Cylinder).unwrap();
    app.set_mesh_field(layer, "sliceStart", 1.0).unwrap();
    assert_eq!(app.mesh_field(layer, "sliceStart").unwrap(), 1.0);
    app.set_mesh_type(layer, ParametricMeshType::Plane).unwrap();
    assert!(app.mesh_field(layer, "sliceStart").is_err());
}

#[test]
fn mesh_cylinder_sliceend() {
    let mut app = App::new();
    let comp = app.add_comp("c", 100, 100, 1.0, 1.0, 24.0).unwrap();
    let layer = app.add_parametric_mesh(comp, "m", ParametricMeshType::Cylinder).unwrap();
    app.set_mesh_field(layer, "sliceEnd", 1.0).unwrap();
    assert_eq!(app.mesh_field(layer, "sliceEnd").unwrap(), 1.0);
    app.set_mesh_type(layer, ParametricMeshType::Plane).unwrap();
    assert!(app.mesh_field(layer, "sliceEnd").is_err());
}

#[test]
fn mesh_cylinder_smoothingangle() {
    let mut app = App::new();
    let comp = app.add_comp("c", 100, 100, 1.0, 1.0, 24.0).unwrap();
    let layer = app.add_parametric_mesh(comp, "m", ParametricMeshType::Cylinder).unwrap();
    app.set_mesh_field(layer, "smoothingAngle", 1.0).unwrap();
    assert_eq!(app.mesh_field(layer, "smoothingAngle").unwrap(), 1.0);
    app.set_mesh_type(layer, ParametricMeshType::Plane).unwrap();
    assert!(app.mesh_field(layer, "smoothingAngle").is_err());
}

#[test]
fn bevel_cube_radius() {
    let mut app = App::new();
    let comp = app.add_comp("c", 100, 100, 1.0, 1.0, 24.0).unwrap();
    let layer = app.add_parametric_mesh(comp, "m", ParametricMeshType::Cube).unwrap();
    app.set_bevel_field(layer, "radius", 1.0).unwrap();
    assert_eq!(app.bevel_field(layer, "radius").unwrap(), 1.0);
    app.set_mesh_type(layer, ParametricMeshType::Sphere).unwrap();
    assert!(app.bevel_field(layer, "radius").is_err());
}

#[test]
fn bevel_cube_sides() {
    let mut app = App::new();
    let comp = app.add_comp("c", 100, 100, 1.0, 1.0, 24.0).unwrap();
    let layer = app.add_parametric_mesh(comp, "m", ParametricMeshType::Cube).unwrap();
    app.set_bevel_field(layer, "sides", 1.0).unwrap();
    assert_eq!(app.bevel_field(layer, "sides").unwrap(), 1.0);
    app.set_mesh_type(layer, ParametricMeshType::Sphere).unwrap();
    assert!(app.bevel_field(layer, "sides").is_err());
}

#[test]
fn bevel_cone_topradius() {
    let mut app = App::new();
    let comp = app.add_comp("c", 100, 100, 1.0, 1.0, 24.0).unwrap();
    let layer = app.add_parametric_mesh(comp, "m", ParametricMeshType::Cone).unwrap();
    app.set_bevel_field(layer, "topRadius", 1.0).unwrap();
    assert_eq!(app.bevel_field(layer, "topRadius").unwrap(), 1.0);
    app.set_mesh_type(layer, ParametricMeshType::Sphere).unwrap();
    assert!(app.bevel_field(layer, "topRadius").is_err());
}

#[test]
fn bevel_cone_topsides() {
    let mut app = App::new();
    let comp = app.add_comp("c", 100, 100, 1.0, 1.0, 24.0).unwrap();
    let layer = app.add_parametric_mesh(comp, "m", ParametricMeshType::Cone).unwrap();
    app.set_bevel_field(layer, "topSides", 1.0).unwrap();
    assert_eq!(app.bevel_field(layer, "topSides").unwrap(), 1.0);
    app.set_mesh_type(layer, ParametricMeshType::Sphere).unwrap();
    assert!(app.bevel_field(layer, "topSides").is_err());
}

#[test]
fn bevel_cone_bottomradius() {
    let mut app = App::new();
    let comp = app.add_comp("c", 100, 100, 1.0, 1.0, 24.0).unwrap();
    let layer = app.add_parametric_mesh(comp, "m", ParametricMeshType::Cone).unwrap();
    app.set_bevel_field(layer, "bottomRadius", 1.0).unwrap();
    assert_eq!(app.bevel_field(layer, "bottomRadius").unwrap(), 1.0);
    app.set_mesh_type(layer, ParametricMeshType::Sphere).unwrap();
    assert!(app.bevel_field(layer, "bottomRadius").is_err());
}

#[test]
fn bevel_cone_bottomsides() {
    let mut app = App::new();
    let comp = app.add_comp("c", 100, 100, 1.0, 1.0, 24.0).unwrap();
    let layer = app.add_parametric_mesh(comp, "m", ParametricMeshType::Cone).unwrap();
    app.set_bevel_field(layer, "bottomSides", 1.0).unwrap();
    assert_eq!(app.bevel_field(layer, "bottomSides").unwrap(), 1.0);
    app.set_mesh_type(layer, ParametricMeshType::Sphere).unwrap();
    assert!(app.bevel_field(layer, "bottomSides").is_err());
}

#[test]
fn bevel_cylinder_radius() {
    let mut app = App::new();
    let comp = app.add_comp("c", 100, 100, 1.0, 1.0, 24.0).unwrap();
    let layer = app.add_parametric_mesh(comp, "m", ParametricMeshType::Cylinder).unwrap();
    app.set_bevel_field(layer, "radius", 1.0).unwrap();
    assert_eq!(app.bevel_field(layer, "radius").unwrap(), 1.0);
    app.set_mesh_type(layer, ParametricMeshType::Sphere).unwrap();
    assert!(app.bevel_field(layer, "radius").is_err());
}

#[test]
fn bevel_cylinder_sides() {
    let mut app = App::new();
    let comp = app.add_comp("c", 100, 100, 1.0, 1.0, 24.0).unwrap();
    let layer = app.add_parametric_mesh(comp, "m", ParametricMeshType::Cylinder).unwrap();
    app.set_bevel_field(layer, "sides", 1.0).unwrap();
    assert_eq!(app.bevel_field(layer, "sides").unwrap(), 1.0);
    app.set_mesh_type(layer, ParametricMeshType::Sphere).unwrap();
    assert!(app.bevel_field(layer, "sides").is_err());
}
