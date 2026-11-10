use model::BitsPerChannel;
use model::Rgba;

use crate::keying;
use crate::matte;
use crate::noise;
use crate::spec::{p, EffectDef, ParamSpec};
use crate::stylize;

const B8: &[BitsPerChannel] = &[BitsPerChannel::Bpc8];
const B8_16: &[BitsPerChannel] = &[BitsPerChannel::Bpc8, BitsPerChannel::Bpc16];
const B8_16_32: &[BitsPerChannel] = &[
    BitsPerChannel::Bpc8,
    BitsPerChannel::Bpc16,
    BitsPerChannel::Bpc32,
];

const BLUE: Rgba = Rgba::new(0.0, 0.0, 1.0, 1.0);
const GRAY: Rgba = Rgba::new(0.5, 0.5, 0.5, 1.0);
const WHITE: Rgba = Rgba::white();

const SPILL_METHOD: &[&str] = &["Standard", "Ultra"];
const WIRE_STYLE: &[&str] = &["Fade", "Frame Offset", "Displace", "Displace Horizontal"];
const ACCURACY: &[&str] = &["Faster", "More Accurate"];
const DIFF_VIEW: &[&str] = &[
    "Matte Corrected",
    "[A, B, Matte] Corrected, Final",
    "Uncorrected",
    "Corrected",
    "Final Output",
];
const COLOR_SPACE: &[&str] = &["Lab", "YUV", "RGB"];
const FIT: &[&str] = &["Center", "Stretch To Fit"];
const DIFF_MATTE_VIEW: &[&str] = &["Final Output", "Matte Only"];
const EXTRACT_CHANNEL: &[&str] = &["Luminance", "Red", "Green", "Blue", "Alpha"];
const KEY_OP: &[&str] = &["Key Colors", "Keep Colors"];
const LINEAR_VIEW: &[&str] = &["Final Output", "Source Only", "Matte Only"];
const BG: &[&str] = &["Black", "White"];
const SIMPLE_VIEW: &[&str] = &["Final Output", "Matte"];
const ADD_VIEW: &[&str] = &["Preview", "Blending Matte", "Final Output"];
const MATCH_VIEW: &[&str] = &[
    "Noise Samples",
    "Compensation Samples",
    "Preview",
    "Blending Matte",
    "Final Output",
];
const REMOVE_VIEW: &[&str] = &["Noise Samples", "Preview", "Blending Matte", "Final Output"];
const GRAIN_BLEND: &[&str] = &["Film", "Multiply", "Add", "Screen", "Overlay"];
const OVERFLOW: &[&str] = &["Clip", "Soft Clamp", "Wrap Back", "Allow HDR Results"];
const NOISE_ALPHA_KIND: &[&str] = &[
    "Unique Random",
    "Squared Random",
    "Uniform Animation",
    "Squared Animation",
];
const ORIGINAL_ALPHA: &[&str] = &["Add", "Clamp", "Scale", "Edges"];
const ALPHA_OVERFLOW: &[&str] = &["Clip", "Wrap Back", "Wrap"];
const HLS_NOISE: &[&str] = &["Uniform", "Squared", "Grain"];
const REMOVE_MODE: &[&str] = &["Multichannel", "Single Channel"];
const INPUT_NOISE: &[&str] = &["Internal", "This Layer", "Other Layer"];
const SOURCE_MAP: &[&str] = &["Fill", "Center"];
const MIRROR_EDGE: &[&str] = &["Off", "On"];
const CURL_VIEW: &[&str] = &["Input Noise", "Curl Generation", "Final Render"];
const PAINT: &[&str] = &[
    "Paint On Original Image",
    "Paint On Transparent",
    "Paint On White",
    "Paint On Black",
];
const CARTOON_RENDER: &[&str] = &["Fill", "Edges", "Fill & Edges", "Fill Only"];
const BUMP_PROPERTY: &[&str] = &[
    "Red",
    "Green",
    "Blue",
    "Alpha",
    "Luminance",
    "Lightness",
    "Hue",
    "Saturation",
];
const PLASTIC_PROPERTY: &[&str] = &["Red", "Green", "Blue", "Alpha", "Luminance", "Lightness"];
const LIGHT_USING: &[&str] = &["Effect Light", "AE Lights"];
const LIGHT_TYPE: &[&str] = &["Distant Light", "Point Light"];
const HEX_RENDER: &[&str] = &["Tile", "Fold Aligned", "Fold Seamlessly"];
const KALEIDA: &[&str] = &[
    "Unfold",
    "Wheel",
    "Fish Head",
    "Can Meas",
    "Flip Flop",
    "Flower",
    "Dia Cross",
    "Flipper",
    "Starlish",
];
const COLOR_LOOP: &[&str] = &["AB", "BA", "ABA", "BAB"];
const TILING: &[&str] = &[
    "Repeat",
    "Checker Flip H",
    "Checker Flip V",
    "Unfold",
    "Checker 180º",
    "Checker Flip 45º",
    "Checker 90º CW",
    "Checker 90º CCW",
    "Rosette",
    "Random",
    "None",
    "Turn CW",
    "Turn CCW",
    "Twist",
    "Slide",
    "Brick",
];
const CC_THRESHOLD_CHANNEL: &[&str] = &["Luminance", "RGB", "Saturation", "Alpha"];
const SCATTER_GRAIN: &[&str] = &["水平", "垂直", "None"];
const STROBE: &[&str] = &["Makes Layer Transparent", "Operates On Color Only"];
const TEXTURE_PLACE: &[&str] = &["Tile Texture", "Center Texture", "Stretch Texture To Fit"];

const SPILL: &[ParamSpec] = &[p("method", "Method").popup(SPILL_METHOD, true)];

const WIRE: &[ParamSpec] = &[
    p("point-a", "Point A").point(),
    p("point-b", "Point B").point(),
    p("removal-style", "Removal Style").popup(WIRE_STYLE, true),
    p("thickness", "Thickness"),
    p("slope", "Slope"),
    p("mirror-blend", "Mirror Blend").def_f(25.0),
    p("frame-offset", "Frame Offset"),
];

const COLOR_DIFF: &[ParamSpec] = &[
    p("view", "View").popup(DIFF_VIEW, false),
    p("key-color", "Key Color").color().def_color(BLUE),
    p("color-matching-accuracy", "Color Matching Accuracy").popup(ACCURACY, true),
    p("black-a", "Black").informal().slider(),
    p("white-a", "White").informal().slider(),
    p("gamma-a", "Gamma").informal().slider().def_f(1.0),
    p("black-b", "Black").informal().slider(),
    p("white-b", "White").informal().slider(),
    p("gamma-b", "Gamma").informal().slider().def_f(1.0),
    p("black-final", "Black").informal().slider(),
    p("white-final", "White").informal().slider(),
    p("gamma-final", "Gamma").informal().slider().def_f(1.0),
];

const COLOR_RANGE: &[ParamSpec] = &[
    p("color-space", "Color Space").popup(COLOR_SPACE, true),
    p("fuzziness", "Fuzziness").slider(),
    p("min-1", "Min").informal().slider(),
    p("max-1", "Max").informal().slider(),
    p("min-2", "Min").informal().slider(),
    p("max-2", "Max").informal().slider(),
    p("min-3", "Min").informal().slider(),
    p("max-3", "Max").informal().slider(),
];

const DIFFERENCE: &[ParamSpec] = &[
    p("view", "View").popup(DIFF_MATTE_VIEW, false),
    p("difference-layer", "Difference Layer").layer(),
    p("if-layer-sizes-differ", "If Layer Sizes Differ").popup(FIT, true),
    p("matching-tolerance", "Matching Tolerance").slider(),
    p("matching-softness", "Matching Softness").slider(),
    p("blur-before-difference", "Blur Before Difference").slider(),
];

const EXTRACT: &[ParamSpec] = &[
    p("channel", "Channel").popup(EXTRACT_CHANNEL, true),
    p("black-point", "Black Point").slider(),
    p("white-point", "White Point").slider(),
    p("black-softness", "Black Softness").slider(),
    p("white-softness", "White Softness").slider(),
];

const INNER: &[ParamSpec] = &[
    p("foreground", "Foreground").mask(),
    p("background", "Background").mask().def_no_mask(),
    p(
        "single-mask-highlight-radius",
        "Single Mask Highlight Radius",
    ),
    p("additional-foreground", "Additional Foreground").mask(),
    p("additional-background", "Additional Background").mask(),
    p("cleanup-foreground", "Cleanup Foreground").mask(),
    p("cleanup-background", "Cleanup Background").mask(),
    p("brush-radius", "Brush Radius"),
    p("brush-pressure", "Brush Pressure"),
    p("edge-thin", "Edge Thin"),
    p("edge-feather", "Edge Feather"),
    p("edge-threshold", "Edge Threshold"),
    p("invert-extraction", "Invert Extraction"),
    p("blend-with-original", "Blend With Original"),
];

const KEYLIGHT: &[ParamSpec] = &[
    p("screen-colour", "Screen Colour").color(),
    p("indeterminate-result", "Indeterminate Result"),
];

const LINEAR: &[ParamSpec] = &[
    p("key-operation", "Key Operation").popup(KEY_OP, false),
    p("match-colors", "Match Colors")
        .popup(&["RGB"], false)
        .def_choice("RGB"),
    p("view", "View").popup(LINEAR_VIEW, false),
    p("key-color", "Key Color").color(),
    p("matching-tolerance", "Matching Tolerance")
        .slider()
        .range(0.0, 100.0),
    p("matching-softness", "Matching Softness").slider(),
];

const UNMULT: &[ParamSpec] = &[
    p("background-color", "Background Color").popup(BG, true),
    p("black-level", "Black Level").slider(),
    p("white-level", "White Level").slider(),
    p("softness", "Softness").slider(),
    p("remove-color-matting", "Remove Color Matting").check(),
    p("clip-hdr-results", "Clip HDR Results").check(),
];

const CHOKER: &[ParamSpec] = &[
    p("geometric-softness-1", "Geometric Softness").informal(),
    p("choke-1", "Choke").informal(),
    p("gray-level-softness-1", "Gray Level Softness").informal(),
    p("geometric-softness-2", "Geometric Softness").informal(),
    p("choke-2", "Choke").informal(),
    p("gray-level-softness-2", "Gray Level Softness").informal(),
    p("iterations", "Iterations"),
];

const REFINE_SOFT: &[ParamSpec] = &[
    p("calculate-edge-details", "Calculate Edge Details").check(),
    p("additional-edge-radius", "Additional Edge Radius"),
    p("view-edge-region", "View Edge Region").check(),
    p("smooth", "Smooth"),
    p("feather", "Feather"),
    p("contrast", "Contrast"),
    p("shift-edge", "Shift Edge"),
    p("chatter-reduction", "Chatter Reduction"),
    p("more-motion-blur", "More Motion Blur").check(),
    p("decontaminate-edge-colors", "Decontaminate Edge Colors").check(),
    p("decontaminate-amount", "Amount"),
    p("extend-where-smoothed", "Extend Where Smoothed"),
    p("increase-decontaminate-radius", "Increase Radius"),
    p("view-map", "View Map"),
];

const SIMPLE: &[ParamSpec] = &[
    p("view", "View").popup(SIMPLE_VIEW, false),
    p("choke-matte", "Choke Matte"),
];

const ADD_GRAIN: &[ParamSpec] = &[
    p("viewing-mode", "Viewing Mode")
        .popup(ADD_VIEW, true)
        .def_choice("Preview"),
    p("preset", "Preset").popup(&[], false),
    p("intensity", "Intensity"),
    p("size", "Size"),
    p("softness", "Softness"),
    p("aspect-ratio", "Aspect Ratio"),
    p("channel-intensities", "Channel Intensities"),
    p("channel-size", "Channel Size"),
    p("monochromatic", "Monochromatic").def_bool(false),
    p("tint-amount", "Tint Amount"),
    p("tint-color", "Tint Color").color(),
    p("saturation", "Saturation"),
    p("blending-mode", "Blending Mode").popup(GRAIN_BLEND, true),
    p("shadows", "Shadows"),
    p("midtones", "Midtones"),
    p("highlights", "Highlights"),
    p("midpoint", "Midpoint").slider().def_f(0.5),
    p("channel-balance", "Channel Balance"),
    p("animation-speed", "Animation Speed").def_f(1.0),
    p("animate-smoothly", "Animate Smoothly").check(),
    p("random-seed", "Random Seed"),
    p("blend-amount", "Amount").def_f(0.0),
    p("matching-color", "Matching Color")
        .color()
        .def_color(GRAY),
    p("invert-match", "Invert Match").check().def_bool(false),
    p("matching-tolerance", "Matching Tolerance"),
    p("match-color-using", "Match Color Using")
        .popup(&["RGB"], false)
        .def_choice("RGB"),
    p("matching-softness", "Matching Softness"),
    p("mask-layer", "Mask Layer").layer(),
    p("masking-mode", "Masking Mode")
        .informal()
        .popup(&[], false),
    p("if-mask-size-differs", "If Mask Size Differs").popup(FIT, true),
    p("blur-matte", "Blur Matte"),
    p("combine-match-and-mask", "Combine Match and Mask Using")
        .popup(&["Screen", "Multiply"], true),
    p("center", "Center").button(),
    p("width", "Width"),
    p("height", "Height"),
    p("show-box", "Show Box").check(),
    p("box-color", "Box Color").color(),
];

const DUST: &[ParamSpec] = &[p("radius", "Radius"), p("threshold", "Threshold")];

const FRACTAL: &[ParamSpec] = &[
    p("fractal-type", "Fractal Type").popup(&[], false),
    p("noise-type", "Noise Type").popup(&[], false),
    p("invert", "Invert").check(),
    p("contrast", "Contrast").def_f(100.0),
    p("brightness", "Brightness"),
    p("overflow", "Overflow").popup(OVERFLOW, true),
    p("perspective-offset", "Perspective Offset"),
    p("size", "Size"),
    p("complexity", "Complexity"),
    p("sub-influence", "Sub Influence"),
    p("sub-scaling", "Sub Scaling"),
    p("sub-rotation", "Sub Rotation").angle(),
    p("sub-offset", "Sub Offset"),
    p("center-subscale", "Center Subscale"),
    p("evolution", "Evolution").angle().key_yes(),
    p("cycle-evolution", "Cycle Evolution").check(),
    p("cycle-revolutions", "Cycle (in Revolutions)"),
    p("random-seed", "Random Seed").key_yes(),
    p("opacity", "Opacity"),
    p("blending-mode", "Blending Mode").popup(&[], false),
];

const MATCH_GRAIN: &[ParamSpec] = &[
    p("viewing-mode", "Viewing Mode")
        .popup(MATCH_VIEW, true)
        .def_choice("Preview"),
    p("noise-source-layer", "Noise Source Layer").layer(),
    p(
        "compensate-for-existing-noise",
        "Compensate for Existing Noise",
    )
    .slider(),
    p("intensity", "Intensity"),
    p("size", "Size"),
    p("softness", "Softness"),
    p("aspect-ratio", "Aspect Ratio"),
    p("channel-intensities", "Channel Intensities"),
    p("channel-size", "Channel Size"),
    p("monochromatic", "Monochromatic"),
    p("tint-amount", "Tint Amount"),
    p("tint-color", "Tint Color").color(),
    p("saturation", "Saturation"),
    p("blending-mode", "Blending Mode").popup(GRAIN_BLEND, true),
    p("shadows", "Shadows"),
    p("midtones", "Midtones"),
    p("highlights", "Highlights"),
    p("midpoint", "Midpoint").slider().def_f(0.5),
    p("channel-balance", "Channel Balance"),
    p("animation-speed", "Animation Speed").def_f(1.0),
    p("animate-smoothly", "Animate Smoothly").check(),
    p("random-seed", "Random Seed"),
    p("sample-selection", "Sample Selection").popup(&["Manual"], false),
    p("sampling-mode", "Sampling Mode").popup(&["Manual"], false),
    p("number-of-samples", "Number of Samples"),
    p("noise-sample-points", "Noise Sample Points").point(),
    p("source-frame", "Source Frame").def_first_frame(),
    p("sample-box-color", "Sample Box Color")
        .color()
        .def_color(WHITE),
    p("compensation-sample-points", "Compensation Sample Points").point(),
    p("blend-amount", "Amount").def_f(0.0),
    p("matching-color", "Matching Color")
        .color()
        .def_color(GRAY),
    p("invert-match", "Invert Match").check(),
    p("matching-tolerance", "Matching Tolerance"),
    p("match-color-using", "Match Color Using")
        .popup(&["RGB"], false)
        .def_choice("RGB"),
    p("matching-softness", "Matching Softness"),
    p("mask-layer", "Mask Layer").layer(),
    p("masking-mode", "Masking Mode")
        .informal()
        .popup(&[], false),
    p("if-mask-size-differs", "If Mask Size Differs").popup(FIT, true),
    p("blur-matte", "Blur Matte"),
    p("combine-match-and-mask", "Combine Match and Mask Using")
        .popup(&["Screen", "Multiply"], true),
    p("center", "Center").button(),
    p("width", "Width"),
    p("height", "Height"),
    p("show-box", "Show Box").check(),
    p("box-color", "Box Color").color(),
];

const MEDIAN: &[ParamSpec] = &[p("radius", "Radius")];
const MEDIAN_LEGACY: &[ParamSpec] = &[
    p("radius", "Radius"),
    p("operate-on-alpha-channel", "Operate on Alpha Channel").check(),
];
const NOISE: &[ParamSpec] = &[
    p("amount-of-noise", "Amount of Noise"),
    p("noise-type", "Noise Type").popup(&["Use Color Noise"], false),
    p("clipping", "Clipping").check(),
];
const NOISE_ALPHA: &[ParamSpec] = &[
    p("noise", "Noise").popup(NOISE_ALPHA_KIND, false),
    p("amount", "Amount"),
    p("original-alpha", "Original Alpha").popup(ORIGINAL_ALPHA, true),
    p("overflow", "Overflow").popup(ALPHA_OVERFLOW, true),
    p("random-seed", "Random Seed").key_yes(),
    p("noise-phase", "Noise Phase").key_yes(),
    p("cycle-noise", "Cycle Noise").check(),
    p("cycle-revolutions", "Cycle (in Revolutions)"),
];
const NOISE_HLS: &[ParamSpec] = &[
    p("noise", "Noise").popup(HLS_NOISE, true).key_yes(),
    p("hue", "Hue").key_yes(),
    p("lightness", "Lightness").key_yes(),
    p("saturation", "Saturation").key_yes(),
    p("grain-size", "Grain Size").key_yes(),
    p("noise-phase", "Noise Phase").key_yes(),
];
const NOISE_HLS_AUTO: &[ParamSpec] = &[
    p("noise", "Noise").popup(HLS_NOISE, true),
    p("hue", "Hue"),
    p("lightness", "Lightness"),
    p("saturation", "Saturation"),
    p("grain-size", "Grain Size"),
    p("noise-animation-speed", "Noise Animation Speed").key_yes(),
];
const REMOVE: &[ParamSpec] = &[
    p("viewing-mode", "Viewing Mode")
        .popup(REMOVE_VIEW, false)
        .def_choice("Preview"),
    p("noise-reduction", "Noise Reduction"),
    p("red-noise-reduction", "Red Noise Reduction"),
    p("green-noise-reduction", "Green Noise Reduction"),
    p("blue-noise-reduction", "Blue Noise Reduction"),
    p("passes", "Passes"),
    p("mode", "Mode").popup(REMOVE_MODE, true),
    p("chroma-suppression", "Chroma Suppression"),
    p("texture", "Texture"),
    p("noise-size-bias", "Noise Size Bias").def_f(0.0),
    p("clean-solid-areas", "Clean Solid Areas"),
    p("unsharp-amount", "Amount"),
    p("unsharp-threshold", "Threshold"),
    p("unsharp-radius", "Radius"),
    p("temporal-enable", "Enable").check(),
    p("temporal-amount", "Amount").informal(),
    p("motion-sensitivity", "Motion Sensitivity"),
    p("sample-selection", "Sample Selection").popup(&["Manual"], false),
    p("number-of-samples", "Number of Samples"),
    p("noise-sample-points", "Noise Sample Points").point(),
    p("source-frame", "Source Frame").def_first_frame(),
    p("sample-box-color", "Sample Box Color")
        .color()
        .def_color(WHITE),
    p("blend-amount", "Amount").def_f(0.0),
    p("matching-color", "Matching Color")
        .color()
        .def_color(GRAY),
    p("invert-match", "Invert Match").check(),
    p("matching-tolerance", "Matching Tolerance"),
    p("match-color-using", "Match Color Using")
        .popup(&["RGB"], false)
        .def_choice("RGB"),
    p("matching-softness", "Matching Softness"),
    p("mask-layer", "Mask Layer").layer(),
    p("masking-mode", "Masking Mode")
        .informal()
        .popup(&[], false),
    p("if-mask-size-differs", "If Mask Size Differs").popup(FIT, true),
    p("blur-matte", "Blur Matte"),
    p("combine-match-and-mask", "Combine Match and Mask Using")
        .popup(&["Screen", "Multiply"], true),
    p("center", "Center").button(),
    p("width", "Width"),
    p("height", "Height"),
    p("show-box", "Show Box").check(),
    p("box-color", "Box Color").color(),
];

const TURBULENT: &[ParamSpec] = &[
    p("fractal-type", "Fractal Type").popup(&[], false),
    p("noise-type", "Noise Type").popup(&[], false),
    p("invert", "Invert").check(),
    p("contrast", "Contrast").def_f(100.0),
    p("brightness", "Brightness"),
    p("overflow", "Overflow").popup(OVERFLOW, true),
    p("perspective-offset", "Perspective Offset"),
    p("size", "Size"),
    p("complexity", "Complexity"),
    p("sub-influence", "Sub Influence"),
    p("sub-scaling", "Sub Scaling"),
    p("evolution", "Evolution").angle().key_yes(),
    p("turbulence-factor", "Turbulence Factor"),
    p("random-seed", "Random Seed").key_yes(),
    p("opacity", "Opacity"),
    p("blending-mode", "Blending Mode").popup(&[], false),
];

const CURL: &[ParamSpec] = &[
    p("input-noise", "Input Noise").popup(INPUT_NOISE, true),
    p("speed", "Speed").range(0.0, 100.0),
    p("direction", "Direction").angle(),
    p("other-layer", "Layer").informal().layer(),
    p("source-mapping", "Source Mapping").popup(SOURCE_MAP, true),
    p("mirror-at-edge", "Mirror at Edge").popup(MIRROR_EDGE, true),
    p("softness", "Softness").range(0.0, 100.0),
    p("scale", "Scale").slider().max_only(10000.0),
    p("scale-width", "Scale Width").slider().max_only(10000.0),
    p("scale-height", "Scale Height").slider().max_only(10000.0),
    p("offset", "Offset"),
    p("rotation", "Rotation"),
    p("evolution", "Evolution").angle().key_yes(),
    p("turbulence-speed", "Turbulence Speed")
        .def_f(10.0)
        .range(0.0, 500.0),
    p("noise-speed", "Noise Speed"),
    p("swirl", "Swirl").range(-360.0, 360.0),
    p("density", "Density").range(-100.0, 100.0),
    p("smoothness", "Smoothness").range(0.0, 100.0),
    p("vertical-bias", "Vertical Bias").range(0.0, 100.0),
    p("sample-count", "Sample Count").range(3.0, 24.0),
    p("sample-radius", "Sample Radius").range(0.0, 200.0),
    p("flow-softness", "Flow Softness").range(0.0, 100.0),
    p("edge-definition", "Edge Definition").range(0.0, 100.0),
    p("flow-falloff", "Flow Falloff").range(0.0, 100.0),
    p("view", "View").popup(CURL_VIEW, true),
    p("contrast", "Contrast"),
    p("brightness", "Brightness"),
    p("clip-hdr-results", "Clip HDR Results"),
    p("channel", "Channel").popup(&[], false),
];

const BRUSH: &[ParamSpec] = &[
    p("stroke-angle", "Stroke Angle").angle(),
    p("brush-size", "Brush Size"),
    p("stroke-length", "Stroke Length"),
    p("stroke-density", "Stroke Density"),
    p("stroke-randomness", "Stroke Randomness"),
    p("paint-surface", "Paint Surface")
        .popup(PAINT, true)
        .def_choice("Paint On Original Image"),
    p("blend-with-original", "Blend With Original"),
];

const CARTOON: &[ParamSpec] = &[
    p("render", "Render").popup(CARTOON_RENDER, false).key_yes(),
    p("detail-radius", "Detail Radius").key_yes(),
    p("detail-threshold", "Detail Threshold").key_yes(),
    p("shading-steps", "Shading Steps").key_yes(),
    p("shading-smoothness", "Shading Smoothness").key_yes(),
    p("edge-threshold", "Threshold").key_yes(),
    p("edge-width", "Width").key_yes(),
    p("edge-softness", "Softness").key_yes(),
    p("edge-opacity", "Opacity").key_yes(),
    p("edge-enhancement", "Edge Enhancement").key_yes(),
    p("edge-black-level", "Edge Black Level").key_yes(),
    p("edge-contrast", "Edge Contrast").key_yes(),
];

const BLOCK: &[ParamSpec] = &[
    p("completion", "Completion").key_yes(),
    p("scans", "Scans"),
    p("start-cleared", "Start Cleared").check(),
    p("bilinear", "Bilinear").check(),
];

const BURN: &[ParamSpec] = &[
    p("burn", "Burn").slider().key_yes(),
    p("center", "Center").point(),
    p("random-seed", "Random Seed"),
];

const GLASS: &[ParamSpec] = &[
    p("bump-map", "Bump Map").layer().def_current_layer(),
    p("property", "Property").popup(BUMP_PROPERTY, true),
    p("softness", "Softness"),
    p("height", "Height"),
    p("displacement", "Displacement"),
    p("using", "Using").popup(LIGHT_USING, true),
    p("light-intensity", "Light Intensity").slider(),
    p("light-color", "Light Color").color(),
    p("light-type", "Light Type").popup(LIGHT_TYPE, true),
    p("light-height", "Light Height"),
    p("light-position", "Light Position").point(),
    p("light-direction", "Light Direction").angle(),
    p("ambient", "Ambient"),
    p("diffuse", "Diffuse"),
    p("specular", "Specular"),
    p("roughness", "Roughness"),
    p("metal", "Metal"),
];

const HEX: &[ParamSpec] = &[
    p("render", "Render").popup(HEX_RENDER, true),
    p("radius", "Radius"),
    p("center", "Center").point(),
    p("lock-center-tile", "Lock Center Tile")
        .check()
        .def_bool(false),
    p("rotate", "Rotate").angle(),
    p("smearing", "Smearing"),
];

const KALEIDA_PARAMS: &[ParamSpec] = &[
    p("center", "Center").point(),
    p("size", "Size"),
    p("mirroring", "Mirroring").popup(KALEIDA, true),
    p("rotation", "Rotation").angle(),
    p("floating-center", "Floating Center")
        .check()
        .def_bool(false),
];

const SMOOTHIE: &[ParamSpec] = &[
    p("flow-layer", "Flow Layer").layer().def_current_layer(),
    p("property", "Property")
        .popup(BUMP_PROPERTY, true)
        .def_choice("Luminance"),
    p("smoothness", "Smoothness"),
    p("sample-a", "Sample A").point(),
    p("sample-b", "Sample B").point(),
    p("phase", "Phase").angle(),
    p("color-loop", "Color Loop").popup(COLOR_LOOP, true),
];

const PLASTIC: &[ParamSpec] = &[
    p("bump-layer", "Bump layer").layer().def_current_layer(),
    p("property", "Property").popup(PLASTIC_PROPERTY, true),
    p("softness", "Softness"),
    p("height", "Height"),
    p("cut-min", "Cut Min"),
    p("cut-max", "Cut Max"),
    p("using", "Using").popup(LIGHT_USING, true),
    p("light-intensity", "Light Intensity").slider(),
    p("light-color", "Light Color").color(),
    p("light-type", "Light Type").popup(LIGHT_TYPE, true),
    p("light-height", "Light Height"),
    p("light-position", "Light Position").point(),
    p("light-direction", "Light Direction").angle(),
    p("ambient-light-color", "Ambient Light Color").color(),
    p("ambient", "Ambient"),
    p("diffuse", "Diffuse"),
    p("dust", "Dust"),
    p("specular", "Specular"),
    p("roughness", "Roughness"),
    p("metal", "Metal"),
];

const REPE: &[ParamSpec] = &[
    p("expand-right", "Expand Right"),
    p("expand-left", "Expand Left"),
    p("expand-down", "Expand Down"),
    p("expand-up", "Expand Up"),
    p("tiling", "Tiling").popup(TILING, true),
    p("blend-borders", "Blend Borders"),
];

const CC_TH: &[ParamSpec] = &[
    p("threshold", "Threshold"),
    p("channel", "Channel").popup(CC_THRESHOLD_CHANNEL, true),
    p("invert", "Invert").check().def_bool(false),
    p("blend-with-original", "Blend w. Original"),
];

const CC_TH_RGB: &[ParamSpec] = &[
    p("red-threshold", "Red Threshold"),
    p("green-threshold", "Green Threshold"),
    p("blue-threshold", "Blue Threshold"),
    p("invert-red", "Invert Red").check().def_bool(false),
    p("invert-green", "Invert Green").check().def_bool(false),
    p("invert-blue", "Invert Blue").check().def_bool(false),
    p("blend-with-original", "Blend w. Original"),
];

const VIGNETTE: &[ParamSpec] = &[
    p("amount", "Amount"),
    p("angle-of-view", "Angle of View").angle(),
    p("center", "Center").point(),
    p("pin-highlights", "Pin Highlights"),
];

const EMBOSS: &[ParamSpec] = &[
    p("direction", "Direction").angle(),
    p("relief", "Relief"),
    p("contrast", "Contrast"),
    p("blend-with-original", "Blend With Original"),
];

const FIND: &[ParamSpec] = &[
    p("invert", "Invert").check(),
    p("blend-with-original", "Blend With Original"),
];

const GLOW: &[ParamSpec] = &[
    p("glow-based-on", "Glow Based On").informal(),
    p("glow-threshold", "Glow Threshold"),
    p("glow-radius", "Glow Radius"),
    p("glow-intensity", "Glow Intensity"),
    p("composite-original", "Composite Original").popup(&["On Top", "Behind", "None"], false),
    p("glow-operation", "Glow Operation").popup(&["None", "Silhouette Alpha"], false),
    p("glow-colors", "Glow Colors").popup(&["A & B Colors"], false),
    p("color-looping", "Color Looping").popup(&[], false),
    p("color-loops", "Color Loops"),
    p("color-phase", "Color Phase"),
    p("ab-midpoint", "A & B Midpoint"),
    p("color-a", "Color A").color(),
    p("color-b", "Color B").color(),
    p("glow-dimensions", "Glow Dimensions").informal(),
    p("options", "Options"),
];

const MOSAIC: &[ParamSpec] = &[
    p("horizontal-blocks", "Horizontal Blocks"),
    p("vertical-blocks", "Vertical Blocks"),
    p("sharp-colors", "Sharp Colors").check(),
];

const MOTION: &[ParamSpec] = &[
    p("tile-center", "Tile Center").point(),
    p("tile-width", "Tile Width"),
    p("tile-height", "Tile Height"),
    p("output-width", "Output Width"),
    p("output-height", "Output Height"),
    p("mirror-edges", "Mirror Edges").check(),
    p("phase", "Phase"),
    p("horizontal-phase-shift", "Horizontal Phase Shift").check(),
];

const POSTER: &[ParamSpec] = &[p("level", "Level").range(2.0, 255.0)];

const ROUGH: &[ParamSpec] = &[
    p("edge-type", "Edge Type").popup(&["Rusty Color", "Roughen Color", "Photocopy Color"], false),
    p("edge-color", "Edge Color").color(),
    p("border", "Border"),
    p("edge-sharpness", "Edge Sharpness"),
    p("fractal-influence", "Fractal Influence"),
    p("scale", "Scale"),
    p("stretch", "Stretch Width or Height"),
    p("offset-turbulence", "Offset (Turbulence)"),
    p("complexity", "Complexity"),
    p("evolution", "Evolution").key_yes(),
    p("cycle-evolution", "Cycle Evolution").check(),
    p("cycle", "Cycle"),
    p("random-seed", "Random Seed").key_yes(),
];

const SCATTER: &[ParamSpec] = &[
    p("grain", "Grain").informal().popup(SCATTER_GRAIN, false),
    p("scatter-randomness", "Scatter Randomness").popup(&["Randomize Every Frame"], false),
];

const STROBE_PARAMS: &[ParamSpec] = &[
    p("strobe-color", "Strobe Color").color(),
    p("blend-with-original", "Blend With Original"),
    p("strobe-duration", "Strobe Duration (secs)"),
    p("strobe-period", "Strobe Period (secs)"),
    p("random-strobe-probability", "Random Strobe Probability"),
    p("strobe", "Strobe").popup(STROBE, false),
    p("strobe-operator", "Strobe Operator").popup(&[], false),
];

const TEXTURIZE: &[ParamSpec] = &[
    p("texture-layer", "Texture Layer").layer(),
    p("light-direction", "Light Direction").angle(),
    p("texture-contrast", "Texture Contrast"),
    p("texture-placement", "Texture Placement").popup(TEXTURE_PLACE, true),
];

pub static EFFECTS: &[EffectDef] = &[
    EffectDef {
        id: "advanced-spill-suppressor",
        name: "Advanced Spill Suppressor",
        menu: "Effect > Keying > Advanced Spill Suppressor",
        bits: Some(B8_16_32),
        params: SPILL,
        apply: keying::advanced_spill_suppressor,
    },
    EffectDef {
        id: "cc-simple-wire-removal",
        name: "CC Simple Wire Removal",
        menu: "Effect > Keying > CC Simple Wire Removal",
        bits: Some(B8_16_32),
        params: WIRE,
        apply: keying::cc_simple_wire_removal,
    },
    EffectDef {
        id: "color-difference-key",
        name: "Color Difference Key",
        menu: "Effect > Keying > Color Difference Key",
        bits: Some(B8_16),
        params: COLOR_DIFF,
        apply: keying::color_difference_key,
    },
    EffectDef {
        id: "color-range",
        name: "Color Range",
        menu: "Effect > Keying > Color Range",
        bits: Some(B8),
        params: COLOR_RANGE,
        apply: keying::color_range,
    },
    EffectDef {
        id: "difference-matte",
        name: "Difference Matte",
        menu: "Effect > Keying > Difference Matte",
        bits: Some(B8_16),
        params: DIFFERENCE,
        apply: keying::difference_matte,
    },
    EffectDef {
        id: "extract",
        name: "Extract",
        menu: "Effect > Keying > Extract",
        bits: Some(B8_16),
        params: EXTRACT,
        apply: keying::extract,
    },
    EffectDef {
        id: "inner-outer-key",
        name: "Inner/Outer Key",
        menu: "Effect > Keying > Inner/Outer Key",
        bits: Some(B8_16),
        params: INNER,
        apply: keying::inner_outer_key,
    },
    EffectDef {
        id: "key-cleaner",
        name: "Key Cleaner",
        menu: "Effect > Keying > Key Cleaner",
        bits: Some(B8_16),
        params: &[],
        apply: keying::key_cleaner,
    },
    EffectDef {
        id: "keylight",
        name: "Keylight (1.2)",
        menu: "Effect > Keying > Keylight (1.2)",
        bits: Some(B8_16_32),
        params: KEYLIGHT,
        apply: keying::keylight,
    },
    EffectDef {
        id: "linear-color-key",
        name: "Linear Color Key",
        menu: "Effect > Keying > Linear Color Key",
        bits: Some(B8_16_32),
        params: LINEAR,
        apply: keying::linear_color_key,
    },
    EffectDef {
        id: "unmult",
        name: "Unmult",
        menu: "Effect > Keying > Unmult",
        bits: Some(B8_16_32),
        params: UNMULT,
        apply: keying::unmult,
    },
    EffectDef {
        id: "matte-choker",
        name: "Matte Choker",
        menu: "Effect > Matte > Matte Choker",
        bits: Some(B8_16),
        params: CHOKER,
        apply: matte::matte_choker,
    },
    EffectDef {
        id: "mocha-shape",
        name: "Mocha shape",
        menu: "Effect > Matte > Mocha shape",
        bits: None,
        params: &[],
        apply: matte::mocha_shape,
    },
    EffectDef {
        id: "refine-hard-matte",
        name: "Refine Hard Matte",
        menu: "Effect > Matte > Refine Hard Matte",
        bits: None,
        params: &[],
        apply: matte::refine_hard_matte,
    },
    EffectDef {
        id: "refine-soft-matte",
        name: "Refine Soft Matte",
        menu: "Effect > Matte > Refine Soft Matte",
        bits: None,
        params: REFINE_SOFT,
        apply: matte::refine_soft_matte,
    },
    EffectDef {
        id: "simple-choker",
        name: "Simple Choker",
        menu: "Effect > Matte > Simple Choker",
        bits: Some(B8_16_32),
        params: SIMPLE,
        apply: matte::simple_choker,
    },
    EffectDef {
        id: "add-grain",
        name: "Add Grain",
        menu: "Effect > Noise & Grain > Add Grain",
        bits: Some(B8_16),
        params: ADD_GRAIN,
        apply: noise::add_grain,
    },
    EffectDef {
        id: "dust-scratches",
        name: "Dust & Scratches",
        menu: "Effect > Noise & Grain > Dust & Scratches",
        bits: Some(B8_16),
        params: DUST,
        apply: noise::dust_and_scratches,
    },
    EffectDef {
        id: "fractal-noise",
        name: "Fractal Noise",
        menu: "Effect > Noise & Grain > Fractal Noise",
        bits: Some(B8_16_32),
        params: FRACTAL,
        apply: noise::fractal_noise,
    },
    EffectDef {
        id: "match-grain",
        name: "Match Grain",
        menu: "Effect > Noise & Grain > Match Grain",
        bits: Some(B8_16),
        params: MATCH_GRAIN,
        apply: noise::match_grain,
    },
    EffectDef {
        id: "median",
        name: "Median",
        menu: "Effect > Noise & Grain > Median",
        bits: Some(B8_16),
        params: MEDIAN,
        apply: noise::median,
    },
    EffectDef {
        id: "median-legacy",
        name: "Median (Legacy)",
        menu: "Effect > Noise & Grain > Median (Legacy)",
        bits: None,
        params: MEDIAN_LEGACY,
        apply: noise::median_legacy,
    },
    EffectDef {
        id: "noise",
        name: "Noise",
        menu: "Effect > Noise & Grain > Noise",
        bits: Some(B8_16_32),
        params: NOISE,
        apply: noise::noise,
    },
    EffectDef {
        id: "noise-alpha",
        name: "Noise Alpha",
        menu: "Effect > Noise & Grain > Noise Alpha",
        bits: Some(B8),
        params: NOISE_ALPHA,
        apply: noise::noise_alpha,
    },
    EffectDef {
        id: "noise-hls",
        name: "Noise HLS",
        menu: "Effect > Noise & Grain > Noise HLS",
        bits: Some(B8),
        params: NOISE_HLS,
        apply: noise::noise_hls,
    },
    EffectDef {
        id: "noise-hls-auto",
        name: "Noise HLS Auto",
        menu: "Effect > Noise & Grain > Noise HLS Auto",
        bits: Some(B8),
        params: NOISE_HLS_AUTO,
        apply: noise::noise_hls_auto,
    },
    EffectDef {
        id: "remove-grain",
        name: "Remove Grain",
        menu: "Effect > Noise & Grain > Remove Grain",
        bits: Some(B8_16),
        params: REMOVE,
        apply: noise::remove_grain,
    },
    EffectDef {
        id: "turbulent-noise",
        name: "Turbulent Noise",
        menu: "Effect > Noise & Grain > Turbulent Noise",
        bits: Some(B8_16_32),
        params: TURBULENT,
        apply: noise::turbulent_noise,
    },
    EffectDef {
        id: "curl-noise",
        name: "Curl Noise",
        menu: "Effect > Noise & Grain > Curl Noise",
        bits: None,
        params: CURL,
        apply: noise::curl_noise,
    },
    EffectDef {
        id: "brush-strokes",
        name: "Brush Strokes",
        menu: "Effect > Stylize > Brush Strokes",
        bits: Some(B8),
        params: BRUSH,
        apply: stylize::brush_strokes,
    },
    EffectDef {
        id: "cartoon",
        name: "Cartoon",
        menu: "Effect > Stylize > Cartoon",
        bits: Some(B8_16_32),
        params: CARTOON,
        apply: stylize::cartoon,
    },
    EffectDef {
        id: "cc-block-load",
        name: "CC Block Load",
        menu: "Effect > Stylize > CC Block Load",
        bits: None,
        params: BLOCK,
        apply: stylize::cc_block_load,
    },
    EffectDef {
        id: "cc-burn-film",
        name: "CC Burn Film",
        menu: "Effect > Stylize > CC Burn Film",
        bits: None,
        params: BURN,
        apply: stylize::cc_burn_film,
    },
    EffectDef {
        id: "cc-glass",
        name: "CC Glass",
        menu: "Effect > Stylize > CC Glass",
        bits: None,
        params: GLASS,
        apply: stylize::cc_glass,
    },
    EffectDef {
        id: "cc-hex-tile",
        name: "CC HexTile",
        menu: "Effect > Stylize > CC HexTile",
        bits: None,
        params: HEX,
        apply: stylize::cc_hex_tile,
    },
    EffectDef {
        id: "cc-kaleida",
        name: "CC Kaleida",
        menu: "Effect > Stylize > CC Kaleida",
        bits: None,
        params: KALEIDA_PARAMS,
        apply: stylize::cc_kaleida,
    },
    EffectDef {
        id: "cc-mr-smoothie",
        name: "CC Mr. Smoothie",
        menu: "Effect > Stylize > CC Mr. Smoothie",
        bits: None,
        params: SMOOTHIE,
        apply: stylize::cc_mr_smoothie,
    },
    EffectDef {
        id: "cc-plastic",
        name: "CC Plastic",
        menu: "Effect > Stylize > CC Plastic",
        bits: None,
        params: PLASTIC,
        apply: stylize::cc_plastic,
    },
    EffectDef {
        id: "cc-repeatile",
        name: "CC RepeTile",
        menu: "Effect > Stylize > CC RepeTile",
        bits: None,
        params: REPE,
        apply: stylize::cc_repeatile,
    },
    EffectDef {
        id: "cc-threshold",
        name: "CC Threshold",
        menu: "Effect > Stylize > CC Threshold",
        bits: None,
        params: CC_TH,
        apply: stylize::cc_threshold,
    },
    EffectDef {
        id: "cc-threshold-rgb",
        name: "CC Threshold RGB",
        menu: "Effect > Stylize > CC Threshold RGB",
        bits: None,
        params: CC_TH_RGB,
        apply: stylize::cc_threshold_rgb,
    },
    EffectDef {
        id: "cc-vignette",
        name: "CC Vignette",
        menu: "Effect > Stylize > CC Vignette",
        bits: None,
        params: VIGNETTE,
        apply: stylize::cc_vignette,
    },
    EffectDef {
        id: "color-emboss",
        name: "Color Emboss",
        menu: "Effect > Stylize > Color Emboss",
        bits: Some(B8_16),
        params: &[],
        apply: stylize::color_emboss,
    },
    EffectDef {
        id: "emboss",
        name: "Emboss",
        menu: "Effect > Stylize > Emboss",
        bits: Some(B8_16),
        params: EMBOSS,
        apply: stylize::emboss,
    },
    EffectDef {
        id: "find-edges",
        name: "Find Edges",
        menu: "Effect > Stylize > Find Edges",
        bits: Some(B8),
        params: FIND,
        apply: stylize::find_edges,
    },
    EffectDef {
        id: "glow",
        name: "Glow",
        menu: "Effect > Stylize > Glow",
        bits: Some(B8_16_32),
        params: GLOW,
        apply: stylize::glow,
    },
    EffectDef {
        id: "mosaic",
        name: "Mosaic",
        menu: "Effect > Stylize > Mosaic",
        bits: Some(B8_16),
        params: MOSAIC,
        apply: stylize::mosaic,
    },
    EffectDef {
        id: "motion-tile",
        name: "Motion Tile",
        menu: "Effect > Stylize > Motion Tile",
        bits: Some(B8),
        params: MOTION,
        apply: stylize::motion_tile,
    },
    EffectDef {
        id: "posterize",
        name: "Posterize",
        menu: "Effect > Stylize > Posterize",
        bits: Some(B8_16_32),
        params: POSTER,
        apply: stylize::posterize,
    },
    EffectDef {
        id: "roughen-edges",
        name: "Roughen Edges",
        menu: "Effect > Stylize > Roughen Edges",
        bits: Some(B8),
        params: ROUGH,
        apply: stylize::roughen_edges,
    },
    EffectDef {
        id: "scatter",
        name: "Scatter",
        menu: "Effect > Stylize > Scatter",
        bits: Some(B8_16),
        params: SCATTER,
        apply: stylize::scatter,
    },
    EffectDef {
        id: "strobe-light",
        name: "Strobe Light",
        menu: "Effect > Stylize > Strobe Light",
        bits: Some(B8),
        params: STROBE_PARAMS,
        apply: stylize::strobe_light,
    },
    EffectDef {
        id: "texturize",
        name: "Texturize",
        menu: "Effect > Stylize > Texturize",
        bits: Some(B8),
        params: TEXTURIZE,
        apply: stylize::texturize,
    },
    EffectDef {
        id: "threshold",
        name: "Threshold",
        menu: "Effect > Stylize > Threshold",
        bits: Some(B8_16),
        params: &[],
        apply: stylize::threshold,
    },
];

pub fn effect(id: &str) -> Option<&'static EffectDef> {
    EFFECTS.iter().find(|effect| effect.id == id)
}
