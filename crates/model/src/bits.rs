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
}
