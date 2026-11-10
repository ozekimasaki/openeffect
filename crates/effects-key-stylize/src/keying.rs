use effect_api::{ApplyError, ApplyResult};
use model::Rgba;

use crate::image::Image;
use crate::ops::{
    box_blur_channel, clamp01, dist3, luma, map_stops, max_channel_delta, mix_rgb, percent, rgb,
    with_alpha, with_rgb,
};
use crate::spec::ApplyCtx;

pub fn advanced_spill_suppressor(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    let method = ctx.choice("method")?;
    let mut out = ctx.source.clone();
    for pixel in &mut out.pixels {
        let next = match method.as_str() {
            "Standard" => standard_spill(*pixel),
            "Ultra" => ultra_spill(*pixel),
            _ => {
                return Err(ApplyError::new(format!(
                    "Method の選択肢に {method} は無い"
                )))
            }
        };
        *pixel = next;
    }
    Ok(out)
}

fn standard_spill(pixel: [f32; 4]) -> [f32; 4] {
    let mut rgb = rgb(pixel);
    if let Some(index) = spill_channel(&rgb) {
        let others = max_of_others(&rgb, index);
        if rgb[index] > others {
            rgb[index] = others;
        }
    }
    with_rgb(pixel, rgb)
}

fn ultra_spill(pixel: [f32; 4]) -> [f32; 4] {
    let original_luma = luma(pixel);
    let mut rgb = rgb(pixel);
    if let Some(index) = spill_channel(&rgb) {
        rgb[index] = min_of_others(&rgb, index);
        let next_luma = luma(with_rgb(pixel, rgb));
        let delta = original_luma - next_luma;
        for channel in &mut rgb {
            *channel += delta;
        }
    }
    with_rgb(pixel, rgb)
}

/// 緑か青が赤より強いときだけスピルとみなす。赤の前景は落とさない。
fn spill_channel(rgb: &[f32; 3]) -> Option<usize> {
    if rgb[1] > rgb[0] && rgb[1] >= rgb[2] {
        Some(1)
    } else if rgb[2] > rgb[0] && rgb[2] > rgb[1] {
        Some(2)
    } else {
        None
    }
}

fn max_of_others(rgb: &[f32; 3], index: usize) -> f32 {
    match index {
        0 => rgb[1].max(rgb[2]),
        1 => rgb[0].max(rgb[2]),
        _ => rgb[0].max(rgb[1]),
    }
}

fn min_of_others(rgb: &[f32; 3], index: usize) -> f32 {
    match index {
        0 => rgb[1].min(rgb[2]),
        1 => rgb[0].min(rgb[2]),
        _ => rgb[0].min(rgb[1]),
    }
}

pub fn cc_simple_wire_removal(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    let a = ctx.point("point-a")?;
    let b = ctx.point("point-b")?;
    let style = ctx.choice("removal-style")?;
    let thickness = ctx.f64("thickness")? as f32;
    let slope = ctx.f64("slope")? as f32;
    let mirror = percent(ctx.f64("mirror-blend")?);
    let mut out = ctx.source.clone();
    let ax = a[0] as f32;
    let ay = a[1] as f32;
    let bx = b[0] as f32;
    let by = b[1] as f32;
    let dx = bx - ax;
    let dy = by - ay;
    let len = (dx * dx + dy * dy).sqrt().max(1e-4);
    let nx = -dy / len;
    let ny = dx / len;
    for y in 0..out.height as i32 {
        for x in 0..out.width as i32 {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let dist = distance_to_segment(px, py, ax, ay, bx, by);
            let cover = wire_cover(dist, thickness, slope);
            if cover <= 0.0 {
                continue;
            }
            let original = ctx.source.get(x, y);
            let replaced = match style.as_str() {
                "Fade" | "Displace" => {
                    let offset = thickness.max(1.0);
                    let pos = ctx.source.sample(px + nx * offset, py + ny * offset);
                    let neg = ctx.source.sample(px - nx * offset, py - ny * offset);
                    mix_rgb(neg, pos, mirror.clamp(0.0, 1.0))
                }
                "Displace Horizontal" => {
                    let shift = thickness.max(1.0);
                    ctx.source.sample(px + shift, py)
                }
                "Frame Offset" => {
                    let sample = ctx
                        .sample
                        .ok_or_else(|| ApplyError::new("Frame Offset には比較フレームが必要"))?;
                    sample.sample(px, py)
                }
                _ => {
                    return Err(ApplyError::new(format!(
                        "Removal Style の選択肢に {style} は無い"
                    )))
                }
            };
            out.set(x, y, mix_rgb(original, replaced, cover));
        }
    }
    Ok(out)
}

fn distance_to_segment(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let dx = bx - ax;
    let dy = by - ay;
    let len2 = dx * dx + dy * dy;
    let t = if len2 <= 1e-6 {
        0.0
    } else {
        ((px - ax) * dx + (py - ay) * dy) / len2
    };
    let t = t.clamp(0.0, 1.0);
    let qx = ax + dx * t;
    let qy = ay + dy * t;
    ((px - qx).powi(2) + (py - qy).powi(2)).sqrt()
}

fn wire_cover(dist: f32, thickness: f32, slope: f32) -> f32 {
    let band = thickness.max(0.0);
    if slope >= 50.0 {
        return if dist <= band { 1.0 } else { 0.0 };
    }
    let soft = (band / slope.max(0.05)).max(band);
    if dist <= band {
        1.0
    } else if dist >= band + soft {
        0.0
    } else {
        1.0 - (dist - band) / soft
    }
}

pub fn color_difference_key(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    let key = ctx.color("key-color")?;
    let accuracy = ctx.choice("color-matching-accuracy")?;
    let view = ctx.choice_opt("view")?;
    let black_a = ctx.f64("black-a")? as f32;
    let white_a = ctx.f64("white-a")? as f32;
    let gamma_a = ctx.f64("gamma-a")? as f32;
    let black_b = ctx.f64("black-b")? as f32;
    let white_b = ctx.f64("white-b")? as f32;
    let gamma_b = ctx.f64("gamma-b")? as f32;
    let black_f = ctx.f64("black-final")? as f32;
    let white_f = ctx.f64("white-final")? as f32;
    let gamma_f = ctx.f64("gamma-final")? as f32;
    let mut out = ctx.source.clone();
    for (pixel, src) in out.pixels.iter_mut().zip(&ctx.source.pixels) {
        let mut sample = *src;
        if accuracy == "Faster" {
            for channel in 0..3 {
                let steps = 16.0;
                sample[channel] = (sample[channel] * steps).round() / steps;
            }
        } else if accuracy != "More Accurate" {
            return Err(ApplyError::new(format!(
                "Color Matching Accuracy の選択肢に {accuracy} は無い"
            )));
        }
        let (raw_a, raw_b) = partial_mattes(sample, key);
        let matte_a = map_stops(raw_a, black_a, white_a, gamma_a);
        let matte_b = map_stops(raw_b, black_b, white_b, gamma_b);
        let uncorrected = (matte_a - matte_b).clamp(0.0, 1.0);
        let corrected = map_stops(uncorrected, black_f, white_f, gamma_f);
        let view_name = view.as_deref().unwrap_or("Final Output");
        *pixel = match view_name {
            "Final Output" | "Corrected" => with_alpha(*src, src[3] * (1.0 - corrected)),
            "Matte Corrected" => [corrected, corrected, corrected, 1.0],
            "Uncorrected" => [uncorrected, uncorrected, uncorrected, 1.0],
            "[A, B, Matte] Corrected, Final" => [matte_a, matte_b, corrected, 1.0],
            _ => {
                return Err(ApplyError::new(format!(
                    "View の選択肢に {view_name} は無い"
                )))
            }
        };
    }
    Ok(out)
}

fn partial_mattes(pixel: [f32; 4], key: Rgba) -> (f32, f32) {
    let key = [key.r, key.g, key.b];
    let norm = (key[0] * key[0] + key[1] * key[1] + key[2] * key[2])
        .sqrt()
        .max(1e-6);
    let primary = (pixel[0] * key[0] + pixel[1] * key[1] + pixel[2] * key[2]) / norm;
    let secondary = ((1.0 - key[0]) * pixel[0])
        .max((1.0 - key[1]) * pixel[1])
        .max((1.0 - key[2]) * pixel[2]);
    (
        (primary - secondary).clamp(0.0, 1.0),
        (secondary - primary).clamp(0.0, 1.0),
    )
}

pub fn color_range(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    let space = ctx.choice("color-space")?;
    let fuzz = ctx.f64("fuzziness")? as f32;
    let min1 = ctx.f64("min-1")? as f32;
    let max1 = ctx.f64("max-1")? as f32;
    let min2 = ctx.f64("min-2")? as f32;
    let max2 = ctx.f64("max-2")? as f32;
    let min3 = ctx.f64("min-3")? as f32;
    let max3 = ctx.f64("max-3")? as f32;
    let mut out = ctx.source.clone();
    for pixel in &mut out.pixels {
        let comps = match space.as_str() {
            "RGB" => [pixel[0], pixel[1], pixel[2]],
            "YUV" => crate::ops::rgb_to_yuv(pixel[0], pixel[1], pixel[2]),
            "Lab" => crate::ops::rgb_to_lab(pixel[0], pixel[1], pixel[2]),
            _ => {
                return Err(ApplyError::new(format!(
                    "Color Space の選択肢に {space} は無い"
                )))
            }
        };
        let inside = range_gate(comps[0], min1, max1, fuzz)
            * range_gate(comps[1], min2, max2, fuzz)
            * range_gate(comps[2], min3, max3, fuzz);
        pixel[3] *= 1.0 - inside;
    }
    Ok(out)
}

fn range_gate(value: f32, min: f32, max: f32, fuzz: f32) -> f32 {
    if fuzz <= 0.0 {
        return if value >= min && value <= max {
            1.0
        } else {
            0.0
        };
    }
    let delta = if value < min {
        min - value
    } else if value > max {
        value - max
    } else {
        0.0
    };
    (1.0 - delta / fuzz).clamp(0.0, 1.0)
}

pub fn difference_matte(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    let other = ctx.layer("difference-layer")?;
    let fit = ctx.choice("if-layer-sizes-differ")?;
    let tolerance = ctx.f64("matching-tolerance")? as f32;
    let softness = ctx.f64("matching-softness")? as f32;
    let blur = ctx.f64("blur-before-difference")?.round() as i32;
    let view = ctx.choice_opt("view")?;
    let source_cmp = blurred_rgb(ctx.source, blur);
    let other_cmp = blurred_rgb(other, blur);
    let mut out = ctx.source.clone();
    for y in 0..out.height as i32 {
        for x in 0..out.width as i32 {
            let (ox, oy) = map_layer(x, y, ctx.source, other, &fit)?;
            let src = sample_vec(&source_cmp, ctx.source.width, x, y);
            let diff = if (0..other.width as i32).contains(&ox)
                && (0..other.height as i32).contains(&oy)
            {
                sample_vec(&other_cmp, other.width, ox, oy)
            } else {
                [f32::MAX, f32::MAX, f32::MAX]
            };
            let distance = max_channel_delta(src, diff);
            let transparent = softness_key(distance, tolerance, softness);
            let alpha = ctx.source.get(x, y)[3] * (1.0 - transparent);
            let view_name = view.as_deref().unwrap_or("Final Output");
            let pixel = match view_name {
                "Final Output" => with_alpha(ctx.source.get(x, y), alpha),
                "Matte Only" => [alpha, alpha, alpha, 1.0],
                _ => {
                    return Err(ApplyError::new(format!(
                        "View の選択肢に {view_name} は無い"
                    )))
                }
            };
            out.set(x, y, pixel);
        }
    }
    Ok(out)
}

fn map_layer(x: i32, y: i32, source: &Image, other: &Image, fit: &str) -> ApplyResult<(i32, i32)> {
    if source.width == other.width && source.height == other.height {
        return Ok((x, y));
    }
    match fit {
        "Center" => {
            let ox = x - (source.width as i32 - other.width as i32) / 2;
            let oy = y - (source.height as i32 - other.height as i32) / 2;
            Ok((ox, oy))
        }
        "Stretch To Fit" => {
            let ox = if source.width <= 1 {
                0
            } else {
                (x as f32 / (source.width - 1) as f32 * (other.width.saturating_sub(1)) as f32)
                    .round() as i32
            };
            let oy = if source.height <= 1 {
                0
            } else {
                (y as f32 / (source.height - 1) as f32 * (other.height.saturating_sub(1)) as f32)
                    .round() as i32
            };
            Ok((ox, oy))
        }
        _ => Err(ApplyError::new(format!(
            "If Layer Sizes Differ の選択肢に {fit} は無い"
        ))),
    }
}

fn blurred_rgb(image: &Image, radius: i32) -> Vec<[f32; 3]> {
    if radius <= 0 {
        return image.pixels.iter().map(|pixel| rgb(*pixel)).collect();
    }
    let mut tmp = image.clone();
    for channel in 0..3 {
        let blurred = box_blur_channel(image, channel, radius);
        for (pixel, value) in tmp.pixels.iter_mut().zip(&blurred) {
            pixel[channel] = *value;
        }
    }
    tmp.pixels.iter().map(|pixel| rgb(*pixel)).collect()
}

fn sample_vec(pixels: &[[f32; 3]], width: u32, x: i32, y: i32) -> [f32; 3] {
    pixels[y as usize * width as usize + x as usize]
}

fn softness_key(distance: f32, tolerance: f32, softness: f32) -> f32 {
    let start = (tolerance - softness).max(0.0);
    if distance <= start {
        1.0
    } else if softness <= 0.0 || distance >= tolerance {
        if distance <= tolerance {
            1.0
        } else {
            0.0
        }
    } else {
        1.0 - (distance - start) / (tolerance - start).max(1e-4)
    }
}

pub fn extract(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    let channel = ctx.choice("channel")?;
    let black = ctx.f64("black-point")? as f32 / 255.0;
    let white = ctx.f64("white-point")? as f32 / 255.0;
    let black_soft = ctx.f64("black-softness")? as f32 / 255.0;
    let white_soft = ctx.f64("white-softness")? as f32 / 255.0;
    let mut out = ctx.source.clone();
    for pixel in &mut out.pixels {
        let value = match channel.as_str() {
            "Luminance" => luma(*pixel),
            "Red" => pixel[0],
            "Green" => pixel[1],
            "Blue" => pixel[2],
            "Alpha" => pixel[3],
            _ => {
                return Err(ApplyError::new(format!(
                    "Channel の選択肢に {channel} は無い"
                )))
            }
        };
        let keep = extract_keep(value, black, white, black_soft, white_soft);
        pixel[3] *= keep;
    }
    Ok(out)
}

fn extract_keep(value: f32, black: f32, white: f32, black_soft: f32, white_soft: f32) -> f32 {
    if value < black {
        if black_soft <= 0.0 {
            0.0
        } else {
            ((value - (black - black_soft)) / black_soft).clamp(0.0, 1.0)
        }
    } else if value > white {
        if white_soft <= 0.0 {
            0.0
        } else {
            (((white + white_soft) - value) / white_soft).clamp(0.0, 1.0)
        }
    } else {
        1.0
    }
}

pub fn inner_outer_key(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    let foreground = ctx
        .mask("foreground")?
        .ok_or_else(|| ApplyError::new("Foreground のマスクが無い"))?;
    let background = ctx.mask("background")?;
    let radius = ctx.f64("single-mask-highlight-radius")? as f32;
    let edge_thin = ctx.f64("edge-thin")? as f32;
    let edge_feather = ctx.f64("edge-feather")?.round() as i32;
    let edge_threshold = ctx.f64("edge-threshold")? as f32;
    let invert = ctx.bool("invert-extraction")?;
    let blend = percent(ctx.f64("blend-with-original")?);
    let brush_radius = ctx.f64("brush-radius")?.round() as i32;
    let brush_pressure = percent(ctx.f64("brush-pressure")?);
    let extra_fg = ctx.mask_if_present("additional-foreground")?;
    let extra_bg = ctx.mask_if_present("additional-background")?;
    let mut alpha = Vec::with_capacity(ctx.source.pixels.len());
    for y in 0..ctx.source.height as i32 {
        for x in 0..ctx.source.width as i32 {
            let inside = mask_at(foreground, x, y);
            let outside = background.map(|mask| mask_at(mask, x, y)).unwrap_or(false);
            let mut value = if background.is_none() {
                single_mask_alpha(foreground, x, y, inside, radius)
            } else if inside && !outside {
                1.0
            } else if outside && !inside {
                0.0
            } else {
                0.5
            };
            if extra_fg.is_some_and(|mask| mask_at(mask, x, y)) {
                value = 1.0;
            }
            if extra_bg.is_some_and(|mask| mask_at(mask, x, y)) {
                value = 0.0;
            }
            alpha.push(value);
        }
    }
    apply_cleanup(
        ctx,
        &mut alpha,
        "cleanup-foreground",
        true,
        brush_radius,
        brush_pressure,
    )?;
    apply_cleanup(
        ctx,
        &mut alpha,
        "cleanup-background",
        false,
        brush_radius,
        brush_pressure,
    )?;
    let mut image = ctx.source.clone();
    write_alpha_vec(&mut image, &alpha);
    let morphed = crate::ops::morph_alpha(&image, edge_thin);
    write_alpha_vec(&mut image, &morphed);
    if edge_feather > 0 {
        let blurred = box_blur_channel(&image, 3, edge_feather);
        write_alpha_vec(&mut image, &blurred);
    }
    for pixel in &mut image.pixels {
        if pixel[3] < edge_threshold {
            pixel[3] = 0.0;
        }
        if invert {
            pixel[3] = 1.0 - pixel[3];
        }
    }
    // 境界の背景色を、内側の画素色で置き換える。
    if background.is_none() {
        let mut rgb_fixed = image.clone();
        for y in 0..image.height as i32 {
            for x in 0..image.width as i32 {
                let a = image.get(x, y)[3];
                if a > 0.02 && a < 0.98 {
                    if let Some(color) = nearest_opaque(&image, x, y) {
                        let mut pixel = rgb_fixed.get(x, y);
                        pixel[0] = color[0];
                        pixel[1] = color[1];
                        pixel[2] = color[2];
                        rgb_fixed.set(x, y, pixel);
                    }
                }
            }
        }
        image = rgb_fixed;
    }
    for (pixel, src) in image.pixels.iter_mut().zip(&ctx.source.pixels) {
        *pixel = mix_rgb(*pixel, *src, blend.clamp(0.0, 1.0));
    }
    Ok(image)
}

fn mask_at(mask: &Image, x: i32, y: i32) -> bool {
    luma(mask.get_clamped(x, y)) >= 0.5 || mask.get_clamped(x, y)[3] >= 0.5
}

fn single_mask_alpha(mask: &Image, x: i32, y: i32, inside: bool, radius: f32) -> f32 {
    if !inside {
        return 0.0;
    }
    if radius <= 0.0 {
        return 1.0;
    }
    let mut dist = radius;
    let r = radius.ceil() as i32;
    for dy in -r..=r {
        for dx in -r..=r {
            if !mask_at(mask, x + dx, y + dy) {
                dist = dist.min((dx * dx + dy * dy) as f32).sqrt();
            }
        }
    }
    (dist / radius).clamp(0.0, 1.0)
}

fn apply_cleanup(
    ctx: &ApplyCtx<'_>,
    alpha: &mut [f32],
    id: &str,
    raise: bool,
    radius: i32,
    pressure: f32,
) -> ApplyResult<()> {
    let Some(mask) = ctx.mask_if_present(id)? else {
        return Ok(());
    };
    let width = ctx.source.width as i32;
    for y in 0..ctx.source.height as i32 {
        for x in 0..width {
            let mut hit = false;
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    if mask_at(mask, x + dx, y + dy) {
                        hit = true;
                    }
                }
            }
            if hit {
                let index = (y * width + x) as usize;
                let target = if raise { 1.0 } else { 0.0 };
                alpha[index] = crate::ops::lerp(alpha[index], target, pressure.clamp(0.0, 1.0));
            }
        }
    }
    Ok(())
}

fn write_alpha_vec(image: &mut Image, alpha: &[f32]) {
    for (pixel, value) in image.pixels.iter_mut().zip(alpha) {
        pixel[3] = *value;
    }
}

fn nearest_opaque(image: &Image, x: i32, y: i32) -> Option<[f32; 4]> {
    for radius in 1..6 {
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let pixel = image.get_clamped(x + dx, y + dy);
                if pixel[3] >= 0.98 {
                    return Some(pixel);
                }
            }
        }
    }
    None
}

pub fn key_cleaner(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    let mut out = ctx.source.clone();
    let width = ctx.source.width as i32;
    let height = ctx.source.height as i32;
    for y in 0..height {
        for x in 0..width {
            let src = ctx.source.get(x, y);
            let mut min_l = luma(src);
            let mut max_l = min_l;
            let mut neighbor_alpha = 0.0;
            let mut count = 0.0;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let pixel = ctx.source.get_clamped(x + dx, y + dy);
                    let value = luma(pixel);
                    min_l = min_l.min(value);
                    max_l = max_l.max(value);
                    neighbor_alpha += pixel[3];
                    count += 1.0;
                }
            }
            let detail = max_l - min_l;
            let average = neighbor_alpha / count;
            let flat = src[3] <= 0.02 || src[3] >= 0.98;
            let edge = (average - src[3]).abs() > 0.2;
            if flat && edge && detail > 0.15 {
                let restored = clamp01(luma(src));
                out.set(
                    x,
                    y,
                    with_alpha(src, crate::ops::lerp(src[3], restored, 0.65)),
                );
            }
        }
    }
    Ok(out)
}

pub fn keylight(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    let screen = ctx.color("screen-colour")?;
    let screen = [screen.r, screen.g, screen.b];
    let indeterminate = ctx.choice_opt("indeterminate-result")?;
    let mut out = ctx.source.clone();
    for pixel in &mut out.pixels {
        let distance = dist3(rgb(*pixel), screen) / 3.0_f32.sqrt();
        pixel[3] = pixel[3] * distance.clamp(0.0, 1.0);
        if indeterminate.as_deref() == Some("RGB") {
            // スピル抑制なしの RGB。アルファだけをスクリーン距離で作る。
        }
    }
    Ok(out)
}

pub fn linear_color_key(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    let operation = ctx.choice("key-operation")?;
    let space = ctx.choice("match-colors")?;
    if space != "RGB" {
        return Err(ApplyError::new(
            "Match Colors は RGB 以外の色空間名が未確認",
        ));
    }
    let view = ctx.choice_opt("view")?;
    let key = ctx.color("key-color")?;
    let key = [key.r, key.g, key.b];
    let tolerance = percent(ctx.f64("matching-tolerance")?);
    let softness = percent(ctx.f64("matching-softness")?);
    let mut out = ctx.source.clone();
    for pixel in &mut out.pixels {
        let distance = dist3(rgb(*pixel), key) / 3.0_f32.sqrt();
        let mut transparency = linear_transparency(distance, tolerance, softness);
        if operation == "Keep Colors" {
            transparency = 1.0 - transparency;
        } else if operation != "Key Colors" {
            return Err(ApplyError::new(format!(
                "Key Operation の選択肢に {operation} は無い"
            )));
        }
        let alpha = pixel[3] * (1.0 - transparency);
        let view_name = view.as_deref().unwrap_or("Final Output");
        *pixel = match view_name {
            "Final Output" => with_alpha(*pixel, alpha),
            "Source Only" => *pixel,
            "Matte Only" => [alpha, alpha, alpha, 1.0],
            _ => {
                return Err(ApplyError::new(format!(
                    "View の選択肢に {view_name} は無い"
                )))
            }
        };
    }
    Ok(out)
}

fn linear_transparency(distance: f32, tolerance: f32, softness: f32) -> f32 {
    if tolerance <= 0.0 {
        return 0.0;
    }
    if tolerance >= 1.0 {
        return 1.0;
    }
    let end = tolerance;
    let start = (tolerance - softness).clamp(0.0, end);
    if (end - start).abs() < 1e-6 {
        if distance <= end {
            1.0
        } else {
            0.0
        }
    } else if distance <= start {
        1.0
    } else if distance >= end {
        0.0
    } else {
        1.0 - (distance - start) / (end - start)
    }
}

pub fn unmult(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    let background = ctx.choice("background-color")?;
    let black_level = ctx.f64("black-level")? as f32;
    let white_level = ctx.f64("white-level")? as f32;
    let softness = ctx.f64("softness")? as f32;
    let remove = ctx.bool("remove-color-matting")?;
    let clip = ctx.bool("clip-hdr-results")?;
    let mut out = ctx.source.clone();
    for pixel in &mut out.pixels {
        let value = pixel[0].max(pixel[1]).max(pixel[2]);
        let alpha = match background.as_str() {
            "Black" => unmult_alpha(value - black_level, softness),
            "White" => unmult_alpha(white_level - value, softness),
            _ => {
                return Err(ApplyError::new(format!(
                    "Background Color の選択肢に {background} は無い"
                )))
            }
        };
        let mut rgb = rgb(*pixel);
        if remove {
            let bg = if background == "White" { 1.0 } else { 0.0 };
            if alpha > 1e-4 {
                for channel in &mut rgb {
                    *channel = (*channel - bg * (1.0 - alpha)) / alpha;
                }
            }
            if clip {
                for channel in &mut rgb {
                    *channel = clamp01(*channel);
                }
            }
        }
        *pixel = [rgb[0], rgb[1], rgb[2], alpha];
    }
    Ok(out)
}

fn unmult_alpha(delta: f32, softness: f32) -> f32 {
    if softness <= 0.0 {
        if delta <= 0.0 {
            0.0
        } else {
            1.0
        }
    } else {
        (delta / softness).clamp(0.0, 1.0)
    }
}
