//! Blur & Sharpen の CPU 処理。型が要確認の量は引数で受け、既定値は置かない。

use effect_api::{ApplyError, ApplyResult};

use crate::color_math::luma;
use crate::image::{
    bilateral, blur_channels, box_blur, directional_blur, gaussian_blur, sharpness, spin_blur,
    zoom_blur, Edge, Image,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IrisSides {
    Four,
    Six,
}

pub fn bilateral_blur(
    src: &Image,
    radius: f32,
    threshold: f32,
    colorize: bool,
) -> ApplyResult<Image> {
    bilateral(src, radius, threshold, colorize)
}

pub fn lens_blur(
    src: &Image,
    amount: f32,
    sides: IrisSides,
    roundness: f32,
    aspect: f32,
    rotation_deg: f32,
    diffraction: f32,
    map: Option<&Image>,
    map_channel: MapChannel,
    focal: f32,
    highlight_gain: f32,
    threshold: f32,
    saturation: f32,
    edge: Edge,
    linear_working_space: bool,
) -> ApplyResult<Image> {
    if !amount.is_finite() || amount < 0.0 {
        return Err(ApplyError::new("ぼかし量が不正"));
    }
    let radius = amount.floor().min(64.0) as i32;
    let mut out = src.clone();
    let rot = rotation_deg.to_radians();
    let (sin, cos) = rot.sin_cos();
    let aspect = if aspect.abs() < 1.0e-3 { 1.0 } else { aspect };
    for y in 0..src.height {
        for x in 0..src.width {
            let local = if let Some(map) = map {
                let sample = sample_map(map, x, y, src);
                channel_of(sample, map_channel)
            } else {
                0.0
            };
            let here = (radius as f32) * (local - focal).abs();
            let here = if map.is_none() { radius as f32 } else { here };
            let pixel = gather_iris(
                src,
                x as i32,
                y as i32,
                here,
                sides,
                roundness,
                aspect,
                sin,
                cos,
                diffraction,
                threshold,
                highlight_gain,
                saturation,
                edge,
                linear_working_space,
            );
            out.set(x, y, pixel);
        }
    }
    Ok(out)
}

fn sample_map(map: &Image, x: u32, y: u32, dst: &Image) -> [f32; 4] {
    if map.width == 0 || map.height == 0 {
        return [0.0; 4];
    }
    let sx = (x as f32 * map.width as f32 / dst.width as f32).floor() as u32;
    let sy = (y as f32 * map.height as f32 / dst.height as f32).floor() as u32;
    map.get(sx.min(map.width - 1), sy.min(map.height - 1))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapChannel {
    Luminance,
    Color,
    Alpha,
}

fn channel_of(pixel: [f32; 4], channel: MapChannel) -> f32 {
    match channel {
        MapChannel::Luminance => luma(pixel[0], pixel[1], pixel[2]),
        MapChannel::Color => (pixel[0] + pixel[1] + pixel[2]) / 3.0,
        MapChannel::Alpha => pixel[3],
    }
}

fn gather_iris(
    src: &Image,
    x: i32,
    y: i32,
    radius: f32,
    sides: IrisSides,
    roundness: f32,
    aspect: f32,
    sin: f32,
    cos: f32,
    diffraction: f32,
    threshold: f32,
    highlight_gain: f32,
    saturation: f32,
    edge: Edge,
    linear: bool,
) -> [f32; 4] {
    let radius_i = radius.ceil().max(0.0).min(48.0) as i32;
    if radius_i == 0 {
        return src.sample(x, y, edge);
    }
    let ring = (diffraction / 500.0).clamp(0.0, 1.0);
    let mut acc = [0.0; 4];
    let mut weight_sum = 0.0;
    for oy in -radius_i..=radius_i {
        for ox in -radius_i..=radius_i {
            let fx = ox as f32;
            let fy = oy as f32 / aspect;
            let rx = fx * cos - fy * sin;
            let ry = fx * sin + fy * cos;
            let nx = rx / radius_i as f32;
            let ny = ry / radius_i as f32;
            let dist = iris_dist(nx, ny, sides, roundness);
            if dist > 1.0 {
                continue;
            }
            let edge_w = smoothstep(0.65, 1.0, dist);
            let disk_w = 1.0 - dist;
            let weight = disk_w * (1.0 - ring) + edge_w * ring;
            if weight <= 0.0 {
                continue;
            }
            let mut sample = src.sample(x + ox, y + oy, edge);
            if linear {
                for channel in 0..3 {
                    sample[channel] = crate::color_math::srgb_to_linear(sample[channel]);
                }
            }
            let yv = luma(sample[0], sample[1], sample[2]);
            if yv > threshold {
                let lift = (yv - threshold) * highlight_gain;
                for channel in 0..3 {
                    sample[channel] += lift;
                }
                let sat = (saturation / 100.0).clamp(0.0, 1.0);
                let lifted = luma(sample[0], sample[1], sample[2]);
                for channel in 0..3 {
                    sample[channel] = lifted * (1.0 - sat) + sample[channel] * sat;
                }
            }
            for channel in 0..4 {
                acc[channel] += sample[channel] * weight;
            }
            weight_sum += weight;
        }
    }
    if weight_sum > 0.0 {
        for channel in 0..4 {
            acc[channel] /= weight_sum;
        }
    }
    if linear {
        for channel in 0..3 {
            acc[channel] = crate::color_math::linear_to_srgb(acc[channel]);
        }
    }
    acc
}

fn iris_dist(nx: f32, ny: f32, sides: IrisSides, roundness: f32) -> f32 {
    let circle = (nx * nx + ny * ny).sqrt();
    let sides_n = match sides {
        IrisSides::Four => 4.0,
        IrisSides::Six => 6.0,
    };
    let angle = ny.atan2(nx);
    let sector = std::f32::consts::TAU / sides_n;
    let local = (angle + std::f32::consts::FRAC_PI_2).rem_euclid(sector) - sector * 0.5;
    let poly = circle * local.cos() / (sector * 0.5).cos();
    let roundness = roundness.clamp(0.0, 1.0);
    poly * (1.0 - roundness) + circle * roundness
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeblurMethod {
    HighQuality,
    Standard,
}

pub struct DeblurOutput {
    pub image: Image,
    pub comparisons: u32,
}

pub fn camera_shake_deblur(
    current: &Image,
    before: &[Image],
    after: &[Image],
    duration_frames: f32,
    method: DeblurMethod,
    strength_percent: f32,
    sensitivity: f32,
) -> ApplyResult<DeblurOutput> {
    if !strength_percent.is_finite() || strength_percent > 200.0 {
        return Err(ApplyError::new("Strength は 200% を超える"));
    }
    let current_sharp = sharpness(current);
    if current_sharp > sensitivity {
        return Ok(DeblurOutput {
            image: current.clone(),
            comparisons: 0,
        });
    }
    let mut neighbors = Vec::new();
    neighbors.extend(before.iter().chain(after.iter()));
    if neighbors.is_empty() {
        return Ok(DeblurOutput {
            image: current.clone(),
            comparisons: 0,
        });
    }
    let sharpest = neighbors
        .iter()
        .max_by(|a, b| {
            sharpness(a)
                .partial_cmp(&sharpness(b))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .copied()
        .unwrap_or(current);
    let window = duration_frames.max(0.0).round().min(8.0) as i32;
    let (aligned, comparisons) = align(current, sharpest, window, method)?;
    let t = strength_percent / 100.0;
    let image = current.map(|x, y, pixel| {
        let other = aligned.get(x, y);
        let mut out = [0.0; 4];
        for channel in 0..current.channels as usize {
            out[channel] = pixel[channel] + (other[channel] - pixel[channel]) * t;
        }
        out
    });
    Ok(DeblurOutput { image, comparisons })
}

fn align(
    current: &Image,
    neighbor: &Image,
    window: i32,
    method: DeblurMethod,
) -> ApplyResult<(Image, u32)> {
    let mut best = f32::MAX;
    let mut best_shift = (0.0f32, 0.0f32);
    let mut comparisons = 0u32;
    let mut shifts = Vec::new();
    for y in -window..=window {
        for x in -window..=window {
            shifts.push((x as f32, y as f32));
            if method == DeblurMethod::HighQuality {
                shifts.push((x as f32 + 0.5, y as f32));
                shifts.push((x as f32, y as f32 + 0.5));
            }
        }
    }
    for (sx, sy) in shifts {
        comparisons += 1;
        let score = shift_score(current, neighbor, sx, sy);
        if score < best {
            best = score;
            best_shift = (sx, sy);
        }
    }
    let shifted = current.map(|x, y, _| {
        neighbor.sample_bilinear(
            x as f32 - best_shift.0,
            y as f32 - best_shift.1,
            Edge::Repeat,
        )
    });
    Ok((shifted, comparisons))
}

fn shift_score(current: &Image, neighbor: &Image, sx: f32, sy: f32) -> f32 {
    let mut sum = 0.0;
    let step = (current.width.max(1) / 4).max(1);
    let mut count = 0.0;
    let mut y = 0;
    while y < current.height {
        let mut x = 0;
        while x < current.width {
            let a = current.get(x, y);
            let b = neighbor.sample_bilinear(x as f32 - sx, y as f32 - sy, Edge::Repeat);
            sum += (a[0] - b[0]).abs() + (a[1] - b[1]).abs() + (a[2] - b[2]).abs();
            count += 1.0;
            x += step;
        }
        y += step;
    }
    if count == 0.0 {
        f32::MAX
    } else {
        sum / count
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrossMode {
    Blend,
    Add,
    Screen,
    Multiply,
    Lighten,
    Darken,
}

pub fn cross_blur(
    src: &Image,
    radius_x: f32,
    radius_y: f32,
    mode: CrossMode,
    repeat_edge: bool,
) -> ApplyResult<Image> {
    let edge = if repeat_edge {
        Edge::Repeat
    } else {
        Edge::Zero
    };
    let horizontal = if radius_x <= 0.0 {
        src.clone()
    } else {
        blur_axis(src, radius_x, true, edge)?
    };
    let vertical = if radius_y <= 0.0 {
        src.clone()
    } else {
        blur_axis(src, radius_y, false, edge)?
    };
    Ok(src.map(|x, y, original| {
        let a = if radius_y <= 0.0 {
            horizontal.get(x, y)
        } else if radius_x <= 0.0 {
            vertical.get(x, y)
        } else {
            transfer(mode, horizontal.get(x, y), vertical.get(x, y))
        };
        if radius_x <= 0.0 && radius_y <= 0.0 {
            original
        } else if radius_x <= 0.0 || radius_y <= 0.0 {
            transfer(mode, a, original)
        } else {
            a
        }
    }))
}

fn blur_axis(src: &Image, radius: f32, horizontal: bool, edge: Edge) -> ApplyResult<Image> {
    let radius = crate::image::radius_px(radius)?;
    let kernel = crate::image::box_kernel(radius.max(1));
    Ok(crate::image::convolve_axis(src, &kernel, horizontal, edge))
}

fn transfer(mode: CrossMode, a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    let mut out = [0.0; 4];
    for channel in 0..4 {
        out[channel] = match mode {
            CrossMode::Blend => (a[channel] + b[channel]) * 0.5,
            CrossMode::Add => a[channel] + b[channel],
            CrossMode::Screen => crate::color_math::screen(a[channel], b[channel]),
            CrossMode::Multiply => a[channel] * b[channel],
            CrossMode::Lighten => a[channel].max(b[channel]),
            CrossMode::Darken => a[channel].min(b[channel]),
        };
    }
    out
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RadialType {
    StraightZoom,
    FadingZoom,
    CenteredZoom,
    Rotate,
    Scratch,
    RotateFading,
}

pub fn cc_radial(
    src: &Image,
    kind: RadialType,
    amount: f32,
    quality: u32,
    center: [f32; 2],
) -> ApplyResult<Image> {
    let samples = quality.clamp(2, 32);
    match kind {
        RadialType::StraightZoom => {
            zoom_blur(src, center, amount, samples, false, false, Edge::Zero)
        }
        RadialType::FadingZoom => zoom_blur(src, center, amount, samples, false, true, Edge::Zero),
        RadialType::CenteredZoom => {
            zoom_blur(src, center, amount, samples, true, false, Edge::Zero)
        }
        RadialType::Rotate => spin_blur(src, center, amount, samples, false, Edge::Zero),
        RadialType::Scratch => spin_blur(src, center, amount * 0.25, samples, false, Edge::Zero),
        RadialType::RotateFading => spin_blur(src, center, amount, samples, true, Edge::Zero),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FastZoom {
    Standard,
    Brightest,
    Darkest,
}

pub fn radial_fast(
    src: &Image,
    center: [f32; 2],
    amount: f32,
    zoom: FastZoom,
) -> ApplyResult<Image> {
    let samples = 8u32;
    Ok(src.map(|x, y, pixel| {
        if amount.abs() < 1.0e-6 {
            return pixel;
        }
        let mut acc = [0.0; 4];
        let mut extreme = match zoom {
            FastZoom::Brightest => [f32::MIN; 4],
            FastZoom::Darkest => [f32::MAX; 4],
            FastZoom::Standard => [0.0; 4],
        };
        for step in 0..samples {
            let u = step as f32 / (samples - 1) as f32;
            let scale = 1.0 + (amount / 100.0) * u;
            let sx = center[0] + (x as f32 - center[0]) * scale;
            let sy = center[1] + (y as f32 - center[1]) * scale;
            let sample = src.sample_bilinear(sx, sy, Edge::Zero);
            match zoom {
                FastZoom::Standard => {
                    for channel in 0..4 {
                        acc[channel] += sample[channel];
                    }
                }
                FastZoom::Brightest => {
                    for channel in 0..4 {
                        extreme[channel] = extreme[channel].max(sample[channel]);
                    }
                }
                FastZoom::Darkest => {
                    for channel in 0..4 {
                        extreme[channel] = extreme[channel].min(sample[channel]);
                    }
                }
            }
        }
        if zoom == FastZoom::Standard {
            for channel in 0..4 {
                acc[channel] /= samples as f32;
            }
            acc
        } else {
            extreme
        }
    }))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VectorType {
    Natural,
    ConstantLength,
    Perpendicular,
    DirectionCenter,
    DirectionFading,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VectorProperty {
    Red,
    Green,
    Blue,
    Alpha,
    Luminance,
    Lightness,
    Hue,
    Saturation,
}

pub fn vector_blur(
    src: &Image,
    map: &Image,
    kind: VectorType,
    amount: f32,
    angle_offset_deg: f32,
    ridge: f32,
    property: VectorProperty,
    map_softness: f32,
) -> ApplyResult<Image> {
    let field = if map_softness > 0.0 {
        gaussian_blur(map, map_softness, Edge::Repeat)?
    } else {
        map.clone()
    };
    let direction_type = matches!(
        kind,
        VectorType::DirectionCenter | VectorType::DirectionFading
    );
    Ok(src.map(|x, y, pixel| {
        let value = property_value(field.get(x, y), property);
        let (mut vx, mut vy) = if direction_type {
            let turns = if ridge == 0.0 { 1.0 } else { ridge };
            let angle = (value * turns + angle_offset_deg / 360.0) * std::f32::consts::TAU;
            (angle.cos(), angle.sin())
        } else {
            let gx = property_value(field.sample(x as i32 + 1, y as i32, Edge::Repeat), property)
                - property_value(field.sample(x as i32 - 1, y as i32, Edge::Repeat), property);
            let gy = property_value(field.sample(x as i32, y as i32 + 1, Edge::Repeat), property)
                - property_value(field.sample(x as i32, y as i32 - 1, Edge::Repeat), property);
            let (gx, gy) = if kind == VectorType::Perpendicular {
                (-gy, gx)
            } else {
                (gx, gy)
            };
            let offset = angle_offset_deg.to_radians();
            let (sin, cos) = offset.sin_cos();
            (gx * cos - gy * sin, gx * sin + gy * cos)
        };
        let length = match kind {
            VectorType::Natural => {
                let slope = (vx * vx + vy * vy).sqrt();
                let smooth = 1.0 / (1.0 + ridge.abs());
                amount * slope * smooth
            }
            VectorType::ConstantLength
            | VectorType::Perpendicular
            | VectorType::DirectionCenter => amount,
            VectorType::DirectionFading => amount * value,
        };
        let norm = (vx * vx + vy * vy).sqrt();
        if norm > 1.0e-6 {
            vx /= norm;
            vy /= norm;
        }
        let mut acc = [0.0; 4];
        let steps = 4;
        for step in 0..=steps {
            let t = step as f32 / steps as f32;
            let sample = src.sample_bilinear(
                x as f32 + vx * length * t,
                y as f32 + vy * length * t,
                Edge::Zero,
            );
            for channel in 0..4 {
                acc[channel] += sample[channel];
            }
        }
        for channel in 0..4 {
            acc[channel] /= (steps + 1) as f32;
        }
        let _ = pixel;
        acc
    }))
}

fn property_value(pixel: [f32; 4], property: VectorProperty) -> f32 {
    match property {
        VectorProperty::Red => pixel[0],
        VectorProperty::Green => pixel[1],
        VectorProperty::Blue => pixel[2],
        VectorProperty::Alpha => pixel[3],
        VectorProperty::Luminance => luma(pixel[0], pixel[1], pixel[2]),
        VectorProperty::Lightness => crate::color_math::rgb_to_hsl(pixel[0], pixel[1], pixel[2])[2],
        VectorProperty::Hue => crate::color_math::rgb_to_hsl(pixel[0], pixel[1], pixel[2])[0],
        VectorProperty::Saturation => {
            crate::color_math::rgb_to_hsl(pixel[0], pixel[1], pixel[2])[1]
        }
    }
}

pub fn channel_blur(src: &Image, radii: [f32; 4], repeat_edge: bool) -> ApplyResult<Image> {
    blur_channels(
        src,
        radii,
        if repeat_edge {
            Edge::Repeat
        } else {
            Edge::Zero
        },
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapFit {
    Stretch,
    Center,
    TopLeft,
}

pub fn compound_blur(
    src: &Image,
    map: &Image,
    maximum: f32,
    invert: bool,
    fit: MapFit,
) -> ApplyResult<Image> {
    let mut out = src.clone();
    for y in 0..src.height {
        for x in 0..src.width {
            let mapped = placed(map, x, y, src, fit);
            let mut amount = luma(mapped[0], mapped[1], mapped[2]);
            if invert {
                amount = 1.0 - amount;
            }
            let radius = (maximum * amount).max(0.0);
            let blurred = if radius <= 0.0 {
                src.get(x, y)
            } else {
                let r = crate::image::radius_px(radius.min(8.0))? as i32;
                let mut acc = [0.0; 4];
                let mut count = 0.0;
                for oy in -r..=r {
                    for ox in -r..=r {
                        let sample = src.sample(x as i32 + ox, y as i32 + oy, Edge::Zero);
                        for channel in 0..4 {
                            acc[channel] += sample[channel];
                        }
                        count += 1.0;
                    }
                }
                for channel in 0..4 {
                    acc[channel] /= count;
                }
                acc
            };
            out.set(x, y, blurred);
        }
    }
    Ok(out)
}

pub fn placed(map: &Image, x: u32, y: u32, dst: &Image, fit: MapFit) -> [f32; 4] {
    match fit {
        MapFit::Stretch => sample_map(map, x, y, dst),
        MapFit::Center => sample_placed(map, x, y, dst, true),
        MapFit::TopLeft => sample_placed(map, x, y, dst, false),
    }
}

fn sample_placed(map: &Image, x: u32, y: u32, dst: &Image, center: bool) -> [f32; 4] {
    let ox = if center {
        (dst.width as i32 - map.width as i32) / 2
    } else {
        0
    };
    let oy = if center {
        (dst.height as i32 - map.height as i32) / 2
    } else {
        0
    };
    let sx = x as i32 - ox;
    let sy = y as i32 - oy;
    if sx < 0 || sy < 0 || sx >= map.width as i32 || sy >= map.height as i32 {
        [0.0; 4]
    } else {
        map.get(sx as u32, sy as u32)
    }
}

pub fn directional(src: &Image, angle_deg: f32, length: f32, edge: Edge) -> ApplyResult<Image> {
    directional_blur(src, angle_deg, length, edge)
}

pub fn fast_box(src: &Image, radius: f32, edge: Edge) -> ApplyResult<Image> {
    box_blur(src, radius, edge)
}

pub fn gaussian(src: &Image, radius: f32, repeat_edge: bool) -> ApplyResult<Image> {
    gaussian_blur(
        src,
        radius,
        if repeat_edge {
            Edge::Repeat
        } else {
            Edge::Zero
        },
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpinZoom {
    Spin,
    Zoom,
}

pub fn radial_blur(
    src: &Image,
    kind: SpinZoom,
    amount: f32,
    center: [f32; 2],
) -> ApplyResult<Image> {
    match kind {
        SpinZoom::Spin => spin_blur(src, center, amount, 12, false, Edge::Zero),
        SpinZoom::Zoom => zoom_blur(src, center, amount / 100.0, 12, false, false, Edge::Zero),
    }
}

pub fn sharpen(src: &Image, amount: f32) -> ApplyResult<Image> {
    unsharp(src, 1.0, 0.0, amount)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SmartMode {
    Normal,
    EdgeOnly,
    OverlayEdge,
}

pub fn smart_blur(src: &Image, radius: f32, threshold: f32, mode: SmartMode) -> ApplyResult<Image> {
    let radius = crate::image::radius_px(radius)? as i32;
    Ok(src.map(|x, y, pixel| {
        let mut acc = [0.0; 4];
        let mut count = 0.0;
        let mut edge_score = 0.0;
        for oy in -radius..=radius {
            for ox in -radius..=radius {
                let sample = src.sample(x as i32 + ox, y as i32 + oy, Edge::Repeat);
                let mut dist = 0.0;
                for channel in 0..3 {
                    dist += (sample[channel] - pixel[channel]).abs();
                }
                if dist / 3.0 <= threshold {
                    for channel in 0..4 {
                        acc[channel] += sample[channel];
                    }
                    count += 1.0;
                } else {
                    edge_score += 1.0;
                }
            }
        }
        let smoothed = if count > 0.0 {
            for channel in 0..4 {
                acc[channel] /= count;
            }
            acc
        } else {
            pixel
        };
        let is_edge = edge_score > 0.0;
        match mode {
            SmartMode::Normal => smoothed,
            SmartMode::EdgeOnly => {
                if is_edge {
                    [1.0, 1.0, 1.0, pixel[3]]
                } else {
                    [0.0, 0.0, 0.0, pixel[3]]
                }
            }
            SmartMode::OverlayEdge => {
                if is_edge {
                    [1.0, 1.0, 1.0, pixel[3]]
                } else {
                    pixel
                }
            }
        }
    }))
}

pub fn unsharp(src: &Image, radius: f32, threshold: f32, amount: f32) -> ApplyResult<Image> {
    let blurred = gaussian_blur(src, radius.max(0.0), Edge::Repeat)?;
    Ok(src.map(|x, y, pixel| {
        let blur = blurred.get(x, y);
        let mut out = pixel;
        for channel in 0..3.min(src.channels as usize) {
            let delta = pixel[channel] - blur[channel];
            if delta.abs() > threshold {
                out[channel] = pixel[channel] + amount * delta;
            }
        }
        out
    }))
}
