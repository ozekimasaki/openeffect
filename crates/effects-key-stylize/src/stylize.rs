use std::f32::consts::TAU;

use effect_api::{ApplyError, ApplyResult};

use crate::image::Image;
use crate::ops::{
    box_blur_channel, clamp01, degrees_to_dir, hash01, lerp, luma, mix_rgb, percent, rgb_to_hsl,
    value_noise,
};
use crate::spec::ApplyCtx;

pub fn brush_strokes(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    let angle = ctx.angle("stroke-angle")?;
    let size = ctx.f64("brush-size")?.max(0.0) as f32;
    let length = ctx.f64("stroke-length")?.max(0.0) as f32;
    let density = ctx.f64("stroke-density")?.max(1.0).round() as i32;
    let randomness = ctx.f64("stroke-randomness")? as f32;
    let surface = ctx.choice("paint-surface")?;
    let blend = percent(ctx.f64("blend-with-original")?);
    let (dx, dy) = degrees_to_dir(angle);
    let mut out = ctx.source.clone();
    for y in 0..out.height as i32 {
        for x in 0..out.width as i32 {
            let mut acc = [0.0; 4];
            let mut count = 0.0;
            for step in 0..density.max(1) {
                let along = if length == 0.0 {
                    0.0
                } else {
                    (step as f32 / density.max(1) as f32 - 0.5) * length
                };
                let jitter = (hash01(x, y, step as u32, 2) - 0.5) * randomness;
                let sx = x as f32 + dx * along + -dy * jitter;
                let sy = y as f32 + dy * along + dx * jitter;
                let sample = ctx.source.sample(sx, sy);
                for channel in 0..4 {
                    acc[channel] += sample[channel];
                }
                count += 1.0;
            }
            let _ = size;
            let mut painted = [
                acc[0] / count,
                acc[1] / count,
                acc[2] / count,
                acc[3] / count,
            ];
            let base = match surface.as_str() {
                "Paint On Original Image" => ctx.source.get(x, y),
                "Paint On Transparent" => [0.0, 0.0, 0.0, 0.0],
                "Paint On White" => [1.0, 1.0, 1.0, 1.0],
                "Paint On Black" => [0.0, 0.0, 0.0, 1.0],
                _ => {
                    return Err(ApplyError::new(format!(
                        "Paint Surface の選択肢に {surface} は無い"
                    )))
                }
            };
            if surface != "Paint On Original Image" {
                painted = mix_rgb(base, painted, painted[3].clamp(0.0, 1.0));
            }
            out.set(
                x,
                y,
                mix_rgb(painted, ctx.source.get(x, y), blend.clamp(0.0, 1.0)),
            );
        }
    }
    Ok(out)
}

pub fn cartoon(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    let render = ctx.choice("render")?;
    let detail_radius = ctx.f64("detail-radius")?.round().max(0.0) as i32;
    let detail_threshold = ctx.f64("detail-threshold")? as f32;
    let steps = ctx.f64("shading-steps")?.round().max(1.0) as i32;
    let shading_smooth = percent(ctx.f64("shading-smoothness")?);
    let edge_threshold = ctx.f64("edge-threshold")? as f32;
    let width = ctx.f64("edge-width")?.round().max(0.0) as i32;
    let softness = ctx.f64("edge-softness")? as f32;
    let opacity = percent(ctx.f64("edge-opacity")?);
    let mut base = ctx.source.clone();
    if detail_radius > 0 {
        for channel in 0..3 {
            let blurred = box_blur_channel(ctx.source, channel, detail_radius);
            for (pixel, value) in base.pixels.iter_mut().zip(&blurred) {
                let src = pixel[channel];
                pixel[channel] = if (src - value).abs() < detail_threshold {
                    *value
                } else {
                    src
                };
            }
        }
    }
    if render == "Fill" || render == "Fill Only" || render == "Fill & Edges" {
        for pixel in &mut base.pixels {
            let shade = quantize(luma(*pixel), steps, shading_smooth);
            let current = luma(*pixel).max(1e-4);
            let scale = shade / current;
            pixel[0] *= scale;
            pixel[1] *= scale;
            pixel[2] *= scale;
        }
    }
    if render == "Edges" || render == "Fill & Edges" {
        let edges = edge_mask(ctx.source, edge_threshold, width, softness);
        for (pixel, edge) in base.pixels.iter_mut().zip(&edges) {
            let ink = opacity.clamp(0.0, 1.0) * edge;
            if render == "Edges" {
                *pixel = [1.0 - ink, 1.0 - ink, 1.0 - ink, pixel[3]];
            } else {
                pixel[0] = lerp(pixel[0], 0.0, ink);
                pixel[1] = lerp(pixel[1], 0.0, ink);
                pixel[2] = lerp(pixel[2], 0.0, ink);
            }
        }
    } else if render != "Fill" && render != "Fill Only" {
        return Err(ApplyError::new(format!(
            "Render の選択肢に {render} は無い"
        )));
    }
    Ok(base)
}

fn quantize(value: f32, steps: i32, smooth: f32) -> f32 {
    let steps = steps.max(1) as f32;
    let scaled = value.clamp(0.0, 1.0) * steps;
    let hard = scaled.floor() / steps;
    let soft = scaled / steps;
    lerp(hard, soft, smooth.clamp(0.0, 1.0))
}

fn edge_mask(image: &Image, threshold: f32, width: i32, softness: f32) -> Vec<f32> {
    let mut mask = Vec::with_capacity(image.pixels.len());
    for y in 0..image.height as i32 {
        for x in 0..image.width as i32 {
            let here = luma(image.get(x, y));
            let dx = (luma(image.get_clamped(x + 1, y)) - luma(image.get_clamped(x - 1, y))).abs();
            let dy = (luma(image.get_clamped(x, y + 1)) - luma(image.get_clamped(x, y - 1))).abs();
            let edge = if dx + dy >= threshold { 1.0 } else { 0.0 };
            let _ = (here, softness);
            mask.push(edge);
        }
    }
    if width <= 1 {
        return mask;
    }
    let mut grown = mask.clone();
    let w = image.width as i32;
    for y in 0..image.height as i32 {
        for x in 0..w {
            let mut value: f32 = 0.0;
            for dy in -width..=width {
                for dx in -width..=width {
                    let xx = (x + dx).clamp(0, w - 1);
                    let yy = (y + dy).clamp(0, image.height as i32 - 1);
                    value = value.max(mask[(yy * w + xx) as usize]);
                }
            }
            grown[(y * w + x) as usize] = value;
        }
    }
    grown
}

pub fn cc_block_load(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    let completion = percent(ctx.f64("completion")?).clamp(0.0, 1.0);
    let scans = ctx.f64("scans")?.round().max(1.0) as i32;
    let start_cleared = ctx.bool("start-cleared")?;
    let mut out = ctx.source.clone();
    let cols = scans;
    let rows = scans;
    let total = (cols * rows).max(1);
    let revealed = if completion <= 0.0 {
        if start_cleared {
            0
        } else {
            cols
        }
    } else if completion >= 1.0 {
        total
    } else {
        (completion * total as f32).floor() as i32
    };
    let block_w = div_ceil_i32(out.width as i32, cols);
    let block_h = div_ceil_i32(out.height as i32, rows);
    for y in 0..out.height as i32 {
        for x in 0..out.width as i32 {
            let col = (x / block_w).min(cols - 1);
            let row = (y / block_h).min(rows - 1);
            let index = row * cols + col;
            if index < revealed {
                out.set(x, y, block_average(ctx.source, x, y, block_w, block_h));
            } else if start_cleared || completion <= 0.0 {
                out.set(x, y, [0.0, 0.0, 0.0, 0.0]);
            }
        }
    }
    if ctx.bool_opt("bilinear")?.unwrap_or(false) {
        for channel in 0..3 {
            let blurred = box_blur_channel(&out, channel, 1);
            for (pixel, value) in out.pixels.iter_mut().zip(&blurred) {
                pixel[channel] = *value;
            }
        }
    }
    Ok(out)
}

fn div_ceil_i32(value: i32, divisor: i32) -> i32 {
    let divisor = divisor.max(1);
    (value + divisor - 1) / divisor
}

fn block_average(image: &Image, x: i32, y: i32, block_w: i32, block_h: i32) -> [f32; 4] {
    let x0 = (x / block_w) * block_w;
    let y0 = (y / block_h) * block_h;
    let mut acc = [0.0; 4];
    let mut count = 0.0;
    for yy in y0..(y0 + block_h).min(image.height as i32) {
        for xx in x0..(x0 + block_w).min(image.width as i32) {
            let pixel = image.get(xx, yy);
            for channel in 0..4 {
                acc[channel] += pixel[channel];
            }
            count += 1.0;
        }
    }
    if count == 0.0 {
        image.get(x, y)
    } else {
        [
            acc[0] / count,
            acc[1] / count,
            acc[2] / count,
            acc[3] / count,
        ]
    }
}

pub fn cc_burn_film(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    let burn = ctx.f64("burn")? as f32;
    let center = ctx.point("center")?;
    let mut out = ctx.source.clone();
    if burn <= 0.0 {
        return Ok(out);
    }
    let max_d = (out.width.max(out.height) as f32) * 0.5;
    for y in 0..out.height as i32 {
        for x in 0..out.width as i32 {
            let dx = x as f32 - center[0] as f32;
            let dy = y as f32 - center[1] as f32;
            let dist = (dx * dx + dy * dy).sqrt();
            let hole = (burn - dist / max_d.max(1.0)).clamp(0.0, 1.0);
            let pixel = ctx
                .source
                .sample(x as f32 + dx * hole, y as f32 + dy * hole);
            out.set(
                x,
                y,
                [pixel[0], pixel[1], pixel[2], pixel[3] * (1.0 - hole)],
            );
        }
    }
    Ok(out)
}

pub fn cc_glass(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    shade_bump(ctx, false)
}

pub fn cc_plastic(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    shade_bump(ctx, true)
}

fn shade_bump(ctx: &ApplyCtx<'_>, plastic: bool) -> ApplyResult<Image> {
    let bump = ctx.layer(if plastic { "bump-layer" } else { "bump-map" })?;
    let property = ctx.choice("property")?;
    let height = ctx.f64("height")? as f32;
    let displacement = ctx.f64_opt("displacement")?.unwrap_or(0.0) as f32;
    let light_type = ctx.choice("light-type")?;
    let direction = ctx.angle("light-direction")?;
    let (lx, ly) = degrees_to_dir(direction);
    let intensity = ctx.f64_opt("light-intensity")?.unwrap_or(1.0) as f32;
    let diffuse = ctx.f64_opt("diffuse")?.unwrap_or(1.0) as f32;
    let ambient = ctx.f64_opt("ambient")?.unwrap_or(0.0) as f32;
    let mut out = ctx.source.clone();
    for y in 0..out.height as i32 {
        for x in 0..out.width as i32 {
            let h = channel_of(bump.get_clamped(x, y), &property)?;
            let hx = channel_of(bump.get_clamped(x + 1, y), &property)?
                - channel_of(bump.get_clamped(x - 1, y), &property)?;
            let hy = channel_of(bump.get_clamped(x, y + 1), &property)?
                - channel_of(bump.get_clamped(x, y - 1), &property)?;
            let nx = -hx * height;
            let ny = -hy * height;
            let shade = if light_type == "Distant Light" || light_type == "Point Light" {
                (nx * lx + ny * ly).max(0.0) * diffuse * intensity + ambient
            } else {
                return Err(ApplyError::new(format!(
                    "Light Type の選択肢に {light_type} は無い"
                )));
            };
            let sample = ctx
                .source
                .sample(x as f32 - nx * displacement, y as f32 - ny * displacement);
            let mut pixel = [
                sample[0] * (1.0 + shade),
                sample[1] * (1.0 + shade),
                sample[2] * (1.0 + shade),
                sample[3],
            ];
            if plastic {
                let min = ctx.f64("cut-min")? as f32;
                let max = ctx.f64("cut-max")? as f32;
                if h < min || h > max {
                    pixel = ctx.source.get(x, y);
                }
            }
            out.set(x, y, pixel);
        }
    }
    Ok(out)
}

fn channel_of(pixel: [f32; 4], property: &str) -> ApplyResult<f32> {
    Ok(match property {
        "Red" => pixel[0],
        "Green" => pixel[1],
        "Blue" => pixel[2],
        "Alpha" => pixel[3],
        "Luminance" | "Lightness" => luma(pixel),
        "Hue" => rgb_to_hsl(pixel[0], pixel[1], pixel[2])[0] / 360.0,
        "Saturation" => rgb_to_hsl(pixel[0], pixel[1], pixel[2])[1],
        _ => {
            return Err(ApplyError::new(format!(
                "Property の選択肢に {property} は無い"
            )))
        }
    })
}

pub fn cc_hex_tile(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    let radius = ctx.f64("radius")?.max(1.0) as f32;
    let center = ctx.point("center")?;
    let lock = ctx.bool("lock-center-tile")?;
    let rotate = ctx.angle("rotate")?;
    let (cs, sn) = {
        let (c, s) = degrees_to_dir(rotate);
        (c, s)
    };
    let origin = if lock {
        [
            ctx.source.width as f32 * 0.5,
            ctx.source.height as f32 * 0.5,
        ]
    } else {
        [center[0] as f32, center[1] as f32]
    };
    let mut out = ctx.source.clone();
    for y in 0..out.height as i32 {
        for x in 0..out.width as i32 {
            let dx = x as f32 - origin[0];
            let dy = y as f32 - origin[1];
            let rx = dx * cs + dy * sn;
            let ry = -dx * sn + dy * cs;
            let wrapped = hex_wrap(rx, ry, radius);
            out.set(
                x,
                y,
                ctx.source
                    .sample(origin[0] + wrapped.0, origin[1] + wrapped.1),
            );
        }
    }
    Ok(out)
}

fn hex_wrap(x: f32, y: f32, radius: f32) -> (f32, f32) {
    let w = radius * 1.5;
    let h = radius * 3.0_f32.sqrt();
    let col = (x / w).round();
    let row = (y / h).round();
    (x - col * w, y - row * h)
}

pub fn cc_kaleida(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    let center = ctx.point("center")?;
    let size = ctx.f64("size")?.max(1.0) as f32;
    let mirroring = ctx.choice("mirroring")?;
    let rotation = ctx.angle("rotation")?;
    let floating = ctx.bool("floating-center")?;
    let origin = if floating {
        [center[0] as f32, center[1] as f32]
    } else {
        [
            ctx.source.width as f32 * 0.5,
            ctx.source.height as f32 * 0.5,
        ]
    };
    let wedges = match mirroring.as_str() {
        "Unfold" => 1.0,
        "Wheel" | "Flower" | "Starlish" => 8.0,
        "Fish Head" | "Can Meas" | "Flip Flop" | "Dia Cross" | "Flipper" => 4.0,
        _ => {
            return Err(ApplyError::new(format!(
                "Mirroring の選択肢に {mirroring} は無い"
            )))
        }
    };
    let (cs, sn) = degrees_to_dir(rotation);
    let mut out = ctx.source.clone();
    for y in 0..out.height as i32 {
        for x in 0..out.width as i32 {
            let dx = x as f32 - origin[0];
            let dy = y as f32 - origin[1];
            let rx = dx * cs - dy * sn;
            let ry = dx * sn + dy * cs;
            let mut angle = ry.atan2(rx);
            let sector = TAU / wedges;
            angle = angle.rem_euclid(sector);
            if mirroring != "Unfold" && angle > sector * 0.5 {
                angle = sector - angle;
            }
            let dist = (rx * rx + ry * ry).sqrt().min(size);
            let sx = origin[0] + angle.cos() * dist;
            let sy = origin[1] + angle.sin() * dist;
            out.set(x, y, ctx.source.sample(sx, sy));
        }
    }
    Ok(out)
}

pub fn cc_mr_smoothie(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    let flow = ctx.layer("flow-layer")?;
    let property = ctx.choice("property")?;
    let sample_a = ctx.point("sample-a")?;
    let sample_b = ctx.point("sample-b")?;
    let phase = ctx.angle("phase")? as f32;
    let color_a = ctx.source.sample(sample_a[0] as f32, sample_a[1] as f32);
    let color_b = ctx.source.sample(sample_b[0] as f32, sample_b[1] as f32);
    let loop_mode = ctx.choice("color-loop")?;
    let mut out = ctx.source.clone();
    for y in 0..out.height as i32 {
        for x in 0..out.width as i32 {
            let raw = channel_of(flow.get_clamped(x, y), &property)? + phase / 360.0;
            let t = if (0.0..=1.0).contains(&raw) {
                raw
            } else {
                raw.rem_euclid(1.0)
            };
            let mixed = match loop_mode.as_str() {
                "AB" => mix_rgb(color_a, color_b, t),
                "BA" => mix_rgb(color_b, color_a, t),
                "ABA" => {
                    if t < 0.5 {
                        mix_rgb(color_a, color_b, t * 2.0)
                    } else {
                        mix_rgb(color_b, color_a, (t - 0.5) * 2.0)
                    }
                }
                "BAB" => {
                    if t < 0.5 {
                        mix_rgb(color_b, color_a, t * 2.0)
                    } else {
                        mix_rgb(color_a, color_b, (t - 0.5) * 2.0)
                    }
                }
                _ => {
                    return Err(ApplyError::new(format!(
                        "Color Loop の選択肢に {loop_mode} は無い"
                    )))
                }
            };
            out.set(x, y, mixed);
        }
    }
    Ok(out)
}

pub fn cc_repeatile(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    let right = ctx.f64("expand-right")?.round().max(0.0) as u32;
    let left = ctx.f64("expand-left")?.round().max(0.0) as u32;
    let down = ctx.f64("expand-down")?.round().max(0.0) as u32;
    let up = ctx.f64("expand-up")?.round().max(0.0) as u32;
    let tiling = ctx.choice("tiling")?;
    let width = ctx.source.width + left + right;
    let height = ctx.source.height + up + down;
    let mut out = Image::new(width, height, ctx.source.bits, [0.0; 4]);
    for y in 0..height as i32 {
        for x in 0..width as i32 {
            let sx = x - left as i32;
            let sy = y - up as i32;
            let (mx, my, flip) = tile_map(
                sx,
                sy,
                ctx.source.width as i32,
                ctx.source.height as i32,
                &tiling,
            )?;
            let mut pixel = ctx.source.get_clamped(mx, my);
            if flip {
                pixel[3] = pixel[3];
            }
            out.set(x, y, pixel);
        }
    }
    Ok(out)
}

fn tile_map(
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    tiling: &str,
) -> ApplyResult<(i32, i32, bool)> {
    if width <= 0 || height <= 0 {
        return Ok((0, 0, false));
    }
    let tx = x.div_euclid(width);
    let ty = y.div_euclid(height);
    let mut mx = x.rem_euclid(width);
    let mut my = y.rem_euclid(height);
    match tiling {
        "Repeat" | "None" => Ok((mx, my, false)),
        "Checker Flip H" => {
            if tx.rem_euclid(2) == 1 {
                mx = width - 1 - mx;
            }
            Ok((mx, my, false))
        }
        "Checker Flip V" => {
            if ty.rem_euclid(2) == 1 {
                my = height - 1 - my;
            }
            Ok((mx, my, false))
        }
        "Unfold" => {
            if tx.rem_euclid(2) == 1 {
                mx = width - 1 - mx;
            }
            if ty.rem_euclid(2) == 1 {
                my = height - 1 - my;
            }
            Ok((mx, my, false))
        }
        "Slide" => Ok((mx, my, false)),
        _ => Err(ApplyError::new(format!(
            "Tiling の選択肢に {tiling} は無い"
        ))),
    }
}

pub fn cc_threshold(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    let threshold = ctx.f64("threshold")? as f32;
    let channel = ctx.choice("channel")?;
    let invert = ctx.bool("invert")?;
    let blend = percent(ctx.f64("blend-with-original")?);
    let mut out = ctx.source.clone();
    for pixel in &mut out.pixels {
        let value = match channel.as_str() {
            "Luminance" => luma(*pixel),
            "RGB" => (pixel[0] + pixel[1] + pixel[2]) / 3.0,
            "Saturation" => rgb_to_hsl(pixel[0], pixel[1], pixel[2])[1],
            "Alpha" => pixel[3],
            _ => {
                return Err(ApplyError::new(format!(
                    "Channel の選択肢に {channel} は無い"
                )))
            }
        };
        let mut on = value >= threshold;
        if invert {
            on = !on;
        }
        let bw = if on { 1.0 } else { 0.0 };
        let hard = [bw, bw, bw, pixel[3]];
        *pixel = mix_rgb(hard, *pixel, blend.clamp(0.0, 1.0));
    }
    Ok(out)
}

pub fn cc_threshold_rgb(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    let red = ctx.f64("red-threshold")? as f32;
    let green = ctx.f64("green-threshold")? as f32;
    let blue = ctx.f64("blue-threshold")? as f32;
    let invert_r = ctx.bool("invert-red")?;
    let invert_g = ctx.bool("invert-green")?;
    let invert_b = ctx.bool("invert-blue")?;
    let blend = percent(ctx.f64("blend-with-original")?);
    let mut out = ctx.source.clone();
    for (pixel, src) in out.pixels.iter_mut().zip(&ctx.source.pixels) {
        let hard = [
            channel_threshold(src[0], red, invert_r),
            channel_threshold(src[1], green, invert_g),
            channel_threshold(src[2], blue, invert_b),
            src[3],
        ];
        *pixel = mix_rgb(hard, *src, blend.clamp(0.0, 1.0));
    }
    Ok(out)
}

fn channel_threshold(value: f32, threshold: f32, invert: bool) -> f32 {
    let on = value >= threshold;
    let on = if invert { !on } else { on };
    if on {
        1.0
    } else {
        0.0
    }
}

pub fn cc_vignette(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    let amount = ctx.f64("amount")? as f32;
    let angle = ctx.angle("angle-of-view")? as f32;
    let center = ctx.point("center")?;
    let pin = ctx.f64_opt("pin-highlights")?.unwrap_or(0.0) as f32;
    let mut out = ctx.source.clone();
    let reach = (angle.to_radians().cos().abs()).max(0.15);
    for y in 0..out.height as i32 {
        for x in 0..out.width as i32 {
            let dx = (x as f32 - center[0] as f32) / out.width.max(1) as f32;
            let dy = (y as f32 - center[1] as f32) / out.height.max(1) as f32;
            let dist = (dx * dx + dy * dy).sqrt() / reach;
            let pixel = out.get(x, y);
            let highlight = luma(pixel);
            let pinned = if pin > 0.0 { highlight * pin } else { 0.0 };
            let shade = 1.0 - amount * dist.clamp(0.0, 1.5) * (1.0 - pinned.clamp(0.0, 1.0));
            out.set(
                x,
                y,
                [
                    pixel[0] * shade,
                    pixel[1] * shade,
                    pixel[2] * shade,
                    pixel[3],
                ],
            );
        }
    }
    Ok(out)
}

pub fn color_emboss(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    // 方向と Relief の初期値は未確認。1 ピクセルの固定差はパラメータではない。
    emboss_image(ctx.source, 0.0, 1.0, 1.0, true)
}

pub fn emboss(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    let direction = ctx.angle("direction")?;
    let relief = ctx.f64("relief")? as f32;
    let contrast = percent(ctx.f64("contrast")?);
    let blend = percent(ctx.f64("blend-with-original")?);
    let mut image = emboss_image(ctx.source, direction, relief, contrast, false)?;
    for (pixel, src) in image.pixels.iter_mut().zip(&ctx.source.pixels) {
        *pixel = mix_rgb(*pixel, *src, blend.clamp(0.0, 1.0));
    }
    Ok(image)
}

fn emboss_image(
    source: &Image,
    direction: f64,
    relief: f32,
    contrast: f32,
    keep_color: bool,
) -> ApplyResult<Image> {
    let (dx, dy) = degrees_to_dir(direction);
    let mut out = source.clone();
    for y in 0..source.height as i32 {
        for x in 0..source.width as i32 {
            let here = luma(source.get(x, y));
            let other = luma(source.sample(x as f32 + dx * relief, y as f32 + dy * relief));
            let shade = (here - other) * contrast;
            let pixel = source.get(x, y);
            if keep_color {
                let next = luma(pixel) + shade;
                let scale = if luma(pixel) > 1e-4 {
                    next / luma(pixel)
                } else {
                    next
                };
                out.set(
                    x,
                    y,
                    [
                        pixel[0] * scale,
                        pixel[1] * scale,
                        pixel[2] * scale,
                        pixel[3],
                    ],
                );
            } else {
                let gray = clamp01(0.5 + shade);
                out.set(x, y, [gray, gray, gray, pixel[3]]);
            }
        }
    }
    Ok(out)
}

pub fn find_edges(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    let invert = ctx.bool("invert")?;
    let blend = percent(ctx.f64("blend-with-original")?);
    let mut out = ctx.source.clone();
    for y in 0..out.height as i32 {
        for x in 0..out.width as i32 {
            let dx = (luma(ctx.source.get_clamped(x + 1, y))
                - luma(ctx.source.get_clamped(x - 1, y)))
            .abs();
            let dy = (luma(ctx.source.get_clamped(x, y + 1))
                - luma(ctx.source.get_clamped(x, y - 1)))
            .abs();
            let edge = (dx + dy).clamp(0.0, 1.0);
            let shown = if invert { edge } else { 1.0 - edge };
            let hard = [shown, shown, shown, ctx.source.get(x, y)[3]];
            out.set(
                x,
                y,
                mix_rgb(hard, ctx.source.get(x, y), blend.clamp(0.0, 1.0)),
            );
        }
    }
    Ok(out)
}

pub fn glow(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    let threshold = percent(ctx.f64("glow-threshold")?);
    let radius = ctx.f64("glow-radius")?.round().max(0.0) as i32;
    let intensity = ctx.f64("glow-intensity")? as f32;
    let mut glow = Image::new(
        ctx.source.width,
        ctx.source.height,
        ctx.source.bits,
        [0.0; 4],
    );
    for y in 0..ctx.source.height as i32 {
        for x in 0..ctx.source.width as i32 {
            let pixel = ctx.source.get(x, y);
            let gate = (luma(pixel) - threshold).max(0.0);
            glow.set(
                x,
                y,
                [pixel[0] * gate, pixel[1] * gate, pixel[2] * gate, gate],
            );
        }
    }
    if radius > 0 {
        for channel in 0..3 {
            let blurred = box_blur_channel(&glow, channel, radius);
            for (pixel, value) in glow.pixels.iter_mut().zip(&blurred) {
                pixel[channel] = *value * intensity;
            }
        }
    }
    let mut out = ctx.source.clone();
    for (pixel, glow_px) in out.pixels.iter_mut().zip(&glow.pixels) {
        pixel[0] += glow_px[0];
        pixel[1] += glow_px[1];
        pixel[2] += glow_px[2];
    }
    Ok(out)
}

pub fn mosaic(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    let horizontal = ctx.f64("horizontal-blocks")?.round().max(1.0) as i32;
    let vertical = ctx.f64("vertical-blocks")?.round().max(1.0) as i32;
    let sharp = ctx.bool("sharp-colors")?;
    let mut out = ctx.source.clone();
    let block_w = div_ceil_i32(out.width as i32, horizontal).max(1);
    let block_h = div_ceil_i32(out.height as i32, vertical).max(1);
    for by in 0..vertical {
        for bx in 0..horizontal {
            let x0 = bx * block_w;
            let y0 = by * block_h;
            let color = if sharp {
                ctx.source.get_clamped(x0 + block_w / 2, y0 + block_h / 2)
            } else {
                block_average(ctx.source, x0, y0, block_w, block_h)
            };
            for y in y0..(y0 + block_h).min(out.height as i32) {
                for x in x0..(x0 + block_w).min(out.width as i32) {
                    out.set(x, y, color);
                }
            }
        }
    }
    Ok(out)
}

pub fn motion_tile(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    let center = ctx.point("tile-center")?;
    let tile_w = percent(ctx.f64("tile-width")?).max(0.01);
    let tile_h = percent(ctx.f64("tile-height")?).max(0.01);
    let out_w = percent(ctx.f64("output-width")?).max(0.01);
    let out_h = percent(ctx.f64("output-height")?).max(0.01);
    let mirror = ctx.bool("mirror-edges")?;
    let phase = percent(ctx.f64("phase")?);
    let horizontal = ctx.bool("horizontal-phase-shift")?;
    let width = ((ctx.source.width as f32) * out_w).round().max(1.0) as u32;
    let height = ((ctx.source.height as f32) * out_h).round().max(1.0) as u32;
    let mut out = Image::new(width, height, ctx.source.bits, [0.0; 4]);
    let tw = (ctx.source.width as f32 * tile_w).max(1.0);
    let th = (ctx.source.height as f32 * tile_h).max(1.0);
    for y in 0..height as i32 {
        for x in 0..width as i32 {
            let mut sx = (x as f32 - center[0] as f32) / tw * ctx.source.width as f32;
            let mut sy = (y as f32 - center[1] as f32) / th * ctx.source.height as f32;
            if horizontal {
                sx += phase * ctx.source.width as f32;
            } else {
                sy += phase * ctx.source.height as f32;
            }
            let pixel = if mirror {
                mirrored_sample(ctx.source, sx, sy)
            } else {
                ctx.source.get_clamped(sx.round() as i32, sy.round() as i32)
            };
            out.set(x, y, pixel);
        }
    }
    Ok(out)
}

fn mirrored_sample(image: &Image, x: f32, y: f32) -> [f32; 4] {
    let mx = mirror_coord(x, image.width.max(1));
    let my = mirror_coord(y, image.height.max(1));
    image.get_clamped(mx.round() as i32, my.round() as i32)
}

fn mirror_coord(value: f32, size: u32) -> f32 {
    let span = size as f32;
    let period = span * 2.0;
    let mut v = value.rem_euclid(period);
    if v >= span {
        v = period - v;
    }
    v.clamp(0.0, span - 1.0)
}

pub fn posterize(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    let levels = ctx.f64("level")?.round().clamp(2.0, 255.0) as i32;
    let mut out = ctx.source.clone();
    let steps = (levels - 1) as f32;
    for pixel in &mut out.pixels {
        for channel in 0..3 {
            pixel[channel] = (pixel[channel].clamp(0.0, 1.0) * steps).round() / steps;
        }
    }
    Ok(out)
}

pub fn roughen_edges(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    let influence = ctx.f64("fractal-influence")? as f32;
    let evolution = ctx.f64_opt("evolution")?.unwrap_or(0.0) as f32;
    let seed = ctx.f64_opt("random-seed")?.unwrap_or(0.0) as u32;
    let mut out = ctx.source.clone();
    for y in 0..out.height as i32 {
        for x in 0..out.width as i32 {
            let n = value_noise(x as f32 * 0.15 + evolution, y as f32 * 0.15, seed) - 0.5;
            let shift = (n * influence).round() as i32;
            let sample = ctx.source.get_clamped(x + shift, y);
            out.set(x, y, sample);
        }
    }
    Ok(out)
}

pub fn scatter(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    let grain = ctx.choice("grain")?;
    let every_frame =
        ctx.choice_opt("scatter-randomness")?.as_deref() == Some("Randomize Every Frame");
    let salt = if every_frame {
        ctx.time.seconds().floor() as u32
    } else {
        5
    };
    // ばらまき幅は「近く」の動作幅で、未確認の初期値ではない。
    let amount = 2.0_f32;
    let mut order: Vec<usize> = (0..ctx.source.pixels.len()).collect();
    let width = ctx.source.width as i32;
    let height = ctx.source.height as i32;
    for y in 0..height {
        for x in 0..width {
            let index = (y * width + x) as usize;
            let jx = ((hash01(x, y, salt, 1) - 0.5) * amount).round() as i32;
            let jy = ((hash01(x, y, salt, 2) - 0.5) * amount).round() as i32;
            let (dx, dy) = match grain.as_str() {
                "水平" => (jx, 0),
                "垂直" => (0, jy),
                "None" => (jx, jy),
                _ => return Err(ApplyError::new(format!("Grain の値に {grain} は無い"))),
            };
            let xx = (x + dx).clamp(0, width - 1);
            let yy = (y + dy).clamp(0, height - 1);
            let other = (yy * width + xx) as usize;
            order.swap(index, other);
        }
    }
    let mut out = ctx.source.clone();
    for (index, source_index) in order.iter().enumerate() {
        out.pixels[index] = ctx.source.pixels[*source_index];
    }
    Ok(out)
}

pub fn strobe_light(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    let color = ctx.color("strobe-color")?;
    let blend = percent(ctx.f64("blend-with-original")?);
    let duration = ctx.f64("strobe-duration")?;
    let period = ctx.f64("strobe-period")?.max(1e-4);
    let probability = percent(ctx.f64("random-strobe-probability")?);
    let strobe = ctx.choice("strobe")?;
    let operator = ctx.choice_opt("strobe-operator")?;
    let phase = ctx.time.seconds().rem_euclid(period);
    let mut on = phase < duration;
    if probability > 0.0 {
        let frame = ctx.time.seconds().floor() as i32;
        on = hash01(frame, 0, 3, 8) < probability;
    }
    let mut out = ctx.source.clone();
    if !on {
        return Ok(out);
    }
    let strobe_px = [color.r, color.g, color.b, color.a];
    for pixel in &mut out.pixels {
        let hit = match strobe.as_str() {
            "Makes Layer Transparent" => [pixel[0], pixel[1], pixel[2], 0.0],
            "Operates On Color Only" => match operator.as_deref().unwrap_or("Add") {
                "Add" => [
                    pixel[0] + strobe_px[0],
                    pixel[1] + strobe_px[1],
                    pixel[2] + strobe_px[2],
                    pixel[3],
                ],
                "Copy" => [strobe_px[0], strobe_px[1], strobe_px[2], pixel[3]],
                _ => return Err(ApplyError::new("Strobe Operator の全選択肢は未確認")),
            },
            _ => {
                return Err(ApplyError::new(format!(
                    "Strobe の選択肢に {strobe} は無い"
                )))
            }
        };
        *pixel = mix_rgb(hit, *pixel, blend.clamp(0.0, 1.0));
    }
    Ok(out)
}

pub fn texturize(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    let texture = ctx.layer("texture-layer")?;
    let direction = ctx.angle("light-direction")?;
    let contrast = ctx.f64("texture-contrast")? as f32;
    let (dx, dy) = degrees_to_dir(direction);
    let mut out = ctx.source.clone();
    for y in 0..out.height as i32 {
        for x in 0..out.width as i32 {
            let here = luma(texture.get_clamped(x, y));
            let other = luma(texture.sample(x as f32 + dx, y as f32 + dy));
            let shade = (here - other) * contrast;
            let pixel = ctx.source.get(x, y);
            out.set(
                x,
                y,
                [
                    pixel[0] * (1.0 + shade),
                    pixel[1] * (1.0 + shade),
                    pixel[2] * (1.0 + shade),
                    pixel[3],
                ],
            );
        }
    }
    Ok(out)
}

pub fn threshold(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    let _ = ctx;
    Err(ApplyError::new("Threshold のしきい値の初期値は未確認"))
}

pub fn threshold_luma(source: &Image, cutoff: f32) -> Image {
    let mut out = source.clone();
    for pixel in &mut out.pixels {
        let on = luma(*pixel) >= cutoff;
        let value = if on { 1.0 } else { 0.0 };
        *pixel = [value, value, value, pixel[3]];
    }
    out
}
