use model::{BitsPerChannel, BlendMode, Rgba};

/// 1 画素のブレンド。色はストレート、0 が黒、1 が白。
pub fn blend_pixel(
    mode: BlendMode,
    src: Rgba,
    dst: Rgba,
    opacity: f32,
    bits: BitsPerChannel,
    is_3d: bool,
    frame: u32,
    x: u32,
    y: u32,
) -> Rgba {
    let mode = if is_3d && mode.ignored_on_3d() {
        BlendMode::Normal
    } else {
        mode
    };
    let src_a = (src.a * opacity).clamp(0.0, if bits == BitsPerChannel::Bpc32 { f32::MAX } else { 1.0 });
    match mode {
        BlendMode::Dissolve => dissolve(src, dst, src_a, false, frame, x, y),
        BlendMode::DancingDissolve => dissolve(src, dst, src_a, true, frame, x, y),
        BlendMode::Normal => over(src, dst, src_a, bits),
        BlendMode::StencilAlpha
        | BlendMode::StencilLuma
        | BlendMode::SilhouetteAlpha
        | BlendMode::SilhouetteLuma => apply_matte_mode(mode, src, dst, src_a),
        BlendMode::AlphaAdd => alpha_add(src, dst, opacity, bits),
        BlendMode::LuminescentPremul => luminescent(src, dst, opacity, bits),
        _ => {
            let mixed = Rgba::new(
                blend_channel(mode, src.r, dst.r, bits),
                blend_channel(mode, src.g, dst.g, bits),
                blend_channel(mode, src.b, dst.b, bits),
                dst.a,
            );
            let color = if matches!(
                mode,
                BlendMode::Hue | BlendMode::Saturation | BlendMode::Color | BlendMode::Luminosity | BlendMode::DarkerColor | BlendMode::LighterColor
            ) {
                blend_color_mode(mode, src, dst, bits)
            } else {
                mixed
            };
            over(color, dst, src_a, bits)
        }
    }
}

fn finish(value: f32, bits: BitsPerChannel) -> f32 {
    match bits {
        BitsPerChannel::Bpc32 => value,
        BitsPerChannel::Bpc8 | BitsPerChannel::Bpc16 => bits.quantize_unit(value),
    }
}

pub fn blend_channel(mode: BlendMode, src: f32, dst: f32, bits: BitsPerChannel) -> f32 {
    let value = match mode {
        BlendMode::Darken => src.min(dst),
        BlendMode::Multiply => src * dst,
        BlendMode::ColorBurn | BlendMode::ClassicColorBurn => color_burn(src, dst),
        BlendMode::LinearBurn => (src + dst - 1.0).max(0.0),
        BlendMode::Add | BlendMode::LinearDodge => src + dst,
        BlendMode::Lighten => src.max(dst),
        BlendMode::Screen => 1.0 - (1.0 - src) * (1.0 - dst),
        BlendMode::ColorDodge | BlendMode::ClassicColorDodge => color_dodge(src, dst),
        BlendMode::Overlay => overlay(src, dst),
        BlendMode::SoftLight => soft_light(src, dst),
        BlendMode::HardLight => hard_light(src, dst),
        BlendMode::LinearLight => dst + 2.0 * src - 1.0,
        BlendMode::VividLight => vivid_light(src, dst),
        BlendMode::PinLight => pin_light(src, dst),
        BlendMode::HardMix => {
            if vivid_light(src, dst) < 0.5 {
                0.0
            } else {
                1.0
            }
        }
        BlendMode::Difference | BlendMode::ClassicDifference => (src - dst).abs(),
        BlendMode::Exclusion => src + dst - 2.0 * src * dst,
        BlendMode::Subtract => dst - src,
        BlendMode::Divide => {
            if src == 0.0 {
                if dst == 0.0 { 0.0 } else { f32::MAX }
            } else {
                dst / src
            }
        }
        _ => src,
    };
    finish(value, bits)
}

fn color_burn(src: f32, dst: f32) -> f32 {
    if src >= 1.0 {
        dst
    } else if src <= 0.0 {
        0.0
    } else {
        1.0 - ((1.0 - dst) / src).min(1.0)
    }
}

fn color_dodge(src: f32, dst: f32) -> f32 {
    if src <= 0.0 {
        dst
    } else if src >= 1.0 {
        1.0
    } else {
        (dst / (1.0 - src)).min(1.0)
    }
}

fn overlay(src: f32, dst: f32) -> f32 {
    if dst <= 0.5 {
        2.0 * src * dst
    } else {
        1.0 - 2.0 * (1.0 - src) * (1.0 - dst)
    }
}

fn hard_light(src: f32, dst: f32) -> f32 {
    if src <= 0.5 {
        2.0 * src * dst
    } else {
        1.0 - 2.0 * (1.0 - src) * (1.0 - dst)
    }
}

fn soft_light(src: f32, dst: f32) -> f32 {
    if src <= 0.5 {
        dst - (1.0 - 2.0 * src) * dst * (1.0 - dst)
    } else {
        let d = if dst <= 0.25 {
            ((16.0 * dst - 12.0) * dst + 4.0) * dst
        } else {
            dst.sqrt()
        };
        dst + (2.0 * src - 1.0) * (d - dst)
    }
}

fn vivid_light(src: f32, dst: f32) -> f32 {
    if src <= 0.5 {
        color_burn(2.0 * src, dst)
    } else {
        color_dodge(2.0 * (src - 0.5), dst)
    }
}

/// ブレンド色が 50% より明るいかで、置き換える側を決める。
fn pin_light(src: f32, dst: f32) -> f32 {
    if src >= 0.5 {
        dst.max(2.0 * src - 1.0)
    } else {
        dst.min(2.0 * src)
    }
}

fn blend_color_mode(mode: BlendMode, src: Rgba, dst: Rgba, bits: BitsPerChannel) -> Rgba {
    let mixed = match mode {
        BlendMode::DarkerColor => {
            if src.luma() < dst.luma() { src } else { dst }
        }
        BlendMode::LighterColor => {
            if src.luma() > dst.luma() { src } else { dst }
        }
        BlendMode::Hue => set_hsl(dst, hue(src), sat(dst), lum(dst)),
        BlendMode::Saturation => set_hsl(dst, hue(dst), sat(src), lum(dst)),
        BlendMode::Color => set_hsl(dst, hue(src), sat(src), lum(dst)),
        BlendMode::Luminosity => set_hsl(dst, hue(dst), sat(dst), lum(src)),
        _ => src,
    };
    Rgba::new(
        finish(mixed.r, bits),
        finish(mixed.g, bits),
        finish(mixed.b, bits),
        mixed.a,
    )
}

fn over(src: Rgba, dst: Rgba, src_a: f32, bits: BitsPerChannel) -> Rgba {
    let out_a = src_a + dst.a * (1.0 - src_a);
    let mix = |s: f32, d: f32| {
        if out_a == 0.0 {
            0.0
        } else {
            (s * src_a + d * dst.a * (1.0 - src_a)) / out_a
        }
    };
    Rgba::new(
        finish(mix(src.r, dst.r), bits),
        finish(mix(src.g, dst.g), bits),
        finish(mix(src.b, dst.b), bits),
        finish(out_a, bits),
    )
}

fn dissolve(src: Rgba, dst: Rgba, opacity: f32, dancing: bool, frame: u32, x: u32, y: u32) -> Rgba {
    let frame = if dancing { frame } else { 0 };
    let pick = hash01(x, y, frame);
    if pick < opacity { src } else { dst }
}

fn hash01(x: u32, y: u32, frame: u32) -> f32 {
    let mut n = x.wrapping_mul(374761393).wrapping_add(y.wrapping_mul(668265263)).wrapping_add(frame.wrapping_mul(1440671));
    n = (n ^ (n >> 13)).wrapping_mul(1274126177);
    (n & 0xffff) as f32 / 65535.0
}

fn apply_matte_mode(mode: BlendMode, src: Rgba, dst: Rgba, src_a: f32) -> Rgba {
    let factor = match mode {
        BlendMode::StencilAlpha => src_a,
        BlendMode::StencilLuma => src.luma().clamp(0.0, 1.0) * src_a.max(src.a),
        BlendMode::SilhouetteAlpha => 1.0 - src_a,
        BlendMode::SilhouetteLuma => 1.0 - src.luma().clamp(0.0, 1.0),
        _ => 1.0,
    };
    Rgba::new(dst.r, dst.g, dst.b, dst.a * factor)
}

fn alpha_add(src: Rgba, dst: Rgba, opacity: f32, bits: BitsPerChannel) -> Rgba {
    let added = src.a * opacity + dst.a;
    let out_a = match bits {
        BitsPerChannel::Bpc32 => added,
        BitsPerChannel::Bpc8 | BitsPerChannel::Bpc16 => added.min(1.0),
    };
    let normal = over(src, dst, src.a * opacity, bits);
    Rgba::new(normal.r, normal.g, normal.b, finish(out_a, bits))
}

fn luminescent(src: Rgba, dst: Rgba, opacity: f32, bits: BitsPerChannel) -> Rgba {
    let normal = over(
        Rgba::new(src.r.min(src.a), src.g.min(src.a), src.b.min(src.a), src.a),
        dst,
        src.a * opacity,
        bits,
    );
    let extra = |channel: f32| (channel - src.a).max(0.0) * opacity;
    Rgba::new(
        finish(normal.r + extra(src.r), bits),
        finish(normal.g + extra(src.g), bits),
        finish(normal.b + extra(src.b), bits),
        normal.a,
    )
}

fn lum(color: Rgba) -> f32 {
    color.luma()
}

fn sat(color: Rgba) -> f32 {
    color.r.max(color.g).max(color.b) - color.r.min(color.g).min(color.b)
}

fn hue(color: Rgba) -> f32 {
    let max = color.r.max(color.g).max(color.b);
    let min = color.r.min(color.g).min(color.b);
    let span = max - min;
    if span == 0.0 {
        return 0.0;
    }
    let h = if max == color.r {
        (color.g - color.b) / span
    } else if max == color.g {
        2.0 + (color.b - color.r) / span
    } else {
        4.0 + (color.r - color.g) / span
    };
    let h = h / 6.0;
    if h < 0.0 { h + 1.0 } else { h }
}

fn set_hsl(base: Rgba, hue_value: f32, sat_value: f32, lum_value: f32) -> Rgba {
    let mut color = from_hue(hue_value);
    color = set_sat(color, sat_value);
    color = set_lum(color, lum_value);
    let _ = base;
    color
}

fn from_hue(h: f32) -> Rgba {
    let h = h.rem_euclid(1.0) * 6.0;
    let x = 1.0 - (h.rem_euclid(2.0) - 1.0).abs();
    let (r, g, b) = if h < 1.0 {
        (1.0, x, 0.0)
    } else if h < 2.0 {
        (x, 1.0, 0.0)
    } else if h < 3.0 {
        (0.0, 1.0, x)
    } else if h < 4.0 {
        (0.0, x, 1.0)
    } else if h < 5.0 {
        (x, 0.0, 1.0)
    } else {
        (1.0, 0.0, x)
    };
    Rgba::new(r, g, b, 1.0)
}

fn set_sat(color: Rgba, saturation: f32) -> Rgba {
    let mut channels = [color.r, color.g, color.b];
    let mut order = [0usize, 1, 2];
    order.sort_by(|a, b| channels[*a].partial_cmp(&channels[*b]).unwrap_or(std::cmp::Ordering::Equal));
    let min = channels[order[0]];
    let mid = channels[order[1]];
    let max = channels[order[2]];
    let span = max - min;
    if span > 0.0 {
        channels[order[1]] = ((mid - min) / span) * saturation;
        channels[order[2]] = saturation;
    } else {
        channels[order[1]] = 0.0;
        channels[order[2]] = 0.0;
    }
    channels[order[0]] = 0.0;
    Rgba::new(channels[0], channels[1], channels[2], color.a)
}

fn set_lum(color: Rgba, luminance: f32) -> Rgba {
    let delta = luminance - lum(color);
    clip_color(Rgba::new(color.r + delta, color.g + delta, color.b + delta, color.a))
}

fn clip_color(color: Rgba) -> Rgba {
    let l = lum(color);
    let mut n = color.r.min(color.g).min(color.b);
    let mut x = color.r.max(color.g).max(color.b);
    let mut out = color;
    if n < 0.0 && l != n {
        let scale = l / (l - n);
        out.r = l + (out.r - l) * scale;
        out.g = l + (out.g - l) * scale;
        out.b = l + (out.b - l) * scale;
        n = 0.0;
        x = out.r.max(out.g).max(out.b);
    }
    if x > 1.0 && x != l {
        let scale = (1.0 - l) / (x - l);
        out.r = l + (out.r - l) * scale;
        out.g = l + (out.g - l) * scale;
        out.b = l + (out.b - l) * scale;
    }
    let _ = n;
    out
}

pub fn normal_alpha(src_a: f32, dst_a: f32) -> f32 {
    src_a + dst_a * (1.0 - src_a)
}
