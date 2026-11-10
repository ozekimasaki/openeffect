use crate::confirmed::{Confirmed, Keyframes, RangeError};

/// コンポジションの 3D レンダラー。初期値はクラシック 3D。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Renderer {
    Classic3d,
    Advanced3d,
    Cinema4d,
}

impl Renderer {
    pub const fn for_new_composition() -> Self {
        Self::Classic3d
    }

    pub const fn extrusion_enabled(self) -> bool {
        match self {
            Self::Classic3d => false,
            Self::Advanced3d | Self::Cinema4d => true,
        }
    }

    pub const fn environment_image_lighting(self) -> bool {
        match self {
            Self::Advanced3d => true,
            Self::Classic3d | Self::Cinema4d => false,
        }
    }

    pub const fn planes_only(self) -> bool {
        match self {
            Self::Classic3d => true,
            Self::Advanced3d | Self::Cinema4d => false,
        }
    }

    pub const fn render_quality_available(self) -> bool {
        matches!(self, Self::Advanced3d)
    }

    pub const fn cinema_options_available(self) -> bool {
        matches!(self, Self::Cinema4d)
    }

    /// 3D モデルを足すと Advanced 3D になる。
    pub const fn after_model_added(self) -> Self {
        let _ = self;
        Self::Advanced3d
    }

    pub const fn keyframes(self) -> Keyframes {
        let _ = self;
        Keyframes::Unconfirmed
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::for_new_composition()
    }
}

/// カメラ、ライト、または 3D レイヤーが 1 つでもあるとメニューが出る。
pub const fn renderer_menu_visible(has_camera: bool, has_light: bool, has_3d_layer: bool) -> bool {
    has_camera || has_light || has_3d_layer
}

/// Advanced 3D のシャドウ解像度。表示名にメモリ量が付く。初期値は要確認。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShadowResolution {
    Half,
    Full,
    Double,
}

impl ShadowResolution {
    pub const ALL: [Self; 3] = [Self::Half, Self::Full, Self::Double];

    pub const fn megabytes(self) -> u32 {
        match self {
            Self::Half => 2,
            Self::Full => 16,
            Self::Double => 128,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Half => "Half（2 MB）",
            Self::Full => "Full（16 MB）",
            Self::Double => "Double（128 MB）",
        }
    }

    pub const fn initial() -> Confirmed<Self> {
        Confirmed::Unconfirmed
    }

    pub const fn keyframes() -> Keyframes {
        Keyframes::Unconfirmed
    }
}

/// 1 から 32。初期値は 3。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ShadowSmoothness(u8);

impl ShadowSmoothness {
    pub const DEFAULT: u8 = 3;

    pub const fn new(value: u8) -> Result<Self, RangeError> {
        if value >= 1 && value <= 32 {
            Ok(Self(value))
        } else {
            Err(RangeError)
        }
    }

    pub const fn get(self) -> u8 {
        self.0
    }

    pub const fn keyframes() -> Keyframes {
        Keyframes::Unconfirmed
    }
}

impl Default for ShadowSmoothness {
    fn default() -> Self {
        Self(Self::DEFAULT)
    }
}

/// 品質を上げるとノイズは減る。スムージングを上げてもノイズは減る。
/// 品質を下げたときはスムージングを上げる。係数は公式に無いので順位だけを返す。
pub fn shadow_noise_rank(quality_step: i32, smoothness: ShadowSmoothness) -> i32 {
    -(quality_step + i32::from(smoothness.get()))
}

pub fn recommended_smoothness_delta(quality_delta: i32) -> i32 {
    -quality_delta
}

/// フォーカス距離を動かしたときの波紋。品質を上げると減る。
pub fn depth_of_field_ripple(quality_step: i32) -> i32 {
    -quality_step
}

/// 同じ解像度をより大きい箱へ広げると影は柔らかくなる。解像度を上げると隙間は小さくなる。
/// メガバイトは公式の表示量で、ピクセル寸法は要確認なのでメモリ量を相対密度に使う。
pub fn voxel_shadow_softness(box_extent: f64, resolution: ShadowResolution) -> f64 {
    box_extent / f64::from(resolution.megabytes())
}

pub fn intersection_gap(resolution: ShadowResolution) -> f64 {
    1.0 / f64::from(resolution.megabytes())
}

/// キャスティングボックス。寸法の公式な型名は要確認。中心と大きさで範囲を表す。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CastingBox {
    pub center: [f64; 3],
    pub size: [f64; 3],
}

impl CastingBox {
    pub fn contains(self, point: [f64; 3]) -> bool {
        let half = [self.size[0] / 2.0, self.size[1] / 2.0, self.size[2] / 2.0];
        (0..3).all(|axis| (point[axis] - self.center[axis]).abs() <= half[axis])
    }

    pub const fn size_initial() -> Confirmed<[f64; 3]> {
        Confirmed::Unconfirmed
    }

    pub const fn center_initial() -> Confirmed<[f64; 3]> {
        Confirmed::Unconfirmed
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CasterBounds {
    pub visible: bool,
    pub can_cast: bool,
    pub min: [f64; 3],
    pub max: [f64; 3],
}

/// 現在のフレームで、影を落とせる表示レイヤーを囲む。別フレームへは追わない。
pub fn fit_casting_box(casters: &[CasterBounds]) -> CastingBox {
    let mut included = casters
        .iter()
        .filter(|caster| caster.visible && caster.can_cast);
    let Some(first) = included.next() else {
        return CastingBox {
            center: [0.0, 0.0, 0.0],
            size: [0.0, 0.0, 0.0],
        };
    };
    let mut min = first.min;
    let mut max = first.max;
    for caster in included {
        for axis in 0..3 {
            min[axis] = min[axis].min(caster.min[axis]);
            max[axis] = max[axis].max(caster.max[axis]);
        }
    }
    CastingBox {
        center: [
            (min[0] + max[0]) / 2.0,
            (min[1] + max[1]) / 2.0,
            (min[2] + max[2]) / 2.0,
        ],
        size: [max[0] - min[0], max[1] - min[1], max[2] - min[2]],
    }
}

pub const fn casting_box_frame_visible(dialog_open: bool) -> bool {
    dialog_open
}

pub const fn fit_to_scene_keyframes() -> Keyframes {
    Keyframes::Forbidden
}

/// Cinema 4D のアンチエイリアス。初期値は Geometry。他の候補名は要確認。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AntiAliasing {
    Geometry,
    Unlisted,
}

/// Cinema 4D の反射率。初期値は Layer sampling。他の候補名は要確認。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ReflectanceSampling {
    LayerSampling,
    Unlisted,
}

/// 品質スライダが同時に書き換える値。数値の対応表は要確認。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Driven<T> {
    pub value: Confirmed<T>,
    pub changed_by_quality: bool,
}

impl<T> Driven<T> {
    const fn known(value: T) -> Self {
        Self {
            value: Confirmed::Known(value),
            changed_by_quality: false,
        }
    }

    const fn unknown() -> Self {
        Self {
            value: Confirmed::Unconfirmed,
            changed_by_quality: false,
        }
    }

    fn invalidate(&mut self) {
        self.value = Confirmed::Unconfirmed;
        self.changed_by_quality = true;
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CinemaOptions {
    pub quality: Confirmed<f64>,
    pub ray_threshold: Driven<f64>,
    pub ray_depth: Driven<u32>,
    pub reflection_depth: Driven<u32>,
    pub shadow_depth: Driven<u32>,
    pub anti_aliasing: Driven<AntiAliasing>,
    pub reflectance: Driven<ReflectanceSampling>,
}

impl CinemaOptions {
    pub const fn initial() -> Self {
        Self {
            quality: Confirmed::Unconfirmed,
            ray_threshold: Driven::unknown(),
            ray_depth: Driven::unknown(),
            reflection_depth: Driven::unknown(),
            shadow_depth: Driven::unknown(),
            anti_aliasing: Driven::known(AntiAliasing::Geometry),
            reflectance: Driven::known(ReflectanceSampling::LayerSampling),
        }
    }

    /// 品質を動かすと 6 項目が同時に変わる。変わる先の数値は要確認。
    pub fn move_quality_slider(&mut self) {
        self.quality = Confirmed::Unconfirmed;
        self.ray_threshold.invalidate();
        self.ray_depth.invalidate();
        self.reflection_depth.invalidate();
        self.shadow_depth.invalidate();
        self.anti_aliasing.invalidate();
        self.reflectance.invalidate();
    }
}

impl Default for CinemaOptions {
    fn default() -> Self {
        Self::initial()
    }
}

/// 1 始まりの深さ。値を超えた面は描かない。
pub const fn depth_is_drawn(depth_from_front: u32, limit: u32) -> bool {
    depth_from_front <= limit
}

pub fn cinema_options(renderer: Renderer) -> Option<CinemaOptions> {
    if renderer.cinema_options_available() {
        Some(CinemaOptions::initial())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{
        casting_box_frame_visible, cinema_options, depth_is_drawn, depth_of_field_ripple,
        fit_casting_box, fit_to_scene_keyframes, intersection_gap, recommended_smoothness_delta,
        renderer_menu_visible, shadow_noise_rank, voxel_shadow_softness, AntiAliasing,
        CasterBounds, CinemaOptions, Confirmed, Keyframes, ReflectanceSampling, Renderer,
        ShadowResolution, ShadowSmoothness,
    };

    #[test]
    fn new_composition_renderer_is_classic_and_model_switches_to_advanced() {
        assert_eq!(Renderer::for_new_composition(), Renderer::Classic3d);
        assert!(Renderer::Classic3d.planes_only());
        assert!(!Renderer::Classic3d.extrusion_enabled());
        assert!(!Renderer::Classic3d.environment_image_lighting());
        assert!(Renderer::Advanced3d.extrusion_enabled());
        assert!(Renderer::Cinema4d.extrusion_enabled());
        assert!(Renderer::Advanced3d.environment_image_lighting());
        assert!(!Renderer::Cinema4d.environment_image_lighting());
        assert!(!Renderer::Advanced3d.planes_only());
        assert_eq!(
            Renderer::Classic3d.after_model_added(),
            Renderer::Advanced3d
        );
        assert!(renderer_menu_visible(true, false, false));
        assert!(renderer_menu_visible(false, true, false));
        assert!(renderer_menu_visible(false, false, true));
        assert!(!renderer_menu_visible(false, false, false));
        assert_eq!(Renderer::Classic3d.keyframes(), Keyframes::Unconfirmed);
    }

    #[test]
    fn render_quality_exists_only_for_advanced_and_changes_shadow_and_dof() {
        assert!(Renderer::Advanced3d.render_quality_available());
        assert!(!Renderer::Classic3d.render_quality_available());
        assert!(!Renderer::Cinema4d.render_quality_available());
        assert!(
            shadow_noise_rank(1, ShadowSmoothness::default())
                < shadow_noise_rank(0, ShadowSmoothness::default())
        );
        assert!(depth_of_field_ripple(1) < depth_of_field_ripple(0));
    }

    #[test]
    fn shadow_resolution_has_three_labeled_choices() {
        assert!(ShadowResolution::initial().is_unconfirmed());
        assert_eq!(ShadowResolution::ALL.len(), 3);
        assert!(ShadowResolution::Half.label().contains("2 MB"));
        assert!(ShadowResolution::Full.label().contains("16 MB"));
        assert!(ShadowResolution::Double.label().contains("128 MB"));
        assert_eq!(ShadowResolution::Half.megabytes(), 2);
        assert_eq!(ShadowResolution::Full.megabytes(), 16);
        assert_eq!(ShadowResolution::Double.megabytes(), 128);
        assert!(
            intersection_gap(ShadowResolution::Double) < intersection_gap(ShadowResolution::Half)
        );
    }

    #[test]
    fn shadow_smoothness_defaults_to_3_and_opposes_quality() {
        assert_eq!(ShadowSmoothness::default().get(), 3);
        assert!(ShadowSmoothness::new(0).is_err());
        assert!(ShadowSmoothness::new(33).is_err());
        assert_eq!(ShadowSmoothness::new(1).unwrap().get(), 1);
        assert_eq!(ShadowSmoothness::new(32).unwrap().get(), 32);
        let base = shadow_noise_rank(0, ShadowSmoothness::new(3).unwrap());
        let smoother = shadow_noise_rank(0, ShadowSmoothness::new(8).unwrap());
        assert!(smoother < base);
        assert!(recommended_smoothness_delta(1) < 0);
        assert!(recommended_smoothness_delta(-1) > 0);
        assert_eq!(ShadowSmoothness::keyframes(), Keyframes::Unconfirmed);
    }

    #[test]
    fn casting_box_size_changes_shadow_reach_and_softness() {
        assert!(CastingBoxSizeProbe::initial_is_unconfirmed());
        let small = super::CastingBox {
            center: [0.0, 0.0, 0.0],
            size: [10.0, 10.0, 10.0],
        };
        let large = super::CastingBox {
            center: [0.0, 0.0, 0.0],
            size: [30.0, 30.0, 30.0],
        };
        let point = [12.0, 0.0, 0.0];
        assert!(!small.contains(point));
        assert!(large.contains(point));
        assert!(
            voxel_shadow_softness(30.0, ShadowResolution::Full)
                > voxel_shadow_softness(10.0, ShadowResolution::Full)
        );
        assert!(casting_box_frame_visible(true));
        assert!(!casting_box_frame_visible(false));
    }

    struct CastingBoxSizeProbe;

    impl CastingBoxSizeProbe {
        fn initial_is_unconfirmed() -> bool {
            super::CastingBox::size_initial().is_unconfirmed()
        }
    }

    #[test]
    fn casting_box_center_moves_the_volume() {
        assert!(super::CastingBox::center_initial().is_unconfirmed());
        let moved = super::CastingBox {
            center: [5.0, 0.0, 0.0],
            size: [4.0, 4.0, 4.0],
        };
        let stayed = super::CastingBox {
            center: [0.0, 0.0, 0.0],
            size: [4.0, 4.0, 4.0],
        };
        let point = [5.0, 0.0, 0.0];
        assert!(moved.contains(point));
        assert!(!stayed.contains(point));
    }

    #[test]
    fn fit_to_scene_encloses_visible_casters_on_that_frame_only() {
        let frame_0 = [CasterBounds {
            visible: true,
            can_cast: true,
            min: [0.0, 0.0, 0.0],
            max: [10.0, 4.0, 2.0],
        }];
        let hidden = CasterBounds {
            visible: false,
            can_cast: true,
            min: [100.0, 100.0, 100.0],
            max: [110.0, 110.0, 110.0],
        };
        let fitted = fit_casting_box(&[frame_0[0], hidden]);
        assert!(fitted.contains([10.0, 4.0, 2.0]));
        assert!(!fitted.contains([100.0, 100.0, 100.0]));
        let after_frame_change = fitted;
        assert_eq!(after_frame_change, fitted);
        assert_eq!(fit_to_scene_keyframes(), Keyframes::Forbidden);
    }

    #[test]
    fn cinema_quality_slider_is_cinema_only_and_drives_every_option() {
        assert!(cinema_options(Renderer::Cinema4d).is_some());
        assert!(cinema_options(Renderer::Classic3d).is_none());
        assert!(cinema_options(Renderer::Advanced3d).is_none());
        let mut options = CinemaOptions::initial();
        assert!(options.quality.is_unconfirmed());
        options.move_quality_slider();
        assert!(options.ray_threshold.changed_by_quality);
        assert!(options.ray_depth.changed_by_quality);
        assert!(options.reflection_depth.changed_by_quality);
        assert!(options.shadow_depth.changed_by_quality);
        assert!(options.anti_aliasing.changed_by_quality);
        assert!(options.reflectance.changed_by_quality);
        assert!(options.ray_threshold.value.is_unconfirmed());
    }

    #[test]
    fn cinema_ray_threshold_follows_quality_and_is_absent_on_classic() {
        assert!(cinema_options(Renderer::Classic3d).is_none());
        let mut options = CinemaOptions::initial();
        assert!(!options.ray_threshold.changed_by_quality);
        options.move_quality_slider();
        assert!(options.ray_threshold.changed_by_quality);
        assert!(matches!(
            options.ray_threshold.value,
            Confirmed::Unconfirmed
        ));
    }

    #[test]
    fn cinema_ray_depth_cuts_transparency() {
        let mut options = CinemaOptions::initial();
        options.move_quality_slider();
        assert!(options.ray_depth.changed_by_quality);
        assert!(depth_is_drawn(2, 2));
        assert!(!depth_is_drawn(3, 2));
    }

    #[test]
    fn cinema_reflection_depth_cuts_bounces() {
        let mut options = CinemaOptions::initial();
        options.move_quality_slider();
        assert!(options.reflection_depth.changed_by_quality);
        assert!(depth_is_drawn(1, 1));
        assert!(!depth_is_drawn(2, 1));
    }

    #[test]
    fn cinema_shadow_depth_cuts_shadow_rays() {
        let mut options = CinemaOptions::initial();
        options.move_quality_slider();
        assert!(options.shadow_depth.changed_by_quality);
        assert!(!depth_is_drawn(4, 3));
    }

    #[test]
    fn cinema_anti_aliasing_starts_at_geometry_and_follows_quality() {
        let mut options = CinemaOptions::initial();
        assert_eq!(
            options.anti_aliasing.value,
            Confirmed::Known(AntiAliasing::Geometry)
        );
        options.move_quality_slider();
        assert!(options.anti_aliasing.value.is_unconfirmed());
        assert!(options.anti_aliasing.changed_by_quality);
    }

    #[test]
    fn cinema_reflectance_starts_at_layer_sampling_and_follows_quality() {
        let mut options = CinemaOptions::initial();
        assert_eq!(
            options.reflectance.value,
            Confirmed::Known(ReflectanceSampling::LayerSampling)
        );
        options.move_quality_slider();
        assert!(options.reflectance.value.is_unconfirmed());
        assert!(options.reflectance.changed_by_quality);
    }
}
