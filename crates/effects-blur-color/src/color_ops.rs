//! Color Correction の CPU 処理。初期値が要確認の数値は引数で受け、埋めない。

use effect_api::ApplyResult;

use crate::blur_ops::{placed, MapFit};
use crate::color_math::{
    clamp01, cmyk_to_rgb, hsl_to_rgb, ire_from_linear, lerp, luma, phase_shift, primary_weight,
    rgb_to_cmyk, rgb_to_hsl, srgb_to_linear, Curve, Overflow, Primary,
};
use crate::host::{ArbitraryMap, CycleStop, StabilizeRef};
use crate::image::Image;

pub struct AutoInput<'a> {
    pub black_clip: f32,
    pub white_clip: f32,
    pub snap_neutral: bool,
    pub per_channel: bool,
    pub before: &'a [Image],
    pub after: &'a [Image],
    pub temporal_seconds: f32,
    pub frame_seconds: Option<f64>,
    pub scene_detect: bool,
    pub scene_cut: &'a [bool],
    pub blend_with_original: f32,
}

pub fn auto_tone(src: &Image, input: &AutoInput<'_>) -> ApplyResult<Image> {
    let (black, white) = if input.temporal_seconds <= 0.0 {
        range_of(src, input.black_clip, input.white_clip, input.per_channel)
    } else {
        let Some(frame_seconds) = input.frame_seconds else {
            return Err(effect_api::ApplyError::new("フレーム間隔が無い"));
        };
        let window = if frame_seconds <= 0.0 {
            0
        } else {
            (f64::from(input.temporal_seconds) / frame_seconds).round() as usize
        };
        let mut blacks = Vec::new();
        let mut whites = Vec::new();
        let (b, w) = range_of(src, input.black_clip, input.white_clip, input.per_channel);
        blacks.push(b);
        whites.push(w);
        for (index, frame) in input.before.iter().rev().take(window).enumerate() {
            if input.scene_detect && input.scene_cut.get(index).copied().unwrap_or(false) {
                continue;
            }
            let (b, w) = range_of(frame, input.black_clip, input.white_clip, input.per_channel);
            blacks.push(b);
            whites.push(w);
        }
        for (index, frame) in input.after.iter().take(window).enumerate() {
            if input.scene_detect
                && input
                    .scene_cut
                    .get(input.before.len() + index)
                    .copied()
                    .unwrap_or(false)
            {
                continue;
            }
            let (b, w) = range_of(frame, input.black_clip, input.white_clip, input.per_channel);
            blacks.push(b);
            whites.push(w);
        }
        (average3(&blacks), average3(&whites))
    };
    let mut out = src.map(|_, _, pixel| {
        let mut rgb = [pixel[0], pixel[1], pixel[2]];
        if input.per_channel {
            for channel in 0..3 {
                rgb[channel] = stretch(rgb[channel], black[channel], white[channel]);
            }
        } else {
            let y = luma(rgb[0], rgb[1], rgb[2]);
            let mapped = stretch(y, black[0], white[0]);
            if y > 1.0e-5 {
                let scale = mapped / y;
                for channel in 0..3 {
                    rgb[channel] *= scale;
                }
            } else {
                rgb = [mapped, mapped, mapped];
            }
        }
        [rgb[0], rgb[1], rgb[2], pixel[3]]
    });
    if input.snap_neutral {
        let mean = mean_rgb(&out);
        let gray = (mean[0] + mean[1] + mean[2]) / 3.0;
        out = out.map(|_, _, pixel| {
            let mut rgb = [pixel[0], pixel[1], pixel[2]];
            for channel in 0..3 {
                if mean[channel] > 1.0e-5 {
                    rgb[channel] *= gray / mean[channel];
                }
            }
            [rgb[0], rgb[1], rgb[2], pixel[3]]
        });
    }
    Ok(blend_original(src, &out, input.blend_with_original))
}

fn range_of(
    src: &Image,
    black_clip: f32,
    white_clip: f32,
    per_channel: bool,
) -> ([f32; 3], [f32; 3]) {
    if per_channel {
        let mut black = [0.0; 3];
        let mut white = [1.0; 3];
        for channel in 0..3 {
            let (lo, hi) = percentile(src, channel, black_clip, white_clip, false);
            black[channel] = lo;
            white[channel] = hi;
        }
        (black, white)
    } else {
        let (lo, hi) = percentile(src, 0, black_clip, white_clip, true);
        ([lo, lo, lo], [hi, hi, hi])
    }
}

fn percentile(
    src: &Image,
    channel: usize,
    black_clip: f32,
    white_clip: f32,
    use_luma: bool,
) -> (f32, f32) {
    let mut values = Vec::new();
    for y in 0..src.height {
        for x in 0..src.width {
            let pixel = src.get(x, y);
            if src.channels >= 4 && pixel[3] <= 0.0 {
                continue;
            }
            let value = if use_luma {
                luma(pixel[0], pixel[1], pixel[2])
            } else {
                pixel[channel]
            };
            values.push(value);
        }
    }
    if values.is_empty() {
        return (0.0, 1.0);
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let last = values.len() - 1;
    let black_index = ((values.len() as f32) * black_clip.clamp(0.0, 0.49)) as usize;
    let white_index =
        last.saturating_sub(((values.len() as f32) * white_clip.clamp(0.0, 0.49)) as usize);
    let lo = values[black_index.min(last)];
    let hi = values[white_index.max(black_index).min(last)];
    if hi <= lo {
        (lo, lo + 1.0e-3)
    } else {
        (lo, hi)
    }
}

fn stretch(value: f32, black: f32, white: f32) -> f32 {
    if white <= black {
        return value;
    }
    ((value - black) / (white - black)).clamp(0.0, 1.0)
}

fn average3(values: &[[f32; 3]]) -> [f32; 3] {
    let mut acc = [0.0; 3];
    for value in values {
        for channel in 0..3 {
            acc[channel] += value[channel];
        }
    }
    let n = values.len() as f32;
    [acc[0] / n, acc[1] / n, acc[2] / n]
}

fn mean_rgb(src: &Image) -> [f32; 3] {
    let mut acc = [0.0; 3];
    let mut count = 0.0;
    for y in 0..src.height {
        for x in 0..src.width {
            let pixel = src.get(x, y);
            for channel in 0..3 {
                acc[channel] += pixel[channel];
            }
            count += 1.0;
        }
    }
    if count == 0.0 {
        [0.0; 3]
    } else {
        [acc[0] / count, acc[1] / count, acc[2] / count]
    }
}

fn blend_original(src: &Image, effect: &Image, amount: f32) -> Image {
    let amount = amount.clamp(0.0, 1.0);
    src.map(|x, y, pixel| {
        let other = effect.get(x, y);
        [
            other[0] + (pixel[0] - other[0]) * amount,
            other[1] + (pixel[1] - other[1]) * amount,
            other[2] + (pixel[2] - other[2]) * amount,
            other[3] + (pixel[3] - other[3]) * amount,
        ]
    })
}

pub fn black_white(
    src: &Image,
    weight_at_hue: impl Fn(f32) -> f32,
    tint: Option<[f32; 3]>,
) -> Image {
    src.map(|_, _, pixel| {
        let [h, s, l] = rgb_to_hsl(pixel[0], pixel[1], pixel[2]);
        let weight = weight_at_hue(h * 360.0);
        let y = (l * (1.0 + (weight - 1.0) * s)).clamp(0.0, 1.0);
        let rgb = if let Some(tint) = tint {
            [y * tint[0], y * tint[1], y * tint[2]]
        } else {
            [y, y, y]
        };
        [rgb[0], rgb[1], rgb[2], pixel[3]]
    })
}

pub fn brightness_contrast(src: &Image, brightness: f32, contrast: f32) -> Image {
    src.map(|_, _, pixel| {
        let mut out = pixel;
        for channel in 0..3 {
            let v = (pixel[channel] - 0.5) * (1.0 + contrast) + 0.5 + brightness;
            out[channel] = v;
        }
        out
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BroadcastLocale {
    Ntsc,
    Pal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorSafe {
    ReduceLuminance,
    ReduceSaturation,
    KeyOutUnsafe,
    KeyOutSafe,
}

pub fn broadcast_colors(
    src: &Image,
    locale: BroadcastLocale,
    how: ColorSafe,
    maximum_ire: f32,
) -> Image {
    let setup = match locale {
        BroadcastLocale::Ntsc => 7.5,
        BroadcastLocale::Pal => 0.0,
    };
    src.map(|_, _, pixel| {
        let mut rgb = [pixel[0], pixel[1], pixel[2]];
        let y = luma(rgb[0], rgb[1], rgb[2]);
        let ire = setup + y * (100.0 - setup);
        let limit = maximum_ire.max(setup);
        let unsafe_pixel = ire > limit || rgb.iter().any(|c| ire_from_linear(*c) > limit);
        match how {
            ColorSafe::ReduceLuminance if ire > limit => {
                let target = (limit - setup) / (100.0 - setup);
                let scale = target / y.max(1.0e-5);
                for channel in 0..3 {
                    rgb[channel] *= scale;
                }
            }
            ColorSafe::ReduceSaturation if unsafe_pixel => {
                let target = (limit / 100.0).min(y);
                for channel in 0..3 {
                    rgb[channel] = y + (rgb[channel] - y) * (target / y.max(1.0e-5)).min(1.0);
                    if ire_from_linear(rgb[channel]) > limit {
                        rgb[channel] = limit / 100.0;
                    }
                }
            }
            ColorSafe::KeyOutUnsafe => {
                return if unsafe_pixel {
                    [1.0, 1.0, 1.0, pixel[3]]
                } else {
                    [0.0, 0.0, 0.0, 0.0]
                };
            }
            ColorSafe::KeyOutSafe => {
                return if unsafe_pixel {
                    [0.0, 0.0, 0.0, 0.0]
                } else {
                    [1.0, 1.0, 1.0, pixel[3]]
                };
            }
            _ => {}
        }
        let _ = setup;
        [rgb[0], rgb[1], rgb[2], pixel[3]]
    })
}

pub struct Neutralizer {
    pub shadow: [f32; 3],
    pub mid: [f32; 3],
    pub high: [f32; 3],
    pub add_shadow: [f32; 3],
    pub add_mid: [f32; 3],
    pub add_high: [f32; 3],
    pub pinning: f32,
    pub blend: f32,
    pub black_point: f32,
    pub white_point: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NeutralView {
    Final,
    BlackToWhite,
    Stair,
}

pub fn color_neutralizer(src: &Image, input: &Neutralizer, view: NeutralView) -> Image {
    let corrected = src.map(|_, _, pixel| {
        let y = luma(pixel[0], pixel[1], pixel[2]);
        let pin = input.pinning.clamp(0.0, 1.0);
        let protected = y < pin * 0.25 || y > 1.0 - pin * 0.25;
        if protected {
            return pixel;
        }
        let (shadow_w, mid_w, high_w) = tone_weights(y, input.black_point, input.white_point);
        let mut rgb = [pixel[0], pixel[1], pixel[2]];
        for channel in 0..3 {
            let remove = input.shadow[channel] * shadow_w
                + input.mid[channel] * mid_w
                + input.high[channel] * high_w;
            let add = input.add_shadow[channel] * shadow_w
                + input.add_mid[channel] * mid_w
                + input.add_high[channel] * high_w;
            rgb[channel] = rgb[channel] - (remove - 0.5) * (shadow_w + mid_w + high_w) + add;
        }
        [rgb[0], rgb[1], rgb[2], pixel[3]]
    });
    let corrected = blend_original(src, &corrected, input.blend);
    match view {
        NeutralView::Final => corrected,
        NeutralView::BlackToWhite => src.map(|x, y, pixel| {
            let after = corrected.get(x, y);
            let d = (after[0] - pixel[0] + after[1] - pixel[1] + after[2] - pixel[2]) / 3.0;
            let g = clamp01(0.5 + d);
            [g, g, g, pixel[3]]
        }),
        NeutralView::Stair => src.map(|x, _, pixel| {
            let band = (x as f32 / src.width.max(1) as f32 * 8.0).floor() / 8.0;
            [band, band, band, pixel[3]]
        }),
    }
}

fn tone_weights(y: f32, black: f32, white: f32) -> (f32, f32, f32) {
    let y = if white > black {
        ((y - black) / (white - black)).clamp(0.0, 1.0)
    } else {
        y.clamp(0.0, 1.0)
    };
    let shadow = (1.0 - y * 2.0).clamp(0.0, 1.0);
    let high = ((y - 0.5) * 2.0).clamp(0.0, 1.0);
    let mid = 1.0 - shadow - high;
    (shadow, mid.max(0.0), high)
}

pub fn color_offset(src: &Image, phases: [f32; 3], overflow: Overflow) -> Image {
    src.map(|_, _, pixel| {
        let mut out = pixel;
        for channel in 0..3 {
            out[channel] = phase_shift(pixel[channel], phases[channel], overflow).clamp(0.0, 1.0);
        }
        out
    })
}

pub fn kernel3(
    src: &Image,
    coeff: [f32; 9],
    divider: f32,
    absolute: bool,
    blend: f32,
    bits_32: bool,
) -> ApplyResult<Image> {
    if divider == 0.0 {
        return Err(effect_api::ApplyError::new("Divider が 0"));
    }
    let filtered = src.map(|x, y, _| {
        let mut acc = [0.0; 4];
        for oy in 0..3 {
            for ox in 0..3 {
                let sample = src.sample(
                    x as i32 + ox as i32 - 1,
                    y as i32 + oy as i32 - 1,
                    crate::image::Edge::Zero,
                );
                let weight = coeff[oy * 3 + ox];
                for channel in 0..3 {
                    acc[channel] += sample[channel] * weight;
                }
            }
        }
        for channel in 0..3 {
            acc[channel] /= divider;
            if absolute {
                acc[channel] = acc[channel].abs();
            } else if !bits_32 {
                acc[channel] = acc[channel].max(0.0);
            }
        }
        acc[3] = src.get(x, y)[3];
        acc
    });
    Ok(blend_original(src, &filtered, blend))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tones {
    Duotone,
    Tritone,
    Pentone,
}

pub struct TonerColors {
    pub highlights: [f32; 3],
    pub brights: [f32; 3],
    pub midtones: [f32; 3],
    pub darktones: [f32; 3],
    pub shadows: [f32; 3],
}

pub fn toner(src: &Image, tones: Tones, colors: &TonerColors, blend: f32) -> Image {
    let mapped = src.map(|_, _, pixel| {
        let y = luma(pixel[0], pixel[1], pixel[2]).clamp(0.0, 1.0);
        let rgb = match tones {
            Tones::Duotone => lerp3(colors.shadows, colors.highlights, y),
            Tones::Tritone => {
                if y < 0.5 {
                    lerp3(colors.shadows, colors.midtones, y * 2.0)
                } else {
                    lerp3(colors.midtones, colors.highlights, (y - 0.5) * 2.0)
                }
            }
            Tones::Pentone => pentone(y, colors),
        };
        [rgb[0], rgb[1], rgb[2], pixel[3]]
    });
    blend_original(src, &mapped, blend)
}

fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        lerp(a[0], b[0], t),
        lerp(a[1], b[1], t),
        lerp(a[2], b[2], t),
    ]
}

fn pentone(y: f32, colors: &TonerColors) -> [f32; 3] {
    let stops = [
        (0.0, colors.shadows),
        (0.25, colors.darktones),
        (0.5, colors.midtones),
        (0.75, colors.brights),
        (1.0, colors.highlights),
    ];
    for pair in stops.windows(2) {
        if y <= pair[1].0 {
            let t = (y - pair[0].0) / (pair[1].0 - pair[0].0);
            return lerp3(pair[0].1, pair[1].1, t);
        }
    }
    colors.highlights
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatchSpace {
    Rgb,
    Hue,
    Chroma,
}

pub fn change_color(
    src: &Image,
    hue_deg: f32,
    lightness: f32,
    saturation: f32,
    target: [f32; 3],
    tolerance: f32,
    softness: f32,
    space: MatchSpace,
    invert_mask: bool,
    show_mask: bool,
) -> Image {
    src.map(|_, _, pixel| {
        let mut mask = color_match(pixel, target, tolerance, softness, space);
        if invert_mask {
            mask = 1.0 - mask;
        }
        if show_mask {
            return [mask, mask, mask, pixel[3]];
        }
        let shifted = shift_hsl(pixel, hue_deg * mask, lightness * mask, saturation * mask);
        [shifted[0], shifted[1], shifted[2], pixel[3]]
    })
}

fn color_match(
    pixel: [f32; 4],
    target: [f32; 3],
    tolerance: f32,
    softness: f32,
    space: MatchSpace,
) -> f32 {
    let dist = match space {
        MatchSpace::Rgb => {
            let d0 = pixel[0] - target[0];
            let d1 = pixel[1] - target[1];
            let d2 = pixel[2] - target[2];
            (d0 * d0 + d1 * d1 + d2 * d2).sqrt()
        }
        MatchSpace::Hue => {
            let h0 = rgb_to_hsl(pixel[0], pixel[1], pixel[2])[0];
            let h1 = rgb_to_hsl(target[0], target[1], target[2])[0];
            let mut d = (h0 - h1).abs();
            if d > 0.5 {
                d = 1.0 - d;
            }
            d
        }
        MatchSpace::Chroma => {
            let s0 = rgb_to_hsl(pixel[0], pixel[1], pixel[2])[1];
            let s1 = rgb_to_hsl(target[0], target[1], target[2])[1];
            (s0 - s1).abs()
        }
    };
    let tol = tolerance.max(0.0);
    let soft = softness.max(0.0);
    if dist <= tol {
        1.0
    } else if soft <= 0.0 || dist >= tol + soft {
        0.0
    } else {
        1.0 - (dist - tol) / soft
    }
}

fn shift_hsl(pixel: [f32; 4], hue_deg: f32, lightness: f32, saturation: f32) -> [f32; 3] {
    let [h, s, l] = rgb_to_hsl(pixel[0], pixel[1], pixel[2]);
    hsl_to_rgb(
        h + hue_deg / 360.0,
        (s + saturation).clamp(0.0, 1.0),
        (l + lightness).clamp(0.0, 1.0),
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeMode {
    ToColor,
    TransformingToColor,
}

pub struct ChangeTolerance {
    pub hue: f32,
    pub lightness: f32,
    pub saturation: f32,
}

pub fn change_to_color(
    src: &Image,
    from: [f32; 3],
    to: [f32; 3],
    change_hue: bool,
    change_lightness: bool,
    change_saturation: bool,
    mode: ChangeMode,
    tolerance: ChangeTolerance,
    softness: f32,
    view_matte: bool,
) -> Image {
    let from_hsl = rgb_to_hsl(from[0], from[1], from[2]);
    let to_hsl = rgb_to_hsl(to[0], to[1], to[2]);
    src.map(|_, _, pixel| {
        let hsl = rgb_to_hsl(pixel[0], pixel[1], pixel[2]);
        let hue_d = hue_distance(hsl[0], from_hsl[0]);
        let in_hue = hue_d <= tolerance.hue.max(0.0);
        let in_l = (hsl[2] - from_hsl[2]).abs() <= tolerance.lightness.max(0.0);
        let in_s = (hsl[1] - from_hsl[1]).abs() <= tolerance.saturation.max(0.0);
        let hit = in_hue && in_l && in_s;
        let mut mask = if hit { 1.0 } else { 0.0 };
        if !hit && softness > 0.0 {
            let extra = (hue_d - tolerance.hue).max(0.0) + (hsl[2] - from_hsl[2]).abs()
                - tolerance.lightness
                + (hsl[1] - from_hsl[1]).abs()
                - tolerance.saturation;
            if extra < softness {
                mask = (1.0 - extra / softness).clamp(0.0, 1.0);
            }
        }
        if view_matte {
            return [mask, mask, mask, pixel[3]];
        }
        let rgb = match mode {
            ChangeMode::ToColor => lerp3([pixel[0], pixel[1], pixel[2]], to, mask),
            ChangeMode::TransformingToColor => {
                let mut h = hsl[0];
                let mut s = hsl[1];
                let mut l = hsl[2];
                if change_hue {
                    h = lerp_hue(h, to_hsl[0], mask);
                }
                if change_saturation {
                    s = lerp(s, to_hsl[1], mask);
                }
                if change_lightness {
                    l = lerp(l, to_hsl[2], mask);
                }
                hsl_to_rgb(h, s, l)
            }
        };
        [rgb[0], rgb[1], rgb[2], pixel[3]]
    })
}

fn hue_distance(a: f32, b: f32) -> f32 {
    let mut d = (a - b).abs();
    if d > 0.5 {
        d = 1.0 - d;
    }
    d
}

fn lerp_hue(a: f32, b: f32, t: f32) -> f32 {
    let mut d = b - a;
    if d > 0.5 {
        d -= 1.0;
    } else if d < -0.5 {
        d += 1.0;
    }
    (a + d * t).rem_euclid(1.0)
}

pub fn channel_mixer(src: &Image, matrix: [[f32; 4]; 3], monochrome: bool) -> Image {
    src.map(|_, _, pixel| {
        let mut rgb = [0.0; 3];
        for out_c in 0..3 {
            rgb[out_c] = (matrix[out_c][0] * pixel[0]
                + matrix[out_c][1] * pixel[1]
                + matrix[out_c][2] * pixel[2]
                + matrix[out_c][3])
                / 100.0;
        }
        if monochrome {
            rgb[1] = rgb[0];
            rgb[2] = rgb[0];
        }
        [rgb[0], rgb[1], rgb[2], pixel[3]]
    })
}

pub struct Balance {
    pub shadow: [f32; 3],
    pub mid: [f32; 3],
    pub high: [f32; 3],
    pub preserve_luminosity: bool,
}

pub fn color_balance(src: &Image, balance: &Balance) -> Image {
    src.map(|_, _, pixel| {
        let y = luma(pixel[0], pixel[1], pixel[2]);
        let (sw, mw, hw) = tone_weights(y, 0.0, 1.0);
        let mut rgb = [pixel[0], pixel[1], pixel[2]];
        for channel in 0..3 {
            rgb[channel] += balance.shadow[channel] * sw
                + balance.mid[channel] * mw
                + balance.high[channel] * hw;
        }
        if balance.preserve_luminosity {
            let new_y = luma(rgb[0], rgb[1], rgb[2]);
            if new_y.abs() > 1.0e-5 {
                let scale = y / new_y;
                for channel in 0..3 {
                    rgb[channel] *= scale;
                }
            }
        }
        [rgb[0], rgb[1], rgb[2], pixel[3]]
    })
}

pub fn color_balance_hls(src: &Image, hue: f32, lightness: f32, saturation: f32) -> Image {
    src.map(|_, _, pixel| {
        let [h, s, l] = rgb_to_hsl(pixel[0], pixel[1], pixel[2]);
        let rgb = hsl_to_rgb(
            h + hue / 360.0,
            (s + saturation).clamp(0.0, 1.0),
            (l + lightness).clamp(0.0, 1.0),
        );
        [rgb[0], rgb[1], rgb[2], pixel[3]]
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkSample {
    AverageColor,
    AverageLuma,
}

pub fn color_link(
    src: &Image,
    source: &Image,
    sample: LinkSample,
    clip: f32,
    stencil_alpha: bool,
    opacity: f32,
    plus: bool,
) -> Image {
    let average = clipped_average(source, clip, sample);
    src.map(|_, _, pixel| {
        let tinted = match sample {
            LinkSample::AverageColor => [average[0], average[1], average[2]],
            LinkSample::AverageLuma => {
                let y = luma(pixel[0], pixel[1], pixel[2]);
                [y * average[0], y * average[1], y * average[2]]
            }
        };
        let mixed = if plus {
            [
                pixel[0] + tinted[0],
                pixel[1] + tinted[1],
                pixel[2] + tinted[2],
            ]
        } else {
            tinted
        };
        let t = opacity.clamp(0.0, 1.0);
        let rgb = lerp3([pixel[0], pixel[1], pixel[2]], mixed, t);
        let alpha = if stencil_alpha { pixel[3] } else { pixel[3] };
        [rgb[0], rgb[1], rgb[2], alpha]
    })
}

fn clipped_average(src: &Image, clip: f32, sample: LinkSample) -> [f32; 3] {
    let mut values = Vec::new();
    for y in 0..src.height {
        for x in 0..src.width {
            let pixel = src.get(x, y);
            let key = luma(pixel[0], pixel[1], pixel[2]);
            values.push((key, pixel));
        }
    }
    values.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let skip = ((values.len() as f32) * clip.clamp(0.0, 0.49)) as usize;
    let end = values.len().saturating_sub(skip);
    let slice = if skip < end {
        &values[skip..end]
    } else {
        &values
    };
    let mut acc = [0.0; 3];
    for (_, pixel) in slice {
        match sample {
            LinkSample::AverageColor | LinkSample::AverageLuma => {
                acc[0] += pixel[0];
                acc[1] += pixel[1];
                acc[2] += pixel[2];
            }
        }
    }
    let n = slice.len().max(1) as f32;
    [acc[0] / n, acc[1] / n, acc[2] / n]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StabilizeMode {
    Brightness,
    Color,
    Curves,
}

pub fn sample_region(src: &Image, point: [f32; 2], radius: f32) -> [f32; 3] {
    let r = radius.max(0.0).ceil() as i32;
    let mut acc = [0.0; 3];
    let mut count = 0.0;
    for oy in -r..=r {
        for ox in -r..=r {
            let sample = src.sample(
                point[0].round() as i32 + ox,
                point[1].round() as i32 + oy,
                crate::image::Edge::Repeat,
            );
            for channel in 0..3 {
                acc[channel] += sample[channel];
            }
            count += 1.0;
        }
    }
    [acc[0] / count, acc[1] / count, acc[2] / count]
}

pub fn set_frame(
    src: &Image,
    black: [f32; 2],
    mid: [f32; 2],
    white: [f32; 2],
    radius: f32,
) -> StabilizeRef {
    StabilizeRef {
        black: sample_region(src, black, radius),
        mid: sample_region(src, mid, radius),
        white: sample_region(src, white, radius),
    }
}

pub fn color_stabilizer(
    src: &Image,
    reference: &StabilizeRef,
    black: [f32; 2],
    mid: [f32; 2],
    white: [f32; 2],
    radius: f32,
    mode: StabilizeMode,
) -> Image {
    let now_black = sample_region(src, black, radius);
    let now_mid = sample_region(src, mid, radius);
    let now_white = sample_region(src, white, radius);
    src.map(|_, _, pixel| {
        let rgb = match mode {
            StabilizeMode::Brightness => {
                let delta = luma(reference.black[0], reference.black[1], reference.black[2])
                    - luma(now_black[0], now_black[1], now_black[2]);
                [pixel[0] + delta, pixel[1] + delta, pixel[2] + delta]
            }
            StabilizeMode::Color => remap_ends(
                pixel,
                now_black,
                now_white,
                reference.black,
                reference.white,
            ),
            StabilizeMode::Curves => {
                let mut out = [0.0; 3];
                for channel in 0..3 {
                    out[channel] = three_point(
                        pixel[channel],
                        now_black[channel],
                        now_mid[channel],
                        now_white[channel],
                        reference.black[channel],
                        reference.mid[channel],
                        reference.white[channel],
                    );
                }
                out
            }
        };
        [rgb[0], rgb[1], rgb[2], pixel[3]]
    })
}

fn remap_ends(
    pixel: [f32; 4],
    black: [f32; 3],
    white: [f32; 3],
    target_black: [f32; 3],
    target_white: [f32; 3],
) -> [f32; 3] {
    let mut out = [0.0; 3];
    for channel in 0..3 {
        let span = white[channel] - black[channel];
        let t = if span.abs() < 1.0e-5 {
            0.0
        } else {
            (pixel[channel] - black[channel]) / span
        };
        out[channel] = target_black[channel] + t * (target_white[channel] - target_black[channel]);
    }
    out
}

fn three_point(v: f32, b0: f32, m0: f32, w0: f32, b1: f32, m1: f32, w1: f32) -> f32 {
    if v <= m0 {
        let span = (m0 - b0).abs().max(1.0e-5);
        let t = (v - b0) / span;
        b1 + t * (m1 - b1)
    } else {
        let span = (w0 - m0).abs().max(1.0e-5);
        let t = (v - m0) / span;
        m1 + t * (w1 - m1)
    }
}

pub fn hue_cycle_color(phase: f32) -> [f32; 3] {
    hsl_to_rgb(phase.rem_euclid(1.0), 1.0, 0.5)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhaseSource {
    Zero,
    Hue,
    Luminance,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhaseCombine {
    Wrap,
    Clamp,
    Average,
    Screen,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModifyAttr {
    Hue,
    None,
}

pub struct ColoramaInput<'a> {
    pub phase_from: PhaseSource,
    pub add: Option<&'a Image>,
    pub add_from: PhaseSource,
    pub combine: PhaseCombine,
    pub phase_shift: f32,
    pub repetitions: f32,
    pub stops: Option<&'a [CycleStop]>,
    pub interpolate: bool,
    pub modify: ModifyAttr,
    pub modify_alpha: bool,
    pub change_empty: bool,
    pub matching_color: [f32; 3],
    pub matching_tolerance: f32,
    pub matching_softness: f32,
    pub matching_off: bool,
    pub mask: Option<&'a Image>,
    pub mask_luma: bool,
    pub composite_over: bool,
    pub blend: f32,
}

pub fn colorama(src: &Image, input: &ColoramaInput<'_>) -> Image {
    let recolored = src.map(|x, y, pixel| {
        if pixel[3] <= 0.0 && !input.change_empty {
            return pixel;
        }
        let phase = phase_of(pixel, input.phase_from);
        let phase = if let Some(add) = input.add {
            let sample = placed(add, x, y, src, MapFit::Stretch);
            let extra = phase_of(sample, input.add_from);
            combine_phase(phase, extra, input.combine)
        } else {
            phase
        };
        let phase = (phase + input.phase_shift).rem_euclid(1.0);
        let phase = (phase * input.repetitions.max(0.0)).rem_euclid(1.0);
        let cycle = cycle_color(phase, input.stops, input.interpolate);
        let mut mask = if input.matching_off {
            1.0
        } else {
            color_match(
                pixel,
                input.matching_color,
                input.matching_tolerance,
                input.matching_softness,
                MatchSpace::Rgb,
            )
        };
        if let Some(matte) = input.mask {
            let sample = placed(matte, x, y, src, MapFit::Stretch);
            let m = if input.mask_luma {
                luma(sample[0], sample[1], sample[2])
            } else {
                sample[3]
            };
            mask *= m;
        }
        let rgb = match input.modify {
            ModifyAttr::None => [pixel[0], pixel[1], pixel[2]],
            ModifyAttr::Hue => {
                let [_, s, l] = rgb_to_hsl(pixel[0], pixel[1], pixel[2]);
                let [h, _, _] = rgb_to_hsl(cycle[0], cycle[1], cycle[2]);
                hsl_to_rgb(h, s, l)
            }
        };
        let rgb = if input.modify == ModifyAttr::Hue {
            rgb
        } else if matches!(input.phase_from, PhaseSource::Luminance) || input.stops.is_some() {
            cycle
        } else {
            rgb
        };
        let rgb = lerp3([pixel[0], pixel[1], pixel[2]], rgb, mask);
        let mut alpha = pixel[3];
        if input.modify_alpha {
            alpha = cycle_alpha(phase, input.stops);
        }
        let out = [rgb[0], rgb[1], rgb[2], alpha];
        if input.composite_over {
            let a = mask;
            [
                out[0] * a + pixel[0] * (1.0 - a),
                out[1] * a + pixel[1] * (1.0 - a),
                out[2] * a + pixel[2] * (1.0 - a),
                pixel[3],
            ]
        } else {
            out
        }
    });
    blend_original(src, &recolored, input.blend)
}

fn phase_of(pixel: [f32; 4], source: PhaseSource) -> f32 {
    match source {
        PhaseSource::Zero => 0.0,
        PhaseSource::Hue => rgb_to_hsl(pixel[0], pixel[1], pixel[2])[0],
        PhaseSource::Luminance => luma(pixel[0], pixel[1], pixel[2]).clamp(0.0, 1.0),
    }
}

fn combine_phase(a: f32, b: f32, mode: PhaseCombine) -> f32 {
    match mode {
        PhaseCombine::Wrap => (a + b).rem_euclid(1.0),
        PhaseCombine::Clamp => (a + b).clamp(0.0, 1.0),
        PhaseCombine::Average => (a + b) * 0.5,
        PhaseCombine::Screen => a + b - a * b,
    }
}

fn cycle_color(phase: f32, stops: Option<&[CycleStop]>, interpolate: bool) -> [f32; 3] {
    let Some(stops) = stops else {
        return hue_cycle_color(phase);
    };
    if stops.is_empty() {
        return hue_cycle_color(phase);
    }
    if !interpolate || stops.len() == 1 {
        let mut best = &stops[0];
        let mut best_d = f32::MAX;
        for stop in stops {
            let d = (stop.position - phase).abs();
            if d < best_d {
                best_d = d;
                best = stop;
            }
        }
        return [best.red, best.green, best.blue];
    }
    let mut ordered = stops.to_vec();
    ordered.sort_by(|a, b| {
        a.position
            .partial_cmp(&b.position)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    if phase <= ordered[0].position {
        return [ordered[0].red, ordered[0].green, ordered[0].blue];
    }
    for pair in ordered.windows(2) {
        if phase <= pair[1].position {
            let span = (pair[1].position - pair[0].position).max(1.0e-5);
            let t = (phase - pair[0].position) / span;
            return lerp3(
                [pair[0].red, pair[0].green, pair[0].blue],
                [pair[1].red, pair[1].green, pair[1].blue],
                t,
            );
        }
    }
    let last = ordered.last().unwrap();
    [last.red, last.green, last.blue]
}

fn cycle_alpha(phase: f32, stops: Option<&[CycleStop]>) -> f32 {
    let Some(stops) = stops else {
        return 1.0;
    };
    if stops.is_empty() {
        return 1.0;
    }
    let mut best = stops[0].alpha;
    let mut best_d = f32::MAX;
    for stop in stops {
        let d = (stop.position - phase).abs();
        if d < best_d {
            best_d = d;
            best = stop.alpha;
        }
    }
    best
}

pub fn curves_master(src: &Image, curve: &Curve) -> Image {
    src.map(|_, _, pixel| {
        let mut out = pixel;
        for channel in 0..3 {
            out[channel] = curve.eval(pixel[channel]);
        }
        out
    })
}

pub fn curves_channel(src: &Image, channel: usize, curve: &Curve) -> Image {
    src.map(|_, _, pixel| {
        let mut out = pixel;
        if channel < 4 {
            out[channel] = curve.eval(pixel[channel]);
        }
        out
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EqualizeMode {
    Rgb,
    Brightness,
    Photoshop,
}

pub fn equalize(src: &Image, mode: EqualizeMode, amount: f32) -> Image {
    let amount = amount.clamp(0.0, 1.0);
    let luts = match mode {
        EqualizeMode::Rgb => [
            histogram_lut(src, 0, false),
            histogram_lut(src, 1, false),
            histogram_lut(src, 2, false),
        ],
        EqualizeMode::Brightness | EqualizeMode::Photoshop => {
            let lut = histogram_lut(src, 0, true);
            [lut, lut, lut]
        }
    };
    src.map(|_, _, pixel| {
        if src.channels >= 4 && pixel[3] <= 0.0 {
            return pixel;
        }
        let mut out = pixel;
        match mode {
            EqualizeMode::Rgb | EqualizeMode::Photoshop => {
                for channel in 0..3 {
                    let index = (pixel[channel].clamp(0.0, 1.0) * 255.0).round() as usize;
                    let eq = luts[channel][index.min(255)];
                    out[channel] = lerp(pixel[channel], eq, amount);
                }
            }
            EqualizeMode::Brightness => {
                let y = luma(pixel[0], pixel[1], pixel[2]);
                let index = (y.clamp(0.0, 1.0) * 255.0).round() as usize;
                let eq = luts[0][index.min(255)];
                if y > 1.0e-5 {
                    let scale = eq / y;
                    for channel in 0..3 {
                        out[channel] = lerp(pixel[channel], pixel[channel] * scale, amount);
                    }
                }
            }
        }
        out
    })
}

fn histogram_lut(src: &Image, channel: usize, use_luma: bool) -> [f32; 256] {
    let mut bins = [0u32; 256];
    let mut count = 0u32;
    for y in 0..src.height {
        for x in 0..src.width {
            let pixel = src.get(x, y);
            if src.channels >= 4 && pixel[3] <= 0.0 {
                continue;
            }
            let value = if use_luma {
                luma(pixel[0], pixel[1], pixel[2])
            } else {
                pixel[channel]
            };
            let index = (value.clamp(0.0, 1.0) * 255.0).round() as usize;
            bins[index.min(255)] += 1;
            count += 1;
        }
    }
    let mut lut = [0.0; 256];
    if count == 0 {
        for (index, slot) in lut.iter_mut().enumerate() {
            *slot = index as f32 / 255.0;
        }
        return lut;
    }
    let mut acc = 0u32;
    for (index, bin) in bins.iter().copied().enumerate() {
        acc += bin;
        lut[index] = (acc as f32 - bin as f32 * 0.5) / count as f32;
    }
    lut
}

pub fn exposure(
    src: &Image,
    stops: f32,
    offset: f32,
    gamma: f32,
    bypass_linear: bool,
    encoded: bool,
) -> Image {
    src.map(|_, _, pixel| {
        let mut rgb = [pixel[0], pixel[1], pixel[2]];
        if encoded && !bypass_linear {
            for channel in 0..3 {
                rgb[channel] = srgb_to_linear(rgb[channel]);
            }
        }
        let gain = 2.0_f32.powf(stops);
        for channel in 0..3 {
            let v = rgb[channel] * gain + offset * (1.0 - rgb[channel]);
            rgb[channel] = gamma_correct(v, gamma);
        }
        if encoded && !bypass_linear {
            for channel in 0..3 {
                rgb[channel] = crate::color_math::linear_to_srgb(rgb[channel]);
            }
        }
        [rgb[0], rgb[1], rgb[2], pixel[3]]
    })
}

pub fn gamma_correct(value: f32, gamma: f32) -> f32 {
    if gamma < 0.0 {
        return -gamma_correct(-value, -gamma);
    }
    if gamma == 0.0 {
        return 0.0;
    }
    let sign = if value < 0.0 { -1.0 } else { 1.0 };
    sign * value.abs().powf(1.0 / gamma)
}

pub fn gamma_pedestal_gain(
    src: &Image,
    black_stretch: f32,
    gamma: [f32; 3],
    pedestal: [f32; 3],
    gain: [f32; 3],
) -> Image {
    src.map(|_, _, pixel| {
        let mut out = pixel;
        for channel in 0..3 {
            let stretched = if black_stretch <= 0.0 {
                pixel[channel].max(0.0)
            } else {
                pixel[channel].max(0.0).powf(1.0 / (1.0 + black_stretch))
            };
            let curved = if gamma[channel] == 0.0 {
                0.0
            } else {
                stretched.powf(gamma[channel])
            };
            out[channel] = pedestal[channel] + (gain[channel] - pedestal[channel]) * curved;
        }
        out
    })
}

pub struct HueSat {
    pub hue: f32,
    pub saturation: f32,
    pub lightness: f32,
    pub range_center: Option<f32>,
    pub range_width: f32,
    pub range_falloff: f32,
    pub colorize: bool,
    pub colorize_hue: f32,
    pub colorize_saturation: f32,
    pub colorize_lightness: f32,
}

pub fn hue_saturation(src: &Image, input: &HueSat) -> Image {
    src.map(|_, _, pixel| {
        if input.colorize {
            let l = rgb_to_hsl(pixel[0], pixel[1], pixel[2])[2];
            let rgb = hsl_to_rgb(
                input.colorize_hue / 360.0,
                input.colorize_saturation.clamp(0.0, 1.0),
                (l + input.colorize_lightness).clamp(0.0, 1.0),
            );
            return [rgb[0], rgb[1], rgb[2], pixel[3]];
        }
        let weight = if let Some(center) = input.range_center {
            let h = rgb_to_hsl(pixel[0], pixel[1], pixel[2])[0] * 360.0;
            hue_window(h, center, input.range_width, input.range_falloff)
        } else {
            1.0
        };
        let rgb = shift_hsl(
            pixel,
            input.hue * weight,
            input.lightness / 100.0 * weight,
            input.saturation / 100.0 * weight,
        );
        [rgb[0], rgb[1], rgb[2], pixel[3]]
    })
}

fn hue_window(hue: f32, center: f32, width: f32, falloff: f32) -> f32 {
    let mut d = (hue - center).rem_euclid(360.0);
    if d > 180.0 {
        d = 360.0 - d;
    }
    let half = width.max(0.0) * 0.5;
    if d <= half {
        1.0
    } else if falloff <= 0.0 || d >= half + falloff {
        0.0
    } else {
        1.0 - (d - half) / falloff
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LeaveMatch {
    Rgb,
    Hsb,
}

pub fn leave_color(
    src: &Image,
    amount: f32,
    color: [f32; 3],
    tolerance: f32,
    softness: f32,
    matching: LeaveMatch,
) -> Image {
    let amount = amount.clamp(0.0, 1.0);
    src.map(|_, _, pixel| {
        let keep = match matching {
            LeaveMatch::Rgb => {
                let d = ((pixel[0] - color[0]).powi(2)
                    + (pixel[1] - color[1]).powi(2)
                    + (pixel[2] - color[2]).powi(2))
                .sqrt();
                if d <= tolerance {
                    1.0
                } else if softness > 0.0 && d < tolerance + softness {
                    1.0 - (d - tolerance) / softness
                } else {
                    0.0
                }
            }
            LeaveMatch::Hsb => {
                let h0 = rgb_to_hsl(pixel[0], pixel[1], pixel[2])[0];
                let h1 = rgb_to_hsl(color[0], color[1], color[2])[0];
                let d = hue_distance(h0, h1);
                if d <= tolerance {
                    1.0
                } else if softness > 0.0 && d < tolerance + softness {
                    1.0 - (d - tolerance) / softness
                } else {
                    0.0
                }
            }
        };
        let y = luma(pixel[0], pixel[1], pixel[2]);
        let t = amount * (1.0 - keep);
        [
            lerp(pixel[0], y, t),
            lerp(pixel[1], y, t),
            lerp(pixel[2], y, t),
            pixel[3],
        ]
    })
}

pub struct Levels {
    pub input_black: f32,
    pub output_black: f32,
    pub input_white: f32,
    pub output_white: f32,
    pub gamma: f32,
    pub clip_black: bool,
    pub clip_white: bool,
}

pub fn levels_channel(value: f32, levels: &Levels) -> f32 {
    let span = levels.input_white - levels.input_black;
    let t = if span.abs() < 1.0e-6 {
        0.0
    } else {
        (value - levels.input_black) / span
    };
    let mut t = if levels.gamma == 0.0 {
        0.0
    } else if t >= 0.0 {
        t.powf(1.0 / levels.gamma)
    } else {
        t
    };
    if levels.clip_black && value <= levels.input_black {
        t = 0.0;
    }
    if levels.clip_white && value >= levels.input_white {
        t = 1.0;
    }
    levels.output_black + t * (levels.output_white - levels.output_black)
}

pub fn levels(src: &Image, channel: Option<usize>, levels_in: &Levels) -> Image {
    src.map(|_, _, pixel| {
        let mut out = pixel;
        match channel {
            None => {
                for index in 0..3 {
                    out[index] = levels_channel(pixel[index], levels_in);
                }
            }
            Some(index) if index < 4 => {
                out[index] = levels_channel(pixel[index], levels_in);
            }
            Some(_) => {}
        }
        out
    })
}

pub fn histogram_counts(src: &Image, channel: Option<usize>) -> [u32; 256] {
    let mut bins = [0u32; 256];
    for y in 0..src.height {
        for x in 0..src.width {
            let pixel = src.get(x, y);
            let value = match channel {
                Some(index) => pixel[index.min(3)],
                None => luma(pixel[0], pixel[1], pixel[2]),
            };
            let index = (value.clamp(0.0, 1.0) * 255.0).round() as usize;
            bins[index.min(255)] += 1;
        }
    }
    bins
}

pub fn lumetri(
    src: &Image,
    master: &Curve,
    red: Option<&Curve>,
    green: Option<&Curve>,
    blue: Option<&Curve>,
) -> Image {
    src.map(|_, _, pixel| {
        let y = master.eval(luma(pixel[0], pixel[1], pixel[2]));
        let old = luma(pixel[0], pixel[1], pixel[2]);
        let scale = if old > 1.0e-5 { y / old } else { 0.0 };
        let mut rgb = [pixel[0] * scale, pixel[1] * scale, pixel[2] * scale];
        if let Some(curve) = red {
            rgb[0] = curve.eval(pixel[0]);
        }
        if let Some(curve) = green {
            rgb[1] = curve.eval(pixel[1]);
        }
        if let Some(curve) = blue {
            rgb[2] = curve.eval(pixel[2]);
        }
        [rgb[0], rgb[1], rgb[2], pixel[3]]
    })
}

pub fn hue_curve(src: &Image, curve: &Curve, mode: HueCurveMode) -> Image {
    src.map(|_, _, pixel| {
        let [h, s, l] = rgb_to_hsl(pixel[0], pixel[1], pixel[2]);
        let delta = curve.eval(match mode {
            HueCurveMode::HueHue | HueCurveMode::HueSat | HueCurveMode::HueLuma => h,
            HueCurveMode::SatSat => s,
            HueCurveMode::LumaSat => l,
        }) - match mode {
            HueCurveMode::HueHue | HueCurveMode::HueSat | HueCurveMode::HueLuma => h,
            HueCurveMode::SatSat => s,
            HueCurveMode::LumaSat => l,
        };
        let (h, s, l) = match mode {
            HueCurveMode::HueHue => ((h + delta).rem_euclid(1.0), s, l),
            HueCurveMode::HueSat | HueCurveMode::SatSat | HueCurveMode::LumaSat => {
                (h, (s + delta).clamp(0.0, 1.0), l)
            }
            HueCurveMode::HueLuma => (h, s, (l + delta).clamp(0.0, 1.0)),
        };
        let rgb = hsl_to_rgb(h, s, l);
        [rgb[0], rgb[1], rgb[2], pixel[3]]
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HueCurveMode {
    HueHue,
    HueSat,
    HueLuma,
    SatSat,
    LumaSat,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhotoFilterKind {
    Warming85,
    Cooling80,
    Warming81,
    Cooling82,
    Individual,
    Underwater,
}

pub fn photo_filter(
    src: &Image,
    kind: PhotoFilterKind,
    color: [f32; 3],
    density: f32,
    preserve: bool,
) -> Image {
    let filter = match kind {
        PhotoFilterKind::Warming85 => [1.0, 0.55, 0.15],
        PhotoFilterKind::Cooling80 => [0.2, 0.45, 1.0],
        PhotoFilterKind::Warming81 => [1.0, 0.8, 0.55],
        PhotoFilterKind::Cooling82 => [0.65, 0.8, 1.0],
        PhotoFilterKind::Underwater => [0.15, 0.55, 0.7],
        PhotoFilterKind::Individual => color,
    };
    let density = density.clamp(0.0, 1.0);
    src.map(|_, _, pixel| {
        let filtered = [
            pixel[0] * filter[0],
            pixel[1] * filter[1],
            pixel[2] * filter[2],
        ];
        let mut rgb = lerp3([pixel[0], pixel[1], pixel[2]], filtered, density);
        if preserve {
            let old = luma(pixel[0], pixel[1], pixel[2]);
            let new = luma(rgb[0], rgb[1], rgb[2]);
            if new > 1.0e-5 {
                let scale = old / new;
                rgb = [rgb[0] * scale, rgb[1] * scale, rgb[2] * scale];
            }
        }
        [rgb[0], rgb[1], rgb[2], pixel[3]]
    })
}

pub fn arbitrary_map(
    src: &Image,
    map: Option<&ArbitraryMap>,
    phase: f32,
    apply_alpha: bool,
) -> Image {
    src.map(|_, _, pixel| {
        let mut out = pixel;
        for channel in 0..3 {
            out[channel] = sample_map_entry(
                pixel[channel],
                map.map(|item| item.entries.as_slice()),
                phase,
            );
        }
        if apply_alpha {
            out[3] = match map.and_then(|item| item.alpha.as_ref()) {
                Some(alpha) => sample_map_entry(pixel[3], Some(alpha.as_slice()), phase),
                None => pixel[3],
            };
        }
        out
    })
}

fn sample_map_entry(value: f32, map: Option<&[f32]>, phase: f32) -> f32 {
    let Some(map) = map else {
        return value;
    };
    if map.is_empty() {
        return value;
    }
    let pos = (value.clamp(0.0, 1.0) * (map.len() - 1) as f32 - phase).rem_euclid(map.len() as f32);
    let index = pos.floor() as usize % map.len();
    let next = (index + 1) % map.len();
    let t = pos - index as f32;
    lerp(map[index], map[next], t)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectiveMethod {
    Relative,
    Absolute,
}

pub fn selective_color(
    src: &Image,
    method: SelectiveMethod,
    primary: Primary,
    delta: [f32; 4],
) -> Image {
    src.map(|_, _, pixel| {
        let weight = primary_weight([pixel[0], pixel[1], pixel[2]], primary);
        if weight <= 0.0 {
            return pixel;
        }
        let mut cmyk = rgb_to_cmyk(pixel[0], pixel[1], pixel[2]);
        for index in 0..4 {
            cmyk[index] = match method {
                SelectiveMethod::Relative => cmyk[index] + cmyk[index] * delta[index] * weight,
                SelectiveMethod::Absolute => cmyk[index] + delta[index] * weight,
            };
            cmyk[index] = cmyk[index].clamp(0.0, 1.0);
        }
        let rgb = cmyk_to_rgb(cmyk[0], cmyk[1], cmyk[2], cmyk[3]);
        [rgb[0], rgb[1], rgb[2], pixel[3]]
    })
}

pub struct ShadowHighlight {
    pub auto_amounts: bool,
    pub shadow_amount: f32,
    pub highlight_amount: f32,
    pub shadow_width: f32,
    pub highlight_width: f32,
    pub shadow_radius: f32,
    pub highlight_radius: f32,
    pub color_correction: f32,
    pub midtone_contrast: f32,
    pub black_clip: f32,
    pub white_clip: f32,
    pub blend: f32,
}

pub fn shadow_highlight(src: &Image, input: &ShadowHighlight) -> ApplyResult<Image> {
    let (auto_shadow, auto_high) = if input.auto_amounts {
        let (lo, hi) = percentile(src, 0, input.black_clip, input.white_clip, true);
        ((0.5 - lo).clamp(0.0, 1.0), (hi - 0.5).clamp(0.0, 1.0))
    } else {
        (input.shadow_amount, input.highlight_amount)
    };
    let blurred = crate::image::gaussian_blur(
        src,
        input.shadow_radius.max(input.highlight_radius).max(0.0),
        crate::image::Edge::Repeat,
    )?;
    let adjusted = src.map(|x, y, pixel| {
        let surround = luma(
            blurred.get(x, y)[0],
            blurred.get(x, y)[1],
            blurred.get(x, y)[2],
        );
        let y = luma(pixel[0], pixel[1], pixel[2]);
        let darker_than_surround = (surround - y).max(0.0).min(1.0);
        let brighter_than_surround = (y - surround).max(0.0).min(1.0);
        let shadow_w = (1.0 - y / input.shadow_width.max(1.0e-3)).clamp(0.0, 1.0)
            * (0.5 + 0.5 * darker_than_surround);
        let high_w = ((y - (1.0 - input.highlight_width)) / input.highlight_width.max(1.0e-3))
            .clamp(0.0, 1.0)
            * (0.5 + 0.5 * brighter_than_surround);
        let mut factor = 1.0 + auto_shadow * shadow_w * 4.0 - auto_high * high_w;
        let mid = (1.0 - (y - 0.5).abs() * 2.0).clamp(0.0, 1.0);
        factor += input.midtone_contrast * (y - 0.5) * mid;
        let mut rgb = [pixel[0] * factor, pixel[1] * factor, pixel[2] * factor];
        let sat = input.color_correction.clamp(0.0, 1.0) * (shadow_w + high_w).clamp(0.0, 1.0);
        let gray = luma(rgb[0], rgb[1], rgb[2]);
        for channel in 0..3 {
            rgb[channel] = lerp(gray, rgb[channel], 1.0 + sat);
        }
        [rgb[0], rgb[1], rgb[2], pixel[3]]
    });
    Ok(blend_original(src, &adjusted, input.blend))
}

pub fn tint(src: &Image, black: [f32; 3], white: [f32; 3], amount: f32) -> Image {
    let amount = amount.clamp(0.0, 1.0);
    src.map(|_, _, pixel| {
        let y = luma(pixel[0], pixel[1], pixel[2]).clamp(0.0, 1.0);
        let tinted = lerp3(black, white, y);
        let rgb = lerp3([pixel[0], pixel[1], pixel[2]], tinted, amount);
        [rgb[0], rgb[1], rgb[2], pixel[3]]
    })
}

pub fn swap_colors(black: [f32; 3], white: [f32; 3]) -> ([f32; 3], [f32; 3]) {
    (white, black)
}

pub fn tritone(src: &Image, shadow: [f32; 3], mid: [f32; 3], high: [f32; 3], blend: f32) -> Image {
    let mapped = src.map(|_, _, pixel| {
        let y = luma(pixel[0], pixel[1], pixel[2]).clamp(0.0, 1.0);
        let rgb = if y < 0.5 {
            lerp3(shadow, mid, y * 2.0)
        } else {
            lerp3(mid, high, (y - 0.5) * 2.0)
        };
        [rgb[0], rgb[1], rgb[2], pixel[3]]
    });
    blend_original(src, &mapped, blend)
}

pub fn vibrance(src: &Image, vibrance_amount: f32, saturation: f32) -> Image {
    src.map(|_, _, pixel| {
        let [h, s, l] = rgb_to_hsl(pixel[0], pixel[1], pixel[2]);
        let sat = (s + saturation).clamp(0.0, 1.0);
        let skin = skin_weight(h * 360.0);
        let boost = vibrance_amount * (1.0 - sat) * (1.0 - 0.7 * skin);
        let rgb = hsl_to_rgb(h, (sat + boost).clamp(0.0, 1.0), l);
        [rgb[0], rgb[1], rgb[2], pixel[3]]
    })
}

fn skin_weight(hue: f32) -> f32 {
    let hue = hue.rem_euclid(360.0);
    if hue >= 330.0 || hue <= 45.0 {
        1.0
    } else {
        0.0
    }
}

pub fn video_limiter(src: &Image, maximum_ire: f32) -> Image {
    src.map(|_, _, pixel| {
        let mut rgb = [pixel[0], pixel[1], pixel[2]];
        let y = luma(rgb[0], rgb[1], rgb[2]);
        let limit = maximum_ire / 100.0;
        if y > limit && y > 1.0e-5 {
            let scale = limit / y;
            for channel in 0..3 {
                rgb[channel] *= scale;
            }
        }
        for channel in 0..3 {
            if rgb[channel] > limit {
                rgb[channel] = limit;
            }
            if rgb[channel] < 0.0 {
                rgb[channel] = 0.0;
            }
        }
        [rgb[0], rgb[1], rgb[2], pixel[3]]
    })
}

pub fn linear_identity_map(size: usize) -> Vec<f32> {
    (0..size)
        .map(|index| index as f32 / (size.saturating_sub(1).max(1)) as f32)
        .collect()
}
