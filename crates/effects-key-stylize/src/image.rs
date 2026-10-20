use effect_api::{
    channel_16_to_32, channel_32_to_8, channel_8_to_16, ApplyError, ApplyResult, Frame, Frame16,
    Frame32, Frame8,
};
use model::BitsPerChannel;

/// 直線化した 0–1 相当の RGBA。32 bpc は 1 を超えてよい。
#[derive(Clone, Debug, PartialEq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub bits: BitsPerChannel,
    pub pixels: Vec<[f32; 4]>,
}

impl Image {
    pub fn new(width: u32, height: u32, bits: BitsPerChannel, fill: [f32; 4]) -> Self {
        let len = (width as usize).saturating_mul(height as usize);
        Self {
            width,
            height,
            bits,
            pixels: vec![fill; len],
        }
    }

    pub fn from_frame(frame: &Frame) -> ApplyResult<Self> {
        if frame.channels() != 4 {
            return Err(ApplyError::new("入力は RGBA の 4 チャンネルである"));
        }
        let pixels = match frame {
            Frame::Bpc8(frame) => frame
                .samples
                .chunks(4)
                .map(|px| {
                    [
                        u8_to_unit(px[0]),
                        u8_to_unit(px[1]),
                        u8_to_unit(px[2]),
                        u8_to_unit(px[3]),
                    ]
                })
                .collect(),
            Frame::Bpc16(frame) => frame
                .samples
                .chunks(4)
                .map(|px| {
                    [
                        channel_16_to_32(px[0]),
                        channel_16_to_32(px[1]),
                        channel_16_to_32(px[2]),
                        channel_16_to_32(px[3]),
                    ]
                })
                .collect(),
            Frame::Bpc32(frame) => frame
                .samples
                .chunks(4)
                .map(|px| [px[0], px[1], px[2], px[3]])
                .collect(),
        };
        Ok(Self {
            width: frame.width(),
            height: frame.height(),
            bits: frame.bits(),
            pixels,
        })
    }

    pub fn to_frame(&self) -> Frame {
        match self.bits {
            BitsPerChannel::Bpc8 => Frame::Bpc8(Frame8 {
                width: self.width,
                height: self.height,
                channels: 4,
                samples: self
                    .pixels
                    .iter()
                    .flat_map(|px| px.map(channel_32_to_8))
                    .collect(),
            }),
            BitsPerChannel::Bpc16 => Frame::Bpc16(Frame16 {
                width: self.width,
                height: self.height,
                channels: 4,
                samples: self
                    .pixels
                    .iter()
                    .flat_map(|px| px.map(channel_32_to_16))
                    .collect(),
            }),
            BitsPerChannel::Bpc32 => Frame::Bpc32(Frame32 {
                width: self.width,
                height: self.height,
                channels: 4,
                samples: self.pixels.iter().flat_map(|px| *px).collect(),
            }),
        }
    }

    pub fn index(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return None;
        }
        Some(y as usize * self.width as usize + x as usize)
    }

    pub fn get(&self, x: i32, y: i32) -> [f32; 4] {
        self.index(x, y)
            .map(|index| self.pixels[index])
            .unwrap_or([0.0, 0.0, 0.0, 0.0])
    }

    pub fn get_clamped(&self, x: i32, y: i32) -> [f32; 4] {
        if self.width == 0 || self.height == 0 {
            return [0.0, 0.0, 0.0, 0.0];
        }
        let x = x.clamp(0, self.width as i32 - 1);
        let y = y.clamp(0, self.height as i32 - 1);
        self.pixels[y as usize * self.width as usize + x as usize]
    }

    pub fn set(&mut self, x: i32, y: i32, pixel: [f32; 4]) {
        if let Some(index) = self.index(x, y) {
            self.pixels[index] = pixel;
        }
    }

    pub fn sample(&self, x: f32, y: f32) -> [f32; 4] {
        if self.width == 0 || self.height == 0 {
            return [0.0, 0.0, 0.0, 0.0];
        }
        let x = x.clamp(0.0, self.width as f32 - 1.0);
        let y = y.clamp(0.0, self.height as f32 - 1.0);
        let x0 = x.floor() as i32;
        let y0 = y.floor() as i32;
        let tx = x - x0 as f32;
        let ty = y - y0 as f32;
        let a = self.get_clamped(x0, y0);
        let b = self.get_clamped(x0 + 1, y0);
        let c = self.get_clamped(x0, y0 + 1);
        let d = self.get_clamped(x0 + 1, y0 + 1);
        let mut out = [0.0; 4];
        for channel in 0..4 {
            let ab = a[channel] + (b[channel] - a[channel]) * tx;
            let cd = c[channel] + (d[channel] - c[channel]) * tx;
            out[channel] = ab + (cd - ab) * ty;
        }
        out
    }
}

fn u8_to_unit(value: u8) -> f32 {
    channel_16_to_32(channel_8_to_16(value))
}

fn channel_32_to_16(value: f32) -> u16 {
    if !value.is_finite() || value <= 0.0 {
        0
    } else if value >= 1.0 {
        32768
    } else {
        (value * 32768.0).round() as u16
    }
}
