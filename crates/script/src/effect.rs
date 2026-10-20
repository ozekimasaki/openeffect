//! エフェクト用の自前 C API。PF_ 名前と Adobe のヘッダは使わない。

use std::collections::HashMap;

use effect_api::{
    channel_16_to_32, channel_32_to_8, channel_8_to_16, convert_16_to_32, convert_32_to_8,
    convert_8_to_16, copy_frame, CpuApply, CpuInput, Frame, Frame16, Frame32, Frame8,
    IdentityEffect, ParamIndex, ParamList, ParamValue, ParamValues,
};
use model::{BitsPerChannel, LayerId, Rgba, Time};

use crate::error::{ScriptError, ScriptResult};
use crate::App;

#[derive(Clone, Debug)]
struct ParamTrack {
    keys: Vec<(f64, ParamValue)>,
    value: ParamValue,
}

#[derive(Clone, Debug)]
pub struct FxEffect {
    pub list: ParamList,
    tracks: Vec<ParamTrack>,
    checkouts: HashMap<u64, CheckedBuffer>,
    next_id: u64,
}

#[derive(Clone, Debug)]
struct CheckedBuffer {
    frame: Frame,
    live: bool,
}

impl FxEffect {
    pub fn new() -> Self {
        Self {
            list: ParamList::new(),
            tracks: Vec::new(),
            checkouts: HashMap::new(),
            next_id: 1,
        }
    }

    fn push_track(&mut self, value: ParamValue) -> u32 {
        self.tracks.push(ParamTrack {
            keys: Vec::new(),
            value,
        });
        (self.tracks.len() - 1) as u32
    }
}

impl Default for FxEffect {
    fn default() -> Self {
        Self::new()
    }
}

pub fn oe_fx_define_float(
    effect: &mut FxEffect,
    name: &str,
    default: f64,
    min: f64,
    max: f64,
) -> u32 {
    let index = effect.list.define_float(name, default, min, max);
    effect.push_track(ParamValue::Float(default));
    index.get()
}

pub fn oe_fx_define_angle(effect: &mut FxEffect, name: &str, default_degrees: f64) -> u32 {
    let index = effect.list.define_angle(name, default_degrees);
    effect.push_track(ParamValue::Angle(default_degrees));
    index.get()
}

pub fn oe_fx_define_checkbox(effect: &mut FxEffect, name: &str, default: bool) -> u32 {
    let index = effect.list.define_checkbox(name, default);
    effect.push_track(ParamValue::Checkbox(default));
    index.get()
}

pub fn oe_fx_define_color(effect: &mut FxEffect, name: &str, default: [f64; 3]) -> u32 {
    let rgba = Rgba::new(default[0] as f32, default[1] as f32, default[2] as f32, 1.0);
    let index = effect.list.define_color(name, rgba);
    effect.push_track(ParamValue::Color(rgba));
    index.get()
}

pub fn oe_fx_define_point(effect: &mut FxEffect, name: &str, default: [f64; 2]) -> u32 {
    let index = effect.list.define_point(name, default);
    effect.push_track(ParamValue::Point(default));
    index.get()
}

pub fn oe_fx_define_popup(
    effect: &mut FxEffect,
    name: &str,
    choices: Vec<String>,
    default_number: u32,
) -> u32 {
    let stored = default_number.saturating_sub(1);
    let index = effect.list.define_popup(name, choices, stored);
    effect.push_track(ParamValue::Popup(default_number));
    index.get()
}

pub fn oe_fx_define_layer(effect: &mut FxEffect, name: &str) -> u32 {
    let index = effect.list.define_layer(name);
    effect.push_track(ParamValue::Layer(None));
    index.get()
}

pub fn oe_fx_param_count(effect: &FxEffect) -> i32 {
    effect.list.len() as i32
}

pub fn oe_fx_set_param(effect: &mut FxEffect, index: u32, value: ParamValue) -> ScriptResult<()> {
    let track = effect
        .tracks
        .get_mut(index as usize)
        .ok_or_else(|| ScriptError::range("パラメータ番号"))?;
    track.value = value;
    Ok(())
}

pub fn oe_fx_set_param_key(
    effect: &mut FxEffect,
    index: u32,
    time: f64,
    value: ParamValue,
) -> ScriptResult<()> {
    let track = effect
        .tracks
        .get_mut(index as usize)
        .ok_or_else(|| ScriptError::range("パラメータ番号"))?;
    if let Some(existing) = track.keys.iter_mut().find(|(key_time, _)| (*key_time - time).abs() < 1e-9)
    {
        existing.1 = value;
    } else {
        track.keys.push((time, value));
        track.keys.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    }
    Ok(())
}

pub fn oe_fx_param_get(effect: &FxEffect, index: u32, time: f64) -> ScriptResult<ParamValue> {
    let track = effect
        .tracks
        .get(index as usize)
        .ok_or_else(|| ScriptError::range("パラメータ番号"))?;
    Ok(sample_track(track, time))
}

fn sample_track(track: &ParamTrack, time: f64) -> ParamValue {
    if track.keys.is_empty() {
        return track.value.clone();
    }
    let mut left: Option<&(f64, ParamValue)> = None;
    let mut right: Option<&(f64, ParamValue)> = None;
    for key in &track.keys {
        if key.0 <= time {
            left = Some(key);
        } else if right.is_none() {
            right = Some(key);
        }
    }
    match (left, right) {
        (Some((t0, v0)), Some((t1, v1))) if (t1 - t0).abs() > 1e-9 => {
            let amount = (time - t0) / (t1 - t0);
            lerp_param(v0, v1, amount)
        }
        (Some((_, value)), _) => value.clone(),
        (None, Some((_, value))) => value.clone(),
        (None, None) => track.value.clone(),
    }
}

fn lerp_param(left: &ParamValue, right: &ParamValue, amount: f64) -> ParamValue {
    match (left, right) {
        (ParamValue::Float(a), ParamValue::Float(b)) => ParamValue::Float(a + (b - a) * amount),
        (ParamValue::Angle(a), ParamValue::Angle(b)) => ParamValue::Angle(a + (b - a) * amount),
        (ParamValue::Point(a), ParamValue::Point(b)) => ParamValue::Point([
            a[0] + (b[0] - a[0]) * amount,
            a[1] + (b[1] - a[1]) * amount,
        ]),
        _ => left.clone(),
    }
}

pub fn oe_fx_slider_visible(effect: &FxEffect, index: u32) -> bool {
    effect.list.get(ParamIndex::new(index)).is_some()
}

pub fn oe_fx_render(
    effect: &FxEffect,
    input: &Frame,
    output: &mut Frame,
    time: f64,
    bits: BitsPerChannel,
) -> ScriptResult<()> {
    let mut values = ParamValues::new();
    for (index, _) in effect.list.iter() {
        values.push(sample_track(&effect.tracks[index.get() as usize], time));
    }
    let cpu = CpuInput {
        time: Time::from_seconds(time),
        bits,
        params: &values,
        frame: input,
    };
    IdentityEffect
        .apply_cpu(&cpu, output)
        .map_err(|error| ScriptError::invalid(error.message))
}

pub fn oe_fx_checkout_input(
    app: &App,
    effect: &mut FxEffect,
    layer_param: u32,
    time: f64,
) -> ScriptResult<u64> {
    let value = oe_fx_param_get(effect, layer_param, time)?;
    let ParamValue::Layer(layer) = value else {
        return Err(ScriptError::invalid("レイヤーパラメータではない"));
    };
    let Some(layer) = layer else {
        return Err(ScriptError::missing("レイヤー入力"));
    };
    let (width, height) = app.layer_source_size_at(layer, time)?;
    let frame = Frame::Bpc8(Frame8::new(width, height, 4, 255));
    let id = effect.next_id;
    effect.next_id += 1;
    effect.checkouts.insert(id, CheckedBuffer { frame, live: true });
    Ok(id)
}

pub fn oe_fx_checked_size(effect: &FxEffect, buffer: u64) -> ScriptResult<(u32, u32)> {
    let checked = effect
        .checkouts
        .get(&buffer)
        .ok_or_else(|| ScriptError::missing("バッファ"))?;
    if !checked.live {
        return Err(ScriptError::invalid("解放済みのバッファ"));
    }
    Ok((checked.frame.width(), checked.frame.height()))
}

pub fn oe_fx_release_input(effect: &mut FxEffect, buffer: u64) -> ScriptResult<()> {
    let checked = effect
        .checkouts
        .get_mut(&buffer)
        .ok_or_else(|| ScriptError::missing("バッファ"))?;
    checked.live = false;
    Ok(())
}

pub fn oe_fx_frame_8(width: u32, height: u32, channels: u8) -> Frame8 {
    Frame8::new(width, height, channels, 255)
}

pub fn oe_fx_frame_16(width: u32, height: u32, channels: u8) -> Frame16 {
    Frame16::new(width, height, channels, 32768)
}

pub fn oe_fx_frame_32(width: u32, height: u32, channels: u8) -> Frame32 {
    Frame32::new(width, height, channels, 1.0)
}

pub fn oe_fx_frame_32_with(width: u32, height: u32, channels: u8, fill: f32) -> Frame32 {
    Frame32::new(width, height, channels, fill)
}

pub fn oe_fx_convert_8_to_16(input: &Frame8) -> Frame16 {
    convert_8_to_16(input)
}

pub fn oe_fx_convert_16_to_32(input: &Frame16) -> Frame32 {
    convert_16_to_32(input)
}

pub fn oe_fx_convert_32_to_8(input: &Frame32) -> Frame8 {
    convert_32_to_8(input)
}

pub fn oe_fx_layer_to_world(app: &App, layer: LayerId, time: f64, point: [f64; 2]) -> ScriptResult<[f64; 2]> {
    app.layer_to_world(layer, time, point)
}

pub fn oe_fx_world_to_layer(app: &App, layer: LayerId, time: f64, point: [f64; 2]) -> ScriptResult<[f64; 2]> {
    app.world_to_layer(layer, time, point)
}

pub fn oe_fx_layer_to_comp(app: &App, layer: LayerId, time: f64, point: [f64; 2]) -> ScriptResult<[f64; 2]> {
    app.source_point_to_comp(layer, time, point)
}

pub fn oe_fx_comp_to_layer(app: &App, layer: LayerId, time: f64, point: [f64; 2]) -> ScriptResult<[f64; 2]> {
    app.comp_point_to_source(layer, time, point)
}

pub fn channel_white_8() -> u8 {
    channel_32_to_8(1.0)
}

pub fn channel_white_16_from_8() -> u16 {
    channel_8_to_16(255)
}

pub fn channel_one_from_16() -> f32 {
    channel_16_to_32(32768)
}

pub fn copy_identity(input: &Frame, output: &mut Frame) {
    copy_frame(input, output);
}
