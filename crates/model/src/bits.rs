/// チャンネル深度。取りうる値は 8、16、32。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BitsPerChannel {
    Bpc8,
    Bpc16,
    Bpc32,
}

impl BitsPerChannel {
    pub const fn from_u8(value: u8) -> Option<Self> {
        match value {
            8 => Some(Self::Bpc8),
            16 => Some(Self::Bpc16),
            32 => Some(Self::Bpc32),
            _ => None,
        }
    }

    pub const fn bits(self) -> u8 {
        match self {
            Self::Bpc8 => 8,
            Self::Bpc16 => 16,
            Self::Bpc32 => 32,
        }
    }

    /// 白の規約値。8 は 255、16 は 32768、32 は 1.0。
    /// 32 bpc はこの値を超えるチャネルを保持できる。
    pub const fn white(self) -> f32 {
        match self {
            Self::Bpc8 => 255.0,
            Self::Bpc16 => 32768.0,
            Self::Bpc32 => 1.0,
        }
    }

    /// 深度表示の Alt+クリック。8、16、32 の順に戻る。
    pub const fn cycle(self) -> Self {
        match self {
            Self::Bpc8 => Self::Bpc16,
            Self::Bpc16 => Self::Bpc32,
            Self::Bpc32 => Self::Bpc8,
        }
    }

    /// 0 から白までの段階数。32 bpc は段階を持たない。
    pub const fn level_count(self) -> Option<u32> {
        match self {
            Self::Bpc8 => Some(256),
            Self::Bpc16 => Some(32769),
            Self::Bpc32 => None,
        }
    }

    /// 0..1 をその深度の段階へ量子化する。
    /// 8 と 16 は範囲外を白黒へ切る。32 は超過を残す。
    pub fn quantize_unit(self, value: f32) -> f32 {
        match self {
            Self::Bpc8 => quantize_steps(value, 255.0),
            Self::Bpc16 => quantize_steps(value, 32768.0),
            Self::Bpc32 => value,
        }
    }
}

fn quantize_steps(value: f32, white: f32) -> f32 {
    if !value.is_finite() {
        return 0.0;
    }
    (value.clamp(0.0, 1.0) * white).round() / white
}
