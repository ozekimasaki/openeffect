use model::{BitsPerChannel, BlendMode};

/// 色相・彩度・輝度とルママットに使う係数。W3C Compositing の輝度。
/// ヘルプは係数を書いていない。
pub(crate) fn luminance(rgb: [f32; 3]) -> f32 {
    0.3 * rgb[0] + 0.59 * rgb[1] + 0.11 * rgb[2]
}

pub(crate) fn composite_pixel(
    mode: BlendMode,
    src: [f32; 4],
    dst: [f32; 4],
    opacity: f32,
    bits: BitsPerChannel,
    three_d: bool,
    frame_index: u64,
    x: u32,
    y: u32,
) -> [f32; 4] {
    let opacity = if opacity.is_finite() {
        opacity.clamp(0.0, 1.0)
    } else {
        0.0
    };
    if opacity == 0.0 {
        return dst;
    }
    match mode {
        BlendMode::Dissolve | BlendMode::DancingDissolve if !three_d => dissolve(
            src,
            dst,
            opacity,
            x,
            y,
            frame_index,
            matches!(mode, BlendMode::DancingDissolve),
        ),
        BlendMode::Dissolve | BlendMode::DancingDissolve => over(src, dst, opacity, bits, false),
        BlendMode::StencilAlpha => {
            let strength = (src[3] * opacity).clamp(0.0, 1.0);
            [dst[0], dst[1], dst[2], (dst[3] * strength).clamp(0.0, 1.0)]
        }
        BlendMode::StencilLuma => {
            let strength =
                (luminance([src[0], src[1], src[2]]).clamp(0.0, 1.0) * opacity).clamp(0.0, 1.0);
            [dst[0], dst[1], dst[2], (dst[3] * strength).clamp(0.0, 1.0)]
        }
        BlendMode::SilhouetteAlpha => {
            let strength = (src[3] * opacity).clamp(0.0, 1.0);
            [
                dst[0],
                dst[1],
                dst[2],
                (dst[3] * (1.0 - strength)).clamp(0.0, 1.0),
            ]
        }
        BlendMode::SilhouetteLuma => {
            let strength =
                (luminance([src[0], src[1], src[2]]).clamp(0.0, 1.0) * opacity).clamp(0.0, 1.0);
            [
                dst[0],
                dst[1],
                dst[2],
                (dst[3] * (1.0 - strength)).clamp(0.0, 1.0),
            ]
        }
        BlendMode::AlphaAdd => alpha_add(src, dst, opacity, bits),
        BlendMode::LuminescentPremul => luminescent(src, dst, opacity, bits),
        BlendMode::Normal
        | BlendMode::Darken
        | BlendMode::Multiply
        | BlendMode::ColorBurn
        | BlendMode::ClassicColorBurn
        | BlendMode::LinearBurn
        | BlendMode::DarkerColor
        | BlendMode::Add
        | BlendMode::Lighten
        | BlendMode::Screen
        | BlendMode::ColorDodge
        | BlendMode::ClassicColorDodge
        | BlendMode::LinearDodge
        | BlendMode::LighterColor
        | BlendMode::Overlay
        | BlendMode::SoftLight
        | BlendMode::HardLight
        | BlendMode::LinearLight
        | BlendMode::VividLight
        | BlendMode::PinLight
        | BlendMode::HardMix
        | BlendMode::Difference
        | BlendMode::ClassicDifference
        | BlendMode::Exclusion
        | BlendMode::Subtract
        | BlendMode::Divide
        | BlendMode::Hue
        | BlendMode::Saturation
        | BlendMode::Color
        | BlendMode::Luminosity => {
            let blended = blend_rgb(mode, [src[0], src[1], src[2]], [dst[0], dst[1], dst[2]]);
            over(
                [blended[0], blended[1], blended[2], src[3]],
                dst,
                opacity,
                bits,
                allows_hdr(mode),
            )
        }
    }
}

fn allows_hdr(mode: BlendMode) -> bool {
    matches!(
        mode,
        BlendMode::Add | BlendMode::LinearDodge | BlendMode::Subtract | BlendMode::Divide
    )
}

fn dissolve(
    src: [f32; 4],
    dst: [f32; 4],
    opacity: f32,
    x: u32,
    y: u32,
    frame_index: u64,
    dancing: bool,
) -> [f32; 4] {
    if take_source(x, y, opacity, frame_index, dancing) {
        src
    } else {
        dst
    }
}

fn take_source(x: u32, y: u32, opacity: f32, frame_index: u64, dancing: bool) -> bool {
    if opacity >= 1.0 {
        return true;
    }
    if opacity <= 0.0 {
        return false;
    }
    let mut n = u64::from(x).wrapping_mul(0x9E37_79B1) ^ u64::from(y).wrapping_mul(0x85EB_CA6B);
    if dancing {
        n ^= frame_index
            .wrapping_mul(0xC2B2_AE3D)
            .wrapping_add(0x1656_67B1);
    }
    n = n.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    n ^= n >> 32;
    let unit = (n >> 40) as f32 / 16_777_216.0;
    unit < opacity
}

fn alpha_add(src: [f32; 4], dst: [f32; 4], opacity: f32, bits: BitsPerChannel) -> [f32; 4] {
    let sa = (src[3] * opacity).clamp(0.0, 1.0);
    let da = dst[3].clamp(0.0, 1.0);
    let out_a = (sa + da).clamp(0.0, 1.0);
    if out_a == 0.0 {
        return [0.0; 4];
    }
    let mut out = [0.0; 4];
    for channel in 0..3 {
        let premul = src[channel] * sa + dst[channel] * da;
        out[channel] = finish(premul / out_a, bits, false);
    }
    out[3] = out_a;
    out
}

fn luminescent(src: [f32; 4], dst: [f32; 4], opacity: f32, bits: BitsPerChannel) -> [f32; 4] {
    let sa = (src[3] * opacity).clamp(0.0, 1.0);
    let da = dst[3].clamp(0.0, 1.0);
    let out_a = sa + da * (1.0 - sa);
    if out_a == 0.0 {
        return [0.0; 4];
    }
    let mut out = [0.0; 4];
    for channel in 0..3 {
        let premul = src[channel] * opacity + dst[channel] * da * (1.0 - sa);
        out[channel] = finish(premul / out_a, bits, true);
    }
    out[3] = out_a.clamp(0.0, 1.0);
    out
}

fn over(src: [f32; 4], dst: [f32; 4], opacity: f32, bits: BitsPerChannel, hdr: bool) -> [f32; 4] {
    let sa = (src[3] * opacity).clamp(0.0, 1.0);
    let da = dst[3].clamp(0.0, 1.0);
    let out_a = sa + da * (1.0 - sa);
    if out_a == 0.0 {
        return [0.0; 4];
    }
    let mut out = [0.0; 4];
    for channel in 0..3 {
        let premul = src[channel] * sa + dst[channel] * da * (1.0 - sa);
        out[channel] = finish(premul / out_a, bits, hdr);
    }
    out[3] = out_a.clamp(0.0, 1.0);
    out
}

fn finish(value: f32, bits: BitsPerChannel, hdr: bool) -> f32 {
    if !value.is_finite() {
        return 0.0;
    }
    if hdr && matches!(bits, BitsPerChannel::Bpc32) {
        value
    } else {
        value.clamp(0.0, 1.0)
    }
}

fn blend_rgb(mode: BlendMode, src: [f32; 3], dst: [f32; 3]) -> [f32; 3] {
    match mode {
        BlendMode::Normal
        | BlendMode::Dissolve
        | BlendMode::DancingDissolve
        | BlendMode::StencilAlpha
        | BlendMode::StencilLuma
        | BlendMode::SilhouetteAlpha
        | BlendMode::SilhouetteLuma
        | BlendMode::AlphaAdd
        | BlendMode::LuminescentPremul => src,
        BlendMode::Darken => map_channels(dst, src, |cb, cs| cb.min(cs)),
        BlendMode::Multiply => map_channels(dst, src, |cb, cs| cb * cs),
        BlendMode::ColorBurn | BlendMode::ClassicColorBurn => map_channels(dst, src, color_burn),
        BlendMode::LinearBurn => map_channels(dst, src, |cb, cs| cb + cs - 1.0),
        BlendMode::DarkerColor => {
            if luminance(src) <= luminance(dst) {
                src
            } else {
                dst
            }
        }
        BlendMode::Add | BlendMode::LinearDodge => map_channels(dst, src, |cb, cs| cb + cs),
        BlendMode::Lighten => map_channels(dst, src, |cb, cs| cb.max(cs)),
        BlendMode::Screen => map_channels(dst, src, screen),
        BlendMode::ColorDodge | BlendMode::ClassicColorDodge => map_channels(dst, src, color_dodge),
        BlendMode::LighterColor => {
            if luminance(src) >= luminance(dst) {
                src
            } else {
                dst
            }
        }
        BlendMode::Overlay => map_channels(dst, src, |cb, cs| hard_light(cs, cb)),
        BlendMode::SoftLight => map_channels(dst, src, soft_light),
        BlendMode::HardLight => map_channels(dst, src, hard_light),
        BlendMode::LinearLight => map_channels(dst, src, |cb, cs| cb + 2.0 * cs - 1.0),
        BlendMode::VividLight => map_channels(dst, src, vivid_light),
        BlendMode::PinLight => map_channels(dst, src, pin_light),
        BlendMode::HardMix => {
            map_channels(
                dst,
                src,
                |cb, cs| {
                    if vivid_light(cb, cs) < 0.5 {
                        0.0
                    } else {
                        1.0
                    }
                },
            )
        }
        BlendMode::Difference | BlendMode::ClassicDifference => {
            map_channels(dst, src, |cb, cs| (cb - cs).abs())
        }
        BlendMode::Exclusion => map_channels(dst, src, |cb, cs| cb + cs - 2.0 * cb * cs),
        BlendMode::Subtract => map_channels(dst, src, |cb, cs| cb - cs),
        BlendMode::Divide => map_channels(dst, src, divide),
        BlendMode::Hue => set_lum(set_sat(src, sat(dst)), luminance(dst)),
        BlendMode::Saturation => set_lum(set_sat(dst, sat(src)), luminance(dst)),
        BlendMode::Color => set_lum(src, luminance(dst)),
        BlendMode::Luminosity => set_lum(dst, luminance(src)),
    }
}

fn map_channels(dst: [f32; 3], src: [f32; 3], mix: impl Fn(f32, f32) -> f32) -> [f32; 3] {
    [
        mix(dst[0], src[0]),
        mix(dst[1], src[1]),
        mix(dst[2], src[2]),
    ]
}

fn screen(cb: f32, cs: f32) -> f32 {
    1.0 - (1.0 - cb) * (1.0 - cs)
}

fn color_burn(cb: f32, cs: f32) -> f32 {
    if cb >= 1.0 {
        1.0
    } else if cs <= 0.0 {
        0.0
    } else {
        1.0 - ((1.0 - cb) / cs).min(1.0)
    }
}

fn color_dodge(cb: f32, cs: f32) -> f32 {
    if cb <= 0.0 {
        0.0
    } else if cs >= 1.0 {
        1.0
    } else {
        (cb / (1.0 - cs)).min(1.0)
    }
}

fn hard_light(cb: f32, cs: f32) -> f32 {
    if cs <= 0.5 {
        cb * (2.0 * cs)
    } else {
        screen(cb, 2.0 * cs - 1.0)
    }
}

fn soft_light(cb: f32, cs: f32) -> f32 {
    let raised = if cb <= 0.25 {
        ((16.0 * cb - 12.0) * cb + 4.0) * cb
    } else {
        cb.sqrt()
    };
    if cs <= 0.5 {
        cb - (1.0 - 2.0 * cs) * cb * (1.0 - cb)
    } else {
        cb + (2.0 * cs - 1.0) * (raised - cb)
    }
}

fn vivid_light(cb: f32, cs: f32) -> f32 {
    if cs <= 0.5 {
        color_burn(cb, 2.0 * cs)
    } else {
        color_dodge(cb, 2.0 * cs - 1.0)
    }
}

fn pin_light(cb: f32, cs: f32) -> f32 {
    if cs <= 0.5 {
        cb.min(2.0 * cs)
    } else {
        cb.max(2.0 * cs - 1.0)
    }
}

fn divide(cb: f32, cs: f32) -> f32 {
    if cs.abs() < 1.0e-6 {
        if cb.abs() < 1.0e-6 {
            0.0
        } else {
            cb.signum() / 1.0e-6
        }
    } else {
        cb / cs
    }
}

fn sat(color: [f32; 3]) -> f32 {
    color[0].max(color[1]).max(color[2]) - color[0].min(color[1]).min(color[2])
}

fn set_sat(color: [f32; 3], saturation: f32) -> [f32; 3] {
    let mut index = [0usize, 1, 2];
    index.sort_by(|&left, &right| {
        color[left]
            .partial_cmp(&color[right])
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut out = [0.0; 3];
    let cmin = color[index[0]];
    let cmid = color[index[1]];
    let cmax = color[index[2]];
    if cmax > cmin {
        out[index[1]] = ((cmid - cmin) * saturation) / (cmax - cmin);
        out[index[2]] = saturation;
    }
    out
}

fn set_lum(color: [f32; 3], lum: f32) -> [f32; 3] {
    let delta = lum - luminance(color);
    clip_color([color[0] + delta, color[1] + delta, color[2] + delta])
}

fn clip_color(mut color: [f32; 3]) -> [f32; 3] {
    let lum = luminance(color);
    let low = color[0].min(color[1]).min(color[2]);
    let high = color[0].max(color[1]).max(color[2]);
    if low < 0.0 {
        let denom = lum - low;
        if denom != 0.0 {
            for channel in &mut color {
                *channel = lum + (*channel - lum) * lum / denom;
            }
        }
    }
    if high > 1.0 {
        let denom = high - lum;
        if denom != 0.0 {
            for channel in &mut color {
                *channel = lum + (*channel - lum) * (1.0 - lum) / denom;
            }
        }
    }
    color
}
