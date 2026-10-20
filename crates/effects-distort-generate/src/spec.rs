use model::BitsPerChannel;

/// 効果メニューの区分。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Category {
    Distort,
    Generate,
    Perspective,
}

/// 効果一覧の 1 項目。並びはメニュー順。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EffectKind {
    BezierWarp,
    Bulge,
    CcBendIt,
    CcBender,
    CcBlobbylize,
    CcFloMotion,
    CcGriddler,
    CcLens,
    CcPageTurn,
    CcPowerPin,
    CcRipplePulse,
    CcSlant,
    CcSmear,
    CcSplit,
    CcSplit2,
    CcTiler,
    CornerPin,
    DetailPreservingUpscale,
    DisplacementMap,
    Liquify,
    Magnify,
    MeshWarp,
    Mirror,
    Offset,
    OpticsCompensation,
    PolarCoordinates,
    Reshape,
    Ripple,
    RollingShutterRepair,
    Smear,
    Spherize,
    Transform,
    TurbulentDisplace,
    Twirl,
    Warp,
    WarpStabilizer,
    WaveWarp,
    FourColorGradient,
    AdvancedLightning,
    AudioSpectrum,
    AudioWaveform,
    Beam,
    CcGlueGun,
    CcLightBurst,
    CcLightRays,
    CcLightSweep,
    CcThreads,
    CellPattern,
    Checkerboard,
    Circle,
    Ellipse,
    EyedropperFill,
    Fill,
    Fractal,
    GradientRamp,
    Grid,
    LensFlare,
    PaintBucket,
    RadioWaves,
    Scribble,
    Stroke,
    Vegas,
    WriteOn,
    CameraTracker,
    Glasses3d,
    BevelAlpha,
    BevelEdges,
    CcCylinder,
    CcEnvironment,
    CcSphere,
    CcSpotlight,
    DropShadow,
    RadialShadow,
}

impl EffectKind {
    pub const ALL: [Self; 73] = [
        Self::BezierWarp,
        Self::Bulge,
        Self::CcBendIt,
        Self::CcBender,
        Self::CcBlobbylize,
        Self::CcFloMotion,
        Self::CcGriddler,
        Self::CcLens,
        Self::CcPageTurn,
        Self::CcPowerPin,
        Self::CcRipplePulse,
        Self::CcSlant,
        Self::CcSmear,
        Self::CcSplit,
        Self::CcSplit2,
        Self::CcTiler,
        Self::CornerPin,
        Self::DetailPreservingUpscale,
        Self::DisplacementMap,
        Self::Liquify,
        Self::Magnify,
        Self::MeshWarp,
        Self::Mirror,
        Self::Offset,
        Self::OpticsCompensation,
        Self::PolarCoordinates,
        Self::Reshape,
        Self::Ripple,
        Self::RollingShutterRepair,
        Self::Smear,
        Self::Spherize,
        Self::Transform,
        Self::TurbulentDisplace,
        Self::Twirl,
        Self::Warp,
        Self::WarpStabilizer,
        Self::WaveWarp,
        Self::FourColorGradient,
        Self::AdvancedLightning,
        Self::AudioSpectrum,
        Self::AudioWaveform,
        Self::Beam,
        Self::CcGlueGun,
        Self::CcLightBurst,
        Self::CcLightRays,
        Self::CcLightSweep,
        Self::CcThreads,
        Self::CellPattern,
        Self::Checkerboard,
        Self::Circle,
        Self::Ellipse,
        Self::EyedropperFill,
        Self::Fill,
        Self::Fractal,
        Self::GradientRamp,
        Self::Grid,
        Self::LensFlare,
        Self::PaintBucket,
        Self::RadioWaves,
        Self::Scribble,
        Self::Stroke,
        Self::Vegas,
        Self::WriteOn,
        Self::CameraTracker,
        Self::Glasses3d,
        Self::BevelAlpha,
        Self::BevelEdges,
        Self::CcCylinder,
        Self::CcEnvironment,
        Self::CcSphere,
        Self::CcSpotlight,
        Self::DropShadow,
        Self::RadialShadow,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::BezierWarp => "Bezier Warp",
            Self::Bulge => "Bulge",
            Self::CcBendIt => "CC Bend It",
            Self::CcBender => "CC Bender",
            Self::CcBlobbylize => "CC Blobbylize",
            Self::CcFloMotion => "CC Flo Motion",
            Self::CcGriddler => "CC Griddler",
            Self::CcLens => "CC Lens",
            Self::CcPageTurn => "CC Page Turn",
            Self::CcPowerPin => "CC Power Pin",
            Self::CcRipplePulse => "CC Ripple Pulse",
            Self::CcSlant => "CC Slant",
            Self::CcSmear => "CC Smear",
            Self::CcSplit => "CC Split",
            Self::CcSplit2 => "CC Split 2",
            Self::CcTiler => "CC Tiler",
            Self::CornerPin => "Corner Pin",
            Self::DetailPreservingUpscale => "Detail-preserving Upscale",
            Self::DisplacementMap => "Displacement Map",
            Self::Liquify => "Liquify",
            Self::Magnify => "Magnify",
            Self::MeshWarp => "Mesh Warp",
            Self::Mirror => "Mirror",
            Self::Offset => "Offset",
            Self::OpticsCompensation => "Optics Compensation",
            Self::PolarCoordinates => "Polar Coordinates",
            Self::Reshape => "Reshape",
            Self::Ripple => "Ripple",
            Self::RollingShutterRepair => "Rolling Shutter Repair",
            Self::Smear => "Smear",
            Self::Spherize => "Spherize",
            Self::Transform => "Transform",
            Self::TurbulentDisplace => "Turbulent Displace",
            Self::Twirl => "Twirl",
            Self::Warp => "Warp",
            Self::WarpStabilizer => "Warp Stabilizer",
            Self::WaveWarp => "Wave Warp",
            Self::FourColorGradient => "4-Color Gradient",
            Self::AdvancedLightning => "Advanced Lightning",
            Self::AudioSpectrum => "Audio Spectrum",
            Self::AudioWaveform => "Audio Waveform",
            Self::Beam => "Beam",
            Self::CcGlueGun => "CC Glue Gun",
            Self::CcLightBurst => "CC Light Burst 2.5",
            Self::CcLightRays => "CC Light Rays",
            Self::CcLightSweep => "CC Light Sweep",
            Self::CcThreads => "CC Threads",
            Self::CellPattern => "Cell Pattern",
            Self::Checkerboard => "Checkerboard",
            Self::Circle => "Circle",
            Self::Ellipse => "Ellipse",
            Self::EyedropperFill => "Eyedropper Fill",
            Self::Fill => "Fill",
            Self::Fractal => "Fractal",
            Self::GradientRamp => "Gradient Ramp",
            Self::Grid => "Grid",
            Self::LensFlare => "Lens Flare",
            Self::PaintBucket => "Paint Bucket",
            Self::RadioWaves => "Radio Waves",
            Self::Scribble => "Scribble",
            Self::Stroke => "Stroke",
            Self::Vegas => "Vegas",
            Self::WriteOn => "Write-on",
            Self::CameraTracker => "3D Camera Tracker",
            Self::Glasses3d => "3D Glasses",
            Self::BevelAlpha => "Bevel Alpha",
            Self::BevelEdges => "Bevel Edges",
            Self::CcCylinder => "CC Cylinder",
            Self::CcEnvironment => "CC Environment",
            Self::CcSphere => "CC Sphere",
            Self::CcSpotlight => "CC Spotlight",
            Self::DropShadow => "Drop Shadow",
            Self::RadialShadow => "Radial Shadow",
        }
    }

    pub const fn category(self) -> Category {
        match self {
            Self::BezierWarp
            | Self::Bulge
            | Self::CcBendIt
            | Self::CcBender
            | Self::CcBlobbylize
            | Self::CcFloMotion
            | Self::CcGriddler
            | Self::CcLens
            | Self::CcPageTurn
            | Self::CcPowerPin
            | Self::CcRipplePulse
            | Self::CcSlant
            | Self::CcSmear
            | Self::CcSplit
            | Self::CcSplit2
            | Self::CcTiler
            | Self::CornerPin
            | Self::DetailPreservingUpscale
            | Self::DisplacementMap
            | Self::Liquify
            | Self::Magnify
            | Self::MeshWarp
            | Self::Mirror
            | Self::Offset
            | Self::OpticsCompensation
            | Self::PolarCoordinates
            | Self::Reshape
            | Self::Ripple
            | Self::RollingShutterRepair
            | Self::Smear
            | Self::Spherize
            | Self::Transform
            | Self::TurbulentDisplace
            | Self::Twirl
            | Self::Warp
            | Self::WarpStabilizer
            | Self::WaveWarp => Category::Distort,
            Self::FourColorGradient
            | Self::AdvancedLightning
            | Self::AudioSpectrum
            | Self::AudioWaveform
            | Self::Beam
            | Self::CcGlueGun
            | Self::CcLightBurst
            | Self::CcLightRays
            | Self::CcLightSweep
            | Self::CcThreads
            | Self::CellPattern
            | Self::Checkerboard
            | Self::Circle
            | Self::Ellipse
            | Self::EyedropperFill
            | Self::Fill
            | Self::Fractal
            | Self::GradientRamp
            | Self::Grid
            | Self::LensFlare
            | Self::PaintBucket
            | Self::RadioWaves
            | Self::Scribble
            | Self::Stroke
            | Self::Vegas
            | Self::WriteOn => Category::Generate,
            Self::CameraTracker
            | Self::Glasses3d
            | Self::BevelAlpha
            | Self::BevelEdges
            | Self::CcCylinder
            | Self::CcEnvironment
            | Self::CcSphere
            | Self::CcSpotlight
            | Self::DropShadow
            | Self::RadialShadow => Category::Perspective,
        }
    }

    pub fn menu(self) -> String {
        let group = match self.category() {
            Category::Distort => "Distort",
            Category::Generate => "Generate",
            Category::Perspective => "Perspective",
        };
        format!("Effect > {group} > {}", self.name())
    }

    pub fn bits(self) -> &'static [BitsPerChannel] {
        use BitsPerChannel::{Bpc16, Bpc32, Bpc8};
        match self {
            Self::BezierWarp
            | Self::Bulge
            | Self::CcBendIt
            | Self::CcBender
            | Self::CcBlobbylize
            | Self::CcPageTurn
            | Self::CcSlant
            | Self::CcSplit
            | Self::CcSplit2
            | Self::Liquify
            | Self::MeshWarp
            | Self::Mirror
            | Self::Offset
            | Self::Reshape
            | Self::Ripple
            | Self::Smear
            | Self::Spherize
            | Self::Warp
            | Self::WaveWarp
            | Self::FourColorGradient
            | Self::Fractal
            | Self::BevelAlpha
            | Self::CcCylinder
            | Self::CcSpotlight => &[Bpc8, Bpc16],
            Self::Magnify
            | Self::AdvancedLightning
            | Self::CellPattern
            | Self::Checkerboard
            | Self::Circle
            | Self::EyedropperFill
            | Self::Grid
            | Self::LensFlare
            | Self::PaintBucket
            | Self::RadioWaves
            | Self::Scribble
            | Self::Stroke
            | Self::Vegas
            | Self::WriteOn
            | Self::BevelEdges
            | Self::RadialShadow => &[Bpc8],
            _ => &[Bpc8, Bpc16, Bpc32],
        }
    }

    /// 効果一覧が GPU 対応を Y としているもの。実行は CPU。
    pub const fn gpu_listed(self) -> bool {
        matches!(
            self,
            Self::Offset | Self::Transform | Self::Fill | Self::GradientRamp | Self::DropShadow
        )
    }

    /// 公式解説が Effect Controls の項目を確定していない。
    pub const fn parameters_withheld(self) -> bool {
        matches!(
            self,
            Self::CcBendIt
                | Self::CcBender
                | Self::CcBlobbylize
                | Self::CcFloMotion
                | Self::CcGriddler
                | Self::CcLens
                | Self::CcPageTurn
                | Self::CcPowerPin
                | Self::CcRipplePulse
                | Self::CcSlant
                | Self::CcSmear
                | Self::CcSplit
                | Self::CcSplit2
                | Self::CcTiler
                | Self::Spherize
                | Self::Warp
                | Self::CcGlueGun
                | Self::CcLightBurst
                | Self::CcLightRays
                | Self::CcLightSweep
                | Self::CcThreads
                | Self::Ellipse
                | Self::Fill
                | Self::LensFlare
                | Self::BevelAlpha
                | Self::BevelEdges
                | Self::CcCylinder
                | Self::CcEnvironment
                | Self::CcSphere
                | Self::CcSpotlight
        )
    }

    pub fn from_menu(menu: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.menu() == menu)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Keyframe {
    Yes,
    No,
    Unconfirmed,
    /// ボタン、ツール、グループ。
    NotApplicable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParamKind {
    Number,
    Integer,
    Angle,
    Percent,
    Checkbox,
    Color,
    Point,
    Popup,
    Layer,
    Button,
    Group,
    /// 型が解説で確定していない。
    Unconfirmed,
}

#[derive(Clone, Debug)]
pub struct ParamSpec {
    pub name: String,
    pub kind: ParamKind,
    pub default_number: Option<f64>,
    pub default_index: Option<u32>,
    pub default_bool: Option<bool>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub choices: Vec<String>,
    pub keyframe: Keyframe,
    pub group: Option<String>,
    /// 初期値が解説にある。
    pub default_confirmed: bool,
    /// ポップアップの選択肢を解説が閉じている。
    pub choices_complete: bool,
}

impl ParamSpec {
    fn new(name: &str, kind: ParamKind) -> Self {
        Self {
            name: name.to_string(),
            kind,
            default_number: None,
            default_index: None,
            default_bool: None,
            min: None,
            max: None,
            choices: Vec::new(),
            keyframe: Keyframe::Unconfirmed,
            group: None,
            default_confirmed: false,
            choices_complete: true,
        }
    }
}

struct B {
    items: Vec<ParamSpec>,
    group: Option<String>,
}

impl B {
    fn new() -> Self {
        Self {
            items: Vec::new(),
            group: None,
        }
    }

    fn group(&mut self, name: &str) -> &mut Self {
        let mut p = ParamSpec::new(name, ParamKind::Group);
        p.keyframe = Keyframe::NotApplicable;
        p.group = self.group.clone();
        self.items.push(p);
        self.group = Some(name.to_string());
        self
    }

    fn end_group(&mut self) -> &mut Self {
        self.group = None;
        self
    }

    fn push(&mut self, mut p: ParamSpec) -> &mut Self {
        if p.group.is_none() {
            p.group = self.group.clone();
        }
        self.items.push(p);
        self
    }

    fn num(&mut self, name: &str) -> &mut Self {
        self.push(ParamSpec::new(name, ParamKind::Number))
    }

    fn integer(&mut self, name: &str) -> &mut Self {
        self.push(ParamSpec::new(name, ParamKind::Integer))
    }

    fn angle(&mut self, name: &str) -> &mut Self {
        self.push(ParamSpec::new(name, ParamKind::Angle))
    }

    fn percent(&mut self, name: &str) -> &mut Self {
        self.push(ParamSpec::new(name, ParamKind::Percent))
    }

    fn check(&mut self, name: &str) -> &mut Self {
        self.push(ParamSpec::new(name, ParamKind::Checkbox))
    }

    fn color(&mut self, name: &str) -> &mut Self {
        self.push(ParamSpec::new(name, ParamKind::Color))
    }

    fn point(&mut self, name: &str) -> &mut Self {
        self.push(ParamSpec::new(name, ParamKind::Point))
    }

    fn layer(&mut self, name: &str) -> &mut Self {
        self.push(ParamSpec::new(name, ParamKind::Layer))
    }

    fn button(&mut self, name: &str) -> &mut Self {
        let mut p = ParamSpec::new(name, ParamKind::Button);
        p.keyframe = Keyframe::No;
        p.default_confirmed = true;
        self.push(p)
    }

    fn unknown(&mut self, name: &str) -> &mut Self {
        self.push(ParamSpec::new(name, ParamKind::Unconfirmed))
    }

    fn popup(&mut self, name: &str, choices: &[&str], default_index: Option<u32>) -> &mut Self {
        let mut p = ParamSpec::new(name, ParamKind::Popup);
        p.choices = choices.iter().map(|c| (*c).to_string()).collect();
        p.default_index = default_index;
        p.default_confirmed = default_index.is_some();
        self.push(p)
    }

    fn range(&mut self, min: f64, max: f64) -> &mut Self {
        if let Some(last) = self.items.last_mut() {
            last.min = Some(min);
            last.max = Some(max);
        }
        self
    }

    fn default_number(&mut self, value: f64) -> &mut Self {
        if let Some(last) = self.items.last_mut() {
            last.default_number = Some(value);
            last.default_confirmed = true;
        }
        self
    }

    fn partial_choices(&mut self) -> &mut Self {
        if let Some(last) = self.items.last_mut() {
            last.choices_complete = false;
        }
        self
    }

    fn keys(&mut self, keyframe: Keyframe) -> &mut Self {
        if let Some(last) = self.items.last_mut() {
            last.keyframe = keyframe;
        }
        self
    }

    fn done(self) -> Vec<ParamSpec> {
        self.items
    }
}

pub fn parameters(kind: EffectKind) -> Vec<ParamSpec> {
    if kind.parameters_withheld() {
        return Vec::new();
    }
    let mut b = B::new();
    match kind {
        EffectKind::Bulge => {
            b.num("Horizontal Radius")
                .num("Vertical Radius")
                .num("Bulge Height")
                .num("Taper Radius")
                .unknown("Antialiasing")
                .check("Pin All Edges");
        }
        EffectKind::DetailPreservingUpscale => {
            b.button("Fit to Comp Width")
                .button("Fit to Comp Height")
                .unknown("Scale")
                .unknown("Reduce Noise")
                .unknown("Detail")
                .popup("Alpha", &["Bilinear", "Bicubic"], None);
        }
        EffectKind::DisplacementMap => {
            b.layer("Displacement Map Layer")
                .unknown("Displacement Map Behavior")
                .check("Expand Output")
                .check("Wrap Pixels Around");
        }
        EffectKind::Liquify => {
            b.unknown("Freeze Area Mask")
                .percent("Mask Opacity")
                .unknown("Mask Feather")
                .popup(
                    "Reconstruction Mode",
                    &["Revert", "Displace", "Amplitwist", "Affine"],
                    None,
                )
                .unknown("Distortion Percentage")
                .unknown("View Options");
        }
        EffectKind::Magnify => {
            b.unknown("Shape")
                .point("Center")
                .percent("Magnification")
                .popup(
                    "Link",
                    &[
                        "None",
                        "Size To Magnification",
                        "Size & Feather To Magnification",
                    ],
                    None,
                )
                .num("Size")
                .num("Feather")
                .percent("Opacity")
                .popup("Scaling", &["Standard", "Soft", "Scatter"], None)
                .unknown("Blending Mode")
                .check("Resize Layer");
        }
        EffectKind::MeshWarp => {
            b.integer("Rows")
                .range(1.0, 31.0)
                .integer("Columns")
                .range(1.0, 31.0)
                .num("Quality")
                .unknown("Distortion Mesh")
                .keys(Keyframe::Yes);
        }
        EffectKind::Mirror => {
            b.point("Reflection Center").angle("Reflection Angle");
        }
        EffectKind::Offset => {
            b.point("Shift Center To").percent("Blend With Original");
        }
        EffectKind::OpticsCompensation => {
            b.num("Field Of View (FOV)")
                .keys(Keyframe::Yes)
                .check("Reverse Lens Distortion")
                .unknown("FOV Orientation")
                .point("View Center")
                .check("Optimal Pixels")
                .popup("Resize", &["Off", "Max 2X", "Max 4X", "Unlimited"], None);
        }
        EffectKind::PolarCoordinates => {
            b.percent("Interpolation").popup(
                "Type of Conversion",
                &["Rect To Polar", "Polar To Rect"],
                None,
            );
        }
        EffectKind::Reshape => {
            b.unknown("Source Mask")
                .unknown("Destination Mask")
                .unknown("Boundary Mask")
                .num("Percent")
                .popup(
                    "Elasticity",
                    &[
                        "Stiff",
                        "Less Stiff",
                        "Below Normal",
                        "Normal",
                        "Absolutely Normal",
                        "Above Average",
                        "Loose",
                        "Liquid",
                        "Super Fluid",
                    ],
                    None,
                )
                .unknown("Correspondence Pairs")
                .popup(
                    "Interpolation Method",
                    &["Discrete", "Linear", "Smooth"],
                    Some(1),
                );
        }
        EffectKind::Ripple => {
            b.percent("Radius")
                .point("Center Of Ripple")
                .popup("Type Of Conversion", &["Asymmetric", "Symmetric"], None)
                .num("Wave Speed")
                .num("Wave Width")
                .num("Wave Height")
                .angle("Ripple Phase")
                .keys(Keyframe::Yes);
        }
        EffectKind::RollingShutterRepair => {
            b.percent("Rolling Shutter Rate")
                .unknown("Scan Direction")
                .popup("Method", &["Pixel Motion", "Warp"], None)
                .check("Detailed Analysis")
                .num("Pixel Motion Detail");
        }
        EffectKind::Smear => {
            b.unknown("Source Mask")
                .unknown("Boundary Mask")
                .point("Mask Offset")
                .keys(Keyframe::Yes)
                .angle("Mask Rotation")
                .keys(Keyframe::Yes)
                .num("Mask Scale")
                .keys(Keyframe::Yes)
                .percent("Percent")
                .unknown("Elasticity")
                .popup(
                    "Interpolation Method",
                    &["Linear", "Discrete", "Smooth"],
                    None,
                );
        }
        EffectKind::Transform => {
            b.point("Anchor Point")
                .point("Position")
                .unknown("Scale")
                .angle("Rotation")
                .unknown("Opacity")
                .num("Skew")
                .angle("Skew Axis");
        }
        EffectKind::TurbulentDisplace => {
            b.popup(
                "Displacement",
                &[
                    "Turbulent",
                    "Bulge",
                    "Twist",
                    "Turbulent Smoother",
                    "Bulge Smoother",
                    "Twist Smoother",
                    "Vertical Displacement",
                    "Horizontal Displacement",
                    "Cross Displacement",
                ],
                None,
            )
            .num("Amount")
            .num("Size")
            .unknown("Offset (Turbulence)")
            .num("Complexity")
            .num("Evolution")
            .keys(Keyframe::Yes)
            .group("Evolution Options")
            .check("Cycle Evolution")
            .num("Cycle")
            .num("Random Seed")
            .keys(Keyframe::Yes)
            .end_group()
            .unknown("Pinning")
            .check("Resize Layer");
        }
        EffectKind::Twirl => {
            b.angle("Angle").keys(Keyframe::Yes).percent("Twirl Radius");
        }
        EffectKind::WarpStabilizer => {
            b.button("Analyze")
                .button("Cancel")
                .group("Stabilization")
                .popup("Result", &["Smooth Motion", "No Motion"], Some(0))
                .num("Smoothness")
                .popup(
                    "Method",
                    &[
                        "Position",
                        "Position, Scale And Rotation",
                        "Perspective",
                        "Subspace Warp",
                    ],
                    Some(3),
                )
                .check("Preserve Scale")
                .end_group()
                .group("Borders")
                .popup(
                    "Framing",
                    &[
                        "Stabilize Only",
                        "Stabilize, Crop",
                        "Stabilize, Crop, Auto-scale",
                        "Stabilize, Synthesize Edges",
                    ],
                    Some(2),
                )
                .group("Auto-scale")
                .num("Maximum Scale")
                .num("Action-Safe Margin")
                .end_group()
                .num("Additional Scale")
                .end_group()
                .check("Detailed Analysis")
                .popup(
                    "Rolling Shutter Ripple",
                    &["Automatic Reduction", "Enhanced Reduction"],
                    Some(0),
                )
                .partial_choices()
                .num("Crop Less <-> Smooth More")
                .num("Synthesis Input Range (seconds)")
                .num("Synthesis Edge Feather")
                .num("Synthesis Edge Cropping")
                .default_number(0.0)
                .popup(
                    "Objective",
                    &[
                        "Stabilize",
                        "Reversible Stabilization",
                        "Reverse Stabilization",
                        "Apply Motion to Target",
                        "Apply Motion to Target over Original",
                    ],
                    Some(0),
                )
                .layer("Target Layer")
                .num("Track Point Size")
                .check("Auto-delete Points Across Time")
                .check("Hide Warning Banner");
        }
        EffectKind::WaveWarp => {
            b.unknown("Wave Type")
                .num("Wave Height")
                .num("Wave Width")
                .angle("Direction")
                .num("Wave Speed")
                .unknown("Pinning")
                .angle("Phase")
                .keys(Keyframe::Yes)
                .unknown("Antialiasing");
        }
        EffectKind::FourColorGradient => {
            b.group("Positions & Colors")
                .keys(Keyframe::Yes)
                .end_group()
                .num("Blend")
                .unknown("Jitter")
                .unknown("Opacity")
                .unknown("Blending Mode");
        }
        EffectKind::AdvancedLightning => {
            b.popup(
                "Lightning Type",
                &[
                    "Vertical",
                    "Omni",
                    "Anywhere",
                    "Direction",
                    "Strike",
                    "Breaking",
                    "Bouncy",
                    "Two-Way Strike",
                ],
                None,
            )
            .point("Origin")
            .point("Direction")
            .num("Outer Radius")
            .unknown("Conductivity State")
            .keys(Keyframe::Yes)
            .group("Core Settings")
            .end_group()
            .num("Glow Opacity")
            .num("Alpha Obstacle")
            .num("Turbulence")
            .unknown("Forking")
            .num("Decay")
            .check("Decay Main Core")
            .check("Composite On Original")
            .num("Complexity")
            .num("Min. Forkdistance")
            .num("Termination Threshold")
            .check("Main Core Collision Only")
            .unknown("Fractal Type")
            .percent("Core Drain")
            .percent("Fork Strength")
            .num("Fork Variation");
        }
        EffectKind::AudioSpectrum => {
            b.layer("Audio Layer")
                .point("Start Point")
                .point("End Point")
                .unknown("Path")
                .check("Use Polar Path")
                .num("Start Frequency")
                .num("End Frequency")
                .integer("Frequency Bands")
                .num("Maximum Height")
                .num("Audio Duration")
                .num("Audio Offset")
                .num("Thickness")
                .num("Softness")
                .color("Inside Color")
                .color("Outside Color")
                .check("Blend Overlapping Colors")
                .num("Hue Interpolation")
                .check("Dynamic Hue Phase")
                .check("Color Symmetry")
                .popup(
                    "Display Options",
                    &["Digital", "Analog Lines", "Analog Dots"],
                    None,
                )
                .popup("Side Options", &["Side A", "Side B", "Side A & B"], None)
                .check("Duration Averaging")
                .check("Composite On Original");
        }
        EffectKind::AudioWaveform => {
            b.layer("Audio Layer")
                .point("Start Point")
                .point("End Point")
                .unknown("Path")
                .integer("Displayed Samples")
                .num("Maximum Height")
                .num("Audio Duration")
                .num("Audio Offset")
                .num("Thickness")
                .num("Softness")
                .color("Inside Color")
                .color("Outside Color")
                .unknown("Waveform Options")
                .popup(
                    "Display Options",
                    &["Digital", "Analog Lines", "Analog Dots"],
                    None,
                )
                .check("Composite On Original");
        }
        EffectKind::Beam => {
            b.percent("Length").percent("Time").check("3D Perspective");
        }
        EffectKind::CellPattern => {
            b.unknown("Cell Pattern")
                .check("Invert")
                .num("Contrast/Sharpness")
                .popup("Overflow", &["Clip", "Soft Clamp", "Wrap Back"], None)
                .num("Disperse")
                .num("Size")
                .default_number(60.0)
                .unknown("Offset")
                .group("Tiling Options")
                .check("Enable Tiling")
                .integer("Cells Horizontal")
                .integer("Cells Vertical")
                .end_group()
                .num("Evolution")
                .keys(Keyframe::Yes)
                .group("Evolution Options")
                .check("Cycle Evolution")
                .num("Cycle")
                .num("Random Seed")
                .keys(Keyframe::Yes)
                .end_group();
        }
        EffectKind::Checkerboard => {
            b.point("Anchor")
                .popup(
                    "Size From",
                    &["Corner Point", "Width Slider", "Width & Height Sliders"],
                    None,
                )
                .point("Corner Point")
                .num("Width")
                .num("Height")
                .unknown("Feather")
                .color("Color")
                .num("Opacity")
                .unknown("Blending Mode");
        }
        EffectKind::Circle => {
            b.popup(
                "Edge",
                &[
                    "None",
                    "Edge Radius",
                    "Thickness",
                    "Thickness * Radius",
                    "Thickness & Feather * Radius",
                ],
                None,
            )
            .num("Radius")
            .num("Thickness")
            .num("Feather")
            .check("Invert Circle")
            .unknown("Blending Mode");
        }
        EffectKind::EyedropperFill => {
            b.point("Sample Point")
                .num("Sample Radius")
                .popup(
                    "Average Pixel Color",
                    &["Skip Empty", "All", "All Premultiplied", "Including Alpha"],
                    None,
                )
                .check("Maintain Original Alpha")
                .percent("Blend With Original");
        }
        EffectKind::Fractal => {
            b.popup(
                "Set Choice",
                &[
                    "Mandelbrot",
                    "Mandelbrot Inverse",
                    "Julia",
                    "Julia Inverse",
                    "Mandelbrot Over Julia",
                    "Mandelbrot Inverse over Julia",
                ],
                None,
            )
            .num("X (Real)")
            .num("Y (Imaginary)")
            .num("Magnification")
            .integer("Escape Limit")
            .check("Overlay")
            .check("Transparency")
            .popup(
                "Palette",
                &[
                    "Lightness Gradient",
                    "Hue Wheel",
                    "Black And White",
                    "Solid Color",
                ],
                None,
            )
            .unknown("Hue")
            .integer("Cycle Steps")
            .check("Edge Highlight")
            .group("High Quality Settings")
            .popup(
                "Oversample Method",
                &[
                    "Edge Detect-Fast-May Miss Pixels",
                    "Brute Force-Slow-Every Pixel",
                ],
                None,
            )
            .integer("Oversample Factor")
            .end_group();
        }
        EffectKind::GradientRamp => {
            b.point("Start Of Ramp")
                .point("End Of Ramp")
                .num("Ramp Scatter");
        }
        EffectKind::Grid => {
            b.point("Anchor")
                .popup(
                    "Size From",
                    &["Corner Point", "Width Slider", "Width & Height Sliders"],
                    None,
                )
                .point("Corner Point")
                .num("Width")
                .num("Height")
                .num("Border")
                .num("Feather")
                .check("Invert Grid")
                .color("Color")
                .num("Opacity")
                .unknown("Blending Mode");
        }
        EffectKind::PaintBucket => {
            b.point("Fill Point")
                .popup(
                    "Fill Selector",
                    &[
                        "Color & Alpha",
                        "Straight Color",
                        "Transparency",
                        "Opacity",
                        "Alpha Channel",
                    ],
                    None,
                )
                .num("Tolerance")
                .check("View Threshold")
                .popup(
                    "Stroke",
                    &["Antialias", "Feather", "Spread", "Choke", "Stroke"],
                    None,
                )
                .num("Feather Softness")
                .num("Spread Radius")
                .num("Stroke Width")
                .color("Color")
                .num("Opacity")
                .unknown("Blending Mode");
        }
        EffectKind::RadioWaves => {
            b.point("Producer Point")
                .popup("Parameters Are Set At", &["Birth", "Each Frame"], None)
                .unknown("Render Quality")
                .unknown("Wave Type")
                .integer("Sides")
                .num("Curve Size")
                .num("Curvyness")
                .check("Star")
                .num("Star Depth")
                .unknown("Mask")
                .layer("Source Layer")
                .point("Source Center")
                .unknown("Value Channel")
                .check("Invert Input")
                .percent("Value Threshold")
                .num("Pre-Blur")
                .num("Tolerance")
                .integer("Contour")
                .num("Frequency")
                .num("Expansion")
                .angle("Orientation")
                .angle("Direction")
                .num("Velocity")
                .num("Spin")
                .num("Lifespan (sec)")
                .check("Reflection")
                .unknown("Profile")
                .color("Color")
                .num("Opacity")
                .num("Fade-in Time")
                .num("Fade-out Time")
                .num("Start Width")
                .num("End Width");
        }
        EffectKind::Scribble => {
            b.popup(
                "Scribble",
                &["Single Mask", "All Masks", "All Masks Using Modes"],
                None,
            )
            .unknown("Mask")
            .unknown("Fill Type")
            .num("Edge Width")
            .popup("End Cap", &["Round", "Butt", "Projecting"], None)
            .popup("Join", &["Round", "Bevel", "Miter"], None)
            .num("Miter Limit")
            .default_number(4.0)
            .range(1.0, 500.0)
            .popup(
                "Start/End Apply To",
                &["Scribble Result", "Mask Path"],
                None,
            )
            .angle("Angle")
            .num("Stroke Width")
            .range(0.1, 50.0)
            .angle("Curviness")
            .range(0.0, 100.0)
            .percent("Curviness Variation")
            .range(0.0, 100.0)
            .num("Spacing")
            .num("Spacing Variation")
            .num("Path Overlap")
            .num("Path Overlap Variation")
            .percent("Start")
            .percent("End")
            .check("Fill Paths Sequentially")
            .popup("Wiggle Type", &["Static", "Jumpy", "Smooth"], None)
            .num("Wiggles/Second")
            .popup(
                "Composite",
                &[
                    "On Transparent",
                    "On Original Image",
                    "Reveal Original Image",
                ],
                None,
            );
        }
        EffectKind::Stroke => {
            b.num("Brush Hardness")
                .num("Spacing")
                .unknown("Paint Style");
        }
        EffectKind::Vegas => {
            b.popup("Stroke", &["Image Contours", "Mask/Path"], None)
                .layer("Input Layer")
                .check("Invert Input")
                .popup("If Layer Sizes Differ", &["Center", "Stretch To Fit"], None)
                .unknown("Channel")
                .percent("Threshold")
                .num("Pre-Blur")
                .num("Tolerance")
                .unknown("Render")
                .integer("Selected Contour")
                .unknown("Shorter Contours Have")
                .unknown("Mask/Path")
                .integer("Segments")
                .unknown("Length")
                .popup("Segment Distribution", &["Bunched", "Even"], None)
                .num("Rotation")
                .keys(Keyframe::Yes)
                .check("Random Phase")
                .popup(
                    "Blend Mode",
                    &["Transparent", "Over", "Under", "Stencil"],
                    None,
                )
                .color("Color")
                .num("Width")
                .num("Hardness")
                .num("Start Opacity")
                .num("End Opacity")
                .num("Mid-point Opacity")
                .num("Mid-point Position");
        }
        EffectKind::WriteOn => {
            b.point("Brush Position")
                .keys(Keyframe::Yes)
                .num("Stroke Length (secs)")
                .keys(Keyframe::Yes)
                .num("Brush Spacing (secs)")
                .unknown("Paint Time Properties")
                .unknown("Brush Time Properties")
                .popup(
                    "Paint Style",
                    &[
                        "On Original Image",
                        "On Transparent",
                        "Reveal Original Image",
                    ],
                    None,
                );
        }
        EffectKind::CameraTracker => {
            b.button("Analyze")
                .popup("Shot Type", &["Fixed Angle of View", "Variable Zoom"], None)
                .unknown("Horizontal Angle of View")
                .unknown("Show Track Points")
                .check("Render Track Points")
                .num("Track Point Size")
                .num("Target Size")
                .button("Create Camera");
        }
        EffectKind::Glasses3d => {
            b.layer("Left View")
                .layer("Right View")
                .num("Convergence Offset")
                .keys(Keyframe::Yes)
                .num("Vertical Alignment")
                .popup("Units", &["Pixels", "% of Source"], None)
                .check("Swap Left-Right")
                .popup(
                    "3D View",
                    &[
                        "Stereo Pair (Side by Side)",
                        "Over Under",
                        "Interlace Upper L Lower R",
                        "Red Green LR",
                        "Red Blue LR",
                        "Balanced Red Green LR",
                        "Balanced Red Blue LR",
                        "Balanced Colored Red Blue",
                    ],
                    None,
                )
                .num("Balance");
        }
        EffectKind::DropShadow => {
            b.check("Shadow Only");
        }
        EffectKind::RadialShadow => {
            b.color("Shadow Color")
                .num("Opacity")
                .point("Light Source")
                .num("Projection Distance")
                .num("Softness")
                .popup("Render", &["Regular", "Glass Edge"], None)
                .percent("Color Influence")
                .check("Shadow Only")
                .check("Resize Layer");
        }
        _ => {}
    }
    // グループのキーフレーム指定を、直前に積んだグループへ戻す。
    b.done()
}

/// 表示名が要確認の役割。Effect Controls の項目名ではない。
pub fn unconfirmed_note(kind: EffectKind) -> Option<&'static str> {
    match kind {
        EffectKind::BezierWarp => {
            Some("高品質設定でのアニメーションに解説が触れる。境界の頂点と接線の表示名は要確認")
        }
        EffectKind::DisplacementMap => {
            Some("最大変位量と、変位に使う水平・垂直チャンネルの項目名は要確認")
        }
        EffectKind::Twirl => Some("ねじれの中心の項目名は要確認"),
        EffectKind::Transform => Some("モーションブラーのシャッター角度の項目名は要確認"),
        EffectKind::Liquify => {
            Some("ブラシサイズ、筆圧、乱流のジッタ、メッシュオフセットの正式名は要確認")
        }
        EffectKind::CornerPin => Some("四隅のピンの表示名は要確認"),
        EffectKind::MeshWarp => Some("弾性の設定の項目名は要確認"),
        EffectKind::Beam => Some("始点、終点、太さの項目名は要確認"),
        EffectKind::GradientRamp => Some("ランプの形状と色の項目名は要確認"),
        EffectKind::Circle => Some("円の中心と色の項目名は要確認"),
        EffectKind::FourColorGradient => Some("4点の位置と色の子項目の表示名は要確認"),
        EffectKind::Stroke => Some("色、不透明度、マスク、ブラシの残りの項目名は要確認"),
        EffectKind::DropShadow => Some("距離、方向、柔らかさ、色、不透明度の項目名は要確認"),
        EffectKind::WarpStabilizer => Some(
            "Rolling Shutter Ripple の初期値は Automatic Reduction。列挙されていない選択肢は要確認",
        ),
        _ => None,
    }
}
