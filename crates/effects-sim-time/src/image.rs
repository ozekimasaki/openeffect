use effect_api::{
    channel_16_to_32, channel_32_to_8, channel_8_to_16, Frame, Frame16, Frame32, Frame8,
};
use model::BitsPerChannel;

/// 線形 RGBA。白は 1。32bpc では 1 を超えてよい。
#[derive(Clone, Debug, PartialEq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub px: Vec<f32>,
}

impl Image {
    pub fn new(width: u32, height: u32, fill: [f32; 4]) -> Self {
        let count = pixel_count(width, height);
        let mut px = Vec::with_capacity(count * 4);
        for _ in 0..count {
            px.extend_from_slice(&fill);
        }
        Self { width, height, px }
    }

    pub fn at(&self, x: i32, y: i32) -> [f32; 4] {
        let x = x.clamp(0, self.width.saturating_sub(1) as i32);
        let y = y.clamp(0, self.height.saturating_sub(1) as i32);
        let index = (y as u32 * self.width + x as u32) as usize * 4;
        [
            self.px[index],
            self.px[index + 1],
            self.px[index + 2],
            self.px[index + 3],
        ]
    }

    pub fn sample(&self, x: f32, y: f32) -> [f32; 4] {
        if self.width == 0 || self.height == 0 {
            return [0.0; 4];
        }
        let x = x.clamp(0.0, (self.width - 1) as f32);
        let y = y.clamp(0.0, (self.height - 1) as f32);
        let x0 = x.floor() as i32;
        let y0 = y.floor() as i32;
        let tx = x - x0 as f32;
        let ty = y - y0 as f32;
        let a = self.at(x0, y0);
        let b = self.at(x0 + 1, y0);
        let c = self.at(x0, y0 + 1);
        let d = self.at(x0 + 1, y0 + 1);
        let mut out = [0.0; 4];
        for channel in 0..4 {
            let top = a[channel] + (b[channel] - a[channel]) * tx;
            let bottom = c[channel] + (d[channel] - c[channel]) * tx;
            out[channel] = top + (bottom - top) * ty;
        }
        out
    }

    pub fn put(&mut self, x: i32, y: i32, pixel: [f32; 4]) {
        if x < 0 || y < 0 || x as u32 >= self.width || y as u32 >= self.height {
            return;
        }
        let index = (y as u32 * self.width + x as u32) as usize * 4;
        self.px[index..index + 4].copy_from_slice(&pixel);
    }
}

/// インターリーブされた音声。
#[derive(Clone, Debug, PartialEq)]
pub struct Audio {
    pub rate: u32,
    pub channels: u8,
    pub samples: Vec<f32>,
}

impl Audio {
    pub fn silence(rate: u32, channels: u8, frames: usize) -> Self {
        let channels = channels.max(1);
        Self {
            rate,
            channels,
            samples: vec![0.0; frames * usize::from(channels)],
        }
    }

    pub fn frames(&self) -> usize {
        let channels = usize::from(self.channels.max(1));
        self.samples.len() / channels
    }
}

pub fn image_from_frame(frame: &Frame) -> Image {
    match frame {
        Frame::Bpc8(frame) => from_8(frame),
        Frame::Bpc16(frame) => from_16(frame),
        Frame::Bpc32(frame) => from_32(frame),
    }
}

pub fn image_to_frame(image: &Image, bits: BitsPerChannel) -> Frame {
    match bits {
        BitsPerChannel::Bpc8 => Frame::Bpc8(to_8(image)),
        BitsPerChannel::Bpc16 => Frame::Bpc16(to_16(image)),
        BitsPerChannel::Bpc32 => Frame::Bpc32(to_32(image)),
    }
}

fn from_8(frame: &Frame8) -> Image {
    Image {
        width: frame.width,
        height: frame.height,
        px: expand(&frame.samples, frame.channels, |value| {
            channel_16_to_32(channel_8_to_16(value))
        }),
    }
}

fn from_16(frame: &Frame16) -> Image {
    Image {
        width: frame.width,
        height: frame.height,
        px: expand(&frame.samples, frame.channels, channel_16_to_32),
    }
}

fn from_32(frame: &Frame32) -> Image {
    Image {
        width: frame.width,
        height: frame.height,
        px: expand(&frame.samples, frame.channels, |value| value),
    }
}

fn to_8(image: &Image) -> Frame8 {
    Frame8 {
        width: image.width,
        height: image.height,
        channels: 4,
        samples: image.px.iter().copied().map(channel_32_to_8).collect(),
    }
}

fn to_16(image: &Image) -> Frame16 {
    Frame16 {
        width: image.width,
        height: image.height,
        channels: 4,
        samples: image.px.iter().copied().map(f32_to_16).collect(),
    }
}

fn to_32(image: &Image) -> Frame32 {
    Frame32 {
        width: image.width,
        height: image.height,
        channels: 4,
        samples: image.px.clone(),
    }
}

fn expand<T: Copy>(samples: &[T], channels: u8, map: impl Fn(T) -> f32) -> Vec<f32> {
    let channels = usize::from(channels.max(1)).min(4);
    let mut px = Vec::with_capacity(samples.len().saturating_mul(4) / channels.max(1));
    for pixel in samples.chunks(channels) {
        let mut rgba = [0.0, 0.0, 0.0, 1.0];
        for (index, sample) in pixel.iter().copied().enumerate() {
            rgba[index] = map(sample);
        }
        px.extend_from_slice(&rgba);
    }
    px
}

fn pixel_count(width: u32, height: u32) -> usize {
    usize::try_from(width)
        .ok()
        .zip(usize::try_from(height).ok())
        .and_then(|(width, height)| width.checked_mul(height))
        .unwrap_or(0)
}

fn f32_to_16(value: f32) -> u16 {
    if !value.is_finite() || value <= 0.0 {
        0
    } else if value >= 1.0 {
        32768
    } else {
        (value * 32768.0).round() as u16
    }
}

pub fn luma(pixel: [f32; 4]) -> f32 {
    pixel[0] * 0.2126 + pixel[1] * 0.7152 + pixel[2] * 0.0722
}
