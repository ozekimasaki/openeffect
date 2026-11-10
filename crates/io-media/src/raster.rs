use model::BitsPerChannel;

/// 直線の RGB または RGBA。8/16 の白は 1、32 bpc は 1 を超えてよい。
#[derive(Clone, Debug, PartialEq)]
pub struct Raster {
    pub width: u32,
    pub height: u32,
    pub channels: u8,
    pub bits: BitsPerChannel,
    samples: Vec<f32>,
}

impl Raster {
    pub fn new(width: u32, height: u32, channels: u8, bits: BitsPerChannel, samples: Vec<f32>) -> Self {
        let channels = if channels == 4 { 4 } else { 3 };
        debug_assert_eq!(samples.len(), width as usize * height as usize * channels as usize);
        Self {
            width,
            height,
            channels,
            bits,
            samples,
        }
    }

    pub fn solid(width: u32, height: u32, bits: BitsPerChannel, pixel: [f32; 4]) -> Self {
        let mut samples = Vec::with_capacity(width as usize * height as usize * 4);
        for _ in 0..width as usize * height as usize {
            samples.extend_from_slice(&pixel);
        }
        Self::new(width, height, 4, bits, samples)
    }

    pub fn has_alpha(&self) -> bool {
        self.channels == 4
    }

    pub fn samples(&self) -> &[f32] {
        &self.samples
    }

    pub fn get(&self, x: u32, y: u32) -> [f32; 4] {
        let index = self.index(x, y);
        let channels = self.channels as usize;
        let pixel = &self.samples[index..index + channels];
        [
            pixel[0],
            pixel[1],
            pixel[2],
            if channels == 4 { pixel[3] } else { 1.0 },
        ]
    }

    pub fn set(&mut self, x: u32, y: u32, pixel: [f32; 4]) {
        let index = self.index(x, y);
        let channels = self.channels as usize;
        self.samples[index] = pixel[0];
        self.samples[index + 1] = pixel[1];
        self.samples[index + 2] = pixel[2];
        if channels == 4 {
            self.samples[index + 3] = pixel[3];
        }
    }

    pub fn map_pixels(&self, mut map: impl FnMut([f32; 4]) -> [f32; 4]) -> Self {
        let mut out = self.clone();
        for y in 0..self.height {
            for x in 0..self.width {
                out.set(x, y, map(self.get(x, y)));
            }
        }
        out
    }

    fn index(&self, x: u32, y: u32) -> usize {
        (y as usize * self.width as usize + x as usize) * self.channels as usize
    }
}
