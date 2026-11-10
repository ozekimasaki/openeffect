//! 特徴パッチの照合。正規化相互相関ではなく、チャンネルごとの差の二乗和。

use crate::raster::{channel_count, channel_sample, CompareChannel, Raster};

#[derive(Clone, Debug, PartialEq)]
pub struct Patch {
    pub width: i32,
    pub height: i32,
    pub values: Vec<f32>,
    pub channels: usize,
    pub channel: CompareChannel,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MatchHit {
    pub center: [f64; 2],
    pub confidence: f64,
}

pub fn capture_patch(
    image: &Raster,
    center: [f64; 2],
    size: [f64; 2],
    channel: CompareChannel,
) -> Patch {
    let width = size[0].round().max(1.0) as i32;
    let height = size[1].round().max(1.0) as i32;
    let channels = channel_count(channel);
    let mut values = Vec::with_capacity((width * height) as usize * channels);
    let origin_x = center[0] - f64::from(width) * 0.5;
    let origin_y = center[1] - f64::from(height) * 0.5;
    for y in 0..height {
        for x in 0..width {
            let pixel = image.sample(origin_x + f64::from(x), origin_y + f64::from(y));
            let sample = channel_sample(pixel, channel);
            values.extend_from_slice(&sample[..channels]);
        }
    }
    Patch {
        width,
        height,
        values,
        channels,
        channel,
    }
}

pub fn match_patch(
    image: &Raster,
    patch: &Patch,
    search_center: [f64; 2],
    search_size: [f64; 2],
    feature_size: [f64; 2],
    subpixel: bool,
) -> MatchHit {
    let slack_x = ((search_size[0] - feature_size[0]) * 0.5).max(0.0);
    let slack_y = ((search_size[1] - feature_size[1]) * 0.5).max(0.0);
    let min_x = (search_center[0] - slack_x).floor() as i32;
    let max_x = (search_center[0] + slack_x).ceil() as i32;
    let min_y = (search_center[1] - slack_y).floor() as i32;
    let max_y = (search_center[1] + slack_y).ceil() as i32;
    let mut best = (
        search_center[0].round() as i32,
        search_center[1].round() as i32,
    );
    let mut best_ssd = f64::MAX;
    let mut worst_ssd = 0.0;
    let mut field = Vec::new();
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let ssd = ssd_at(image, patch, [f64::from(x), f64::from(y)]);
            field.push((x, y, ssd));
            if ssd < best_ssd {
                best_ssd = ssd;
                best = (x, y);
            }
            if ssd > worst_ssd {
                worst_ssd = ssd;
            }
        }
    }
    let mut center = [f64::from(best.0), f64::from(best.1)];
    if subpixel {
        center[0] += parabolic(&field, best, true);
        center[1] += parabolic(&field, best, false);
    }
    let samples = (patch.width * patch.height).max(1) as f64 * patch.channels as f64;
    let mse = best_ssd / samples;
    let confidence = ((1.0 - mse).clamp(0.0, 1.0)) * 100.0;
    let _ = worst_ssd;
    MatchHit { center, confidence }
}

fn ssd_at(image: &Raster, patch: &Patch, center: [f64; 2]) -> f64 {
    let origin_x = center[0] - f64::from(patch.width) * 0.5;
    let origin_y = center[1] - f64::from(patch.height) * 0.5;
    let mut sum = 0.0;
    let mut cursor = 0;
    for y in 0..patch.height {
        for x in 0..patch.width {
            let pixel = image.sample(origin_x + f64::from(x), origin_y + f64::from(y));
            let sample = channel_sample(pixel, patch.channel);
            for channel in 0..patch.channels {
                let delta = f64::from(sample[channel] - patch.values[cursor]);
                sum += delta * delta;
                cursor += 1;
            }
        }
    }
    sum
}

fn parabolic(field: &[(i32, i32, f64)], best: (i32, i32), horizontal: bool) -> f64 {
    let left = neighbor(
        field,
        best,
        if horizontal { -1 } else { 0 },
        if horizontal { 0 } else { -1 },
    );
    let mid = neighbor(field, best, 0, 0);
    let right = neighbor(
        field,
        best,
        if horizontal { 1 } else { 0 },
        if horizontal { 0 } else { 1 },
    );
    let Some((left, mid, right)) = left.zip(mid).zip(right).map(|((l, m), r)| (l, m, r)) else {
        return 0.0;
    };
    let denom = left + right - 2.0 * mid;
    if denom.abs() < 1e-9 {
        return 0.0;
    }
    (0.5 * (left - right) / denom).clamp(-1.0, 1.0)
}

fn neighbor(field: &[(i32, i32, f64)], best: (i32, i32), dx: i32, dy: i32) -> Option<f64> {
    field
        .iter()
        .find(|entry| entry.0 == best.0 + dx && entry.1 == best.1 + dy)
        .map(|entry| entry.2)
}
