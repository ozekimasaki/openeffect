//! CPU の画素バッファ。深度の白は effect-api の変換に従う。

use effect_api::{
    channel_16_to_32, channel_32_to_8, channel_8_to_16, ApplyError, ApplyResult, Frame, Frame16,
    Frame32, Frame8,
};
use model::BitsPerChannel;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge {
    Zero,
    Repeat,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axes {
    Horizontal,
    Vertical,
    Both,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub channels: u8,
    pub samples: Vec<f32>,
}

impl Image {
    pub fn new(width: u32, height: u32, channels: u8, fill: f32) -> ApplyResult<Self> {
        let len = sample_len(width, height, channels)?;
        Ok(Self {
            width,
            height,
            channels,
            samples: vec![fill; len],
        })
    }

    pub fn from_frame(frame: &Frame) -> ApplyResult<Self> {
        let width = frame.width();
        let height = frame.height();
        let channels = frame.channels();
        let len = sample_len(width, height, channels)?;
        let samples = match frame {
            Frame::Bpc8(frame) => {
                check_len(frame.samples.len(), len)?;
                frame.samples.iter().copied().map(channel_8_to_32).collect()
            }
            Frame::Bpc16(frame) => {
                check_len(frame.samples.len(), len)?;
                frame
                    .samples
                    .iter()
                    .copied()
                    .map(channel_16_to_32)
                    .collect()
            }
            Frame::Bpc32(frame) => {
                check_len(frame.samples.len(), len)?;
                frame.samples.clone()
            }
        };
        Ok(Self {
            width,
            height,
            channels,
            samples,
        })
    }

    pub fn to_frame(&self, bits: BitsPerChannel) -> Frame {
        match bits {
            BitsPerChannel::Bpc8 => Frame::Bpc8(Frame8 {
                width: self.width,
                height: self.height,
                channels: self.channels,
                samples: self.samples.iter().copied().map(channel_32_to_8).collect(),
            }),
            BitsPerChannel::Bpc16 => Frame::Bpc16(Frame16 {
                width: self.width,
                height: self.height,
                channels: self.channels,
                samples: self.samples.iter().copied().map(channel_32_to_16).collect(),
            }),
            BitsPerChannel::Bpc32 => Frame::Bpc32(Frame32 {
                width: self.width,
                height: self.height,
                channels: self.channels,
                samples: self.samples.clone(),
            }),
        }
    }

    pub fn pixel_count(&self) -> usize {
        self.width as usize * self.height as usize
    }

    pub fn get(&self, x: u32, y: u32) -> [f32; 4] {
        self.load(self.offset(x, y))
    }

    pub fn set(&mut self, x: u32, y: u32, pixel: [f32; 4]) {
        let offset = self.offset(x, y);
        self.store(offset, pixel);
    }

    pub fn offset(&self, x: u32, y: u32) -> usize {
        ((y as usize * self.width as usize) + x as usize) * self.channels as usize
    }

    fn load(&self, offset: usize) -> [f32; 4] {
        let mut pixel = [0.0; 4];
        for channel in 0..self.channels as usize {
            pixel[channel] = self.samples[offset + channel];
        }
        pixel
    }

    fn store(&mut self, offset: usize, pixel: [f32; 4]) {
        for channel in 0..self.channels as usize {
            self.samples[offset + channel] = pixel[channel];
        }
    }

    pub fn sample(&self, x: i32, y: i32, edge: Edge) -> [f32; 4] {
        let (x, y, inside) = map_coord(x, y, self.width, self.height, edge);
        if !inside {
            return [0.0; 4];
        }
        self.get(x, y)
    }

    pub fn sample_bilinear(&self, x: f32, y: f32, edge: Edge) -> [f32; 4] {
        let x0 = x.floor() as i32;
        let y0 = y.floor() as i32;
        let tx = x - x0 as f32;
        let ty = y - y0 as f32;
        let c00 = self.sample(x0, y0, edge);
        let c10 = self.sample(x0 + 1, y0, edge);
        let c01 = self.sample(x0, y0 + 1, edge);
        let c11 = self.sample(x0 + 1, y0 + 1, edge);
        let mut out = [0.0; 4];
        for channel in 0..4 {
            let a = c00[channel] + (c10[channel] - c00[channel]) * tx;
            let b = c01[channel] + (c11[channel] - c01[channel]) * tx;
            out[channel] = a + (b - a) * ty;
        }
        out
    }

    pub fn map(&self, mut f: impl FnMut(u32, u32, [f32; 4]) -> [f32; 4]) -> Self {
        let mut out = self.clone();
        for y in 0..self.height {
            for x in 0..self.width {
                let pixel = f(x, y, self.get(x, y));
                out.set(x, y, pixel);
            }
        }
        out
    }
}

fn map_coord(x: i32, y: i32, width: u32, height: u32, edge: Edge) -> (u32, u32, bool) {
    let w = width as i32;
    let h = height as i32;
    if w <= 0 || h <= 0 {
        return (0, 0, false);
    }
    let inside = x >= 0 && y >= 0 && x < w && y < h;
    match edge {
        Edge::Zero if !inside => (0, 0, false),
        Edge::Zero | Edge::Repeat => {
            let x = x.clamp(0, w - 1) as u32;
            let y = y.clamp(0, h - 1) as u32;
            (x, y, true)
        }
    }
}

pub fn sample_len(width: u32, height: u32, channels: u8) -> ApplyResult<usize> {
    let channels = usize::from(channels);
    usize::try_from(width)
        .ok()
        .zip(usize::try_from(height).ok())
        .and_then(|(width, height)| width.checked_mul(height))
        .and_then(|pixels| pixels.checked_mul(channels))
        .ok_or_else(|| ApplyError::new("フレームが大きすぎる"))
}

fn check_len(actual: usize, expected: usize) -> ApplyResult<()> {
    if actual == expected {
        Ok(())
    } else {
        Err(ApplyError::new("フレームのサンプル数が一致しない"))
    }
}

pub(crate) fn channel_8_to_32(value: u8) -> f32 {
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

pub fn write_frame(image: &Image, bits: BitsPerChannel, output: &mut Frame) {
    *output = image.to_frame(bits);
}

pub fn radius_px(radius: f32) -> ApplyResult<u32> {
    if !radius.is_finite() || radius < 0.0 {
        return Err(ApplyError::new("半径が負"));
    }
    if radius > 4096.0 {
        return Err(ApplyError::new("半径が処理上限を超える"));
    }
    Ok(radius.floor() as u32)
}

pub fn box_blur(src: &Image, radius: f32, edge: Edge) -> ApplyResult<Image> {
    let radius = radius_px(radius)?;
    if radius == 0 {
        return Ok(src.clone());
    }
    let kernel = box_kernel(radius);
    convolve_separable(src, &kernel, edge)
}

pub fn gaussian_blur(src: &Image, radius: f32, edge: Edge) -> ApplyResult<Image> {
    if !radius.is_finite() || radius < 0.0 {
        return Err(ApplyError::new("半径が負"));
    }
    if radius == 0.0 {
        return Ok(src.clone());
    }
    if radius > 4096.0 {
        return Err(ApplyError::new("半径が処理上限を超える"));
    }
    let kernel = gaussian_kernel(radius);
    convolve_separable(src, &kernel, edge)
}

pub fn box_kernel(radius: u32) -> Vec<f32> {
    let width = radius as usize * 2 + 1;
    vec![1.0 / width as f32; width]
}

pub fn gaussian_kernel(sigma: f32) -> Vec<f32> {
    let radius = (sigma * 3.0).ceil().max(1.0) as i32;
    let mut kernel = Vec::with_capacity((radius * 2 + 1) as usize);
    let mut sum = 0.0;
    for offset in -radius..=radius {
        let x = offset as f32;
        let weight = (-0.5 * (x / sigma) * (x / sigma)).exp();
        kernel.push(weight);
        sum += weight;
    }
    if sum > 0.0 {
        for weight in &mut kernel {
            *weight /= sum;
        }
    }
    kernel
}

pub fn convolve_separable(src: &Image, kernel: &[f32], edge: Edge) -> ApplyResult<Image> {
    if kernel.is_empty() || kernel.len() % 2 == 0 {
        return Err(ApplyError::new("カーネル長が不正"));
    }
    let horizontal = convolve_axis(src, kernel, true, edge);
    Ok(convolve_axis(&horizontal, kernel, false, edge))
}

pub fn convolve_axis(src: &Image, kernel: &[f32], horizontal: bool, edge: Edge) -> Image {
    let mut out = src.clone();
    let radius = (kernel.len() / 2) as i32;
    for y in 0..src.height as i32 {
        for x in 0..src.width as i32 {
            let mut acc = [0.0; 4];
            for (index, weight) in kernel.iter().copied().enumerate() {
                let offset = index as i32 - radius;
                let (sx, sy) = if horizontal {
                    (x + offset, y)
                } else {
                    (x, y + offset)
                };
                let sample = src.sample(sx, sy, edge);
                for channel in 0..src.channels as usize {
                    acc[channel] += sample[channel] * weight;
                }
            }
            out.set(x as u32, y as u32, acc);
        }
    }
    out
}

pub fn blur_channels(src: &Image, radii: [f32; 4], edge: Edge) -> ApplyResult<Image> {
    let mut out = src.clone();
    for channel in 0..src.channels as usize {
        let radius = radii[channel];
        if radius == 0.0 {
            continue;
        }
        let blurred = gaussian_blur(src, radius, edge)?;
        for y in 0..src.height {
            for x in 0..src.width {
                let mut pixel = out.get(x, y);
                pixel[channel] = blurred.get(x, y)[channel];
                out.set(x, y, pixel);
            }
        }
    }
    Ok(out)
}

pub fn directional_blur(
    src: &Image,
    angle_deg: f32,
    length: f32,
    edge: Edge,
) -> ApplyResult<Image> {
    if !angle_deg.is_finite() || !length.is_finite() || length < 0.0 {
        return Err(ApplyError::new("方向ブラーの値が不正"));
    }
    if length == 0.0 {
        return Ok(src.clone());
    }
    let steps = (length.ceil() as i32).clamp(1, 256);
    let radians = angle_deg.to_radians();
    let dx = radians.cos();
    let dy = radians.sin();
    let mut out = src.clone();
    for y in 0..src.height {
        for x in 0..src.width {
            let mut acc = [0.0; 4];
            let mut count = 0.0;
            for step in -steps..=steps {
                let t = step as f32 / steps as f32;
                let sx = x as f32 + dx * length * 0.5 * t;
                let sy = y as f32 + dy * length * 0.5 * t;
                let sample = src.sample_bilinear(sx, sy, edge);
                for channel in 0..src.channels as usize {
                    acc[channel] += sample[channel];
                }
                count += 1.0;
            }
            for channel in 0..src.channels as usize {
                acc[channel] /= count;
            }
            out.set(x, y, acc);
        }
    }
    Ok(out)
}

pub fn zoom_blur(
    src: &Image,
    center: [f32; 2],
    amount: f32,
    samples: u32,
    centered: bool,
    fade: bool,
    edge: Edge,
) -> ApplyResult<Image> {
    let samples = samples.max(2).min(64);
    Ok(src.map(|x, y, _| {
        let mut acc = [0.0; 4];
        let mut weight_sum = 0.0;
        let px = x as f32;
        let py = y as f32;
        let dist = ((px - center[0]).powi(2) + (py - center[1]).powi(2)).sqrt();
        let fade_w = if fade {
            (dist / (src.width.max(src.height) as f32)).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let directions: &[f32] = if centered { &[1.0, -1.0] } else { &[1.0] };
        for direction in directions {
            for step in 0..samples {
                let u = step as f32 / (samples - 1) as f32;
                let scale = 1.0 + amount * fade_w * u * direction;
                let sx = center[0] + (px - center[0]) * scale;
                let sy = center[1] + (py - center[1]) * scale;
                let sample = src.sample_bilinear(sx, sy, edge);
                let weight = 1.0;
                for channel in 0..src.channels as usize {
                    acc[channel] += sample[channel] * weight;
                }
                weight_sum += weight;
            }
        }
        for channel in 0..src.channels as usize {
            acc[channel] /= weight_sum;
        }
        acc
    }))
}

pub fn spin_blur(
    src: &Image,
    center: [f32; 2],
    angle_deg: f32,
    samples: u32,
    fade: bool,
    edge: Edge,
) -> ApplyResult<Image> {
    let samples = samples.max(2).min(128);
    let angle = angle_deg.to_radians();
    Ok(src.map(|x, y, pixel| {
        let dx = x as f32 - center[0];
        let dy = y as f32 - center[1];
        let dist = (dx * dx + dy * dy).sqrt();
        if dist < 0.5 {
            return pixel;
        }
        let fade_w = if fade {
            (dist / (src.width.max(src.height) as f32)).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let mut acc = [0.0; 4];
        for step in 0..samples {
            let u = step as f32 / (samples - 1) as f32;
            let theta = angle * fade_w * u;
            let (sin, cos) = theta.sin_cos();
            let sx = center[0] + dx * cos - dy * sin;
            let sy = center[1] + dx * sin + dy * cos;
            let sample = src.sample_bilinear(sx, sy, edge);
            for channel in 0..src.channels as usize {
                acc[channel] += sample[channel];
            }
        }
        for channel in 0..src.channels as usize {
            acc[channel] /= samples as f32;
        }
        acc
    }))
}

pub fn extremum(src: &Image, radius: i32, axes: Axes, maximum: bool) -> Image {
    let radius = radius.max(0);
    src.map(|x, y, _| {
        let mut best = if maximum {
            [f32::MIN; 4]
        } else {
            [f32::MAX; 4]
        };
        let x = x as i32;
        let y = y as i32;
        for oy in -radius..=radius {
            for ox in -radius..=radius {
                let use_x = matches!(axes, Axes::Horizontal | Axes::Both) || ox == 0;
                let use_y = matches!(axes, Axes::Vertical | Axes::Both) || oy == 0;
                if !use_x || !use_y {
                    continue;
                }
                if !matches!(axes, Axes::Both) && ox != 0 && oy != 0 {
                    continue;
                }
                let sample = src.sample(x + ox, y + oy, Edge::Zero);
                for channel in 0..src.channels as usize {
                    best[channel] = if maximum {
                        best[channel].max(sample[channel])
                    } else {
                        best[channel].min(sample[channel])
                    };
                }
            }
        }
        best
    })
}

pub fn bilateral(src: &Image, radius: f32, range: f32, colorize: bool) -> ApplyResult<Image> {
    let radius_i = radius_px(radius)? as i32;
    if radius_i == 0 {
        return Ok(if colorize {
            src.clone()
        } else {
            to_luma_image(src)
        });
    }
    let spatial = (radius.max(1.0e-3)).powi(2);
    Ok(src.map(|x, y, center| {
        let center_key = if colorize {
            center
        } else {
            let y = crate::color_math::luma(center[0], center[1], center[2]);
            [y, y, y, center[3]]
        };
        let mut acc = [0.0; 4];
        let mut weight_sum = 0.0;
        for oy in -radius_i..=radius_i {
            for ox in -radius_i..=radius_i {
                let sample = src.sample(x as i32 + ox, y as i32 + oy, Edge::Repeat);
                let key = if colorize {
                    sample
                } else {
                    let y = crate::color_math::luma(sample[0], sample[1], sample[2]);
                    [y, y, y, sample[3]]
                };
                let spatial_w = (-((ox * ox + oy * oy) as f32) / (2.0 * spatial)).exp();
                let range_w = if range <= 0.0 {
                    let different = (0..src.channels as usize)
                        .any(|channel| (key[channel] - center_key[channel]).abs() > 0.0);
                    if different {
                        0.0
                    } else {
                        1.0
                    }
                } else {
                    let mut dist = 0.0;
                    for channel in 0..3.min(src.channels as usize) {
                        let d = key[channel] - center_key[channel];
                        dist += d * d;
                    }
                    (-dist / (2.0 * range * range)).exp()
                };
                let weight = spatial_w * range_w;
                for channel in 0..src.channels as usize {
                    acc[channel] += key[channel] * weight;
                }
                weight_sum += weight;
            }
        }
        if weight_sum > 0.0 {
            for channel in 0..src.channels as usize {
                acc[channel] /= weight_sum;
            }
        }
        if !colorize {
            let y = acc[0];
            acc[0] = y;
            acc[1] = y;
            acc[2] = y;
        }
        acc
    }))
}

pub fn to_luma_image(src: &Image) -> Image {
    src.map(|_, _, pixel| {
        let y = crate::color_math::luma(pixel[0], pixel[1], pixel[2]);
        [y, y, y, pixel[3]]
    })
}

pub fn sharpness(src: &Image) -> f32 {
    if src.width == 0 || src.height == 0 {
        return 0.0;
    }
    let mut sum = 0.0;
    let mut count = 0.0;
    for y in 0..src.height {
        for x in 0..src.width {
            let here =
                crate::color_math::luma(src.get(x, y)[0], src.get(x, y)[1], src.get(x, y)[2]);
            if x + 1 < src.width {
                let next = src.get(x + 1, y);
                let ny = crate::color_math::luma(next[0], next[1], next[2]);
                sum += (here - ny).abs();
                count += 1.0;
            }
            if y + 1 < src.height {
                let next = src.get(x, y + 1);
                let ny = crate::color_math::luma(next[0], next[1], next[2]);
                sum += (here - ny).abs();
                count += 1.0;
            }
        }
    }
    if count == 0.0 {
        0.0
    } else {
        sum / count
    }
}

pub fn mean_abs(a: &Image, b: &Image) -> f32 {
    let mut sum = 0.0;
    let mut count = 0.0;
    let pixels = a.pixel_count().min(b.pixel_count());
    for index in 0..pixels {
        let offset = index * a.channels as usize;
        let other = index * b.channels as usize;
        for channel in 0..a.channels as usize {
            sum += (a.samples[offset + channel] - b.samples[other + channel]).abs();
            count += 1.0;
        }
    }
    if count == 0.0 {
        0.0
    } else {
        sum / count
    }
}

pub fn channel_variance(src: &Image, channel: usize) -> f32 {
    let mut sum = 0.0;
    let mut count = 0.0;
    for index in 0..src.pixel_count() {
        let value = src.samples[index * src.channels as usize + channel];
        sum += value;
        count += 1.0;
    }
    if count == 0.0 {
        return 0.0;
    }
    let mean = sum / count;
    let mut var = 0.0;
    for index in 0..src.pixel_count() {
        let value = src.samples[index * src.channels as usize + channel];
        let d = value - mean;
        var += d * d;
    }
    var / count
}
