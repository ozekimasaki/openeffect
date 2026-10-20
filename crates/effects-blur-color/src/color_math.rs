//! 色の変換。パラメータの既定値は持たない。

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Overflow {
    Wrap,
    Solarize,
    Polarize,
}

pub fn luma(r: f32, g: f32, b: f32) -> f32 {
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

pub fn clamp01(v: f32) -> f32 {
    v.clamp(0.0, 1.0)
}

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

pub fn rgb_to_hsl(r: f32, g: f32, b: f32) -> [f32; 3] {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) * 0.5;
    let d = max - min;
    if d <= 1.0e-8 {
        return [0.0, 0.0, l];
    }
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };
    let h = if (max - r).abs() <= 1.0e-8 {
        let mut v = (g - b) / d;
        if g < b {
            v += 6.0;
        }
        v
    } else if (max - g).abs() <= 1.0e-8 {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    } / 6.0;
    [h.rem_euclid(1.0), s.clamp(0.0, 1.0), l]
}

fn hue_channel(p: f32, q: f32, t: f32) -> f32 {
    let t = t.rem_euclid(1.0);
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

pub fn hsl_to_rgb(h: f32, s: f32, l: f32) -> [f32; 3] {
    let s = s.clamp(0.0, 1.0);
    let l = l.clamp(0.0, 1.0);
    if s <= 1.0e-8 {
        return [l, l, l];
    }
    let h = h.rem_euclid(1.0);
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    [
        hue_channel(p, q, h + 1.0 / 3.0),
        hue_channel(p, q, h),
        hue_channel(p, q, h - 1.0 / 3.0),
    ]
}

pub fn srgb_to_linear(c: f32) -> f32 {
    let sign = if c < 0.0 { -1.0 } else { 1.0 };
    let c = c.abs();
    let v = if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    };
    sign * v
}

pub fn linear_to_srgb(c: f32) -> f32 {
    let sign = if c < 0.0 { -1.0 } else { 1.0 };
    let c = c.abs();
    let v = if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };
    sign * v
}

pub fn rgb_to_cmyk(r: f32, g: f32, b: f32) -> [f32; 4] {
    let r = clamp01(r);
    let g = clamp01(g);
    let b = clamp01(b);
    let k = 1.0 - r.max(g).max(b);
    if k >= 1.0 - 1.0e-6 {
        return [0.0, 0.0, 0.0, 1.0];
    }
    let scale = 1.0 - k;
    [
        (1.0 - r - k) / scale,
        (1.0 - g - k) / scale,
        (1.0 - b - k) / scale,
        k,
    ]
}

pub fn cmyk_to_rgb(c: f32, m: f32, y: f32, k: f32) -> [f32; 3] {
    let c = clamp01(c);
    let m = clamp01(m);
    let y = clamp01(y);
    let k = clamp01(k);
    [
        (1.0 - c) * (1.0 - k),
        (1.0 - m) * (1.0 - k),
        (1.0 - y) * (1.0 - k),
    ]
}

/// 2 周（720 度）で元の値に戻る。位相 0 では入力のまま。
pub fn phase_shift(value: f32, phase_degrees: f32, overflow: Overflow) -> f32 {
    let turned = value + phase_degrees / 360.0;
    let within_two = turned.rem_euclid(2.0);
    match overflow {
        Overflow::Wrap => turned.rem_euclid(1.0),
        Overflow::Solarize => {
            if within_two <= 1.0 {
                within_two
            } else {
                2.0 - within_two
            }
        }
        Overflow::Polarize => {
            if within_two <= 1.0 {
                within_two
            } else {
                let reflected = 2.0 - within_two;
                reflected * reflected
            }
        }
    }
}

pub fn screen(a: f32, b: f32) -> f32 {
    1.0 - (1.0 - a) * (1.0 - b)
}

pub fn ire_from_linear(value: f32) -> f32 {
    value * 100.0
}

#[derive(Clone, Debug, PartialEq)]
pub struct Curve {
    pub points: Vec<(f32, f32)>,
}

impl Curve {
    pub fn identity() -> Self {
        Self {
            points: vec![(0.0, 0.0), (1.0, 1.0)],
        }
    }

    pub fn eval(&self, x: f32) -> f32 {
        let points = &self.points;
        if points.is_empty() {
            return x;
        }
        if x <= points[0].0 {
            return points[0].1;
        }
        if x >= points[points.len() - 1].0 {
            return points[points.len() - 1].1;
        }
        for pair in points.windows(2) {
            let (x0, y0) = pair[0];
            let (x1, y1) = pair[1];
            if x >= x0 && x <= x1 {
                let span = x1 - x0;
                if span.abs() <= 1.0e-8 {
                    return y1;
                }
                let t = (x - x0) / span;
                return lerp(y0, y1, t);
            }
        }
        x
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Primary {
    Reds,
    Yellows,
    Greens,
    Cyans,
    Blues,
    Magentas,
    Whites,
    Neutrals,
    Blacks,
}

pub fn primary_weight(rgb: [f32; 3], primary: Primary) -> f32 {
    let [h, s, l] = rgb_to_hsl(rgb[0], rgb[1], rgb[2]);
    let hue_deg = h * 360.0;
    let around = |center: f32| -> f32 {
        let mut d = (hue_deg - center).rem_euclid(360.0);
        if d > 180.0 {
            d = 360.0 - d;
        }
        (1.0 - d / 30.0).clamp(0.0, 1.0) * s
    };
    match primary {
        Primary::Reds => around(0.0),
        Primary::Yellows => around(60.0),
        Primary::Greens => around(120.0),
        Primary::Cyans => around(180.0),
        Primary::Blues => around(240.0),
        Primary::Magentas => around(300.0),
        Primary::Whites => {
            if l >= 0.9 && s <= 0.1 {
                1.0
            } else {
                0.0
            }
        }
        Primary::Blacks => {
            if l <= 0.1 {
                1.0
            } else {
                0.0
            }
        }
        Primary::Neutrals => {
            if s <= 0.1 && l > 0.1 && l < 0.9 {
                1.0
            } else {
                0.0
            }
        }
    }
}
