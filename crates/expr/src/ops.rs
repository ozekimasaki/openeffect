use std::f64::consts::PI;

use crate::value::{array_of, numbers_of, PathData, Value};

pub fn pad(mut values: Vec<f64>, len: usize) -> Vec<f64> {
    values.resize(len, 0.0);
    values
}

pub fn add(a: &[f64], b: &[f64]) -> Vec<f64> {
    let n = a.len().max(b.len());
    let a = pad(a.to_vec(), n);
    let b = pad(b.to_vec(), n);
    a.into_iter().zip(b).map(|(x, y)| x + y).collect()
}

pub fn sub(a: &[f64], b: &[f64]) -> Vec<f64> {
    let n = a.len().max(b.len());
    let a = pad(a.to_vec(), n);
    let b = pad(b.to_vec(), n);
    a.into_iter().zip(b).map(|(x, y)| x - y).collect()
}

pub fn mul(a: &[f64], amount: f64) -> Vec<f64> {
    a.iter().map(|component| component * amount).collect()
}

pub fn div(a: &[f64], amount: f64) -> Vec<f64> {
    a.iter().map(|component| component / amount).collect()
}

pub fn dot(a: &[f64], b: &[f64]) -> f64 {
    let n = a.len().max(b.len());
    let a = pad(a.to_vec(), n);
    let b = pad(b.to_vec(), n);
    a.into_iter().zip(b).map(|(x, y)| x * y).sum()
}

pub fn cross(a: &[f64], b: &[f64]) -> Vec<f64> {
    let a = pad(a.to_vec(), 3);
    let b = pad(b.to_vec(), 3);
    vec![
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

pub fn length(a: &[f64]) -> f64 {
    dot(a, a).sqrt()
}

pub fn normalize(a: &[f64]) -> Vec<f64> {
    div(a, length(a))
}

pub fn clamp_num(value: f64, limit1: f64, limit2: f64) -> f64 {
    let lo = limit1.min(limit2);
    let hi = limit1.max(limit2);
    value.clamp(lo, hi)
}

pub fn degrees_to_radians(degrees: f64) -> f64 {
    degrees * PI / 180.0
}

pub fn radians_to_degrees(radians: f64) -> f64 {
    radians * 180.0 / PI
}

pub fn round_half_away(value: f64) -> f64 {
    if value == 0.0 {
        0.0
    } else {
        value.signum() * value.abs().round()
    }
}

pub fn frames_number(t: f64, fps: f64, is_duration: bool) -> f64 {
    if fps == 0.0 {
        return 0.0;
    }
    let raw = t * fps;
    if is_duration {
        round_half_away(raw)
    } else {
        raw.floor()
    }
}

pub fn format_timecode(seconds: f64, base: f64, drop: bool, is_duration: bool) -> String {
    let frames = frames_number(seconds, base, is_duration);
    let base_i = base.round().max(1.0) as i64;
    let negative = frames < 0.0;
    let whole = frames.abs().round() as i64;
    let ff = whole % base_i;
    let total_seconds = whole / base_i;
    let sec = total_seconds % 60;
    let min = (total_seconds / 60) % 60;
    let hour = total_seconds / 3600;
    let sep = if drop { ';' } else { ':' };
    let sign = if negative { "-" } else { "" };
    format!("{sign}{hour:02}:{min:02}:{sec:02}{sep}{ff:02}")
}

pub fn format_feet(seconds: f64, fps: f64, frames_per_foot: f64, is_duration: bool) -> String {
    let frames = frames_number(seconds, fps, is_duration).round() as i64;
    let per = frames_per_foot.round().max(1.0) as i64;
    let negative = frames < 0;
    let whole = frames.abs();
    let feet = whole / per;
    let extra = whole % per;
    let sign = if negative { "-" } else { "" };
    format!("{sign}{feet}+{extra}")
}

fn hue_to_rgb(p: f64, q: f64, mut t: f64) -> f64 {
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }
    if t < 1.0 / 6.0 {
        return p + (q - p) * 6.0 * t;
    }
    if t < 1.0 / 2.0 {
        return q;
    }
    if t < 2.0 / 3.0 {
        return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
    }
    p
}

pub fn rgb_to_hsl(rgba: [f64; 4]) -> [f64; 4] {
    let (r, g, b, a) = (rgba[0], rgba[1], rgba[2], rgba[3]);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    if (max - min).abs() < 1e-12 {
        return [0.0, 0.0, l, a];
    }
    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };
    let h = if (max - r).abs() < 1e-12 {
        ((g - b) / d + if g < b { 6.0 } else { 0.0 }) / 6.0
    } else if (max - g).abs() < 1e-12 {
        ((b - r) / d + 2.0) / 6.0
    } else {
        ((r - g) / d + 4.0) / 6.0
    };
    [h, s, l, a]
}

pub fn hsl_to_rgb(hsla: [f64; 4]) -> [f64; 4] {
    let (h, s, l, a) = (hsla[0], hsla[1], hsla[2], hsla[3]);
    if s.abs() < 1e-12 {
        return [l, l, l, a];
    }
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    [
        hue_to_rgb(p, q, h + 1.0 / 3.0),
        hue_to_rgb(p, q, h),
        hue_to_rgb(p, q, h - 1.0 / 3.0),
        a,
    ]
}

pub fn hex_to_rgb(text: &str) -> Option<[f64; 4]> {
    let rest = text
        .strip_prefix("0x")
        .or_else(|| text.strip_prefix("0X"))
        .or_else(|| text.strip_prefix('#'))
        .unwrap_or(text);
    let hex: String = rest
        .chars()
        .take_while(|ch| ch.is_ascii_hexdigit())
        .take(8)
        .collect();
    if hex.len() != 6 && hex.len() != 8 {
        return None;
    }
    let channel = |index: usize| -> f64 {
        let byte = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16).unwrap_or(0);
        f64::from(byte) / 255.0
    };
    let alpha = if hex.len() == 8 { channel(3) } else { 1.0 };
    Some([channel(0), channel(1), channel(2), alpha])
}

fn fade(t: f64) -> f64 {
    t * t * (3.0 - 2.0 * t)
}

fn hash_unit(n: i32) -> f64 {
    let n = n.wrapping_mul(374761393).wrapping_add(668265263);
    let n = (n ^ (n >> 13)).wrapping_mul(1274126177);
    f64::from((n ^ (n >> 16)) as u32) / f64::from(u32::MAX) * 2.0 - 1.0
}

pub fn noise1(x: f64) -> f64 {
    let i = x.floor() as i32;
    let f = x - x.floor();
    let u = fade(f);
    let a = hash_unit(i);
    let b = hash_unit(i.wrapping_add(1));
    a * (1.0 - u) + b * u
}

pub fn noise2(x: f64, y: f64) -> f64 {
    noise1(x + y * 57.0)
}

pub fn noise3(x: f64, y: f64, z: f64) -> f64 {
    noise1(x + y * 57.0 + z * 113.0)
}

pub fn lerp(a: f64, b: f64, u: f64) -> f64 {
    a + (b - a) * u
}

pub fn smoothstep(u: f64) -> f64 {
    let u = u.clamp(0.0, 1.0);
    u * u * (3.0 - 2.0 * u)
}

pub fn ease_in_curve(u: f64) -> f64 {
    let u = u.clamp(0.0, 1.0);
    u * u
}

pub fn ease_out_curve(u: f64) -> f64 {
    let u = u.clamp(0.0, 1.0);
    1.0 - (1.0 - u) * (1.0 - u)
}

pub fn mix_values(a: &Value, b: &Value, curve: f64) -> Option<Value> {
    if let (Some(left), Some(right)) = (a.number(), b.number()) {
        return Some(Value::Number(lerp(left, right, curve)));
    }
    let left = numbers_of(a)?;
    let right = numbers_of(b)?;
    let n = left.len().max(right.len());
    let left = pad(left, n);
    let right = pad(right, n);
    Some(array_of(
        left.into_iter().zip(right).map(|(x, y)| lerp(x, y, curve)),
    ))
}

pub fn interp(
    t: f64,
    t_min: f64,
    t_max: f64,
    a: &Value,
    b: &Value,
    curve: fn(f64) -> f64,
) -> Option<Value> {
    if (t_max - t_min).abs() < 1e-12 {
        return if t < t_max {
            Some(a.clone())
        } else {
            Some(b.clone())
        };
    }
    let u = ((t - t_min) / (t_max - t_min)).clamp(0.0, 1.0);
    mix_values(a, b, curve(u))
}

/// 単位姿勢の z 軸を、X→Y→Z の順で回した向き。
pub fn orientation_forward(orientation: [f64; 3]) -> [f64; 3] {
    let x = degrees_to_radians(orientation[0]);
    let y = degrees_to_radians(orientation[1]);
    let z = degrees_to_radians(orientation[2]);
    let (sx, cx) = (x.sin(), x.cos());
    let (sy, cy) = (y.sin(), y.cos());
    let (sz, cz) = (z.sin(), z.cos());
    let v = [0.0, -sx, cx];
    let v = [sy * v[2], v[1], cy * v[2]];
    [cz * v[0] - sz * v[1], sz * v[0] + cz * v[1], v[2]]
}

pub fn look_at(from: [f64; 3], at: [f64; 3]) -> [f64; 3] {
    let dir = [at[0] - from[0], at[1] - from[1], at[2] - from[2]];
    let len = length(&dir);
    if len < 1e-12 {
        return [0.0, 0.0, 0.0];
    }
    let dir = [dir[0] / len, dir[1] / len, dir[2] / len];
    let rx = (-dir[1]).atan2(dir[0].hypot(dir[2]));
    let ry = dir[0].atan2(dir[2]);
    [radians_to_degrees(rx), radians_to_degrees(ry), 0.0]
}

#[derive(Clone, Copy, Debug)]
pub struct Xform {
    pub anchor: [f64; 3],
    pub position: [f64; 3],
    pub scale: [f64; 3],
    pub rotation: [f64; 3],
}

impl Xform {
    pub fn identity() -> Self {
        Self {
            anchor: [0.0, 0.0, 0.0],
            position: [0.0, 0.0, 0.0],
            scale: [100.0, 100.0, 100.0],
            rotation: [0.0, 0.0, 0.0],
        }
    }
}

fn rot_x(v: [f64; 3], degrees: f64) -> [f64; 3] {
    let r = degrees_to_radians(degrees);
    let (s, c) = (r.sin(), r.cos());
    [v[0], c * v[1] - s * v[2], s * v[1] + c * v[2]]
}

fn rot_y(v: [f64; 3], degrees: f64) -> [f64; 3] {
    let r = degrees_to_radians(degrees);
    let (s, c) = (r.sin(), r.cos());
    [c * v[0] + s * v[2], v[1], -s * v[0] + c * v[2]]
}

fn rot_z(v: [f64; 3], degrees: f64) -> [f64; 3] {
    let r = degrees_to_radians(degrees);
    let (s, c) = (r.sin(), r.cos());
    [c * v[0] - s * v[1], s * v[0] + c * v[1], v[2]]
}

fn rotate(v: [f64; 3], rotation: [f64; 3], two_d: bool) -> [f64; 3] {
    if two_d {
        return rot_z(v, rotation[2]);
    }
    let v = rot_x(v, rotation[0]);
    let v = rot_y(v, rotation[1]);
    rot_z(v, rotation[2])
}

fn rotate_inv(v: [f64; 3], rotation: [f64; 3], two_d: bool) -> [f64; 3] {
    if two_d {
        return rot_z(v, -rotation[2]);
    }
    let v = rot_z(v, -rotation[2]);
    let v = rot_y(v, -rotation[1]);
    rot_x(v, -rotation[0])
}

fn as3(point: &[f64]) -> [f64; 3] {
    [
        point.first().copied().unwrap_or(0.0),
        point.get(1).copied().unwrap_or(0.0),
        point.get(2).copied().unwrap_or(0.0),
    ]
}

pub fn apply_point(xf: &Xform, point: &[f64], two_d: bool) -> Vec<f64> {
    let dims = point.len().clamp(2, 3);
    let mut p = as3(point);
    p[0] -= xf.anchor[0];
    p[1] -= xf.anchor[1];
    p[2] -= xf.anchor[2];
    p[0] *= xf.scale[0] / 100.0;
    p[1] *= xf.scale[1] / 100.0;
    p[2] *= xf.scale[2] / 100.0;
    p = rotate(p, xf.rotation, two_d);
    p[0] += xf.position[0];
    p[1] += xf.position[1];
    p[2] += xf.position[2];
    p[..dims].to_vec()
}

pub fn apply_vec(xf: &Xform, vector: &[f64], two_d: bool) -> Vec<f64> {
    let dims = vector.len().clamp(2, 3);
    let mut p = as3(vector);
    p[0] *= xf.scale[0] / 100.0;
    p[1] *= xf.scale[1] / 100.0;
    p[2] *= xf.scale[2] / 100.0;
    p = rotate(p, xf.rotation, two_d);
    p[..dims].to_vec()
}

pub fn invert_point(xf: &Xform, point: &[f64], two_d: bool) -> Vec<f64> {
    let dims = point.len().clamp(2, 3);
    let mut p = as3(point);
    p[0] -= xf.position[0];
    p[1] -= xf.position[1];
    p[2] -= xf.position[2];
    p = rotate_inv(p, xf.rotation, two_d);
    p[0] /= xf.scale[0] / 100.0;
    p[1] /= xf.scale[1] / 100.0;
    p[2] /= xf.scale[2] / 100.0;
    p[0] += xf.anchor[0];
    p[1] += xf.anchor[1];
    p[2] += xf.anchor[2];
    p[..dims].to_vec()
}

pub fn invert_vec(xf: &Xform, vector: &[f64], two_d: bool) -> Vec<f64> {
    let dims = vector.len().clamp(2, 3);
    let mut p = as3(vector);
    p = rotate_inv(p, xf.rotation, two_d);
    p[0] /= xf.scale[0] / 100.0;
    p[1] /= xf.scale[1] / 100.0;
    p[2] /= xf.scale[2] / 100.0;
    p[..dims].to_vec()
}

fn polyline_lengths(points: &[[f64; 2]], closed: bool) -> (Vec<f64>, f64) {
    let mut spans = Vec::new();
    let mut total = 0.0;
    if points.len() >= 2 {
        for pair in points.windows(2) {
            let dx = pair[1][0] - pair[0][0];
            let dy = pair[1][1] - pair[0][1];
            let len = dx.hypot(dy);
            spans.push(len);
            total += len;
        }
        if closed {
            let dx = points[0][0] - points[points.len() - 1][0];
            let dy = points[0][1] - points[points.len() - 1][1];
            let len = dx.hypot(dy);
            spans.push(len);
            total += len;
        }
    }
    (spans, total)
}

pub fn point_on_path(points: &[[f64; 2]], closed: bool, percent: f64) -> [f64; 2] {
    if points.is_empty() {
        return [0.0, 0.0];
    }
    if points.len() == 1 {
        return points[0];
    }
    let percent = if closed {
        percent.rem_euclid(1.0)
    } else {
        percent.clamp(0.0, 1.0)
    };
    let (spans, total) = polyline_lengths(points, closed);
    if total < 1e-12 {
        return points[0];
    }
    let mut remain = percent * total;
    for (index, span) in spans.iter().copied().enumerate() {
        if remain <= span || index + 1 == spans.len() {
            let u = if span < 1e-12 { 0.0 } else { remain / span };
            let a = points[index % points.len()];
            let b = points[(index + 1) % points.len()];
            return [lerp(a[0], b[0], u), lerp(a[1], b[1], u)];
        }
        remain -= span;
    }
    points[points.len() - 1]
}

pub fn tangent_on_path(points: &[[f64; 2]], closed: bool, percent: f64) -> [f64; 2] {
    if points.len() < 2 {
        return [1.0, 0.0];
    }
    let percent = if closed {
        percent.rem_euclid(1.0)
    } else {
        percent.clamp(0.0, 1.0)
    };
    let (spans, total) = polyline_lengths(points, closed);
    if total < 1e-12 {
        return [1.0, 0.0];
    }
    let mut remain = percent * total;
    for (index, span) in spans.iter().copied().enumerate() {
        if remain <= span || index + 1 == spans.len() {
            let a = points[index % points.len()];
            let b = points[(index + 1) % points.len()];
            let dx = b[0] - a[0];
            let dy = b[1] - a[1];
            let len = dx.hypot(dy);
            if len < 1e-12 {
                return [1.0, 0.0];
            }
            return [dx / len, dy / len];
        }
        remain -= span;
    }
    [1.0, 0.0]
}

pub fn normal_on_path(points: &[[f64; 2]], closed: bool, percent: f64) -> [f64; 2] {
    let [tx, ty] = tangent_on_path(points, closed, percent);
    [-ty, tx]
}

pub fn default_path() -> PathData {
    PathData {
        points: vec![[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]],
        in_tangents: Vec::new(),
        out_tangents: Vec::new(),
        closed: true,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoopMode {
    Cycle,
    Pingpong,
    Offset,
    Continue,
}

pub fn parse_loop_mode(text: &str) -> Option<LoopMode> {
    match text {
        "cycle" => Some(LoopMode::Cycle),
        "pingpong" => Some(LoopMode::Pingpong),
        "offset" => Some(LoopMode::Offset),
        "continue" => Some(LoopMode::Continue),
        _ => None,
    }
}

#[derive(Clone, Debug)]
pub struct KeySample {
    pub time: f64,
    pub value: Vec<f64>,
}

fn lerp_vec(a: &[f64], b: &[f64], u: f64) -> Vec<f64> {
    let n = a.len().max(b.len());
    let a = pad(a.to_vec(), n);
    let b = pad(b.to_vec(), n);
    a.into_iter().zip(b).map(|(x, y)| lerp(x, y, u)).collect()
}

fn sample_linear(keys: &[KeySample], t: f64) -> Vec<f64> {
    if keys.is_empty() {
        return Vec::new();
    }
    if t <= keys[0].time {
        return keys[0].value.clone();
    }
    if t >= keys[keys.len() - 1].time {
        return keys[keys.len() - 1].value.clone();
    }
    for pair in keys.windows(2) {
        if pair[0].time <= t && t <= pair[1].time {
            let dt = pair[1].time - pair[0].time;
            let u = if dt.abs() < 1e-12 {
                0.0
            } else {
                (t - pair[0].time) / dt
            };
            return lerp_vec(&pair[0].value, &pair[1].value, u);
        }
    }
    keys[keys.len() - 1].value.clone()
}

fn map_cycle(t: f64, start: f64, end: f64) -> (f64, f64) {
    let dur = end - start;
    if dur.abs() < 1e-12 {
        return (start, 0.0);
    }
    let cycles = ((t - start) / dur).floor();
    let mapped = start + (t - start).rem_euclid(dur);
    (mapped, cycles)
}

fn map_pingpong(t: f64, start: f64, end: f64) -> f64 {
    let dur = end - start;
    if dur.abs() < 1e-12 {
        return start;
    }
    let period = dur * 2.0;
    let p = (t - start).rem_euclid(period);
    if p <= dur {
        start + p
    } else {
        end - (p - dur)
    }
}

pub fn loop_sample(
    keys: &[KeySample],
    t: f64,
    mode: LoopMode,
    outbound: bool,
    segment: Option<(f64, f64)>,
) -> Vec<f64> {
    if keys.len() < 2 {
        return sample_linear(keys, t);
    }
    let start = keys[0].time;
    let end = keys[keys.len() - 1].time;
    let outside = if outbound { t > end } else { t < start };
    if !outside {
        return sample_linear(keys, t);
    }
    let (seg_start, seg_end) = segment.unwrap_or((start, end));
    let seg_keys: Vec<KeySample> = keys
        .iter()
        .filter(|key| key.time >= seg_start - 1e-9 && key.time <= seg_end + 1e-9)
        .cloned()
        .collect();
    let owned;
    let use_keys = if seg_keys.len() >= 2 {
        &seg_keys
    } else {
        owned = keys.to_vec();
        &owned
    };
    match mode {
        LoopMode::Continue => {
            let (left, right, origin) = if outbound {
                let n = use_keys.len();
                (&use_keys[n - 2], &use_keys[n - 1], use_keys[n - 1].time)
            } else {
                (&use_keys[0], &use_keys[1], use_keys[0].time)
            };
            let dt = right.time - left.time;
            let slope = if dt.abs() < 1e-12 {
                vec![0.0; left.value.len()]
            } else {
                sub(&right.value, &left.value)
                    .into_iter()
                    .map(|component| component / dt)
                    .collect::<Vec<_>>()
            };
            let base = if outbound {
                right.value.clone()
            } else {
                left.value.clone()
            };
            let elapsed = t - origin;
            base.into_iter()
                .zip(slope)
                .map(|(value, slope)| value + slope * elapsed)
                .collect()
        }
        LoopMode::Pingpong => sample_linear(use_keys, map_pingpong(t, seg_start, seg_end)),
        LoopMode::Cycle | LoopMode::Offset => {
            let (mapped, cycles) = map_cycle(t, seg_start, seg_end);
            let sampled = sample_linear(use_keys, mapped);
            if mode == LoopMode::Cycle {
                sampled
            } else {
                let delta = sub(&use_keys[use_keys.len() - 1].value, &use_keys[0].value);
                sampled
                    .into_iter()
                    .zip(delta)
                    .map(|(value, delta)| value + delta * cycles)
                    .collect()
            }
        }
    }
}

pub fn unit_random(parts: &[u64]) -> f64 {
    let mut mixed: u64 = 0xA0761D6478BD642F;
    for part in parts {
        mixed = mixed.wrapping_mul(0xE7037ED1A0B428DB).wrapping_add(*part);
        mixed ^= mixed >> 33;
    }
    mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94D049BB133111EB);
    mixed ^= mixed >> 31;
    (mixed >> 11) as f64 / ((1u64 << 53) as f64)
}
