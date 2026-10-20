use crate::bits::BitsPerChannel;

/// CS6 以降の新規平面。各チャンネルのコードは 45。
pub const SOLID_GRAY_CODE: u8 = 45;

/// 0 から 1 の直線 RGB。深度には依存しない。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Rgba {
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub const fn transparent() -> Self {
        Self::new(0.0, 0.0, 0.0, 0.0)
    }

    pub const fn black() -> Self {
        Self::new(0.0, 0.0, 0.0, 1.0)
    }

    pub const fn white() -> Self {
        Self::new(1.0, 1.0, 1.0, 1.0)
    }

    /// 新規平面の初期色。各チャンネル 45/255。
    pub fn solid_default() -> Self {
        let g = f32::from(SOLID_GRAY_CODE) / 255.0;
        Self::new(g, g, g, 1.0)
    }

    pub fn luma(self) -> f32 {
        0.3 * self.r + 0.59 * self.g + 0.11 * self.b
    }

    pub fn quantize(self, bits: BitsPerChannel) -> Self {
        Self {
            r: bits.quantize_unit(self.r),
            g: bits.quantize_unit(self.g),
            b: bits.quantize_unit(self.b),
            a: bits.quantize_unit(self.a),
        }
    }

    pub fn clamp_unit(self) -> Self {
        Self {
            r: self.r.clamp(0.0, 1.0),
            g: self.g.clamp(0.0, 1.0),
            b: self.b.clamp(0.0, 1.0),
            a: self.a.clamp(0.0, 1.0),
        }
    }

    pub fn map_rgb(self, f: impl Fn(f32) -> f32) -> Self {
        Self {
            r: f(self.r),
            g: f(self.g),
            b: f(self.b),
            a: self.a,
        }
    }
}

impl Default for Rgba {
    fn default() -> Self {
        Self::transparent()
    }
}
