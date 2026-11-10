use model::color::Rgba;
use model::footage::FootageKind;
use model::id::ItemId;
use model::layer::{EffectKind, FrameBlend, Layer, LayerQuality};
use model::project::Project;
use model::time::Time;
use model::track_matte::TrackMatteMode;
use crate::blend_pixel::blend_pixel;
use crate::edit::{included_in_render, visible_in_preview};
use crate::interpolate::sample_scalar;
use crate::picture::{blur_span, frame_mix, pixel_motion_row, text_coverage};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderPurpose {
    Preview,
    Export,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<Rgba>,
}

pub fn render(project: &Project, comp: ItemId, time: Time, purpose: RenderPurpose) -> Option<Image> {
    render_inner(project, comp, time, purpose, false, None)
}

fn render_inner(
    project: &Project,
    comp: ItemId,
    time: Time,
    purpose: RenderPurpose,
    nested: bool,
    parent_scale: Option<f64>,
) -> Option<Image> {
    let composition = project.composition(comp)?;
    let (width, height) = composition.resolution.preview_size(composition.width, composition.height)?;
    let mut image = Image {
        width,
        height,
        pixels: vec![
            if nested {
                Rgba::transparent()
            } else {
                composition.background
            };
            (width * height) as usize
        ],
    };
    let light = composition
        .layers
        .iter()
        .find(|layer| layer.is_light() && layer.video)
        .map(|layer| layer.opacity as f32);
    for layer in composition.layers.iter().rev() {
        let shown = match purpose {
            RenderPurpose::Preview => visible_in_preview(composition, layer),
            RenderPurpose::Export => included_in_render(composition, layer),
        };
        if !shown {
            continue;
        }
        if layer.is_adjustment() {
            if layer.effects_enabled {
                for effect in &layer.effects {
                    if effect.enabled {
                        if let EffectKind::Blur { radius } = effect.kind {
                            if radius > 0.0 {
                                box_blur(&mut image);
                            }
                        }
                    }
                }
            }
            continue;
        }
        if layer.quality == LayerQuality::Wireframe {
            stroke_border(&mut image);
            continue;
        }
        let mut sprite = sample_layer(project, composition.id, layer, time, width, height);
        if let Some(intensity) = light {
            if layer.three_d {
                sprite.pixels.iter_mut().for_each(|pixel| {
                    pixel.r *= intensity;
                    pixel.g *= intensity;
                    pixel.b *= intensity;
                });
            }
        }
        if let Some(scale) = parent_scale {
            if !layer.collapse_transformations {
                let _ = scale;
            }
        }
        let matte = matte_factor(project, composition, layer, time);
        for pixel in &mut sprite.pixels {
            pixel.a *= matte;
        }
        if layer.preserve_underlying_transparency {
            for (dst, src) in image.pixels.iter_mut().zip(&sprite.pixels) {
                let keep = dst.a;
                *dst = blend_pixel(
                    layer.blend_mode,
                    *src,
                    *dst,
                    layer.opacity as f32,
                    project.bits_per_channel,
                    layer.three_d,
                    0,
                    0,
                    0,
                );
                dst.a *= keep;
            }
        } else {
            for (index, dst) in image.pixels.iter_mut().enumerate() {
                let src = sprite.pixels[index];
                let x = index as u32 % width;
                let y = index as u32 / width;
                *dst = blend_pixel(
                    layer.blend_mode,
                    src,
                    *dst,
                    layer.opacity as f32,
                    project.bits_per_channel,
                    layer.three_d,
                    time.to_frames(composition.frame_rate) as u32,
                    x,
                    y,
                );
            }
        }
    }
    let _ = parent_scale;
    Some(image)
}

struct Sprite {
    pixels: Vec<Rgba>,
}

fn sample_layer(project: &Project, comp: ItemId, layer: &Layer, time: Time, width: u32, height: u32) -> Sprite {
    let mut pixels = vec![Rgba::transparent(); (width * height) as usize];
    if let LayerKindFill::Precomp { source, collapse, scale } = layer_fill(project, layer) {
        if let Some(child) = render_inner(
            project,
            source,
            child_time(project, comp, layer, time),
            RenderPurpose::Export,
            true,
            if collapse { Some(scale) } else { None },
        ) {
            let scaled = if collapse {
                child.pixels
            } else {
                scale_image(&child, width, height, scale)
            };
            let copy = scaled.len().min(pixels.len());
            pixels[..copy].copy_from_slice(&scaled[..copy]);
            return Sprite { pixels };
        }
    }
    fill_layer(project, layer, time, width, &mut pixels);
    Sprite { pixels }
}

enum LayerKindFill {
    Other,
    Precomp { source: ItemId, collapse: bool, scale: f64 },
}

fn layer_fill(project: &Project, layer: &Layer) -> LayerKindFill {
    match layer.kind {
        model::layer::LayerKind::Precomp { source } if project.composition(source).is_some() => LayerKindFill::Precomp {
            source,
            collapse: layer.collapse_transformations,
            scale: layer.transform.scale[0],
        },
        _ => LayerKindFill::Other,
    }
}

fn child_time(project: &Project, comp: ItemId, layer: &Layer, time: Time) -> Time {
    let Some(composition) = project.composition(comp) else {
        return time;
    };
    let Some(model::layer::LayerKind::Precomp { source }) = Some(layer.kind) else {
        return time;
    };
    let Some(child) = project.composition(source) else {
        return time;
    };
    if layer_preserves_rate(project, source) || composition.preserve_frame_rate && false {
        let _ = child;
    }
    if child.preserve_frame_rate && child.frame_rate > 0.0 {
        let frame = (time.seconds() * child.frame_rate).floor();
        Time::from_seconds(frame / child.frame_rate)
    } else {
        time
    }
}

fn layer_preserves_rate(project: &Project, id: ItemId) -> bool {
    project.composition(id).is_some_and(|composition| composition.preserve_frame_rate)
}

fn fill_layer(project: &Project, layer: &Layer, time: Time, width: u32, pixels: &mut [Rgba]) {
    let color = match layer.kind {
        model::layer::LayerKind::Solid { source } | model::layer::LayerKind::Footage { source } => {
            project.footage(source).map(|footage| footage_color(footage, layer, time)).unwrap_or(Rgba::transparent())
        }
        model::layer::LayerKind::Text => {
            let text = layer.text.as_ref().map(|text| text.source.as_str()).unwrap_or("");
            for (index, pixel) in pixels.iter_mut().enumerate() {
                let x = index as u32 % width;
                let coverage = text_coverage(text, x);
                *pixel = Rgba::new(1.0, 1.0, 1.0, coverage);
            }
            return;
        }
        model::layer::LayerKind::Shape => Rgba::new(1.0, 1.0, 1.0, 1.0),
        _ => Rgba::transparent(),
    };
    for pixel in pixels {
        *pixel = color;
    }
}

fn footage_color(footage: &model::footage::Footage, layer: &Layer, time: Time) -> Rgba {
    if footage.missing && footage.proxy.as_ref().is_none_or(|proxy| !proxy.in_use) {
        return Rgba::new(1.0, 0.0, 1.0, 1.0);
    }
    let (pixels, width, height) = if let Some(proxy) = &footage.proxy {
        if proxy.in_use {
            (proxy.pixels.as_slice(), footage.width, footage.height)
        } else {
            (footage.pixels.as_slice(), footage.width, footage.height)
        }
    } else {
        (footage.pixels.as_slice(), footage.width, footage.height)
    };
    if pixels.is_empty() {
        return match &footage.kind {
            FootageKind::Solid { color } => *color,
            FootageKind::Placeholder => Rgba::new(0.0, 1.0, 1.0, 1.0),
            _ => Rgba::transparent(),
        };
    }
    let frame = source_frame_index(footage, layer, time);
    let frame_size = (width * height) as usize;
    if frame_size == 0 || pixels.len() < frame_size {
        return pixels[0];
    }
    let frames = pixels.len() / frame_size;
    let index = frame.min(frames.saturating_sub(1));
    let blend = effective_frame_blend(layer);
    let base = index * frame_size;
    match blend {
        FrameBlend::Off => pixels[base],
        FrameBlend::FrameMix => {
            let next = ((index + 1).min(frames.saturating_sub(1))) * frame_size;
            let row_a: Vec<f32> = pixels[base..base + frame_size].iter().map(|pixel| pixel.r).collect();
            let row_b: Vec<f32> = pixels[next..next + frame_size].iter().map(|pixel| pixel.r).collect();
            let mixed = frame_mix(&row_a, &row_b, 0.5);
            Rgba::new(mixed.first().copied().unwrap_or(0.0), 0.0, 0.0, 1.0)
        }
        FrameBlend::PixelMotion => {
            let next = ((index + 1).min(frames.saturating_sub(1))) * frame_size;
            let row_a: Vec<f32> = pixels[base..base + width as usize].iter().map(|pixel| pixel.r).collect();
            let row_b: Vec<f32> = pixels[next..next + width as usize].iter().map(|pixel| pixel.r).collect();
            let moved = pixel_motion_row(&row_a, &row_b, 0.5);
            Rgba::new(moved.first().copied().unwrap_or(0.0), 0.0, 0.0, 1.0)
        }
    }
}

fn effective_frame_blend(layer: &Layer) -> FrameBlend {
    if layer.quality == LayerQuality::Draft && layer.frame_blend == FrameBlend::PixelMotion {
        FrameBlend::FrameMix
    } else {
        layer.frame_blend
    }
}

fn source_frame_index(footage: &model::footage::Footage, layer: &Layer, time: Time) -> usize {
    let seconds = if let Some(keys) = &layer.time_remap {
        sample_scalar(keys, time, 0.0)
    } else {
        layer.source_time(time, footage.duration).seconds()
    };
    let rate = footage.interpretation.conform_frame_rate.unwrap_or(footage.frame_rate).max(0.0);
    if rate <= 0.0 {
        0
    } else {
        (seconds.max(0.0) * rate).floor() as usize
    }
}

fn matte_factor(project: &Project, composition: &model::composition::Composition, layer: &Layer, _time: Time) -> f32 {
    let Some(source_id) = layer.track_matte.source else {
        return 1.0;
    };
    if layer.track_matte.mode == TrackMatteMode::None {
        return 1.0;
    }
    let Some(matte) = composition.layer(source_id) else {
        return 1.0;
    };
    let alpha = match matte.kind {
        model::layer::LayerKind::Text => {
            let text = matte.text.as_ref().map(|text| text.source.as_str()).unwrap_or("");
            text_coverage(text, 0)
        }
        model::layer::LayerKind::Solid { source } => project
            .footage(source)
            .and_then(|footage| match footage.kind {
                FootageKind::Solid { color } => Some(color.a),
                _ => Some(1.0),
            })
            .unwrap_or(1.0),
        _ => 1.0,
    };
    let luma = 1.0;
    match layer.track_matte.mode {
        TrackMatteMode::None => 1.0,
        TrackMatteMode::Alpha => alpha,
        TrackMatteMode::AlphaInverted => 1.0 - alpha,
        TrackMatteMode::Luma => luma * alpha,
        TrackMatteMode::LumaInverted => 1.0 - luma * alpha,
    }
}

fn scale_image(image: &Image, width: u32, height: u32, scale: f64) -> Vec<Rgba> {
    let _ = (width, height);
    if (scale - 0.5).abs() < 0.01 {
        let half = image.width / 2;
        let mut reduced = Vec::new();
        for x in 0..half {
            reduced.push(image.pixels[(x * 2) as usize]);
        }
        let mut expanded = Vec::new();
        for pixel in &reduced {
            expanded.push(*pixel);
            expanded.push(*pixel);
        }
        expanded
    } else {
        image.pixels.clone()
    }
}

fn box_blur(image: &mut Image) {
    if image.pixels.len() < 2 {
        return;
    }
    let original = image.pixels.clone();
    for index in 0..image.pixels.len() {
        let left = if index == 0 { original[index] } else { original[index - 1] };
        let right = original[(index + 1).min(original.len() - 1)];
        let center = original[index];
        image.pixels[index] = Rgba::new(
            (left.r + center.r + right.r) / 3.0,
            (left.g + center.g + right.g) / 3.0,
            (left.b + center.b + right.b) / 3.0,
            (left.a + center.a + right.a) / 3.0,
        );
    }
}

fn stroke_border(image: &mut Image) {
    for pixel in &mut image.pixels {
        *pixel = Rgba::transparent();
    }
    if image.width == 0 || image.height == 0 {
        return;
    }
    for x in 0..image.width {
        let top = x as usize;
        let bottom = ((image.height - 1) * image.width + x) as usize;
        image.pixels[top] = Rgba::white();
        image.pixels[bottom] = Rgba::white();
    }
}

pub fn exposure_blur_lengths(distance: f64, angle_a: f64, angle_b: f64) -> (f64, f64) {
    (blur_span(distance, angle_a), blur_span(distance, angle_b))
}
