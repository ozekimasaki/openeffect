//! Channel の CPU 処理。

use effect_api::channel_32_to_8;
use model::Rgba;

use crate::blur_ops::{placed, MapFit};
use crate::color_math::{hsl_to_rgb, luma, rgb_to_hsl};
use crate::image::{extremum, Axes, Image};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operator {
    And,
    Or,
    Xor,
    Add,
    Subtract,
    Multiply,
    Difference,
    Max,
    Min,
    BlockAbove,
    BlockBelow,
    Slice,
    Screen,
}

pub fn arithmetic(src: &Image, op: Operator, clip: bool, operand: [u8; 3]) -> Image {
    src.map(|_, _, pixel| {
        let mut out = pixel;
        for channel in 0..3 {
            let value = channel_32_to_8(pixel[channel]);
            let mixed = apply_op(op, value, operand[channel]);
            let code = if clip {
                mixed.clamp(0, 255) as u8
            } else {
                mixed.rem_euclid(256) as u8
            };
            out[channel] = crate::image::channel_8_to_32(code);
        }
        out
    })
}

fn apply_op(op: Operator, src: u8, operand: u8) -> i32 {
    let s = i32::from(src);
    let o = i32::from(operand);
    match op {
        Operator::And => s & o,
        Operator::Or => s | o,
        Operator::Xor => s ^ o,
        Operator::Add => s + o,
        Operator::Subtract => s - o,
        Operator::Multiply => s * o,
        Operator::Difference => (s - o).abs(),
        Operator::Max => s.max(o),
        Operator::Min => s.min(o),
        Operator::BlockAbove => {
            if s > o {
                0
            } else {
                s
            }
        }
        Operator::BlockBelow => {
            if s < o {
                0
            } else {
                s
            }
        }
        Operator::Slice => {
            if s >= o {
                1
            } else {
                0
            }
        }
        Operator::Screen => s + o - (s * o) / 255,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlendMethod {
    ColorOnly,
    TintOnly,
    DarkenOnly,
    LightenOnly,
    Crossfade,
}

pub fn blend_layers(
    src: &Image,
    other: &Image,
    method: BlendMethod,
    blend_with_original: f32,
    stretch: bool,
) -> Image {
    let fit = if stretch {
        MapFit::Stretch
    } else {
        MapFit::Center
    };
    src.map(|x, y, pixel| {
        let sample = placed(other, x, y, src, fit);
        let mixed = match method {
            BlendMethod::ColorOnly => {
                let [h, s, _] = rgb_to_hsl(sample[0], sample[1], sample[2]);
                let l = luma(pixel[0], pixel[1], pixel[2]);
                let [r, g, b] = hsl_to_rgb(h, s, l);
                [r, g, b, pixel[3]]
            }
            BlendMethod::TintOnly => {
                let src_sat = rgb_to_hsl(pixel[0], pixel[1], pixel[2])[1];
                if src_sat <= 1.0e-3 {
                    pixel
                } else {
                    let [h, s, _] = rgb_to_hsl(sample[0], sample[1], sample[2]);
                    let l = luma(pixel[0], pixel[1], pixel[2]);
                    let [r, g, b] = hsl_to_rgb(h, s, l);
                    [r, g, b, pixel[3]]
                }
            }
            BlendMethod::DarkenOnly => min4(pixel, sample),
            BlendMethod::LightenOnly => max4(pixel, sample),
            BlendMethod::Crossfade => lerp4(pixel, sample, 0.5),
        };
        lerp4(mixed, pixel, blend_with_original.clamp(0.0, 1.0))
    })
}

fn min4(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [
        a[0].min(b[0]),
        a[1].min(b[1]),
        a[2].min(b[2]),
        a[3].min(b[3]),
    ]
}

fn max4(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [
        a[0].max(b[0]),
        a[1].max(b[1]),
        a[2].max(b[2]),
        a[3].max(b[3]),
    ]
}

fn lerp4(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        a[3] + (b[3] - a[3]) * t,
    ]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputChannel {
    Rgba,
    Gray,
    Red,
    Green,
    Blue,
    Alpha,
}

pub fn calculations(
    src: &Image,
    input: InputChannel,
    invert_input: bool,
    second: &Image,
    second_channel: InputChannel,
    second_opacity: f32,
    invert_second: bool,
    stretch: bool,
    preserve_transparency: bool,
) -> Image {
    let fit = if stretch {
        MapFit::Stretch
    } else {
        MapFit::Center
    };
    let opacity = second_opacity.clamp(0.0, 1.0);
    src.map(|x, y, pixel| {
        let mut a = extract(pixel, input);
        if invert_input {
            a = invert_pixel(a);
        }
        if opacity <= 0.0 {
            let mut out = a;
            if preserve_transparency {
                out[3] = pixel[3];
            }
            return out;
        }
        let sample = placed(second, x, y, src, fit);
        let mut b = extract(sample, second_channel);
        if invert_second {
            b = invert_pixel(b);
        }
        let mut out = lerp4(a, b, opacity);
        if preserve_transparency {
            out[3] = pixel[3];
        }
        out
    })
}

fn extract(pixel: [f32; 4], channel: InputChannel) -> [f32; 4] {
    match channel {
        InputChannel::Rgba => pixel,
        InputChannel::Gray => {
            let y = luma(pixel[0], pixel[1], pixel[2]);
            [y, y, y, pixel[3]]
        }
        InputChannel::Red => [pixel[0], pixel[0], pixel[0], pixel[3]],
        InputChannel::Green => [pixel[1], pixel[1], pixel[1], pixel[3]],
        InputChannel::Blue => [pixel[2], pixel[2], pixel[2], pixel[3]],
        InputChannel::Alpha => [pixel[3], pixel[3], pixel[3], pixel[3]],
    }
}

fn invert_pixel(mut pixel: [f32; 4]) -> [f32; 4] {
    for channel in 0..4 {
        pixel[channel] = 1.0 - pixel[channel];
    }
    pixel
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompositeMode {
    Copy,
    Behind,
    InFront,
}

pub fn cc_composite(
    effected: &Image,
    original: &Image,
    opacity: f32,
    mode: CompositeMode,
    rgb_only: bool,
) -> Image {
    let opacity = opacity.clamp(0.0, 1.0);
    effected.map(|x, y, effect| {
        let mut source = original.get(
            x.min(original.width.saturating_sub(1)),
            y.min(original.height.saturating_sub(1)),
        );
        if rgb_only {
            source[3] = 1.0;
        }
        source[3] *= opacity;
        match mode {
            CompositeMode::Copy => {
                let mut out = lerp4(effect, source, opacity);
                if rgb_only {
                    out[3] = effect[3];
                }
                out
            }
            CompositeMode::Behind => over(effect, source, rgb_only),
            CompositeMode::InFront => over(source, effect, rgb_only),
        }
    })
}

fn over(front: [f32; 4], back: [f32; 4], rgb_only: bool) -> [f32; 4] {
    if rgb_only {
        return [
            front[0] * front[3] + back[0] * (1.0 - front[3]),
            front[1] * front[3] + back[1] * (1.0 - front[3]),
            front[2] * front[3] + back[2] * (1.0 - front[3]),
            back[3],
        ];
    }
    let a = front[3] + back[3] * (1.0 - front[3]);
    if a <= 1.0e-6 {
        return [0.0; 4];
    }
    [
        (front[0] * front[3] + back[0] * back[3] * (1.0 - front[3])) / a,
        (front[1] * front[3] + back[1] * back[3] * (1.0 - front[3])) / a,
        (front[2] * front[3] + back[2] * back[3] * (1.0 - front[3])) / a,
        a,
    ]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombinerFrom {
    SaturationMultiplied,
    MinRgb,
    MaxRgb,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombinerTo {
    RedOnly,
    GreenOnly,
    BlueOnly,
    AlphaOnly,
    HueOnly,
    LightnessOnly,
    SaturationOnly,
}

pub fn channel_combiner(
    src: &Image,
    second: Option<&Image>,
    from: CombinerFrom,
    to: CombinerTo,
    invert: bool,
    solid_alpha: bool,
) -> Image {
    src.map(|x, y, pixel| {
        let sample = second
            .map(|image| placed(image, x, y, src, MapFit::Stretch))
            .unwrap_or(pixel);
        let mut value = match from {
            CombinerFrom::SaturationMultiplied => {
                let [h, s, l] = rgb_to_hsl(sample[0], sample[1], sample[2]);
                let _ = h;
                s * l
            }
            CombinerFrom::MinRgb => sample[0].min(sample[1]).min(sample[2]),
            CombinerFrom::MaxRgb => sample[0].max(sample[1]).max(sample[2]),
        };
        if invert {
            value = 1.0 - value;
        }
        let mut out = match to {
            CombinerTo::RedOnly => [value, 0.0, 0.0, pixel[3]],
            CombinerTo::GreenOnly => [0.0, value, 0.0, pixel[3]],
            CombinerTo::BlueOnly => [0.0, 0.0, value, pixel[3]],
            CombinerTo::AlphaOnly => [1.0, 1.0, 1.0, value],
            CombinerTo::HueOnly => {
                let [_, s, l] = rgb_to_hsl(pixel[0], pixel[1], pixel[2]);
                let [r, g, b] = hsl_to_rgb(value, s, l);
                [r, g, b, pixel[3]]
            }
            CombinerTo::LightnessOnly => [value, value, value, pixel[3]],
            CombinerTo::SaturationOnly => {
                let [h, _, l] = rgb_to_hsl(pixel[0], pixel[1], pixel[2]);
                let [r, g, b] = hsl_to_rgb(h, value, l);
                [r, g, b, pixel[3]]
            }
        };
        if solid_alpha {
            out[3] = 1.0;
        }
        out
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverflowBehavior {
    Clip,
    Wrap,
    Scale,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Formula {
    Sum,
    Product,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChannelMask {
    Red,
    Green,
    Blue,
    Rgb,
}

pub fn compound_arithmetic(
    src: &Image,
    second: &Image,
    formula: Formula,
    channels: ChannelMask,
    overflow: OverflowBehavior,
    stretch: bool,
    blend_with_original: f32,
) -> Image {
    let fit = if stretch {
        MapFit::Stretch
    } else {
        MapFit::TopLeft
    };
    let mut raw = Vec::with_capacity(src.pixel_count() * 3);
    for y in 0..src.height {
        for x in 0..src.width {
            let a = src.get(x, y);
            let b = placed(second, x, y, src, fit);
            for channel in 0..3 {
                let value = match formula {
                    Formula::Sum => a[channel] + b[channel],
                    Formula::Product => a[channel] * b[channel],
                };
                raw.push(value);
            }
        }
    }
    let max = raw.iter().copied().fold(0.0_f32, f32::max).max(1.0e-6);
    let mut index = 0;
    let blended = src.map(|_, _, pixel| {
        let mut out = pixel;
        for channel in 0..3 {
            let value = raw[index];
            index += 1;
            if !channel_enabled(channels, channel) {
                continue;
            }
            out[channel] = match overflow {
                OverflowBehavior::Clip => value.clamp(0.0, 1.0),
                OverflowBehavior::Wrap => value.rem_euclid(1.0),
                OverflowBehavior::Scale => (value / max).clamp(0.0, 1.0),
            };
        }
        out
    });
    blended.map(|x, y, pixel| lerp4(pixel, src.get(x, y), blend_with_original.clamp(0.0, 1.0)))
}

fn channel_enabled(mask: ChannelMask, channel: usize) -> bool {
    match mask {
        ChannelMask::Rgb => true,
        ChannelMask::Red => channel == 0,
        ChannelMask::Green => channel == 1,
        ChannelMask::Blue => channel == 2,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvertChannel {
    Rgb,
    Red,
    Green,
    Blue,
    Hls,
    Hue,
    Lightness,
    Saturation,
    Yiq,
    Luminance,
    InPhase,
    Quadrature,
    Alpha,
}

pub fn invert(src: &Image, channel: InvertChannel, blend_with_original: f32) -> Image {
    src.map(|_, _, pixel| {
        let inverted = invert_channel(pixel, channel);
        lerp4(inverted, pixel, blend_with_original.clamp(0.0, 1.0))
    })
}

fn invert_channel(pixel: [f32; 4], channel: InvertChannel) -> [f32; 4] {
    let mut out = pixel;
    match channel {
        InvertChannel::Rgb => {
            out[0] = 1.0 - pixel[0];
            out[1] = 1.0 - pixel[1];
            out[2] = 1.0 - pixel[2];
        }
        InvertChannel::Red => out[0] = 1.0 - pixel[0],
        InvertChannel::Green => out[1] = 1.0 - pixel[1],
        InvertChannel::Blue => out[2] = 1.0 - pixel[2],
        InvertChannel::Alpha => out[3] = 1.0 - pixel[3],
        InvertChannel::Hls => {
            let [h, s, l] = rgb_to_hsl(pixel[0], pixel[1], pixel[2]);
            let [r, g, b] = hsl_to_rgb(1.0 - h, 1.0 - s, 1.0 - l);
            out[0] = r;
            out[1] = g;
            out[2] = b;
        }
        InvertChannel::Hue => {
            let [h, s, l] = rgb_to_hsl(pixel[0], pixel[1], pixel[2]);
            let [r, g, b] = hsl_to_rgb(h + 0.5, s, l);
            out[0] = r;
            out[1] = g;
            out[2] = b;
        }
        InvertChannel::Lightness => {
            let [h, s, l] = rgb_to_hsl(pixel[0], pixel[1], pixel[2]);
            let [r, g, b] = hsl_to_rgb(h, s, 1.0 - l);
            out[0] = r;
            out[1] = g;
            out[2] = b;
        }
        InvertChannel::Saturation => {
            let [h, s, l] = rgb_to_hsl(pixel[0], pixel[1], pixel[2]);
            let [r, g, b] = hsl_to_rgb(h, 1.0 - s, l);
            out[0] = r;
            out[1] = g;
            out[2] = b;
        }
        InvertChannel::Yiq => {
            let yiq = rgb_to_yiq(pixel);
            let [r, g, b] = yiq_to_rgb([1.0 - yiq[0], -yiq[1], -yiq[2]]);
            out[0] = r;
            out[1] = g;
            out[2] = b;
        }
        InvertChannel::Luminance => {
            let yiq = rgb_to_yiq(pixel);
            let [r, g, b] = yiq_to_rgb([1.0 - yiq[0], yiq[1], yiq[2]]);
            out[0] = r;
            out[1] = g;
            out[2] = b;
        }
        InvertChannel::InPhase => {
            let yiq = rgb_to_yiq(pixel);
            let [r, g, b] = yiq_to_rgb([yiq[0], -yiq[1], yiq[2]]);
            out[0] = r;
            out[1] = g;
            out[2] = b;
        }
        InvertChannel::Quadrature => {
            let yiq = rgb_to_yiq(pixel);
            let [r, g, b] = yiq_to_rgb([yiq[0], yiq[1], -yiq[2]]);
            out[0] = r;
            out[1] = g;
            out[2] = b;
        }
    }
    out
}

fn rgb_to_yiq(pixel: [f32; 4]) -> [f32; 3] {
    let r = pixel[0];
    let g = pixel[1];
    let b = pixel[2];
    [
        0.299 * r + 0.587 * g + 0.114 * b,
        0.596 * r - 0.274 * g - 0.322 * b,
        0.211 * r - 0.523 * g + 0.312 * b,
    ]
}

fn yiq_to_rgb(yiq: [f32; 3]) -> [f32; 3] {
    let [y, i, q] = yiq;
    [
        y + 0.956 * i + 0.621 * q,
        y - 0.272 * i - 0.647 * q,
        y - 1.106 * i + 1.703 * q,
    ]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MinimaxOp {
    Minimum,
    Maximum,
    MinimumThenMaximum,
    MaximumThenMinimum,
}

pub fn minimax(src: &Image, op: MinimaxOp, axes: Axes, radius: i32) -> Image {
    match op {
        MinimaxOp::Minimum => extremum(src, radius, axes, false),
        MinimaxOp::Maximum => extremum(src, radius, axes, true),
        MinimaxOp::MinimumThenMaximum => {
            extremum(&extremum(src, radius, axes, false), radius, axes, true)
        }
        MinimaxOp::MaximumThenMinimum => {
            extremum(&extremum(src, radius, axes, true), radius, axes, false)
        }
    }
}

pub fn remove_color_matting(src: &Image, background: Rgba, clip_hdr: bool, bits_32: bool) -> Image {
    src.map(|_, _, pixel| {
        let alpha = pixel[3];
        if alpha <= 1.0e-5 {
            return pixel;
        }
        let mut out = pixel;
        let bg = [background.r, background.g, background.b];
        for channel in 0..3 {
            let straight = (pixel[channel] - bg[channel] * (1.0 - alpha)) / alpha;
            out[channel] = if clip_hdr || !bits_32 {
                straight.clamp(0.0, 1.0)
            } else {
                straight
            };
        }
        out
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RgbChannel {
    Red,
    Green,
    Blue,
    Alpha,
    Luminance,
}

pub fn set_channels(src: &Image, control: &Image, sources: [RgbChannel; 4]) -> Image {
    src.map(|x, y, pixel| {
        let sample = placed(control, x, y, src, MapFit::Stretch);
        let mut out = pixel;
        for (index, source) in sources.iter().copied().enumerate() {
            out[index] = pick_channel(sample, source);
        }
        out
    })
}

fn pick_channel(pixel: [f32; 4], source: RgbChannel) -> f32 {
    match source {
        RgbChannel::Red => pixel[0],
        RgbChannel::Green => pixel[1],
        RgbChannel::Blue => pixel[2],
        RgbChannel::Alpha => pixel[3],
        RgbChannel::Luminance => luma(pixel[0], pixel[1], pixel[2]),
    }
}

pub fn set_matte(
    src: &Image,
    matte: &Image,
    source: RgbChannel,
    invert: bool,
    stretch: bool,
    composite_original: bool,
    premultiply: bool,
) -> Image {
    let fit = if stretch {
        MapFit::Stretch
    } else {
        MapFit::Center
    };
    src.map(|x, y, pixel| {
        let sample = placed(matte, x, y, src, fit);
        let mut alpha = pick_channel(sample, source);
        if invert {
            alpha = 1.0 - alpha;
        }
        if composite_original {
            alpha *= pixel[3];
        }
        let mut out = pixel;
        if premultiply {
            for channel in 0..3 {
                out[channel] *= alpha;
            }
        }
        out[3] = alpha;
        out
    })
}

pub fn shift_channels(src: &Image, take: [RgbChannel; 4]) -> Image {
    src.map(|_, _, pixel| {
        let mut out = [0.0; 4];
        for (index, source) in take.iter().copied().enumerate() {
            out[index] = pick_channel(pixel, source);
        }
        out
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SolidBlend {
    Over,
    Plus,
}

pub fn solid_composite(
    src: &Image,
    source_opacity: f32,
    color: [f32; 3],
    opacity: f32,
    mode: SolidBlend,
) -> Image {
    let source_opacity = source_opacity.clamp(0.0, 1.0);
    let opacity = opacity.clamp(0.0, 1.0);
    src.map(|_, _, pixel| {
        let src_a = pixel[3] * source_opacity;
        let color_a = opacity;
        match mode {
            SolidBlend::Over => {
                let out_a = src_a + color_a * (1.0 - src_a);
                if out_a <= 1.0e-6 {
                    return [0.0; 4];
                }
                [
                    (pixel[0] * src_a + color[0] * color_a * (1.0 - src_a)) / out_a,
                    (pixel[1] * src_a + color[1] * color_a * (1.0 - src_a)) / out_a,
                    (pixel[2] * src_a + color[2] * color_a * (1.0 - src_a)) / out_a,
                    out_a,
                ]
            }
            SolidBlend::Plus => [
                pixel[0] * src_a + color[0] * color_a,
                pixel[1] * src_a + color[1] * color_a,
                pixel[2] * src_a + color[2] * color_a,
                (src_a + color_a).min(1.0),
            ],
        }
    })
}
