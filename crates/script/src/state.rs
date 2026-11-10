//! スクリプティング文書。契約クレートの公開型はここから使う。

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use effect_api::Frame;
use io_api::{FootageReference, RenderQueue as IoRenderQueue};
use model::{BitsPerChannel, Ids, ItemId, LayerId, PropertyId};
use ui_api::{Panel, PanelId};

use crate::enums::*;
use crate::error::{ScriptError, ScriptResult};
use crate::text::TextDocument;
use crate::values::{Guide, MarkerValue, ShapeValue, Viewer};

mod comp;
mod item;
mod layer;
mod persist;
mod project;
mod prop;
mod render;
mod source;

pub struct App {
    pub(crate) platform: Platform,
    pub(crate) tool: ToolType,
    pub(crate) project_panel: bool,
    pub(crate) render_panel: bool,
    pub(crate) viewer: Option<Viewer>,
    pub(crate) essential: Option<ItemId>,
    pub(crate) import_dialog: Option<Vec<PathBuf>>,
    pub(crate) save_dialog: Option<PathBuf>,
    pub(crate) save_dialog_set: bool,
    pub(crate) default_import_folder: Option<PathBuf>,
    pub(crate) ids: Ids,
    pub(crate) root: ItemId,
    pub(crate) order: Vec<ItemId>,
    pub(crate) items: HashMap<ItemId, ItemRec>,
    pub(crate) layers: HashMap<LayerId, LayerRec>,
    pub(crate) props: HashMap<PropertyId, Prop>,
    pub(crate) catalog: HashMap<String, PropertyId>,
    pub(crate) bits: BitsPerChannel,
    pub(crate) compensate_scene: bool,
    pub(crate) dirty: bool,
    pub(crate) display_start_frame: i32,
    pub(crate) frames_count: FramesCountType,
    pub(crate) expression_engine: String,
    pub(crate) feet_film: FeetFramesFilmType,
    pub(crate) file: Option<PathBuf>,
    pub(crate) timecode_start: FootageTimecodeDisplayStartType,
    pub(crate) feet_frames: bool,
    pub(crate) gpu: GpuAccelType,
    pub(crate) linear_blending: bool,
    pub(crate) linearize_working_space: bool,
    pub(crate) revision: i64,
    pub(crate) time_display: TimeDisplayType,
    pub(crate) transparency_grid: bool,
    pub(crate) working_gamma: f64,
    pub(crate) working_space: String,
    pub(crate) xmp: String,
    pub(crate) render: RenderState,
    pub(crate) io_queue: IoRenderQueue,
    pub(crate) project_ui: Panel,
    pub(crate) render_ui: Panel,
}

pub(crate) struct ItemRec {
    pub name: String,
    pub comment: String,
    pub label: i32,
    pub selected: bool,
    pub parent: ItemId,
    pub guides: Vec<Guide>,
    pub body: ItemBody,
}

pub(crate) enum ItemBody {
    Folder { children: Vec<ItemId> },
    Comp(CompRec),
    Footage(FootageRec),
}

pub(crate) struct CompRec {
    pub width: u32,
    pub height: u32,
    pub pixel_aspect: f64,
    pub frame_rate: f64,
    pub duration: f64,
    pub time: f64,
    pub work_start: f64,
    pub work_duration: f64,
    pub layers: Vec<LayerId>,
    pub bg: [f64; 3],
    pub display_start_frame: i32,
    pub display_start_time: f64,
    pub draft3d: bool,
    pub drop_frame: bool,
    pub frame_blending: bool,
    pub hide_shy: bool,
    pub motion_blur: bool,
    pub motion_blur_adaptive: u32,
    pub motion_blur_samples: u32,
    pub shutter_angle: f64,
    pub shutter_phase: f64,
    pub preserve_nested_frame_rate: bool,
    pub preserve_nested_resolution: bool,
    pub renderer: String,
    pub resolution: [i32; 2],
    pub counters: bool,
    pub template_name: String,
    pub controllers: Vec<Controller>,
    pub marker: PropertyId,
}

pub(crate) struct Controller {
    pub name: String,
    pub property: Option<PropertyId>,
    pub layer: Option<LayerId>,
    pub type_name: String,
    pub default_number: f64,
}

pub(crate) struct FootageRec {
    pub main: SourceRec,
    pub proxy: Option<SourceRec>,
    pub use_proxy: bool,
    pub eval: Frame,
}

#[derive(Clone)]
pub(crate) struct SourceRec {
    pub kind: SourceKind,
    pub reference: Option<FootageReference>,
    pub path: Option<PathBuf>,
    pub width: u32,
    pub height: u32,
    pub duration: f64,
    pub native_duration: f64,
    pub frame_rate: f64,
    pub native_frame_rate: f64,
    pub conform_frame_rate: f64,
    pub pixel_aspect: f64,
    pub color: [f64; 3],
    pub has_audio: bool,
    pub has_video: bool,
    pub has_alpha: bool,
    pub alpha_mode: AlphaMode,
    pub field_separation: FieldSeparationType,
    pub high_quality_fields: bool,
    pub invert_alpha: bool,
    pub is_still: bool,
    pub loops: i32,
    pub premul_color: [f64; 3],
    pub pulldown: PulldownPhase,
    pub sequence: bool,
    pub sequence_paths: Vec<PathBuf>,
    pub missing: bool,
    pub name: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SourceKind {
    File,
    Solid,
    Placeholder,
    Sequence,
}

pub(crate) struct LayerRec {
    pub name: String,
    pub name_set: bool,
    pub comp: ItemId,
    pub kind: LayerKind,
    pub comment: String,
    pub label: i32,
    pub locked: bool,
    pub shy: bool,
    pub solo: bool,
    pub enabled: bool,
    pub in_point: f64,
    pub out_point: f64,
    pub start_time: f64,
    pub source_span: f64,
    pub stretch: f64,
    pub selected: bool,
    pub auto_orient: AutoOrientType,
    pub parent: Option<LayerId>,
    pub guides: Vec<Guide>,
    pub root_props: Vec<PropertyId>,
    pub blend: model::BlendMode,
    pub track_matte: model::TrackMatte,
    pub matte_type: TrackMatteType,
    pub preserve_transparency: bool,
    pub collapse: bool,
    pub motion_blur: bool,
    pub three_d: bool,
    pub three_d_per_char: bool,
    pub adjustment: bool,
    pub environment: bool,
    pub guide_layer: bool,
    pub frame_blending: bool,
    pub frame_blending_type: FrameBlendingType,
    pub effects_active: bool,
    pub audio_enabled: bool,
    pub quality: LayerQuality,
    pub sampling: LayerSamplingQuality,
    pub time_remap: bool,
    pub has_video: bool,
    pub has_audio: bool,
    pub light_type: LightType,
    pub light_source: Option<LayerId>,
    pub point_of_interest: [f64; 3],
    pub focus_area_width: f64,
    pub near_far_blur: f64,
    pub camera_nodes: u8,
    pub mesh: ParametricMeshType,
    pub mesh_fields: BTreeMap<String, f64>,
    pub bevel_fields: BTreeMap<String, f64>,
    pub text: Option<TextDocument>,
}

pub(crate) enum LayerKind {
    Av { source: Option<ItemId> },
    Null,
    Camera,
    Light,
    Text,
    Shape,
    Mesh,
}

pub(crate) struct Prop {
    pub parent: Option<PropertyId>,
    pub children: Vec<PropertyId>,
    pub layer: Option<LayerId>,
    pub name: String,
    pub match_name: String,
    pub aliases: Vec<String>,
    pub property_type: PropertyType,
    pub value_type: PropertyValueType,
    pub value: Value,
    pub keys: Vec<KeyRec>,
    pub expression: String,
    pub expression_enabled: bool,
    pub expression_error: String,
    pub selected: bool,
    pub enabled: bool,
    pub can_set_enabled: bool,
    pub elided: bool,
    pub is_effect: bool,
    pub is_mask: bool,
    pub can_set_expression: bool,
    pub can_vary: bool,
    pub is_spatial: bool,
    pub has_min: bool,
    pub has_max: bool,
    pub min_value: f64,
    pub max_value: f64,
    pub units: String,
    pub dimensions_separated: bool,
    pub separation_leader: Option<PropertyId>,
    pub separation_dimension: i32,
    pub is_leader: bool,
    pub is_follower: bool,
    pub dropdown: bool,
    pub choices: Vec<String>,
    pub alternate: Option<ItemId>,
    pub can_set_alternate: bool,
    pub input_layer: Option<i32>,
    pub input_stage: Option<i32>,
    pub mask_color: [f64; 3],
    pub mask_locked: bool,
    pub mask_inverted: bool,
    pub mask_falloff: MaskFeatherFalloff,
    pub mask_mode: ScriptMaskMode,
    pub mask_motion_blur: MaskMotionBlur,
    pub roto_bezier: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct KeyRec {
    pub time: f64,
    pub value: Value,
    pub in_type: KeyframeInterpolationType,
    pub out_type: KeyframeInterpolationType,
    pub ease_in: Vec<crate::values::ScriptEase>,
    pub ease_out: Vec<crate::values::ScriptEase>,
    pub roving: bool,
    pub selected: bool,
    pub label: i32,
    pub in_tangent: Vec<f64>,
    pub out_tangent: Vec<f64>,
    pub spatial_auto: bool,
    pub spatial_continuous: bool,
    pub temporal_auto: bool,
    pub temporal_continuous: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Value {
    Empty,
    Scalar(f64),
    Vec2([f64; 2]),
    Vec3([f64; 3]),
    Color([f64; 4]),
    Bool(bool),
    Text(TextDocument),
    Shape(ShapeValue),
    Marker(MarkerValue),
    Index(i32),
}

pub(crate) struct RenderState {
    pub notify: bool,
    pub rendering: bool,
    pub paused: bool,
    pub items: Vec<RqItem>,
    pub render_templates: BTreeMap<String, BTreeMap<String, String>>,
    pub output_templates: BTreeMap<String, String>,
    pub status_hook: Option<fn(RqItemStatus)>,
}

pub(crate) struct RqItem {
    pub comp: ItemId,
    pub log_type: LogType,
    pub notify: bool,
    pub render: bool,
    pub skip_frames: i32,
    pub status: RqItemStatus,
    pub time_span_start: f64,
    pub time_span_duration: f64,
    pub elapsed: Option<i32>,
    pub started: Option<String>,
    pub template: String,
    pub settings: BTreeMap<String, String>,
    pub modules: Vec<OutputMod>,
    pub rendered_frames: Vec<i32>,
}

pub(crate) struct OutputMod {
    pub file: Option<PathBuf>,
    pub include_source_xmp: bool,
    pub name: String,
    pub post_render: ScriptPostRenderAction,
    pub template: String,
    pub settings: BTreeMap<String, String>,
}

impl App {
    pub fn new() -> Self {
        Self::with_platform(Platform::host())
    }

    pub fn with_platform(platform: Platform) -> Self {
        let mut ids = Ids::new();
        let root = ids.item();
        let mut items = HashMap::new();
        items.insert(
            root,
            ItemRec {
                name: "Root".to_string(),
                comment: String::new(),
                label: 0,
                selected: false,
                parent: root,
                guides: Vec::new(),
                body: ItemBody::Folder { children: Vec::new() },
            },
        );
        let mut render_templates = BTreeMap::new();
        let mut best = BTreeMap::new();
        best.insert("Quality".to_string(), "Best".to_string());
        best.insert("Resolution".to_string(), "1,1".to_string());
        render_templates.insert("Best Settings".to_string(), best);
        let mut draft = BTreeMap::new();
        draft.insert("Quality".to_string(), "Draft".to_string());
        draft.insert("Resolution".to_string(), "2,2".to_string());
        render_templates.insert("Draft Settings".to_string(), draft);
        let mut output_templates = BTreeMap::new();
        output_templates.insert("PNG Sequence".to_string(), "PNG".to_string());
        Self {
            platform,
            tool: ToolType::Selection,
            project_panel: true,
            render_panel: false,
            viewer: None,
            essential: None,
            import_dialog: None,
            save_dialog: None,
            save_dialog_set: false,
            default_import_folder: None,
            ids,
            root,
            order: Vec::new(),
            items,
            layers: HashMap::new(),
            props: HashMap::new(),
            catalog: HashMap::new(),
            bits: BitsPerChannel::Bpc8,
            compensate_scene: false,
            dirty: false,
            display_start_frame: 0,
            frames_count: FramesCountType::Start1,
            expression_engine: "extendscript".to_string(),
            feet_film: FeetFramesFilmType::Mm35,
            file: None,
            timecode_start: FootageTimecodeDisplayStartType::UseSourceMedia,
            feet_frames: false,
            gpu: GpuAccelType::Software,
            linear_blending: false,
            linearize_working_space: false,
            revision: 1,
            time_display: TimeDisplayType::Timecode,
            transparency_grid: false,
            working_gamma: 2.2,
            working_space: String::new(),
            xmp: String::new(),
            render: RenderState {
                notify: false,
                rendering: false,
                paused: false,
                items: Vec::new(),
                render_templates,
                output_templates,
                status_hook: None,
            },
            io_queue: IoRenderQueue::new(),
            project_ui: Panel::new("project", "プロジェクト"),
            render_ui: Panel::new("render-queue", "レンダーキュー"),
        }
    }

    pub(crate) fn touch(&mut self) {
        self.dirty = true;
    }

    pub(crate) fn alloc_item(&mut self) -> ItemId {
        self.ids.item()
    }

    pub(crate) fn alloc_layer(&mut self) -> LayerId {
        self.ids.layer()
    }

    pub(crate) fn alloc_prop(&mut self) -> PropertyId {
        self.ids.property()
    }

    pub(crate) fn item(&self, id: ItemId) -> ScriptResult<&ItemRec> {
        self.items.get(&id).ok_or_else(|| ScriptError::missing("項目"))
    }

    pub(crate) fn item_mut(&mut self, id: ItemId) -> ScriptResult<&mut ItemRec> {
        self.items.get_mut(&id).ok_or_else(|| ScriptError::missing("項目"))
    }

    pub(crate) fn require_comp(&self, id: ItemId) -> ScriptResult<&CompRec> {
        match &self.item(id)?.body {
            ItemBody::Comp(comp) => Ok(comp),
            _ => Err(ScriptError::invalid("コンポジションではない")),
        }
    }

    pub(crate) fn require_comp_mut(&mut self, id: ItemId) -> ScriptResult<&mut CompRec> {
        match &mut self
            .items
            .get_mut(&id)
            .ok_or_else(|| ScriptError::missing("項目"))?
            .body
        {
            ItemBody::Comp(comp) => Ok(comp),
            _ => Err(ScriptError::invalid("コンポジションではない")),
        }
    }

    pub(crate) fn require_footage(&self, id: ItemId) -> ScriptResult<&FootageRec> {
        match &self.item(id)?.body {
            ItemBody::Footage(footage) => Ok(footage),
            _ => Err(ScriptError::invalid("フッテージではない")),
        }
    }

    pub(crate) fn require_footage_mut(&mut self, id: ItemId) -> ScriptResult<&mut FootageRec> {
        match &mut self
            .items
            .get_mut(&id)
            .ok_or_else(|| ScriptError::missing("項目"))?
            .body
        {
            ItemBody::Footage(footage) => Ok(footage),
            _ => Err(ScriptError::invalid("フッテージではない")),
        }
    }

    pub(crate) fn layer(&self, id: LayerId) -> ScriptResult<&LayerRec> {
        self.layers.get(&id).ok_or_else(|| ScriptError::missing("レイヤー"))
    }

    pub(crate) fn layer_mut(&mut self, id: LayerId) -> ScriptResult<&mut LayerRec> {
        self.layers.get_mut(&id).ok_or_else(|| ScriptError::missing("レイヤー"))
    }

    pub(crate) fn prop(&self, id: PropertyId) -> ScriptResult<&Prop> {
        self.props.get(&id).ok_or_else(|| ScriptError::missing("プロパティ"))
    }

    pub(crate) fn prop_mut(&mut self, id: PropertyId) -> ScriptResult<&mut Prop> {
        self.props.get_mut(&id).ok_or_else(|| ScriptError::missing("プロパティ"))
    }

    pub fn project_panel_id(&self) -> &PanelId {
        &self.project_ui.id
    }

    pub fn render_panel_id(&self) -> &PanelId {
        &self.render_ui.id
    }

    pub(crate) fn color_profiles() -> &'static [&'static str] {
        &[
            "sRGB IEC61966-2.1",
            "Adobe RGB (1998)",
            "Rec.709 Gamma 2.4",
            "Rec.2020",
        ]
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) fn check_range(value: f64, min: f64, max: f64, name: &str) -> ScriptResult<()> {
    if value < min || value > max {
        Err(ScriptError::range(name))
    } else {
        Ok(())
    }
}

pub(crate) fn check_int_range(value: i32, min: i32, max: i32, name: &str) -> ScriptResult<()> {
    if value < min || value > max {
        Err(ScriptError::range(name))
    } else {
        Ok(())
    }
}
