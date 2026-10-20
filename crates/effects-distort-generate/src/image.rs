use effect_api::{
    channel_16_to_32, channel_32_to_8, channel_8_to_16, Frame, Frame16, Frame32, Frame8,
};
use model::{BitsPerChannel, BlendMode, Rgba};

/// 作業用の直線 RGBA。32 bpc では 1 を超えてよい。
#[derive(Clone, Debug)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub channels: u8,
    pub bits: BitsPerChannel,
    pub px: Vec<[f32; 4]>,
}

impl Image {
    pub fn new(
        width: u32,
        height: u32,
        channels: u8,
        bits: BitsPerChannel,
        fill: [f32; 4],
    ) -> Self {
        let n = (width as usize) * (height as usize);
        Self {
            width,
            height,
            channels,
            bits,
            px: vec![fill; n],
        }
    }

    pub fn from_frame(frame: &Frame) -> Self {
        let bits = frame.bits();
        let width = frame.width();
        let height = frame.height();
        let channels = frame.channels();
        let mut px = Vec::with_capacity((width as usize) * (height as usize));
        match frame {
            Frame::Bpc8(frame) => push_samples(&mut px, &frame.samples, channels, |v| {
                channel_16_to_32(channel_8_to_16(v))
            }),
            Frame::Bpc16(frame) => {
                push_samples(&mut px, &frame.samples, channels, channel_16_to_32)
            }
            Frame::Bpc32(frame) => push_samples(&mut px, &frame.samples, channels, |v| v),
        }
        Self {
            width,
            height,
            channels,
            bits,
            px,
        }
    }

    pub fn to_frame(&self) -> Frame {
        let channels = self.channels.max(1);
        match self.bits {
            BitsPerChannel::Bpc8 => {
                let mut samples = Vec::with_capacity(self.px.len() * channels as usize);
                for px in &self.px {
                    push_out(&mut samples, *px, channels, channel_32_to_8);
                }
                Frame::Bpc8(Frame8 {
                    width: self.width,
                    height: self.height,
                    channels,
                    samples,
                })
            }
            BitsPerChannel::Bpc16 => {
                let mut samples = Vec::with_capacity(self.px.len() * channels as usize);
                for px in &self.px {
                    push_out(&mut samples, *px, channels, linear_to_16);
                }
                Frame::Bpc16(Frame16 {
                    width: self.width,
                    height: self.height,
                    channels,
                    samples,
                })
            }
            BitsPerChannel::Bpc32 => {
                let mut samples = Vec::with_capacity(self.px.len() * channels as usize);
                for px in &self.px {
                    push_out(&mut samples, *px, channels, |v| v);
                }
                Frame::Bpc32(Frame32 {
                    width: self.width,
                    height: self.height,
                    channels,
                    samples,
                })
            }
        }
    }

    pub fn len(&self) -> usize {
        self.px.len()
    }

    pub fn index(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return None;
        }
        Some((y as usize) * (self.width as usize) + (x as usize))
    }

    pub fn get(&self, x: i32, y: i32) -> [f32; 4] {
        self.index(x, y).map(|i| self.px[i]).unwrap_or([0.0; 4])
    }

    pub fn set(&mut self, x: i32, y: i32, px: [f32; 4]) {
        if let Some(i) = self.index(x, y) {
            self.px[i] = px;
        }
    }

    pub fn at(&self, x: u32, y: u32) -> [f32; 4] {
        self.get(x as i32, y as i32)
    }

    /// 画素中心は整数 + 0.5。
    pub fn sample(&self, x: f32, y: f32, filter: Filter, wrap: bool) -> [f32; 4] {
        match filter {
            Filter::Nearest => {
                let sx = x.floor() as i32;
                let sy = y.floor() as i32;
                self.fetch(sx, sy, wrap)
            }
            Filter::Bilinear => self.bilinear(x, y, wrap),
            Filter::Bicubic => self.bicubic(x, y, wrap),
        }
    }

    fn fetch(&self, x: i32, y: i32, wrap: bool) -> [f32; 4] {
        let (x, y) = if wrap {
            (wrap_i(x, self.width), wrap_i(y, self.height))
        } else if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return [0.0; 4];
        } else {
            (x, y)
        };
        self.get(x, y)
    }

    fn bilinear(&self, x: f32, y: f32, wrap: bool) -> [f32; 4] {
        let x = x - 0.5;
        let y = y - 0.5;
        let x0 = x.floor() as i32;
        let y0 = y.floor() as i32;
        let tx = x - x0 as f32;
        let ty = y - y0 as f32;
        let a = self.fetch(x0, y0, wrap);
        let b = self.fetch(x0 + 1, y0, wrap);
        let c = self.fetch(x0, y0 + 1, wrap);
        let d = self.fetch(x0 + 1, y0 + 1, wrap);
        lerp4(lerp4(a, b, tx), lerp4(c, d, tx), ty)
    }

    fn bicubic(&self, x: f32, y: f32, wrap: bool) -> [f32; 4] {
        let x = x - 0.5;
        let y = y - 0.5;
        let x0 = x.floor() as i32;
        let y0 = y.floor() as i32;
        let tx = x - x0 as f32;
        let ty = y - y0 as f32;
        let mut rows = [[0.0; 4]; 4];
        for (j, row) in rows.iter_mut().enumerate() {
            let y = y0 - 1 + j as i32;
            let p0 = self.fetch(x0 - 1, y, wrap);
            let p1 = self.fetch(x0, y, wrap);
            let p2 = self.fetch(x0 + 1, y, wrap);
            let p3 = self.fetch(x0 + 2, y, wrap);
            *row = catmull(p0, p1, p2, p3, tx);
        }
        catmull(rows[0], rows[1], rows[2], rows[3], ty)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Filter {
    Nearest,
    Bilinear,
    Bicubic,
}

pub fn rgba(color: Rgba) -> [f32; 4] {
    [color.r, color.g, color.b, color.a]
}

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

pub fn lerp4(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    [
        lerp(a[0], b[0], t),
        lerp(a[1], b[1], t),
        lerp(a[2], b[2], t),
        lerp(a[3], b[3], t),
    ]
}

pub fn mix(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    lerp4(a, b, t.clamp(0.0, 1.0))
}

fn catmull(p0: [f32; 4], p1: [f32; 4], p2: [f32; 4], p3: [f32; 4], t: f32) -> [f32; 4] {
    let mut out = [0.0; 4];
    for i in 0..4 {
        out[i] = catmull_ch(p0[i], p1[i], p2[i], p3[i], t);
    }
    out
}

fn catmull_ch(p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
    let t2 = t * t;
    let t3 = t2 * t;
    0.5 * ((2.0 * p1)
        + (-p0 + p2) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
        + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
}

fn wrap_i(v: i32, size: u32) -> i32 {
    let size = size as i32;
    if size <= 0 {
        return 0;
    }
    let mut v = v % size;
    if v < 0 {
        v += size;
    }
    v
}

fn push_samples<T: Copy>(
    px: &mut Vec<[f32; 4]>,
    samples: &[T],
    channels: u8,
    map: impl Fn(T) -> f32,
) {
    let channels = channels.max(1) as usize;
    if channels == 0 {
        return;
    }
    let pixels = samples.len() / channels;
    for i in 0..pixels {
        let base = i * channels;
        let mut out = [0.0; 4];
        for c in 0..channels.min(4) {
            out[c] = map(samples[base + c]);
        }
        if channels == 1 {
            out[1] = out[0];
            out[2] = out[0];
            out[3] = 1.0;
        } else if channels == 3 {
            out[3] = 1.0;
        }
        px.push(out);
    }
}

fn linear_to_16(value: f32) -> u16 {
    if !value.is_finite() || value <= 0.0 {
        0
    } else if value >= 1.0 {
        32768
    } else {
        (value * 32768.0).round() as u16
    }
}

fn push_out<T>(samples: &mut Vec<T>, px: [f32; 4], channels: u8, map: impl Fn(f32) -> T) {
    let n = channels.max(1) as usize;
    for c in 0..n {
        let v = if c < 4 { px[c] } else { 0.0 };
        samples.push(map(v));
    }
}

/// タイムラインと同じ合成。`None` は効果だけを残す。
pub fn composite(mode: Option<BlendMode>, fg: [f32; 4], bg: [f32; 4]) -> [f32; 4] {
    match mode {
        None => fg,
        Some(mode) => {
            let fg_a = fg[3].clamp(0.0, 1.0);
            let rgb = blend_rgb(mode, fg, bg);
            let out_a = match mode {
                BlendMode::StencilAlpha | BlendMode::SilhouetteAlpha => fg_a * bg[3],
                BlendMode::StencilLuma | BlendMode::SilhouetteLuma => luma(fg) * bg[3],
                _ => fg_a + bg[3] * (1.0 - fg_a),
            };
            if out_a <= f32::EPSILON {
                return [0.0; 4];
            }
            let premul = |c: f32| (c * fg_a + bg_channel(mode, bg, c) * (1.0 - fg_a)) / out_a;
            match mode {
                BlendMode::Normal => [premul(fg[0]), premul(fg[1]), premul(fg[2]), out_a],
                BlendMode::StencilAlpha | BlendMode::SilhouetteAlpha => {
                    [bg[0], bg[1], bg[2], out_a]
                }
                BlendMode::StencilLuma | BlendMode::SilhouetteLuma => [bg[0], bg[1], bg[2], out_a],
                _ => {
                    let a = out_a.max(fg_a).max(bg[3]);
                    [rgb[0], rgb[1], rgb[2], a.clamp(0.0, 1.0)]
                }
            }
        }
    }
}

fn bg_channel(mode: BlendMode, bg: [f32; 4], fg_c: f32) -> f32 {
    match mode {
        BlendMode::Normal => fg_c,
        _ => fg_c,
    }
}

fn blend_rgb(mode: BlendMode, fg: [f32; 4], bg: [f32; 4]) -> [f32; 3] {
    match mode {
        BlendMode::Hue => hsl_set(bg, fg, true, false, false),
        BlendMode::Saturation => hsl_set(bg, fg, false, true, false),
        BlendMode::Color => hsl_set(bg, fg, true, true, false),
        BlendMode::Luminosity => hsl_set(bg, fg, false, false, true),
        _ => {
            let mut out = [0.0; 3];
            for i in 0..3 {
                out[i] = blend_ch(mode, fg[i], bg[i], fg, bg);
            }
            out
        }
    }
}

fn blend_ch(mode: BlendMode, fg: f32, bg: f32, fg4: [f32; 4], bg4: [f32; 4]) -> f32 {
    match mode {
        BlendMode::Normal | BlendMode::Dissolve | BlendMode::DancingDissolve => fg,
        BlendMode::Darken => fg.min(bg),
        BlendMode::Multiply => fg * bg,
        BlendMode::ColorBurn | BlendMode::ClassicColorBurn => {
            if fg <= 0.0 {
                0.0
            } else {
                1.0 - (1.0 - bg) / fg
            }
        }
        BlendMode::LinearBurn => fg + bg - 1.0,
        BlendMode::DarkerColor => {
            if luma(fg4) < luma(bg4) {
                fg
            } else {
                bg
            }
        }
        BlendMode::Add | BlendMode::LinearDodge => fg + bg,
        BlendMode::Lighten => fg.max(bg),
        BlendMode::Screen => 1.0 - (1.0 - fg) * (1.0 - bg),
        BlendMode::ColorDodge | BlendMode::ClassicColorDodge => {
            if fg >= 1.0 {
                1.0
            } else {
                bg / (1.0 - fg)
            }
        }
        BlendMode::LighterColor => {
            if luma(fg4) > luma(bg4) {
                fg
            } else {
                bg
            }
        }
        BlendMode::Overlay => overlay(bg, fg),
        BlendMode::SoftLight => soft_light(bg, fg),
        BlendMode::HardLight => overlay(fg, bg),
        BlendMode::LinearLight => bg + 2.0 * fg - 1.0,
        BlendMode::VividLight => {
            if fg < 0.5 {
                blend_ch(BlendMode::ColorBurn, fg * 2.0, bg, fg4, bg4)
            } else {
                blend_ch(BlendMode::ColorDodge, 2.0 * fg - 1.0, bg, fg4, bg4)
            }
        }
        BlendMode::PinLight => {
            if fg < 0.5 {
                bg.min(2.0 * fg)
            } else {
                bg.max(2.0 * fg - 1.0)
            }
        }
        BlendMode::HardMix => {
            if blend_ch(BlendMode::VividLight, fg, bg, fg4, bg4) < 0.5 {
                0.0
            } else {
                1.0
            }
        }
        BlendMode::Difference | BlendMode::ClassicDifference => (fg - bg).abs(),
        BlendMode::Exclusion => fg + bg - 2.0 * fg * bg,
        BlendMode::Subtract => bg - fg,
        BlendMode::Divide => {
            if fg == 0.0 {
                1.0
            } else {
                bg / fg
            }
        }
        BlendMode::Hue | BlendMode::Saturation | BlendMode::Color | BlendMode::Luminosity => fg,
        BlendMode::AlphaAdd | BlendMode::LuminescentPremul => fg + bg,
        BlendMode::StencilAlpha
        | BlendMode::StencilLuma
        | BlendMode::SilhouetteAlpha
        | BlendMode::SilhouetteLuma => bg,
    }
}

fn overlay(base: f32, blend: f32) -> f32 {
    if base < 0.5 {
        2.0 * base * blend
    } else {
        1.0 - 2.0 * (1.0 - base) * (1.0 - blend)
    }
}

fn soft_light(base: f32, blend: f32) -> f32 {
    if blend < 0.5 {
        base - (1.0 - 2.0 * blend) * base * (1.0 - base)
    } else {
        let d = if base < 0.25 {
            ((16.0 * base - 12.0) * base + 4.0) * base
        } else {
            base.sqrt()
        };
        base + (2.0 * blend - 1.0) * (d - base)
    }
}

pub fn luma(px: [f32; 4]) -> f32 {
    0.2126 * px[0] + 0.7152 * px[1] + 0.0722 * px[2]
}

fn hsl_set(base: [f32; 4], src: [f32; 4], hue: bool, sat: bool, lum: bool) -> [f32; 3] {
    let mut b = rgb_hsl(base);
    let s = rgb_hsl(src);
    if hue {
        b.0 = s.0;
    }
    if sat {
        b.1 = s.1;
    }
    if lum {
        b.2 = s.2;
    }
    hsl_rgb(b.0, b.1, b.2)
}

fn rgb_hsl(px: [f32; 4]) -> (f32, f32, f32) {
    let r = px[0];
    let g = px[1];
    let b = px[2];
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) * 0.5;
    if (max - min).abs() < 1e-6 {
        return (0.0, 0.0, l);
    }
    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };
    let h = if max == r {
        (g - b) / d + if g < b { 6.0 } else { 0.0 }
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    (h / 6.0, s, l)
}

fn hsl_rgb(h: f32, s: f32, l: f32) -> [f32; 3] {
    if s <= 1e-6 {
        return [l, l, l];
    }
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    [
        hue_ch(p, q, h + 1.0 / 3.0),
        hue_ch(p, q, h),
        hue_ch(p, q, h - 1.0 / 3.0),
    ]
}

fn hue_ch(p: f32, q: f32, mut t: f32) -> f32 {
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }
    if t < 1.0 / 6.0 {
        p + (q - p) * 6.0 * t
    } else if t < 0.5 {
        q
    } else if t < 2.0 / 3.0 {
        p + (q - p) * (2.0 / 3.0 - t) * 6.0
    } else {
        p
    }
}

pub fn hash01(x: u32, y: u32, seed: u32) -> f32 {
    let mut n = x
        .wrapping_mul(374761393)
        .wrapping_add(y.wrapping_mul(668265263))
        .wrapping_add(seed.wrapping_mul(1442695041));
    n = (n ^ (n >> 13)).wrapping_mul(1274126177);
    n ^= n >> 16;
    (n & 0x00ff_ffff) as f32 / 16_777_215.0
}

pub fn value_noise(x: f32, y: f32, seed: u32) -> f32 {
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let tx = x - x0 as f32;
    let ty = y - y0 as f32;
    let sx = tx * tx * (3.0 - 2.0 * tx);
    let sy = ty * ty * (3.0 - 2.0 * ty);
    let h = |ix: i32, iy: i32| hash01(ix as u32, iy as u32, seed);
    let a = h(x0, y0);
    let b = h(x0 + 1, y0);
    let c = h(x0, y0 + 1);
    let d = h(x0 + 1, y0 + 1);
    a + (b - a) * sx + (c - a) * sy + (a - b - c + d) * sx * sy
}

pub fn fbm(x: f32, y: f32, octaves: u32, seed: u32) -> f32 {
    let mut amp = 0.5;
    let mut freq = 1.0;
    let mut sum = 0.0;
    let mut norm = 0.0;
    for i in 0..octaves.max(1) {
        sum += value_noise(x * freq, y * freq, seed.wrapping_add(i * 17)) * amp;
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    if norm == 0.0 {
        0.0
    } else {
        sum / norm
    }
}

pub fn dist(ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let dx = ax - bx;
    let dy = ay - by;
    (dx * dx + dy * dy).sqrt()
}

pub fn point_in_poly(x: f32, y: f32, poly: &[[f64; 2]]) -> bool {
    if poly.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let yi = poly[i][1] as f32;
        let yj = poly[j][1] as f32;
        let xi = poly[i][0] as f32;
        let xj = poly[j][0] as f32;
        if ((yi > y) != (yj > y)) && (x < (xj - xi) * (y - yi) / (yj - yi + 1e-12) + xi) {
            inside = !inside;
        }
        j = i;
    }
    inside
}

pub fn dist_to_poly(x: f32, y: f32, poly: &[[f64; 2]]) -> f32 {
    if poly.len() < 2 {
        return f32::MAX;
    }
    let mut best = f32::MAX;
    for i in 0..poly.len() {
        let a = poly[i];
        let b = poly[(i + 1) % poly.len()];
        best = best.min(dist_to_seg(
            x,
            y,
            a[0] as f32,
            a[1] as f32,
            b[0] as f32,
            b[1] as f32,
        ));
    }
    best
}

fn dist_to_seg(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let abx = bx - ax;
    let aby = by - ay;
    let len2 = abx * abx + aby * aby;
    if len2 <= f32::EPSILON {
        return dist(px, py, ax, ay);
    }
    let t = (((px - ax) * abx + (py - ay) * aby) / len2).clamp(0.0, 1.0);
    dist(px, py, ax + abx * t, ay + aby * t)
}

/// 出力側の四角形から入力矩形へ戻す射影。失敗したら None。
pub fn homography(dst: [[f64; 2]; 4], src: [[f64; 2]; 4]) -> Option<[f64; 8]> {
    let mut a = [[0.0; 8]; 8];
    let mut b = [0.0; 8];
    for i in 0..4 {
        let x = dst[i][0];
        let y = dst[i][1];
        let u = src[i][0];
        let v = src[i][1];
        let r = i * 2;
        a[r] = [x, y, 1.0, 0.0, 0.0, 0.0, -u * x, -u * y];
        b[r] = u;
        a[r + 1] = [0.0, 0.0, 0.0, x, y, 1.0, -v * x, -v * y];
        b[r + 1] = v;
    }
    gauss(&mut a, &mut b)
}

pub fn apply_h(h: [f64; 8], x: f64, y: f64) -> (f64, f64) {
    let w = h[6] * x + h[7] * y + 1.0;
    if w.abs() < 1e-12 {
        return (x, y);
    }
    (
        (h[0] * x + h[1] * y + h[2]) / w,
        (h[3] * x + h[4] * y + h[5]) / w,
    )
}

fn gauss(a: &mut [[f64; 8]; 8], b: &mut [f64; 8]) -> Option<[f64; 8]> {
    for col in 0..8 {
        let mut pivot = col;
        let mut best = a[col][col].abs();
        for row in (col + 1)..8 {
            let v = a[row][col].abs();
            if v > best {
                best = v;
                pivot = row;
            }
        }
        if best < 1e-12 {
            return None;
        }
        if pivot != col {
            a.swap(pivot, col);
            b.swap(pivot, col);
        }
        let div = a[col][col];
        for c in col..8 {
            a[col][c] /= div;
        }
        b[col] /= div;
        for row in 0..8 {
            if row == col {
                continue;
            }
            let f = a[row][col];
            for c in col..8 {
                a[row][c] -= f * a[col][c];
            }
            b[row] -= f * b[col];
        }
    }
    Some(*b)
}

pub fn map_each(src: &Image, mut f: impl FnMut(u32, u32, [f32; 4]) -> [f32; 4]) -> Image {
    let mut out = src.clone();
    for y in 0..src.height {
        for x in 0..src.width {
            let i = (y as usize) * (src.width as usize) + (x as usize);
            out.px[i] = f(x, y, src.px[i]);
        }
    }
    out
}

pub fn resize_canvas(src: &Image, width: u32, height: u32, ox: i32, oy: i32) -> Image {
    let mut out = Image::new(width, height, src.channels, src.bits, [0.0; 4]);
    for y in 0..src.height {
        for x in 0..src.width {
            out.set(x as i32 + ox, y as i32 + oy, src.at(x, y));
        }
    }
    out
}

pub fn max_abs_diff(a: &Image, b: &Image) -> f32 {
    let n = a.px.len().min(b.px.len());
    let mut m = 0.0_f32;
    for i in 0..n {
        for c in 0..4 {
            m = m.max((a.px[i][c] - b.px[i][c]).abs());
        }
    }
    m
}

/// 8 bpc の符号付き変位。0 が最大の負、128 が 0、255 が最大の正。
pub fn displace_amount(sample01: f32, bits: BitsPerChannel) -> f32 {
    match bits {
        BitsPerChannel::Bpc8 => {
            let code = (sample01 * 255.0).round().clamp(0.0, 255.0);
            if code <= 128.0 {
                (code - 128.0) / 128.0
            } else {
                (code - 128.0) / 127.0
            }
        }
        BitsPerChannel::Bpc16 => {
            let code = (sample01 * 32768.0).round().clamp(0.0, 32768.0);
            (code - 16384.0) / 16384.0
        }
        BitsPerChannel::Bpc32 => (sample01 - 0.5) / 0.5,
    }
}
