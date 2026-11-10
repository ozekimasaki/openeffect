use effect_api::{ApplyError, ApplyResult};

use crate::image::Image;
use crate::ops::{
    box_blur_channel, clamp01, gray_softness, lerp, limit_choke, morph_alpha, percent, write_alpha,
};
use crate::spec::ApplyCtx;

pub fn matte_choker(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    let iterations = ctx.f64("iterations")?.round().max(0.0) as i32;
    let stages = [
        (
            ctx.f64("geometric-softness-1")? as f32,
            ctx.f64("choke-1")? as f32,
            percent(ctx.f64("gray-level-softness-1")?),
        ),
        (
            ctx.f64("geometric-softness-2")? as f32,
            ctx.f64("choke-2")? as f32,
            percent(ctx.f64("gray-level-softness-2")?),
        ),
    ];
    let mut image = ctx.source.clone();
    for _ in 0..iterations {
        for (geometric, choke, gray) in stages {
            let amount = limit_choke(choke, geometric);
            let morphed = morph_alpha(&image, amount);
            write_alpha(&mut image, &morphed);
            for pixel in &mut image.pixels {
                pixel[3] = gray_softness(pixel[3], gray);
            }
        }
    }
    Ok(image)
}

pub fn mocha_shape(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    // 処理内容は文書が書かない。画素処理は足さない。
    Ok(ctx.source.clone())
}

pub fn refine_hard_matte(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    Ok(ctx.source.clone())
}

pub fn refine_soft_matte(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    let calculate = ctx.bool("calculate-edge-details")?;
    let extra = ctx.f64("additional-edge-radius")?.round() as i32;
    let view_region = ctx.bool("view-edge-region")?;
    let smooth = ctx.f64("smooth")?.round() as i32;
    let feather = ctx.f64("feather")?.round() as i32;
    let contrast = ctx.f64("contrast")? as f32;
    let shift = ctx.f64("shift-edge")? as f32;
    let decontaminate = ctx.bool("decontaminate-edge-colors")?;
    let amount = percent(ctx.f64("decontaminate-amount")?);
    let extend = ctx.f64("extend-where-smoothed")? as f32;
    let increase = ctx.f64("increase-decontaminate-radius")?.round() as i32;
    let view_map = ctx.bool("view-map")?;
    let chatter = percent(ctx.f64("chatter-reduction")?);
    let more_blur = ctx.bool("more-motion-blur")?;
    let mut image = ctx.source.clone();
    if smooth > 0 {
        let blurred = box_blur_channel(&image, 3, smooth);
        write_alpha(&mut image, &blurred);
    }
    if feather > 0 {
        let blurred = box_blur_channel(&image, 3, feather);
        write_alpha(&mut image, &blurred);
    }
    for pixel in &mut image.pixels {
        pixel[3] = clamp01((pixel[3] - 0.5) * (1.0 + contrast) + 0.5);
    }
    let shifted = morph_alpha(&image, -shift);
    write_alpha(&mut image, &shifted);
    let mut region = vec![0.0_f32; image.pixels.len()];
    for (index, pixel) in image.pixels.iter().enumerate() {
        let soft = pixel[3] > 0.02 && pixel[3] < 0.98;
        region[index] = if calculate && soft { 1.0 } else { 0.0 };
    }
    if extra > 0 {
        let mut grown = region.clone();
        let width = image.width as i32;
        let height = image.height as i32;
        for y in 0..height {
            for x in 0..width {
                let index = (y * width + x) as usize;
                if region[index] > 0.5 {
                    continue;
                }
                for dy in -extra..=extra {
                    for dx in -extra..=extra {
                        let xx = x + dx;
                        let yy = y + dy;
                        if xx >= 0 && yy >= 0 && xx < width && yy < height {
                            let other = (yy * width + xx) as usize;
                            if region[other] > 0.5 {
                                grown[index] = 1.0;
                            }
                        }
                    }
                }
            }
        }
        region = grown;
    }
    if extend > 0.0 {
        for (flag, pixel) in region.iter_mut().zip(&image.pixels) {
            if pixel[3] > 0.02 && pixel[3] < 0.98 {
                *flag = (*flag).max(extend.clamp(0.0, 1.0));
            }
        }
    }
    if let Some(previous) = ctx.sample {
        if chatter > 0.0 {
            for (pixel, prev) in image.pixels.iter_mut().zip(&previous.pixels) {
                pixel[3] = lerp(pixel[3], prev[3], chatter.clamp(0.0, 1.0));
            }
        }
        if more_blur {
            let blurred = box_blur_channel(&image, 3, 1);
            write_alpha(&mut image, &blurred);
        }
    }
    if decontaminate {
        let before = image.clone();
        let reach = 8 + increase;
        for y in 0..image.height as i32 {
            for x in 0..image.width as i32 {
                let index = (y * image.width as i32 + x) as usize;
                if region[index] < 0.5 && image.get(x, y)[3] >= 0.98 {
                    continue;
                }
                if let Some(color) = nearest_solid(&before, x, y, reach) {
                    let pixel = image.get(x, y);
                    let mix = amount.clamp(0.0, 1.0);
                    image.set(
                        x,
                        y,
                        [
                            lerp(pixel[0], color[0], mix),
                            lerp(pixel[1], color[1], mix),
                            lerp(pixel[2], color[2], mix),
                            pixel[3],
                        ],
                    );
                }
            }
        }
    }
    if view_region {
        for (pixel, flag) in image.pixels.iter_mut().zip(&region) {
            *pixel = if *flag > 0.5 {
                [1.0, 1.0, 1.0, 1.0]
            } else {
                [0.0, 0.0, 0.0, 1.0]
            };
        }
    } else if view_map {
        for (pixel, flag) in image.pixels.iter_mut().zip(&region) {
            let value = if *flag > 0.5 { 1.0 } else { pixel[3] };
            *pixel = [value, value, value, 1.0];
        }
    }
    Ok(image)
}

fn nearest_solid(image: &Image, x: i32, y: i32, reach: i32) -> Option<[f32; 4]> {
    for radius in 1..=reach {
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let pixel = image.get(x + dx, y + dy);
                if pixel[3] >= 0.98 {
                    return Some(pixel);
                }
            }
        }
    }
    None
}

pub fn simple_choker(ctx: &ApplyCtx<'_>) -> ApplyResult<Image> {
    let view = ctx.choice_opt("view")?;
    let choke = ctx.f64("choke-matte")? as f32;
    let mut image = ctx.source.clone();
    let alpha = morph_alpha(&image, choke);
    write_alpha(&mut image, &alpha);
    let view_name = view.as_deref().unwrap_or("Final Output");
    match view_name {
        "Final Output" => Ok(image),
        "Matte" => {
            for pixel in &mut image.pixels {
                *pixel = [pixel[3], pixel[3], pixel[3], 1.0];
            }
            Ok(image)
        }
        _ => Err(ApplyError::new(format!(
            "View の選択肢に {view_name} は無い"
        ))),
    }
}
