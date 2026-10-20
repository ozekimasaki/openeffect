use model::footage::{AlphaInterpretation, AlphaMode, FieldSeparation, Pulldown};
use model::layer::{LayerQuality, Sampling};
use model::Rgba;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlphaGuess {
    Straight,
    PremultipliedBlack,
    Uncertain,
}

pub struct GuessReport {
    pub guess: AlphaGuess,
    pub open_dialog: bool,
    pub warn: bool,
}

pub fn guess_alpha(pixels: &[Rgba], ask_user: bool) -> GuessReport {
    let mut straight = 0;
    let mut premul = 0;
    for pixel in pixels {
        if pixel.a <= 0.02 || pixel.a >= 0.98 {
            continue;
        }
        let color = pixel.r.max(pixel.g).max(pixel.b);
        if color <= pixel.a * 0.15 {
            premul += 1;
        } else if (color - pixel.a).abs() > 0.2 || color > pixel.a + 0.05 {
            straight += 1;
        }
    }
    let guess = if straight == 0 && premul == 0 {
        AlphaGuess::Uncertain
    } else if premul > straight * 2 {
        AlphaGuess::PremultipliedBlack
    } else if straight > premul * 2 {
        AlphaGuess::Straight
    } else {
        AlphaGuess::Uncertain
    };
    let uncertain = guess == AlphaGuess::Uncertain;
    GuessReport {
        guess,
        open_dialog: uncertain && ask_user,
        warn: uncertain && !ask_user,
    }
}

pub fn apply_alpha(pixel: Rgba, interpretation: AlphaInterpretation, invert: bool) -> Rgba {
    let mut alpha = match interpretation.mode {
        AlphaMode::Ignored => 1.0,
        AlphaMode::Straight | AlphaMode::Premultiplied => pixel.a,
    };
    if invert {
        alpha = 1.0 - alpha;
    }
    let (r, g, b) = if interpretation.mode == AlphaMode::Premultiplied && pixel.a > 0.0 {
        let unpremul = |channel: f32, matte: f32| (channel - matte * (1.0 - pixel.a)) / pixel.a;
        (
            unpremul(pixel.r, interpretation.matte.r),
            unpremul(pixel.g, interpretation.matte.g),
            unpremul(pixel.b, interpretation.matte.b),
        )
    } else {
        (pixel.r, pixel.g, pixel.b)
    };
    Rgba::new(r, g, b, alpha)
}

pub fn separate_fields(frame: &[Rgba], width: u32, height: u32, fields: FieldSeparation) -> Vec<Vec<Rgba>> {
    if fields == FieldSeparation::Off || width == 0 || height == 0 {
        return vec![frame.to_vec()];
    }
    let pixel = |x: u32, y: u32| frame[(y * width + x) as usize];
    let mut upper = vec![Rgba::transparent(); frame.len()];
    let mut lower = vec![Rgba::transparent(); frame.len()];
    for y in 0..height {
        for x in 0..width {
            let index = (y * width + x) as usize;
            if y % 2 == 0 {
                upper[index] = pixel(x, y);
                if y + 1 < height {
                    upper[((y + 1) * width + x) as usize] = pixel(x, y);
                }
            } else {
                lower[index] = pixel(x, y);
                lower[((y - 1) * width + x) as usize] = pixel(x, y);
            }
        }
    }
    match fields {
        FieldSeparation::Off => vec![frame.to_vec()],
        FieldSeparation::UpperFirst => vec![upper, lower],
        FieldSeparation::LowerFirst => vec![lower, upper],
    }
}

pub fn preserve_edges_static(before: &[Rgba], after: &[Rgba], quality: LayerQuality, preserve: bool) -> Vec<Rgba> {
    if quality == LayerQuality::Draft || !preserve {
        return after.to_vec();
    }
    before
        .iter()
        .zip(after)
        .map(|(src, filtered)| {
            if (src.r - filtered.r).abs() < 0.05 {
                *src
            } else {
                *filtered
            }
        })
        .collect()
}

/// 3:2 は 5 ビデオフレームを 4 フィルムフレームに戻す。位相の英語名は要確認。
pub fn remove_pulldown(frames: &[Rgba], pulldown: Pulldown, fields_separated: bool) -> Option<Vec<Rgba>> {
    if matches!(pulldown, Pulldown::Off) {
        return Some(frames.to_vec());
    }
    if !fields_separated {
        return None;
    }
    let phase = match pulldown {
        Pulldown::Off => return Some(frames.to_vec()),
        Pulldown::ThreeTwo { phase } | Pulldown::TwentyFourPa { phase } => phase,
    };
    let pattern: [usize; 4] = match pulldown {
        Pulldown::TwentyFourPa { .. } => [0, 1, 3, 4],
        Pulldown::ThreeTwo { .. } | Pulldown::Off => [0, 2, 3, 4],
    };
    let mut out = Vec::new();
    let mut offset = 0;
    while offset + 5 <= frames.len() {
        for slot in pattern {
            let index = offset + ((slot + phase as usize) % 5);
            out.push(frames[index]);
        }
        offset += 5;
    }
    Some(out)
}

pub fn pulldown_rate(frame_rate: f64, pulldown: Pulldown) -> f64 {
    match pulldown {
        Pulldown::Off => frame_rate,
        Pulldown::ThreeTwo { .. } | Pulldown::TwentyFourPa { .. } => frame_rate * 4.0 / 5.0,
    }
}

pub fn snap_subpixel(position: f64, quality: LayerQuality) -> f64 {
    match quality {
        LayerQuality::Best => (position * 65536.0).round() / 65536.0,
        LayerQuality::Draft | LayerQuality::Wireframe => position.round(),
    }
}

pub fn diagonal_coverage(x: f64, y: f64, quality: LayerQuality) -> f32 {
    let delta = x - y;
    match quality {
        LayerQuality::Draft | LayerQuality::Wireframe => {
            if delta >= 0.0 { 1.0 } else { 0.0 }
        }
        LayerQuality::Best => {
            let snapped = snap_subpixel(delta, quality);
            (0.5 + snapped).clamp(0.0, 1.0) as f32
        }
    }
}

pub fn resample_1d(samples: &[f32], x: f32, sampling: Sampling) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    match sampling {
        Sampling::Bilinear => {
            let x0 = x.floor() as isize;
            let t = x - x0 as f32;
            let a = at(samples, x0);
            let b = at(samples, x0 + 1);
            a + (b - a) * t
        }
        Sampling::Bicubic => {
            let x0 = x.floor() as isize;
            let t = x - x0 as f32;
            let p0 = at(samples, x0 - 1);
            let p1 = at(samples, x0);
            let p2 = at(samples, x0 + 1);
            let p3 = at(samples, x0 + 2);
            let a = -0.5 * p0 + 1.5 * p1 - 1.5 * p2 + 0.5 * p3;
            let b = p0 - 2.5 * p1 + 2.0 * p2 - 0.5 * p3;
            let c = -0.5 * p0 + 0.5 * p2;
            let d = p1;
            ((a * t + b) * t + c) * t + d
        }
    }
}

fn at(samples: &[f32], index: isize) -> f32 {
    if index < 0 {
        samples[0]
    } else if index as usize >= samples.len() {
        samples[samples.len() - 1]
    } else {
        samples[index as usize]
    }
}

pub fn collapse_resample(row: &[f32], child_scale: f32, parent_scale: f32, collapse: bool) -> Vec<f32> {
    if collapse {
        scale_nearest(row, child_scale * parent_scale)
    } else {
        let reduced = scale_nearest(row, child_scale);
        scale_nearest(&reduced, parent_scale)
    }
}

pub fn rasterize_step(scale: f32, continuous: bool) -> Vec<f32> {
    let count = (4.0 * scale).round().max(1.0) as usize;
    (0..count)
        .map(|index| {
            let x = index as f32 / scale;
            if continuous {
                (x - 1.5).clamp(0.0, 1.0)
            } else if x >= 2.0 {
                1.0
            } else {
                0.0
            }
        })
        .collect()
}

pub fn scale_nearest(row: &[f32], factor: f32) -> Vec<f32> {
    let out_len = ((row.len() as f32) * factor).round().max(1.0) as usize;
    (0..out_len)
        .map(|index| {
            let src = ((index as f32) / factor).floor() as usize;
            row[src.min(row.len() - 1)]
        })
        .collect()
}

pub fn blur_span(distance_per_frame: f64, shutter_angle_degrees: f64) -> f64 {
    distance_per_frame * shutter_angle_degrees / 360.0
}

pub fn motion_blur_sample_count(minimum: u32, limit: u32, adaptive: Option<u32>) -> u32 {
    let minimum = minimum.max(1);
    let limit = limit.max(minimum);
    match adaptive {
        None => minimum,
        Some(adaptive) => adaptive.clamp(minimum, limit),
    }
}

pub fn shadow_map_size(width: u32, height: u32, divisor: u32) -> (u32, u32) {
    let divisor = divisor.max(1);
    (width / divisor, height / divisor)
}

pub fn frame_mix(a: &[f32], b: &[f32], t: f32) -> Vec<f32> {
    a.iter().zip(b).map(|(left, right)| left + (right - left) * t).collect()
}

pub fn pixel_motion_row(a: &[f32], b: &[f32], t: f32) -> Vec<f32> {
    let Some(ca) = centroid(a) else {
        return frame_mix(a, b, t);
    };
    let Some(cb) = centroid(b) else {
        return frame_mix(a, b, t);
    };
    let center = ca + (cb - ca) * t;
    let mut out = vec![0.0; a.len()];
    let index = center.round() as isize;
    if (0..out.len() as isize).contains(&index) {
        out[index as usize] = 1.0;
    }
    out
}

fn centroid(row: &[f32]) -> Option<f32> {
    let mut weight = 0.0;
    let mut moment = 0.0;
    for (index, sample) in row.iter().enumerate() {
        weight += *sample;
        moment += *sample * index as f32;
    }
    if weight <= 0.0 {
        None
    } else {
        Some(moment / weight)
    }
}

pub fn text_coverage(text: &str, x: u32) -> f32 {
    if text.is_empty() {
        0.0
    } else if x < text.chars().count() as u32 {
        1.0
    } else {
        0.0
    }
}

pub fn project_camera(
    point: [f64; 3],
    camera_position: [f64; 3],
    yaw_degrees: f64,
    focal: f64,
) -> [f64; 2] {
    let yaw = yaw_degrees.to_radians();
    let x = point[0] - camera_position[0];
    let y = point[1] - camera_position[1];
    let z = point[2] - camera_position[2];
    let xr = x * yaw.cos() - z * yaw.sin();
    let zr = x * yaw.sin() + z * yaw.cos();
    let depth = if zr.abs() < 1.0e-6 { 1.0 } else { zr };
    [focal * xr / depth, focal * y / depth]
}
