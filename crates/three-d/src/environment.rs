use model::LayerId;

use crate::confirmed::Confirmed;
use crate::light::SceneLightKind;
use crate::renderer::Renderer;

/// 空のソースには戻せない。既定マップかレイヤーのどちらか。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EnvironmentSource {
    DefaultMap,
    Layer(LayerId),
}

impl EnvironmentSource {
    pub const fn is_empty(self) -> bool {
        let _ = self;
        false
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ReflectionSource {
    None,
    HiddenDefault,
    Environment(EnvironmentSource),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SceneLightRef {
    pub kind: SceneLightKind,
    pub source: Option<EnvironmentSource>,
}

pub fn image_based_lighting(renderer: Renderer, kind: SceneLightKind) -> bool {
    renderer.environment_image_lighting() && kind == SceneLightKind::Environment
}

pub fn reflection_source(
    renderer: Renderer,
    has_3d_layer: bool,
    lights: &[SceneLightRef],
) -> ReflectionSource {
    if !renderer.environment_image_lighting() || !has_3d_layer {
        return ReflectionSource::None;
    }
    if let Some(light) = lights
        .iter()
        .find(|light| light.kind == SceneLightKind::Environment)
    {
        return ReflectionSource::Environment(
            light.source.unwrap_or(EnvironmentSource::DefaultMap),
        );
    }
    if lights.is_empty() {
        ReflectionSource::HiddenDefault
    } else {
        ReflectionSource::None
    }
}

pub const fn hidden_default_casts_shadows() -> bool {
    false
}

pub const fn environment_color_property() -> Confirmed<()> {
    Confirmed::Unconfirmed
}

pub const fn environment_intensity_property() -> Confirmed<()> {
    Confirmed::Unconfirmed
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EnvironmentBackground {
    pub at_bottom: bool,
    pub blur_radius_px: f64,
    pub effect: &'static str,
    pub follows_source: bool,
    pub source: EnvironmentSource,
}

pub const fn create_environment_background(source: EnvironmentSource) -> EnvironmentBackground {
    EnvironmentBackground {
        at_bottom: true,
        blur_radius_px: 5.0,
        effect: "CC Environment",
        follows_source: true,
        source,
    }
}

pub const fn environment_layer_switch_is_map(renderer: Renderer, switch_on: bool) -> bool {
    matches!(renderer, Renderer::Cinema4d) && switch_on
}

#[cfg(test)]
mod tests {
    use super::{
        create_environment_background, environment_color_property, environment_intensity_property,
        environment_layer_switch_is_map, hidden_default_casts_shadows, image_based_lighting,
        reflection_source, EnvironmentSource, ReflectionSource, SceneLightRef,
    };
    use crate::light::SceneLightKind;
    use crate::renderer::Renderer;
    use model::LayerId;

    #[test]
    fn environment_light_is_image_based_only_on_advanced() {
        assert!(image_based_lighting(
            Renderer::Advanced3d,
            SceneLightKind::Environment
        ));
        assert!(!image_based_lighting(
            Renderer::Classic3d,
            SceneLightKind::Environment
        ));
        assert_eq!(SceneLightKind::Environment.script_type(), "ENVIRONMENT");
    }

    #[test]
    fn environment_source_is_the_default_map_or_a_layer() {
        assert!(!EnvironmentSource::DefaultMap.is_empty());
        let first = EnvironmentSource::Layer(LayerId::new(1));
        let second = EnvironmentSource::Layer(LayerId::new(2));
        assert_ne!(first, second);
        let lights = [SceneLightRef {
            kind: SceneLightKind::Environment,
            source: Some(second),
        }];
        assert_eq!(
            reflection_source(Renderer::Advanced3d, true, &lights),
            ReflectionSource::Environment(second)
        );
    }

    #[test]
    fn environment_color_property_is_unconfirmed() {
        assert!(environment_color_property().is_unconfirmed());
    }

    #[test]
    fn environment_intensity_property_is_unconfirmed() {
        assert!(environment_intensity_property().is_unconfirmed());
    }

    #[test]
    fn new_environment_light_does_not_cast_shadows() {
        let light = crate::light::Light::new(SceneLightKind::Environment, 100, 100, 1);
        assert_eq!(
            light.casts_shadows,
            crate::confirmed::Confirmed::Known(false)
        );
        assert!(!hidden_default_casts_shadows());
    }

    #[test]
    fn hidden_default_environment_turns_off_when_a_light_is_added() {
        assert_eq!(
            reflection_source(Renderer::Advanced3d, true, &[]),
            ReflectionSource::HiddenDefault
        );
        let parallel = [SceneLightRef {
            kind: SceneLightKind::Parallel,
            source: None,
        }];
        assert_eq!(
            reflection_source(Renderer::Advanced3d, true, &parallel),
            ReflectionSource::None
        );
        let environment = [SceneLightRef {
            kind: SceneLightKind::Environment,
            source: Some(EnvironmentSource::DefaultMap),
        }];
        assert_eq!(
            reflection_source(Renderer::Advanced3d, true, &environment),
            ReflectionSource::Environment(EnvironmentSource::DefaultMap)
        );
        assert_eq!(
            reflection_source(Renderer::Classic3d, true, &[]),
            ReflectionSource::None
        );
    }

    #[test]
    fn environment_background_is_a_bottom_solid_with_a_5px_blur() {
        let background = create_environment_background(EnvironmentSource::Layer(LayerId::new(3)));
        assert!(background.at_bottom);
        assert_eq!(background.blur_radius_px, 5.0);
        assert!(background.follows_source);
        assert_eq!(background.effect, "CC Environment");
    }

    #[test]
    fn environment_layer_switch_works_for_cinema_only() {
        assert!(environment_layer_switch_is_map(Renderer::Cinema4d, true));
        assert!(!environment_layer_switch_is_map(Renderer::Advanced3d, true));
        assert!(!environment_layer_switch_is_map(Renderer::Cinema4d, false));
    }
}
