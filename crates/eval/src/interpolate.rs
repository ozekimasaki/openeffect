use model::{
    Keyframe, PropertyValue, Roving, SpatialInterpolation, TemporalInterpolation, Time,
};

pub fn sample_scalar(keys: &[Keyframe], time: Time, base: f64) -> f64 {
    if keys.is_empty() {
        return base;
    }
    let mut ordered: Vec<&Keyframe> = keys.iter().collect();
    ordered.sort_by(|a, b| a.time.seconds().partial_cmp(&b.time.seconds()).unwrap_or(std::cmp::Ordering::Equal));
    if time.seconds() <= ordered[0].time.seconds() {
        return scalar(&ordered[0].value).unwrap_or(base);
    }
    if time.seconds() >= ordered[ordered.len() - 1].time.seconds() {
        return scalar(&ordered[ordered.len() - 1].value).unwrap_or(base);
    }
    let mut index = 0;
    while index + 1 < ordered.len() && ordered[index + 1].time.seconds() < time.seconds() {
        index += 1;
    }
    let left = ordered[index];
    let right = ordered[index + 1];
    let v0 = scalar(&left.value).unwrap_or(base);
    let v1 = scalar(&right.value).unwrap_or(base);
    match left.temporal_out {
        TemporalInterpolation::Hold => v0,
        TemporalInterpolation::Linear => lerp(v0, v1, portion(left.time, right.time, time)),
        TemporalInterpolation::Bezier
        | TemporalInterpolation::ContinuousBezier
        | TemporalInterpolation::AutoBezier => bezier_value(left, right, time, v0, v1),
    }
}

fn bezier_value(left: &Keyframe, right: &Keyframe, time: Time, v0: f64, v1: f64) -> f64 {
    let dt = right.time.seconds() - left.time.seconds();
    if dt == 0.0 {
        return v0;
    }
    let u = portion(left.time, right.time, time);
    let out_inf = (left.ease_out.influence / 100.0).clamp(0.0, 1.0);
    let in_inf = (right.ease_in.influence / 100.0).clamp(0.0, 1.0);
    let p1 = v0 + left.ease_out.speed * dt * out_inf;
    let p2 = v1 - right.ease_in.speed * dt * in_inf;
    cubic(v0, p1, p2, v1, u)
}

fn cubic(p0: f64, p1: f64, p2: f64, p3: f64, u: f64) -> f64 {
    let t = u;
    let m = 1.0 - t;
    m * m * m * p0 + 3.0 * m * m * t * p1 + 3.0 * m * t * t * p2 + t * t * t * p3
}

fn portion(start: Time, end: Time, time: Time) -> f64 {
    let span = end.seconds() - start.seconds();
    if span == 0.0 {
        0.0
    } else {
        ((time.seconds() - start.seconds()) / span).clamp(0.0, 1.0)
    }
}

fn lerp(a: f64, b: f64, u: f64) -> f64 {
    a + (b - a) * u
}

pub fn scalar(value: &PropertyValue) -> Option<f64> {
    match value {
        PropertyValue::Scalar(value) => Some(*value),
        _ => None,
    }
}

pub fn sample_position(keys: &[Keyframe], time: Time) -> [f64; 2] {
    if keys.is_empty() {
        return [0.0, 0.0];
    }
    let mut ordered: Vec<&Keyframe> = keys.iter().collect();
    ordered.sort_by(|a, b| a.time.seconds().partial_cmp(&b.time.seconds()).unwrap_or(std::cmp::Ordering::Equal));
    let point = |key: &Keyframe| match &key.value {
        PropertyValue::TwoD(value) => *value,
        PropertyValue::ThreeD(value) => [value[0], value[1]],
        _ => [0.0, 0.0],
    };
    if ordered.len() == 1 || time.seconds() <= ordered[0].time.seconds() {
        return point(ordered[0]);
    }
    if time.seconds() >= ordered[ordered.len() - 1].time.seconds() {
        return point(ordered[ordered.len() - 1]);
    }
    let mut index = 0;
    while index + 1 < ordered.len() && ordered[index + 1].time.seconds() < time.seconds() {
        index += 1;
    }
    let left = ordered[index];
    let right = ordered[index + 1];
    let u = portion(left.time, right.time, time);
    let a = point(left);
    let b = point(right);
    match left.spatial {
        SpatialInterpolation::Linear => [lerp(a[0], b[0], u), lerp(a[1], b[1], u)],
        SpatialInterpolation::AutoBezier | SpatialInterpolation::ContinuousBezier | SpatialInterpolation::Bezier => {
            let prev = if index > 0 { point(ordered[index - 1]) } else { a };
            let next = if index + 2 < ordered.len() { point(ordered[index + 2]) } else { b };
            let (c1, c2) = spatial_controls(left, right, prev, a, b, next);
            [cubic(a[0], c1[0], c2[0], b[0], u), cubic(a[1], c1[1], c2[1], b[1], u)]
        }
    }
}

fn spatial_controls(
    left: &Keyframe,
    right: &Keyframe,
    prev: [f64; 2],
    a: [f64; 2],
    b: [f64; 2],
    next: [f64; 2],
) -> ([f64; 2], [f64; 2]) {
    match left.spatial {
        SpatialInterpolation::Linear => (a, b),
        SpatialInterpolation::Bezier | SpatialInterpolation::ContinuousBezier => (
            [a[0] + left.spatial_out[0], a[1] + left.spatial_out[1]],
            [b[0] + right.spatial_in[0], b[1] + right.spatial_in[1]],
        ),
        SpatialInterpolation::AutoBezier => {
            let out = [(b[0] - prev[0]) / 6.0, (b[1] - prev[1]) / 6.0];
            let inn = [(a[0] - next[0]) / 6.0, (a[1] - next[1]) / 6.0];
            ([a[0] + out[0], a[1] + out[1]], [b[0] + inn[0], b[1] + inn[1]])
        }
    }
}

/// ハンドルを掴むと Auto Bezier は Continuous Bezier になる。
pub fn grab_spatial_handle(key: &mut Keyframe, outgoing: [f64; 3]) {
    if key.spatial == SpatialInterpolation::AutoBezier {
        key.spatial = SpatialInterpolation::ContinuousBezier;
    }
    key.spatial_out = outgoing;
    if key.spatial == SpatialInterpolation::ContinuousBezier {
        key.spatial_in = [-outgoing[0], -outgoing[1], -outgoing[2]];
    }
}

pub fn rove_middle_key(keys: &mut [Keyframe]) {
    if keys.len() < 3 {
        return;
    }
    let mut indexed: Vec<usize> = (0..keys.len()).collect();
    indexed.sort_by(|a, b| keys[*a].time.seconds().partial_cmp(&keys[*b].time.seconds()).unwrap_or(std::cmp::Ordering::Equal));
    let first = indexed[0];
    let last = indexed[indexed.len() - 1];
    let start = keys[first].time.seconds();
    let end = keys[last].time.seconds();
    let mut distance = 0.0;
    let mut lengths = Vec::new();
    for pair in indexed.windows(2) {
        let a = xy(&keys[pair[0]]);
        let b = xy(&keys[pair[1]]);
        let len = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
        lengths.push(len);
        distance += len;
    }
    if distance == 0.0 {
        return;
    }
    let mut walked = 0.0;
    for (offset, index) in indexed.iter().enumerate() {
        if keys[*index].roving != Roving::RoveAcrossTime {
            if offset > 0 {
                walked += lengths[offset - 1];
            }
            continue;
        }
        if offset > 0 {
            walked += lengths[offset - 1];
        }
        let u = walked / distance;
        keys[*index].time = Time::from_seconds(start + (end - start) * u);
    }
}

fn xy(key: &Keyframe) -> [f64; 2] {
    match &key.value {
        PropertyValue::TwoD(value) => *value,
        PropertyValue::ThreeD(value) => [value[0], value[1]],
        _ => [0.0, 0.0],
    }
}

pub fn speed_variation(keys: &[Keyframe]) -> f64 {
    if keys.len() < 3 {
        return 0.0;
    }
    let mut ordered: Vec<&Keyframe> = keys.iter().collect();
    ordered.sort_by(|a, b| a.time.seconds().partial_cmp(&b.time.seconds()).unwrap_or(std::cmp::Ordering::Equal));
    let mut speeds = Vec::new();
    for pair in ordered.windows(2) {
        let dt = pair[1].time.seconds() - pair[0].time.seconds();
        if dt <= 0.0 {
            continue;
        }
        let a = xy(pair[0]);
        let b = xy(pair[1]);
        let dist = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
        speeds.push(dist / dt);
    }
    if speeds.len() < 2 {
        return 0.0;
    }
    let max = speeds.iter().cloned().fold(0.0, f64::max);
    let min = speeds.iter().cloned().fold(f64::INFINITY, f64::min);
    max - min
}

pub fn time_reverse_keys(keys: &mut [Keyframe]) {
    if keys.is_empty() {
        return;
    }
    let start = keys.iter().map(|key| key.time.seconds()).fold(f64::INFINITY, f64::min);
    let end = keys.iter().map(|key| key.time.seconds()).fold(f64::NEG_INFINITY, f64::max);
    for key in keys {
        key.time = Time::from_seconds(start + end - key.time.seconds());
    }
}

/// 0 を含むスケールの扱いは要確認。正の値だけ幾何平均のキーを足す。
pub fn exponential_scale(start: f64, end: f64, start_time: Time, end_time: Time) -> Option<Vec<Keyframe>> {
    if start <= 0.0 || end <= 0.0 {
        return None;
    }
    let mid_value = (start * end).sqrt();
    let mid_time = Time::from_seconds(0.5 * (start_time.seconds() + end_time.seconds()));
    Some(vec![
        Keyframe::at(start_time, PropertyValue::Scalar(start)),
        Keyframe::at(mid_time, PropertyValue::Scalar(mid_value)),
        Keyframe::at(end_time, PropertyValue::Scalar(end)),
    ])
}

pub fn eval_simple_expression(source: &str, time: Time) -> Option<f64> {
    let source = source.trim();
    if let Ok(value) = source.parse::<f64>() {
        return Some(value);
    }
    let parts: Vec<&str> = source.split_whitespace().collect();
    match parts.as_slice() {
        ["time", "*", factor] => factor.parse::<f64>().ok().map(|factor| time.seconds() * factor),
        ["time", "*", factor, "+", offset] => {
            let factor = factor.parse::<f64>().ok()?;
            let offset = offset.parse::<f64>().ok()?;
            Some(time.seconds() * factor + offset)
        }
        _ => None,
    }
}

pub fn bake_expression(source: &str, times: &[Time]) -> Option<Vec<Keyframe>> {
    let mut keys = Vec::new();
    for time in times {
        let value = eval_simple_expression(source, *time)?;
        keys.push(Keyframe::at(*time, PropertyValue::Scalar(value)));
    }
    Some(keys)
}

pub fn audio_amplitude(samples: &[f32], window: usize) -> Vec<f64> {
    if samples.is_empty() || window == 0 {
        return Vec::new();
    }
    samples
        .chunks(window)
        .map(|chunk| {
            let energy = chunk.iter().map(|sample| sample.abs() as f64).sum::<f64>() / chunk.len() as f64;
            energy
        })
        .collect()
}

pub fn keys_from_points(points: &[(Time, f64)]) -> Vec<Keyframe> {
    points
        .iter()
        .map(|(time, value)| Keyframe::at(*time, PropertyValue::Scalar(*value)))
        .collect()
}

pub fn corner_turns(keys: &[Keyframe]) -> bool {
    if keys.len() < 3 {
        return false;
    }
    let a = xy(&keys[0]);
    let b = xy(&keys[1]);
    let c = xy(&keys[2]);
    let in_dir = [b[0] - a[0], b[1] - a[1]];
    let out_dir = [c[0] - b[0], c[1] - b[1]];
    let cross = in_dir[0] * out_dir[1] - in_dir[1] * out_dir[0];
    cross.abs() > 1.0e-6
}
