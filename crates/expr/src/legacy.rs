use crate::parse::{tokenize, TokenKind};

const LEGACY_NAMES: &[&str] = &[
    "this_comp",
    "this_layer",
    "this_property",
    "color_depth",
    "has_parent",
    "in_point",
    "out_point",
    "start_time",
    "has_video",
    "has_audio",
    "audio_active",
    "anchor_point",
    "audio_levels",
    "time_remap",
    "casts_shadows",
    "light_transmission",
    "accepts_shadows",
    "accepts_lights",
    "frame_duration",
    "shutter_angle",
    "shutter_phase",
    "num_layers",
    "pixel_aspect",
    "point_of_interest",
    "depth_of_field",
    "focus_distance",
    "blur_level",
    "cone_angle",
    "cone_feather",
    "shadow_darkness",
    "shadow_diffusion",
    "active_camera",
    "value_at_time",
    "velocity_at_time",
    "speed_at_time",
    "nearest_key",
    "posterize_time",
    "look_at",
    "seed_random",
    "gauss_random",
    "ease_in",
    "ease_out",
    "rgb_to_hsl",
    "hsl_to_rgb",
    "degrees_to_radians",
    "radians_to_degrees",
    "from_comp_to_surface",
    "to_comp_vec",
    "from_comp_vec",
    "to_world_vec",
    "from_world_vec",
    "to_comp",
    "from_comp",
    "to_world",
    "from_world",
    "temporal_wiggle",
    "loop_in_duration",
    "loop_out_duration",
    "loop_in",
    "loop_out",
];

const DOLLAR_OK: &[&str] = &["build", "engineName", "global"];

pub fn precheck(source: &str) -> Result<(), String> {
    let tokens = tokenize(source)?;
    for (index, token) in tokens.iter().enumerate() {
        if let TokenKind::Ident(name) = &token.kind {
            if LEGACY_NAMES.contains(&name.as_str()) {
                return Err(format!("legacy syntax {name}"));
            }
        }
        if matches!(&token.kind, TokenKind::Dot) {
            if let Some(TokenKind::Ident(name)) = tokens.get(index + 1).map(|token| &token.kind) {
                if name == "toSource" {
                    return Err("toSource is not supported".into());
                }
                if let Some(TokenKind::Ident(prev)) =
                    tokens.get(index.wrapping_sub(1)).map(|token| &token.kind)
                {
                    if prev == "reflect" && (name == "properties" || name == "methods") {
                        return Err(format!("reflect.{name} is not supported"));
                    }
                    if prev == "$" && !DOLLAR_OK.contains(&name.as_str()) {
                        return Err(format!("$.{name} is not supported"));
                    }
                }
            }
        }
    }
    Ok(())
}
