use model::BitsPerChannel;

/// 8 bpc。1 チャネルは 0 から 255。白は 255。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame8 {
    pub width: u32,
    pub height: u32,
    pub channels: u8,
    pub samples: Vec<u8>,
}

/// 16 bpc。1 チャネルは 0 から 32768。白は 32768。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame16 {
    pub width: u32,
    pub height: u32,
    pub channels: u8,
    pub samples: Vec<u16>,
}

/// 32 bpc。1 チャネルは線形の浮動小数で、1 を超えてよい。白は 1.0。
#[derive(Clone, Debug, PartialEq)]
pub struct Frame32 {
    pub width: u32,
    pub height: u32,
    pub channels: u8,
    pub samples: Vec<f32>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Frame {
    Bpc8(Frame8),
    Bpc16(Frame16),
    Bpc32(Frame32),
}

impl Frame {
    pub fn bits(&self) -> BitsPerChannel {
        match self {
            Self::Bpc8(_) => BitsPerChannel::Bpc8,
            Self::Bpc16(_) => BitsPerChannel::Bpc16,
            Self::Bpc32(_) => BitsPerChannel::Bpc32,
        }
    }

    pub fn width(&self) -> u32 {
        match self {
            Self::Bpc8(frame) => frame.width,
            Self::Bpc16(frame) => frame.width,
            Self::Bpc32(frame) => frame.width,
        }
    }

    pub fn height(&self) -> u32 {
        match self {
            Self::Bpc8(frame) => frame.height,
            Self::Bpc16(frame) => frame.height,
            Self::Bpc32(frame) => frame.height,
        }
    }

    pub fn channels(&self) -> u8 {
        match self {
            Self::Bpc8(frame) => frame.channels,
            Self::Bpc16(frame) => frame.channels,
            Self::Bpc32(frame) => frame.channels,
        }
    }
}

impl Frame8 {
    pub fn new(width: u32, height: u32, channels: u8, fill: u8) -> Self {
        let len = sample_len(width, height, channels);
        Self {
            width,
            height,
            channels,
            samples: vec![fill; len],
        }
    }
}

impl Frame16 {
    pub fn new(width: u32, height: u32, channels: u8, fill: u16) -> Self {
        let len = sample_len(width, height, channels);
        Self {
            width,
            height,
            channels,
            samples: vec![fill; len],
        }
    }
}

impl Frame32 {
    pub fn new(width: u32, height: u32, channels: u8, fill: f32) -> Self {
        let len = sample_len(width, height, channels);
        Self {
            width,
            height,
            channels,
            samples: vec![fill; len],
        }
    }
}

pub fn channel_8_to_16(value: u8) -> u16 {
    (u32::from(value) * 32768 / 255) as u16
}

pub fn channel_16_to_32(value: u16) -> f32 {
    f32::from(value) / 32768.0
}

pub fn channel_32_to_8(value: f32) -> u8 {
    if !value.is_finite() || value <= 0.0 {
        0
    } else if value >= 1.0 {
        255
    } else {
        (value * 255.0).round() as u8
    }
}

pub fn channel_8_to_32(value: u8) -> f32 {
    channel_16_to_32(channel_8_to_16(value))
}

pub fn channel_16_to_8(value: u16) -> u8 {
    channel_32_to_8(channel_16_to_32(value))
}

pub fn channel_32_to_16(value: f32) -> u16 {
    if !value.is_finite() || value <= 0.0 {
        0
    } else if value >= 1.0 {
        32768
    } else {
        (value * 32768.0).round() as u16
    }
}

/// 出力の深度へ変換してコピーする。サイズは入力に合わせる。
pub fn copy_frame(input: &Frame, output: &mut Frame) {
    let converted = match (input, output.bits()) {
        (Frame::Bpc8(frame), BitsPerChannel::Bpc8) => Frame::Bpc8(frame.clone()),
        (Frame::Bpc16(frame), BitsPerChannel::Bpc16) => Frame::Bpc16(frame.clone()),
        (Frame::Bpc32(frame), BitsPerChannel::Bpc32) => Frame::Bpc32(frame.clone()),
        (Frame::Bpc8(frame), BitsPerChannel::Bpc16) => Frame::Bpc16(convert_8_to_16(frame)),
        (Frame::Bpc8(frame), BitsPerChannel::Bpc32) => Frame::Bpc32(convert_8_to_32(frame)),
        (Frame::Bpc16(frame), BitsPerChannel::Bpc8) => Frame::Bpc8(convert_16_to_8(frame)),
        (Frame::Bpc16(frame), BitsPerChannel::Bpc32) => Frame::Bpc32(convert_16_to_32(frame)),
        (Frame::Bpc32(frame), BitsPerChannel::Bpc8) => Frame::Bpc8(convert_32_to_8(frame)),
        (Frame::Bpc32(frame), BitsPerChannel::Bpc16) => Frame::Bpc16(convert_32_to_16(frame)),
    };
    *output = converted;
}

pub fn convert_8_to_16(frame: &Frame8) -> Frame16 {
    Frame16 {
        width: frame.width,
        height: frame.height,
        channels: frame.channels,
        samples: frame.samples.iter().copied().map(channel_8_to_16).collect(),
    }
}

pub fn convert_16_to_32(frame: &Frame16) -> Frame32 {
    Frame32 {
        width: frame.width,
        height: frame.height,
        channels: frame.channels,
        samples: frame
            .samples
            .iter()
            .copied()
            .map(channel_16_to_32)
            .collect(),
    }
}

pub fn convert_32_to_8(frame: &Frame32) -> Frame8 {
    Frame8 {
        width: frame.width,
        height: frame.height,
        channels: frame.channels,
        samples: frame.samples.iter().copied().map(channel_32_to_8).collect(),
    }
}

fn convert_8_to_32(frame: &Frame8) -> Frame32 {
    convert_16_to_32(&convert_8_to_16(frame))
}

fn convert_16_to_8(frame: &Frame16) -> Frame8 {
    Frame8 {
        width: frame.width,
        height: frame.height,
        channels: frame.channels,
        samples: frame.samples.iter().copied().map(channel_16_to_8).collect(),
    }
}

fn convert_32_to_16(frame: &Frame32) -> Frame16 {
    Frame16 {
        width: frame.width,
        height: frame.height,
        channels: frame.channels,
        samples: frame
            .samples
            .iter()
            .copied()
            .map(channel_32_to_16)
            .collect(),
    }
}

fn sample_len(width: u32, height: u32, channels: u8) -> usize {
    let channels = usize::from(channels);
    usize::try_from(width)
        .ok()
        .zip(usize::try_from(height).ok())
        .and_then(|(width, height)| width.checked_mul(height))
        .and_then(|pixels| pixels.checked_mul(channels))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::{channel_16_to_32, channel_32_to_8, channel_8_to_16};

    #[test]
    fn white_and_black_map_across_depths() {
        assert_eq!(channel_8_to_16(255), 32768);
        assert_eq!(channel_8_to_16(0), 0);
        assert_eq!(channel_16_to_32(32768), 1.0);
        assert_eq!(channel_32_to_8(1.0), 255);
        assert_eq!(channel_32_to_8(2.0), 255);
        assert_eq!(channel_32_to_8(0.0), 0);
    }
}
