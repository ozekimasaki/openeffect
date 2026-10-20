//! ツールが照合と塗りに使う直線 RGBA。1 が白。

use effect_api::Frame8;

#[derive(Clone, Debug, PartialEq)]
pub struct Raster {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<[f32; 4]>,
}

impl Raster {
    pub fn new(width: u32, height: u32, fill: [f32; 4]) -> Self {
        let len = (width as usize) * (height as usize);
        Self {
            width,
            height,
            pixels: vec![fill; len],
        }
    }

    pub fn filled(width: u32, height: u32, color: [f32; 4]) -> Self {
        Self::new(width, height, color)
    }

    fn index(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            None
        } else {
            Some(y as usize * self.width as usize + x as usize)
        }
    }

    pub fn get(&self, x: i32, y: i32) -> [f32; 4] {
        self.index(x, y)
            .map(|index| self.pixels[index])
            .unwrap_or([0.0, 0.0, 0.0, 0.0])
    }

    pub fn set(&mut self, x: i32, y: i32, pixel: [f32; 4]) {
        if let Some(index) = self.index(x, y) {
            self.pixels[index] = pixel;
        }
    }

    pub fn sample(&self, x: f64, y: f64) -> [f32; 4] {
        let x0 = x.floor() as i32;
        let y0 = y.floor() as i32;
        let tx = (x - f64::from(x0)) as f32;
        let ty = (y - f64::from(y0)) as f32;
        let a = self.get(x0, y0);
        let b = self.get(x0 + 1, y0);
        let c = self.get(x0, y0 + 1);
        let d = self.get(x0 + 1, y0 + 1);
        let mut out = [0.0; 4];
        for channel in 0..4 {
            let top = a[channel] * (1.0 - tx) + b[channel] * tx;
            let bottom = c[channel] * (1.0 - tx) + d[channel] * tx;
            out[channel] = top * (1.0 - ty) + bottom * ty;
        }
        out
    }

    pub fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: [f32; 4]) {
        for py in y..(y + h) {
            for px in x..(x + w) {
                self.set(px, py, color);
            }
        }
    }

    pub fn stamp_disc(&mut self, cx: f64, cy: f64, radius: f64, color: [f32; 4]) {
        let r = radius.ceil() as i32 + 1;
        let x0 = cx.floor() as i32;
        let y0 = cy.floor() as i32;
        for y in (y0 - r)..=(y0 + r) {
            for x in (x0 - r)..=(x0 + r) {
                let dx = f64::from(x) + 0.5 - cx;
                let dy = f64::from(y) + 0.5 - cy;
                if dx * dx + dy * dy <= radius * radius {
                    self.set(x, y, color);
                }
            }
        }
    }

    pub fn shift(&self, dx: f64, dy: f64) -> Self {
        let mut out = Self::new(self.width, self.height, [0.0, 0.0, 0.0, 0.0]);
        for y in 0..self.height as i32 {
            for x in 0..self.width as i32 {
                let pixel = self.sample(f64::from(x) - dx, f64::from(y) - dy);
                out.set(x, y, pixel);
            }
        }
        out
    }

    pub fn rotate(&self, degrees: f64, origin: [f64; 2]) -> Self {
        let radians = degrees.to_radians();
        let (s, c) = radians.sin_cos();
        let mut out = Self::new(self.width, self.height, [0.0, 0.0, 0.0, 0.0]);
        for y in 0..self.height as i32 {
            for x in 0..self.width as i32 {
                let px = f64::from(x) - origin[0];
                let py = f64::from(y) - origin[1];
                let sx = c * px + s * py + origin[0];
                let sy = -s * px + c * py + origin[1];
                out.set(x, y, self.sample(sx, sy));
            }
        }
        out
    }

    pub fn scale_about(&self, factor: f64, origin: [f64; 2]) -> Self {
        let mut out = Self::new(self.width, self.height, [0.0, 0.0, 0.0, 0.0]);
        for y in 0..self.height as i32 {
            for x in 0..self.width as i32 {
                let sx = (f64::from(x) - origin[0]) / factor + origin[0];
                let sy = (f64::from(y) - origin[1]) / factor + origin[1];
                out.set(x, y, self.sample(sx, sy));
            }
        }
        out
    }

    pub fn blur(&self, radius: i32) -> Self {
        if radius <= 0 {
            return self.clone();
        }
        let mut out = Self::new(self.width, self.height, [0.0, 0.0, 0.0, 0.0]);
        let window = (radius * 2 + 1) as f32;
        let weight = window * window;
        for y in 0..self.height as i32 {
            for x in 0..self.width as i32 {
                let mut sum = [0.0; 4];
                for oy in -radius..=radius {
                    for ox in -radius..=radius {
                        let pixel = self.get(x + ox, y + oy);
                        for channel in 0..4 {
                            sum[channel] += pixel[channel];
                        }
                    }
                }
                for channel in 0..4 {
                    sum[channel] /= weight;
                }
                out.set(x, y, sum);
            }
        }
        out
    }

    pub fn sharpen(&self, amount: f32) -> Self {
        let blur = self.blur(1);
        let mut out = self.clone();
        for (pixel, blurred) in out.pixels.iter_mut().zip(blur.pixels.iter()) {
            for channel in 0..3 {
                pixel[channel] =
                    (pixel[channel] + (pixel[channel] - blurred[channel]) * amount).clamp(0.0, 1.0);
            }
        }
        out
    }

    pub fn to_frame8(&self) -> Frame8 {
        let mut frame = Frame8::new(self.width, self.height, 4, 0);
        for (index, pixel) in self.pixels.iter().enumerate() {
            for channel in 0..4 {
                let value = (pixel[channel].clamp(0.0, 1.0) * 255.0).round() as u8;
                frame.samples[index * 4 + channel] = value;
            }
        }
        frame
    }
}

pub fn luma(pixel: [f32; 4]) -> f32 {
    0.2126 * pixel[0] + 0.7152 * pixel[1] + 0.0722 * pixel[2]
}

pub fn saturation(pixel: [f32; 4]) -> f32 {
    let max = pixel[0].max(pixel[1]).max(pixel[2]);
    let min = pixel[0].min(pixel[1]).min(pixel[2]);
    max - min
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompareChannel {
    Rgb,
    Luminance,
    Saturation,
}

pub fn channel_sample(pixel: [f32; 4], channel: CompareChannel) -> [f32; 3] {
    match channel {
        CompareChannel::Rgb => [pixel[0], pixel[1], pixel[2]],
        CompareChannel::Luminance => {
            let y = luma(pixel);
            [y, 0.0, 0.0]
        }
        CompareChannel::Saturation => {
            let s = saturation(pixel);
            [s, 0.0, 0.0]
        }
    }
}

pub fn channel_count(channel: CompareChannel) -> usize {
    match channel {
        CompareChannel::Rgb => 3,
        CompareChannel::Luminance | CompareChannel::Saturation => 1,
    }
}

/// 非対称な印。平行移動の照合が別の印と混ざりにくい。
#[cfg(test)]
pub fn paint_marker(raster: &mut Raster, cx: i32, cy: i32, color: [f32; 4]) {
    raster.fill_rect(cx - 3, cy - 1, 7, 3, color);
    raster.fill_rect(cx - 1, cy - 3, 3, 7, color);
    raster.set(cx + 2, cy - 3, color);
    raster.set(cx + 3, cy - 3, color);
    raster.set(
        cx - 3,
        cy + 2,
        [color[0] * 0.4, color[1] * 0.4, color[2] * 0.4, color[3]],
    );
}

#[derive(Clone, Debug)]
pub struct FrameClip {
    pub frames: Vec<Raster>,
    pub frame_rate: f64,
    pub in_frame: i32,
    pub out_frame: i32,
}

impl FrameClip {
    pub fn from_frames(frames: Vec<Raster>, frame_rate: f64) -> Self {
        let last = frames.len().saturating_sub(1) as i32;
        Self {
            frames,
            frame_rate,
            in_frame: 0,
            out_frame: last,
        }
    }

    pub fn frame(&self, index: i32) -> Option<&Raster> {
        self.frames.get(index as usize)
    }
}
