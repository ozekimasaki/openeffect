/// レイヤーのブレンドモード。初期値は Normal。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum BlendMode {
    #[default]
    Normal,
    Dissolve,
    DancingDissolve,
    Darken,
    Multiply,
    ColorBurn,
    ClassicColorBurn,
    LinearBurn,
    DarkerColor,
    Add,
    Lighten,
    Screen,
    ColorDodge,
    ClassicColorDodge,
    LinearDodge,
    LighterColor,
    Overlay,
    SoftLight,
    HardLight,
    LinearLight,
    VividLight,
    PinLight,
    HardMix,
    Difference,
    ClassicDifference,
    Exclusion,
    Subtract,
    Divide,
    Hue,
    Saturation,
    Color,
    Luminosity,
    StencilAlpha,
    StencilLuma,
    SilhouetteAlpha,
    SilhouetteLuma,
    AlphaAdd,
    LuminescentPremul,
}

impl BlendMode {
    pub const ALL: [Self; 38] = [
        Self::Normal,
        Self::Dissolve,
        Self::DancingDissolve,
        Self::Darken,
        Self::Multiply,
        Self::ColorBurn,
        Self::ClassicColorBurn,
        Self::LinearBurn,
        Self::DarkerColor,
        Self::Add,
        Self::Lighten,
        Self::Screen,
        Self::ColorDodge,
        Self::ClassicColorDodge,
        Self::LinearDodge,
        Self::LighterColor,
        Self::Overlay,
        Self::SoftLight,
        Self::HardLight,
        Self::LinearLight,
        Self::VividLight,
        Self::PinLight,
        Self::HardMix,
        Self::Difference,
        Self::ClassicDifference,
        Self::Exclusion,
        Self::Subtract,
        Self::Divide,
        Self::Hue,
        Self::Saturation,
        Self::Color,
        Self::Luminosity,
        Self::StencilAlpha,
        Self::StencilLuma,
        Self::SilhouetteAlpha,
        Self::SilhouetteLuma,
        Self::AlphaAdd,
        Self::LuminescentPremul,
    ];

    /// 3D レイヤーでは Dissolve と Dancing Dissolve が効かない。
    pub const fn ignored_on_3d(self) -> bool {
        matches!(self, Self::Dissolve | Self::DancingDissolve)
    }

    pub const fn next(self) -> Self {
        let index = self.index();
        Self::ALL[(index + 1) % Self::ALL.len()]
    }

    pub const fn previous(self) -> Self {
        let index = self.index();
        Self::ALL[(index + Self::ALL.len() - 1) % Self::ALL.len()]
    }

    const fn index(self) -> usize {
        let mut index = 0;
        while index < Self::ALL.len() {
            if Self::ALL[index] as u8 == self as u8 {
                return index;
            }
            index += 1;
        }
        0
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Dissolve => "Dissolve",
            Self::DancingDissolve => "Dancing Dissolve",
            Self::Darken => "Darken",
            Self::Multiply => "Multiply",
            Self::ColorBurn => "Color Burn",
            Self::ClassicColorBurn => "Classic Color Burn",
            Self::LinearBurn => "Linear Burn",
            Self::DarkerColor => "Darker Color",
            Self::Add => "Add",
            Self::Lighten => "Lighten",
            Self::Screen => "Screen",
            Self::ColorDodge => "Color Dodge",
            Self::ClassicColorDodge => "Classic Color Dodge",
            Self::LinearDodge => "Linear Dodge",
            Self::LighterColor => "Lighter Color",
            Self::Overlay => "Overlay",
            Self::SoftLight => "Soft Light",
            Self::HardLight => "Hard Light",
            Self::LinearLight => "Linear Light",
            Self::VividLight => "Vivid Light",
            Self::PinLight => "Pin Light",
            Self::HardMix => "Hard Mix",
            Self::Difference => "Difference",
            Self::ClassicDifference => "Classic Difference",
            Self::Exclusion => "Exclusion",
            Self::Subtract => "Subtract",
            Self::Divide => "Divide",
            Self::Hue => "Hue",
            Self::Saturation => "Saturation",
            Self::Color => "Color",
            Self::Luminosity => "Luminosity",
            Self::StencilAlpha => "Stencil Alpha",
            Self::StencilLuma => "Stencil Luma",
            Self::SilhouetteAlpha => "Silhouette Alpha",
            Self::SilhouetteLuma => "Silhouette Luma",
            Self::AlphaAdd => "Alpha Add",
            Self::LuminescentPremul => "Luminescent Premul",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::BlendMode;

    #[test]
    fn names_are_unique() {
        let mut names: Vec<&str> = BlendMode::ALL.iter().map(|mode| mode.name()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), BlendMode::ALL.len());
    }
}
