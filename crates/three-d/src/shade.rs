use crate::light::SceneLightKind;
use crate::math::{add, dot, scale, sub};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Surface {
    pub color: [f64; 3],
    pub normal: [f64; 3],
    pub view: [f64; 3],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShadeMaterial {
    pub ambient_percent: f64,
    pub diffuse_percent: f64,
    pub specular_percent: f64,
    pub shininess_percent: f64,
    pub metal_percent: f64,
    pub accepts_lights: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShadeLight {
    pub kind: SceneLightKind,
    pub color: [f64; 3],
    pub intensity_percent: f64,
    /// 面から光源へ。アンビエントでは使わない。
    pub direction_to_light: [f64; 3],
    pub cone_factor: f64,
    pub falloff_factor: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShadeParts {
    pub color: [f64; 3],
    pub ambient: [f64; 3],
    pub diffuse: [f64; 3],
    pub specular: [f64; 3],
}

/// ハイライトの相対的な大きさ。100% が小さく、0% が大きい。内側の曲線は要確認。
pub fn highlight_radius(shininess_percent: f64) -> f64 {
    (100.0 - shininess_percent) / 100.0 * 2.0
}

pub fn shade(surface: Surface, material: ShadeMaterial, lights: &[ShadeLight]) -> ShadeParts {
    if !material.accepts_lights {
        return ShadeParts {
            color: surface.color,
            ambient: [0.0, 0.0, 0.0],
            diffuse: [0.0, 0.0, 0.0],
            specular: [0.0, 0.0, 0.0],
        };
    }
    let mut ambient = [0.0, 0.0, 0.0];
    let mut diffuse = [0.0, 0.0, 0.0];
    let mut specular = [0.0, 0.0, 0.0];
    for light in lights {
        let signed = light.intensity_percent / 100.0;
        if light.kind == SceneLightKind::Ambient {
            let gain = material.ambient_percent / 100.0 * signed;
            ambient = add(ambient, scale(light.color, gain));
            continue;
        }
        if matches!(light.kind, SceneLightKind::Environment) {
            continue;
        }
        let lambert = dot(surface.normal, light.direction_to_light).max(0.0);
        let diffuse_gain = material.diffuse_percent / 100.0
            * lambert
            * light.cone_factor
            * light.falloff_factor
            * signed;
        diffuse = add(diffuse, scale(light.color, diffuse_gain));
        if material.specular_percent > 0.0 {
            let weight = specular_weight(
                surface.normal,
                light.direction_to_light,
                surface.view,
                material.shininess_percent,
            );
            let metal = material.metal_percent / 100.0;
            let highlight = add(scale(light.color, 1.0 - metal), scale(surface.color, metal));
            let spec_gain = material.specular_percent / 100.0
                * weight
                * light.cone_factor
                * light.falloff_factor
                * signed;
            specular = add(specular, scale(highlight, spec_gain));
        }
    }
    let lit = add(ambient, diffuse);
    let body = [
        surface.color[0] * lit[0],
        surface.color[1] * lit[1],
        surface.color[2] * lit[2],
    ];
    ShadeParts {
        color: add(body, specular),
        ambient,
        diffuse,
        specular,
    }
}

fn specular_weight(
    normal: [f64; 3],
    direction_to_light: [f64; 3],
    view: [f64; 3],
    shininess: f64,
) -> f64 {
    let reflect = sub(
        scale(normal, 2.0 * dot(normal, direction_to_light)),
        direction_to_light,
    );
    let deviation = 1.0 - dot(reflect, view).clamp(-1.0, 1.0);
    let radius = highlight_radius(shininess);
    if deviation <= radius {
        1.0
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::{highlight_radius, shade, ShadeLight, ShadeMaterial, Surface};
    use crate::light::SceneLightKind;
    use crate::math::close;

    fn white_light(kind: SceneLightKind, intensity: f64, direction: [f64; 3]) -> ShadeLight {
        ShadeLight {
            kind,
            color: [1.0, 0.0, 0.0],
            intensity_percent: intensity,
            direction_to_light: direction,
            cone_factor: 1.0,
            falloff_factor: 1.0,
        }
    }

    fn material(
        ambient: f64,
        diffuse: f64,
        specular: f64,
        shininess: f64,
        metal: f64,
    ) -> ShadeMaterial {
        ShadeMaterial {
            ambient_percent: ambient,
            diffuse_percent: diffuse,
            specular_percent: specular,
            shininess_percent: shininess,
            metal_percent: metal,
            accepts_lights: true,
        }
    }

    #[test]
    fn accepts_lights_off_keeps_the_base_color() {
        let surface = Surface {
            color: [0.2, 0.4, 0.6],
            normal: [0.0, 0.0, 1.0],
            view: [0.0, 0.0, 1.0],
        };
        let mut mat = material(100.0, 100.0, 100.0, 50.0, 0.0);
        mat.accepts_lights = false;
        let lit = shade(
            surface,
            mat,
            &[white_light(
                SceneLightKind::Parallel,
                100.0,
                [0.0, 0.0, 1.0],
            )],
        );
        assert!(close(lit.color[0], 0.2));
        assert!(close(lit.color[1], 0.4));
        let changed = shade(
            surface,
            material(0.0, 100.0, 0.0, 0.0, 0.0),
            &[white_light(
                SceneLightKind::Parallel,
                100.0,
                [0.0, 0.0, 1.0],
            )],
        );
        assert!(!close(changed.color[1], surface.color[1]));
    }

    #[test]
    fn ambient_term_ignores_orientation() {
        let light = ShadeLight {
            kind: SceneLightKind::Ambient,
            color: [0.0, 1.0, 0.0],
            intensity_percent: 100.0,
            direction_to_light: [0.0, 0.0, 1.0],
            cone_factor: 1.0,
            falloff_factor: 1.0,
        };
        let mat = material(100.0, 0.0, 0.0, 0.0, 0.0);
        let up = shade(
            Surface {
                color: [1.0, 1.0, 1.0],
                normal: [0.0, 1.0, 0.0],
                view: [0.0, 0.0, 1.0],
            },
            mat,
            &[light],
        );
        let down = shade(
            Surface {
                color: [1.0, 1.0, 1.0],
                normal: [0.0, -1.0, 0.0],
                view: [0.0, 0.0, 1.0],
            },
            mat,
            &[light],
        );
        assert!(close(up.ambient[1], down.ambient[1]));
        let none = material(0.0, 0.0, 0.0, 0.0, 0.0);
        assert!(close(
            shade(
                Surface {
                    color: [1.0, 1.0, 1.0],
                    normal: [0.0, 0.0, 1.0],
                    view: [0.0, 0.0, 1.0]
                },
                none,
                &[light]
            )
            .ambient[1],
            0.0
        ));
        assert!(close(up.ambient[1], 1.0));
    }

    #[test]
    fn diffuse_tracks_the_surface_when_enabled() {
        let facing = shade(
            Surface {
                color: [1.0, 1.0, 1.0],
                normal: [0.0, 0.0, 1.0],
                view: [0.0, 0.0, 1.0],
            },
            material(0.0, 100.0, 0.0, 0.0, 0.0),
            &[white_light(
                SceneLightKind::Parallel,
                100.0,
                [0.0, 0.0, 1.0],
            )],
        );
        let away = shade(
            Surface {
                color: [1.0, 1.0, 1.0],
                normal: [0.0, 0.0, -1.0],
                view: [0.0, 0.0, 1.0],
            },
            material(0.0, 100.0, 0.0, 0.0, 0.0),
            &[white_light(
                SceneLightKind::Parallel,
                100.0,
                [0.0, 0.0, 1.0],
            )],
        );
        let off = shade(
            Surface {
                color: [1.0, 1.0, 1.0],
                normal: [0.0, 0.0, 1.0],
                view: [0.0, 0.0, 1.0],
            },
            material(0.0, 0.0, 0.0, 0.0, 0.0),
            &[white_light(
                SceneLightKind::Parallel,
                100.0,
                [0.0, 0.0, 1.0],
            )],
        );
        assert!(facing.diffuse[0] > away.diffuse[0]);
        assert!(close(off.diffuse[0], 0.0));
    }

    #[test]
    fn specular_appears_only_above_zero_and_shrinks_at_100_percent() {
        assert!(highlight_radius(100.0) < highlight_radius(0.0));
        let none = shade(
            Surface {
                color: [1.0, 1.0, 1.0],
                normal: [0.0, 0.0, 1.0],
                view: [0.0, 0.0, 1.0],
            },
            material(0.0, 0.0, 0.0, 0.0, 0.0),
            &[white_light(
                SceneLightKind::Parallel,
                100.0,
                [0.0, 0.0, 1.0],
            )],
        );
        let some = shade(
            Surface {
                color: [1.0, 1.0, 1.0],
                normal: [0.0, 0.0, 1.0],
                view: [0.0, 0.0, 1.0],
            },
            material(0.0, 0.0, 100.0, 100.0, 0.0),
            &[white_light(
                SceneLightKind::Parallel,
                100.0,
                [0.0, 0.0, 1.0],
            )],
        );
        assert!(close(none.specular[0], 0.0));
        assert!(some.specular[0] > 0.0);
        assert!(!crate::material::shininess_editable(0.0));
    }

    #[test]
    fn metal_mixes_the_layer_color_into_the_highlight() {
        let surface = Surface {
            color: [0.2, 0.4, 0.0],
            normal: [0.0, 0.0, 1.0],
            view: [0.0, 0.0, 1.0],
        };
        let light = ShadeLight {
            kind: SceneLightKind::Parallel,
            color: [1.0, 1.0, 1.0],
            intensity_percent: 100.0,
            direction_to_light: [0.0, 0.0, 1.0],
            cone_factor: 1.0,
            falloff_factor: 1.0,
        };
        let dielectric = shade(surface, material(0.0, 0.0, 100.0, 100.0, 0.0), &[light]);
        let metal = shade(surface, material(0.0, 0.0, 100.0, 100.0, 100.0), &[light]);
        assert!(close(dielectric.specular[0], 1.0));
        assert!(close(metal.specular[0], 0.2));
        assert!(close(metal.specular[1], 0.4));
    }
}
