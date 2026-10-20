use model::LightKind;

use crate::confirmed::{Confirmed, Keyframes};
use crate::math::composition_center;

/// モデルの 4 種類に、Advanced 3D の環境光を足す。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SceneLightKind {
    Parallel,
    Spot,
    Point,
    Ambient,
    Environment,
}

impl SceneLightKind {
    pub const fn from_model(kind: LightKind) -> Self {
        match kind {
            LightKind::Parallel => Self::Parallel,
            LightKind::Spot => Self::Spot,
            LightKind::Point => Self::Point,
            LightKind::Ambient => Self::Ambient,
        }
    }

    pub const fn to_model(self) -> Option<LightKind> {
        match self {
            Self::Parallel => Some(LightKind::Parallel),
            Self::Spot => Some(LightKind::Spot),
            Self::Point => Some(LightKind::Point),
            Self::Ambient => Some(LightKind::Ambient),
            Self::Environment => None,
        }
    }

    pub const fn cone_editable(self) -> bool {
        matches!(self, Self::Spot)
    }

    pub const fn falloff_applies(self) -> bool {
        match self {
            Self::Parallel | Self::Spot | Self::Point => true,
            Self::Ambient | Self::Environment => false,
        }
    }

    /// スクリプトの lightType。環境光は ENVIRONMENT。
    pub const fn script_type(self) -> &'static str {
        match self {
            Self::Parallel => "PARALLEL",
            Self::Spot => "SPOT",
            Self::Point => "POINT",
            Self::Ambient => "AMBIENT",
            Self::Environment => "ENVIRONMENT",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Falloff {
    None,
    Smooth,
    InverseSquareClamped,
}

impl Falloff {
    pub const ALL: [Self; 3] = [Self::None, Self::Smooth, Self::InverseSquareClamped];

    pub const fn radius_editable(self) -> bool {
        !matches!(self, Self::None)
    }

    pub const fn distance_editable(self) -> Confirmed<bool> {
        match self {
            Self::None => Confirmed::Known(false),
            Self::Smooth => Confirmed::Known(true),
            Self::InverseSquareClamped => Confirmed::Unconfirmed,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AutoOrient {
    TowardPointOfInterest,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NameEditSource {
    Dialog,
    Timeline,
}

pub fn spot_name(number: u32) -> String {
    format!("スポットライト {number}")
}

pub fn point_name(number: u32) -> String {
    format!("ポイントライト {number}")
}

pub fn generated_name(kind: SceneLightKind, number: u32) -> Confirmed<String> {
    match kind {
        SceneLightKind::Spot => Confirmed::Known(spot_name(number)),
        SceneLightKind::Point
        | SceneLightKind::Parallel
        | SceneLightKind::Ambient
        | SceneLightKind::Environment => Confirmed::Unconfirmed,
    }
}

pub fn rename_for_kind_change(
    current: &str,
    generated: bool,
    source: NameEditSource,
    new_kind: SceneLightKind,
    number: u32,
) -> Confirmed<String> {
    if !generated || matches!(source, NameEditSource::Timeline) {
        return Confirmed::Known(current.to_string());
    }
    match new_kind {
        SceneLightKind::Spot => Confirmed::Known(spot_name(number)),
        SceneLightKind::Point => Confirmed::Known(point_name(number)),
        SceneLightKind::Parallel | SceneLightKind::Ambient | SceneLightKind::Environment => {
            Confirmed::Unconfirmed
        }
    }
}

/// 距離による減衰。なしは一定。スムーズは開始半径から距離だけ直線で 0 になる。
/// 逆二乗クランプは半径の内側を一定にし、外側を (半径 / 距離)^2 にする。
pub fn falloff_factor(kind: Falloff, distance: f64, radius: f64, falloff_distance: f64) -> f64 {
    if distance <= radius {
        return 1.0;
    }
    match kind {
        Falloff::None => 1.0,
        Falloff::Smooth => {
            if falloff_distance <= 0.0 {
                0.0
            } else {
                (1.0 - (distance - radius) / falloff_distance).max(0.0)
            }
        }
        Falloff::InverseSquareClamped => {
            let ratio = radius / distance;
            ratio * ratio
        }
    }
}

/// 円錐の外は 0。円錐ぼかしは頂角のうち縁の割合をなだらかにする。
pub fn cone_factor(angle_from_axis_deg: f64, cone_angle_deg: f64, feather_percent: f64) -> f64 {
    let half = cone_angle_deg * 0.5;
    if angle_from_axis_deg > half {
        return 0.0;
    }
    if feather_percent <= 0.0 {
        return 1.0;
    }
    let inner = half * (1.0 - feather_percent / 100.0);
    if angle_from_axis_deg <= inner {
        1.0
    } else if (half - inner).abs() <= f64::EPSILON {
        0.0
    } else {
        1.0 - (angle_from_axis_deg - inner) / (half - inner)
    }
}

#[allow(clippy::too_many_arguments)]
pub fn illuminance(
    kind: SceneLightKind,
    intensity_percent: f64,
    falloff: Falloff,
    distance: f64,
    radius: f64,
    falloff_distance: f64,
    angle_from_axis_deg: f64,
    cone_angle_deg: f64,
    cone_feather_percent: f64,
) -> f64 {
    let signed = intensity_percent / 100.0;
    match kind {
        SceneLightKind::Ambient => signed,
        SceneLightKind::Environment => 0.0,
        SceneLightKind::Parallel => {
            signed * falloff_factor(falloff, distance, radius, falloff_distance)
        }
        SceneLightKind::Point => {
            signed * falloff_factor(falloff, distance, radius, falloff_distance)
        }
        SceneLightKind::Spot => {
            signed
                * cone_factor(angle_from_axis_deg, cone_angle_deg, cone_feather_percent)
                * falloff_factor(falloff, distance, radius, falloff_distance)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LightField {
    Kind,
    Color,
    Intensity,
    ConeAngle,
    ConeFeather,
    Falloff,
    Radius,
    FalloffDistance,
    CastShadows,
    ShadowDarkness,
    ShadowDiffusion,
    PointOfInterest,
    Position,
    Direction,
    Rotation,
    AutoOrient,
}

pub const fn light_field_keyframes(field: LightField) -> Keyframes {
    match field {
        LightField::Kind | LightField::CastShadows => Keyframes::Forbidden,
        LightField::Color
        | LightField::Intensity
        | LightField::ConeAngle
        | LightField::ConeFeather
        | LightField::Falloff
        | LightField::Radius
        | LightField::FalloffDistance
        | LightField::ShadowDarkness
        | LightField::ShadowDiffusion => Keyframes::Allowed,
        LightField::PointOfInterest
        | LightField::Position
        | LightField::Direction
        | LightField::Rotation
        | LightField::AutoOrient => Keyframes::Unconfirmed,
    }
}

pub const fn environment_shadow_darkness_keyframes() -> Keyframes {
    Keyframes::Unconfirmed
}

pub fn nudge_intensity_percent(shift: bool) -> f64 {
    if shift {
        10.0
    } else {
        1.0
    }
}

pub const fn interest_steers(orient: AutoOrient) -> bool {
    matches!(orient, AutoOrient::TowardPointOfInterest)
}

pub const fn z_rotation_nudge_applies(kind: SceneLightKind) -> Confirmed<bool> {
    match kind {
        SceneLightKind::Parallel | SceneLightKind::Spot => Confirmed::Known(true),
        SceneLightKind::Point | SceneLightKind::Ambient | SceneLightKind::Environment => {
            Confirmed::Unconfirmed
        }
    }
}

pub const fn point_of_interest_affects_point_distribution() -> Confirmed<bool> {
    Confirmed::Unconfirmed
}

pub const fn point_direction_affects_illuminance() -> Confirmed<bool> {
    Confirmed::Unconfirmed
}

pub const fn point_rotation_affects_illuminance() -> Confirmed<bool> {
    Confirmed::Unconfirmed
}

pub const fn ambient_position_affects_brightness() -> bool {
    false
}

pub const fn ambient_has_comp_icon() -> bool {
    false
}

pub const fn ambient_position_on_timeline() -> Confirmed<bool> {
    Confirmed::Unconfirmed
}

pub const fn has_comp_icon(kind: SceneLightKind) -> Confirmed<bool> {
    match kind {
        SceneLightKind::Ambient => Confirmed::Known(false),
        SceneLightKind::Parallel | SceneLightKind::Spot | SceneLightKind::Point => {
            Confirmed::Known(true)
        }
        SceneLightKind::Environment => Confirmed::Unconfirmed,
    }
}

/// 調整レイヤーのライトは、タイムラインで自分より下だけを照らす。
pub const fn light_affects_layer(adjustment: bool, light_index: usize, layer_index: usize) -> bool {
    if adjustment {
        layer_index > light_index
    } else {
        true
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Light {
    pub kind: SceneLightKind,
    pub name: Confirmed<String>,
    pub name_generated: bool,
    pub name_number: u32,
    pub preview: Confirmed<bool>,
    pub adjustment: Confirmed<bool>,
    pub color_known: bool,
    pub intensity_percent: Confirmed<f64>,
    pub cone_angle_degrees: Confirmed<f64>,
    pub cone_feather_percent: Confirmed<f64>,
    pub point_of_interest: [f64; 3],
    pub position: Confirmed<[f64; 3]>,
    pub orientation: Confirmed<[f64; 3]>,
    pub rotation_x: Confirmed<f64>,
    pub rotation_y: Confirmed<f64>,
    pub rotation_z: Confirmed<f64>,
    pub auto_orient: AutoOrient,
    pub falloff: Confirmed<Falloff>,
    pub radius: Confirmed<f64>,
    pub falloff_distance: Confirmed<f64>,
    pub casts_shadows: Confirmed<bool>,
}

impl Light {
    pub fn new(kind: SceneLightKind, width: u32, height: u32, number: u32) -> Self {
        Self {
            kind,
            name: generated_name(kind, number),
            name_generated: true,
            name_number: number,
            preview: Confirmed::Unconfirmed,
            adjustment: Confirmed::Unconfirmed,
            color_known: false,
            intensity_percent: Confirmed::Unconfirmed,
            cone_angle_degrees: Confirmed::Unconfirmed,
            cone_feather_percent: Confirmed::Unconfirmed,
            point_of_interest: composition_center(width, height),
            position: Confirmed::Unconfirmed,
            orientation: Confirmed::Unconfirmed,
            rotation_x: Confirmed::Unconfirmed,
            rotation_y: Confirmed::Unconfirmed,
            rotation_z: Confirmed::Unconfirmed,
            auto_orient: AutoOrient::TowardPointOfInterest,
            falloff: Confirmed::Unconfirmed,
            radius: Confirmed::Unconfirmed,
            falloff_distance: Confirmed::Unconfirmed,
            casts_shadows: match kind {
                SceneLightKind::Environment => Confirmed::Known(false),
                SceneLightKind::Parallel
                | SceneLightKind::Spot
                | SceneLightKind::Point
                | SceneLightKind::Ambient => Confirmed::Unconfirmed,
            },
        }
    }

    pub fn set_kind(&mut self, kind: SceneLightKind, source: NameEditSource) {
        let renamed = rename_for_kind_change(
            self.name.as_ref().known().map(String::as_str).unwrap_or(""),
            self.name_generated,
            source,
            kind,
            self.name_number,
        );
        self.kind = kind;
        if renamed.is_known() || self.name_generated {
            self.name = renamed;
        }
        if kind == SceneLightKind::Environment && self.casts_shadows.is_unconfirmed() {
            self.casts_shadows = Confirmed::Known(false);
        }
    }

    pub fn rename(&mut self, name: impl Into<String>) {
        self.name = Confirmed::Known(name.into());
        self.name_generated = false;
    }

    pub fn add_rotation_z(&mut self, degrees: f64) {
        let base = match self.rotation_z {
            Confirmed::Known(value) => value,
            Confirmed::Unconfirmed => 0.0,
        };
        self.rotation_z = Confirmed::Known(base + degrees);
    }
}

pub const LIGHT_EXCLUDED_PROPERTIES: &[&str] = &[
    "ソース",
    "エフェクト",
    "マスク",
    "幅",
    "高さ",
    "アンカーポイント",
    "スケール",
    "不透明度",
    "オーディオレベル",
    "タイムリマップ",
    "マテリアル",
];

pub const fn new_light_shortcut() -> &'static str {
    "Ctrl+Alt+Shift+L"
}

pub const fn light_preview_keyframes() -> Keyframes {
    Keyframes::Forbidden
}

#[derive(Clone, Debug, PartialEq)]
pub struct LightDialog {
    pub preview: Confirmed<bool>,
    pub shown_intensity: f64,
    pub committed_intensity: f64,
}

impl LightDialog {
    pub fn open(intensity: f64) -> Self {
        Self {
            preview: Confirmed::Unconfirmed,
            shown_intensity: intensity,
            committed_intensity: intensity,
        }
    }

    pub fn set_preview(&mut self, preview: bool) {
        self.preview = Confirmed::Known(preview);
    }

    pub fn edit_intensity(&mut self, intensity: f64) {
        self.shown_intensity = intensity;
        if self.preview == Confirmed::Known(true) {
            self.committed_intensity = intensity;
        }
    }

    pub fn close(mut self) -> f64 {
        self.committed_intensity = self.shown_intensity;
        self.committed_intensity
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ambient_has_comp_icon, ambient_position_affects_brightness, ambient_position_on_timeline,
        cone_factor, environment_shadow_darkness_keyframes, falloff_factor, generated_name,
        has_comp_icon, illuminance, light_affects_layer, light_field_keyframes, new_light_shortcut,
        nudge_intensity_percent, point_direction_affects_illuminance,
        point_of_interest_affects_point_distribution, point_rotation_affects_illuminance,
        rename_for_kind_change, z_rotation_nudge_applies, AutoOrient, Confirmed, Falloff,
        Keyframes, Light, LightDialog, LightField, NameEditSource, SceneLightKind,
    };
    use crate::camera::nudge_degrees;
    use crate::math::close;

    #[allow(clippy::too_many_arguments)]
    fn lit(
        kind: SceneLightKind,
        intensity: f64,
        falloff: Falloff,
        distance: f64,
        radius: f64,
        falloff_distance: f64,
        angle: f64,
        cone: f64,
        feather: f64,
    ) -> f64 {
        illuminance(
            kind,
            intensity,
            falloff,
            distance,
            radius,
            falloff_distance,
            angle,
            cone,
            feather,
        )
    }

    #[test]
    fn new_light_spot_name_survives_only_until_a_hand_edit() {
        assert_eq!(new_light_shortcut(), "Ctrl+Alt+Shift+L");
        let mut light = Light::new(SceneLightKind::Spot, 1920, 1080, 1);
        assert_eq!(light.name, Confirmed::Known("スポットライト 1".to_string()));
        light.set_kind(SceneLightKind::Point, NameEditSource::Dialog);
        assert_eq!(light.name, Confirmed::Known("ポイントライト 1".to_string()));
        light.rename("手元");
        light.set_kind(SceneLightKind::Spot, NameEditSource::Dialog);
        assert_eq!(light.name, Confirmed::Known("手元".to_string()));
    }

    #[test]
    fn light_preview_initial_is_unconfirmed_and_gates_the_picture() {
        let mut dialog = LightDialog::open(100.0);
        assert!(dialog.preview.is_unconfirmed());
        dialog.set_preview(true);
        dialog.edit_intensity(80.0);
        assert!(close(dialog.committed_intensity, 80.0));
        dialog.set_preview(false);
        dialog.edit_intensity(10.0);
        assert!(close(dialog.committed_intensity, 80.0));
        assert!(close(dialog.close(), 10.0));
        assert_eq!(super::light_preview_keyframes(), Keyframes::Forbidden);
    }

    #[test]
    fn adjustment_light_skips_layers_above_it() {
        assert!(!light_affects_layer(true, 1, 0));
        assert!(light_affects_layer(true, 1, 2));
        assert!(light_affects_layer(false, 1, 0));
        assert!(Light::new(SceneLightKind::Spot, 10, 10, 1)
            .adjustment
            .is_unconfirmed());
    }

    #[test]
    fn parallel_disables_cone_and_changes_facing() {
        assert!(!SceneLightKind::Parallel.cone_editable());
        assert!(SceneLightKind::Spot.cone_editable());
        let facing = lit(
            SceneLightKind::Parallel,
            100.0,
            Falloff::None,
            10.0,
            0.0,
            0.0,
            0.0,
            90.0,
            0.0,
        );
        let same = lit(
            SceneLightKind::Parallel,
            100.0,
            Falloff::None,
            80.0,
            0.0,
            0.0,
            40.0,
            90.0,
            0.0,
        );
        assert!(close(facing, same));
        assert_eq!(
            light_field_keyframes(LightField::Kind),
            Keyframes::Forbidden
        );
    }

    #[test]
    fn parallel_generated_name_is_unconfirmed_but_edits_stick() {
        assert!(generated_name(SceneLightKind::Parallel, 1).is_unconfirmed());
        let kept = rename_for_kind_change(
            "手元",
            false,
            NameEditSource::Dialog,
            SceneLightKind::Spot,
            1,
        );
        assert_eq!(kept, Confirmed::Known("手元".to_string()));
        let generated = rename_for_kind_change(
            "スポットライト 1",
            true,
            NameEditSource::Dialog,
            SceneLightKind::Point,
            1,
        );
        assert_eq!(generated, Confirmed::Known("ポイントライト 1".to_string()));
    }

    #[test]
    fn parallel_color_can_be_keyed() {
        assert_eq!(light_field_keyframes(LightField::Color), Keyframes::Allowed);
    }

    #[test]
    fn parallel_intensity_signs() {
        assert!(close(
            lit(
                SceneLightKind::Parallel,
                0.0,
                Falloff::None,
                1.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0
            ),
            0.0
        ));
        assert!(
            lit(
                SceneLightKind::Parallel,
                50.0,
                Falloff::None,
                1.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0
            ) > 0.0
        );
        assert!(
            lit(
                SceneLightKind::Parallel,
                -50.0,
                Falloff::None,
                1.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0
            ) < 0.0
        );
        assert!(close(nudge_intensity_percent(false), 1.0));
        assert!(close(nudge_intensity_percent(true), 10.0));
        assert_eq!(
            light_field_keyframes(LightField::Intensity),
            Keyframes::Allowed
        );
    }

    #[test]
    fn parallel_interest_starts_at_center() {
        let light = Light::new(SceneLightKind::Parallel, 200, 100, 1);
        assert!(close(light.point_of_interest[0], 100.0));
        assert!(close(light.point_of_interest[1], 50.0));
        assert!(close(light.point_of_interest[2], 0.0));
        assert_eq!(light.auto_orient, AutoOrient::TowardPointOfInterest);
    }

    #[test]
    fn parallel_without_falloff_ignores_position() {
        let near = lit(
            SceneLightKind::Parallel,
            100.0,
            Falloff::None,
            5.0,
            10.0,
            20.0,
            0.0,
            0.0,
            0.0,
        );
        let far = lit(
            SceneLightKind::Parallel,
            100.0,
            Falloff::None,
            500.0,
            10.0,
            20.0,
            0.0,
            0.0,
            0.0,
        );
        assert!(close(near, far));
        let faded = lit(
            SceneLightKind::Parallel,
            100.0,
            Falloff::Smooth,
            30.0,
            10.0,
            20.0,
            0.0,
            0.0,
            0.0,
        );
        assert!(faded < near);
    }

    #[test]
    fn parallel_other_auto_orient_ignores_interest() {
        assert!(super::interest_steers(AutoOrient::TowardPointOfInterest));
        assert!(!super::interest_steers(AutoOrient::Other));
    }

    #[test]
    fn parallel_x_rotation_is_independent() {
        let mut light = Light::new(SceneLightKind::Parallel, 10, 10, 1);
        light.rotation_x = Confirmed::Known(0.0);
        light.rotation_y = Confirmed::Known(3.0);
        light.rotation_z = Confirmed::Known(4.0);
        light.rotation_x = Confirmed::Known(light.rotation_x.known().unwrap() + 9.0);
        assert_eq!(light.rotation_x, Confirmed::Known(9.0));
        assert_eq!(light.rotation_y, Confirmed::Known(3.0));
        assert_eq!(light.rotation_z, Confirmed::Known(4.0));
    }

    #[test]
    fn parallel_y_rotation_is_independent() {
        let mut light = Light::new(SceneLightKind::Parallel, 10, 10, 1);
        light.rotation_x = Confirmed::Known(1.0);
        light.rotation_y = Confirmed::Known(2.0);
        light.rotation_z = Confirmed::Known(3.0);
        light.rotation_y = Confirmed::Known(6.0);
        assert_eq!(light.rotation_y, Confirmed::Known(6.0));
        assert_eq!(light.rotation_x, Confirmed::Known(1.0));
    }

    #[test]
    fn parallel_z_rotation_nudge_is_1_and_10_degrees() {
        let mut light = Light::new(SceneLightKind::Parallel, 10, 10, 1);
        light.rotation_z = Confirmed::Known(0.0);
        light.add_rotation_z(nudge_degrees(false));
        assert!(close(light.rotation_z.known().unwrap(), 1.0));
        light.add_rotation_z(nudge_degrees(true));
        assert!(close(light.rotation_z.known().unwrap(), 11.0));
        assert_eq!(
            z_rotation_nudge_applies(SceneLightKind::Parallel),
            Confirmed::Known(true)
        );
    }

    #[test]
    fn parallel_auto_orient_other_drops_interest() {
        let mut light = Light::new(SceneLightKind::Parallel, 20, 20, 1);
        let interest = light.point_of_interest;
        light.auto_orient = AutoOrient::Other;
        light.point_of_interest = [3.0, 4.0, 5.0];
        assert_ne!(light.point_of_interest, interest);
        assert_eq!(light.auto_orient, AutoOrient::Other);
    }

    #[test]
    fn parallel_falloff_holds_inside_the_radius() {
        let inside = falloff_factor(Falloff::Smooth, 5.0, 10.0, 10.0);
        let edge = falloff_factor(Falloff::Smooth, 20.0, 10.0, 10.0);
        let inverse = falloff_factor(Falloff::InverseSquareClamped, 20.0, 10.0, 10.0);
        let none = falloff_factor(Falloff::None, 20.0, 10.0, 10.0);
        assert!(close(inside, 1.0));
        assert!(close(edge, 0.0));
        assert!(close(none, 1.0));
        assert!(inverse < 1.0);
        assert_eq!(Falloff::ALL.len(), 3);
    }

    #[test]
    fn parallel_radius_splits_constant_and_falling_light() {
        let inside = falloff_factor(Falloff::Smooth, 4.0, 8.0, 8.0);
        let also = falloff_factor(Falloff::Smooth, 8.0, 8.0, 8.0);
        let outside = falloff_factor(Falloff::Smooth, 12.0, 8.0, 8.0);
        assert!(close(inside, also));
        assert!(outside < inside);
        assert!(!Falloff::None.radius_editable());
    }

    #[test]
    fn parallel_smooth_reaches_zero_at_the_falloff_distance() {
        assert!(close(
            falloff_factor(Falloff::Smooth, 30.0, 10.0, 20.0),
            0.0
        ));
        assert!(close(
            falloff_factor(Falloff::Smooth, 20.0, 10.0, 20.0),
            0.5
        ));
    }

    #[test]
    fn parallel_cast_shadows_cannot_be_keyed() {
        assert_eq!(
            light_field_keyframes(LightField::CastShadows),
            Keyframes::Forbidden
        );
        assert!(Light::new(SceneLightKind::Parallel, 10, 10, 1)
            .casts_shadows
            .is_unconfirmed());
    }

    #[test]
    fn parallel_shadow_darkness_is_editable_only_while_casting() {
        assert_eq!(
            light_field_keyframes(LightField::ShadowDarkness),
            Keyframes::Allowed
        );
    }

    #[test]
    fn parallel_shadow_diffusion_is_editable_only_while_casting() {
        assert_eq!(
            light_field_keyframes(LightField::ShadowDiffusion),
            Keyframes::Allowed
        );
    }

    #[test]
    fn spot_cone_is_editable_and_cuts_the_outside() {
        assert!(SceneLightKind::Spot.cone_editable());
        assert!(!SceneLightKind::Point.cone_editable());
        assert!(!SceneLightKind::Ambient.cone_editable());
        assert!(close(cone_factor(50.0, 40.0, 0.0), 0.0));
        assert!(close(cone_factor(10.0, 40.0, 0.0), 1.0));
    }

    #[test]
    fn spot_name_starts_at_spot_light_1() {
        assert_eq!(
            generated_name(SceneLightKind::Spot, 1),
            Confirmed::Known("スポットライト 1".to_string())
        );
        let edited = rename_for_kind_change(
            "別",
            false,
            NameEditSource::Dialog,
            SceneLightKind::Point,
            1,
        );
        assert_eq!(edited, Confirmed::Known("別".to_string()));
    }

    #[test]
    fn spot_color_stays_inside_the_cone() {
        let inside = lit(
            SceneLightKind::Spot,
            100.0,
            Falloff::None,
            10.0,
            0.0,
            0.0,
            5.0,
            30.0,
            0.0,
        );
        let outside = lit(
            SceneLightKind::Spot,
            100.0,
            Falloff::None,
            10.0,
            0.0,
            0.0,
            40.0,
            30.0,
            0.0,
        );
        assert!(inside > 0.0);
        assert!(close(outside, 0.0));
    }

    #[test]
    fn spot_intensity_is_confined_to_the_cone() {
        let positive = lit(
            SceneLightKind::Spot,
            80.0,
            Falloff::None,
            5.0,
            0.0,
            0.0,
            0.0,
            40.0,
            0.0,
        );
        let negative = lit(
            SceneLightKind::Spot,
            -80.0,
            Falloff::None,
            5.0,
            0.0,
            0.0,
            0.0,
            40.0,
            0.0,
        );
        let outside = lit(
            SceneLightKind::Spot,
            80.0,
            Falloff::None,
            5.0,
            0.0,
            0.0,
            90.0,
            40.0,
            0.0,
        );
        assert!(positive > 0.0);
        assert!(negative < 0.0);
        assert!(close(outside, 0.0));
    }

    #[test]
    fn spot_cone_angle_widens_the_lit_region() {
        let narrow = cone_factor(20.0, 30.0, 0.0);
        let wide = cone_factor(20.0, 50.0, 0.0);
        assert!(close(narrow, 0.0));
        assert!(close(wide, 1.0));
        assert!(!SceneLightKind::Parallel.cone_editable());
    }

    #[test]
    fn spot_feather_softens_the_edge() {
        let hard = cone_factor(14.0, 30.0, 0.0);
        let soft = cone_factor(14.0, 30.0, 80.0);
        assert!(soft < hard);
        assert!(close(cone_factor(20.0, 30.0, 50.0), 0.0));
    }

    #[test]
    fn spot_interest_starts_at_center() {
        let light = Light::new(SceneLightKind::Spot, 80, 40, 1);
        assert!(close(light.point_of_interest[0], 40.0));
        assert!(close(light.point_of_interest[1], 20.0));
    }

    #[test]
    fn spot_position_initial_is_unconfirmed() {
        assert!(Light::new(SceneLightKind::Spot, 10, 10, 1)
            .position
            .is_unconfirmed());
    }

    #[test]
    fn spot_direction_follows_auto_orient_other() {
        let mut light = Light::new(SceneLightKind::Spot, 10, 10, 1);
        light.auto_orient = AutoOrient::Other;
        assert_eq!(light.auto_orient, AutoOrient::Other);
        assert_eq!(
            light_field_keyframes(LightField::Direction),
            Keyframes::Unconfirmed
        );
    }

    #[test]
    fn spot_x_rotation_changes_only_x() {
        let mut light = Light::new(SceneLightKind::Spot, 10, 10, 1);
        light.rotation_x = Confirmed::Known(1.0);
        light.rotation_y = Confirmed::Known(2.0);
        light.rotation_z = Confirmed::Known(3.0);
        light.rotation_x = Confirmed::Known(7.0);
        assert_eq!(light.rotation_y, Confirmed::Known(2.0));
        assert_eq!(light.rotation_z, Confirmed::Known(3.0));
    }

    #[test]
    fn spot_y_rotation_changes_only_y() {
        let mut light = Light::new(SceneLightKind::Spot, 10, 10, 1);
        light.rotation_x = Confirmed::Known(1.0);
        light.rotation_y = Confirmed::Known(2.0);
        light.rotation_z = Confirmed::Known(3.0);
        light.rotation_y = Confirmed::Known(9.0);
        assert_eq!(light.rotation_x, Confirmed::Known(1.0));
    }

    #[test]
    fn spot_z_rotation_nudge() {
        let mut light = Light::new(SceneLightKind::Spot, 10, 10, 1);
        light.rotation_z = Confirmed::Known(0.0);
        light.add_rotation_z(nudge_degrees(false));
        light.add_rotation_z(nudge_degrees(true));
        assert!(close(light.rotation_z.known().unwrap(), 11.0));
    }

    #[test]
    fn spot_auto_orient_other_stops_following_interest() {
        let mut light = Light::new(SceneLightKind::Spot, 10, 10, 1);
        light.auto_orient = AutoOrient::Other;
        let before = light.auto_orient;
        light.point_of_interest = [9.0, 9.0, 9.0];
        assert_eq!(light.auto_orient, before);
    }

    #[test]
    fn spot_falloff_does_not_light_outside_the_cone() {
        let outside = lit(
            SceneLightKind::Spot,
            100.0,
            Falloff::None,
            1.0,
            0.0,
            100.0,
            80.0,
            20.0,
            0.0,
        );
        let inside = lit(
            SceneLightKind::Spot,
            100.0,
            Falloff::Smooth,
            5.0,
            10.0,
            10.0,
            0.0,
            40.0,
            0.0,
        );
        let fallen = lit(
            SceneLightKind::Spot,
            100.0,
            Falloff::Smooth,
            20.0,
            10.0,
            10.0,
            0.0,
            40.0,
            0.0,
        );
        assert!(close(outside, 0.0));
        assert!(close(inside, 1.0));
        assert!(fallen < inside);
    }

    #[test]
    fn spot_radius_holds_illuminance_inside() {
        let a = lit(
            SceneLightKind::Spot,
            100.0,
            Falloff::Smooth,
            2.0,
            6.0,
            6.0,
            0.0,
            40.0,
            0.0,
        );
        let b = lit(
            SceneLightKind::Spot,
            100.0,
            Falloff::Smooth,
            6.0,
            6.0,
            6.0,
            0.0,
            40.0,
            0.0,
        );
        let c = lit(
            SceneLightKind::Spot,
            100.0,
            Falloff::Smooth,
            9.0,
            6.0,
            6.0,
            0.0,
            40.0,
            0.0,
        );
        assert!(close(a, b));
        assert!(c < a);
    }

    #[test]
    fn spot_smooth_is_zero_after_the_falloff_distance() {
        assert!(close(
            lit(
                SceneLightKind::Spot,
                100.0,
                Falloff::Smooth,
                16.0,
                6.0,
                10.0,
                0.0,
                50.0,
                0.0
            ),
            0.0
        ));
    }

    #[test]
    fn spot_cast_shadows_keyframes_are_forbidden() {
        assert_eq!(
            light_field_keyframes(LightField::CastShadows),
            Keyframes::Forbidden
        );
    }

    #[test]
    fn spot_shadow_darkness_can_be_keyed() {
        assert_eq!(
            light_field_keyframes(LightField::ShadowDarkness),
            Keyframes::Allowed
        );
    }

    #[test]
    fn spot_shadow_diffusion_can_be_keyed() {
        assert_eq!(
            light_field_keyframes(LightField::ShadowDiffusion),
            Keyframes::Allowed
        );
    }

    #[test]
    fn point_light_disables_the_cone_and_spreads() {
        assert!(!SceneLightKind::Point.cone_editable());
        let left = lit(
            SceneLightKind::Point,
            100.0,
            Falloff::None,
            12.0,
            0.0,
            0.0,
            0.0,
            10.0,
            0.0,
        );
        let right = lit(
            SceneLightKind::Point,
            100.0,
            Falloff::None,
            12.0,
            0.0,
            0.0,
            180.0,
            10.0,
            0.0,
        );
        assert!(close(left, right));
        assert!(left > 0.0);
    }

    #[test]
    fn point_rename_from_an_unedited_spot_uses_the_official_name() {
        let renamed = rename_for_kind_change(
            "スポットライト 1",
            true,
            NameEditSource::Dialog,
            SceneLightKind::Point,
            1,
        );
        assert_eq!(renamed, Confirmed::Known("ポイントライト 1".to_string()));
        assert!(generated_name(SceneLightKind::Point, 1).is_unconfirmed());
        assert!(Light::new(SceneLightKind::Point, 10, 10, 1)
            .name
            .is_unconfirmed());
    }

    #[test]
    fn point_color_reaches_both_sides() {
        let east = lit(
            SceneLightKind::Point,
            100.0,
            Falloff::None,
            4.0,
            0.0,
            0.0,
            0.0,
            1.0,
            0.0,
        );
        let west = lit(
            SceneLightKind::Point,
            100.0,
            Falloff::None,
            4.0,
            0.0,
            0.0,
            180.0,
            1.0,
            0.0,
        );
        assert!(close(east, west));
    }

    #[test]
    fn point_intensity_signs_are_omnidirectional() {
        assert!(
            lit(
                SceneLightKind::Point,
                25.0,
                Falloff::None,
                3.0,
                0.0,
                0.0,
                90.0,
                1.0,
                0.0
            ) > 0.0
        );
        assert!(
            lit(
                SceneLightKind::Point,
                -25.0,
                Falloff::None,
                3.0,
                0.0,
                0.0,
                90.0,
                1.0,
                0.0
            ) < 0.0
        );
    }

    #[test]
    fn point_interest_effect_is_unconfirmed() {
        let light = Light::new(SceneLightKind::Point, 30, 20, 1);
        assert!(close(light.point_of_interest[0], 15.0));
        assert!(point_of_interest_affects_point_distribution().is_unconfirmed());
    }

    #[test]
    fn point_gets_brighter_as_it_approaches_when_falloff_is_on() {
        let far = lit(
            SceneLightKind::Point,
            100.0,
            Falloff::InverseSquareClamped,
            40.0,
            10.0,
            0.0,
            0.0,
            0.0,
            0.0,
        );
        let near = lit(
            SceneLightKind::Point,
            100.0,
            Falloff::InverseSquareClamped,
            20.0,
            10.0,
            0.0,
            0.0,
            0.0,
            0.0,
        );
        assert!(near > far);
    }

    #[test]
    fn point_direction_effect_is_unconfirmed() {
        assert!(point_direction_affects_illuminance().is_unconfirmed());
    }

    #[test]
    fn point_x_rotation_effect_is_unconfirmed() {
        assert!(point_rotation_affects_illuminance().is_unconfirmed());
    }

    #[test]
    fn point_y_rotation_effect_is_unconfirmed() {
        assert!(point_rotation_affects_illuminance().is_unconfirmed());
    }

    #[test]
    fn point_z_rotation_shortcut_is_unconfirmed() {
        assert!(z_rotation_nudge_applies(SceneLightKind::Point).is_unconfirmed());
    }

    #[test]
    fn point_auto_orient_can_ignore_interest() {
        let mut light = Light::new(SceneLightKind::Point, 10, 10, 1);
        light.auto_orient = AutoOrient::Other;
        assert_eq!(light.auto_orient, AutoOrient::Other);
    }

    #[test]
    fn point_falloff_matches_the_three_modes() {
        let none_near = falloff_factor(Falloff::None, 5.0, 10.0, 10.0);
        let none_far = falloff_factor(Falloff::None, 50.0, 10.0, 10.0);
        assert!(close(none_near, none_far));
        assert!(falloff_factor(Falloff::Smooth, 15.0, 10.0, 10.0) < 1.0);
        assert!(falloff_factor(Falloff::InverseSquareClamped, 20.0, 10.0, 10.0) < 1.0);
    }

    #[test]
    fn point_radius_shares_illuminance_inside() {
        assert!(close(
            falloff_factor(Falloff::Smooth, 1.0, 12.0, 12.0),
            falloff_factor(Falloff::Smooth, 12.0, 12.0, 12.0)
        ));
    }

    #[test]
    fn point_smooth_hits_zero() {
        assert!(close(
            falloff_factor(Falloff::Smooth, 22.0, 10.0, 12.0),
            0.0
        ));
    }

    #[test]
    fn point_shadows_are_unconfirmed_per_renderer() {
        assert!(Light::new(SceneLightKind::Point, 10, 10, 1)
            .casts_shadows
            .is_unconfirmed());
        assert_eq!(
            light_field_keyframes(LightField::CastShadows),
            Keyframes::Forbidden
        );
    }

    #[test]
    fn point_shadow_darkness_policy_follows_whether_shadows_exist() {
        assert_eq!(
            light_field_keyframes(LightField::ShadowDarkness),
            Keyframes::Allowed
        );
    }

    #[test]
    fn point_shadow_diffusion_policy_follows_whether_shadows_exist() {
        assert_eq!(
            light_field_keyframes(LightField::ShadowDiffusion),
            Keyframes::Allowed
        );
    }

    #[test]
    fn ambient_has_no_icon_and_ignores_position() {
        assert!(!ambient_has_comp_icon());
        assert!(!ambient_position_affects_brightness());
        let near = lit(
            SceneLightKind::Ambient,
            40.0,
            Falloff::Smooth,
            1.0,
            5.0,
            5.0,
            0.0,
            10.0,
            0.0,
        );
        let far = lit(
            SceneLightKind::Ambient,
            40.0,
            Falloff::Smooth,
            100.0,
            5.0,
            5.0,
            80.0,
            10.0,
            100.0,
        );
        assert!(close(near, far));
        assert!(!SceneLightKind::Ambient.cone_editable());
        assert!(!SceneLightKind::Ambient.falloff_applies());
    }

    #[test]
    fn ambient_generated_name_is_unconfirmed() {
        assert!(generated_name(SceneLightKind::Ambient, 1).is_unconfirmed());
        let kept = rename_for_kind_change(
            "固定",
            false,
            NameEditSource::Dialog,
            SceneLightKind::Spot,
            2,
        );
        assert_eq!(kept, Confirmed::Known("固定".to_string()));
    }

    #[test]
    fn ambient_color_can_be_keyed() {
        assert_eq!(light_field_keyframes(LightField::Color), Keyframes::Allowed);
    }

    #[test]
    fn ambient_intensity_does_not_create_a_shadow_term() {
        assert!(
            lit(
                SceneLightKind::Ambient,
                70.0,
                Falloff::None,
                1.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0
            ) > 0.0
        );
        assert!(
            lit(
                SceneLightKind::Ambient,
                -20.0,
                Falloff::None,
                1.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0
            ) < 0.0
        );
    }

    #[test]
    fn ambient_position_does_not_change_brightness() {
        assert!(!ambient_position_affects_brightness());
        assert!(ambient_position_on_timeline().is_unconfirmed());
    }

    #[test]
    fn ambient_cast_switch_does_not_cast() {
        assert_eq!(
            has_comp_icon(SceneLightKind::Ambient),
            Confirmed::Known(false)
        );
        assert!(close(
            lit(
                SceneLightKind::Ambient,
                100.0,
                Falloff::None,
                1.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0
            ),
            1.0
        ));
    }

    #[test]
    fn timeline_kind_change_does_not_rename() {
        let kept = rename_for_kind_change(
            "スポットライト 1",
            true,
            NameEditSource::Timeline,
            SceneLightKind::Point,
            1,
        );
        assert_eq!(kept, Confirmed::Known("スポットライト 1".to_string()));
    }

    #[test]
    fn environment_shadow_darkness_keyframes_stay_unconfirmed() {
        assert_eq!(
            environment_shadow_darkness_keyframes(),
            Keyframes::Unconfirmed
        );
    }
}
