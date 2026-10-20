use model::{BitsPerChannel, CameraNodes, LayerId};

use crate::confirmed::{Confirmed, Keyframes, RangeError};
use crate::math::{
    composition_center, distance, normalize, rotate_x, rotate_y, rotate_z, sub,
    FOCUS_DISTANCE_EXPRESSION,
};
use crate::renderer::Renderer;

/// 公式ヘルプに名前があるプリセットは 50 mm だけ。一覧の残りは要確認。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CameraPreset {
    Mm50,
    Custom,
}

/// 自動方向。目標点以外の候補名は要確認。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AutoOrient {
    TowardPointOfInterest,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Optics {
    pub comp_width_px: f64,
    pub measured_pixels: Confirmed<f64>,
    pub film_size_mm: Confirmed<f64>,
    pub focal_length_mm: f64,
    pub zoom_px: Confirmed<f64>,
    pub angle_degrees: Confirmed<f64>,
    pub focus_distance_px: Confirmed<f64>,
    pub lock_to_zoom: Confirmed<bool>,
    pub aperture_px: Confirmed<f64>,
    pub f_stop: Confirmed<f64>,
    pub preset: CameraPreset,
    pub display_per_mm: Confirmed<f64>,
}

impl Optics {
    pub const fn new(comp_width_px: f64) -> Self {
        Self {
            comp_width_px,
            measured_pixels: Confirmed::Unconfirmed,
            film_size_mm: Confirmed::Unconfirmed,
            focal_length_mm: 50.0,
            zoom_px: Confirmed::Unconfirmed,
            angle_degrees: Confirmed::Unconfirmed,
            focus_distance_px: Confirmed::Unconfirmed,
            lock_to_zoom: Confirmed::Unconfirmed,
            aperture_px: Confirmed::Unconfirmed,
            f_stop: Confirmed::Unconfirmed,
            preset: CameraPreset::Mm50,
            display_per_mm: Confirmed::Unconfirmed,
        }
    }

    pub fn set_measured_pixels(&mut self, pixels: f64) {
        self.measured_pixels = Confirmed::Known(pixels);
        self.preset = CameraPreset::Custom;
        self.recompute_from_lens();
    }

    pub fn set_film_size_mm(&mut self, film_size_mm: f64) {
        self.film_size_mm = Confirmed::Known(film_size_mm);
        self.preset = CameraPreset::Custom;
        self.recompute_from_lens();
    }

    pub fn set_focal_length_mm(&mut self, focal_length_mm: f64) {
        self.focal_length_mm = focal_length_mm;
        self.preset = CameraPreset::Custom;
        self.recompute_from_lens();
        if self.zoom_px.is_unconfirmed() {
            self.aperture_px = Confirmed::Unconfirmed;
        }
    }

    pub fn edit_zoom_in_dialog(&mut self, zoom_px: f64) {
        self.write_zoom(zoom_px, true);
    }

    pub fn edit_zoom_on_timeline(&mut self, zoom_px: f64) {
        self.lock_to_zoom = Confirmed::Known(false);
        self.write_zoom(zoom_px, false);
    }

    pub fn edit_focus_in_dialog(&mut self, focus_px: f64) {
        if self.lock_to_zoom == Confirmed::Known(true) {
            self.write_zoom(focus_px, true);
        } else {
            self.focus_distance_px = Confirmed::Known(focus_px);
            self.preset = CameraPreset::Custom;
        }
    }

    pub fn edit_focus_on_timeline(&mut self, focus_px: f64) {
        self.lock_to_zoom = Confirmed::Known(false);
        self.focus_distance_px = Confirmed::Known(focus_px);
        self.preset = CameraPreset::Custom;
    }

    pub fn edit_angle_degrees(&mut self, angle_degrees: f64) {
        self.angle_degrees = Confirmed::Known(angle_degrees);
        if let Some(zoom) = zoom_from_angle(self.comp_width_px, angle_degrees) {
            self.zoom_px = Confirmed::Known(zoom);
            if self.lock_to_zoom == Confirmed::Known(true) {
                self.focus_distance_px = Confirmed::Known(zoom);
            }
        }
        self.preset = CameraPreset::Custom;
    }

    pub fn edit_aperture_px(&mut self, aperture_px: f64) {
        self.aperture_px = Confirmed::Known(aperture_px);
        if let Confirmed::Known(zoom) = self.zoom_px {
            if aperture_px != 0.0 {
                self.f_stop = Confirmed::Known(zoom / aperture_px);
            }
        }
        self.preset = CameraPreset::Custom;
    }

    pub fn edit_f_stop(&mut self, f_stop: f64) {
        self.f_stop = Confirmed::Known(f_stop);
        if let Confirmed::Known(zoom) = self.zoom_px {
            if f_stop != 0.0 {
                self.aperture_px = Confirmed::Known(zoom / f_stop);
            }
        }
        self.preset = CameraPreset::Custom;
    }

    pub fn set_display_scale(&mut self, per_mm: f64) {
        self.display_per_mm = Confirmed::Known(per_mm);
    }

    pub fn display_mm(&self, millimeters: f64) -> Confirmed<f64> {
        match self.display_per_mm {
            Confirmed::Known(scale) => Confirmed::Known(millimeters * scale),
            Confirmed::Unconfirmed => Confirmed::Unconfirmed,
        }
    }

    /// プリセットは 5 つの光学値を同時に入れる。50 mm 以外の名前は要確認。
    pub fn assign_bundle(
        &mut self,
        zoom_px: f64,
        angle_degrees: f64,
        focus_distance_px: f64,
        focal_length_mm: f64,
        aperture_px: f64,
    ) {
        self.zoom_px = Confirmed::Known(zoom_px);
        self.angle_degrees = Confirmed::Known(angle_degrees);
        self.focus_distance_px = Confirmed::Known(focus_distance_px);
        self.focal_length_mm = focal_length_mm;
        self.aperture_px = Confirmed::Known(aperture_px);
        if aperture_px != 0.0 {
            self.f_stop = Confirmed::Known(zoom_px / aperture_px);
        }
        self.preset = if focal_length_mm == 50.0 {
            CameraPreset::Mm50
        } else {
            CameraPreset::Custom
        };
    }

    pub fn apply_preset_50mm(&mut self) {
        self.focal_length_mm = 50.0;
        self.preset = CameraPreset::Mm50;
        self.recompute_from_lens();
        if self.lock_to_zoom != Confirmed::Known(true) {
            self.focus_distance_px = Confirmed::Unconfirmed;
        }
        self.aperture_px = Confirmed::Unconfirmed;
        self.f_stop = Confirmed::Unconfirmed;
    }

    fn write_zoom(&mut self, zoom_px: f64, keep_lock: bool) {
        self.zoom_px = Confirmed::Known(zoom_px);
        self.angle_degrees = Confirmed::Known(angle_of_view_degrees(self.comp_width_px, zoom_px));
        if keep_lock && self.lock_to_zoom == Confirmed::Known(true) {
            self.focus_distance_px = Confirmed::Known(zoom_px);
        }
        self.preset = CameraPreset::Custom;
    }

    fn recompute_from_lens(&mut self) {
        let Confirmed::Known(film) = self.film_size_mm else {
            self.zoom_px = Confirmed::Unconfirmed;
            self.angle_degrees = Confirmed::Unconfirmed;
            return;
        };
        let Confirmed::Known(measured) = self.measured_pixels else {
            self.zoom_px = Confirmed::Unconfirmed;
            self.angle_degrees = Confirmed::Unconfirmed;
            return;
        };
        if film == 0.0 {
            return;
        }
        let zoom = zoom_from_lens(self.focal_length_mm, film, measured);
        self.zoom_px = Confirmed::Known(zoom);
        self.angle_degrees = Confirmed::Known(angle_of_view_degrees(self.comp_width_px, zoom));
        if self.lock_to_zoom == Confirmed::Known(true) {
            self.focus_distance_px = Confirmed::Known(zoom);
        }
        if let Confirmed::Known(f_stop) = self.f_stop {
            if f_stop != 0.0 {
                self.aperture_px = Confirmed::Known(zoom / f_stop);
            }
        }
    }
}

/// ズームと同じ Z 距離で等倍、2 倍の距離で半分。
pub fn apparent_scale(zoom_px: f64, z_distance_px: f64) -> f64 {
    zoom_px / z_distance_px
}

pub fn frame_coverage(layer_px: f64, zoom_px: f64, z_distance_px: f64, frame_px: f64) -> f64 {
    layer_px * apparent_scale(zoom_px, z_distance_px) / frame_px
}

/// 像面の幅はコンポジションの幅。画角はその幅とズームから決まる。
pub fn angle_of_view_degrees(comp_width_px: f64, zoom_px: f64) -> f64 {
    let half = (comp_width_px / 2.0) / zoom_px;
    half.atan().to_degrees() * 2.0
}

pub fn zoom_from_angle(comp_width_px: f64, angle_degrees: f64) -> Option<f64> {
    let half = (angle_degrees / 2.0).to_radians().tan();
    if half.abs() <= f64::EPSILON {
        None
    } else {
        Some((comp_width_px / 2.0) / half)
    }
}

/// 焦点距離 / フィルムサイズ = ズーム / 測定ピクセル。
pub fn zoom_from_lens(focal_length_mm: f64, film_size_mm: f64, measured_pixels: f64) -> f64 {
    focal_length_mm / film_size_mm * measured_pixels
}

/// 被写界深度。オフは 0。オンでフォーカス距離から離れるほど、絞りが大きいほどボケる。
/// ブラーレベル 100% がこの量、それより小さいと弱い。
pub fn defocus_pixels(
    enabled: bool,
    distance_px: f64,
    focus_distance_px: f64,
    aperture_px: f64,
    blur_level_percent: f64,
) -> f64 {
    if !enabled || focus_distance_px <= 0.0 {
        return 0.0;
    }
    let delta = (distance_px - focus_distance_px).abs();
    delta * (aperture_px / focus_distance_px) * (blur_level_percent / 100.0)
}

pub const fn depth_of_field_expression_value(enabled: bool) -> f64 {
    if enabled {
        1.0
    } else {
        0.0
    }
}

pub const fn aperture_changes_exposure() -> bool {
    false
}

/// Advanced 3D のエンジン内被写界深度。クラシックでは幅とニア／ファーを効かせない。
#[allow(clippy::too_many_arguments)]
pub fn engine_defocus_pixels(
    renderer: Renderer,
    enabled: bool,
    distance_px: f64,
    focus_distance_px: f64,
    aperture_px: f64,
    blur_level_percent: f64,
    focus_area_width: f64,
    near_blur_percent: f64,
    far_blur_percent: f64,
) -> f64 {
    if !enabled {
        return 0.0;
    }
    if renderer != Renderer::Advanced3d {
        return defocus_pixels(
            true,
            distance_px,
            focus_distance_px,
            aperture_px,
            blur_level_percent,
        );
    }
    let delta = distance_px - focus_distance_px;
    if delta.abs() * 2.0 <= focus_area_width {
        return 0.0;
    }
    let level = if delta < 0.0 {
        near_blur_percent
    } else {
        far_blur_percent
    };
    defocus_pixels(true, distance_px, focus_distance_px, aperture_px, level)
}

pub const fn engine_dof_affects(renderer: Renderer) -> Confirmed<bool> {
    match renderer {
        Renderer::Advanced3d => Confirmed::Known(true),
        Renderer::Classic3d => Confirmed::Known(false),
        Renderer::Cinema4d => Confirmed::Unconfirmed,
    }
}

pub fn highlight_threshold_range(bits: BitsPerChannel) -> (f64, f64) {
    match bits {
        BitsPerChannel::Bpc8 => (0.0, 100.0),
        BitsPerChannel::Bpc16 => (0.0, 32768.0),
        BitsPerChannel::Bpc32 => (0.0, 1.0),
    }
}

pub fn set_highlight_threshold(value: f64, bits: BitsPerChannel) -> Result<f64, RangeError> {
    let (min, max) = highlight_threshold_range(bits);
    if (min..=max).contains(&value) {
        Ok(value)
    } else {
        Err(RangeError)
    }
}

/// エクスプレッション参照のゲイン範囲。スライダ端の一致は要確認。
pub const fn highlight_gain_expression_range() -> (f64, f64) {
    (1.0, 100.0)
}

pub const fn highlight_gain_slider_range() -> Confirmed<(f64, f64)> {
    Confirmed::Unconfirmed
}

pub fn highlight_energy(luminance: f64, threshold: f64, gain: f64) -> f64 {
    if luminance <= threshold {
        0.0
    } else {
        (luminance - threshold) * gain
    }
}

/// 0 で白、100 で色が残る。スライダ下限が 0 か 1 かは要確認なので、ここでは締めない。
pub fn highlight_saturation_color(
    red: f64,
    green: f64,
    blue: f64,
    saturation_percent: f64,
) -> [f64; 3] {
    let keep = saturation_percent / 100.0;
    [
        1.0 + (red - 1.0) * keep,
        1.0 + (green - 1.0) * keep,
        1.0 + (blue - 1.0) * keep,
    ]
}

pub const fn diffraction_fringe_range() -> Confirmed<(f64, f64)> {
    Confirmed::Unconfirmed
}

pub fn diffraction_strength(value: f64) -> f64 {
    value
}

pub fn set_iris_shape(value: u8) -> Result<u8, RangeError> {
    if (1..=10).contains(&value) {
        Ok(value)
    } else {
        Err(RangeError)
    }
}

pub fn set_iris_aspect(value: f64) -> Result<f64, RangeError> {
    if (1.0..=100.0).contains(&value) {
        Ok(value)
    } else {
        Err(RangeError)
    }
}

pub fn bokeh_circle_closeness(roundness_percent: f64) -> f64 {
    roundness_percent
}

pub const fn iris_rotation_changes_zoom() -> bool {
    false
}

pub fn bokeh_stretch(aspect: f64) -> [f64; 2] {
    [aspect, 1.0]
}

#[derive(Clone, Debug, PartialEq)]
pub struct Camera {
    pub nodes: CameraNodes,
    pub name: String,
    pub optics: Optics,
    pub depth_of_field: Confirmed<bool>,
    pub blur_level_percent: Confirmed<f64>,
    pub position: Confirmed<[f64; 3]>,
    pub orientation: Confirmed<[f64; 3]>,
    pub rotation_x: Confirmed<f64>,
    pub rotation_y: Confirmed<f64>,
    pub rotation_z: Confirmed<f64>,
    pub point_of_interest: [f64; 3],
    pub auto_orient: AutoOrient,
    pub preview: bool,
    pub iris_shape: Confirmed<u8>,
    pub iris_rotation: Confirmed<f64>,
    pub iris_roundness: Confirmed<f64>,
    pub iris_aspect: Confirmed<f64>,
    pub diffraction_fringe: Confirmed<f64>,
    pub highlight_gain: Confirmed<f64>,
    pub highlight_threshold: Confirmed<f64>,
    pub highlight_saturation: Confirmed<f64>,
    pub focus_area_width: Confirmed<f64>,
    pub near_blur_level: Confirmed<f64>,
    pub far_blur_level: Confirmed<f64>,
    pub focus_expression: Option<String>,
}

impl Camera {
    pub fn new(nodes: CameraNodes, width: u32, height: u32, existing_names: &[&str]) -> Self {
        Self {
            nodes,
            name: next_camera_name(existing_names),
            optics: Optics::new(f64::from(width)),
            depth_of_field: Confirmed::Unconfirmed,
            blur_level_percent: Confirmed::Unconfirmed,
            position: Confirmed::Unconfirmed,
            orientation: Confirmed::Unconfirmed,
            rotation_x: Confirmed::Unconfirmed,
            rotation_y: Confirmed::Unconfirmed,
            rotation_z: Confirmed::Unconfirmed,
            point_of_interest: composition_center(width, height),
            auto_orient: AutoOrient::TowardPointOfInterest,
            preview: true,
            iris_shape: Confirmed::Unconfirmed,
            iris_rotation: Confirmed::Unconfirmed,
            iris_roundness: Confirmed::Unconfirmed,
            iris_aspect: Confirmed::Unconfirmed,
            diffraction_fringe: Confirmed::Unconfirmed,
            highlight_gain: Confirmed::Unconfirmed,
            highlight_threshold: Confirmed::Unconfirmed,
            highlight_saturation: Confirmed::Unconfirmed,
            focus_area_width: Confirmed::Unconfirmed,
            near_blur_level: Confirmed::Unconfirmed,
            far_blur_level: Confirmed::Unconfirmed,
            focus_expression: None,
        }
    }

    pub fn gaze(&self) -> Confirmed<Gaze> {
        match self.nodes {
            CameraNodes::One => self.gaze_from_rotations(),
            CameraNodes::Two => self.gaze_toward_interest(),
        }
    }

    pub fn add_rotation_x(&mut self, degrees: f64) {
        self.rotation_x = Confirmed::Known(self.rotation_degrees(self.rotation_x) + degrees);
    }

    pub fn add_rotation_y(&mut self, degrees: f64) {
        self.rotation_y = Confirmed::Known(self.rotation_degrees(self.rotation_y) + degrees);
    }

    pub fn add_rotation_z(&mut self, degrees: f64) {
        self.rotation_z = Confirmed::Known(self.rotation_degrees(self.rotation_z) + degrees);
    }

    fn rotation_degrees(&self, value: Confirmed<f64>) -> f64 {
        match value {
            Confirmed::Known(degrees) => degrees,
            Confirmed::Unconfirmed => 0.0,
        }
    }

    fn gaze_from_rotations(&self) -> Confirmed<Gaze> {
        let Confirmed::Known(orientation) = self.orientation else {
            return Confirmed::Unconfirmed;
        };
        let Confirmed::Known(rotation_x) = self.rotation_x else {
            return Confirmed::Unconfirmed;
        };
        let Confirmed::Known(rotation_y) = self.rotation_y else {
            return Confirmed::Unconfirmed;
        };
        let Confirmed::Known(rotation_z) = self.rotation_z else {
            return Confirmed::Unconfirmed;
        };
        let mut forward = [0.0, 0.0, -1.0];
        let mut up = [0.0, -1.0, 0.0];
        for (angle, rotate) in [
            (orientation[0], rotate_x as fn([f64; 3], f64) -> [f64; 3]),
            (orientation[1], rotate_y),
            (orientation[2], rotate_z),
            (rotation_x, rotate_x),
            (rotation_y, rotate_y),
            (rotation_z, rotate_z),
        ] {
            forward = rotate(forward, angle);
            up = rotate(up, angle);
        }
        Confirmed::Known(Gaze { forward, up })
    }

    fn gaze_toward_interest(&self) -> Confirmed<Gaze> {
        if self.auto_orient != AutoOrient::TowardPointOfInterest {
            return self.gaze_from_rotations();
        }
        let orientation = match self.orientation {
            Confirmed::Known(value) => value,
            Confirmed::Unconfirmed => return Confirmed::Unconfirmed,
        };
        let rotation_x = match self.rotation_x {
            Confirmed::Known(value) => value,
            Confirmed::Unconfirmed => return Confirmed::Unconfirmed,
        };
        let rotation_y = match self.rotation_y {
            Confirmed::Known(value) => value,
            Confirmed::Unconfirmed => return Confirmed::Unconfirmed,
        };
        let rotation_z = match self.rotation_z {
            Confirmed::Known(value) => value,
            Confirmed::Unconfirmed => return Confirmed::Unconfirmed,
        };
        let position = match self.position {
            Confirmed::Known(value) => value,
            Confirmed::Unconfirmed => return Confirmed::Unconfirmed,
        };
        let extra = orientation != [0.0, 0.0, 0.0] || rotation_x != 0.0 || rotation_y != 0.0;
        if extra {
            return Confirmed::Unconfirmed;
        }
        let Some(forward) = normalize(sub(self.point_of_interest, position)) else {
            return Confirmed::Unconfirmed;
        };
        let up = roll_up(forward, rotation_z);
        Confirmed::Known(Gaze { forward, up })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gaze {
    pub forward: [f64; 3],
    pub up: [f64; 3],
}

fn roll_up(forward: [f64; 3], roll_degrees: f64) -> [f64; 3] {
    let world_up = [0.0, -1.0, 0.0];
    let Some(right) = normalize(crate::math::cross(world_up, forward))
        .or_else(|| normalize(crate::math::cross([1.0, 0.0, 0.0], forward)))
    else {
        return world_up;
    };
    let up = crate::math::cross(forward, right);
    let radians = roll_degrees.to_radians();
    let (sin, cos) = radians.sin_cos();
    crate::math::add(crate::math::scale(up, cos), crate::math::scale(right, sin))
}

pub fn next_camera_name(existing: &[&str]) -> String {
    let mut max_number = 0_u32;
    for name in existing {
        if let Some(number) = parse_camera_number(name) {
            max_number = max_number.max(number);
        }
    }
    format!("カメラ {}", max_number + 1)
}

fn parse_camera_number(name: &str) -> Option<u32> {
    let rest = name.strip_prefix("カメラ ")?;
    rest.parse().ok()
}

pub const fn new_camera_shortcut() -> &'static str {
    "Ctrl+Alt+Shift+C"
}

pub const fn new_camera_preset_is_50mm() -> bool {
    true
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CameraSlot {
    pub id: LayerId,
    pub timeline_index: usize,
    pub video_on: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputView {
    Camera(LayerId),
    DefaultComp,
}

/// ビデオがオンの、タイムライン最上段のカメラ。無ければ既定ビュー。
pub fn active_camera(slots: &[CameraSlot]) -> OutputView {
    let mut chosen: Option<&CameraSlot> = None;
    for slot in slots {
        if !slot.video_on {
            continue;
        }
        chosen = Some(match chosen {
            Some(current) if current.timeline_index <= slot.timeline_index => current,
            _ => slot,
        });
    }
    match chosen {
        Some(slot) => OutputView::Camera(slot.id),
        None => OutputView::DefaultComp,
    }
}

pub const ACTIVE_CAMERA_VIEW_SHORTCUT: &str = "F12";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WorkViewKind {
    Front,
    Back,
    Left,
    Right,
    Top,
    Bottom,
    Custom,
}

pub const fn can_edit_default_camera(view: WorkViewKind) -> bool {
    match view {
        WorkViewKind::Top | WorkViewKind::Left | WorkViewKind::Right | WorkViewKind::Bottom => {
            false
        }
        WorkViewKind::Front | WorkViewKind::Back | WorkViewKind::Custom => true,
    }
}

pub const fn default_camera_settings_without_camera_layer() -> bool {
    true
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusCommand {
    LinkToPointOfInterest,
    LinkToLayer(LayerId),
    SetToLayer(LayerId),
}

#[derive(Clone, Debug, PartialEq)]
pub struct FocusBinding {
    pub expression: Option<String>,
    pub tracks: bool,
    pub fixed_distance_px: Option<f64>,
    pub focal_plane_uses_target: Confirmed<bool>,
}

pub fn bind_focus(
    nodes: CameraNodes,
    command: FocusCommand,
    camera_position: [f64; 3],
    target: [f64; 3],
) -> FocusBinding {
    let distance_px = distance(camera_position, target);
    match command {
        FocusCommand::LinkToPointOfInterest => FocusBinding {
            expression: Some(FOCUS_DISTANCE_EXPRESSION.to_string()),
            tracks: true,
            fixed_distance_px: None,
            focal_plane_uses_target: match nodes {
                CameraNodes::Two => Confirmed::Known(true),
                CameraNodes::One => Confirmed::Unconfirmed,
            },
        },
        FocusCommand::LinkToLayer(_) => FocusBinding {
            expression: Some(FOCUS_DISTANCE_EXPRESSION.to_string()),
            tracks: true,
            fixed_distance_px: None,
            focal_plane_uses_target: Confirmed::Known(true),
        },
        FocusCommand::SetToLayer(_) => FocusBinding {
            expression: None,
            tracks: false,
            fixed_distance_px: Some(distance_px),
            focal_plane_uses_target: Confirmed::Known(false),
        },
    }
}

pub fn focus_distance_now(
    binding: &FocusBinding,
    camera_position: [f64; 3],
    target: [f64; 3],
) -> f64 {
    if binding.tracks {
        distance(camera_position, target)
    } else {
        binding.fixed_distance_px.unwrap_or(0.0)
    }
}

pub const CAMERA_EXCLUDED_PROPERTIES: &[&str] = &[
    "アンカーポイント",
    "スケール",
    "不透明度",
    "エフェクト",
    "マスク",
    "ソース",
    "オーディオレベル",
    "タイムリマップ",
    "マテリアル",
];

pub fn camera_has_excluded_property(name: &str) -> bool {
    CAMERA_EXCLUDED_PROPERTIES.contains(&name)
}

/// 矢印キー。現在の拡大率で 1 ピクセル、Shift で 10 ピクセル。戻り値はコンポジション画素。
pub fn nudge_pixels(shift: bool, magnification: f64) -> f64 {
    let screen_pixels = if shift { 10.0 } else { 1.0 };
    screen_pixels / magnification
}

pub fn nudge_degrees(shift: bool) -> f64 {
    if shift {
        10.0
    } else {
        1.0
    }
}

pub const fn camera_preview_keyframes() -> Keyframes {
    Keyframes::Forbidden
}

#[derive(Clone, Debug, PartialEq)]
pub struct CameraDialog {
    pub preview: bool,
    pub shown: Optics,
    pub committed: Optics,
}

impl CameraDialog {
    pub fn open(optics: Optics) -> Self {
        Self {
            preview: true,
            shown: optics,
            committed: optics,
        }
    }

    pub fn set_preview(&mut self, preview: bool) {
        self.preview = preview;
    }

    pub fn edit_zoom(&mut self, zoom_px: f64) {
        self.shown.edit_zoom_in_dialog(zoom_px);
        if self.preview {
            self.committed = self.shown;
        }
    }

    pub fn close(mut self) -> Optics {
        self.committed = self.shown;
        self.committed
    }
}

pub const fn optical_keyframes() -> Keyframes {
    Keyframes::Unconfirmed
}

#[cfg(test)]
mod tests {
    use super::{
        active_camera, angle_of_view_degrees, aperture_changes_exposure, apparent_scale,
        bind_focus, bokeh_circle_closeness, bokeh_stretch, camera_has_excluded_property,
        camera_preview_keyframes, can_edit_default_camera, defocus_pixels,
        depth_of_field_expression_value, diffraction_fringe_range, diffraction_strength,
        engine_defocus_pixels, engine_dof_affects, focus_distance_now, frame_coverage,
        highlight_energy, highlight_gain_expression_range, highlight_gain_slider_range,
        highlight_saturation_color, highlight_threshold_range, new_camera_shortcut,
        next_camera_name, nudge_degrees, nudge_pixels, set_highlight_threshold, set_iris_aspect,
        set_iris_shape, zoom_from_angle, zoom_from_lens, AutoOrient, Camera, CameraDialog,
        CameraNodes, CameraPreset, CameraSlot, Confirmed, FocusCommand, Keyframes, OutputView,
        WorkViewKind, FOCUS_DISTANCE_EXPRESSION,
    };
    use crate::math::{close, close3, length, sub};
    use crate::renderer::Renderer;
    use model::{BitsPerChannel, LayerId};

    fn known_pose(camera: &mut Camera, position: [f64; 3]) {
        camera.position = Confirmed::Known(position);
        camera.orientation = Confirmed::Known([0.0, 0.0, 0.0]);
        camera.rotation_x = Confirmed::Known(0.0);
        camera.rotation_y = Confirmed::Known(0.0);
        camera.rotation_z = Confirmed::Known(0.0);
    }

    fn with_lens(camera: &mut Camera) {
        camera.optics.measured_pixels = Confirmed::Known(1920.0);
        camera.optics.film_size_mm = Confirmed::Known(36.0);
        camera.optics.f_stop = Confirmed::Known(2.0);
        camera.optics.recompute_from_lens();
    }

    #[test]
    fn new_camera_is_named_in_order_with_50mm_preset() {
        assert_eq!(new_camera_shortcut(), "Ctrl+Alt+Shift+C");
        let first = Camera::new(CameraNodes::One, 1920, 1080, &[]);
        let second = Camera::new(CameraNodes::One, 1920, 1080, &["カメラ 1"]);
        assert_eq!(first.name, "カメラ 1");
        assert_eq!(second.name, "カメラ 2");
        assert_eq!(first.optics.preset, CameraPreset::Mm50);
        assert_eq!(first.optics.focal_length_mm, 50.0);
        let from_menu = Camera::new(CameraNodes::Two, 1920, 1080, &[]);
        let from_shortcut = Camera::new(CameraNodes::Two, 1920, 1080, &[]);
        assert_eq!(from_menu.name, from_shortcut.name);
        assert_eq!(from_menu.optics.preset, from_shortcut.optics.preset);
    }

    #[test]
    fn active_camera_is_the_top_enabled_camera_or_the_default_view() {
        let top = LayerId::new(1);
        let bottom = LayerId::new(2);
        let slots = [
            CameraSlot {
                id: top,
                timeline_index: 0,
                video_on: false,
            },
            CameraSlot {
                id: bottom,
                timeline_index: 1,
                video_on: true,
            },
        ];
        assert_eq!(active_camera(&slots), OutputView::Camera(bottom));
        assert_eq!(active_camera(&[]), OutputView::DefaultComp);
        assert_eq!(super::ACTIVE_CAMERA_VIEW_SHORTCUT, "F12");
    }

    #[test]
    fn default_camera_settings_open_without_a_camera_except_four_ortho_views() {
        assert!(super::default_camera_settings_without_camera_layer());
        assert!(!can_edit_default_camera(WorkViewKind::Top));
        assert!(!can_edit_default_camera(WorkViewKind::Left));
        assert!(!can_edit_default_camera(WorkViewKind::Right));
        assert!(!can_edit_default_camera(WorkViewKind::Bottom));
        assert!(can_edit_default_camera(WorkViewKind::Front));
        assert!(can_edit_default_camera(WorkViewKind::Custom));
    }

    #[test]
    fn focus_link_to_point_of_interest_tracks_on_two_node() {
        let position = [0.0, 0.0, 1000.0];
        let interest = [0.0, 0.0, 0.0];
        let binding = bind_focus(
            CameraNodes::Two,
            FocusCommand::LinkToPointOfInterest,
            position,
            interest,
        );
        assert_eq!(
            binding.expression.as_deref(),
            Some(FOCUS_DISTANCE_EXPRESSION)
        );
        assert!(binding.tracks);
        assert_eq!(binding.focal_plane_uses_target, Confirmed::Known(true));
        let moved = [0.0, 0.0, 250.0];
        assert!(close(focus_distance_now(&binding, position, moved), 750.0));
        let one = bind_focus(
            CameraNodes::One,
            FocusCommand::LinkToPointOfInterest,
            position,
            interest,
        );
        assert!(one.expression.is_some());
        assert!(one.focal_plane_uses_target.is_unconfirmed());
    }

    #[test]
    fn focus_link_to_layer_tracks_the_layer() {
        let binding = bind_focus(
            CameraNodes::Two,
            FocusCommand::LinkToLayer(LayerId::new(4)),
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 100.0],
        );
        assert!(binding.tracks);
        assert!(close(
            focus_distance_now(&binding, [0.0, 0.0, 0.0], [0.0, 0.0, 40.0]),
            40.0
        ));
    }

    #[test]
    fn focus_set_to_layer_does_not_track() {
        let binding = bind_focus(
            CameraNodes::One,
            FocusCommand::SetToLayer(LayerId::new(4)),
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 80.0],
        );
        assert!(!binding.tracks);
        assert!(close(binding.fixed_distance_px.unwrap(), 80.0));
        assert!(close(
            focus_distance_now(&binding, [0.0, 0.0, 0.0], [0.0, 0.0, 10.0]),
            80.0
        ));
    }

    #[test]
    fn one_node_ignores_point_of_interest_until_it_becomes_two_node() {
        let mut camera = Camera::new(CameraNodes::One, 100, 50, &[]);
        known_pose(&mut camera, [50.0, 25.0, 400.0]);
        let before = camera.gaze();
        camera.point_of_interest = [10.0, 10.0, 10.0];
        assert_eq!(camera.gaze(), before);
        camera.nodes = CameraNodes::Two;
        let toward = camera.gaze().known().unwrap().forward;
        assert!(close3(
            toward,
            crate::math::normalize(sub(camera.point_of_interest, [50.0, 25.0, 400.0])).unwrap()
        ));
    }

    #[test]
    fn one_node_names_count_from_camera_1() {
        assert_eq!(next_camera_name(&[]), "カメラ 1");
        assert_eq!(next_camera_name(&["カメラ 1"]), "カメラ 2");
    }

    #[test]
    fn one_node_preset_is_50mm_and_a_hand_edit_becomes_custom() {
        let mut camera = Camera::new(CameraNodes::One, 1920, 1080, &[]);
        assert_eq!(camera.optics.preset, CameraPreset::Mm50);
        camera.optics.assign_bundle(800.0, 40.0, 800.0, 35.0, 20.0);
        assert_eq!(camera.optics.focal_length_mm, 35.0);
        assert_eq!(camera.optics.zoom_px, Confirmed::Known(800.0));
        assert_eq!(camera.optics.angle_degrees, Confirmed::Known(40.0));
        assert_eq!(camera.optics.focus_distance_px, Confirmed::Known(800.0));
        assert_eq!(camera.optics.aperture_px, Confirmed::Known(20.0));
        camera.optics.edit_zoom_in_dialog(900.0);
        assert_eq!(camera.optics.preset, CameraPreset::Custom);
    }

    #[test]
    fn one_node_zoom_scales_layers_and_updates_angle() {
        assert!(close(apparent_scale(1000.0, 1000.0), 1.0));
        assert!(close(apparent_scale(1000.0, 2000.0), 0.5));
        let mut camera = Camera::new(CameraNodes::One, 1920, 1080, &[]);
        camera.optics.edit_zoom_in_dialog(1000.0);
        assert!(close(
            camera.optics.angle_degrees.known().unwrap(),
            angle_of_view_degrees(1920.0, 1000.0)
        ));
        camera.optics.lock_to_zoom = Confirmed::Known(true);
        camera.optics.edit_zoom_on_timeline(1100.0);
        assert_eq!(camera.optics.lock_to_zoom, Confirmed::Known(false));
    }

    #[test]
    fn one_node_wider_angle_fits_more_and_follows_lens() {
        let narrow = frame_coverage(100.0, 1000.0, 1000.0, 1920.0);
        let wide_zoom =
            zoom_from_angle(1920.0, angle_of_view_degrees(1920.0, 1000.0) + 10.0).unwrap();
        let wide = frame_coverage(100.0, wide_zoom, 1000.0, 1920.0);
        assert!(wide < narrow);
        let mut camera = Camera::new(CameraNodes::One, 1920, 1080, &[]);
        with_lens(&mut camera);
        let before = camera.optics.angle_degrees;
        camera.optics.set_focal_length_mm(100.0);
        assert_ne!(camera.optics.angle_degrees, before);
    }

    #[test]
    fn one_node_depth_of_field_blurs_only_when_enabled() {
        assert!(depth_of_field_initial_is_unconfirmed());
        let sharp = defocus_pixels(false, 500.0, 1000.0, 40.0, 100.0);
        let blurred = defocus_pixels(true, 500.0, 1000.0, 40.0, 100.0);
        let focused = defocus_pixels(true, 1000.0, 1000.0, 40.0, 100.0);
        assert_eq!(sharp, 0.0);
        assert_eq!(focused, 0.0);
        assert!(blurred > 0.0);
        assert_eq!(depth_of_field_expression_value(true), 1.0);
        assert_eq!(depth_of_field_expression_value(false), 0.0);
    }

    fn depth_of_field_initial_is_unconfirmed() -> bool {
        Camera::new(CameraNodes::One, 100, 100, &[])
            .depth_of_field
            .is_unconfirmed()
    }

    #[test]
    fn one_node_focus_distance_is_sharp_and_timeline_edit_breaks_the_lock() {
        assert_eq!(defocus_pixels(true, 800.0, 800.0, 20.0, 100.0), 0.0);
        assert!(defocus_pixels(true, 400.0, 800.0, 20.0, 100.0) > 0.0);
        let mut camera = Camera::new(CameraNodes::One, 1920, 1080, &[]);
        camera.optics.lock_to_zoom = Confirmed::Known(true);
        camera.optics.edit_zoom_in_dialog(500.0);
        camera.optics.edit_focus_on_timeline(200.0);
        assert_eq!(camera.optics.lock_to_zoom, Confirmed::Known(false));
        assert_eq!(camera.optics.zoom_px, Confirmed::Known(500.0));
    }

    #[test]
    fn one_node_lock_to_zoom_copies_zoom_in_the_dialog_only() {
        let mut camera = Camera::new(CameraNodes::One, 1920, 1080, &[]);
        camera.optics.lock_to_zoom = Confirmed::Known(true);
        camera.optics.edit_zoom_in_dialog(640.0);
        assert_eq!(camera.optics.focus_distance_px, Confirmed::Known(640.0));
        camera.optics.edit_zoom_on_timeline(700.0);
        assert_eq!(camera.optics.focus_distance_px, Confirmed::Known(640.0));
        assert!(camera.optics.lock_to_zoom.is_known());
        assert_eq!(camera.optics.lock_to_zoom, Confirmed::Known(false));
    }

    #[test]
    fn one_node_aperture_changes_blur_and_f_stop_not_exposure() {
        let small = defocus_pixels(true, 100.0, 200.0, 10.0, 100.0);
        let large = defocus_pixels(true, 100.0, 200.0, 30.0, 100.0);
        assert!(large > small);
        assert!(!aperture_changes_exposure());
        let mut camera = Camera::new(CameraNodes::One, 1920, 1080, &[]);
        camera.optics.zoom_px = Confirmed::Known(1000.0);
        camera.optics.edit_aperture_px(50.0);
        assert!(close(camera.optics.f_stop.known().unwrap(), 20.0));
    }

    #[test]
    fn one_node_f_stop_and_aperture_track_each_other() {
        let mut camera = Camera::new(CameraNodes::One, 1920, 1080, &[]);
        camera.optics.zoom_px = Confirmed::Known(1000.0);
        camera.optics.edit_f_stop(4.0);
        assert!(close(camera.optics.aperture_px.known().unwrap(), 250.0));
        camera.optics.edit_aperture_px(100.0);
        assert!(close(camera.optics.f_stop.known().unwrap(), 10.0));
        assert!(!aperture_changes_exposure());
    }

    #[test]
    fn one_node_lower_blur_level_is_weaker() {
        let full = defocus_pixels(true, 100.0, 400.0, 40.0, 100.0);
        let half = defocus_pixels(true, 100.0, 400.0, 40.0, 40.0);
        assert!(half < full);
        assert!(Camera::new(CameraNodes::One, 10, 10, &[])
            .blur_level_percent
            .is_unconfirmed());
    }

    #[test]
    fn one_node_film_size_changes_zoom_angle_and_preset() {
        let mut camera = Camera::new(CameraNodes::One, 1920, 1080, &[]);
        camera.optics.measured_pixels = Confirmed::Known(1920.0);
        camera.optics.set_film_size_mm(36.0);
        let zoom = camera.optics.zoom_px.known().unwrap();
        assert!(close(zoom, zoom_from_lens(50.0, 36.0, 1920.0)));
        camera.optics.set_film_size_mm(18.0);
        assert!(camera.optics.zoom_px.known().unwrap() > zoom);
        assert_eq!(camera.optics.preset, CameraPreset::Custom);
        assert!(camera.optics.angle_degrees.is_known());
    }

    #[test]
    fn one_node_focal_length_starts_at_50mm_and_updates_dependents() {
        let mut camera = Camera::new(CameraNodes::One, 1920, 1080, &[]);
        assert_eq!(camera.optics.focal_length_mm, 50.0);
        with_lens(&mut camera);
        let zoom = camera.optics.zoom_px.known().unwrap();
        let angle = camera.optics.angle_degrees.known().unwrap();
        let aperture = camera.optics.aperture_px.known().unwrap();
        camera.optics.set_focal_length_mm(100.0);
        assert!(camera.optics.zoom_px.known().unwrap() > zoom);
        assert_ne!(camera.optics.angle_degrees.known().unwrap(), angle);
        assert!(camera.optics.aperture_px.known().unwrap() > aperture);
        assert_eq!(camera.optics.preset, CameraPreset::Custom);
    }

    #[test]
    fn one_node_unit_change_keeps_perspective() {
        let mut camera = Camera::new(CameraNodes::One, 1920, 1080, &[]);
        with_lens(&mut camera);
        let zoom = camera.optics.zoom_px;
        camera.optics.set_display_scale(1.0);
        let mm = camera.optics.display_mm(50.0).known().unwrap();
        camera.optics.set_display_scale(0.1);
        assert_ne!(camera.optics.display_mm(50.0).known().unwrap(), mm);
        assert_eq!(camera.optics.zoom_px, zoom);
        assert!(camera.optics.display_per_mm.is_known());
    }

    #[test]
    fn one_node_film_measure_recomputes_zoom() {
        let mut camera = Camera::new(CameraNodes::One, 1920, 1080, &[]);
        camera.optics.film_size_mm = Confirmed::Known(36.0);
        camera.optics.set_measured_pixels(1920.0);
        let wide = camera.optics.zoom_px.known().unwrap();
        camera.optics.set_measured_pixels(1080.0);
        assert!(camera.optics.zoom_px.known().unwrap() < wide);
    }

    #[test]
    fn one_node_iris_shape_is_one_through_ten() {
        assert!(set_iris_shape(0).is_err());
        assert!(set_iris_shape(11).is_err());
        assert_eq!(set_iris_shape(1).unwrap(), 1);
        assert_eq!(set_iris_shape(10).unwrap(), 10);
        assert_ne!(set_iris_shape(1).unwrap(), set_iris_shape(2).unwrap());
    }

    #[test]
    fn one_node_iris_rotation_does_not_change_zoom() {
        assert!(!super::iris_rotation_changes_zoom());
        let mut camera = Camera::new(CameraNodes::One, 100, 100, &[]);
        camera.optics.zoom_px = Confirmed::Known(400.0);
        camera.iris_rotation = Confirmed::Known(15.0);
        assert_eq!(camera.optics.zoom_px, Confirmed::Known(400.0));
    }

    #[test]
    fn one_node_iris_roundness_moves_toward_a_circle() {
        assert!(bokeh_circle_closeness(80.0) > bokeh_circle_closeness(10.0));
    }

    #[test]
    fn one_node_iris_aspect_rejects_outside_1_to_100() {
        assert!(set_iris_aspect(1.0).is_ok());
        assert!(set_iris_aspect(100.0).is_ok());
        assert!(set_iris_aspect(0.9).is_err());
        assert!(set_iris_aspect(100.1).is_err());
        assert!(bokeh_stretch(100.0)[0] > bokeh_stretch(1.0)[0]);
    }

    #[test]
    fn one_node_diffraction_range_stays_unconfirmed_and_strength_rises() {
        assert!(diffraction_fringe_range().is_unconfirmed());
        assert!(diffraction_strength(40.0) > diffraction_strength(5.0));
    }

    #[test]
    fn one_node_highlight_gain_strengthens_pixels_above_threshold() {
        assert_eq!(highlight_gain_expression_range(), (1.0, 100.0));
        assert!(highlight_gain_slider_range().is_unconfirmed());
        let low = highlight_energy(0.8, 0.5, 1.0);
        let high = highlight_energy(0.8, 0.5, 4.0);
        assert!(high > low);
        assert_eq!(highlight_energy(0.2, 0.5, 4.0), 0.0);
    }

    #[test]
    fn one_node_highlight_threshold_follows_bit_depth() {
        assert_eq!(
            highlight_threshold_range(BitsPerChannel::Bpc8),
            (0.0, 100.0)
        );
        assert_eq!(
            highlight_threshold_range(BitsPerChannel::Bpc16),
            (0.0, 32768.0)
        );
        assert_eq!(highlight_threshold_range(BitsPerChannel::Bpc32), (0.0, 1.0));
        assert!(set_highlight_threshold(0.0, BitsPerChannel::Bpc8).is_ok());
        assert!(set_highlight_threshold(100.0, BitsPerChannel::Bpc8).is_ok());
        assert!(set_highlight_threshold(100.1, BitsPerChannel::Bpc8).is_err());
        assert!(set_highlight_threshold(1.0, BitsPerChannel::Bpc32).is_ok());
        assert!(set_highlight_threshold(1.1, BitsPerChannel::Bpc32).is_err());
    }

    #[test]
    fn one_node_highlight_saturation_moves_between_white_and_the_source() {
        let white = highlight_saturation_color(1.0, 0.2, 0.0, 0.0);
        let color = highlight_saturation_color(1.0, 0.2, 0.0, 100.0);
        assert!(close(white[1], 1.0));
        assert!(close(color[1], 0.2));
    }

    #[test]
    fn one_node_focus_area_width_widens_advanced_focus_and_not_classic() {
        let narrow = engine_defocus_pixels(
            Renderer::Advanced3d,
            true,
            110.0,
            100.0,
            20.0,
            100.0,
            4.0,
            100.0,
            100.0,
        );
        let wide = engine_defocus_pixels(
            Renderer::Advanced3d,
            true,
            110.0,
            100.0,
            20.0,
            100.0,
            40.0,
            100.0,
            100.0,
        );
        assert!(narrow > 0.0);
        assert_eq!(wide, 0.0);
        let classic_a = engine_defocus_pixels(
            Renderer::Classic3d,
            true,
            110.0,
            100.0,
            20.0,
            100.0,
            4.0,
            10.0,
            10.0,
        );
        let classic_b = engine_defocus_pixels(
            Renderer::Classic3d,
            true,
            110.0,
            100.0,
            20.0,
            100.0,
            40.0,
            90.0,
            90.0,
        );
        assert!(close(classic_a, classic_b));
        assert_eq!(
            engine_dof_affects(Renderer::Classic3d),
            Confirmed::Known(false)
        );
    }

    #[test]
    fn one_node_near_blur_does_not_change_the_far_side() {
        let near_low = engine_defocus_pixels(
            Renderer::Advanced3d,
            true,
            50.0,
            100.0,
            20.0,
            100.0,
            0.0,
            20.0,
            80.0,
        );
        let near_high = engine_defocus_pixels(
            Renderer::Advanced3d,
            true,
            50.0,
            100.0,
            20.0,
            100.0,
            0.0,
            90.0,
            80.0,
        );
        let far = engine_defocus_pixels(
            Renderer::Advanced3d,
            true,
            150.0,
            100.0,
            20.0,
            100.0,
            0.0,
            20.0,
            80.0,
        );
        let far_same = engine_defocus_pixels(
            Renderer::Advanced3d,
            true,
            150.0,
            100.0,
            20.0,
            100.0,
            0.0,
            90.0,
            80.0,
        );
        assert!(near_high > near_low);
        assert!(close(far, far_same));
    }

    #[test]
    fn one_node_far_blur_does_not_change_the_near_side() {
        let far_low = engine_defocus_pixels(
            Renderer::Advanced3d,
            true,
            180.0,
            100.0,
            20.0,
            100.0,
            0.0,
            40.0,
            10.0,
        );
        let far_high = engine_defocus_pixels(
            Renderer::Advanced3d,
            true,
            180.0,
            100.0,
            20.0,
            100.0,
            0.0,
            40.0,
            90.0,
        );
        let near = engine_defocus_pixels(
            Renderer::Advanced3d,
            true,
            40.0,
            100.0,
            20.0,
            100.0,
            0.0,
            40.0,
            10.0,
        );
        let near_same = engine_defocus_pixels(
            Renderer::Advanced3d,
            true,
            40.0,
            100.0,
            20.0,
            100.0,
            0.0,
            40.0,
            90.0,
        );
        assert!(far_high > far_low);
        assert!(close(near, near_same));
    }

    #[test]
    fn one_node_position_z_changes_apparent_size_and_leaves_interest() {
        let mut camera = Camera::new(CameraNodes::One, 200, 100, &[]);
        let interest = camera.point_of_interest;
        camera.position = Confirmed::Known([100.0, 50.0, 500.0]);
        let at_zoom = apparent_scale(500.0, 500.0);
        camera.position = Confirmed::Known([100.0, 50.0, 1000.0]);
        let farther = apparent_scale(500.0, 1000.0);
        assert!(close(at_zoom, 1.0));
        assert!(close(farther, 0.5));
        assert_eq!(camera.point_of_interest, interest);
        assert!(close(nudge_pixels(false, 1.0), 1.0));
        assert!(close(nudge_pixels(true, 1.0), 10.0));
        assert!(close(nudge_pixels(false, 2.0), 0.5));
    }

    #[test]
    fn one_node_orientation_changes_gaze_without_moving() {
        let mut camera = Camera::new(CameraNodes::One, 100, 100, &[]);
        known_pose(&mut camera, [1.0, 2.0, 3.0]);
        let before = camera.gaze().known().unwrap().forward;
        camera.orientation = Confirmed::Known([0.0, 30.0, 0.0]);
        let after = camera.gaze().known().unwrap().forward;
        assert!((before[0] - after[0]).abs() > 1e-6);
        assert_eq!(camera.position, Confirmed::Known([1.0, 2.0, 3.0]));
    }

    #[test]
    fn one_node_x_rotation_tilts_vertically() {
        let mut camera = Camera::new(CameraNodes::One, 100, 100, &[]);
        known_pose(&mut camera, [0.0, 0.0, 10.0]);
        camera.add_rotation_x(20.0);
        assert_eq!(camera.rotation_x, Confirmed::Known(20.0));
        assert_eq!(camera.rotation_y, Confirmed::Known(0.0));
        assert_eq!(camera.rotation_z, Confirmed::Known(0.0));
        let forward = camera.gaze().known().unwrap().forward;
        assert!(forward[1].abs() > 1e-6);
    }

    #[test]
    fn one_node_y_rotation_turns_sideways() {
        let mut camera = Camera::new(CameraNodes::One, 100, 100, &[]);
        known_pose(&mut camera, [0.0, 0.0, 10.0]);
        camera.add_rotation_y(25.0);
        assert_eq!(camera.rotation_y, Confirmed::Known(25.0));
        assert_eq!(camera.rotation_x, Confirmed::Known(0.0));
        let forward = camera.gaze().known().unwrap().forward;
        assert!(forward[0].abs() > 1e-6);
    }

    #[test]
    fn one_node_z_rotation_rolls_the_frame() {
        let mut camera = Camera::new(CameraNodes::One, 100, 100, &[]);
        known_pose(&mut camera, [4.0, 5.0, 6.0]);
        let position = camera.position;
        camera.add_rotation_z(nudge_degrees(false));
        assert!(close(camera.rotation_z.known().unwrap(), 1.0));
        camera.add_rotation_z(nudge_degrees(true));
        assert!(close(camera.rotation_z.known().unwrap(), 11.0));
        assert_eq!(camera.position, position);
        let gaze = camera.gaze().known().unwrap();
        assert!(close3(gaze.forward, [0.0, 0.0, -1.0]));
        assert!(gaze.up[0].abs() > 1e-6);
    }

    #[test]
    fn one_node_point_of_interest_starts_at_center_and_does_not_aim() {
        let camera = Camera::new(CameraNodes::One, 200, 80, &[]);
        assert!(close3(camera.point_of_interest, [100.0, 40.0, 0.0]));
        let mut aimed = camera.clone();
        known_pose(&mut aimed, [100.0, 40.0, 300.0]);
        let gaze = aimed.gaze();
        aimed.point_of_interest = [0.0, 0.0, 0.0];
        assert_eq!(aimed.gaze(), gaze);
        aimed.nodes = CameraNodes::Two;
        assert!(aimed.gaze().known().unwrap().forward[0].abs() > 1e-6);
    }

    #[test]
    fn one_node_dialog_preview_starts_on() {
        let camera = Camera::new(CameraNodes::One, 100, 100, &[]);
        let mut dialog = CameraDialog::open(camera.optics);
        assert!(dialog.preview);
        dialog.edit_zoom(300.0);
        assert_eq!(dialog.committed.zoom_px, Confirmed::Known(300.0));
        dialog.set_preview(false);
        dialog.shown.edit_zoom_in_dialog(500.0);
        dialog.edit_zoom(500.0);
        assert_eq!(dialog.committed.zoom_px, Confirmed::Known(300.0));
        let closed = dialog.close();
        assert_eq!(closed.zoom_px, Confirmed::Known(500.0));
        assert_eq!(camera_preview_keyframes(), Keyframes::Forbidden);
    }

    #[test]
    fn two_node_aims_at_interest_and_one_node_stops() {
        let mut camera = Camera::new(CameraNodes::Two, 100, 80, &[]);
        known_pose(&mut camera, [10.0, 40.0, 200.0]);
        let forward = camera.gaze().known().unwrap().forward;
        assert!(close3(
            forward,
            crate::math::normalize(sub([50.0, 40.0, 0.0], [10.0, 40.0, 200.0])).unwrap()
        ));
        camera.nodes = CameraNodes::One;
        camera.point_of_interest = [0.0, 0.0, -50.0];
        assert!(close3(
            camera.gaze().known().unwrap().forward,
            [0.0, 0.0, -1.0]
        ));
    }

    #[test]
    fn two_node_first_name_is_camera_1() {
        let camera = Camera::new(CameraNodes::Two, 100, 100, &[]);
        assert_eq!(camera.name, "カメラ 1");
    }

    #[test]
    fn two_node_preset_bundle_updates_five_values() {
        let mut camera = Camera::new(CameraNodes::Two, 1920, 1080, &[]);
        assert_eq!(camera.optics.preset, CameraPreset::Mm50);
        camera.optics.assign_bundle(700.0, 50.0, 700.0, 85.0, 25.0);
        assert_eq!(camera.optics.zoom_px, Confirmed::Known(700.0));
        assert_eq!(camera.optics.angle_degrees, Confirmed::Known(50.0));
        assert_eq!(camera.optics.focus_distance_px, Confirmed::Known(700.0));
        assert_eq!(camera.optics.focal_length_mm, 85.0);
        assert_eq!(camera.optics.aperture_px, Confirmed::Known(25.0));
    }

    #[test]
    fn two_node_zoom_is_independent_of_interest_distance() {
        assert!(close(apparent_scale(400.0, 400.0), 1.0));
        assert!(close(apparent_scale(400.0, 800.0), 0.5));
        let mut camera = Camera::new(CameraNodes::Two, 1920, 1080, &[]);
        known_pose(&mut camera, [960.0, 540.0, 1500.0]);
        let separation = length(sub(
            camera.position.known().unwrap(),
            camera.point_of_interest,
        ));
        camera.optics.edit_zoom_in_dialog(220.0);
        assert!(close(camera.optics.zoom_px.known().unwrap(), 220.0));
        assert!(!close(separation, 220.0));
    }

    #[test]
    fn two_node_angle_ignores_interest_and_follows_focal_length() {
        let mut camera = Camera::new(CameraNodes::Two, 1920, 1080, &[]);
        with_lens(&mut camera);
        let angle = camera.optics.angle_degrees;
        camera.point_of_interest = [10.0, 20.0, 30.0];
        assert_eq!(camera.optics.angle_degrees, angle);
        camera.optics.set_focal_length_mm(28.0);
        assert_ne!(camera.optics.angle_degrees, angle);
    }

    #[test]
    fn two_node_depth_of_field_blurs_outside_focus() {
        assert_eq!(defocus_pixels(false, 10.0, 100.0, 15.0, 100.0), 0.0);
        assert!(defocus_pixels(true, 10.0, 100.0, 15.0, 100.0) > 0.0);
    }

    #[test]
    fn two_node_focus_link_matches_interest_distance() {
        let mut binding = bind_focus(
            CameraNodes::Two,
            FocusCommand::LinkToPointOfInterest,
            [0.0, 0.0, 300.0],
            [0.0, 0.0, 0.0],
        );
        assert!(close(
            focus_distance_now(&binding, [0.0, 0.0, 300.0], [0.0, 0.0, 0.0]),
            300.0
        ));
        binding = bind_focus(
            CameraNodes::Two,
            FocusCommand::LinkToPointOfInterest,
            [0.0, 0.0, 300.0],
            [0.0, 0.0, 100.0],
        );
        assert!(close(
            focus_distance_now(&binding, [0.0, 0.0, 300.0], [0.0, 0.0, 100.0]),
            200.0
        ));
    }

    #[test]
    fn two_node_lock_to_zoom_matches_focus_in_the_dialog() {
        let mut camera = Camera::new(CameraNodes::Two, 800, 600, &[]);
        camera.optics.lock_to_zoom = Confirmed::Known(true);
        camera.optics.edit_zoom_in_dialog(333.0);
        assert_eq!(camera.optics.focus_distance_px, camera.optics.zoom_px);
    }

    #[test]
    fn two_node_aperture_tracks_f_stop_without_exposure() {
        let mut camera = Camera::new(CameraNodes::Two, 800, 600, &[]);
        camera.optics.zoom_px = Confirmed::Known(500.0);
        camera.optics.edit_aperture_px(25.0);
        assert!(close(camera.optics.f_stop.known().unwrap(), 20.0));
        assert!(!aperture_changes_exposure());
    }

    #[test]
    fn two_node_f_stop_updates_aperture() {
        let mut camera = Camera::new(CameraNodes::Two, 800, 600, &[]);
        camera.optics.zoom_px = Confirmed::Known(500.0);
        camera.optics.edit_f_stop(8.0);
        assert!(close(camera.optics.aperture_px.known().unwrap(), 62.5));
        camera.optics.edit_aperture_px(50.0);
        assert!(close(camera.optics.f_stop.known().unwrap(), 10.0));
    }

    #[test]
    fn two_node_blur_level_reduces_blur() {
        assert!(
            defocus_pixels(true, 20.0, 80.0, 16.0, 25.0)
                < defocus_pixels(true, 20.0, 80.0, 16.0, 100.0)
        );
    }

    #[test]
    fn two_node_film_size_updates_zoom_and_angle() {
        let mut camera = Camera::new(CameraNodes::Two, 1920, 1080, &[]);
        camera.optics.measured_pixels = Confirmed::Known(1000.0);
        camera.optics.set_film_size_mm(50.0);
        let zoom = camera.optics.zoom_px;
        let angle = camera.optics.angle_degrees;
        camera.optics.set_film_size_mm(25.0);
        assert_ne!(camera.optics.zoom_px, zoom);
        assert_ne!(camera.optics.angle_degrees, angle);
    }

    #[test]
    fn two_node_focal_length_defaults_to_50mm() {
        let camera = Camera::new(CameraNodes::Two, 640, 480, &[]);
        assert_eq!(camera.optics.focal_length_mm, 50.0);
        assert_eq!(camera.optics.preset, CameraPreset::Mm50);
    }

    #[test]
    fn two_node_unit_change_changes_display_only() {
        let mut camera = Camera::new(CameraNodes::Two, 640, 480, &[]);
        with_lens(&mut camera);
        let zoom = camera.optics.zoom_px;
        camera.optics.set_display_scale(1.0);
        camera.optics.set_display_scale(2.0);
        assert!(close(
            camera.optics.display_mm(50.0).known().unwrap(),
            100.0
        ));
        assert_eq!(camera.optics.zoom_px, zoom);
    }

    #[test]
    fn two_node_film_measure_recomputes_zoom() {
        let mut camera = Camera::new(CameraNodes::Two, 640, 480, &[]);
        camera.optics.film_size_mm = Confirmed::Known(36.0);
        camera.optics.set_measured_pixels(640.0);
        let first = camera.optics.zoom_px.known().unwrap();
        camera.optics.set_measured_pixels(480.0);
        assert!(camera.optics.zoom_px.known().unwrap() < first);
    }

    #[test]
    fn two_node_iris_shape_changes_bokeh_id() {
        assert_ne!(set_iris_shape(3).unwrap(), set_iris_shape(9).unwrap());
    }

    #[test]
    fn two_node_iris_rotation_keeps_zoom() {
        assert!(!super::iris_rotation_changes_zoom());
    }

    #[test]
    fn two_node_iris_roundness_increases_toward_circle() {
        assert!(bokeh_circle_closeness(100.0) > bokeh_circle_closeness(0.0));
    }

    #[test]
    fn two_node_iris_aspect_endpoints_stretch() {
        assert!(set_iris_aspect(1.0).is_ok());
        assert!(set_iris_aspect(100.0).is_ok());
        assert!(set_iris_aspect(0.0).is_err());
        assert!(bokeh_stretch(100.0)[0] / bokeh_stretch(100.0)[1] > bokeh_stretch(1.0)[0]);
    }

    #[test]
    fn two_node_diffraction_range_is_unconfirmed() {
        assert!(diffraction_fringe_range().is_unconfirmed());
        assert!(diffraction_strength(12.0) > diffraction_strength(1.0));
    }

    #[test]
    fn two_node_highlight_gain_increases_bright_blur_energy() {
        assert!(highlight_energy(90.0, 10.0, 5.0) > highlight_energy(90.0, 10.0, 1.0));
    }

    #[test]
    fn two_node_highlight_threshold_switches_with_bit_depth() {
        assert_eq!(highlight_threshold_range(BitsPerChannel::Bpc8).1, 100.0);
        assert_eq!(highlight_threshold_range(BitsPerChannel::Bpc16).1, 32768.0);
        assert_eq!(highlight_threshold_range(BitsPerChannel::Bpc32).1, 1.0);
    }

    #[test]
    fn two_node_highlight_saturation_keeps_or_washes_color() {
        let washed = highlight_saturation_color(0.2, 0.4, 0.6, 0.0);
        let kept = highlight_saturation_color(0.2, 0.4, 0.6, 100.0);
        assert!(close(washed[0], 1.0));
        assert!(close(kept[0], 0.2));
    }

    #[test]
    fn two_node_focus_area_width_extends_the_sharp_band() {
        let tight = engine_defocus_pixels(
            Renderer::Advanced3d,
            true,
            130.0,
            100.0,
            10.0,
            100.0,
            10.0,
            100.0,
            100.0,
        );
        let loose = engine_defocus_pixels(
            Renderer::Advanced3d,
            true,
            130.0,
            100.0,
            10.0,
            100.0,
            80.0,
            100.0,
            100.0,
        );
        assert!(tight > 0.0);
        assert_eq!(loose, 0.0);
    }

    #[test]
    fn two_node_near_blur_leaves_the_far_side() {
        let changed = engine_defocus_pixels(
            Renderer::Advanced3d,
            true,
            40.0,
            100.0,
            10.0,
            100.0,
            0.0,
            10.0,
            50.0,
        );
        let stronger = engine_defocus_pixels(
            Renderer::Advanced3d,
            true,
            40.0,
            100.0,
            10.0,
            100.0,
            0.0,
            70.0,
            50.0,
        );
        let far = engine_defocus_pixels(
            Renderer::Advanced3d,
            true,
            160.0,
            100.0,
            10.0,
            100.0,
            0.0,
            10.0,
            50.0,
        );
        let far_again = engine_defocus_pixels(
            Renderer::Advanced3d,
            true,
            160.0,
            100.0,
            10.0,
            100.0,
            0.0,
            70.0,
            50.0,
        );
        assert!(stronger > changed);
        assert!(close(far, far_again));
    }

    #[test]
    fn two_node_far_blur_leaves_the_near_side() {
        let changed = engine_defocus_pixels(
            Renderer::Advanced3d,
            true,
            200.0,
            100.0,
            10.0,
            100.0,
            0.0,
            30.0,
            10.0,
        );
        let stronger = engine_defocus_pixels(
            Renderer::Advanced3d,
            true,
            200.0,
            100.0,
            10.0,
            100.0,
            0.0,
            30.0,
            80.0,
        );
        let near = engine_defocus_pixels(
            Renderer::Advanced3d,
            true,
            20.0,
            100.0,
            10.0,
            100.0,
            0.0,
            30.0,
            10.0,
        );
        let near_again = engine_defocus_pixels(
            Renderer::Advanced3d,
            true,
            20.0,
            100.0,
            10.0,
            100.0,
            0.0,
            30.0,
            80.0,
        );
        assert!(stronger > changed);
        assert!(close(near, near_again));
    }

    #[test]
    fn two_node_moving_sideways_keeps_interest_and_changes_gaze() {
        let mut camera = Camera::new(CameraNodes::Two, 200, 100, &[]);
        known_pose(&mut camera, [100.0, 50.0, 400.0]);
        let interest = camera.point_of_interest;
        let before = camera.gaze().known().unwrap().forward;
        camera.position = Confirmed::Known([160.0, 50.0, 400.0]);
        assert_eq!(camera.point_of_interest, interest);
        assert!(!close3(camera.gaze().known().unwrap().forward, before));
    }

    #[test]
    fn two_node_orientation_composition_stays_unconfirmed() {
        let mut camera = Camera::new(CameraNodes::Two, 200, 100, &[]);
        known_pose(&mut camera, [100.0, 50.0, 400.0]);
        camera.orientation = Confirmed::Known([15.0, 0.0, 0.0]);
        assert!(camera.gaze().is_unconfirmed());
        camera.auto_orient = AutoOrient::Other;
        camera.orientation = Confirmed::Known([0.0, 0.0, 0.0]);
        camera.point_of_interest = [0.0, 0.0, 0.0];
        assert!(close3(
            camera.gaze().known().unwrap().forward,
            [0.0, 0.0, -1.0]
        ));
    }

    #[test]
    fn two_node_x_rotation_value_changes_alone() {
        let mut camera = Camera::new(CameraNodes::Two, 100, 100, &[]);
        known_pose(&mut camera, [0.0, 0.0, 10.0]);
        camera.add_rotation_x(12.0);
        assert_eq!(camera.rotation_x, Confirmed::Known(12.0));
        assert_eq!(camera.rotation_y, Confirmed::Known(0.0));
        assert_eq!(camera.rotation_z, Confirmed::Known(0.0));
    }

    #[test]
    fn two_node_y_rotation_value_changes_alone() {
        let mut camera = Camera::new(CameraNodes::Two, 100, 100, &[]);
        known_pose(&mut camera, [0.0, 0.0, 10.0]);
        camera.add_rotation_y(-8.0);
        assert_eq!(camera.rotation_y, Confirmed::Known(-8.0));
        assert_eq!(camera.rotation_x, Confirmed::Known(0.0));
        assert_eq!(camera.rotation_z, Confirmed::Known(0.0));
    }

    #[test]
    fn two_node_z_rotation_rolls_without_moving_interest() {
        let mut camera = Camera::new(CameraNodes::Two, 100, 80, &[]);
        known_pose(&mut camera, [50.0, 40.0, 200.0]);
        let interest = camera.point_of_interest;
        let forward = camera.gaze().known().unwrap().forward;
        camera.add_rotation_z(nudge_degrees(false));
        camera.add_rotation_z(nudge_degrees(true));
        assert!(close(camera.rotation_z.known().unwrap(), 11.0));
        assert_eq!(camera.point_of_interest, interest);
        assert!(close3(camera.gaze().known().unwrap().forward, forward));
        assert!(!close3(camera.gaze().known().unwrap().up, [0.0, -1.0, 0.0]));
    }

    #[test]
    fn two_node_interest_starts_at_center_and_moving_it_keeps_position() {
        let mut camera = Camera::new(CameraNodes::Two, 300, 100, &[]);
        assert!(close3(camera.point_of_interest, [150.0, 50.0, 0.0]));
        known_pose(&mut camera, [150.0, 50.0, 250.0]);
        let position = camera.position;
        camera.point_of_interest = [10.0, 50.0, 0.0];
        assert_eq!(camera.position, position);
        assert!(camera.gaze().known().unwrap().forward[0].abs() > 1e-6);
    }

    #[test]
    fn two_node_dialog_preview_starts_on_and_can_hold_updates() {
        let camera = Camera::new(CameraNodes::Two, 100, 100, &[]);
        let mut dialog = CameraDialog::open(camera.optics);
        assert!(dialog.preview);
        dialog.set_preview(false);
        let committed = dialog.committed.zoom_px;
        dialog.edit_zoom(250.0);
        assert_eq!(dialog.committed.zoom_px, committed);
        assert_eq!(dialog.close().zoom_px, Confirmed::Known(250.0));
    }

    #[test]
    fn cameras_exclude_anchor_scale_opacity_and_materials() {
        for name in super::CAMERA_EXCLUDED_PROPERTIES {
            assert!(!camera_has_excluded_property_inverted(name));
        }
    }

    fn camera_has_excluded_property_inverted(name: &str) -> bool {
        !camera_has_excluded_property(name)
    }
}
