//! 仕様に名前がある列挙。Adobe のヘッダからは写していない。

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FeetFramesFilmType {
    Mm16,
    Mm35,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FootageTimecodeDisplayStartType {
    Start0,
    UseSourceMedia,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FramesCountType {
    Start1,
    Start0,
    TimecodeConversion,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GpuAccelType {
    Cuda,
    Metal,
    OpenCl,
    Software,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TimeDisplayType {
    Frames,
    Timecode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ToolType {
    Selection,
    Arrow,
    Rotate,
    CameraMaya,
    CameraOrbit,
    CameraTrackXy,
    CameraTrackZ,
    Paintbrush,
    CloneStamp,
    Eraser,
    Hand,
    Magnify,
    PanBehind,
    Rect,
    RoundedRect,
    Oval,
    Polygon,
    Star,
    TextH,
    TextV,
    Pen,
    Feather,
    PenPlus,
    PenMinus,
    PenConvert,
    Pin,
    PinStarch,
    PinDepth,
    Quickselect,
    Hairbrush,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CloseOptions {
    SaveChanges,
    PromptToSaveChanges,
    DoNotSaveChanges,
}

/// 仕様は型名だけを示す。呼び出しは未対応エラーになり、値では分岐しない。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ResolveType {
    AcceptMine,
    AcceptTheirs,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AutoOrientType {
    AlongPath,
    CameraOrPointOfInterest,
    CharactersTowardCamera,
    NoAutoOrient,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SceneEditDetectionMode {
    Markers,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FrameBlendingType {
    FrameMix,
    NoFrameBlend,
    PixelMotion,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LayerQuality {
    Best,
    Draft,
    Wireframe,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LayerSamplingQuality {
    Bicubic,
    Bilinear,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TrackMatteType {
    Alpha,
    AlphaInverted,
    Luma,
    LumaInverted,
    NoTrackMatte,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LightType {
    Parallel,
    Spot,
    Point,
    Ambient,
    Environment,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ParametricMeshType {
    Sphere,
    Plane,
    Cylinder,
    Cone,
    Torus,
    Cube,
}

impl ParametricMeshType {
    pub const fn allows_bevel(self) -> bool {
        matches!(self, Self::Cube | Self::Cone | Self::Cylinder)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PropertyValueType {
    NoValue,
    ThreeDSpatial,
    ThreeD,
    TwoDSpatial,
    TwoD,
    OneD,
    Color,
    CustomValue,
    Marker,
    LayerIndex,
    MaskIndex,
    Shape,
    TextDocument,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KeyframeInterpolationType {
    Linear,
    Bezier,
    Hold,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LayerInputStageType {
    Source,
    Masks,
    Effects,
}

impl LayerInputStageType {
    pub const fn index(self) -> i32 {
        match self {
            Self::Source => 0,
            Self::Masks => 1,
            Self::Effects => 2,
        }
    }

    pub const fn from_index(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Source),
            1 => Some(Self::Masks),
            2 => Some(Self::Effects),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PropertyType {
    Property,
    IndexedGroup,
    NamedGroup,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MaskFeatherFalloff {
    Linear,
    Smooth,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScriptMaskMode {
    None,
    Add,
    Subtract,
    Intersect,
    Lighten,
    Darken,
    Difference,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MaskMotionBlur {
    SameAsLayer,
    On,
    Off,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LogType {
    ErrorsOnly,
    ErrorsAndSettings,
    ErrorsAndPerFrameInfo,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RqItemStatus {
    WillContinue,
    NeedsOutput,
    Unqueued,
    Queued,
    Rendering,
    UserStopped,
    ErrStopped,
    Done,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScriptPostRenderAction {
    None,
    Import,
    ImportAndReplaceUsage,
    SetProxy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AlphaMode {
    Ignore,
    Straight,
    Premultiplied,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FieldSeparationType {
    Off,
    UpperFieldFirst,
    LowerFieldFirst,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PulldownPhase {
    Off,
    Wssww,
    Sswww,
    Swwww,
    Wwwss,
    Wwssw,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PulldownMethod {
    FrameRate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ImportAsType {
    CompCroppedLayers,
    Footage,
    Comp,
    Project,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GuideOrientationType {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GuidePositionType {
    Pixels,
    Percentage,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AutoKernType {
    NoAutoKern,
    /// 合格の METRIC。取りうる値の METRIC_KERN と同じ。
    Metric,
    OpticalKern,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BaselineDirection {
    /// 合格が名前を出している水平方向。
    Horizontal,
    WithStream,
    VerticalRotated,
    VerticalCrossStream,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BoxAutoFitPolicy {
    None,
    HeightCursor,
    HeightPreciseBounds,
    HeightBaseline,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BoxFirstBaselineAlignment {
    Ascent,
    CapHeight,
    EmBox,
    Leading,
    LegacyMetric,
    MinimumValueAsian,
    MinimumValueRoman,
    TypoAscent,
    XHeight,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BoxVerticalAlignment {
    Top,
    Center,
    Bottom,
    Justify,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ComposerEngine {
    LatinCjk,
    UniversalType,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DigitSet {
    DefaultDigits,
    ArabicDigits,
    HindiDigits,
    FarsiDigits,
    ArabicDigitsRtl,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ParagraphDirection {
    LeftToRight,
    RightToLeft,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FontBaselineOption {
    Normal,
    FauxSuperscript,
    FauxSubscript,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FontCapsOption {
    Normal,
    SmallCaps,
    AllCaps,
    AllSmallCaps,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ParagraphJustification {
    Left,
    Right,
    Center,
    FullLastLeft,
    FullLastRight,
    FullLastCenter,
    FullLastFull,
    Multiple,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LeadingType {
    Roman,
    Japanese,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LineJoinType {
    Miter,
    Round,
    Bevel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LineOrientation {
    Horizontal,
    VerticalRightToLeft,
    VerticalLeftToRight,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Platform {
    Windows,
    Other,
}

impl Platform {
    pub fn host() -> Self {
        if cfg!(target_os = "windows") {
            Self::Windows
        } else {
            Self::Other
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ItemKindName {
    Composition,
    Folder,
    Footage,
}

impl ItemKindName {
    pub const fn ja(self) -> &'static str {
        match self {
            Self::Composition => "コンポジション",
            Self::Folder => "フォルダー",
            Self::Footage => "フッテージ",
        }
    }
}
