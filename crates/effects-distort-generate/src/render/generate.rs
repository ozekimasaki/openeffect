use std::f32::consts::TAU;

use effect_api::ApplyResult;
use model::Rgba;

use super::distort::evolution_at;
use crate::host::{cell_is_sharp, cell_pattern_index, Instance, MaskShape};
use crate::image::{composite, dist, fbm, hash01, luma, point_in_poly, Filter, Image};
use crate::spec::EffectKind;

use super::{map_px, sample_at};

pub fn four_color(inst: &Instance, src: &Image) -> ApplyResult<Image> {
    let blend = inst.num("Blend").unwrap_or(1.0) as f32;
    let jitter = inst.num("Jitter").unwrap_or(0.0) as f32;
    let opacity = inst.num("Opacity").unwrap_or(1.0) as f32;
    let mode = inst.blend_or_none("Blending Mode");
    let points = [0, 1, 2, 3].map(|i| {
        inst.role_point(&format!("p{i}")).unwrap_or([
            if i % 2 == 0 { 0.0 } else { src.width as f64 },
            if i < 2 { 0.0 } else { src.height as f64 },
        ])
    });
    let colors = [0, 1, 2, 3].map(|i| inst.role_color(&format!("c{i}")).unwrap_or(Rgba::black()));
    Ok(map_px(src, |x, y| {
        let mut weight = [0.0_f32; 4];
        let mut sum = 0.0;
        for i in 0..4 {
            let d = dist(x as f32, y as f32, points[i][0] as f32, points[i][1] as f32).max(0.001);
            let w = 1.0 / d.powf(blend.max(0.05));
            weight[i] = w;
            sum += w;
        }
        let mut px = [0.0; 4];
        for i in 0..4 {
            let c = colors[i];
            let w = weight[i] / sum;
            px[0] += c.r * w;
            px[1] += c.g * w;
            px[2] += c.b * w;
            px[3] += c.a * w;
        }
        let max_w = weight.into_iter().fold(0.0_f32, f32::max) / sum;
        if jitter > 0.0 && max_w < 0.85 {
            let n = hash01(x, y, 4) - 0.5;
            for c in 0..3 {
                px[c] += n * jitter;
            }
        }
        px[3] *= opacity;
        composite(mode, px, src.at(x, y))
    }))
}

pub fn lightning(inst: &Instance, src: &Image) -> ApplyResult<Image> {
    let kind = inst
        .choice("Lightning Type")
        .unwrap_or_else(|| "Vertical".into());
    let origin = inst
        .point_at("Origin", 0.0)
        .unwrap_or([src.width as f64 / 2.0, 0.0]);
    let direction = inst
        .point_at("Direction", 0.0)
        .unwrap_or([src.width as f64 / 2.0, src.height as f64]);
    let outer = inst.num("Outer Radius").unwrap_or(src.height as f64) as f32;
    let conduct = inst.num("Conductivity State").unwrap_or(0.0);
    let glow = inst.num("Glow Opacity").unwrap_or(1.0) as f32;
    let obstacle = inst.num("Alpha Obstacle").unwrap_or(0.0) as f32;
    let turbulence = inst.num("Turbulence").unwrap_or(0.0) as f32;
    let fork = inst.num("Forking").unwrap_or(0.0) as f32;
    let decay = inst.num("Decay").unwrap_or(1.0) as f32;
    let decay_core = inst.flag("Decay Main Core").unwrap_or(false);
    let composite_on = inst.flag("Composite On Original").unwrap_or(false);
    let complexity = inst.num("Complexity").unwrap_or(1.0) as f32;
    let min_fork = inst.num("Min. Forkdistance").unwrap_or(8.0) as f32;
    let stop = inst.num("Termination Threshold").unwrap_or(0.5) as f32;
    let core_only = inst.flag("Main Core Collision Only").unwrap_or(false);
    let drain = inst.num("Core Drain").unwrap_or(0.0) as f32 / 100.0;
    let fork_strength = inst.num("Fork Strength").unwrap_or(100.0) as f32 / 100.0;
    let fork_var = inst.num("Fork Variation").unwrap_or(0.0) as f32;
    let seed = (conduct * 1000.0) as u32;
    let mut out = if composite_on {
        src.clone()
    } else {
        super::blank_like(src, src.width, src.height)
    };
    let target = match kind.as_str() {
        "Omni" | "Anywhere" => [origin[0] as f32, origin[1] as f32 + outer],
        "Vertical" => [origin[0] as f32, src.height as f32],
        _ if inst.enabled("Direction") => [direction[0] as f32, direction[1] as f32],
        _ => [origin[0] as f32, src.height as f32],
    };
    let steps = 24;
    let mut x = origin[0] as f32;
    let mut y = origin[1] as f32;
    let mut forks = Vec::new();
    for step in 0..steps {
        let t = step as f32 / steps as f32;
        let n = fbm(
            x * 0.1 + seed as f32,
            y * 0.1,
            complexity.max(1.0) as u32,
            seed,
        );
        let mut nx = target[0] + (n - 0.5) * turbulence * 8.0;
        let mut ny = y + (target[1] - origin[1] as f32) / steps as f32;
        if kind == "Breaking" {
            let pull = t;
            nx = nx * (1.0 - pull) + target[0] * pull;
        }
        if obstacle != 0.0 {
            let alpha = src.get(nx as i32, ny as i32)[3];
            let blocked = if obstacle > 0.0 {
                alpha > stop
            } else {
                alpha < stop
            };
            if blocked && (!core_only || step == 0) {
                nx += obstacle.signum() * 3.0;
            }
        }
        draw_seg(
            &mut out,
            x,
            y,
            nx,
            ny,
            core_alpha(t, decay, decay_core),
            glow,
        );
        if fork > 0.0 && step as f32 * 4.0 > min_fork && hash01(step, seed, 2) < fork {
            let vary = 1.0 + (hash01(seed, step, 8) - 0.5) * fork_var;
            forks.push((nx, ny, fork_strength * vary * (1.0 - drain)));
        }
        x = nx;
        y = ny;
        if y >= src.height as f32
            || (kind == "Omni" && dist(x, y, origin[0] as f32, origin[1] as f32) > outer)
        {
            break;
        }
    }
    for (fx, fy, alpha) in forks {
        draw_seg(
            &mut out,
            fx,
            fy,
            fx + 6.0,
            fy + 8.0,
            alpha * (1.0 - decay).max(0.1),
            glow,
        );
    }
    Ok(out)
}

fn core_alpha(t: f32, decay: f32, core: bool) -> f32 {
    if core {
        (1.0 - t * decay).clamp(0.0, 1.0)
    } else {
        1.0
    }
}

fn draw_seg(img: &mut Image, x0: f32, y0: f32, x1: f32, y1: f32, alpha: f32, glow: f32) {
    let steps = dist(x0, y0, x1, y1).ceil() as i32 + 1;
    for i in 0..=steps {
        let t = i as f32 / steps.max(1) as f32;
        let x = x0 + (x1 - x0) * t;
        let y = y0 + (y1 - y0) * t;
        stamp(img, x as i32, y as i32, [1.0, 1.0, 1.0, alpha]);
        if glow > 0.0 {
            stamp(
                img,
                x as i32 + 1,
                y as i32,
                [0.6, 0.7, 1.0, alpha * glow * 0.4],
            );
        }
    }
}

fn stamp(img: &mut Image, x: i32, y: i32, px: [f32; 4]) {
    let cur = img.get(x, y);
    img.set(
        x,
        y,
        [
            cur[0] + px[0] * px[3],
            cur[1] + px[1] * px[3],
            cur[2] + px[2] * px[3],
            (cur[3] + px[3]).min(1.0),
        ],
    );
}

pub fn spectrum(inst: &Instance, src: &Image, time: f64) -> ApplyResult<Image> {
    let audio = audio_of(inst);
    let bands = inst.num("Frequency Bands").unwrap_or(8.0).clamp(1.0, 64.0) as usize;
    let start_f = inst.num("Start Frequency").unwrap_or(0.0);
    let end_f = inst.num("End Frequency").unwrap_or(2000.0);
    let max_h = inst.num("Maximum Height").unwrap_or(20.0) as f32;
    let duration = inst.num("Audio Duration").unwrap_or(100.0);
    let offset = inst.num("Audio Offset").unwrap_or(0.0);
    let thickness = inst.num("Thickness").unwrap_or(2.0) as f32;
    let softness = inst.num("Softness").unwrap_or(0.0) as f32;
    let inside = inst.color_at("Inside Color", time).unwrap_or(Rgba::white());
    let outside = inst
        .color_at("Outside Color", time)
        .unwrap_or(Rgba::black());
    let hue = inst.num("Hue Interpolation").unwrap_or(0.0);
    let polar = inst.flag("Use Polar Path").unwrap_or(false);
    let side = inst
        .choice("Side Options")
        .unwrap_or_else(|| "Side A".into());
    let display = inst
        .choice("Display Options")
        .unwrap_or_else(|| "Digital".into());
    let overlap = inst.flag("Blend Overlapping Colors").unwrap_or(false);
    let average = inst.flag("Duration Averaging").unwrap_or(false);
    let on_original = inst.flag("Composite On Original").unwrap_or(false);
    let heights = band_heights(
        audio, time, offset, duration, start_f, end_f, bands, average,
    );
    let (p0, p1) = if inst.enabled("Start Point") {
        (
            inst.point_at("Start Point", time)
                .unwrap_or([0.0, src.height as f64 / 2.0]),
            inst.point_at("End Point", time)
                .unwrap_or([src.width as f64, src.height as f64 / 2.0]),
        )
    } else {
        (
            [0.0, src.height as f64 / 2.0],
            [src.width as f64, src.height as f64 / 2.0],
        )
    };
    let mut out = if on_original {
        src.clone()
    } else {
        super::blank_like(src, src.width, src.height)
    };
    for (i, h) in heights.iter().enumerate() {
        let t = if bands == 1 {
            0.0
        } else {
            i as f32 / (bands - 1) as f32
        };
        let mut color = mix_color(inside, outside, t);
        if hue > 0.0 {
            let shift = if inst.flag("Dynamic Hue Phase").unwrap_or(false) {
                t
            } else {
                0.0
            };
            let sym = if inst.flag("Color Symmetry").unwrap_or(false) {
                (t - 0.5).abs() * 2.0
            } else {
                t
            };
            color = hue_color((sym + shift) * hue as f32);
        }
        let x = p0[0] as f32 + (p1[0] as f32 - p0[0] as f32) * t;
        let y = p0[1] as f32 + (p1[1] as f32 - p0[1] as f32) * t;
        let amp = h * max_h;
        let (dx, dy) = if polar {
            let a = t * TAU;
            (a.cos(), a.sin())
        } else {
            let len = dist(p0[0] as f32, p0[1] as f32, p1[0] as f32, p1[1] as f32).max(1.0);
            (
                -(p1[1] as f32 - p0[1] as f32) / len,
                (p1[0] as f32 - p0[0] as f32) / len,
            )
        };
        let sides: &[(f32, f32)] = match side.as_str() {
            "Side B" => &[(-dx, -dy)],
            "Side A & B" => &[(dx, dy), (-dx, -dy)],
            _ => &[(dx, dy)],
        };
        for (sx, sy) in sides {
            match display.as_str() {
                "Analog Dots" => stamp(
                    &mut out,
                    (x + sx * amp) as i32,
                    (y + sy * amp) as i32,
                    [color[0], color[1], color[2], 1.0],
                ),
                "Analog Lines" => draw_seg(&mut out, x, y, x + sx * amp, y + sy * amp, 1.0, 0.0),
                _ => {
                    for s in 0..=amp.abs().ceil() as i32 {
                        let tt = s as f32;
                        let px = x + sx * tt;
                        let py = y + sy * tt;
                        stamp(
                            &mut out,
                            px as i32,
                            py as i32,
                            [color[0], color[1], color[2], 1.0],
                        );
                        if softness > 0.0 {
                            stamp(
                                &mut out,
                                px as i32 + 1,
                                py as i32,
                                [color[0], color[1], color[2], softness.min(1.0) * 0.5],
                            );
                        }
                        let _ = thickness;
                    }
                }
            }
        }
        let _ = overlap;
    }
    if overlap {
        for px in &mut out.px {
            if px[3] > 1.0 {
                let s = px[3];
                px[0] /= s;
                px[1] /= s;
                px[2] /= s;
                px[3] = 1.0;
            }
        }
    }
    Ok(out)
}

fn hue_color(turns: f32) -> [f32; 3] {
    let h = turns.rem_euclid(1.0);
    [
        (h * TAU).cos() * 0.5 + 0.5,
        ((h + 0.33) * TAU).cos() * 0.5 + 0.5,
        ((h + 0.66) * TAU).cos() * 0.5 + 0.5,
    ]
}

fn mix_color(a: Rgba, b: Rgba, t: f32) -> [f32; 3] {
    [
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
    ]
}

fn audio_of(inst: &Instance) -> Option<&crate::host::AudioClip> {
    if let Some(Some(id)) = inst.layer_value("Audio Layer") {
        return inst
            .host
            .layers
            .iter()
            .find(|layer| layer.id == id)
            .and_then(|layer| layer.audio.as_ref());
    }
    inst.host.audio.as_ref()
}

fn band_heights(
    audio: Option<&crate::host::AudioClip>,
    time: f64,
    offset_ms: f64,
    duration_ms: f64,
    start_f: f64,
    end_f: f64,
    bands: usize,
    average: bool,
) -> Vec<f32> {
    let Some(audio) = audio else {
        return vec![0.0; bands];
    };
    let rate = audio.sample_rate.max(1) as f64;
    let center = ((time * 1000.0 + offset_ms) / 1000.0 * rate) as isize;
    let window = ((duration_ms / 1000.0) * rate).round().max(8.0) as usize;
    let ch = audio.channels.max(1) as usize;
    let mut mags = vec![0.0_f32; bands];
    for band in 0..bands {
        let freq = if bands == 1 {
            start_f
        } else {
            start_f + (end_f - start_f) * band as f64 / (bands - 1) as f64
        };
        let omega = TAU as f64 * freq / rate;
        let mut re = 0.0;
        let mut im = 0.0;
        let passes = if average { 2 } else { 1 };
        for pass in 0..passes {
            for n in 0..window {
                let idx = center + n as isize - window as isize / 2 + pass as isize * 3;
                if idx < 0 {
                    continue;
                }
                let i = idx as usize * ch;
                if i >= audio.samples.len() {
                    continue;
                }
                let sample = audio.samples[i];
                let th = omega * n as f64;
                re += f64::from(sample) * th.cos();
                im += f64::from(sample) * th.sin();
            }
        }
        mags[band] = ((re * re + im * im).sqrt() as f32) / window as f32 / passes as f32;
    }
    let max = mags.iter().copied().fold(0.0_f32, f32::max).max(1e-6);
    mags.iter().map(|v| v / max).collect()
}

pub fn waveform(inst: &Instance, src: &Image, time: f64) -> ApplyResult<Image> {
    let audio = audio_of(inst);
    let samples_n = inst
        .num("Displayed Samples")
        .unwrap_or(32.0)
        .clamp(2.0, 256.0) as usize;
    let max_h = inst.num("Maximum Height").unwrap_or(20.0) as f32;
    let duration = inst.num("Audio Duration").unwrap_or(100.0);
    let offset = inst.num("Audio Offset").unwrap_or(0.0);
    let thickness = inst.num("Thickness").unwrap_or(1.0) as f32;
    let softness = inst.num("Softness").unwrap_or(0.0) as f32;
    let inside = inst.color_at("Inside Color", time).unwrap_or(Rgba::white());
    let outside = inst
        .color_at("Outside Color", time)
        .unwrap_or(Rgba::white());
    let display = inst
        .choice("Display Options")
        .unwrap_or_else(|| "Digital".into());
    let on_original = inst.flag("Composite On Original").unwrap_or(false);
    let mono = inst.role_index("waveform").unwrap_or(0) == 0;
    let (p0, p1) = if inst.enabled("Start Point") {
        (
            inst.point_at("Start Point", time)
                .unwrap_or([0.0, src.height as f64 / 2.0]),
            inst.point_at("End Point", time)
                .unwrap_or([src.width as f64, src.height as f64 / 2.0]),
        )
    } else {
        (
            [0.0, src.height as f64 / 2.0],
            [src.width as f64, src.height as f64 / 2.0],
        )
    };
    let mut out = if on_original {
        src.clone()
    } else {
        super::blank_like(src, src.width, src.height)
    };
    let wave = read_wave(audio, time, offset, duration, samples_n, mono);
    for (i, amp) in wave.iter().enumerate() {
        let t = i as f32 / wave.len().max(2) as f32;
        let x = p0[0] as f32 + (p1[0] as f32 - p0[0] as f32) * t;
        let y = p0[1] as f32 + (p1[1] as f32 - p0[1] as f32) * t;
        let color = mix_color(inside, outside, t);
        let h = amp * max_h;
        match display.as_str() {
            "Analog Dots" => stamp(
                &mut out,
                x as i32,
                (y - h) as i32,
                [color[0], color[1], color[2], 1.0],
            ),
            "Analog Lines" if i + 1 < wave.len() => {
                let t2 = (i + 1) as f32 / wave.len() as f32;
                let x2 = p0[0] as f32 + (p1[0] as f32 - p0[0] as f32) * t2;
                let y2 = p0[1] as f32 + (p1[1] as f32 - p0[1] as f32) * t2 - wave[i + 1] * max_h;
                draw_seg(&mut out, x, y - h, x2, y2, 1.0, softness);
            }
            _ => draw_seg(&mut out, x, y - h.abs(), x, y + h.abs(), 1.0, softness),
        }
        let _ = thickness;
    }
    Ok(out)
}

fn read_wave(
    audio: Option<&crate::host::AudioClip>,
    time: f64,
    offset_ms: f64,
    duration_ms: f64,
    count: usize,
    mono: bool,
) -> Vec<f32> {
    let Some(audio) = audio else {
        return vec![0.0; count];
    };
    let rate = audio.sample_rate.max(1) as f64;
    let start = ((time * 1000.0 + offset_ms) / 1000.0 * rate) as isize;
    let window = ((duration_ms / 1000.0) * rate).round().max(1.0) as usize;
    let ch = audio.channels.max(1) as usize;
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let idx = start + (i * window / count) as isize;
        if idx < 0 {
            out.push(0.0);
            continue;
        }
        let base = idx as usize * ch;
        if base >= audio.samples.len() {
            out.push(0.0);
            continue;
        }
        let left = audio.samples[base];
        let right = if ch > 1 && base + 1 < audio.samples.len() {
            audio.samples[base + 1]
        } else {
            left
        };
        out.push(if mono { (left + right) * 0.5 } else { left });
    }
    out
}

pub fn beam(inst: &Instance, src: &Image) -> ApplyResult<Image> {
    let length = inst.num("Length").unwrap_or(0.0) as f32 / 100.0;
    let time = inst.num("Time").unwrap_or(0.0) as f32 / 100.0;
    let perspective = inst.flag("3D Perspective").unwrap_or(false);
    let start = inst
        .role_point("start")
        .unwrap_or([0.0, src.height as f64 / 2.0]);
    let end = inst
        .role_point("end")
        .unwrap_or([src.width as f64, src.height as f64 / 2.0]);
    let thick0 = inst.role_number("thick0").unwrap_or(2.0) as f32;
    let thick1 = inst.role_number("thick1").unwrap_or(2.0) as f32;
    let visible = length * (time * std::f32::consts::PI).sin().max(0.0);
    let head = time;
    let tail = (head - visible).max(0.0);
    let mut out = super::blank_like(src, src.width, src.height);
    let steps = dist(
        start[0] as f32,
        start[1] as f32,
        end[0] as f32,
        end[1] as f32,
    )
    .ceil() as i32
        + 1;
    for i in 0..=steps {
        let t = i as f32 / steps.max(1) as f32;
        if t < tail || t > head {
            continue;
        }
        let x = start[0] as f32 + (end[0] as f32 - start[0] as f32) * t;
        let y = start[1] as f32 + (end[1] as f32 - start[1] as f32) * t;
        let thick = if perspective {
            thick0 + (thick1 - thick0) * t
        } else {
            thick0.max(thick1)
        };
        let r = thick.ceil() as i32;
        for oy in -r..=r {
            for ox in -r..=r {
                if (ox * ox + oy * oy) as f32 <= thick * thick {
                    stamp(&mut out, x as i32 + ox, y as i32 + oy, [1.0, 1.0, 1.0, 1.0]);
                }
            }
        }
    }
    Ok(out)
}

pub fn cells(inst: &Instance, src: &Image, time: f64) -> ApplyResult<Image> {
    let pattern = cell_pattern_index(inst);
    let invert = inst.flag("Invert").unwrap_or(false);
    let contrast = inst.num("Contrast/Sharpness").unwrap_or(0.0) as f32;
    let disperse = inst.num("Disperse").unwrap_or(1.0) as f32;
    let size = inst.num("Size").unwrap_or(60.0).max(1.0) as f32;
    let offset = inst.role_point("offset").unwrap_or([0.0, 0.0]);
    let tile = inst.flag("Enable Tiling").unwrap_or(false);
    let cells_x = inst.num("Cells Horizontal").unwrap_or(4.0).max(1.0) as f32;
    let cells_y = inst.num("Cells Vertical").unwrap_or(4.0).max(1.0) as f32;
    let evo = evolution_at(inst, time) as f32;
    let seed = inst.num("Random Seed").unwrap_or(0.0) as u32;
    let sharp = cell_is_sharp(inst);
    let best = inst.host.quality == crate::host::Quality::Best;
    Ok(map_px(src, |x, y| {
        let mut fx = x as f32 + offset[0] as f32;
        let mut fy = y as f32 + offset[1] as f32;
        if tile {
            let tw = src.width as f32 / cells_x;
            let th = src.height as f32 / cells_y;
            fx = fx.rem_euclid(tw.max(1.0));
            fy = fy.rem_euclid(th.max(1.0));
        }
        let scale = size / 60.0;
        let nx = fx / scale;
        let ny = fy / scale;
        let jitter = if disperse > 1.0 && !best {
            1.0
        } else {
            disperse
        };
        let n = cell_noise(nx, ny, seed, evo, pattern, jitter);
        let mut v = if sharp {
            let edge = (n - cell_noise(nx + 1.0, ny, seed, evo, pattern, jitter)).abs();
            (edge * (1.0 + contrast / 50.0)).clamp(0.0, 1.0)
        } else {
            let c = (n - 0.5) * (1.0 + contrast / 50.0) + 0.5;
            apply_overflow(inst, c)
        };
        if pattern == 6 {
            // Static Plates は Plates と同じ形で、Evolution では明度を動かさない。
            v = cell_noise(nx, ny, seed, 0.0, 5, jitter);
        }
        if invert {
            v = 1.0 - v;
        }
        [v, v, v, 1.0]
    }))
}

fn apply_overflow(inst: &Instance, value: f32) -> f32 {
    if !inst.enabled("Overflow") {
        return value.clamp(0.0, 1.0);
    }
    let code = value * 255.0;
    match inst.choice("Overflow").as_deref() {
        Some("Soft Clamp") => (code / (1.0 + code.abs() / 255.0)).clamp(0.0, 255.0) / 255.0,
        Some("Wrap Back") => {
            let mut v = code;
            while v > 255.0 {
                v = 255.0 - (v - 255.0);
            }
            while v < 0.0 {
                v = -v;
            }
            v / 255.0
        }
        _ => code.clamp(0.0, 255.0) / 255.0,
    }
}

fn cell_noise(x: f32, y: f32, seed: u32, evo: f32, pattern: u32, disperse: f32) -> f32 {
    let x = x + evo * 4.0;
    let ix = x.floor() as i32;
    let iy = y.floor() as i32;
    let mut best = f32::MAX;
    let mut second = f32::MAX;
    for oy in -1..=1 {
        for ox in -1..=1 {
            let cx = ix + ox;
            let cy = iy + oy;
            let jx = hash01(cx as u32, cy as u32, seed) * disperse;
            let jy = hash01(cy as u32, cx as u32, seed.wrapping_add(3)) * disperse;
            let px = cx as f32 + jx;
            let py = cy as f32 + jy;
            let d = dist(x, y, px, py);
            if d < best {
                second = best;
                best = d;
            } else if d < second {
                second = d;
            }
        }
    }
    match pattern {
        1 | 3 => (second - best).clamp(0.0, 1.0),
        2 => (1.0 - best).clamp(0.0, 1.0),
        4 => best.sin().abs(),
        5 | 6 => (0.5 + 0.5 * (best * 6.0).sin()).clamp(0.0, 1.0),
        7 => (second - best).powf(0.5),
        _ => (1.0 - best.min(1.0)).clamp(0.0, 1.0),
    }
}

pub fn checker(inst: &Instance, src: &Image) -> ApplyResult<Image> {
    let anchor = inst.point_at("Anchor", 0.0).unwrap_or([0.0, 0.0]);
    let (w, h) = tile_size(inst, src);
    let feather = inst.num("Feather").unwrap_or(0.0) as f32;
    let color = inst.color_at("Color", 0.0).unwrap_or(Rgba::white());
    let opacity = inst.num("Opacity").unwrap_or(1.0) as f32;
    let mode = inst.blend_or_none("Blending Mode");
    Ok(map_px(src, |x, y| {
        let lx = x as f32 - anchor[0] as f32;
        let ly = y as f32 - anchor[1] as f32;
        let ix = (lx / w.max(0.001)).floor() as i32;
        let iy = (ly / h.max(0.001)).floor() as i32;
        let on = (ix + iy).rem_euclid(2) == 0;
        if !on {
            return match mode {
                None => [0.0; 4],
                Some(_) => composite(mode, [0.0; 4], src.at(x, y)),
            };
        }
        let fx = (lx.rem_euclid(w) - w / 2.0).abs();
        let fy = (ly.rem_euclid(h) - h / 2.0).abs();
        let edge = (w / 2.0 - fx).min(h / 2.0 - fy);
        let a = if feather <= 0.0 {
            1.0
        } else {
            (edge / feather).clamp(0.0, 1.0)
        } * opacity;
        composite(mode, [color.r, color.g, color.b, a * color.a], src.at(x, y))
    }))
}

fn tile_size(inst: &Instance, src: &Image) -> (f32, f32) {
    match inst.choice("Size From").as_deref() {
        Some("Width Slider") => {
            let w = inst.num("Width").unwrap_or(16.0) as f32;
            (w, w)
        }
        Some("Width & Height Sliders") => (
            inst.num("Width").unwrap_or(16.0) as f32,
            inst.num("Height").unwrap_or(16.0) as f32,
        ),
        _ => {
            let corner = inst.point_at("Corner Point", 0.0).unwrap_or([16.0, 16.0]);
            let anchor = inst.point_at("Anchor", 0.0).unwrap_or([0.0, 0.0]);
            (
                (corner[0] - anchor[0]).abs() as f32,
                (corner[1] - anchor[1]).abs() as f32,
            )
        }
    }
    .let_or(src)
}

trait PairExt {
    fn let_or(self, src: &Image) -> (f32, f32);
}

impl PairExt for (f32, f32) {
    fn let_or(self, _src: &Image) -> (f32, f32) {
        (self.0.max(0.5), self.1.max(0.5))
    }
}

pub fn circle(inst: &Instance, src: &Image) -> ApplyResult<Image> {
    let center = inst
        .role_point("center")
        .unwrap_or([src.width as f64 / 2.0, src.height as f64 / 2.0]);
    let color = inst.role_color("color").unwrap_or(Rgba::white());
    let radius = inst.num("Radius").unwrap_or(10.0) as f32;
    let edge_radius = inst.role_number("edge_radius").unwrap_or(radius as f64) as f32;
    let thickness = inst.num("Thickness").unwrap_or(2.0) as f32;
    let feather = inst.num("Feather").unwrap_or(0.0) as f32;
    let edge = inst.choice("Edge").unwrap_or_else(|| "None".into());
    let invert = inst.flag("Invert Circle").unwrap_or(false);
    let mode = inst.blend_or_none("Blending Mode");
    let (inner, outer, feather) = match edge.as_str() {
        "Edge Radius" => {
            let a = radius.min(edge_radius);
            let b = radius.max(edge_radius);
            (a, b, feather)
        }
        "Thickness" => (radius - thickness, radius, feather),
        "Thickness * Radius" => (radius - thickness * radius, radius, feather),
        "Thickness & Feather * Radius" => (radius - thickness * radius, radius, feather * radius),
        _ => (0.0, radius, feather),
    };
    Ok(map_px(src, |x, y| {
        let d = dist(x as f32, y as f32, center[0] as f32, center[1] as f32);
        let mut cover = if edge == "None" {
            if d <= radius {
                1.0
            } else {
                0.0
            }
        } else if d >= inner && d <= outer {
            1.0
        } else {
            0.0
        };
        if feather > 0.0 {
            let dist_edge = if edge == "None" {
                (radius - d).abs()
            } else {
                (d - inner).abs().min((outer - d).abs())
            };
            if cover > 0.0 {
                cover = (dist_edge / feather).clamp(0.0, 1.0);
            }
        }
        if invert {
            cover = 1.0 - cover;
        }
        composite(
            mode,
            [
                color.r * cover,
                color.g * cover,
                color.b * cover,
                cover * color.a,
            ],
            src.at(x, y),
        )
    }))
}

pub fn ellipse(src: &Image) -> ApplyResult<Image> {
    let rx = src.width as f32 / 2.0;
    let ry = src.height as f32 / 2.0;
    let cx = rx;
    let cy = ry;
    Ok(map_px(src, |x, y| {
        let nx = (x as f32 - cx) / rx.max(0.5);
        let ny = (y as f32 - cy) / ry.max(0.5);
        if nx * nx + ny * ny <= 1.0 {
            [1.0, 1.0, 1.0, 1.0]
        } else {
            [0.0; 4]
        }
    }))
}

pub fn eyedropper(inst: &Instance, src: &Image) -> ApplyResult<Image> {
    let point = inst.point_at("Sample Point", 0.0).unwrap_or([0.0, 0.0]);
    let radius = inst.num("Sample Radius").unwrap_or(0.0) as f32;
    let mode = inst
        .choice("Average Pixel Color")
        .unwrap_or_else(|| "All".into());
    let keep = inst.flag("Maintain Original Alpha").unwrap_or(false);
    let blend = inst.num("Blend With Original").unwrap_or(0.0) as f32 / 100.0;
    let mut acc = [0.0_f32; 4];
    let mut count = 0.0;
    let r = radius.ceil() as i32;
    for y in -r..=r {
        for x in -r..=r {
            if dist(0.0, 0.0, x as f32, y as f32) > radius.max(0.0) && radius > 0.0 {
                continue;
            }
            let px = src.get(point[0] as i32 + x, point[1] as i32 + y);
            let empty = px[3] <= 0.001;
            match mode.as_str() {
                "Skip Empty" if empty => continue,
                "All Premultiplied" => {
                    acc[0] += px[0] * px[3];
                    acc[1] += px[1] * px[3];
                    acc[2] += px[2] * px[3];
                    acc[3] += px[3];
                    count += 1.0;
                    continue;
                }
                "Including Alpha" => {
                    for c in 0..4 {
                        acc[c] += px[c];
                    }
                    count += 1.0;
                    continue;
                }
                "Skip Empty" | "All" => {
                    acc[0] += px[0];
                    acc[1] += px[1];
                    acc[2] += px[2];
                    acc[3] += if mode == "Including Alpha" {
                        px[3]
                    } else {
                        1.0
                    };
                    count += 1.0;
                }
                _ => {}
            }
        }
    }
    if count > 0.0 {
        for c in 0..4 {
            acc[c] /= count;
        }
    }
    if mode != "Including Alpha" {
        acc[3] = 1.0;
    }
    Ok(map_px(src, |x, y| {
        let base = src.at(x, y);
        let mut px = acc;
        if keep || (mode == "Including Alpha" && keep) {
            px[3] = base[3];
        }
        if mode == "Including Alpha" && keep {
            px[3] = base[3] * acc[3];
        }
        crate::image::lerp4(px, base, blend.clamp(0.0, 1.0))
    }))
}

pub fn fill(inst: &Instance, src: &Image) -> ApplyResult<Image> {
    let color = inst.role_color("color").unwrap_or(Rgba::white());
    Ok(map_px(src, |x, y| {
        let inside = inst
            .host
            .masks
            .iter()
            .any(|mask| mask.closed && point_in_poly(x as f32, y as f32, &mask.path));
        if inside {
            [color.r, color.g, color.b, color.a]
        } else {
            src.at(x, y)
        }
    }))
}

pub fn fractal(inst: &Instance, src: &Image) -> ApplyResult<Image> {
    let set = inst
        .choice("Set Choice")
        .unwrap_or_else(|| "Mandelbrot".into());
    let limit = inst.num("Escape Limit").unwrap_or(48.0).clamp(4.0, 200.0) as u32;
    let mag = inst.num("Magnification").unwrap_or(1.0);
    let x0 = inst.num("X (Real)").unwrap_or(-0.75);
    let y0 = inst.num("Y (Imaginary)").unwrap_or(0.0);
    let overlay = inst.flag("Overlay").unwrap_or(false);
    let transparent = inst.flag("Transparency").unwrap_or(false);
    let palette = inst
        .choice("Palette")
        .unwrap_or_else(|| "Black And White".into());
    let hue = inst.num("Hue").unwrap_or(0.0);
    let steps = inst.num("Cycle Steps").unwrap_or(8.0).max(1.0) as u32;
    let highlight = inst.flag("Edge Highlight").unwrap_or(false)
        && inst.host.quality != crate::host::Quality::Best;
    let oversample = inst.choice("Oversample Method");
    let factor = inst.num("Oversample Factor").unwrap_or(1.0).clamp(1.0, 4.0) as i32;
    let span = 1.5 / mag.abs().max(1e-4);
    Ok(map_px(src, |x, y| {
        let mut acc = [0.0_f32; 4];
        let n = if oversample.as_deref() == Some("Brute Force-Slow-Every Pixel") {
            factor
        } else {
            1
        };
        let mut count = 0.0;
        for oy in 0..n {
            for ox in 0..n {
                let fx = (x as f32 + ox as f32 / n as f32) / src.width.max(1) as f32 - 0.5;
                let fy = (y as f32 + oy as f32 / n as f32) / src.height.max(1) as f32 - 0.5;
                let cx = x0 + f64::from(fx) * span * if mag < 0.0 { -1.0 } else { 1.0 };
                let cy = y0 + f64::from(fy) * span;
                let (iter, inside) = iterate_set(&set, cx, cy, x0, y0, limit);
                if oversample.as_deref() == Some("Edge Detect-Fast-May Miss Pixels")
                    && inside
                    && ox + oy > 0
                {
                    continue;
                }
                let mut px = palette_color(&palette, iter, limit, inside, hue, steps, transparent);
                if highlight && iter + 1 == limit {
                    px[0] = 1.0;
                }
                if overlay && set.contains("Julia") {
                    let (it2, _) = iterate_set("Mandelbrot", cx, cy, x0, y0, limit.min(20));
                    if it2 == limit.min(20) {
                        px[0] = px[0] * 0.5 + 0.2;
                    }
                }
                for c in 0..4 {
                    acc[c] += px[c];
                }
                count += 1.0;
            }
        }
        if count == 0.0 {
            [0.0; 4]
        } else {
            acc.map(|v| v / count)
        }
    }))
}

fn iterate_set(set: &str, cx: f64, cy: f64, mx: f64, my: f64, limit: u32) -> (u32, bool) {
    let (mut zx, mut zy, c_x, c_y) = if set.contains("Julia") {
        (cx, cy, mx, my)
    } else {
        (0.0, 0.0, cx, cy)
    };
    if set.contains("Inverse") {
        // 逆集合は反復の内外を入れ替える材料として、始点をずらす。
        zx += 0.01;
    }
    for i in 0..limit {
        let x2 = zx * zx - zy * zy + c_x;
        let y2 = 2.0 * zx * zy + c_y;
        zx = x2;
        zy = y2;
        if zx * zx + zy * zy > 4.0 {
            return (i, false);
        }
    }
    let inside = true;
    if set.contains("Over Julia") {
        let _ = (mx, my);
    }
    (limit, inside)
}

fn palette_color(
    palette: &str,
    iter: u32,
    limit: u32,
    inside: bool,
    hue: f64,
    steps: u32,
    transparent: bool,
) -> [f32; 4] {
    if inside && !palette.starts_with("Solid") {
        let alpha = if transparent { 0.0 } else { 1.0 };
        return [0.0, 0.0, 0.0, alpha];
    }
    let t = iter as f32 / limit.max(1) as f32;
    match palette {
        "Hue Wheel" => {
            let h = (t + hue as f32).rem_euclid(1.0);
            let c = hue_color(h);
            [c[0], c[1], c[2], 1.0]
        }
        "Black And White" => {
            let band = ((iter / steps.max(1)) % 2) == 0;
            let v = if band { 1.0 } else { 0.0 };
            [v, v, v, 1.0]
        }
        "Solid Color" => {
            let h = hue as f32;
            let c = hue_color(h);
            if transparent {
                if inside {
                    [0.0, 0.0, 0.0, 0.0]
                } else {
                    [c[0], c[1], c[2], 1.0]
                }
            } else if inside {
                [c[0], c[1], c[2], 1.0]
            } else {
                [0.0, 0.0, 0.0, 0.0]
            }
        }
        _ => {
            let band = (iter % (steps.max(1) * 8)) as f32 / (steps.max(1) * 8) as f32;
            let v = if inside { 0.0 } else { (t * 8.0).fract() };
            let c = hue_color(hue as f32 + band);
            [
                c[0] * v,
                c[1] * v,
                c[2] * v,
                if transparent && v < 0.05 { 0.0 } else { 1.0 },
            ]
        }
    }
}

pub fn ramp(inst: &Instance, src: &Image) -> ApplyResult<Image> {
    let start = inst
        .point_at("Start Of Ramp", 0.0)
        .unwrap_or([0.0, src.height as f64 / 2.0]);
    let end = inst
        .point_at("End Of Ramp", 0.0)
        .unwrap_or([src.width as f64, src.height as f64 / 2.0]);
    let scatter = inst.num("Ramp Scatter").unwrap_or(0.0) as f32;
    let radial = inst.role_index("shape") == Some(1);
    let c0 = inst.role_color("c0").unwrap_or(Rgba::black());
    let c1 = inst.role_color("c1").unwrap_or(Rgba::white());
    Ok(map_px(src, |x, y| {
        let t = if radial {
            let d = dist(x as f32, y as f32, start[0] as f32, start[1] as f32);
            let max = dist(
                start[0] as f32,
                start[1] as f32,
                end[0] as f32,
                end[1] as f32,
            )
            .max(1.0);
            d / max
        } else {
            let dx = end[0] as f32 - start[0] as f32;
            let dy = end[1] as f32 - start[1] as f32;
            let len2 = dx * dx + dy * dy;
            if len2 < 1e-4 {
                0.0
            } else {
                ((x as f32 - start[0] as f32) * dx + (y as f32 - start[1] as f32) * dy) / len2
            }
        };
        let mut t = t.clamp(0.0, 1.0);
        if scatter > 0.0 {
            t = (t + (hash01(x, y, 19) - 0.5) * scatter).clamp(0.0, 1.0);
        }
        [
            c0.r + (c1.r - c0.r) * t,
            c0.g + (c1.g - c0.g) * t,
            c0.b + (c1.b - c0.b) * t,
            c0.a + (c1.a - c0.a) * t,
        ]
    }))
}

pub fn grid(inst: &Instance, src: &Image) -> ApplyResult<Image> {
    let anchor = inst.point_at("Anchor", 0.0).unwrap_or([0.0, 0.0]);
    let (w, h) = tile_size(inst, src);
    let border = inst.num("Border").unwrap_or(1.0) as f32;
    let feather = inst.num("Feather").unwrap_or(0.0) as f32;
    let invert = inst.flag("Invert Grid").unwrap_or(false);
    let color = inst.color_at("Color", 0.0).unwrap_or(Rgba::white());
    let opacity = inst.num("Opacity").unwrap_or(1.0) as f32;
    let mode = inst.blend_or_none("Blending Mode");
    Ok(map_px(src, |x, y| {
        if border <= 0.0 {
            return match mode {
                None => [0.0; 4],
                Some(_) => src.at(x, y),
            };
        }
        let lx = (x as f32 - anchor[0] as f32).rem_euclid(w);
        let ly = (y as f32 - anchor[1] as f32).rem_euclid(h);
        let dist_line = lx.min(w - lx).min(ly.min(h - ly));
        let mut on = dist_line <= border * 0.5;
        let mut a = if on { 1.0 } else { 0.0 };
        if feather > 0.0 {
            a = (1.0 - (dist_line - border * 0.5) / feather).clamp(0.0, 1.0);
            on = a > 0.0;
        }
        if invert {
            a = 1.0 - a;
        }
        let _ = on;
        composite(
            mode,
            [color.r, color.g, color.b, a * opacity * color.a],
            src.at(x, y),
        )
    }))
}

pub fn flare(inst: &Instance, src: &Image) -> ApplyResult<Image> {
    let center = inst
        .runtime
        .flare_center
        .unwrap_or([src.width as f64 / 2.0, src.height as f64 / 2.0]);
    Ok(map_px(src, |x, y| {
        let d = dist(x as f32, y as f32, center[0] as f32, center[1] as f32);
        let glow = (1.0 - d / 12.0).clamp(0.0, 1.0);
        let base = src.at(x, y);
        [
            base[0] + glow,
            base[1] + glow * 0.9,
            base[2] + glow * 0.6,
            base[3].max(glow),
        ]
    }))
}

pub fn bucket(inst: &Instance, src: &Image) -> ApplyResult<Image> {
    let point = inst.point_at("Fill Point", 0.0).unwrap_or([0.0, 0.0]);
    let tolerance = inst.num("Tolerance").unwrap_or(0.0) as f32;
    let selector = inst
        .choice("Fill Selector")
        .unwrap_or_else(|| "Color & Alpha".into());
    let view = inst.flag("View Threshold").unwrap_or(false);
    let stroke = inst.choice("Stroke").unwrap_or_else(|| "Antialias".into());
    let softness = inst.num("Feather Softness").unwrap_or(0.0) as f32;
    let spread = inst.num("Spread Radius").unwrap_or(0.0) as f32;
    let width = inst.num("Stroke Width").unwrap_or(1.0) as f32;
    let color = inst.color_at("Color", 0.0).unwrap_or(Rgba::white());
    let opacity = inst.num("Opacity").unwrap_or(1.0) as f32;
    let seed = src.get(point[0] as i32, point[1] as i32);
    let mut mask = vec![false; src.len()];
    let mut stack = vec![(point[0] as i32, point[1] as i32)];
    while let Some((x, y)) = stack.pop() {
        let Some(i) = src.index(x, y) else { continue };
        if mask[i] {
            continue;
        }
        let px = src.px[i];
        if !bucket_match(selector.as_str(), seed, px, tolerance) {
            continue;
        }
        mask[i] = true;
        stack.push((x + 1, y));
        stack.push((x - 1, y));
        stack.push((x, y + 1));
        stack.push((x, y - 1));
    }
    if stroke == "Spread" || stroke == "Choke" {
        mask = dilate(
            &mask,
            src.width,
            src.height,
            spread.round() as i32,
            stroke == "Choke",
        );
    }
    let edge = if stroke == "Stroke" {
        let thin = outline(&mask, src.width);
        let grow = (width.round() as i32 - 1).max(0);
        if grow == 0 {
            thin
        } else {
            dilate(&thin, src.width, src.height, grow, false)
        }
    } else {
        vec![false; mask.len()]
    };
    Ok(map_px(src, |x, y| {
        let i = (y * src.width + x) as usize;
        let on = if stroke == "Stroke" { edge[i] } else { mask[i] };
        if view && mask[i] {
            return [1.0, 1.0, 1.0, 1.0];
        }
        if !on {
            return src.at(x, y);
        }
        let mut a = opacity;
        if stroke == "Feather" && softness > 0.0 {
            a *= 0.5;
        }
        if stroke == "Antialias" {
            a *= 0.9;
        }
        let fill_only = inst.role_bool("fill_only").unwrap_or(false);
        if fill_only {
            return [color.r, color.g, color.b, a * color.a];
        }
        let mode = inst.blend_of("Blending Mode");
        composite(
            mode.or(Some(model::BlendMode::Normal)),
            [color.r, color.g, color.b, a * color.a],
            src.at(x, y),
        )
    }))
}

fn bucket_match(selector: &str, seed: [f32; 4], px: [f32; 4], tolerance: f32) -> bool {
    let rgb =
        |a: [f32; 4], b: [f32; 4]| (a[0] - b[0]).abs() + (a[1] - b[1]).abs() + (a[2] - b[2]).abs();
    match selector {
        "Straight Color" => rgb(seed, px) <= tolerance,
        "Transparency" => px[3] <= tolerance.max(0.01) && seed[3] <= tolerance.max(0.01),
        "Opacity" => px[3] >= 1.0 - tolerance.max(0.01) && seed[3] >= 1.0 - tolerance.max(0.01),
        "Alpha Channel" => (seed[3] > 0.5) == (px[3] > 0.5),
        _ => rgb(seed, px) + (seed[3] - px[3]).abs() <= tolerance,
    }
}

fn dilate(mask: &[bool], w: u32, h: u32, radius: i32, choke: bool) -> Vec<bool> {
    let mut out = mask.to_vec();
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let i = (y as u32 * w + x as u32) as usize;
            let mut hit = mask[i];
            for oy in -radius.abs()..=radius.abs() {
                for ox in -radius.abs()..=radius.abs() {
                    if ox * ox + oy * oy > radius * radius {
                        continue;
                    }
                    if let Some(j) = idx(w, h, x + ox, y + oy) {
                        hit = if choke {
                            hit && mask[j]
                        } else {
                            hit || mask[j]
                        };
                    } else if choke {
                        hit = false;
                    }
                }
            }
            out[i] = hit;
        }
    }
    out
}

fn outline(mask: &[bool], w: u32) -> Vec<bool> {
    mask.iter()
        .enumerate()
        .map(|(i, on)| {
            if !*on {
                return false;
            }
            let x = i as u32 % w;
            let y = i as u32 / w;
            let left = x > 0 && mask[i - 1];
            let right = x + 1 < w && mask[i + 1];
            !left || !right
        })
        .collect()
}

fn idx(w: u32, h: u32, x: i32, y: i32) -> Option<usize> {
    if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 {
        None
    } else {
        Some((y as u32 * w + x as u32) as usize)
    }
}

pub fn radio(inst: &Instance, src: &Image, time: f64) -> ApplyResult<Image> {
    let origin = inst
        .point_at("Producer Point", 0.0)
        .unwrap_or([src.width as f64 / 2.0, src.height as f64 / 2.0]);
    let freq = inst.num("Frequency").unwrap_or(1.0).max(0.01);
    let expansion = inst.num("Expansion").unwrap_or(20.0) as f32;
    let life = inst.num("Lifespan (sec)").unwrap_or(3.0);
    let fade_in = inst.num("Fade-in Time").unwrap_or(0.0).min(life);
    let fade_out = inst.num("Fade-out Time").unwrap_or(0.0).min(life);
    let reflect = inst.flag("Reflection").unwrap_or(false);
    let sides = inst.num("Sides").unwrap_or(32.0).max(3.0) as u32;
    let color = inst.color_at("Color", time).unwrap_or(Rgba::white());
    let opacity = inst.num("Opacity").unwrap_or(1.0) as f32;
    let start_w = inst.num("Start Width").unwrap_or(2.0) as f32;
    let end_w = inst.num("End Width").unwrap_or(2.0) as f32;
    let star = inst.flag("Star").unwrap_or(false);
    let star_depth = inst.num("Star Depth").unwrap_or(0.5) as f32;
    let curve = inst.num("Curvyness").unwrap_or(0.0) as f32;
    let orient = inst.num("Orientation").unwrap_or(0.0).to_radians() as f32;
    let spin = inst.num("Spin").unwrap_or(0.0) as f32;
    let velocity = inst.num("Velocity").unwrap_or(0.0) as f32;
    let direction = inst.num("Direction").unwrap_or(0.0).to_radians() as f32;
    let mut out = super::blank_like(src, src.width, src.height);
    let births = (life * freq).ceil() as i32 + 1;
    for n in 0..births {
        let age = time - n as f64 / freq;
        if age < 0.0 || age > life {
            continue;
        }
        let mut alpha = opacity;
        if fade_in > 0.0 {
            alpha *= (age / fade_in).clamp(0.0, 1.0) as f32;
        }
        if fade_out > 0.0 && age > life - fade_out {
            alpha *= ((life - age) / fade_out).clamp(0.0, 1.0) as f32;
        }
        let radius = expansion * age as f32;
        let width = start_w + (end_w - start_w) * (age / life) as f32;
        let ang = orient + spin * age as f32;
        let drift = if velocity > 0.0 {
            [
                direction.cos() * velocity * age as f32,
                -direction.sin() * velocity * age as f32,
            ]
        } else {
            [0.0, 0.0]
        };
        for y in 0..src.height {
            for x in 0..src.width {
                let mut px = x as f32 - origin[0] as f32 - drift[0];
                let mut py = y as f32 - origin[1] as f32 - drift[1];
                if reflect {
                    px = fold(px, src.width as f32);
                    py = fold(py, src.height as f32);
                }
                let d = (px * px + py * py).sqrt();
                let theta = py.atan2(px) + ang;
                let poly = poly_radius(theta, sides, star, star_depth, curve);
                if (d - radius * poly).abs() <= width * 0.5 {
                    stamp(
                        &mut out,
                        x as i32,
                        y as i32,
                        [color.r, color.g, color.b, alpha * color.a],
                    );
                }
            }
        }
    }
    let _ = inst.role_index("wave_type");
    Ok(out)
}

fn fold(v: f32, limit: f32) -> f32 {
    let mut v = v;
    let span = limit.max(1.0);
    while v > span / 2.0 {
        v = span - v;
    }
    while v < -span / 2.0 {
        v = -span - v;
    }
    v
}

fn poly_radius(theta: f32, sides: u32, star: bool, depth: f32, curve: f32) -> f32 {
    let step = TAU / sides as f32;
    let local = (theta.rem_euclid(step) / step - 0.5).abs();
    let mut r = 1.0 - local * curve.abs().min(2.0) * 0.2;
    if star {
        r *= if local > 0.25 { depth.max(0.05) } else { 1.0 };
    }
    if sides > 64 {
        r = 1.0;
    }
    r
}

pub fn scribble(inst: &Instance, src: &Image, time: f64) -> ApplyResult<Image> {
    let masks = masks_for_scribble(inst);
    let angle = inst.num("Angle").unwrap_or(0.0).to_radians() as f32;
    let width = inst.num("Stroke Width").unwrap_or(1.0).clamp(0.1, 50.0) as f32;
    let spacing = inst.num("Spacing").unwrap_or(width as f64) as f32;
    let start = inst.num("Start").unwrap_or(0.0) as f32 / 100.0;
    let end = inst.num("End").unwrap_or(100.0) as f32 / 100.0;
    let wiggle = inst
        .choice("Wiggle Type")
        .unwrap_or_else(|| "Static".into());
    let wps = inst.num("Wiggles/Second").unwrap_or(0.0);
    let composite_mode = inst
        .choice("Composite")
        .unwrap_or_else(|| "On Transparent".into());
    let phase = match wiggle.as_str() {
        "Jumpy" => (time * wps).floor() as f32,
        "Smooth" => (time * wps) as f32,
        _ => 0.0,
    };
    let mut out = match composite_mode.as_str() {
        "On Original Image" => src.clone(),
        _ => super::blank_like(src, src.width, src.height),
    };
    let dir_x = angle.cos();
    let dir_y = angle.sin();
    let nx = -dir_y;
    let ny = dir_x;
    for mask in masks {
        if !mask.closed && mask.path.len() < 3 {
            continue;
        }
        let (min_x, min_y, max_x, max_y) = bounds(&mask.path);
        let span = (max_x - min_x).hypot(max_y - min_y);
        let lines = (span / spacing.max(0.1)).ceil() as i32;
        for i in 0..lines {
            let t = i as f32 / lines.max(1) as f32;
            if t < start || t > end {
                continue;
            }
            let off = (i as f32 - lines as f32 / 2.0) * spacing + phase;
            let cx = (min_x + max_x) * 0.5 + nx * off;
            let cy = (min_y + max_y) * 0.5 + ny * off;
            for s in 0..40 {
                let u = s as f32 / 39.0;
                let x = cx + dir_x * (u - 0.5) * span;
                let y = cy + dir_y * (u - 0.5) * span;
                if point_in_poly(x, y, &mask.path) {
                    stamp(&mut out, x as i32, y as i32, [1.0, 1.0, 1.0, 1.0]);
                    if width > 1.5 {
                        stamp(&mut out, x as i32 + 1, y as i32, [1.0, 1.0, 1.0, 1.0]);
                    }
                }
            }
        }
    }
    if composite_mode == "Reveal Original Image" {
        for (px, base) in out.px.iter_mut().zip(src.px.iter()) {
            if px[3] > 0.0 {
                *px = *base;
            }
        }
    }
    Ok(out)
}

fn masks_for_scribble(inst: &Instance) -> Vec<MaskShape> {
    match inst.choice("Scribble").as_deref() {
        Some("All Masks") | Some("All Masks Using Modes") => inst.host.masks.clone(),
        _ => {
            if let Some(index) = inst.role_index("Mask") {
                inst.host
                    .masks
                    .iter()
                    .filter(|mask| mask.order == index)
                    .cloned()
                    .collect()
            } else {
                inst.host.masks.iter().take(1).cloned().collect()
            }
        }
    }
}

fn bounds(path: &[[f64; 2]]) -> (f32, f32, f32, f32) {
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;
    for p in path {
        min_x = min_x.min(p[0] as f32);
        min_y = min_y.min(p[1] as f32);
        max_x = max_x.max(p[0] as f32);
        max_y = max_y.max(p[1] as f32);
    }
    (min_x, min_y, max_x, max_y)
}

pub fn stroke(inst: &Instance, src: &Image) -> ApplyResult<Image> {
    let hardness = inst.num("Brush Hardness").unwrap_or(1.0) as f32;
    let spacing = inst.num("Spacing").unwrap_or(1.0) as f32;
    let color = inst.role_color("color").unwrap_or(Rgba::white());
    let opacity = inst.role_number("opacity").unwrap_or(1.0) as f32;
    let on_original = inst.role_index("paint_style").unwrap_or(0) == 0;
    let mut out = if on_original {
        src.clone()
    } else {
        super::blank_like(src, src.width, src.height)
    };
    for mask in &inst.host.masks {
        let path = &mask.path;
        if path.len() < 2 {
            continue;
        }
        let count = path.len() - if mask.closed { 0 } else { 1 };
        for i in 0..count {
            let a = path[i];
            let b = path[(i + 1) % path.len()];
            let len = dist(a[0] as f32, a[1] as f32, b[0] as f32, b[1] as f32);
            let steps = (len / spacing.max(0.2)).ceil() as i32;
            for s in 0..=steps {
                let t = s as f32 / steps.max(1) as f32;
                let x = a[0] as f32 + (b[0] as f32 - a[0] as f32) * t;
                let y = a[1] as f32 + (b[1] as f32 - a[1] as f32) * t;
                stamp(
                    &mut out,
                    x as i32,
                    y as i32,
                    [color.r, color.g, color.b, opacity * hardness],
                );
                if hardness < 1.0 {
                    stamp(
                        &mut out,
                        x as i32 + 1,
                        y as i32,
                        [color.r, color.g, color.b, opacity * (1.0 - hardness)],
                    );
                }
            }
        }
    }
    Ok(out)
}

pub fn vegas(inst: &Instance, src: &Image, time: f64) -> ApplyResult<Image> {
    let segments = inst.num("Segments").unwrap_or(8.0).max(1.0) as u32;
    let length = inst.num("Length").unwrap_or(0.5) as f32;
    let rotation = inst.number_at("Rotation", time).unwrap_or(0.0) as f32;
    let width = inst.num("Width").unwrap_or(2.0) as f32;
    let hardness = inst.num("Hardness").unwrap_or(1.0) as f32;
    let start_o = inst.num("Start Opacity").unwrap_or(1.0) as f32;
    let end_o = inst.num("End Opacity").unwrap_or(1.0) as f32;
    let mid_o = inst.num("Mid-point Opacity").unwrap_or(0.0) as f32;
    let mid_p = inst.num("Mid-point Position").unwrap_or(0.5) as f32;
    let color = inst.color_at("Color", time).unwrap_or(Rgba::white());
    let blend = inst
        .choice("Blend Mode")
        .unwrap_or_else(|| "Transparent".into());
    let even = inst.choice("Segment Distribution").as_deref() == Some("Even");
    let mut out = if blend == "Transparent" {
        super::blank_like(src, src.width, src.height)
    } else {
        src.clone()
    };
    let path = vegas_path(inst, src);
    if path.is_empty() {
        return Ok(out);
    }
    let max_len = 1.0 / segments as f32;
    let seg_len = (length * max_len).clamp(0.0, max_len);
    for s in 0..segments {
        let start = if even {
            s as f32 / segments as f32
        } else {
            s as f32 * seg_len
        };
        let t0 = (start + rotation).rem_euclid(1.0);
        let steps = 8;
        for k in 0..steps {
            let u = k as f32 / steps as f32;
            let t = (t0 + u * seg_len).rem_euclid(1.0);
            let idx = ((t * path.len() as f32) as usize).min(path.len() - 1);
            let (x, y) = path[idx];
            let alpha = if mid_o <= 0.0 {
                start_o + (end_o - start_o) * u
            } else if u < mid_p {
                start_o + (mid_o - start_o) * (u / mid_p.max(0.01))
            } else {
                mid_o + (end_o - mid_o) * ((u - mid_p) / (1.0 - mid_p).max(0.01))
            };
            let a = alpha * hardness.max(0.05);
            if blend == "Stencil" {
                let base = src.get(x, y);
                stamp(&mut out, x, y, [base[0], base[1], base[2], a * base[3]]);
            } else if blend != "Under" {
                stamp(&mut out, x, y, [color.r, color.g, color.b, a]);
            }
            if width > 1.5 {
                stamp(&mut out, x + 1, y, [color.r, color.g, color.b, a * 0.5]);
            }
        }
    }
    Ok(out)
}

fn vegas_path(inst: &Instance, src: &Image) -> Vec<(i32, i32)> {
    if inst.choice("Stroke").as_deref() == Some("Mask/Path") {
        let Some(mask) = inst.host.masks.first() else {
            return Vec::new();
        };
        let mut pts = Vec::new();
        if mask.path.len() < 2 {
            return pts;
        }
        let count = mask.path.len() - if mask.closed { 0 } else { 1 };
        for i in 0..count {
            let a = mask.path[i];
            let b = mask.path[(i + 1) % mask.path.len()];
            let steps = dist(a[0] as f32, a[1] as f32, b[0] as f32, b[1] as f32).ceil() as i32;
            for s in 0..steps.max(1) {
                let t = s as f32 / steps.max(1) as f32;
                pts.push((
                    (a[0] + (b[0] - a[0]) * t as f64) as i32,
                    (a[1] + (b[1] - a[1]) * t as f64) as i32,
                ));
            }
        }
        return pts;
    }
    let threshold = inst.num("Threshold").unwrap_or(50.0) as f32 / 100.0;
    let mut pts = Vec::new();
    for y in 0..src.height {
        for x in 0..src.width {
            let v = luma(src.at(x, y));
            if v >= threshold {
                pts.push((x as i32, y as i32));
            }
        }
    }
    pts
}

pub fn write_on(inst: &Instance, src: &Image, time: f64) -> ApplyResult<Image> {
    let keys = brush_keys(inst);
    let life = inst.num("Stroke Length (secs)").unwrap_or(0.0);
    let spacing = inst.num("Brush Spacing (secs)").unwrap_or(0.05).max(0.001);
    let style = inst
        .choice("Paint Style")
        .unwrap_or_else(|| "On Original Image".into());
    let paint_time = inst.role_index("paint_time").unwrap_or(0);
    let mut out = if style == "On Transparent" {
        super::blank_like(src, src.width, src.height)
    } else {
        src.clone()
    };
    if keys.is_empty() {
        if let Some(point) = inst.point_at("Brush Position", time) {
            stamp(
                &mut out,
                point[0] as i32,
                point[1] as i32,
                [1.0, 1.0, 1.0, 1.0],
            );
        }
        return Ok(out);
    }
    let mut t = keys[0].0;
    let end = time.min(keys.last().map(|key| key.0).unwrap_or(time));
    while t <= end + 1e-6 {
        if life > 0.0 && t < time - life {
            t += spacing;
            continue;
        }
        let point = point_along(&keys, t);
        let color = if paint_time == 1 {
            color_along(inst, t)
        } else {
            inst.role_color("color").unwrap_or(Rgba::white())
        };
        stamp(
            &mut out,
            point[0] as i32,
            point[1] as i32,
            [color.r, color.g, color.b, color.a],
        );
        t += spacing;
    }
    if style == "Reveal Original Image" {
        for (px, base) in out.px.iter_mut().zip(src.px.iter()) {
            if px[3] > 0.5 && px != base {
                *px = *base;
            }
        }
    }
    Ok(out)
}

fn brush_keys(inst: &Instance) -> Vec<(f64, [f64; 2])> {
    let mut keys = Vec::new();
    for (time, value) in inst.keys_of("Brush Position") {
        if let crate::host::Value::Point(point) = value {
            keys.push((time, point));
        }
    }
    if keys.is_empty() {
        let mut i = 0;
        while let Some(point) = inst.role_point(&format!("brush_key_{i}")) {
            let t = inst
                .role_number(&format!("brush_time_{i}"))
                .unwrap_or(i as f64);
            keys.push((t, point));
            i += 1;
        }
    }
    keys
}

fn point_along(keys: &[(f64, [f64; 2])], time: f64) -> [f64; 2] {
    if time <= keys[0].0 {
        return keys[0].1;
    }
    for pair in keys.windows(2) {
        if time <= pair[1].0 {
            let span = (pair[1].0 - pair[0].0).max(1e-6);
            let u = (time - pair[0].0) / span;
            return [
                pair[0].1[0] + (pair[1].1[0] - pair[0].1[0]) * u,
                pair[0].1[1] + (pair[1].1[1] - pair[0].1[1]) * u,
            ];
        }
    }
    keys.last().map(|key| key.1).unwrap_or([0.0, 0.0])
}

fn color_along(inst: &Instance, time: f64) -> Rgba {
    let keys = inst.keys_of("Color");
    let mut color = inst.role_color("color").unwrap_or(Rgba::white());
    for (t, value) in keys {
        if t <= time {
            if let crate::host::Value::Color(next) = value {
                color = next;
            }
        }
    }
    color
}

// evolution_at is private in distort. Re-export by calling through a pub wrapper.
// The import above expects it to be public. See distort.rs.
const _: EffectKind = EffectKind::CellPattern;
