use std::cmp::Ordering;

use crate::image::Image;

pub fn luma(pixel: [f32; 4]) -> f32 {
    0.2126 * pixel[0] + 0.7152 * pixel[1] + 0.0722 * pixel[2]
}

pub fn clamp01(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
}

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

pub fn mix_rgb(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    [
        lerp(a[0], b[0], t),
        lerp(a[1], b[1], t),
        lerp(a[2], b[2], t),
        lerp(a[3], b[3], t),
    ]
}

pub fn hash_u32(mut n: u32) -> u32 {
    n ^= n >> 16;
    n = n.wrapping_mul(0x7feb352d);
    n ^= n >> 15;
    n = n.wrapping_mul(0x846ca68b);
    n ^= n >> 16;
    n
}

pub fn hash01(x: i32, y: i32, seed: u32, salt: u32) -> f32 {
    let n = (x as u32)
        .wrapping_mul(374761393)
        .wrapping_add((y as u32).wrapping_mul(668265263))
        .wrapping_add(seed.wrapping_mul(1442695041))
        .wrapping_add(salt);
    (hash_u32(n) >> 8) as f32 / 16_777_215.0
}

pub fn value_noise(x: f32, y: f32, seed: u32) -> f32 {
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let tx = smooth(x - x0 as f32);
    let ty = smooth(y - y0 as f32);
    let v00 = hash01(x0, y0, seed, 1);
    let v10 = hash01(x0 + 1, y0, seed, 1);
    let v01 = hash01(x0, y0 + 1, seed, 1);
    let v11 = hash01(x0 + 1, y0 + 1, seed, 1);
    let a = lerp(v00, v10, tx);
    let b = lerp(v01, v11, tx);
    lerp(a, b, ty)
}

fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

pub fn degrees_to_dir(degrees: f64) -> (f32, f32) {
    let radians = (degrees as f32).to_radians();
    (radians.cos(), radians.sin())
}

pub fn rgb(pixel: [f32; 4]) -> [f32; 3] {
    [pixel[0], pixel[1], pixel[2]]
}

pub fn with_rgb(pixel: [f32; 4], rgb: [f32; 3]) -> [f32; 4] {
    [rgb[0], rgb[1], rgb[2], pixel[3]]
}

pub fn with_alpha(pixel: [f32; 4], alpha: f32) -> [f32; 4] {
    [pixel[0], pixel[1], pixel[2], alpha]
}

pub fn dist3(a: [f32; 3], b: [f32; 3]) -> f32 {
    let dr = a[0] - b[0];
    let dg = a[1] - b[1];
    let db = a[2] - b[2];
    (dr * dr + dg * dg + db * db).sqrt()
}

pub fn max_channel_delta(a: [f32; 3], b: [f32; 3]) -> f32 {
    (a[0] - b[0])
        .abs()
        .max((a[1] - b[1]).abs())
        .max((a[2] - b[2]).abs())
}

/// 正で絞り、負で広げる。半径はピクセル。
pub fn morph_alpha(image: &Image, amount: f32) -> Vec<f32> {
    let radius = amount.abs().round() as i32;
    let dilate = amount < 0.0;
    let mut out = Vec::with_capacity(image.pixels.len());
    for y in 0..image.height as i32 {
        for x in 0..image.width as i32 {
            let mut value: f32 = if dilate { 0.0 } else { 1.0 };
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    let sample = image
                        .index(x + dx, y + dy)
                        .map(|index| image.pixels[index][3])
                        .unwrap_or(0.0);
                    value = if dilate {
                        value.max(sample)
                    } else {
                        value.min(sample)
                    };
                }
            }
            out.push(value);
        }
    }
    out
}

pub fn write_alpha(image: &mut Image, alpha: &[f32]) {
    for (pixel, value) in image.pixels.iter_mut().zip(alpha) {
        pixel[3] = *value;
    }
}

pub fn box_blur_channel(image: &Image, channel: usize, radius: i32) -> Vec<f32> {
    let width = image.width as i32;
    let height = image.height as i32;
    let mut horizontal = vec![0.0; image.pixels.len()];
    let span = (radius * 2 + 1).max(1) as f32;
    for y in 0..height {
        for x in 0..width {
            let mut sum = 0.0;
            for dx in -radius..=radius {
                sum += image.get_clamped(x + dx, y)[channel];
            }
            horizontal[(y * width + x) as usize] = sum / span;
        }
    }
    let mut out = vec![0.0; image.pixels.len()];
    for y in 0..height {
        for x in 0..width {
            let mut sum = 0.0;
            for dy in -radius..=radius {
                let yy = (y + dy).clamp(0, height - 1);
                sum += horizontal[(yy * width + x) as usize];
            }
            out[(y * width + x) as usize] = sum / span;
        }
    }
    out
}

pub fn median_channel(image: &Image, channel: usize, radius: i32) -> Vec<f32> {
    let mut out = Vec::with_capacity(image.pixels.len());
    for y in 0..image.height as i32 {
        for x in 0..image.width as i32 {
            let mut values = Vec::new();
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    values.push(image.get_clamped(x + dx, y + dy)[channel]);
                }
            }
            values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
            out.push(values[values.len() / 2]);
        }
    }
    out
}

pub fn rgb_to_hsl(r: f32, g: f32, b: f32) -> [f32; 3] {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) * 0.5;
    if (max - min).abs() < 1e-6 {
        return [0.0, 0.0, l];
    }
    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };
    let h = if (max - r).abs() < 1e-6 {
        (g - b) / d + if g < b { 6.0 } else { 0.0 }
    } else if (max - g).abs() < 1e-6 {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    [h * 60.0, s, l]
}

pub fn hsl_to_rgb(h: f32, s: f32, l: f32) -> [f32; 3] {
    let s = clamp01(s);
    let l = l.clamp(0.0, 1.0);
    let h = h.rem_euclid(360.0);
    if s <= 1e-6 {
        return [l, l, l];
    }
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = h / 60.0;
    let x = c * (1.0 - (hp.rem_euclid(2.0) - 1.0).abs());
    let (r, g, b) = match hp as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c * 0.5;
    [r + m, g + m, b + m]
}

pub fn rgb_to_yuv(r: f32, g: f32, b: f32) -> [f32; 3] {
    let y = 0.299 * r + 0.587 * g + 0.114 * b;
    let u = (b - y) * 0.5 + 0.5;
    let v = (r - y) * 0.5 + 0.5;
    [y, u, v]
}

fn pivot_lab(value: f32) -> f32 {
    if value > 0.008856 {
        value.cbrt()
    } else {
        (7.787 * value) + 16.0 / 116.0
    }
}

pub fn rgb_to_lab(r: f32, g: f32, b: f32) -> [f32; 3] {
    let r = srgb_to_linear(clamp01(r));
    let g = srgb_to_linear(clamp01(g));
    let b = srgb_to_linear(clamp01(b));
    let x = (r * 0.4124 + g * 0.3576 + b * 0.1805) / 0.95047;
    let y = r * 0.2126 + g * 0.7152 + b * 0.0722;
    let z = (r * 0.0193 + g * 0.1192 + b * 0.9505) / 1.08883;
    let fx = pivot_lab(x);
    let fy = pivot_lab(y);
    let fz = pivot_lab(z);
    let l = 116.0 * fy - 16.0;
    let a = 500.0 * (fx - fy);
    let b = 200.0 * (fy - fz);
    [l / 100.0, (a + 128.0) / 255.0, (b + 128.0) / 255.0]
}

fn srgb_to_linear(value: f32) -> f32 {
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

pub fn map_stops(value: f32, black: f32, white: f32, gamma: f32) -> f32 {
    if (white - black).abs() < 1e-6 {
        return if value >= white { 1.0 } else { 0.0 };
    }
    let t = ((value - black) / (white - black)).clamp(0.0, 1.0);
    t.powf(1.0 / gamma.max(1e-4))
}

pub fn overflow(value: f32, mode: &str) -> f32 {
    match mode {
        "Clip" => clamp01(value),
        "Soft Clamp" => {
            if (0.0..=1.0).contains(&value) {
                value
            } else if value > 1.0 {
                1.0 - 0.25 / (1.0 + (value - 1.0) * 4.0)
            } else {
                0.25 / (1.0 + (-value) * 4.0)
            }
        }
        "Wrap Back" => {
            let mut x = value.rem_euclid(2.0);
            if x > 1.0 {
                x = 2.0 - x;
            }
            x
        }
        "Allow HDR Results" => value,
        _ => value,
    }
}

pub fn blend_mode(base: [f32; 4], grain: [f32; 4], mode: &str) -> [f32; 4] {
    let rgb = match mode {
        "Film" | "Multiply" => [base[0] * grain[0], base[1] * grain[1], base[2] * grain[2]],
        "Add" => [base[0] + grain[0], base[1] + grain[1], base[2] + grain[2]],
        "Screen" => [
            1.0 - (1.0 - base[0]) * (1.0 - grain[0]),
            1.0 - (1.0 - base[1]) * (1.0 - grain[1]),
            1.0 - (1.0 - base[2]) * (1.0 - grain[2]),
        ],
        "Overlay" => [
            overlay(base[0], grain[0]),
            overlay(base[1], grain[1]),
            overlay(base[2], grain[2]),
        ],
        _ => [grain[0], grain[1], grain[2]],
    };
    [rgb[0], rgb[1], rgb[2], base[3]]
}

fn overlay(base: f32, blend: f32) -> f32 {
    if base < 0.5 {
        2.0 * base * blend
    } else {
        1.0 - 2.0 * (1.0 - base) * (1.0 - blend)
    }
}

pub fn fbm(
    x: f32,
    y: f32,
    seed: u32,
    octaves: i32,
    influence: f32,
    sub_scale: f32,
    sub_rotation_deg: f32,
    evolution: f32,
    turbulence: f32,
) -> f32 {
    let mut amp_sum = 0.0;
    let mut value = 0.0;
    let mut px = x;
    let mut py = y;
    let mut frequency = 1.0;
    let rot = sub_rotation_deg.to_radians();
    let (cs, sn) = (rot.cos(), rot.sin());
    let octaves = octaves.max(1);
    for octave in 0..octaves {
        let weight = if octave == 0 {
            1.0
        } else {
            influence.powi(octave)
        };
        let speed = 1.0 + turbulence * frequency;
        let phase = evolution * speed;
        let sample = value_noise(
            px * frequency + phase,
            py * frequency - phase * 0.37,
            seed.wrapping_add(octave as u32 * 17),
        );
        value += sample * weight;
        amp_sum += weight;
        let nx = px * sub_scale;
        let ny = py * sub_scale;
        px = nx * cs - ny * sn;
        py = nx * sn + ny * cs;
        frequency *= 2.0;
    }
    if amp_sum <= 1e-6 {
        0.0
    } else {
        value / amp_sum
    }
}

pub fn limit_choke(choke: f32, geometric: f32) -> f32 {
    let limit = geometric.abs();
    choke.clamp(-limit, limit)
}

pub fn gray_softness(alpha: f32, softness: f32) -> f32 {
    let softness = clamp01(softness);
    if softness >= 0.999 {
        return alpha;
    }
    if softness <= 0.001 {
        return if alpha >= 0.5 { 1.0 } else { 0.0 };
    }
    let contrast = 1.0 / softness;
    clamp01((alpha - 0.5) * contrast + 0.5)
}

pub fn percent(value: f64) -> f32 {
    (value as f32) / 100.0
}
