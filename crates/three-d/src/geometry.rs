use crate::confirmed::{Confirmed, Keyframes, RangeError};
use crate::math::{cross, dot, normalize};
use crate::renderer::Renderer;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BevelStyle {
    None,
    Angular,
    Concave,
    Convex,
}

impl Default for BevelStyle {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BevelBulge {
    None,
    Chamfer,
    Inward,
    Outward,
}

pub const fn bevel_bulge(style: BevelStyle) -> BevelBulge {
    match style {
        BevelStyle::None => BevelBulge::None,
        BevelStyle::Angular => BevelBulge::Chamfer,
        BevelStyle::Concave => BevelBulge::Inward,
        BevelStyle::Convex => BevelBulge::Outward,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GeometryTarget {
    Text,
    Shape,
    Footage,
    Precomp,
}

impl GeometryTarget {
    pub const fn accepts_extrusion(self) -> bool {
        match self {
            Self::Text | Self::Shape => true,
            Self::Footage | Self::Precomp => false,
        }
    }

    pub const fn accepts_curvature(self) -> bool {
        match self {
            Self::Footage | Self::Precomp => true,
            Self::Text | Self::Shape => false,
        }
    }
}

pub const fn extrusion_depth(renderer: Renderer, target: GeometryTarget, depth_px: f64) -> f64 {
    if renderer.extrusion_enabled() && target.accepts_extrusion() {
        depth_px
    } else {
        0.0
    }
}

pub fn side_is_perpendicular(front_normal: [f64; 3], edge: [f64; 3]) -> bool {
    let Some(side) = normalize(cross(edge, front_normal)) else {
        return false;
    };
    dot(side, front_normal).abs() <= 1e-9
}

pub fn hole_bevel_depth(outer_depth_px: f64, percent: f64) -> f64 {
    outer_depth_px * percent / 100.0
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Curvature(f64);

impl Curvature {
    pub const DEFAULT: f64 = 0.0;

    pub fn new(percent: f64) -> Result<Self, RangeError> {
        if (-100.0..=100.0).contains(&percent) {
            Ok(Self(percent))
        } else {
            Err(RangeError)
        }
    }

    pub const fn percent(self) -> f64 {
        self.0
    }

    pub fn sign(self) -> i32 {
        if self.0 > 0.0 {
            1
        } else if self.0 < 0.0 {
            -1
        } else {
            0
        }
    }

    pub const fn keyframes() -> Keyframes {
        Keyframes::Allowed
    }
}

impl Default for Curvature {
    fn default() -> Self {
        Self(Self::DEFAULT)
    }
}

pub const fn curvature_replaces_text_extrusion() -> bool {
    false
}

/// 面ひとつが全体の曲げに占める割合。総角度は要確認なので割合だけを返す。
pub fn facet_fraction(segments: u32) -> Option<f64> {
    if segments == 0 {
        None
    } else {
        Some(1.0 / f64::from(segments))
    }
}

pub const fn segments_initial() -> Confirmed<u32> {
    Confirmed::Unconfirmed
}

pub const fn segments_keyframes() -> Keyframes {
    Keyframes::Allowed
}

pub const fn masks_affect_extrusion() -> bool {
    false
}

pub const fn effects_affect_extrusion() -> bool {
    false
}

pub const fn track_mattes_affect_extrusion() -> bool {
    false
}

pub const fn masks_affect_curvature() -> bool {
    true
}

pub const fn effects_affect_curvature() -> bool {
    true
}

pub const fn extrusion_preserves_text_metrics() -> bool {
    true
}

pub const fn bevel_depth_initial() -> Confirmed<f64> {
    Confirmed::Unconfirmed
}

pub const fn hole_bevel_initial() -> Confirmed<f64> {
    Confirmed::Unconfirmed
}

pub const fn extrusion_depth_initial() -> Confirmed<f64> {
    Confirmed::Unconfirmed
}

#[cfg(test)]
mod tests {
    use super::{
        bevel_bulge, bevel_depth_initial, curvature_replaces_text_extrusion,
        effects_affect_curvature, effects_affect_extrusion, extrusion_depth,
        extrusion_depth_initial, extrusion_preserves_text_metrics, facet_fraction,
        hole_bevel_depth, hole_bevel_initial, masks_affect_curvature, masks_affect_extrusion,
        segments_initial, side_is_perpendicular, track_mattes_affect_extrusion, BevelBulge,
        BevelStyle, Curvature, GeometryTarget,
    };
    use crate::renderer::Renderer;

    #[test]
    fn bevel_style_defaults_to_none_and_changes_the_profile() {
        assert_eq!(BevelStyle::default(), BevelStyle::None);
        assert_eq!(bevel_bulge(BevelStyle::None), BevelBulge::None);
        assert_ne!(
            bevel_bulge(BevelStyle::Angular),
            bevel_bulge(BevelStyle::Concave)
        );
        assert_ne!(
            bevel_bulge(BevelStyle::Concave),
            bevel_bulge(BevelStyle::Convex)
        );
        assert_eq!(
            extrusion_depth(Renderer::Classic3d, GeometryTarget::Text, 20.0),
            0.0
        );
        assert!(extrusion_depth(Renderer::Advanced3d, GeometryTarget::Text, 20.0) > 0.0);
    }

    #[test]
    fn bevel_depth_grows_the_edge_and_zero_is_flat() {
        assert!(bevel_depth_initial().is_unconfirmed());
        assert_eq!(hole_bevel_depth(0.0, 50.0), 0.0);
        assert!(hole_bevel_depth(12.0, 100.0) > hole_bevel_depth(4.0, 100.0));
        assert_ne!(bevel_bulge(BevelStyle::Angular), BevelBulge::None);
    }

    #[test]
    fn hole_bevel_changes_only_the_inner_depth() {
        let outer = 20.0;
        let inner = hole_bevel_depth(outer, 25.0);
        assert!(inner < outer);
        assert_eq!(hole_bevel_depth(outer, 50.0) / outer, 0.5);
        assert!(hole_bevel_initial().is_unconfirmed());
    }

    #[test]
    fn extrusion_depth_is_perpendicular_on_advanced_and_absent_on_classic_and_footage() {
        assert!(extrusion_depth_initial().is_unconfirmed());
        assert_eq!(
            extrusion_depth(Renderer::Classic3d, GeometryTarget::Text, 30.0),
            0.0
        );
        assert_eq!(
            extrusion_depth(Renderer::Advanced3d, GeometryTarget::Text, 30.0),
            30.0
        );
        assert_eq!(
            extrusion_depth(Renderer::Advanced3d, GeometryTarget::Footage, 30.0),
            0.0
        );
        assert!(side_is_perpendicular([0.0, 0.0, 1.0], [1.0, 0.0, 0.0]));
        assert!(!masks_affect_extrusion());
        assert!(!effects_affect_extrusion());
        assert!(!track_mattes_affect_extrusion());
        assert!(extrusion_preserves_text_metrics());
    }

    #[test]
    fn curvature_defaults_to_zero_and_rejects_out_of_range() {
        assert_eq!(Curvature::default().percent(), 0.0);
        assert_eq!(Curvature::default().sign(), 0);
        assert_eq!(Curvature::new(100.0).unwrap().sign(), 1);
        assert_eq!(Curvature::new(-100.0).unwrap().sign(), -1);
        assert!(Curvature::new(100.1).is_err());
        assert!(Curvature::new(-100.1).is_err());
        assert!(!curvature_replaces_text_extrusion());
        assert!(!GeometryTarget::Text.accepts_curvature());
        assert!(GeometryTarget::Footage.accepts_curvature());
        assert!(masks_affect_curvature());
        assert!(effects_affect_curvature());
    }

    #[test]
    fn fewer_segments_make_larger_facets() {
        assert!(segments_initial().is_unconfirmed());
        assert!(facet_fraction(4).unwrap() > facet_fraction(16).unwrap());
    }
}
