//! Blur & Sharpen、Channel、Color Correction。
//! 初期値・範囲・選択肢は仕様で確認できたものだけを入れる。

#![forbid(unsafe_code)]

mod blur_ops;
mod catalog;
mod channel_ops;
mod color_math;
mod color_ops;
mod control;
mod host;
mod image;

use effect_api::{ApplyError, ApplyResult, CpuApply, CpuInput, Frame, ParamValues};
use model::BitsPerChannel;

pub use blur_ops::{
    bilateral_blur, camera_shake_deblur, cc_radial, channel_blur, compound_blur, cross_blur,
    directional, fast_box, gaussian, lens_blur, radial_blur, radial_fast, sharpen, smart_blur,
    unsharp, vector_blur, CrossMode, DeblurMethod, DeblurOutput, FastZoom, IrisSides, MapChannel,
    MapFit, RadialType, SmartMode, SpinZoom, VectorProperty, VectorType,
};
pub use catalog::EffectSpec;
pub use channel_ops::{
    arithmetic, blend_layers, calculations, cc_composite, channel_combiner, compound_arithmetic,
    invert, minimax, remove_color_matting, set_channels, set_matte, shift_channels,
    solid_composite, BlendMethod, ChannelMask, CombinerFrom, CombinerTo, CompositeMode, Formula,
    InputChannel, InvertChannel, MinimaxOp, Operator, OverflowBehavior, RgbChannel, SolidBlend,
};
pub use color_math::{phase_shift, Curve, Overflow, Primary};
pub use color_ops::{
    arbitrary_map, auto_tone, black_white, brightness_contrast, broadcast_colors, change_color,
    change_to_color, channel_mixer, color_balance, color_balance_hls, color_link,
    color_neutralizer, color_offset, color_stabilizer, colorama, curves_channel, curves_master,
    equalize, exposure, gamma_correct, gamma_pedestal_gain, histogram_counts, hue_curve,
    hue_cycle_color, hue_saturation, kernel3, leave_color, levels, levels_channel,
    linear_identity_map, lumetri, photo_filter, sample_region, selective_color, set_frame,
    shadow_highlight, swap_colors, tint, toner, tritone, vibrance, video_limiter, AutoInput,
    Balance, BroadcastLocale, ChangeMode, ChangeTolerance, ColorSafe, ColoramaInput, EqualizeMode,
    HueCurveMode, HueSat, LeaveMatch, Levels, LinkSample, MatchSpace, ModifyAttr, NeutralView,
    Neutralizer, PhaseCombine, PhaseSource, PhotoFilterKind, SelectiveMethod, ShadowHighlight,
    StabilizeMode, TonerColors, Tones,
};
pub use control::{
    check_values, confirmed_param_list, values_from_known_defaults, Category, Control,
};
pub use host::{ArbitraryMap, CurveSet, CycleStop, Host, LayerPixels, StabilizeRef};
pub use image::{sharpness, Axes, Edge, Image};

use catalog::{
    angle_unknown, checkbox_unknown, color_unknown, float_unknown, layer, point_unknown, popup,
    BPC_16_32, BPC_8, BPC_8_16, BPC_ALL,
};
use control::Args;

const CROSS_MODE: &[&str] = &["Blend", "Add", "Screen", "Multiply", "Lighten", "Darken"];
const RADIAL_CC: &[&str] = &[
    "Straight Zoom",
    "Fading Zoom",
    "Centered Zoom",
    "Rotate",
    "Scratch",
    "Rotate Fading",
];
const FAST_ZOOM: &[&str] = &["Standard", "Brightest", "Darkest"];
const VECTOR_TYPE: &[&str] = &[
    "Natural",
    "Constant Length",
    "Perpendicular",
    "Direction Center",
    "Direction Fading",
];
const VECTOR_PROPERTY: &[&str] = &[
    "Red",
    "Green",
    "Blue",
    "Alpha",
    "Luminance",
    "Lightness",
    "Hue",
    "Saturation",
];
const SMART_MODE: &[&str] = &["Normal", "Edge Only", "Overlay Edge"];
const SPIN_ZOOM: &[&str] = &["Spin", "Zoom"];
const OPERATORS: &[&str] = &[
    "And",
    "Or",
    "Xor",
    "Add",
    "Subtract",
    "Multiply",
    "Difference",
    "Max",
    "Min",
    "Block Above",
    "Block Below",
    "Slice",
    "Screen",
];
const BLEND_METHOD: &[&str] = &[
    "Color Only",
    "Tint Only",
    "Darken Only",
    "Lighten Only",
    "Crossfade",
];
const INPUT_CHANNEL: &[&str] = &["RGBA", "Gray", "Red", "Green", "Blue", "Alpha"];
const COMPOSITE_ORIGINAL: &[&str] = &["Copy", "Behind", "In Front"];
const COMBINER_FROM: &[&str] = &["Saturation Multiplied", "Min RGB", "Max RGB"];
const COMBINER_TO: &[&str] = &[
    "Red Only",
    "Green Only",
    "Blue Only",
    "Alpha Only",
    "Hue Only",
    "Lightness Only",
    "Saturation Only",
];
const OVERFLOW: &[&str] = &["Clip", "Wrap", "Scale"];
const INVERT_CHANNEL: &[&str] = &[
    "RGB",
    "Red",
    "Green",
    "Blue",
    "HLS",
    "Hue",
    "Lightness",
    "Saturation",
    "YIQ",
    "Y（Luminance）",
    "I（In Phase Chrominance）",
    "Q（Quadrature Chrominance）",
    "Alpha",
];
const MINIMAX_OP: &[&str] = &[
    "Minimum",
    "Maximum",
    "Minimum Then Maximum",
    "Maximum Then Minimum",
];
const DEBLUR_METHOD: &[&str] = &["High Quality (slower)", "Standard"];
const LENS_CHANNEL: &[&str] = &["Luminance", "color", "alpha"];
const EQUALIZE_MODE: &[&str] = &["RGB", "Brightness", "Photoshop Style"];
const LOCALE: &[&str] = &["NTSC", "PAL"];
const COLOR_SAFE: &[&str] = &[
    "Reduce Luminance",
    "Reduce Saturation",
    "Key Out Unsafe",
    "Key Out Safe",
];
const OFFSET_OVERFLOW: &[&str] = &["Wrap", "Solarize", "Polarize"];
const TONES: &[&str] = &["Duotone", "Tritone", "Pentone"];
const MATCH_COLORS: &[&str] = &["RGB", "Hue", "Chroma"];
pub(crate) const CHANGE_MODE_NAMES_WITHOUT_A_PARAMETER: &[&str] =
    &["To Color", "Transforming To Color"];
const LEAVE_MATCH: &[&str] = &["Using RGB", "Using HSB"];
const SELECTIVE_METHOD: &[&str] = &["Relative", "Absolute"];
const STABILIZE: &[&str] = &["Brightness", "Color", "Curves"];
const PHASE_FROM: &[&str] = &["Zero", "Hue"];
const MODIFY: &[&str] = &["Hue", "None"];
const MATCHING_MODE: &[&str] = &["Off", "RGB", "Chroma"];
const PHOTO_FILTERS: &[&str] = &[
    "Warming Filter (85)",
    "Cooling Filter (80)",
    "Warming Filter (81)",
    "Cooling Filter (82)",
    "Individual Colors",
    "Underwater",
];
const NEUTRAL_VIEW: &[&str] = &[
    "Final Result",
    "View Correction B to W",
    "View Correction Stair",
];
const FAST_BOX_MATCH: &[&str] = &["ADBE Box Blur2", "ADBE Box Blur"];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EffectKind {
    BilateralBlur,
    CameraLensBlur,
    CameraShakeDeblur,
    CcCrossBlur,
    CcRadialBlur,
    CcRadialFastBlur,
    CcVectorBlur,
    ChannelBlur,
    CompoundBlur,
    DirectionalBlur,
    FastBoxBlur,
    GaussianBlur,
    RadialBlur,
    Sharpen,
    SmartBlur,
    UnsharpMask,
    Arithmetic,
    Blend,
    Calculations,
    CcComposite,
    ChannelCombiner,
    CompoundArithmetic,
    Invert,
    Minimax,
    RemoveColorMatting,
    SetChannels,
    SetMatte,
    ShiftChannels,
    SolidComposite,
    AutoColor,
    AutoContrast,
    AutoLevels,
    BlackWhite,
    BrightnessContrast,
    BroadcastColors,
    CcColorNeutralizer,
    CcColorOffset,
    CcKernel,
    CcToner,
    ChangeColor,
    ChangeToColor,
    ChannelMixer,
    ColorBalance,
    ColorBalanceHls,
    ColorLink,
    ColorStabilizer,
    Colorama,
    Curves,
    Equalize,
    Exposure,
    GammaPedestalGain,
    HueSaturation,
    LeaveColor,
    Levels,
    LevelsIndividual,
    LumetriColor,
    PhotoFilter,
    PsArbitraryMap,
    SelectiveColor,
    ShadowHighlight,
    Tint,
    Tritone,
    Vibrance,
    VideoLimiter,
}

impl EffectKind {
    pub const ALL: [EffectKind; 64] = [
        Self::BilateralBlur,
        Self::CameraLensBlur,
        Self::CameraShakeDeblur,
        Self::CcCrossBlur,
        Self::CcRadialBlur,
        Self::CcRadialFastBlur,
        Self::CcVectorBlur,
        Self::ChannelBlur,
        Self::CompoundBlur,
        Self::DirectionalBlur,
        Self::FastBoxBlur,
        Self::GaussianBlur,
        Self::RadialBlur,
        Self::Sharpen,
        Self::SmartBlur,
        Self::UnsharpMask,
        Self::Arithmetic,
        Self::Blend,
        Self::Calculations,
        Self::CcComposite,
        Self::ChannelCombiner,
        Self::CompoundArithmetic,
        Self::Invert,
        Self::Minimax,
        Self::RemoveColorMatting,
        Self::SetChannels,
        Self::SetMatte,
        Self::ShiftChannels,
        Self::SolidComposite,
        Self::AutoColor,
        Self::AutoContrast,
        Self::AutoLevels,
        Self::BlackWhite,
        Self::BrightnessContrast,
        Self::BroadcastColors,
        Self::CcColorNeutralizer,
        Self::CcColorOffset,
        Self::CcKernel,
        Self::CcToner,
        Self::ChangeColor,
        Self::ChangeToColor,
        Self::ChannelMixer,
        Self::ColorBalance,
        Self::ColorBalanceHls,
        Self::ColorLink,
        Self::ColorStabilizer,
        Self::Colorama,
        Self::Curves,
        Self::Equalize,
        Self::Exposure,
        Self::GammaPedestalGain,
        Self::HueSaturation,
        Self::LeaveColor,
        Self::Levels,
        Self::LevelsIndividual,
        Self::LumetriColor,
        Self::PhotoFilter,
        Self::PsArbitraryMap,
        Self::SelectiveColor,
        Self::ShadowHighlight,
        Self::Tint,
        Self::Tritone,
        Self::Vibrance,
        Self::VideoLimiter,
    ];

    pub fn spec(self) -> EffectSpec {
        match self {
            Self::BilateralBlur => spec(
                "Bilateral Blur",
                Category::BlurSharpen,
                BPC_ALL,
                &[],
                &["Threshold", "Radius", "Colorize"],
                &[],
                false,
                &[],
            ),
            Self::CameraLensBlur => spec(
                "Camera Lens Blur",
                Category::BlurSharpen,
                BPC_ALL,
                &[
                    float_unknown("Diffraction Fringe"),
                    layer("Blur Map"),
                    popup("Channel", LENS_CHANNEL, None, false),
                    float_unknown("Threshold"),
                    float_unknown("Saturation"),
                    checkbox_unknown("Use Linear Working Space"),
                ],
                &[
                    "Shape",
                    "Roundness",
                    "Aspect Ratio",
                    "Rotation",
                    "Blur Focal Distance",
                    "Edge Behavior",
                ],
                &[],
                false,
                &[],
            ),
            Self::CameraShakeDeblur => spec(
                "Camera-Shake Deblur",
                Category::BlurSharpen,
                BPC_ALL,
                &[
                    float_unknown("Blur Duration"),
                    popup("Deblur Method", DEBLUR_METHOD, None, true),
                    Control::Float {
                        name: "Strength",
                        default: Some(100.0),
                        min: None,
                        max: Some(200.0),
                    },
                ],
                &["Shake Sensitivity"],
                &[],
                false,
                &[],
            ),
            Self::CcCrossBlur => spec(
                "CC Cross Blur",
                Category::BlurSharpen,
                BPC_ALL,
                &[
                    float_unknown("Radius X"),
                    float_unknown("Radius Y"),
                    popup("Transfer Mode", CROSS_MODE, None, true),
                    checkbox_unknown("Repeat Edge Pixels"),
                ],
                &[],
                &[],
                false,
                &[],
            ),
            Self::CcRadialBlur => spec(
                "CC Radial Blur",
                Category::BlurSharpen,
                BPC_ALL,
                &[
                    popup("Type", RADIAL_CC, None, true),
                    float_unknown("Amount"),
                    point_unknown("Center"),
                ],
                &["Quality"],
                &[],
                false,
                &[],
            ),
            Self::CcRadialFastBlur => spec(
                "CC Radial Fast Blur",
                Category::BlurSharpen,
                BPC_8_16,
                &[
                    point_unknown("Center"),
                    popup("Zoom", FAST_ZOOM, None, true),
                ],
                &["Amount"],
                &[],
                false,
                &[],
            ),
            Self::CcVectorBlur => spec(
                "CC Vector Blur",
                Category::BlurSharpen,
                BPC_8_16,
                &[
                    popup("Type", VECTOR_TYPE, None, true),
                    angle_unknown("Angle Offset"),
                    layer("Vector Map"),
                    popup("Property", VECTOR_PROPERTY, None, true),
                ],
                &["Amount", "Ridge Smoothness/Revolutions", "Map Softness"],
                &[],
                false,
                &[],
            ),
            Self::ChannelBlur => spec(
                "Channel Blur",
                Category::BlurSharpen,
                BPC_ALL,
                &[],
                &[],
                &[],
                false,
                &[],
            ),
            Self::CompoundBlur => spec(
                "Compound Blur",
                Category::BlurSharpen,
                BPC_ALL,
                &[
                    float_unknown("Maximum Blur"),
                    checkbox_unknown("Invert Blur"),
                    checkbox_unknown("Stretch Map To Fit"),
                ],
                &[],
                &[],
                false,
                &[],
            ),
            Self::DirectionalBlur => spec(
                "Directional Blur",
                Category::BlurSharpen,
                BPC_ALL,
                &[],
                &[],
                &[],
                true,
                &[],
            ),
            Self::FastBoxBlur => spec(
                "Fast Box Blur",
                Category::BlurSharpen,
                BPC_ALL,
                &[],
                &[],
                FAST_BOX_MATCH,
                true,
                &[],
            ),
            Self::GaussianBlur => spec(
                "Gaussian Blur",
                Category::BlurSharpen,
                BPC_ALL,
                &[checkbox_unknown("Repeat Edge Pixels")],
                &[],
                &[],
                true,
                &[],
            ),
            Self::RadialBlur => spec(
                "Radial Blur",
                Category::BlurSharpen,
                BPC_ALL,
                &[
                    popup("Type", SPIN_ZOOM, None, true),
                    point_unknown("Center"),
                ],
                &["Amount"],
                &[],
                false,
                &[],
            ),
            Self::Sharpen => spec(
                "Sharpen",
                Category::BlurSharpen,
                BPC_ALL,
                &[],
                &[],
                &[],
                true,
                &[],
            ),
            Self::SmartBlur => spec(
                "Smart Blur",
                Category::BlurSharpen,
                BPC_8_16,
                &[popup("Mode", SMART_MODE, None, true)],
                &["Radius", "Threshold"],
                &[],
                false,
                &[],
            ),
            Self::UnsharpMask => spec(
                "Unsharp Mask",
                Category::BlurSharpen,
                BPC_ALL,
                &[float_unknown("Radius")],
                &["Threshold"],
                &[],
                false,
                &[],
            ),
            Self::Arithmetic => spec(
                "Arithmetic",
                Category::Channel,
                BPC_8,
                &[
                    popup("Operator", OPERATORS, None, true),
                    checkbox_unknown("Clip Result Values"),
                ],
                &[],
                &[],
                false,
                &[],
            ),
            Self::Blend => spec(
                "Blend",
                Category::Channel,
                BPC_8_16,
                &[
                    layer("Blend With Layer"),
                    popup("Blending mode", BLEND_METHOD, None, true),
                ],
                &["Blend With Original", "If Layer Sizes Differ"],
                &[],
                false,
                &[],
            ),
            Self::Calculations => spec(
                "Calculations",
                Category::Channel,
                BPC_8_16,
                &[
                    popup("Input Channel", INPUT_CHANNEL, None, true),
                    checkbox_unknown("Invert Input"),
                    layer("Second Layer"),
                    popup("Second Layer Channel", &[], None, false),
                    float_unknown("Second Layer Opacity"),
                    checkbox_unknown("Invert Second Layer"),
                    checkbox_unknown("Stretch Second Layer To Fit"),
                    checkbox_unknown("Preserve Transparency"),
                ],
                &[],
                &[],
                false,
                &[],
            ),
            Self::CcComposite => spec(
                "CC Composite",
                Category::Channel,
                BPC_ALL,
                &[
                    popup("Composite Original", COMPOSITE_ORIGINAL, None, false),
                    Control::Checkbox {
                        name: "RGB Only",
                        default: Some(true),
                    },
                ],
                &["Opacity"],
                &[],
                false,
                &[],
            ),
            Self::ChannelCombiner => spec(
                "Channel Combiner",
                Category::Channel,
                BPC_16_32,
                &[
                    checkbox_unknown("Use 2nd Layer"),
                    layer("Source Layer"),
                    popup("From", COMBINER_FROM, None, false),
                    popup("To", COMBINER_TO, None, true),
                    checkbox_unknown("Invert"),
                    checkbox_unknown("Solid Alpha"),
                ],
                &[],
                &[],
                true,
                &[],
            ),
            Self::CompoundArithmetic => spec(
                "Compound Arithmetic",
                Category::Channel,
                BPC_8,
                &[
                    layer("Second Source Layer"),
                    popup("Overflow Behavior", OVERFLOW, None, true),
                    checkbox_unknown("Stretch Second Source To Fit"),
                ],
                &["Operate On Channels", "Blend With Original"],
                &[],
                false,
                &[],
            ),
            Self::Invert => spec(
                "Invert",
                Category::Channel,
                BPC_ALL,
                &[popup("Channel", INVERT_CHANNEL, None, true)],
                &["Blend With Original"],
                &[],
                true,
                &[],
            ),
            Self::Minimax => spec(
                "Minimax",
                Category::Channel,
                BPC_ALL,
                &[popup("Operation", MINIMAX_OP, None, true)],
                &[],
                &[],
                true,
                &[],
            ),
            Self::RemoveColorMatting => spec(
                "Remove Color Matting",
                Category::Channel,
                BPC_ALL,
                &[
                    color_unknown("Background Color"),
                    checkbox_unknown("Clip HDR Values"),
                ],
                &[],
                &[],
                false,
                &[],
            ),
            Self::SetChannels => spec(
                "Set Channels",
                Category::Channel,
                BPC_ALL,
                &[],
                &[],
                &[],
                true,
                &[],
            ),
            Self::SetMatte => spec(
                "Set Matte",
                Category::Channel,
                BPC_ALL,
                &[
                    layer("Take Matte From Layer"),
                    popup("Use For Matte", &[], None, false),
                    checkbox_unknown("Invert Matte"),
                    checkbox_unknown("Stretch Matte To Fit"),
                    checkbox_unknown("Composite Matte With Original"),
                    checkbox_unknown("Premultiply Matte Layer"),
                ],
                &[],
                &[],
                true,
                &[],
            ),
            Self::ShiftChannels => spec(
                "Shift Channels",
                Category::Channel,
                BPC_ALL,
                &[popup("Take Channel From", &[], None, false)],
                &[],
                &[],
                true,
                &[],
            ),
            Self::SolidComposite => spec(
                "Solid Composite",
                Category::Channel,
                BPC_ALL,
                &[
                    float_unknown("Source Opacity"),
                    color_unknown("Color"),
                    float_unknown("Opacity"),
                    popup("Blending Mode", &[], None, false),
                ],
                &[],
                &[],
                false,
                &[],
            ),
            Self::AutoColor => auto_spec("Auto Color", true),
            Self::AutoContrast => auto_spec("Auto Contrast", false),
            Self::AutoLevels => auto_spec("Auto Levels", false),
            Self::BlackWhite => spec(
                "Black & White",
                Category::ColorCorrection,
                BPC_8_16,
                &[checkbox_unknown("Tint")],
                &[],
                &[],
                false,
                &[],
            ),
            Self::BrightnessContrast => spec(
                "Brightness & Contrast",
                Category::ColorCorrection,
                BPC_ALL,
                &[
                    Control::Float {
                        name: "Brightness",
                        default: Some(0.0),
                        min: None,
                        max: None,
                    },
                    Control::Float {
                        name: "Contrast",
                        default: Some(0.0),
                        min: None,
                        max: None,
                    },
                ],
                &[],
                &[],
                true,
                &[],
            ),
            Self::BroadcastColors => spec(
                "Broadcast Colors",
                Category::ColorCorrection,
                BPC_8,
                &[
                    popup("Broadcast Locale", LOCALE, None, true),
                    popup("How To Make Color Safe", COLOR_SAFE, Some(0), true),
                    Control::Float {
                        name: "Maximum Signal Amplitude (IRE)",
                        default: Some(110.0),
                        min: None,
                        max: None,
                    },
                ],
                &[],
                &[],
                false,
                &[],
            ),
            Self::CcColorNeutralizer => spec(
                "CC Color Neutralizer",
                Category::ColorCorrection,
                BPC_ALL,
                &[
                    color_unknown("Shadows Unbalance"),
                    color_unknown("Midtones Unbalance"),
                    color_unknown("Highlights Unbalance"),
                    Control::Float {
                        name: "Pinning",
                        default: Some(0.0),
                        min: None,
                        max: None,
                    },
                    float_unknown("Blend w. Original"),
                    popup("View", NEUTRAL_VIEW, None, true),
                ],
                &["Shadows", "Midtones", "Highlights", "Black and White Point"],
                &[],
                false,
                &[],
            ),
            Self::CcColorOffset => spec(
                "CC Color Offset",
                Category::ColorCorrection,
                BPC_8_16,
                &[
                    angle_unknown("Red Phase"),
                    angle_unknown("Green Phase"),
                    angle_unknown("Blue Phase"),
                    popup("Overflow", OFFSET_OVERFLOW, None, true),
                ],
                &[],
                &[],
                false,
                &[],
            ),
            Self::CcKernel => spec(
                "CC Kernel",
                Category::ColorCorrection,
                BPC_ALL,
                &[
                    float_unknown("[1 - L1]"),
                    float_unknown("[2 - L1]"),
                    float_unknown("[3 - L1]"),
                    float_unknown("[1 - L2]"),
                    float_unknown("[2 - L2]"),
                    float_unknown("[3 - L2]"),
                    float_unknown("[1 - L3]"),
                    float_unknown("[2 - L3]"),
                    float_unknown("[3 - L3]"),
                    Control::Checkbox {
                        name: "Absolute Values",
                        default: Some(false),
                    },
                    float_unknown("Blend w. Original"),
                ],
                &["Divider"],
                &[],
                false,
                &[],
            ),
            Self::CcToner => spec(
                "CC Toner",
                Category::ColorCorrection,
                BPC_ALL,
                &[
                    popup("Tones", TONES, None, true),
                    color_unknown("Highlights"),
                    color_unknown("Brights"),
                    color_unknown("Midtones"),
                    color_unknown("Darktones"),
                    color_unknown("Shadows"),
                    float_unknown("Blend w. Original"),
                ],
                &[],
                &[],
                false,
                &[],
            ),
            Self::ChangeColor => spec(
                "Change Color",
                Category::ColorCorrection,
                BPC_ALL,
                &[
                    angle_unknown("Hue Transform"),
                    color_unknown("Color To Change"),
                    popup("Match Colors", MATCH_COLORS, None, true),
                    checkbox_unknown("Invert Color Correction Mask"),
                ],
                &[
                    "Lightness Transform",
                    "Saturation Transform",
                    "Matching Tolerance",
                    "Matching Softness",
                ],
                &[],
                true,
                &[],
            ),
            Self::ChangeToColor => spec(
                "Change to Color",
                Category::ColorCorrection,
                BPC_8_16,
                &[
                    color_unknown("From"),
                    color_unknown("To"),
                    float_unknown("Hue"),
                    float_unknown("Lightness"),
                    float_unknown("Saturation"),
                    checkbox_unknown("View Correction Matte"),
                ],
                &["Softness"],
                &[],
                false,
                &[],
            ),
            Self::ChannelMixer => spec(
                "Channel Mixer",
                Category::ColorCorrection,
                BPC_ALL,
                &[
                    float_unknown("Red-Red"),
                    float_unknown("Red-Green"),
                    float_unknown("Red-Blue"),
                    float_unknown("Red-Const"),
                    float_unknown("Green-Red"),
                    float_unknown("Green-Green"),
                    float_unknown("Green-Blue"),
                    float_unknown("Green-Const"),
                    float_unknown("Blue-Red"),
                    float_unknown("Blue-Green"),
                    float_unknown("Blue-Blue"),
                    float_unknown("Blue-Const"),
                ],
                &["Monochrome"],
                &[],
                false,
                &[],
            ),
            Self::ColorBalance => spec(
                "Color Balance",
                Category::ColorCorrection,
                BPC_8_16,
                &[checkbox_unknown("Preserve Luminosity")],
                &[],
                &[],
                false,
                &[],
            ),
            Self::ColorBalanceHls => spec(
                "Color Balance (HLS)",
                Category::ColorCorrection,
                BPC_ALL,
                &[],
                &["Hue", "Lightness", "Saturation"],
                &[],
                true,
                &[],
            ),
            Self::ColorLink => spec(
                "Color Link",
                Category::ColorCorrection,
                BPC_8,
                &[
                    layer("Source Layer"),
                    float_unknown("Clip"),
                    checkbox_unknown("Stencil Original Alpha"),
                    float_unknown("Opacity"),
                    popup("Blending Mode", &[], None, false),
                ],
                &[],
                &[],
                false,
                &[],
            ),
            Self::ColorStabilizer => spec(
                "Color Stabilizer",
                Category::ColorCorrection,
                BPC_8_16,
                &[
                    popup("Stabilize", STABILIZE, None, true),
                    point_unknown("Black Point"),
                    point_unknown("Mid Point"),
                    point_unknown("White Point"),
                ],
                &[],
                &[],
                false,
                &["Set Frame"],
            ),
            Self::Colorama => spec(
                "Colorama",
                Category::ColorCorrection,
                BPC_8_16,
                &[
                    popup("Get Phase From", PHASE_FROM, None, false),
                    layer("Add Phase"),
                    popup("Add Phase From", &[], None, false),
                    float_unknown("Phase Shift"),
                    popup("Use Preset Palette", &["Hue Cycle"], Some(0), false),
                    Control::Float {
                        name: "Cycle Repetitions",
                        default: Some(1.0),
                        min: None,
                        max: None,
                    },
                    checkbox_unknown("Interpolate Palette"),
                    popup("Modify", MODIFY, None, false),
                    checkbox_unknown("Modify Alpha"),
                    checkbox_unknown("Change Empty Pixels"),
                    color_unknown("Matching Color"),
                    Control::Float {
                        name: "Matching Tolerance",
                        default: None,
                        min: Some(0.0),
                        max: Some(1.0),
                    },
                    popup("Matching Mode", MATCHING_MODE, None, false),
                    layer("Mask Layer"),
                    popup("Masking Mode", &[], None, false),
                    checkbox_unknown("Composite Over Layer"),
                ],
                &["Matching Softness", "Blend With Original"],
                &[],
                false,
                &[],
            ),
            Self::Curves => spec(
                "Curves",
                Category::ColorCorrection,
                BPC_ALL,
                &[popup("Channel", &["RGB"], None, false)],
                &[],
                &[],
                true,
                &[],
            ),
            Self::Equalize => spec(
                "Equalize",
                Category::ColorCorrection,
                BPC_8,
                &[
                    popup("Equalize", EQUALIZE_MODE, None, true),
                    float_unknown("Amount To Equalize"),
                ],
                &[],
                &[],
                false,
                &[],
            ),
            Self::Exposure => spec(
                "Exposure",
                Category::ColorCorrection,
                BPC_ALL,
                &[
                    float_unknown("Exposure"),
                    Control::Float {
                        name: "Gamma Correction",
                        default: Some(1.0),
                        min: None,
                        max: None,
                    },
                    checkbox_unknown("Bypass Linear Light Conversion"),
                ],
                &["Master", "Individual Channels", "Offset"],
                &[],
                true,
                &[],
            ),
            Self::GammaPedestalGain => spec(
                "Gamma/Pedestal/Gain",
                Category::ColorCorrection,
                BPC_8,
                &[
                    float_unknown("Gamma"),
                    float_unknown("Pedestal"),
                    float_unknown("Gain"),
                ],
                &["Black Stretch"],
                &[],
                false,
                &[],
            ),
            Self::HueSaturation => spec(
                "Hue/Saturation",
                Category::ColorCorrection,
                BPC_ALL,
                &[
                    popup("Channel Control", &["Master"], None, false),
                    Control::Angle {
                        name: "Master Hue",
                        default_degrees: None,
                    },
                    Control::Float {
                        name: "Master Saturation",
                        default: None,
                        min: Some(-100.0),
                        max: Some(100.0),
                    },
                    Control::Float {
                        name: "Master Lightness",
                        default: None,
                        min: Some(-100.0),
                        max: Some(100.0),
                    },
                    checkbox_unknown("Colorize"),
                    angle_unknown("Colorize Hue"),
                    float_unknown("Colorize Saturation"),
                    float_unknown("Colorize Lightness"),
                ],
                &["Channel Range"],
                &[],
                true,
                &[],
            ),
            Self::LeaveColor => spec(
                "Leave Color",
                Category::ColorCorrection,
                BPC_8,
                &[
                    float_unknown("Amount To Decolor"),
                    color_unknown("Color To Leave"),
                    popup("Match Colors", LEAVE_MATCH, None, true),
                ],
                &["Edge Softness"],
                &[],
                false,
                &[],
            ),
            Self::Levels => spec(
                "Levels",
                Category::ColorCorrection,
                BPC_ALL,
                &[
                    popup("Channel", &["Alpha"], None, false),
                    float_unknown("Input Black"),
                    float_unknown("Output Black"),
                    float_unknown("Input White"),
                    float_unknown("Output White"),
                    float_unknown("Gamma"),
                    checkbox_unknown("Clip To Output Black"),
                    checkbox_unknown("Clip To Output White"),
                ],
                &[],
                &[],
                true,
                &[],
            ),
            Self::LevelsIndividual => spec(
                "Levels (Individual Controls)",
                Category::ColorCorrection,
                BPC_ALL,
                &[
                    float_unknown("Input Black"),
                    float_unknown("Output Black"),
                    float_unknown("Input White"),
                    float_unknown("Output White"),
                    float_unknown("Gamma"),
                    checkbox_unknown("Clip To Output Black"),
                    checkbox_unknown("Clip To Output White"),
                ],
                &[],
                &[],
                true,
                &[],
            ),
            Self::LumetriColor => spec(
                "Lumetri Color",
                Category::ColorCorrection,
                BPC_ALL,
                &[],
                &[],
                &[],
                true,
                &[],
            ),
            Self::PhotoFilter => spec(
                "Photo Filter",
                Category::ColorCorrection,
                BPC_ALL,
                &[
                    popup("Filter", PHOTO_FILTERS, None, false),
                    color_unknown("Color"),
                    checkbox_unknown("Preserve Luminosity"),
                ],
                &["Density"],
                &[],
                false,
                &[],
            ),
            Self::PsArbitraryMap => spec(
                "PS Arbitrary Map",
                Category::ColorCorrection,
                &[],
                &[checkbox_unknown("Apply Phase Map To Alpha")],
                &["Phase"],
                &[],
                false,
                &[],
            ),
            Self::SelectiveColor => spec(
                "Selective Color",
                Category::ColorCorrection,
                BPC_8_16,
                &[
                    popup("Method", SELECTIVE_METHOD, None, true),
                    popup("Colors", &[], None, false),
                ],
                &[],
                &[],
                false,
                &[],
            ),
            Self::ShadowHighlight => spec(
                "Shadow/Highlight",
                Category::ColorCorrection,
                BPC_8_16,
                &[
                    checkbox_unknown("Auto Amounts"),
                    float_unknown("Temporal Smoothing"),
                    checkbox_unknown("Scene Detect"),
                    float_unknown("Shadow Radius"),
                    float_unknown("Highlight Radius"),
                    Control::Float {
                        name: "Black Clip",
                        default: Some(0.1),
                        min: None,
                        max: None,
                    },
                    Control::Float {
                        name: "White Clip",
                        default: Some(0.1),
                        min: None,
                        max: None,
                    },
                ],
                &[
                    "Shadow Amount",
                    "Highlight Amount",
                    "Blend With Original",
                    "Shadow Tonal Width",
                    "Highlight Tonal Width",
                    "Color Correction",
                    "Midtone Contrast",
                ],
                &[],
                false,
                &[],
            ),
            Self::Tint => spec(
                "Tint",
                Category::ColorCorrection,
                BPC_ALL,
                &[color_unknown("Map Black To"), color_unknown("Map White To")],
                &["Amount To Tint"],
                &[],
                true,
                &["Swap Colors"],
            ),
            Self::Tritone => spec(
                "Tritone",
                Category::ColorCorrection,
                BPC_ALL,
                &[],
                &["Blend With Original"],
                &[],
                true,
                &[],
            ),
            Self::Vibrance => spec(
                "Vibrance",
                Category::ColorCorrection,
                BPC_8_16,
                &[],
                &["Vibrance", "Saturation"],
                &[],
                false,
                &[],
            ),
            Self::VideoLimiter => spec(
                "Video Limiter",
                Category::ColorCorrection,
                BPC_ALL,
                &[],
                &[],
                &[],
                true,
                &[],
            ),
        }
    }

    pub fn apply(
        self,
        input: &CpuInput<'_>,
        host: &Host<'_>,
        output: &mut Frame,
    ) -> ApplyResult<()> {
        let info = self.spec();
        if info.bits.is_empty() {
            return Err(ApplyError::new("ビット深度は要確認"));
        }
        if !info.bits.contains(&input.bits) {
            return Err(ApplyError::new("この効果はそのビット深度に対応しない"));
        }
        check_values(&info.controls, input.params)?;
        let image = Image::from_frame(input.frame)?;
        let rendered = render(self, &image, input, host)?;
        image::write_frame(&rendered, input.bits, output);
        Ok(())
    }
}

fn spec(
    name: &'static str,
    category: Category,
    bits: &'static [BitsPerChannel],
    controls: &[Control],
    untyped: &'static [&'static str],
    match_names: &'static [&'static str],
    gpu: bool,
    buttons: &'static [&'static str],
) -> EffectSpec {
    EffectSpec {
        name,
        category,
        bits,
        controls: controls.to_vec(),
        untyped,
        match_names,
        gpu,
        buttons,
    }
}

fn auto_spec(name: &'static str, snap: bool) -> EffectSpec {
    spec(
        name,
        Category::ColorCorrection,
        BPC_8_16,
        if snap {
            AUTO_COLOR.as_slice()
        } else {
            AUTO_CONTRAST.as_slice()
        },
        &["Blend With Original"],
        &[],
        false,
        &[],
    )
}

const AUTO_COLOR: [Control; 5] = [
    float_unknown("Temporal Smoothing"),
    checkbox_unknown("Scene Detect"),
    Control::Float {
        name: "Black Clip",
        default: Some(0.1),
        min: None,
        max: None,
    },
    Control::Float {
        name: "White Clip",
        default: Some(0.1),
        min: None,
        max: None,
    },
    checkbox_unknown("Snap Neutral Midtones"),
];

const AUTO_CONTRAST: [Control; 4] = [
    float_unknown("Temporal Smoothing"),
    checkbox_unknown("Scene Detect"),
    Control::Float {
        name: "Black Clip",
        default: Some(0.1),
        min: None,
        max: None,
    },
    Control::Float {
        name: "White Clip",
        default: Some(0.1),
        min: None,
        max: None,
    },
];

fn render(
    kind: EffectKind,
    image: &Image,
    input: &CpuInput<'_>,
    host: &Host<'_>,
) -> ApplyResult<Image> {
    let controls = kind.spec().controls;
    let args = Args::new(&controls, input.params)?;
    match kind {
        EffectKind::BrightnessContrast => Ok(brightness_contrast(
            image,
            args.float(0)? as f32,
            args.float(1)? as f32,
        )),
        EffectKind::BroadcastColors => Ok(broadcast_colors(
            image,
            if args.popup(0)? == 0 {
                BroadcastLocale::Ntsc
            } else {
                BroadcastLocale::Pal
            },
            match args.popup(1)? {
                0 => ColorSafe::ReduceLuminance,
                1 => ColorSafe::ReduceSaturation,
                2 => ColorSafe::KeyOutUnsafe,
                _ => ColorSafe::KeyOutSafe,
            },
            args.float(2)? as f32,
        )),
        EffectKind::CcColorOffset => Ok(color_offset(
            image,
            [
                args.angle(0)? as f32,
                args.angle(1)? as f32,
                args.angle(2)? as f32,
            ],
            match args.popup(3)? {
                0 => Overflow::Wrap,
                1 => Overflow::Solarize,
                _ => Overflow::Polarize,
            },
        )),
        EffectKind::CcCrossBlur => cross_blur(
            image,
            args.float(0)? as f32,
            args.float(1)? as f32,
            match args.popup(2)? {
                0 => CrossMode::Blend,
                1 => CrossMode::Add,
                2 => CrossMode::Screen,
                3 => CrossMode::Multiply,
                4 => CrossMode::Lighten,
                _ => CrossMode::Darken,
            },
            args.checkbox(3)?,
        ),
        EffectKind::Equalize => Ok(equalize(
            image,
            match args.popup(0)? {
                0 => EqualizeMode::Rgb,
                1 => EqualizeMode::Brightness,
                _ => EqualizeMode::Photoshop,
            },
            args.float(1)? as f32 / 100.0,
        )),
        EffectKind::RemoveColorMatting => Ok(remove_color_matting(
            image,
            args.color(0)?,
            args.checkbox(1)?,
            input.bits == BitsPerChannel::Bpc32,
        )),
        EffectKind::Curves => {
            let _ = args.popup(0)?;
            let curve = host
                .curves
                .map(|curves| curves.master.clone())
                .unwrap_or_else(Curve::identity);
            Ok(curves_master(image, &curve))
        }
        _ => Err(ApplyError::new(pending_message(kind))),
    }
}

fn pending_message(kind: EffectKind) -> String {
    let info = kind.spec();
    if info.untyped.is_empty() {
        format!("{} は型またはパラメータ名が要確認", info.name)
    } else {
        format!("{} の型は要確認", info.untyped.join(", "))
    }
}

impl CpuApply for EffectKind {
    fn apply_cpu(&self, input: &CpuInput<'_>, output: &mut Frame) -> ApplyResult<()> {
        self.apply(input, &Host::empty(), output)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StackItem {
    pub effect: EffectKind,
    pub values: ParamValues,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct EffectStack {
    items: Vec<StackItem>,
}

impl EffectStack {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn add(&mut self, effect: EffectKind, values: ParamValues) -> ApplyResult<usize> {
        let info = effect.spec();
        check_values(&info.controls, &values)?;
        self.items.push(StackItem { effect, values });
        Ok(self.items.len() - 1)
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<&StackItem> {
        self.items.get(index)
    }
}

pub fn placeholder_values(effect: EffectKind) -> ParamValues {
    let info = effect.spec();
    let mut values = ParamValues::new();
    for control in &info.controls {
        values.push(match control {
            Control::Float { .. } => effect_api::ParamValue::Float(0.0),
            Control::Angle { .. } => effect_api::ParamValue::Angle(0.0),
            Control::Checkbox { .. } => effect_api::ParamValue::Checkbox(false),
            Control::Color { .. } => effect_api::ParamValue::Color(model::Rgba::transparent()),
            Control::Point { .. } => effect_api::ParamValue::Point([0.0, 0.0]),
            Control::Popup { .. } => effect_api::ParamValue::Popup(0),
            Control::Layer { .. } => effect_api::ParamValue::Layer(None),
        });
    }
    values
}

#[cfg(test)]
mod tests;
