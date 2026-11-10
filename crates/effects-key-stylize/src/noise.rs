use effect_api::{ApplyError, ApplyResult};
use model::{BitsPerChannel, Time};

use crate::image::Image;
use crate::ops::{
    blend_mode, box_blur_channel, clamp01, fbm, hash01, hsl_to_rgb, lerp, luma, median_channel,
    mix_rgb, overflow, percent, rgb_to_hsl, value_noise,
};
use crate::spec::ApplyCtx;

pub fn add_grain(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    grain(ctx, None)
}

pub fn match_grain(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    let source = ctx.layer("noise-source-layer")?;
    let compensate = percent(ctx.f64("compensate-for-existing-noise")?);
    let source_energy = high_frequency_energy(source);
    let self_energy = high_frequency_energy(ctx.source);
    let scale = (source_energy - self_energy * compensate).max(0.0);
    grain(ctx, Some(scale))
}

fn grain(ctx: &ApplyCtx<'_>, matched_scale: Option<f32>) -> ApplyResult<Image> {
    let mode = ctx.choice("viewing-mode")?;
    let intensity = ctx.f64("intensity")? as f32;
    let size = ctx.f64("size")?.max(0.1) as f32;
    let softness = ctx.f64("softness")?.round() as i32;
    let aspect = ctx.f64("aspect-ratio")? as f32;
    let mono = ctx.bool("monochromatic")?;
    let blend = ctx.choice("blending-mode")?;
    let speed = ctx.f64("animation-speed")?;
    let seed = ctx.f64("random-seed")? as u32;
    let amount = percent(ctx.f64("blend-amount")?);
    let shadows = ctx.f64_opt("shadows")?.unwrap_or(1.0) as f32;
    let midtones = ctx.f64_opt("midtones")?.unwrap_or(1.0) as f32;
    let highlights = ctx.f64_opt("highlights")?.unwrap_or(1.0) as f32;
    let midpoint = ctx.f64("midpoint")? as f32;
    let tint_amount = percent(ctx.f64_opt("tint-amount")?.unwrap_or(0.0));
    let tint = ctx.color_opt("tint-color")?;
    let frame = noise_frame(
        ctx.time,
        speed,
        ctx.bool_opt("animate-smoothly")?.unwrap_or(false),
    );
    let mut noise = Image::new(
        ctx.source.width,
        ctx.source.height,
        ctx.source.bits,
        [0.0; 4],
    );
    let matched = matched_scale.unwrap_or(1.0);
    for y in 0..noise.height as i32 {
        for x in 0..noise.width as i32 {
            let sx = x as f32 / size / aspect.max(0.05);
            let sy = y as f32 / size;
            let n = |salt| hash01(sx.floor() as i32, sy.floor() as i32, seed, salt + frame);
            let rgb = if mono {
                let v = n(1);
                [v, v, v]
            } else {
                [n(1), n(2), n(3)]
            };
            let mut pixel = [rgb[0], rgb[1], rgb[2], 1.0];
            if let Some(color) = tint {
                let mix = tint_amount.clamp(0.0, 1.0);
                pixel[0] = pixel[0] * (1.0 - mix) + color.r * mix;
                pixel[1] = pixel[1] * (1.0 - mix) + color.g * mix;
                pixel[2] = pixel[2] * (1.0 - mix) + color.b * mix;
            }
            noise.set(x, y, pixel);
        }
    }
    if softness > 0 {
        for channel in 0..3 {
            let blurred = box_blur_channel(&noise, channel, softness);
            for (pixel, value) in noise.pixels.iter_mut().zip(&blurred) {
                pixel[channel] = *value;
            }
        }
    }
    let mut out = ctx.source.clone();
    let (rect, show_box, box_color) = preview_rect(ctx, &mode)?;
    for y in 0..out.height as i32 {
        for x in 0..out.width as i32 {
            let inside = point_in_rect(x, y, rect);
            if mode == "Preview" && !inside {
                continue;
            }
            if mode == "Blending Matte" {
                let matte = if inside { 1.0 } else { 0.0 };
                out.set(x, y, [matte, matte, matte, 1.0]);
                continue;
            }
            let src = ctx.source.get(x, y);
            let mut g = noise.get(x, y);
            let weight = tonal_weight(luma(src), midpoint, shadows, midtones, highlights);
            for channel in 0..3 {
                g[channel] = (g[channel] - 0.5) * intensity * matched * weight + 0.5;
            }
            let mut mixed = blend_mode(src, g, &blend);
            mixed[3] = src[3];
            let keep = amount * mask_luma(ctx, x, y)?;
            out.set(x, y, mix_rgb(mixed, src, keep.clamp(0.0, 1.0)));
        }
    }
    if show_box {
        draw_box(&mut out, rect, box_color);
    }
    if mode != "Preview"
        && mode != "Blending Matte"
        && mode != "Final Output"
        && mode != "Noise Samples"
        && mode != "Compensation Samples"
    {
        return Err(ApplyError::new(format!(
            "Viewing Mode の選択肢に {mode} は無い"
        )));
    }
    Ok(out)
}

fn noise_frame(time: Time, speed: f64, smooth_anim: bool) -> u32 {
    if speed == 0.0 {
        return 0;
    }
    let frames = time.seconds() * speed;
    if smooth_anim && speed < 1.0 {
        (frames * 8.0).floor() as u32
    } else {
        frames.floor() as u32
    }
}

fn tonal_weight(luma: f32, midpoint: f32, shadows: f32, midtones: f32, highlights: f32) -> f32 {
    if luma < midpoint {
        let t = if midpoint <= 1e-4 {
            0.0
        } else {
            luma / midpoint
        };
        shadows + (midtones - shadows) * t
    } else {
        let t = if (1.0 - midpoint) <= 1e-4 {
            1.0
        } else {
            (luma - midpoint) / (1.0 - midpoint)
        };
        midtones + (highlights - midtones) * t
    }
}

fn preview_rect(ctx: &ApplyCtx<'_>, mode: &str) -> ApplyResult<(Option<[i32; 4]>, bool, [f32; 4])> {
    let show = ctx.bool_opt("show-box")?.unwrap_or(false);
    let color = ctx
        .color_opt("box-color")?
        .map(|color| [color.r, color.g, color.b, color.a])
        .unwrap_or([1.0, 1.0, 1.0, 1.0]);
    if mode != "Preview" {
        return Ok((None, show, color));
    }
    let center = ctx.point("center")?;
    let width = ctx.f64("width")? as f32;
    let height = ctx.f64("height")? as f32;
    let x0 = (center[0] as f32 - width * 0.5).floor() as i32;
    let y0 = (center[1] as f32 - height * 0.5).floor() as i32;
    Ok((
        Some([x0, y0, x0 + width as i32, y0 + height as i32]),
        show,
        color,
    ))
}

fn point_in_rect(x: i32, y: i32, rect: Option<[i32; 4]>) -> bool {
    let Some([x0, y0, x1, y1]) = rect else {
        return true;
    };
    x >= x0 && y >= y0 && x < x1 && y < y1
}

fn mask_luma(ctx: &ApplyCtx<'_>, x: i32, y: i32) -> ApplyResult<f32> {
    if ctx.def.param("mask-layer").is_none() {
        return Ok(1.0);
    }
    match ctx.layers.get("mask-layer") {
        Some(mask) => Ok(luma(mask.get_clamped(x, y))),
        None => Ok(1.0),
    }
}

fn draw_box(image: &mut Image, rect: Option<[i32; 4]>, color: [f32; 4]) {
    let Some([x0, y0, x1, y1]) = rect else {
        return;
    };
    for y in y0..y1 {
        image.set(x0, y, color);
        image.set(x1 - 1, y, color);
    }
    for x in x0..x1 {
        image.set(x, y0, color);
        image.set(x, y1 - 1, color);
    }
}

fn high_frequency_energy(image: &Image) -> f32 {
    if image.pixels.len() < 2 {
        return 0.0;
    }
    let mut energy = 0.0;
    for y in 0..image.height as i32 {
        for x in 0..image.width as i32 {
            let here = luma(image.get(x, y));
            let next = luma(image.get_clamped(x + 1, y));
            energy += (here - next).abs();
        }
    }
    energy / image.pixels.len() as f32
}

pub fn dust_and_scratches(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    let radius = ctx.f64("radius")?.round().max(0.0) as i32;
    let threshold = ctx.f64("threshold")? as f32;
    let mut out = ctx.source.clone();
    if radius == 0 {
        return Ok(out);
    }
    for channel in 0..3 {
        let median = median_channel(ctx.source, channel, radius);
        for (index, pixel) in out.pixels.iter_mut().enumerate() {
            let delta = (pixel[channel] - median[index]).abs();
            if delta > threshold {
                pixel[channel] = median[index];
            }
        }
    }
    Ok(out)
}

pub fn fractal_noise(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    layered_noise(ctx, 0.0)
}

pub fn turbulent_noise(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    let factor = ctx.f64("turbulence-factor")? as f32;
    layered_noise(ctx, factor)
}

fn layered_noise(ctx: &ApplyCtx<'_>, turbulence: f32) -> ApplyResult<Image> {
    let contrast = ctx.f64("contrast")? as f32;
    let brightness = ctx.f64_opt("brightness")?.unwrap_or(0.0) as f32;
    let invert = ctx.bool_opt("invert")?.unwrap_or(false);
    let overflow_mode = ctx
        .choice_opt("overflow")?
        .unwrap_or_else(|| "Clip".to_string());
    let complexity = ctx.f64("complexity")?.round().max(1.0) as i32;
    let influence = percent(ctx.f64("sub-influence")?);
    let sub_scale = ctx.f64_opt("sub-scaling")?.map(percent).unwrap_or(1.0);
    let sub_rot = ctx.f64_opt("sub-rotation")?.unwrap_or(0.0) as f32;
    let evolution = ctx.angle("evolution")? as f32;
    let opacity = percent(ctx.f64("opacity")?);
    let size = ctx.f64_opt("size")?.unwrap_or(100.0) as f32;
    let seed = ctx.f64_opt("random-seed")?.unwrap_or(0.0) as u32;
    let blend = ctx.choice_opt("blending-mode")?;
    let mut out = ctx.source.clone();
    let scale = (size / 100.0).max(0.01);
    for y in 0..out.height as i32 {
        for x in 0..out.width as i32 {
            let mut n = fbm(
                x as f32 * scale,
                y as f32 * scale,
                seed,
                complexity,
                influence,
                sub_scale.max(0.05),
                sub_rot,
                evolution.to_radians(),
                turbulence,
            );
            n = (n - 0.5) * (contrast / 100.0) + 0.5 + brightness;
            if invert {
                n = 1.0 - n;
            }
            n = overflow(n, &overflow_mode);
            let noise = [n, n, n, 1.0];
            let src = ctx.source.get(x, y);
            let blended = if let Some(mode) = &blend {
                if mode == "None" {
                    noise
                } else {
                    blend_mode(src, noise, mode)
                }
            } else {
                noise
            };
            out.set(x, y, mix_rgb(src, blended, opacity.clamp(0.0, 1.0)));
        }
    }
    Ok(out)
}

pub fn median(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    let radius = ctx.f64("radius")?.round().max(0.0) as i32;
    median_apply(ctx, radius, false)
}

pub fn median_legacy(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    let radius = ctx.f64("radius")?.round().max(0.0) as i32;
    let alpha = ctx.bool("operate-on-alpha-channel")?;
    median_apply(ctx, radius, alpha)
}

fn median_apply(ctx: &ApplyCtx<'_>, radius: i32, alpha: bool) -> ApplyResult<Image> {
    let mut out = ctx.source.clone();
    if radius == 0 {
        return Ok(out);
    }
    let channels: &[usize] = if alpha { &[0, 1, 2, 3] } else { &[0, 1, 2] };
    for channel in channels {
        let values = median_channel(ctx.source, *channel, radius);
        for (pixel, value) in out.pixels.iter_mut().zip(&values) {
            pixel[*channel] = *value;
        }
    }
    Ok(out)
}

pub fn noise(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    let amount = ctx.f64("amount-of-noise")? as f32;
    let kind = ctx.choice("noise-type")?;
    let clipping = ctx.bool("clipping")?;
    let seed = (ctx.time.seconds() * 24.0).floor() as u32;
    let mut out = ctx.source.clone();
    for y in 0..out.height as i32 {
        for x in 0..out.width as i32 {
            let pixel = out.get(x, y);
            let colored = kind == "Use Color Noise";
            let mono = (hash01(x, y, seed, 4) - 0.5) * 2.0 * amount;
            let mut next = pixel;
            for channel in 0..3 {
                let delta = if colored {
                    (hash01(x, y, seed, channel as u32) - 0.5) * 2.0 * amount
                } else {
                    mono
                };
                next[channel] = pixel[channel] + delta;
                if clipping && ctx.bits != BitsPerChannel::Bpc32 {
                    next[channel] = clamp01(next[channel]);
                }
            }
            out.set(x, y, next);
        }
    }
    Ok(out)
}

pub fn noise_alpha(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    let kind = ctx.choice("noise")?;
    let amount = ctx.f64("amount")? as f32;
    let original = ctx.choice("original-alpha")?;
    let overflow_mode = ctx.choice("overflow")?;
    let seed = ctx.f64("random-seed")? as u32;
    let phase = ctx.f64("noise-phase")? as f32;
    let cycle = ctx.bool_opt("cycle-noise")?.unwrap_or(false);
    let revolutions = ctx.f64_opt("cycle-revolutions")?.unwrap_or(1.0) as f32;
    let mut out = ctx.source.clone();
    let time = if cycle {
        (ctx.time.seconds() as f32 / revolutions.max(0.01)).fract()
    } else {
        ctx.time.seconds() as f32
    };
    for y in 0..out.height as i32 {
        for x in 0..out.width as i32 {
            let pixel = out.get(x, y);
            let cell = match kind.as_str() {
                "Unique Random" | "Uniform Animation" => hash01(x, y, seed, (time + phase) as u32),
                "Squared Random" | "Squared Animation" => {
                    let n = hash01(x, y, seed, (time + phase) as u32);
                    n * n
                }
                _ => return Err(ApplyError::new(format!("Noise の選択肢に {kind} は無い"))),
            };
            let delta = (cell - 0.5) * 2.0 * amount;
            let combined = match original.as_str() {
                "Add" => pixel[3] + delta,
                "Clamp" => clamp01(pixel[3] + delta),
                "Scale" => pixel[3] * (0.5 + cell),
                "Edges" => {
                    if pixel[3] <= 0.02 || pixel[3] >= 0.98 {
                        pixel[3]
                    } else {
                        pixel[3] + delta
                    }
                }
                _ => {
                    return Err(ApplyError::new(format!(
                        "Original Alpha の選択肢に {original} は無い"
                    )))
                }
            };
            let alpha = match overflow_mode.as_str() {
                "Clip" => clamp01(combined),
                "Wrap" => combined.rem_euclid(1.0),
                "Wrap Back" => overflow(combined, "Wrap Back"),
                _ => {
                    return Err(ApplyError::new(format!(
                        "Overflow の選択肢に {overflow_mode} は無い"
                    )))
                }
            };
            let mut next = pixel;
            next[3] = alpha;
            out.set(x, y, next);
        }
    }
    Ok(out)
}

pub fn noise_hls(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    hls_noise(ctx, ctx.f64("noise-phase")? as f32, false)
}

pub fn noise_hls_auto(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    let speed = ctx.f64("noise-animation-speed")? as f32;
    hls_noise(ctx, ctx.time.seconds() as f32 * speed, true)
}

fn hls_noise(ctx: &ApplyCtx<'_>, phase: f32, auto: bool) -> ApplyResult<Image> {
    let kind = ctx.choice("noise")?;
    let hue_amt = ctx.f64("hue")? as f32;
    let light_amt = ctx.f64("lightness")? as f32;
    let sat_amt = ctx.f64("saturation")? as f32;
    let grain = if kind == "Grain" {
        ctx.f64("grain-size")?.max(1.0) as f32
    } else {
        1.0
    };
    let mut out = ctx.source.clone();
    let salt = if auto { 9 } else { phase.floor() as u32 };
    for y in 0..out.height as i32 {
        for x in 0..out.width as i32 {
            let pixel = out.get(x, y);
            let (nx, ny) = if kind == "Grain" {
                (
                    (x as f32 / grain).floor() as i32,
                    (y as f32 / grain).floor() as i32,
                )
            } else {
                (x, y)
            };
            let n = match kind.as_str() {
                "Uniform" | "Grain" => hash01(nx, ny, salt, 3) - 0.5,
                "Squared" => {
                    let v = hash01(nx, ny, salt, 3);
                    v * v - 0.5
                }
                _ => return Err(ApplyError::new(format!("Noise の選択肢に {kind} は無い"))),
            };
            let hsl = rgb_to_hsl(pixel[0], pixel[1], pixel[2]);
            let rgb = hsl_to_rgb(
                hsl[0] + n * hue_amt,
                clamp01(hsl[1] + n * sat_amt),
                clamp01(hsl[2] + n * light_amt),
            );
            out.set(x, y, [rgb[0], rgb[1], rgb[2], pixel[3]]);
        }
    }
    Ok(out)
}

pub fn remove_grain(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    let amount = percent(ctx.f64("noise-reduction")?);
    let passes = ctx.f64("passes")?.round().max(1.0) as i32;
    let texture = percent(ctx.f64("texture")?);
    let mut image = ctx.source.clone();
    for _ in 0..passes {
        for channel in 0..3 {
            let blurred = box_blur_channel(&image, channel, 1);
            for (pixel, value) in image.pixels.iter_mut().zip(&blurred) {
                let kept = lerp(*value, pixel[channel], texture.clamp(0.0, 1.0));
                pixel[channel] = lerp(pixel[channel], kept, amount.clamp(0.0, 1.0));
            }
        }
    }
    Ok(image)
}

pub fn curl_noise(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    let input = ctx.choice("input-noise")?;
    let scale = ctx.f64_opt("scale")?.unwrap_or(100.0) as f32 / 100.0;
    let evolution = ctx.angle("evolution")? as f32;
    let turb = ctx.f64("turbulence-speed")? as f32;
    let swirl = ctx.f64("swirl")? as f32;
    let density = ctx.f64_opt("density")?.unwrap_or(0.0) as f32 / 100.0;
    let vertical = percent(ctx.f64_opt("vertical-bias")?.unwrap_or(0.0));
    let samples = ctx.f64_opt("sample-count")?.unwrap_or(3.0).round() as i32;
    let radius = ctx.f64_opt("sample-radius")?.unwrap_or(0.0) as f32;
    let view = ctx
        .choice_opt("view")?
        .unwrap_or_else(|| "Final Render".to_string());
    let field = curl_field(ctx, &input, scale, evolution, turb)?;
    let mut out = ctx.source.clone();
    for y in 0..out.height as i32 {
        for x in 0..out.width as i32 {
            let (vx, vy) = field_at(&field, out.width, x, y);
            let angle = swirl.to_radians();
            let rx = vx * angle.cos() - vy * angle.sin();
            let ry = vx * angle.sin() + vy * angle.cos();
            let vx = rx * (1.0 - vertical);
            let vy = ry * vertical.max(0.05);
            let mut acc = ctx.source.get(x, y);
            if radius > 0.0 && view != "Input Noise" {
                let mut sum = [0.0; 4];
                let count = samples.clamp(1, 24);
                for step in 0..count {
                    let t = (step as f32 + 1.0) / count as f32;
                    let sample = ctx
                        .source
                        .sample(x as f32 + vx * radius * t, y as f32 + vy * radius * t);
                    for channel in 0..4 {
                        sum[channel] += sample[channel];
                    }
                }
                for channel in 0..4 {
                    acc[channel] = sum[channel] / count as f32;
                }
            }
            let shown = match view.as_str() {
                "Input Noise" => {
                    let n = (vx + 1.0) * 0.5;
                    [n, n, n, 1.0]
                }
                "Curl Generation" => {
                    let n = (vy + 1.0) * 0.5;
                    [n, n, n, 1.0]
                }
                "Final Render" => {
                    let boost = 1.0 + density * (vx.abs() + vy.abs());
                    [acc[0] * boost, acc[1] * boost, acc[2] * boost, acc[3]]
                }
                _ => return Err(ApplyError::new(format!("View の選択肢に {view} は無い"))),
            };
            out.set(x, y, shown);
        }
    }
    Ok(out)
}

fn curl_field(
    ctx: &ApplyCtx<'_>,
    input: &str,
    scale: f32,
    evolution: f32,
    turb: f32,
) -> ApplyResult<Vec<[f32; 2]>> {
    let source = match input {
        "Internal" => None,
        "This Layer" => Some(ctx.source),
        "Other Layer" => Some(ctx.layer("other-layer")?),
        _ => {
            return Err(ApplyError::new(format!(
                "Input Noise の選択肢に {input} は無い"
            )))
        }
    };
    let mut field = Vec::with_capacity(ctx.source.pixels.len());
    let phase = evolution.to_radians() + turb * ctx.time.seconds() as f32 * 0.01;
    for y in 0..ctx.source.height as i32 {
        for x in 0..ctx.source.width as i32 {
            let n = if let Some(image) = source {
                luma(image.get(x, y))
            } else {
                value_noise(x as f32 * scale + phase, y as f32 * scale, 11)
            };
            let nx = if let Some(image) = source {
                luma(image.get_clamped(x + 1, y)) - luma(image.get_clamped(x - 1, y))
            } else {
                value_noise((x + 1) as f32 * scale + phase, y as f32 * scale, 11) - n
            };
            let ny = if let Some(image) = source {
                luma(image.get_clamped(x, y + 1)) - luma(image.get_clamped(x, y - 1))
            } else {
                value_noise(x as f32 * scale + phase, (y + 1) as f32 * scale, 11) - n
            };
            field.push([ny, -nx]);
        }
    }
    Ok(field)
}

fn field_at(field: &[[f32; 2]], width: u32, x: i32, y: i32) -> (f32, f32) {
    let index = y as usize * width as usize + x as usize;
    (field[index][0], field[index][1])
}
