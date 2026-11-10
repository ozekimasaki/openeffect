use crate::id::LayerId;

/// トラックマットの種類。未選択は None。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum TrackMatteMode {
    #[default]
    None,
    Alpha,
    AlphaInverted,
    Luma,
    LumaInverted,
}

impl TrackMatteMode {
    pub const ALL: [Self; 5] = [
        Self::None,
        Self::Alpha,
        Self::AlphaInverted,
        Self::Luma,
        Self::LumaInverted,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Alpha => "Alpha Matte",
            Self::AlphaInverted => "Alpha Inverted Matte",
            Self::Luma => "Luma Matte",
            Self::LumaInverted => "Luma Inverted Matte",
        }
    }
}

/// マット元と種類。下の透明を保持するスイッチは別の真偽値。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct TrackMatte {
    pub source: Option<LayerId>,
    pub mode: TrackMatteMode,
}

impl TrackMatte {
    pub const fn none() -> Self {
        Self {
            source: None,
            mode: TrackMatteMode::None,
        }
    }
}
