use model::Rgba;

use crate::confirmed::{Confirmed, Keyframes};
use crate::renderer::Renderer;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CastShadows {
    Off,
    On,
    Only,
}

impl Default for CastShadows {
    fn default() -> Self {
        Self::Off
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AcceptShadows {
    Off,
    On,
    Only,
}

impl Default for AcceptShadows {
    fn default() -> Self {
        Self::On
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MaterialSlot {
    CastShadows,
    LightTransmission,
    AcceptShadows,
    AcceptLights,
    Ambient,
    Diffuse,
    SpecularIntensity,
    SpecularShininess,
    Metal,
    ShadowColor,
    AppearsInReflections,
    ReflectionIntensity,
    ReflectionSharpness,
    ReflectionRolloff,
}

pub const fn material_available(slot: MaterialSlot, renderer: Renderer) -> Confirmed<bool> {
    match (slot, renderer) {
        (
            MaterialSlot::AppearsInReflections
            | MaterialSlot::ReflectionIntensity
            | MaterialSlot::ReflectionSharpness
            | MaterialSlot::ReflectionRolloff,
            Renderer::Cinema4d,
        ) => Confirmed::Known(true),
        (
            MaterialSlot::AppearsInReflections
            | MaterialSlot::ReflectionIntensity
            | MaterialSlot::ReflectionSharpness
            | MaterialSlot::ReflectionRolloff,
            Renderer::Classic3d | Renderer::Advanced3d,
        ) => Confirmed::Known(false),
        (MaterialSlot::ShadowColor, Renderer::Advanced3d | Renderer::Cinema4d) => {
            Confirmed::Known(true)
        }
        (MaterialSlot::ShadowColor, Renderer::Classic3d) => Confirmed::Unconfirmed,
        (MaterialSlot::LightTransmission, Renderer::Classic3d) => Confirmed::Known(true),
        (MaterialSlot::LightTransmission, Renderer::Advanced3d | Renderer::Cinema4d) => {
            Confirmed::Unconfirmed
        }
        (
            MaterialSlot::CastShadows
            | MaterialSlot::AcceptShadows
            | MaterialSlot::AcceptLights
            | MaterialSlot::Ambient
            | MaterialSlot::Diffuse
            | MaterialSlot::SpecularIntensity
            | MaterialSlot::SpecularShininess
            | MaterialSlot::Metal,
            _,
        ) => Confirmed::Known(true),
    }
}

pub const fn cast_shadow_only_on_advanced() -> Confirmed<bool> {
    Confirmed::Unconfirmed
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Material {
    pub cast_shadows: CastShadows,
    pub accept_shadows: AcceptShadows,
    pub accepts_lights: Confirmed<bool>,
    pub light_transmission_percent: Confirmed<f64>,
    pub ambient_percent: Confirmed<f64>,
    pub diffuse_percent: Confirmed<f64>,
    pub specular_percent: Confirmed<f64>,
    pub shininess_percent: Confirmed<f64>,
    pub metal_percent: Confirmed<f64>,
    pub shadow_color: Rgba,
    pub appears_in_reflections: Confirmed<bool>,
    pub reflection_intensity: Confirmed<f64>,
    pub reflection_sharpness: Confirmed<f64>,
    pub reflection_rolloff: Confirmed<f64>,
}

impl Material {
    pub const fn new() -> Self {
        Self {
            cast_shadows: CastShadows::Off,
            accept_shadows: AcceptShadows::On,
            accepts_lights: Confirmed::Unconfirmed,
            light_transmission_percent: Confirmed::Unconfirmed,
            ambient_percent: Confirmed::Unconfirmed,
            diffuse_percent: Confirmed::Unconfirmed,
            specular_percent: Confirmed::Unconfirmed,
            shininess_percent: Confirmed::Unconfirmed,
            metal_percent: Confirmed::Unconfirmed,
            shadow_color: Rgba::black(),
            appears_in_reflections: Confirmed::Unconfirmed,
            reflection_intensity: Confirmed::Unconfirmed,
            reflection_sharpness: Confirmed::Unconfirmed,
            reflection_rolloff: Confirmed::Unconfirmed,
        }
    }
}

impl Default for Material {
    fn default() -> Self {
        Self::new()
    }
}

pub const fn shininess_editable(specular_percent: f64) -> bool {
    specular_percent > 0.0
}

/// Alt+Shift+C はオンとオフを入れ替える。「シャドウのみ」からの結果は要確認。
pub const fn toggle_cast_shadows(current: CastShadows) -> Confirmed<CastShadows> {
    match current {
        CastShadows::Off => Confirmed::Known(CastShadows::On),
        CastShadows::On => Confirmed::Known(CastShadows::Off),
        CastShadows::Only => Confirmed::Unconfirmed,
    }
}

pub const fn material_keyframes() -> Keyframes {
    Keyframes::Unconfirmed
}

pub const fn caster_layer_visible(mode: CastShadows) -> bool {
    !matches!(mode, CastShadows::Only)
}

pub const fn caster_emits_shadow(mode: CastShadows) -> bool {
    !matches!(mode, CastShadows::Off)
}

pub const fn receiver_layer_visible(mode: AcceptShadows) -> bool {
    !matches!(mode, AcceptShadows::Only)
}

pub const fn receiver_shows_shadow(mode: AcceptShadows) -> bool {
    !matches!(mode, AcceptShadows::Off)
}

/// 0% は黒、100% は落とすレイヤーの色、50% はその中間。
pub fn transmitted_shadow_color(
    red: f64,
    green: f64,
    blue: f64,
    transmission_percent: f64,
) -> [f64; 3] {
    let scale = transmission_percent / 100.0;
    [red * scale, green * scale, blue * scale]
}

pub fn shadow_with_darkness(base: [f64; 3], shadow: [f64; 3], darkness_percent: f64) -> [f64; 3] {
    let amount = darkness_percent / 100.0;
    [
        base[0] + (shadow[0] - base[0]) * amount,
        base[1] + (shadow[1] - base[1]) * amount,
        base[2] + (shadow[2] - base[2]) * amount,
    ]
}

/// 拡散 0 は縁が硬い。大きいほど、落とすレイヤーからの見かけの距離に応じて柔らかい。
pub fn shadow_edge_softness(diffusion_px: f64, apparent_distance_px: f64) -> f64 {
    if diffusion_px <= 0.0 {
        0.0
    } else {
        diffusion_px * apparent_distance_px.max(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        cast_shadow_only_on_advanced, caster_emits_shadow, caster_layer_visible,
        material_available, receiver_layer_visible, receiver_shows_shadow, shadow_edge_softness,
        shadow_with_darkness, shininess_editable, toggle_cast_shadows, transmitted_shadow_color,
        AcceptShadows, CastShadows, Confirmed, Material, MaterialSlot,
    };
    use crate::math::close;
    use crate::renderer::Renderer;
    use model::Rgba;

    #[test]
    fn new_material_does_not_cast_and_does_accept_shadows() {
        let material = Material::new();
        assert_eq!(material.cast_shadows, CastShadows::Off);
        assert!(!caster_emits_shadow(material.cast_shadows));
        assert_eq!(material.accept_shadows, AcceptShadows::On);
        assert!(receiver_shows_shadow(AcceptShadows::On));
        assert!(!receiver_shows_shadow(AcceptShadows::Off));
        assert!(!receiver_layer_visible(AcceptShadows::Only));
        assert!(receiver_shows_shadow(AcceptShadows::Only));
        assert!(!caster_layer_visible(CastShadows::Only));
        assert!(caster_emits_shadow(CastShadows::Only));
        assert_eq!(
            toggle_cast_shadows(CastShadows::Off),
            Confirmed::Known(CastShadows::On)
        );
        assert!(cast_shadow_only_on_advanced().is_unconfirmed());
    }

    #[test]
    fn light_transmission_mixes_black_and_the_caster() {
        let black = transmitted_shadow_color(0.8, 0.4, 0.2, 0.0);
        let full = transmitted_shadow_color(0.8, 0.4, 0.2, 100.0);
        let mid = transmitted_shadow_color(0.8, 0.4, 0.2, 50.0);
        assert!(close(black[0], 0.0));
        assert!(close(full[0], 0.8));
        assert!(close(mid[0], 0.4));
        assert!(!caster_layer_visible(CastShadows::Only));
        assert!(close(transmitted_shadow_color(0.8, 0.0, 0.0, 50.0)[0], 0.4));
        assert_eq!(
            material_available(MaterialSlot::LightTransmission, Renderer::Advanced3d),
            Confirmed::Unconfirmed
        );
    }

    #[test]
    fn accept_shadows_defaults_on_and_only_keeps_the_shadow() {
        assert_eq!(Material::new().accept_shadows, AcceptShadows::On);
        assert!(!receiver_shows_shadow(AcceptShadows::Off));
        assert!(!receiver_layer_visible(AcceptShadows::Only));
        assert!(receiver_shows_shadow(AcceptShadows::Only));
    }

    #[test]
    fn accepts_lights_is_available_on_every_renderer() {
        for renderer in [
            Renderer::Classic3d,
            Renderer::Advanced3d,
            Renderer::Cinema4d,
        ] {
            assert_eq!(
                material_available(MaterialSlot::AcceptLights, renderer),
                Confirmed::Known(true)
            );
        }
        assert!(Material::new().accepts_lights.is_unconfirmed());
    }

    #[test]
    fn ambient_diffuse_specular_and_metal_exist_for_classic() {
        for slot in [
            MaterialSlot::Ambient,
            MaterialSlot::Diffuse,
            MaterialSlot::SpecularIntensity,
            MaterialSlot::SpecularShininess,
            MaterialSlot::Metal,
        ] {
            assert_eq!(
                material_available(slot, Renderer::Classic3d),
                Confirmed::Known(true)
            );
        }
    }

    #[test]
    fn shininess_is_locked_when_specular_is_zero() {
        assert!(!shininess_editable(0.0));
        assert!(shininess_editable(1.0));
    }

    #[test]
    fn shadow_color_defaults_to_black_on_advanced_and_cinema() {
        assert_eq!(Material::new().shadow_color, Rgba::black());
        assert_eq!(
            material_available(MaterialSlot::ShadowColor, Renderer::Advanced3d),
            Confirmed::Known(true)
        );
        assert_eq!(
            material_available(MaterialSlot::ShadowColor, Renderer::Cinema4d),
            Confirmed::Known(true)
        );
        assert!(
            material_available(MaterialSlot::ShadowColor, Renderer::Classic3d).is_unconfirmed()
        );
    }

    #[test]
    fn reflection_slots_exist_only_for_cinema() {
        for slot in [
            MaterialSlot::AppearsInReflections,
            MaterialSlot::ReflectionIntensity,
            MaterialSlot::ReflectionSharpness,
            MaterialSlot::ReflectionRolloff,
        ] {
            assert_eq!(
                material_available(slot, Renderer::Cinema4d),
                Confirmed::Known(true)
            );
            assert_eq!(
                material_available(slot, Renderer::Advanced3d),
                Confirmed::Known(false)
            );
            assert_eq!(
                material_available(slot, Renderer::Classic3d),
                Confirmed::Known(false)
            );
        }
    }

    #[test]
    fn lower_shadow_darkness_is_thinner() {
        let strong = shadow_with_darkness([1.0, 1.0, 1.0], [0.0, 0.0, 0.0], 100.0);
        let thin = shadow_with_darkness([1.0, 1.0, 1.0], [0.0, 0.0, 0.0], 25.0);
        assert!(close(strong[0], 0.0));
        assert!(thin[0] > strong[0]);
    }

    #[test]
    fn zero_diffusion_is_a_hard_edge() {
        assert_eq!(shadow_edge_softness(0.0, 40.0), 0.0);
        assert!(shadow_edge_softness(8.0, 40.0) > shadow_edge_softness(2.0, 40.0));
        assert!(shadow_edge_softness(4.0, 30.0) > shadow_edge_softness(4.0, 10.0));
    }
}
