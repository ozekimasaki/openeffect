use crate::bits::BitsPerChannel;
use crate::composition::{
    Cinema4dOptions, CompAnchor, CompPreset, Composition, MotionBlurSettings, Renderer, Resolution,
};
use crate::folder::{Folder, FolderChild};
use crate::footage::{Footage, Interpretation};
use crate::id::{Ids, ItemId};
use crate::time::{FrameNumbering, Time};

pub const SOLIDS_FOLDER: &str = "Solids";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum ColorEngine {
    #[default]
    AdobeColorManaged,
    OcioColorManaged,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OcioConfig {
    Aces20CgV4,
    Aces20StudioV4,
    Custom(String),
}

/// 作業用スペース。Off は None。初期に Off かどうかは要確認。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkingSpace {
    Off,
    SdtvNtsc,
    SdtvPal,
    HdtvRec709,
    Srgb,
    AdobeRgb,
    ProPhotoRgb,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColorSettings {
    pub engine: ColorEngine,
    /// 初期の OCIO 設定は要確認。
    pub ocio_config: Option<OcioConfig>,
    /// 未設定は None。Off とは別。
    pub working_space: Option<WorkingSpace>,
    pub display_color_space: Option<String>,
    pub linearize_working_space: bool,
    pub blend_in_gamma_one: bool,
}

impl Default for ColorSettings {
    fn default() -> Self {
        Self {
            engine: ColorEngine::AdobeColorManaged,
            ocio_config: None,
            working_space: None,
            display_color_space: None,
            linearize_working_space: false,
            blend_in_gamma_one: false,
        }
    }
}

impl ColorSettings {
    pub fn icc_profiles_available(&self) -> bool {
        self.engine == ColorEngine::AdobeColorManaged
    }

    pub fn ocio_menus_visible(&self) -> bool {
        self.engine == ColorEngine::OcioColorManaged
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum TimeDisplayStyle {
    #[default]
    Timecode,
    Frames,
}

impl TimeDisplayStyle {
    pub const fn toggle(self) -> Self {
        match self {
            Self::Timecode => Self::Frames,
            Self::Frames => Self::Timecode,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TimecodeOrigin {
    MediaSource,
    Zero,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FilmGauge {
    Mm16,
    Mm35,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimeDisplay {
    pub style: TimeDisplayStyle,
    pub origin: Option<TimecodeOrigin>,
    pub numbering: Option<FrameNumbering>,
    pub feet_and_frames: bool,
    pub gauge: Option<FilmGauge>,
    /// 1 フィートのフレーム数。初期値は要確認。
    pub frames_per_foot_16: Option<u32>,
    pub frames_per_foot_35: Option<u32>,
}

impl Default for TimeDisplay {
    fn default() -> Self {
        Self {
            style: TimeDisplayStyle::Timecode,
            origin: None,
            numbering: None,
            feet_and_frames: false,
            gauge: None,
            frames_per_foot_16: None,
            frames_per_foot_35: None,
        }
    }
}

impl TimeDisplay {
    pub fn frames_per_foot(&self) -> Option<u32> {
        match self.gauge {
            Some(FilmGauge::Mm16) => self.frames_per_foot_16,
            Some(FilmGauge::Mm35) => self.frames_per_foot_35,
            None => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LabelColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LabelDefinition {
    pub name: String,
    /// 初期の色値は要確認。
    pub color: Option<LabelColor>,
}

impl LabelDefinition {
    pub fn unnamed(index: u8) -> Self {
        Self {
            name: format!("Label {index}"),
            color: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct LabelDefaults {
    pub composition: Option<u8>,
    pub video: Option<u8>,
    pub audio: Option<u8>,
    pub still: Option<u8>,
    pub folder: Option<u8>,
    pub solid: Option<u8>,
    pub camera: Option<u8>,
    pub light: Option<u8>,
    pub adjustment: Option<u8>,
    pub null_object: Option<u8>,
    pub shape: Option<u8>,
    pub text: Option<u8>,
    pub model_3d: Option<u8>,
    pub other_3d: Option<u8>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LabelPreferences {
    pub labels: Vec<LabelDefinition>,
    pub defaults: LabelDefaults,
    /// 初期値は要確認。
    pub use_for_handles: Option<bool>,
    pub use_for_tabs: Option<bool>,
}

impl Default for LabelPreferences {
    fn default() -> Self {
        let labels = (1..=16).map(LabelDefinition::unnamed).collect();
        Self {
            labels,
            defaults: LabelDefaults::default(),
            use_for_handles: None,
            use_for_tabs: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PresetLibrary {
    pub bundled: Vec<CompPreset>,
    pub user: Vec<CompPreset>,
}

impl Default for PresetLibrary {
    fn default() -> Self {
        Self {
            bundled: Vec::new(),
            user: Vec::new(),
        }
    }
}

impl PresetLibrary {
    pub fn all(&self) -> impl Iterator<Item = &CompPreset> {
        self.bundled.iter().chain(self.user.iter())
    }

    pub fn save_user(&mut self, preset: CompPreset) {
        self.user.push(preset);
    }

    /// Alt+クリック。ユーザー分を消し、同梱だけに戻す。
    pub fn reset_to_bundled(&mut self) {
        self.user.clear();
    }
}

/// 直前に確定したコンポジション設定。保持フラグは新規へ渡さない。
#[derive(Clone, Debug, PartialEq)]
pub struct RememberedCompSettings {
    pub width: u32,
    pub height: u32,
    pub pixel_aspect: f64,
    pub frame_rate: f64,
    pub resolution: Resolution,
    pub start: Time,
    pub duration: Time,
    pub background: crate::color::Rgba,
    pub motion_blur: MotionBlurSettings,
    pub preserve_frame_rate: bool,
    pub preserve_resolution: bool,
    pub renderer: Renderer,
    pub drop_frame: bool,
    pub anchor: CompAnchor,
    pub cinema4d: Cinema4dOptions,
}

/// プロジェクト。色深度は 8、16、32 のいずれか。
#[derive(Clone, Debug, PartialEq)]
pub struct Project {
    pub bits_per_channel: BitsPerChannel,
    pub root: Folder,
    pub footage: Vec<Footage>,
    pub compositions: Vec<Composition>,
    pub color: ColorSettings,
    pub time_display: TimeDisplay,
    pub labels: LabelPreferences,
    pub presets: PresetLibrary,
    pub last_comp_settings: Option<RememberedCompSettings>,
    pub remembered_interpretation: Option<Interpretation>,
    /// 共通のハンドル色。ラベル色を使わないときの色そのものは要確認なので、比較用の印だけ。
    pub common_handle_color: LabelColor,
    ids: Ids,
}

impl Project {
    pub fn new(bits_per_channel: BitsPerChannel) -> Self {
        let mut ids = Ids::new();
        let root = Folder::new(ids.item(), "Root");
        Self {
            bits_per_channel,
            root,
            footage: Vec::new(),
            compositions: Vec::new(),
            color: ColorSettings::default(),
            time_display: TimeDisplay::default(),
            labels: LabelPreferences::default(),
            presets: PresetLibrary::default(),
            last_comp_settings: None,
            remembered_interpretation: None,
            common_handle_color: LabelColor {
                r: 1.0,
                g: 1.0,
                b: 1.0,
            },
            ids,
        }
    }

    pub fn alloc_item(&mut self) -> ItemId {
        self.ids.item()
    }

    pub fn alloc_layer(&mut self) -> crate::id::LayerId {
        self.ids.layer()
    }

    pub fn alloc_property(&mut self) -> crate::id::PropertyId {
        self.ids.property()
    }

    pub fn add_footage(&mut self, footage: Footage) {
        let id = footage.id;
        self.footage.push(footage);
        self.root.children.push(FolderChild::Footage(id));
    }

    pub fn add_composition(&mut self, composition: Composition) {
        let id = composition.id;
        self.compositions.push(composition);
        self.root.children.push(FolderChild::Composition(id));
    }

    pub fn add_folder(&mut self, folder: Folder) {
        self.root.children.push(FolderChild::Folder(folder));
    }

    pub fn footage(&self, id: ItemId) -> Option<&Footage> {
        self.footage.iter().find(|item| item.id == id)
    }

    pub fn footage_mut(&mut self, id: ItemId) -> Option<&mut Footage> {
        self.footage.iter_mut().find(|item| item.id == id)
    }

    pub fn composition(&self, id: ItemId) -> Option<&Composition> {
        self.compositions.iter().find(|item| item.id == id)
    }

    pub fn composition_mut(&mut self, id: ItemId) -> Option<&mut Composition> {
        self.compositions.iter_mut().find(|item| item.id == id)
    }
}
