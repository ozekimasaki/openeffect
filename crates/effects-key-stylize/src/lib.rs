//! Keying、Matte、Noise & Grain、Stylize。
//! 文書が初期値を確定していないパラメータは、共有 API の既定値にしない。

#![forbid(unsafe_code)]

mod catalog;
mod image;
mod instance;
mod keying;
mod matte;
mod noise;
mod ops;
mod spec;
mod stylize;

pub use catalog::{effect, EFFECTS};
pub use effect_api::{
    defaults_from, ApplyError, ApplyResult, CpuApply, CpuInput, Frame, Frame16, Param, ParamIndex,
    ParamList, ParamValue, ParamValues,
};
pub use instance::{published_defaults, EffectInstance, EffectStack};
pub use model::{BitsPerChannel, Rgba, Time};
pub use spec::{EffectDef, KnownDefault, ParamKind, ParamSpec};
pub use stylize::threshold_luma;

pub fn threshold_at(frame: &Frame, cutoff: f32) -> ApplyResult<Frame> {
    let image = image::Image::from_frame(frame)?;
    Ok(threshold_luma(&image, cutoff).to_frame())
}

pub fn instantiate(id: &str) -> ApplyResult<EffectInstance> {
    effect(id)
        .map(EffectInstance::new)
        .ok_or_else(|| ApplyError::new(format!("未知の効果 {id}")))
}

pub fn frame8(width: u32, height: u32, pixels: &[[u8; 4]]) -> Frame {
    let mut samples = Vec::with_capacity(pixels.len() * 4);
    for pixel in pixels {
        samples.extend_from_slice(pixel);
    }
    Frame::Bpc8(effect_api::Frame8 {
        width,
        height,
        channels: 4,
        samples,
    })
}

pub fn solid8(width: u32, height: u32, pixel: [u8; 4]) -> Frame {
    let mut samples = Vec::with_capacity((width * height * 4) as usize);
    for _ in 0..width * height {
        samples.extend_from_slice(&pixel);
    }
    Frame::Bpc8(effect_api::Frame8 {
        width,
        height,
        channels: 4,
        samples,
    })
}

pub fn pixel8(frame: &Frame, x: u32, y: u32) -> [u8; 4] {
    let Frame::Bpc8(frame) = frame else {
        return [0, 0, 0, 0];
    };
    let index = ((y * frame.width + x) * 4) as usize;
    [
        frame.samples[index],
        frame.samples[index + 1],
        frame.samples[index + 2],
        frame.samples[index + 3],
    ]
}

pub fn render_at(
    id: &str,
    source: &Frame,
    time: Time,
    setup: impl FnOnce(&mut EffectInstance),
) -> ApplyResult<Frame> {
    let mut effect = instantiate(id)?;
    setup(&mut effect);
    let mut output = source.clone();
    let values = effect_api::ParamValues::new();
    let input = CpuInput {
        time,
        bits: source.bits(),
        params: &values,
        frame: source,
    };
    effect.apply_cpu(&input, &mut output)?;
    Ok(output)
}

pub fn render(
    id: &str,
    source: &Frame,
    setup: impl FnOnce(&mut EffectInstance),
) -> ApplyResult<Frame> {
    render_at(id, source, Time::ZERO, setup)
}
