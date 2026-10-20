use model::project::WorkingSpace;
use model::Rgba;

pub fn srgb_to_linear(channel: f32) -> f32 {
    if channel <= 0.04045 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}

pub fn linear_to_srgb(channel: f32) -> f32 {
    if channel <= 0.0031308 {
        channel * 12.92
    } else {
        1.055 * channel.powf(1.0 / 2.4) - 0.055
    }
}

pub fn rec709_to_linear(channel: f32) -> f32 {
    if channel < 0.081 {
        channel / 4.5
    } else {
        ((channel + 0.099) / 1.099).powf(1.0 / 0.45)
    }
}

pub fn linear_to_rec709(channel: f32) -> f32 {
    if channel < 0.018 {
        channel * 4.5
    } else {
        1.099 * channel.powf(0.45) - 0.099
    }
}

/// sRGB の符号値を Rec.709 の符号値へ。原色は同じなので伝達関数だけを変える。
pub fn srgb_code_to_rec709(channel: f32) -> f32 {
    linear_to_rec709(srgb_to_linear(channel))
}

pub fn convert_to_working(pixel: Rgba, working: Option<&WorkingSpace>, preserve_rgb: bool, as_linear: bool) -> Rgba {
    if preserve_rgb {
        return pixel;
    }
    match working {
        None | Some(WorkingSpace::Off) => pixel,
        Some(WorkingSpace::HdtvRec709) => {
            if as_linear {
                pixel.map_rgb(linear_to_rec709)
            } else {
                pixel.map_rgb(srgb_code_to_rec709)
            }
        }
        Some(WorkingSpace::Srgb) => {
            if as_linear {
                pixel.map_rgb(linear_to_srgb)
            } else {
                pixel
            }
        }
        Some(_) => pixel,
    }
}

/// 作業用スペースの線形化は、同じ原色のまま伝達を外してから戻す。
pub fn linearize_mix(a: f32, b: f32, working_is_srgb: bool) -> f32 {
    if working_is_srgb {
        linear_to_srgb(0.5 * (srgb_to_linear(a) + srgb_to_linear(b)))
    } else {
        0.5 * (a + b)
    }
}

pub fn gamma_encoded_mix(a: f32, b: f32) -> f32 {
    0.5 * (a + b)
}

/// 表示変換は書き出しには掛けない。
pub fn display_transform(channel: f32, display: Option<&str>) -> f32 {
    match display {
        Some("linear") => srgb_to_linear(channel),
        Some("srgb-view") => linear_to_srgb(channel),
        _ => channel,
    }
}
