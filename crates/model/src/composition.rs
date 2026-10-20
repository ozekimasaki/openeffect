use crate::color::Rgba;
use crate::id::ItemId;
use crate::keyframe::{Keyframe, TemporalInterpolation, TimedMarker};
use crate::layer::Layer;
use crate::time::Time;

/// プレビュー解像度。Third、Quarter、Auto はメニューにあるか要確認なので置かない。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Resolution {
    Full,
    Half,
    Custom { x_denom: u32, y_denom: u32 },
}

impl Resolution {
    pub fn preview_size(self, width: u32, height: u32) -> Option<(u32, u32)> {
        match self {
            Self::Full => Some((width, height)),
            // Half は列と行をそれぞれ半分にする。端数は切り捨てる。
            Self::Half => Some((width / 2, height / 2)),
            Self::Custom { x_denom, y_denom } => {
                if x_denom == 0 || y_denom == 0 {
                    None
                } else {
                    Some((width / x_denom, height / y_denom))
                }
            }
        }
    }
}

/// ヘルプが名前を確認している解像度だけ。
pub fn confirmed_resolution_menu() -> &'static [&'static str] {
    &["Full", "Half", "Custom"]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum EdgeAnchor {
    #[default]
    Start,
    Center,
    End,
}

/// サイズ変更で固定する辺。ボタンの個数は要確認なので、縦横の辺だけを持つ。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct CompAnchor {
    pub horizontal: EdgeAnchor,
    pub vertical: EdgeAnchor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Renderer {
    #[default]
    Classic3d,
    Advanced3d,
    Cinema4d,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Cinema4dAntiAlias {
    /// 初期は Geometry。縁を 16×16 サブピクセルで滑らかにする。
    #[default]
    Geometry,
    Other,
}

pub const CINEMA4D_GEOMETRY_SUBPIXELS: u32 = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Cinema4dReflectance {
    #[default]
    LayerSampling,
    Other,
}

/// Quality スライダが束ねる段数の対応表は要確認。両端の大小だけを持つ。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Cinema4dDepth {
    Low,
    High,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cinema4dOptions {
    pub anti_aliasing: Cinema4dAntiAlias,
    pub geometry_subpixels: u32,
    pub reflectance: Cinema4dReflectance,
    pub quality_low: bool,
    pub installation_path: Option<&'static str>,
}

impl Default for Cinema4dOptions {
    fn default() -> Self {
        Self {
            anti_aliasing: Cinema4dAntiAlias::Geometry,
            geometry_subpixels: CINEMA4D_GEOMETRY_SUBPIXELS,
            reflectance: Cinema4dReflectance::LayerSampling,
            quality_low: false,
            installation_path: None,
        }
    }
}

impl Cinema4dOptions {
    pub fn reflection_depth(self) -> Cinema4dDepth {
        if self.quality_low {
            Cinema4dDepth::Low
        } else {
            Cinema4dDepth::High
        }
    }

    pub fn shadow_depth(self) -> Cinema4dDepth {
        self.reflection_depth()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum NameColumn {
    #[default]
    Layer,
    Source,
}

impl NameColumn {
    pub const fn toggle(self) -> Self {
        match self {
            Self::Layer => Self::Source,
            Self::Source => Self::Layer,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum ShowProperties {
    #[default]
    Selected,
    Animated,
    GraphEditorSet,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum GraphTypeMode {
    #[default]
    Auto,
    Value,
    Speed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GraphKind {
    Value,
    Speed,
}

/// 時間だけのプロパティは値、空間プロパティは速度。
pub fn auto_graph_kind(spatial: bool) -> GraphKind {
    if spatial {
        GraphKind::Speed
    } else {
        GraphKind::Value
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GraphEditor {
    pub open: bool,
    pub show: ShowProperties,
    pub graph_type: GraphTypeMode,
    pub show_reference: bool,
    pub show_audio: bool,
    pub show_layer_in_out: bool,
    pub show_layer_markers: bool,
    pub show_tooltips: bool,
    pub show_expression_editor: bool,
    pub allow_keyframes_between_frames: bool,
    pub snap: bool,
    /// 吸着距離。初期値は要確認なので、使う側が入れる。
    pub snap_threshold: Option<f64>,
    pub auto_zoom_height: bool,
    pub separate_dimensions: bool,
    pub pan: [f64; 2],
    pub zoom: [f64; 2],
    /// グラフエディタセットに入っている match_name。
    pub graph_set: Vec<String>,
}

impl Default for GraphEditor {
    fn default() -> Self {
        Self {
            open: false,
            show: ShowProperties::Selected,
            graph_type: GraphTypeMode::Auto,
            show_reference: false,
            show_audio: false,
            show_layer_in_out: false,
            show_layer_markers: false,
            show_tooltips: true,
            show_expression_editor: false,
            allow_keyframes_between_frames: false,
            snap: false,
            snap_threshold: None,
            auto_zoom_height: false,
            separate_dimensions: false,
            pan: [0.0, 0.0],
            zoom: [1.0, 1.0],
            graph_set: Vec::new(),
        }
    }
}

/// タイムラインの列。キーフレーム列がメニューにあるかは要確認なので、確定した列だけ。
#[derive(Clone, Debug, PartialEq)]
pub struct TimelineColumns {
    pub av_features: bool,
    pub label: bool,
    pub comment: bool,
    pub switches: bool,
    pub modes: bool,
    pub parent: bool,
    pub in_point: bool,
    pub out_point: bool,
    pub duration: bool,
    pub stretch: bool,
    pub name_column: NameColumn,
}

impl Default for TimelineColumns {
    fn default() -> Self {
        Self {
            av_features: true,
            label: true,
            comment: false,
            switches: true,
            modes: false,
            parent: false,
            in_point: false,
            out_point: false,
            duration: false,
            stretch: false,
            name_column: NameColumn::Layer,
        }
    }
}

impl TimelineColumns {
    /// F4。Switches と Modes を入れ替える。
    pub fn toggle_switches_modes(&mut self) {
        if self.switches {
            self.switches = false;
            self.modes = true;
        } else {
            self.switches = true;
            self.modes = false;
        }
    }

    /// Shift+F4。Parent 列の表示を切り替える。
    pub fn toggle_parent(&mut self) {
        self.parent = !self.parent;
    }
}

/// ヘルプが列として書いているもの。キーフレーム列は含まない。
pub fn confirmed_timeline_columns() -> &'static [&'static str] {
    &[
        "A/V Features",
        "Label",
        "Comment",
        "Switches",
        "Modes",
        "Parent",
        "In",
        "Out",
        "Duration",
        "Stretch",
    ]
}

/// プリセットに保存される 4 値。
#[derive(Clone, Debug, PartialEq)]
pub struct CompPreset {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub pixel_aspect: f64,
    pub frame_rate: f64,
    pub bundled: bool,
}

/// グラフに出すプロパティの選択。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphPropertyRef {
    pub match_name: String,
    pub spatial: bool,
    pub animated: bool,
    pub selected: bool,
}

/// モーションブラーのシャッター。角度の単位は度。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MotionBlurSettings {
    pub shutter_angle_degrees: f64,
    pub shutter_phase_degrees: f64,
    pub samples_per_frame: u32,
    pub adaptive_sample_limit: u32,
}

impl Default for MotionBlurSettings {
    fn default() -> Self {
        Self {
            shutter_angle_degrees: 0.0,
            shutter_phase_degrees: 0.0,
            samples_per_frame: 1,
            adaptive_sample_limit: 1,
        }
    }
}

/// コンポジション。`layers[0]` がタイムラインの先頭（手前）。
#[derive(Clone, Debug, PartialEq)]
pub struct Composition {
    pub id: ItemId,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub pixel_aspect: f64,
    pub frame_rate: f64,
    pub duration: Time,
    pub work_area_start: Time,
    pub work_area_duration: Time,
    pub layers: Vec<Layer>,
    pub motion_blur: MotionBlurSettings,
    pub drop_frame: bool,
    pub start: Time,
    pub background: Rgba,
    pub resolution: Resolution,
    pub anchor: CompAnchor,
    pub preserve_frame_rate: bool,
    pub preserve_resolution: bool,
    pub renderer: Renderer,
    pub cinema4d: Cinema4dOptions,
    /// 影の解像度の分母。項目名と初期値は要確認。
    pub shadow_resolution_divisor: Option<u32>,
    pub hide_shy_layers: bool,
    pub enable_motion_blur: bool,
    pub enable_frame_blending: bool,
    pub markers: Vec<TimedMarker>,
    pub markers_locked: bool,
    pub columns: TimelineColumns,
    pub graph: GraphEditor,
    pub label: Option<u8>,
    /// コンポジションパネルのタブ色。ラベル色を使う設定がオフのときは None。
    pub tab_uses_label: bool,
}

impl Composition {
    pub fn new(
        id: ItemId,
        name: impl Into<String>,
        width: u32,
        height: u32,
        frame_rate: f64,
        duration: Time,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            width,
            height,
            pixel_aspect: 1.0,
            frame_rate,
            duration,
            work_area_start: Time::ZERO,
            work_area_duration: duration,
            layers: Vec::new(),
            motion_blur: MotionBlurSettings::default(),
            drop_frame: false,
            start: Time::ZERO,
            background: Rgba::black(),
            resolution: Resolution::Full,
            anchor: CompAnchor::default(),
            preserve_frame_rate: false,
            preserve_resolution: false,
            renderer: Renderer::Classic3d,
            cinema4d: Cinema4dOptions::default(),
            shadow_resolution_divisor: None,
            hide_shy_layers: false,
            enable_motion_blur: false,
            enable_frame_blending: false,
            markers: Vec::new(),
            markers_locked: false,
            columns: TimelineColumns::default(),
            graph: GraphEditor::default(),
            label: None,
            tab_uses_label: false,
        }
    }

    pub fn frame_duration(&self) -> f64 {
        if self.frame_rate <= 0.0 {
            0.0
        } else {
            1.0 / self.frame_rate
        }
    }

    /// 最終フレームの開始時刻。尺ちょうどは含まない。
    pub fn last_frame_time(&self) -> Option<Time> {
        if self.frame_rate <= 0.0 || self.duration.seconds() <= 0.0 {
            return None;
        }
        let frames = self.duration.to_frames(self.frame_rate).floor();
        if frames <= 0.0 {
            return None;
        }
        Some(Time::from_frames(frames - 1.0, self.frame_rate))
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        let old_w = self.width as f64;
        let old_h = self.height as f64;
        let dw = width as f64 - old_w;
        let dh = height as f64 - old_h;
        for layer in &mut self.layers {
            layer.transform.position[0] += edge_shift(self.anchor.horizontal, dw);
            layer.transform.position[1] += edge_shift(self.anchor.vertical, dh);
        }
        self.width = width;
        self.height = height;
    }

    pub fn preview_playing_add_marker(&mut self, layer: Option<crate::id::LayerId>, time: Time) -> bool {
        let playing = true;
        self.add_marker(layer, time, false);
        playing
    }

    pub fn add_marker(&mut self, layer: Option<crate::id::LayerId>, time: Time, open_dialog: bool) -> bool {
        let marker = if layer.is_some() {
            TimedMarker::at(time, crate::keyframe::Marker::layer_default())
        } else {
            let number = self.markers.len() as u8;
            let mut timed = TimedMarker::at(time, crate::keyframe::Marker::comp_numbered(number));
            timed.number = Some(number);
            timed
        };
        if let Some(layer_id) = layer {
            if let Some(layer) = self.layer_mut(layer_id) {
                layer.markers.push(marker);
            }
        } else {
            self.markers.push(marker);
        }
        let _ = open_dialog;
        open_dialog
    }

    /// 同じ番号があれば動かして、無ければ現在時刻に置く。他の番号は詰めない。
    pub fn place_numbered_marker(&mut self, number: u8, time: Time) {
        if let Some(existing) = self.markers.iter_mut().find(|marker| marker.number == Some(number)) {
            existing.time = time;
            return;
        }
        let mut timed = TimedMarker::at(time, crate::keyframe::Marker::comp_numbered(number));
        timed.number = Some(number);
        self.markers.push(timed);
    }

    pub fn delete_marker_at(&mut self, time: Time) {
        self.markers.retain(|marker| marker.time != time);
    }

    pub fn marker_numbers(&self) -> Vec<u8> {
        self.markers.iter().filter_map(|marker| marker.number).collect()
    }

    /// 保護していない区間だけを伸ばす。
    pub fn stretch_protecting(&mut self, new_duration: Time) {
        let old = self.duration.seconds();
        let new = new_duration.seconds();
        if old <= 0.0 || new <= 0.0 {
            return;
        }
        let protected: f64 = self
            .markers
            .iter()
            .filter(|marker| marker.marker.protected_region)
            .map(|marker| marker.marker.duration_seconds)
            .sum();
        let unprotected = (old - protected).max(0.0);
        let extra = new - old;
        let scale = if unprotected == 0.0 {
            1.0
        } else {
            (unprotected + extra) / unprotected
        };
        let mut placed = 0.0;
        let snapshot: Vec<(usize, f64, f64)> = self
            .markers
            .iter()
            .enumerate()
            .filter(|(_, marker)| marker.marker.protected_region)
            .map(|(index, marker)| (index, marker.time.seconds(), marker.marker.duration_seconds))
            .collect();
        let mut cursor = 0.0;
        for (index, start, len) in snapshot {
            let gap = (start - cursor).max(0.0);
            placed += gap * scale;
            self.markers[index].time = Time::from_seconds(placed);
            self.markers[index].marker.duration_seconds = len;
            placed += len;
            cursor = start + len;
        }
        self.duration = new_duration;
    }

    pub fn create_intro_protected(&mut self) {
        let length = self.duration.seconds() * 0.15;
        self.push_protected(Time::ZERO, length);
    }

    pub fn create_outro_protected(&mut self) {
        let length = self.duration.seconds() * 0.15;
        let start = (self.duration.seconds() - length).max(0.0);
        self.push_protected(Time::from_seconds(start), length);
    }

    pub fn create_protected_from_work_area(&mut self) {
        self.push_protected(self.work_area_start, self.work_area_duration.seconds());
    }

    fn push_protected(&mut self, time: Time, length: f64) {
        let mut marker = crate::keyframe::Marker::comp_numbered(self.markers.len() as u8);
        marker.protected_region = true;
        marker.duration_seconds = length;
        let mut timed = TimedMarker::at(time, marker);
        timed.number = Some(self.markers.len() as u8);
        self.markers.push(timed);
    }

    pub fn update_markers_from_source(&mut self, layer_id: crate::id::LayerId, source_markers: &[TimedMarker]) {
        if let Some(layer) = self.layer_mut(layer_id) {
            layer.markers = source_markers.to_vec();
        }
    }

    /// グラフに出す曲線。
    pub fn visible_graphs<'a>(&self, properties: &'a [GraphPropertyRef]) -> Vec<&'a GraphPropertyRef> {
        match self.graph.show {
            ShowProperties::Selected => properties.iter().filter(|item| item.selected).collect(),
            ShowProperties::Animated => properties.iter().filter(|item| item.animated).collect(),
            ShowProperties::GraphEditorSet => properties
                .iter()
                .filter(|item| self.graph.graph_set.iter().any(|name| name == &item.match_name))
                .collect(),
        }
    }

    pub fn graph_kind_for(&self, spatial: bool) -> GraphKind {
        match self.graph.graph_type {
            GraphTypeMode::Auto => auto_graph_kind(spatial),
            GraphTypeMode::Value => GraphKind::Value,
            GraphTypeMode::Speed => GraphKind::Speed,
        }
    }

    pub fn snap_key_time(&self, time: Time, targets: &[Time], ctrl_held: bool) -> Time {
        let enabled = if ctrl_held { !self.graph.snap } else { self.graph.snap };
        let Some(threshold) = self.graph.snap_threshold else {
            return time;
        };
        if !enabled {
            return time;
        }
        let mut best: Option<(f64, Time)> = None;
        for target in targets {
            let distance = (target.seconds() - time.seconds()).abs();
            if distance <= threshold {
                match best {
                    Some((best_distance, _)) if distance >= best_distance => {}
                    _ => best = Some((distance, *target)),
                }
            }
        }
        best.map(|(_, target)| target).unwrap_or(time)
    }

    pub fn quantize_key_time(&self, time: Time) -> Time {
        if self.graph.allow_keyframes_between_frames || self.frame_rate <= 0.0 {
            return time;
        }
        let frame = time.to_frames(self.frame_rate).round();
        Time::from_frames(frame, self.frame_rate)
    }

    pub fn fit_graph_to_times(&mut self, times: &[Time]) {
        if times.is_empty() {
            return;
        }
        let mut min = times[0].seconds();
        let mut max = min;
        for time in times {
            min = min.min(time.seconds());
            max = max.max(time.seconds());
        }
        self.graph.pan[0] = min;
        self.graph.zoom[0] = (max - min).max(self.frame_duration());
    }

    pub fn set_graph_interpolation(&self, key: &mut Keyframe, temporal: TemporalInterpolation) {
        key.temporal_in = temporal;
        key.temporal_out = temporal;
    }

    pub fn layer(&self, id: crate::id::LayerId) -> Option<&Layer> {
        self.layers.iter().find(|layer| layer.id == id)
    }

    pub fn layer_mut(&mut self, id: crate::id::LayerId) -> Option<&mut Layer> {
        self.layers.iter_mut().find(|layer| layer.id == id)
    }
}

fn edge_shift(edge: EdgeAnchor, delta: f64) -> f64 {
    match edge {
        EdgeAnchor::Start => 0.0,
        EdgeAnchor::Center => delta * 0.5,
        EdgeAnchor::End => delta,
    }
}
