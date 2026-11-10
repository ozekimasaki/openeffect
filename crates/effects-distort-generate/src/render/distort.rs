use std::f32::consts::TAU;

use effect_api::ApplyResult;

use super::{blank_like, map_px, sample_at as sample};
use crate::host::{Instance, LiquifyTool, Quality};
use crate::image::{
    apply_h, displace_amount, dist, fbm, homography, luma, point_in_poly, Filter, Image,
};

pub fn bezier(inst: &Instance, src: &Image) -> ApplyResult<Image> {
    if inst.runtime.edges.len() < 4 {
        return Ok(src.clone());
    }
    let w = (src.width.saturating_sub(1)) as f32;
    let h = (src.height.saturating_sub(1)) as f32;
    Ok(map_px(src, |x, y| {
        let (u, v) = invert_patch(inst, x as f32, y as f32, w, h);
        sample(src, u * w, v * h, Filter::Bilinear, false)
    }))
}

fn invert_patch(inst: &Instance, x: f32, y: f32, w: f32, h: f32) -> (f32, f32) {
    let mut u = if w == 0.0 { 0.0 } else { x / w };
    let mut v = if h == 0.0 { 0.0 } else { y / h };
    for _ in 0..6 {
        let (px, py) = patch(inst, u, v, w, h);
        let (ux, uy) = patch(inst, u + 0.002, v, w, h);
        let (vx, vy) = patch(inst, u, v + 0.002, w, h);
        let j00 = (ux - px) / 0.002;
        let j10 = (uy - py) / 0.002;
        let j01 = (vx - px) / 0.002;
        let j11 = (vy - py) / 0.002;
        let det = j00 * j11 - j01 * j10;
        if det.abs() < 1e-6 {
            break;
        }
        let ex = x - px;
        let ey = y - py;
        u += (ex * j11 - ey * j01) / det;
        v += (ey * j00 - ex * j10) / det;
        u = u.clamp(-0.25, 1.25);
        v = v.clamp(-0.25, 1.25);
    }
    (u.clamp(0.0, 1.0), v.clamp(0.0, 1.0))
}

fn patch(inst: &Instance, u: f32, v: f32, w: f32, h: f32) -> (f32, f32) {
    let tl = (0.0, 0.0);
    let tr = (w, 0.0);
    let br = (w, h);
    let bl = (0.0, h);
    let top = curve(inst.runtime.edges[0], u, tl, tr);
    let right = curve(inst.runtime.edges[1], v, tr, br);
    let bottom = curve(inst.runtime.edges[2], 1.0 - u, br, bl);
    let left = curve(inst.runtime.edges[3], 1.0 - v, bl, tl);
    let tb = lerp2(top, bottom, v);
    let lr = lerp2(left, right, u);
    let corners = (
        (1.0 - u) * (1.0 - v) * tl.0 + u * (1.0 - v) * tr.0 + (1.0 - u) * v * bl.0 + u * v * br.0,
        (1.0 - u) * (1.0 - v) * tl.1 + u * (1.0 - v) * tr.1 + (1.0 - u) * v * bl.1 + u * v * br.1,
    );
    (tb.0 + lr.0 - corners.0, tb.1 + lr.1 - corners.1)
}

fn curve(edge: ([f64; 2], [f64; 2], [f64; 2]), t: f32, a: (f32, f32), b: (f32, f32)) -> (f32, f32) {
    let (vertex, ta, tb) = edge;
    let c1 = (
        vertex[0] as f32 + ta[0] as f32,
        vertex[1] as f32 + ta[1] as f32,
    );
    let c2 = (
        vertex[0] as f32 + tb[0] as f32,
        vertex[1] as f32 + tb[1] as f32,
    );
    let u = 1.0 - t;
    let p0 = u * u * u;
    let p1 = 3.0 * u * u * t;
    let p2 = 3.0 * u * t * t;
    let p3 = t * t * t;
    (
        p0 * a.0 + p1 * c1.0 + p2 * c2.0 + p3 * b.0,
        p0 * a.1 + p1 * c1.1 + p2 * c2.1 + p3 * b.1,
    )
}

fn lerp2(a: (f32, f32), b: (f32, f32), t: f32) -> (f32, f32) {
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
}

pub fn bulge(inst: &Instance, src: &Image) -> ApplyResult<Image> {
    let rx = inst.num("Horizontal Radius").unwrap_or(0.0) as f32;
    let ry = inst.num("Vertical Radius").unwrap_or(0.0) as f32;
    let height = inst.num("Bulge Height").unwrap_or(0.0) as f32;
    let taper = inst.num("Taper Radius").unwrap_or(0.0) as f32;
    let pin = inst.flag("Pin All Edges").unwrap_or(false);
    let aa = inst.num("Antialiasing").unwrap_or(0.0) as f32;
    let filter = if inst.host.quality == Quality::Best && aa > 0.0 {
        Filter::Bilinear
    } else {
        Filter::Nearest
    };
    let cx = src.width as f32 / 2.0;
    let cy = src.height as f32 / 2.0;
    Ok(map_px(src, |x, y| {
        let px = x as f32;
        let py = y as f32;
        if rx <= f32::EPSILON || ry <= f32::EPSILON {
            return sample(src, px, py, filter, false);
        }
        let mut weight = 1.0;
        let nx = (px - cx) / rx;
        let ny = (py - cy) / ry;
        let r2 = nx * nx + ny * ny;
        if r2 < 1.0 {
            weight = (1.0 - r2).powf(taper.max(0.0));
        } else {
            weight = 0.0;
        }
        if pin {
            let edge = px
                .min(py)
                .min(src.width as f32 - 1.0 - px)
                .min(src.height as f32 - 1.0 - py);
            if edge <= 0.5 {
                weight = 0.0;
            }
        }
        let factor = 1.0 / (1.0 + height * weight);
        let sx = cx + (px - cx) * factor;
        let sy = cy + (py - cy) * factor;
        if filter == Filter::Bilinear {
            let a = sample(src, sx, sy, Filter::Bilinear, false);
            let b = sample(src, sx + 0.5, sy + 0.5, Filter::Bilinear, false);
            crate::image::lerp4(a, b, 0.5)
        } else {
            sample(src, sx, sy, Filter::Nearest, false)
        }
    }))
}

pub fn corner_pin(inst: &Instance, src: &Image) -> ApplyResult<Image> {
    if inst.runtime.pins.len() < 4 {
        return Ok(src.clone());
    }
    let w = (src.width.saturating_sub(1)) as f64;
    let h = (src.height.saturating_sub(1)) as f64;
    let src_quad = [[0.0, 0.0], [w, 0.0], [w, h], [0.0, h]];
    let dst = [
        inst.runtime.pins[0],
        inst.runtime.pins[1],
        inst.runtime.pins[2],
        inst.runtime.pins[3],
    ];
    let Some(hmat) = homography(dst, src_quad) else {
        return Ok(src.clone());
    };
    Ok(map_px(src, |x, y| {
        let (sx, sy) = apply_h(hmat, x as f64, y as f64);
        sample(src, sx as f32, sy as f32, Filter::Bilinear, false)
    }))
}

pub fn upscale(inst: &Instance, src: &Image) -> ApplyResult<Image> {
    let (scale_x, scale_y) = upscale_factor(inst, src);
    let w = ((src.width as f64) * scale_x).round().max(1.0) as u32;
    let h = ((src.height as f64) * scale_y).round().max(1.0) as u32;
    let noise = inst.num("Reduce Noise").unwrap_or(0.0) as f32;
    let detail = inst.num("Detail").unwrap_or(0.0) as f32;
    let bicubic = inst.choice("Alpha").as_deref() == Some("Bicubic");
    let color_filter = if bicubic {
        Filter::Bicubic
    } else {
        Filter::Bilinear
    };
    let mut out = blank_like(src, w, h);
    for y in 0..h {
        for x in 0..w {
            let sx = (x as f32 + 0.5) * src.width as f32 / w as f32 - 0.5;
            let sy = (y as f32 + 0.5) * src.height as f32 / h as f32 - 0.5;
            let mut px = if noise > 0.0 {
                let n = (noise as i32).clamp(0, 4);
                let mut acc = [0.0; 4];
                let mut count = 0.0;
                for oy in -n..=n {
                    for ox in -n..=n {
                        let s = sample(src, sx + ox as f32, sy + oy as f32, Filter::Nearest, false);
                        for c in 0..4 {
                            acc[c] += s[c];
                        }
                        count += 1.0;
                    }
                }
                acc.map(|v| v / count)
            } else {
                let rgb = sample(src, sx, sy, Filter::Bilinear, false);
                let alpha = sample(src, sx, sy, color_filter, false);
                [rgb[0], rgb[1], rgb[2], alpha[3]]
            };
            if detail > 0.0 {
                let crisp = sample(src, sx, sy, Filter::Nearest, false);
                for c in 0..3 {
                    px[c] += (crisp[c] - px[c]) * detail;
                }
            }
            out.set(x as i32, y as i32, px);
        }
    }
    Ok(out)
}

fn upscale_factor(inst: &Instance, src: &Image) -> (f64, f64) {
    if let Some(0) = inst.role_index("fit") {
        let scale = if src.width == 0 {
            1.0
        } else {
            inst.host.comp_width as f64 / src.width as f64
        };
        return (scale, scale);
    }
    if let Some(1) = inst.role_index("fit") {
        let scale = if src.height == 0 {
            1.0
        } else {
            inst.host.comp_height as f64 / src.height as f64
        };
        return (scale, scale);
    }
    let scale = inst.num("Scale").unwrap_or(1.0);
    (scale, scale)
}

pub fn displacement(inst: &Instance, src: &Image) -> ApplyResult<Image> {
    let max_x = inst.role_number("max_x").unwrap_or(0.0) as f32;
    let max_y = inst.role_number("max_y").unwrap_or(0.0) as f32;
    let ch_x = inst.role_index("channel_x").unwrap_or(0) as usize;
    let ch_y = inst.role_index("channel_y").unwrap_or(1) as usize;
    let behavior = inst.role_index("behavior").unwrap_or(1);
    let expand = inst.flag("Expand Output").unwrap_or(false);
    let wrap = inst.flag("Wrap Pixels Around").unwrap_or(false);
    let map = map_image(inst);
    let pad_x = if expand { max_x.abs().ceil() as i32 } else { 0 };
    let pad_y = if expand { max_y.abs().ceil() as i32 } else { 0 };
    let w = src.width as i32 + pad_x * 2;
    let h = src.height as i32 + pad_y * 2;
    let mut out = blank_like(src, w.max(1) as u32, h.max(1) as u32);
    for y in 0..out.height {
        for x in 0..out.width {
            let ox = x as i32 - pad_x;
            let oy = y as i32 - pad_y;
            let (mx, my) = map_coord(ox, oy, src, map.as_ref(), behavior);
            let sample_px = map
                .as_ref()
                .map(|image| image.sample(mx, my, Filter::Nearest, behavior == 2))
                .unwrap_or([0.5, 0.5, 0.5, 1.0]);
            let sample_px = if behavior == 0 && map_outside(ox, oy, src, map.as_ref()) {
                [0.5, 0.5, 0.5, 1.0]
            } else {
                sample_px
            };
            let ax = channel(sample_px, ch_x);
            let ay = channel(sample_px, ch_y);
            let bits = map.as_ref().map(|image| image.bits).unwrap_or(src.bits);
            let shift_x = displace_amount(ax, bits) * max_x;
            let shift_y = displace_amount(ay, bits) * max_y;
            let sx = ox as f32 - shift_x;
            let sy = oy as f32 - shift_y;
            out.set(
                x as i32,
                y as i32,
                sample(src, sx, sy, Filter::Bilinear, wrap),
            );
        }
    }
    Ok(out)
}

fn channel(px: [f32; 4], index: usize) -> f32 {
    if index == 4 {
        luma(px)
    } else {
        px[index.min(3)]
    }
}

fn map_outside(x: i32, y: i32, src: &Image, map: Option<&Image>) -> bool {
    let Some(map) = map else {
        return true;
    };
    let ox = (src.width as i32 - map.width as i32) / 2;
    let oy = (src.height as i32 - map.height as i32) / 2;
    let mx = x - ox;
    let my = y - oy;
    mx < 0 || my < 0 || mx >= map.width as i32 || my >= map.height as i32
}

fn map_image(inst: &Instance) -> Option<Image> {
    let id = inst.layer_value("Displacement Map Layer").flatten()?;
    let layer = inst.host.layers.iter().find(|layer| layer.id == id)?;
    Some(Image::from_frame(&layer.frame))
}

fn map_coord(x: i32, y: i32, src: &Image, map: Option<&Image>, behavior: u32) -> (f32, f32) {
    let Some(map) = map else {
        return (0.0, 0.0);
    };
    match behavior {
        0 => {
            let ox = (src.width as i32 - map.width as i32) / 2;
            let oy = (src.height as i32 - map.height as i32) / 2;
            ((x - ox) as f32 + 0.5, (y - oy) as f32 + 0.5)
        }
        2 => (
            (x.rem_euclid(map.width as i32)) as f32 + 0.5,
            (y.rem_euclid(map.height as i32)) as f32 + 0.5,
        ),
        _ => (
            (x as f32 + 0.5) * map.width as f32 / src.width.max(1) as f32,
            (y as f32 + 0.5) * map.height as f32 / src.height.max(1) as f32,
        ),
    }
}

pub fn liquify(inst: &Instance, src: &Image) -> ApplyResult<Image> {
    let mut disp = vec![[0.0_f32; 2]; src.len()];
    let radius = inst.role_number("brush_size").unwrap_or(0.0) as f32;
    let pressure = inst.role_number("brush_pressure").unwrap_or(1.0) as f32;
    let jitter = inst.role_number("turbulent_jitter").unwrap_or(0.5) as f32;
    let percent = inst.num("Distortion Percentage").unwrap_or(100.0) as f32 / 100.0;
    let offset = inst.role_point("mesh_offset").unwrap_or([0.0, 0.0]);
    let recon = inst
        .choice("Reconstruction Mode")
        .unwrap_or_else(|| "Revert".into());
    for stroke in &inst.runtime.strokes {
        apply_stroke(
            src,
            &mut disp,
            stroke.tool,
            stroke.from,
            stroke.to,
            stroke.hold,
            radius,
            pressure,
            jitter,
            recon.as_str(),
        );
    }
    let mask = inst.host.masks.first();
    let opacity = inst.num("Mask Opacity").unwrap_or(100.0) as f32 / 100.0;
    let feather = inst.num("Mask Feather").unwrap_or(0.0) as f32;
    let view_mesh = inst.role_bool("view_mesh").unwrap_or(false);
    let mut out = map_px(src, |x, y| {
        let i = (y * src.width + x) as usize;
        let mut d = disp[i];
        if let Some(mask) = mask {
            let inside = point_in_poly(x as f32, y as f32, &mask.path);
            let edge = crate::image::dist_to_poly(x as f32, y as f32, &mask.path);
            let mut keep = if inside { opacity } else { 0.0 };
            if feather > 0.0 {
                let t = (edge / feather).clamp(0.0, 1.0);
                keep = if inside {
                    opacity * t + (1.0 - t) * 0.0
                } else {
                    0.0
                };
                if !inside {
                    keep = (1.0 - t) * (1.0 - opacity);
                }
            }
            // 内側は Mask Opacity が高いほど歪まない。
            let freeze = if inside { opacity } else { 0.0 };
            let freeze = if feather > 0.0 {
                let t = (1.0 - (edge / feather).clamp(0.0, 1.0)).clamp(0.0, 1.0);
                if inside {
                    opacity
                } else {
                    opacity * t
                }
            } else {
                freeze
            };
            let _ = keep;
            d[0] *= 1.0 - freeze;
            d[1] *= 1.0 - freeze;
        }
        d[0] *= percent;
        d[1] *= percent;
        sample(
            src,
            x as f32 - d[0] + offset[0] as f32,
            y as f32 - d[1] + offset[1] as f32,
            Filter::Bilinear,
            false,
        )
    });
    if view_mesh {
        for y in (0..src.height).step_by(4) {
            for x in 0..src.width {
                out.set(x as i32, y as i32, [0.1, 0.9, 0.2, 1.0]);
            }
        }
    }
    Ok(out)
}

fn apply_stroke(
    src: &Image,
    disp: &mut [[f32; 2]],
    tool: LiquifyTool,
    from: [f64; 2],
    to: [f64; 2],
    hold: f64,
    radius: f32,
    pressure: f32,
    jitter: f32,
    recon: &str,
) {
    if radius <= 0.0 {
        return;
    }
    let strength = pressure * (if hold > 0.0 { hold as f32 } else { 1.0 });
    for y in 0..src.height {
        for x in 0..src.width {
            let px = x as f32;
            let py = y as f32;
            let d = dist(px, py, to[0] as f32, to[1] as f32);
            if d > radius {
                continue;
            }
            let fall = 1.0 - d / radius;
            let i = (y * src.width + x) as usize;
            let mut delta = [0.0_f32; 2];
            match tool {
                LiquifyTool::Warp => {
                    delta = [(to[0] - from[0]) as f32, (to[1] - from[1]) as f32];
                }
                LiquifyTool::Turbulence => {
                    let n = fbm(px * jitter.max(0.1), py * jitter.max(0.1), 3, 3);
                    let n2 = fbm(py * jitter.max(0.1), px * jitter.max(0.1), 3, 9);
                    delta = [(n - 0.5) * radius, (n2 - 0.5) * radius];
                }
                LiquifyTool::TwirlClockwise | LiquifyTool::TwirlCounterclockwise => {
                    let sign = if tool == LiquifyTool::TwirlClockwise {
                        1.0
                    } else {
                        -1.0
                    };
                    let dx = px - to[0] as f32;
                    let dy = py - to[1] as f32;
                    delta = [sign * dy, -sign * dx];
                }
                LiquifyTool::Pucker => {
                    delta = [to[0] as f32 - px, to[1] as f32 - py];
                }
                LiquifyTool::Bloat => {
                    delta = [px - to[0] as f32, py - to[1] as f32];
                }
                LiquifyTool::ShiftPixels => {
                    let dx = (to[0] - from[0]) as f32;
                    let dy = (to[1] - from[1]) as f32;
                    delta = [-dy, dx];
                }
                LiquifyTool::Reflection | LiquifyTool::Clone => {
                    delta = [(from[0] - to[0]) as f32, (from[1] - to[1]) as f32];
                }
                LiquifyTool::Reconstruction => {
                    delta = match recon {
                        "Displace" => [disp[i][0] * 0.25, disp[i][1] * 0.25],
                        "Amplitwist" => [-disp[i][1] * 0.5, disp[i][0] * 0.5],
                        "Affine" => [disp[i][0] * 0.2 - disp[i][1] * 0.2, disp[i][1] * 0.4],
                        _ => [-disp[i][0], -disp[i][1]],
                    };
                }
            }
            disp[i][0] += delta[0] * fall * strength;
            disp[i][1] += delta[1] * fall * strength;
        }
    }
}

pub fn magnify(inst: &Instance, src: &Image) -> ApplyResult<Image> {
    let center = inst
        .point_at("Center", 0.0)
        .unwrap_or([src.width as f64 / 2.0, src.height as f64 / 2.0]);
    let mag = inst.num("Magnification").unwrap_or(100.0);
    let factor = (mag / 100.0) as f32;
    let size = inst.num("Size").unwrap_or(0.0) as f32;
    let feather = inst.num("Feather").unwrap_or(0.0) as f32;
    let link = inst.choice("Link");
    let (radius, feather) = match link.as_deref() {
        Some("Size To Magnification") => (factor * size, feather),
        Some("Size & Feather To Magnification") => (factor * size, factor * feather),
        _ => (size, feather),
    };
    let opacity = inst.num("Opacity").unwrap_or(100.0) as f32 / 100.0;
    let shape = inst.role_index("shape").unwrap_or(0);
    let scaling = inst.choice("Scaling").unwrap_or_else(|| "Standard".into());
    let resize = inst.flag("Resize Layer").unwrap_or(false) && inst.enabled("Resize Layer");
    let blend_none = inst.role_index("blend_none") == Some(1);
    let pad = if resize { radius.ceil() as i32 } else { 0 };
    let mut out = if pad > 0 {
        crate::image::resize_canvas(
            src,
            src.width + pad as u32 * 2,
            src.height + pad as u32 * 2,
            pad,
            pad,
        )
    } else if blend_none {
        blank_like(src, src.width, src.height)
    } else {
        src.clone()
    };
    let origin = if pad > 0 { pad as f32 } else { 0.0 };
    for y in 0..out.height {
        for x in 0..out.width {
            let px = x as f32 - origin;
            let py = y as f32 - origin;
            let dx = px - center[0] as f32;
            let dy = py - center[1] as f32;
            let r = if shape == 1 {
                dx.abs().max(dy.abs())
            } else {
                (dx * dx + dy * dy).sqrt()
            };
            let edge = if feather <= 0.0 {
                if r <= radius {
                    1.0
                } else {
                    0.0
                }
            } else {
                (1.0 - (r - radius) / feather).clamp(0.0, 1.0)
            };
            if edge <= 0.0 {
                continue;
            }
            let filter = if scaling == "Soft" && factor > 1.0 {
                Filter::Bilinear
            } else {
                Filter::Nearest
            };
            let mut sx = center[0] as f32 + dx / factor.max(0.01);
            let mut sy = center[1] as f32 + dy / factor.max(0.01);
            if scaling == "Scatter" && factor > 1.0 {
                let n = crate::image::hash01(x, y, 11) - 0.5;
                sx += n * (factor - 1.0) * 2.0;
                sy += (crate::image::hash01(y, x, 13) - 0.5) * (factor - 1.0) * 2.0;
            }
            let mut sample_px = sample(src, sx, sy, filter, false);
            sample_px[3] *= opacity * edge;
            let base = out.at(x, y);
            out.set(x as i32, y as i32, crate::image::mix(base, sample_px, edge));
        }
    }
    Ok(out)
}

pub fn mesh(inst: &Instance, src: &Image) -> ApplyResult<Image> {
    let rows = inst.num("Rows").unwrap_or(1.0).clamp(1.0, 31.0) as u32;
    let cols = inst.num("Columns").unwrap_or(1.0).clamp(1.0, 31.0) as u32;
    let quality = inst.num("Quality").unwrap_or(1.0) as f32;
    let mut verts = vec![[0.0_f32; 2]; ((rows + 1) * (cols + 1)) as usize];
    for row in 0..=rows {
        for col in 0..=cols {
            let i = (row * (cols + 1) + col) as usize;
            verts[i] = [
                col as f32 * (src.width.saturating_sub(1)) as f32 / cols as f32,
                row as f32 * (src.height.saturating_sub(1)) as f32 / rows as f32,
            ];
        }
    }
    for point in &inst.runtime.mesh {
        if point.col <= cols && point.row <= rows {
            let i = (point.row * (cols + 1) + point.col) as usize;
            verts[i] = [point.at[0] as f32, point.at[1] as f32];
        }
    }
    let elastic = inst.role_number("elasticity").unwrap_or(0.0) as f32;
    Ok(map_px(src, |x, y| {
        let px = x as f32;
        let py = y as f32;
        for row in 0..rows {
            for col in 0..cols {
                let i00 = (row * (cols + 1) + col) as usize;
                let i10 = i00 + 1;
                let i01 = i00 + (cols + 1) as usize;
                let i11 = i01 + 1;
                if let Some((u, v)) =
                    inv_quad(verts[i00], verts[i10], verts[i11], verts[i01], px, py)
                {
                    let su = (col as f32 + u) / cols as f32;
                    let sv = (row as f32 + v) / rows as f32;
                    let bend = (quality.max(1.0)).ln() * 0.05 * elastic;
                    let sx = su * (src.width.saturating_sub(1)) as f32 + bend * (u - 0.5);
                    let sy = sv * (src.height.saturating_sub(1)) as f32 + bend * (v - 0.5);
                    return sample(src, sx, sy, Filter::Bilinear, false);
                }
            }
        }
        [0.0; 4]
    }))
}

fn inv_quad(
    a: [f32; 2],
    b: [f32; 2],
    c: [f32; 2],
    d: [f32; 2],
    x: f32,
    y: f32,
) -> Option<(f32, f32)> {
    let mut u = 0.5_f32;
    let mut v = 0.5_f32;
    for _ in 0..5 {
        let p = bil(a, b, c, d, u, v);
        let pu = bil(a, b, c, d, u + 0.01, v);
        let pv = bil(a, b, c, d, u, v + 0.01);
        let j00 = (pu[0] - p[0]) / 0.01;
        let j10 = (pu[1] - p[1]) / 0.01;
        let j01 = (pv[0] - p[0]) / 0.01;
        let j11 = (pv[1] - p[1]) / 0.01;
        let det = j00 * j11 - j01 * j10;
        if det.abs() < 1e-8 {
            return None;
        }
        u += ((x - p[0]) * j11 - (y - p[1]) * j01) / det;
        v += ((y - p[1]) * j00 - (x - p[0]) * j10) / det;
    }
    if (0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&v) {
        Some((u, v))
    } else {
        None
    }
}

fn bil(a: [f32; 2], b: [f32; 2], c: [f32; 2], d: [f32; 2], u: f32, v: f32) -> [f32; 2] {
    let ab = [a[0] + (b[0] - a[0]) * u, a[1] + (b[1] - a[1]) * u];
    let dc = [d[0] + (c[0] - d[0]) * u, d[1] + (c[1] - d[1]) * u];
    [ab[0] + (dc[0] - ab[0]) * v, ab[1] + (dc[1] - ab[1]) * v]
}

pub fn mirror(inst: &Instance, src: &Image) -> ApplyResult<Image> {
    let center = inst
        .point_at("Reflection Center", 0.0)
        .unwrap_or([src.width as f64 / 2.0, src.height as f64 / 2.0]);
    let angle = inst.num("Reflection Angle").unwrap_or(0.0).to_radians() as f32;
    let nx = angle.cos();
    let ny = angle.sin();
    Ok(map_px(src, |x, y| {
        let dx = x as f32 - center[0] as f32;
        let dy = y as f32 - center[1] as f32;
        let dot = dx * nx + dy * ny;
        if dot > 0.0 {
            sample(
                src,
                x as f32 - 2.0 * dot * nx,
                y as f32 - 2.0 * dot * ny,
                Filter::Bilinear,
                false,
            )
        } else {
            sample(src, x as f32, y as f32, Filter::Nearest, false)
        }
    }))
}

pub fn offset(inst: &Instance, src: &Image) -> ApplyResult<Image> {
    let center = inst
        .point_at("Shift Center To", 0.0)
        .unwrap_or([src.width as f64 / 2.0, src.height as f64 / 2.0]);
    let shift_x = center[0] as f32 - src.width as f32 / 2.0;
    let shift_y = center[1] as f32 - src.height as f32 / 2.0;
    let blend = inst.num("Blend With Original").unwrap_or(0.0) as f32 / 100.0;
    let filter = if inst.host.quality == Quality::Best {
        Filter::Bilinear
    } else {
        Filter::Nearest
    };
    Ok(map_px(src, |x, y| {
        let shifted = sample(src, x as f32 - shift_x, y as f32 - shift_y, filter, true);
        let original = sample(src, x as f32, y as f32, Filter::Nearest, false);
        crate::image::lerp4(shifted, original, blend.clamp(0.0, 1.0))
    }))
}

pub fn optics(inst: &Instance, src: &Image) -> ApplyResult<Image> {
    let fov = inst.num("Field Of View (FOV)").unwrap_or(0.0);
    let reverse = inst.flag("Reverse Lens Distortion").unwrap_or(false);
    let optimal = inst.flag("Optimal Pixels").unwrap_or(false);
    let center = inst
        .point_at("View Center", 0.0)
        .unwrap_or([src.width as f64 / 2.0, src.height as f64 / 2.0]);
    let axis = inst.role_index("fov_axis").unwrap_or(0);
    let k = (fov / 40.0) as f32 * 0.25;
    let norm = match axis {
        1 => src.height as f32 / 2.0,
        2 => dist(0.0, 0.0, src.width as f32, src.height as f32) / 2.0,
        _ => src.width as f32 / 2.0,
    }
    .max(1.0);
    let resize = if reverse {
        match inst.choice("Resize").as_deref() {
            Some("Max 2X") => 2.0_f32,
            Some("Max 4X") => 4.0,
            Some("Unlimited") => 1.0 + k.abs() * 4.0,
            _ => 1.0,
        }
    } else {
        1.0
    };
    let grow = if reverse {
        resize.min(1.0 + k.abs() * 4.0).max(1.0)
    } else {
        1.0
    };
    let w = ((src.width as f32) * grow).round().max(1.0) as u32;
    let h = ((src.height as f32) * grow).round().max(1.0) as u32;
    let mut out = blank_like(src, w, h);
    let ox = (w as f32 - src.width as f32) / 2.0;
    let oy = (h as f32 - src.height as f32) / 2.0;
    for y in 0..h {
        for x in 0..w {
            let px = x as f32 - ox;
            let py = y as f32 - oy;
            let dx = px - center[0] as f32;
            let dy = py - center[1] as f32;
            let r = (dx * dx + dy * dy).sqrt() / norm;
            let mut r2 = r;
            if reverse {
                let kk = if optimal { k * 0.45 } else { k };
                r2 = undistort(r, kk);
                if optimal {
                    r2 *= 0.92;
                }
            } else if !optimal {
                r2 = r * (1.0 + k * r * r);
            } else {
                r2 = r * (1.0 + k * 0.45 * r * r) / 0.92;
            }
            let scale = if r > 1e-5 { r2 / r } else { 1.0 };
            out.set(
                x as i32,
                y as i32,
                sample(
                    src,
                    center[0] as f32 + dx * scale,
                    center[1] as f32 + dy * scale,
                    Filter::Bilinear,
                    false,
                ),
            );
        }
    }
    Ok(out)
}

fn undistort(r: f32, k: f32) -> f32 {
    if k.abs() < 1e-6 {
        return r;
    }
    let mut x = r;
    for _ in 0..8 {
        let f = x * (1.0 + k * x * x) - r;
        let d = 1.0 + 3.0 * k * x * x;
        if d.abs() < 1e-6 {
            break;
        }
        x -= f / d;
    }
    x.max(0.0)
}

pub fn polar(inst: &Instance, src: &Image) -> ApplyResult<Image> {
    let t = inst.num("Interpolation").unwrap_or(0.0) as f32 / 100.0;
    let rect_to_polar = inst.choice("Type of Conversion").as_deref() != Some("Polar To Rect");
    let cx = (src.width.saturating_sub(1)) as f32 / 2.0;
    let cy = (src.height.saturating_sub(1)) as f32 / 2.0;
    let max_r = dist(0.0, 0.0, src.width as f32, src.height as f32) / 2.0;
    Ok(map_px(src, |x, y| {
        let px = x as f32;
        let py = y as f32;
        let (sx, sy) = if rect_to_polar {
            let angle = (px / src.width.max(1) as f32) * TAU;
            let r = (py / src.height.max(1) as f32) * max_r;
            (cx + r * angle.cos(), cy + r * angle.sin())
        } else {
            let dx = px - cx;
            let dy = py - cy;
            let angle = dy.atan2(dx);
            let r = (dx * dx + dy * dy).sqrt();
            (
                (angle.rem_euclid(TAU) / TAU) * src.width as f32,
                (r / max_r.max(1.0)) * src.height as f32,
            )
        };
        let fx = px + (sx - px) * t.clamp(0.0, 1.0);
        let fy = py + (sy - py) * t.clamp(0.0, 1.0);
        sample(src, fx, fy, Filter::Bilinear, false)
    }))
}

/// Polar To Rect の座標対応。半径 10、45° は (10, 45)。
pub fn polar_pair(radius: f64, angle_deg: f64) -> [f64; 2] {
    [radius, angle_deg]
}

pub fn reshape(inst: &Instance, src: &Image, time: f64) -> ApplyResult<Image> {
    let masks = &inst.host.masks;
    let source = mask_by(inst, "Source Mask", masks, 1);
    let dest = mask_by(inst, "Destination Mask", masks, 2);
    let boundary = mask_by(inst, "Boundary Mask", masks, 0);
    let percent = inst.number_at("Percent", time).unwrap_or(0.0) as f32 / 100.0;
    let elastic = inst.choice("Elasticity").unwrap_or_else(|| "Normal".into());
    let spread = match elastic.as_str() {
        "Stiff" => 0.15,
        "Less Stiff" => 0.3,
        "Below Normal" => 0.45,
        "Normal" | "Absolutely Normal" | "Above Average" => 0.7,
        "Loose" => 1.1,
        "Liquid" => 1.5,
        "Super Fluid" => 2.2,
        _ => 0.7,
    };
    Ok(map_px(src, |x, y| {
        let px = x as f32;
        let py = y as f32;
        if let Some(boundary) = boundary {
            if boundary.closed && !point_in_poly(px, py, &boundary.path) {
                return sample(src, px, py, Filter::Nearest, false);
            }
        }
        let Some(source) = source else {
            return sample(src, px, py, Filter::Nearest, false);
        };
        let Some(dest) = dest else {
            return sample(src, px, py, Filter::Nearest, false);
        };
        let (sx, sy) = centroid(&source.path);
        let (dx, dy) = centroid(&dest.path);
        let mut ox = (dx - sx) * percent;
        let mut oy = (dy - sy) * percent;
        if !inst.runtime.pairs.is_empty() {
            ox = 0.0;
            oy = 0.0;
            for (a, b) in &inst.runtime.pairs {
                let w = (1.0 / (dist(px, py, a[0] as f32, a[1] as f32) + 1.0)).powf(spread);
                ox += (b[0] as f32 - a[0] as f32) * percent * w;
                oy += (b[1] as f32 - a[1] as f32) * percent * w;
            }
        } else {
            let fall = (1.0 / (dist(px, py, sx, sy) + 1.0)).powf(1.0 / spread);
            ox *= fall;
            oy *= fall;
        }
        sample(src, px - ox, py - oy, Filter::Bilinear, false)
    }))
}

fn mask_by<'a>(
    inst: &'a Instance,
    name: &str,
    masks: &'a [crate::host::MaskShape],
    order_index: usize,
) -> Option<&'a crate::host::MaskShape> {
    if let Some(index) = inst.role_index(name) {
        return masks.iter().find(|mask| mask.order == index);
    }
    let mut ordered: Vec<_> = masks.iter().collect();
    ordered.sort_by_key(|mask| mask.order);
    ordered.into_iter().nth(order_index)
}

fn centroid(path: &[[f64; 2]]) -> (f32, f32) {
    if path.is_empty() {
        return (0.0, 0.0);
    }
    let mut x = 0.0;
    let mut y = 0.0;
    for p in path {
        x += p[0];
        y += p[1];
    }
    let n = path.len() as f64;
    ((x / n) as f32, (y / n) as f32)
}

pub fn ripple(inst: &Instance, src: &Image, time: f64) -> ApplyResult<Image> {
    let radius = inst.num("Radius").unwrap_or(0.0) as f32;
    let center = inst
        .point_at("Center Of Ripple", 0.0)
        .unwrap_or([src.width as f64 / 2.0, src.height as f64 / 2.0]);
    let cx = center[0] as f32;
    let cy = center[1] as f32;
    let reach = (radius / 100.0)
        * cx.max(src.width as f32 - cx)
            .max(cy)
            .max(src.height as f32 - cy);
    let width = inst.num("Wave Width").unwrap_or(1.0) as f32;
    let height = inst.num("Wave Height").unwrap_or(0.0) as f32;
    let speed = inst.num("Wave Speed").unwrap_or(0.0);
    let phase = inst.number_at("Ripple Phase", time).unwrap_or(0.0);
    let symmetric = inst.choice("Type Of Conversion").as_deref() == Some("Symmetric");
    Ok(map_px(src, |x, y| {
        if radius <= 0.0 {
            return sample(src, x as f32, y as f32, Filter::Nearest, false);
        }
        let dx = x as f32 - cx;
        let dy = y as f32 - cy;
        let d = (dx * dx + dy * dy).sqrt();
        if d > reach || reach <= 0.0 {
            return sample(src, x as f32, y as f32, Filter::Nearest, false);
        }
        let amp = height * (1.0 - d / reach);
        let theta =
            (d / width.max(0.001)) * TAU + phase.to_radians() as f32 + (time * speed) as f32 * TAU;
        let off = theta.sin() * amp;
        let (ox, oy) = if d < 1e-3 {
            (0.0, off)
        } else if symmetric {
            (dx / d * off, dy / d * off)
        } else {
            (off, dy / d * off)
        };
        sample(src, x as f32 - ox, y as f32 - oy, Filter::Bilinear, false)
    }))
}

pub fn rolling(inst: &Instance, src: &Image) -> ApplyResult<Image> {
    let rate = inst.num("Rolling Shutter Rate").unwrap_or(0.0) as f32 / 100.0;
    let scan = inst.role_index("scan").unwrap_or(0);
    let method = inst.choice("Method").unwrap_or_else(|| "Warp".into());
    let detail = if method == "Pixel Motion" {
        inst.num("Pixel Motion Detail").unwrap_or(1.0) as f32
    } else {
        1.0
    };
    let samples = if method == "Pixel Motion" {
        detail.round().clamp(1.0, 4.0) as i32
    } else if method == "Warp" && inst.flag("Detailed Analysis").unwrap_or(false) {
        4
    } else {
        1
    };
    Ok(map_px(src, |x, y| {
        let (along, limit) = if scan == 1 {
            (x as f32, src.width.max(1) as f32)
        } else {
            (y as f32, src.height.max(1) as f32)
        };
        let shift = (along / limit - 0.5) * rate * limit * 0.5;
        let mut acc = [0.0; 4];
        for s in 0..samples {
            let jitter = (s as f32 - (samples - 1) as f32 / 2.0) / detail.max(1.0);
            let (sx, sy) = if scan == 1 {
                (x as f32, y as f32 - shift + jitter)
            } else {
                (x as f32 - shift + jitter, y as f32)
            };
            let px = sample(src, sx, sy, Filter::Bilinear, false);
            for c in 0..4 {
                acc[c] += px[c];
            }
        }
        acc.map(|v| v / samples as f32)
    }))
}

pub fn smear(inst: &Instance, src: &Image, time: f64) -> ApplyResult<Image> {
    let masks = &inst.host.masks;
    let source = mask_by(inst, "Source Mask", masks, 1);
    let boundary = mask_by(inst, "Boundary Mask", masks, 0);
    let percent = inst.num("Percent").unwrap_or(100.0) as f32 / 100.0;
    let offset = inst.point_at("Mask Offset", time).unwrap_or([0.0, 0.0]);
    let rotation = inst
        .number_at("Mask Rotation", time)
        .unwrap_or(0.0)
        .to_radians() as f32;
    let scale = inst.number_at("Mask Scale", time).unwrap_or(100.0) as f32 / 100.0;
    let stiff = inst.role_index("elasticity").unwrap_or(0) == 0;
    Ok(map_px(src, |x, y| {
        let px = x as f32;
        let py = y as f32;
        if let Some(boundary) = boundary {
            if boundary.closed && !point_in_poly(px, py, &boundary.path) {
                return sample(src, px, py, Filter::Nearest, false);
            }
        }
        let Some(source) = source else {
            return sample(src, px, py, Filter::Nearest, false);
        };
        let (cx, cy) = centroid(&source.path);
        let dx = px - cx;
        let dy = py - cy;
        let cos = rotation.cos();
        let sin = rotation.sin();
        let rx = (dx * cos - dy * sin) * scale;
        let ry = (dx * sin + dy * cos) * scale;
        let fall = if stiff { 0.35 } else { 1.0 };
        let ox = (offset[0] as f32 + rx - dx) * percent * fall;
        let oy = (offset[1] as f32 + ry - dy) * percent * fall;
        let inside = point_in_poly(px, py, &source.path);
        let local = if inside { 1.0 } else { 0.45 };
        sample(
            src,
            px - ox * local,
            py - oy * local,
            Filter::Bilinear,
            false,
        )
    }))
}

pub fn spherize(inst: &Instance, src: &Image) -> ApplyResult<Image> {
    let filter = if inst.host.quality == Quality::Best {
        Filter::Bilinear
    } else {
        Filter::Nearest
    };
    let cx = src.width as f32 / 2.0;
    let cy = src.height as f32 / 2.0;
    let radius = (src.width.min(src.height) as f32) / 2.0;
    Ok(map_px(src, |x, y| {
        let dx = x as f32 - cx;
        let dy = y as f32 - cy;
        let d = (dx * dx + dy * dy).sqrt();
        if d < radius && radius > 0.0 {
            let r = d / radius;
            let z = (1.0 - r * r).max(0.0).sqrt();
            let k = 1.0 - 0.35 * z;
            sample(src, cx + dx * k, cy + dy * k, filter, false)
        } else {
            sample(src, x as f32, y as f32, filter, false)
        }
    }))
}

pub fn transform(inst: &Instance, src: &Image, time: f64) -> ApplyResult<Image> {
    let anchor = inst
        .point_at("Anchor Point", time)
        .unwrap_or([src.width as f64 / 2.0, src.height as f64 / 2.0]);
    let position = inst.point_at("Position", time).unwrap_or(anchor);
    let scale = inst.num("Scale").unwrap_or(1.0) as f32;
    let rotation = inst.number_at("Rotation", time).unwrap_or(0.0).to_radians() as f32;
    let opacity = inst.num("Opacity").unwrap_or(1.0) as f32;
    let skew = inst.num("Skew").unwrap_or(0.0).to_radians() as f32;
    let axis = inst.num("Skew Axis").unwrap_or(0.0).to_radians() as f32;
    let samples = motion_samples(inst);
    let mut acc = blank_like(src, src.width, src.height);
    for s in 0..samples {
        let shift = if samples == 1 {
            0.0
        } else {
            (s as f32 / (samples - 1) as f32 - 0.5) * rotation
        };
        let frame = map_px(src, |x, y| {
            let (sx, sy) = inverse_transform(
                x as f32,
                y as f32,
                anchor,
                position,
                scale,
                rotation + shift,
                skew,
                axis,
            );
            let mut px = sample(src, sx, sy, Filter::Bilinear, false);
            px[3] *= opacity;
            px
        });
        for (dst, src_px) in acc.px.iter_mut().zip(frame.px.iter()) {
            for c in 0..4 {
                *dst.get_mut(c).unwrap() += src_px[c] / samples as f32;
            }
        }
    }
    Ok(acc)
}

fn motion_samples(inst: &Instance) -> u32 {
    if !(inst.host.layer_motion_blur && inst.host.comp_motion_blur) {
        return 1;
    }
    let shutter = inst.host.shutter.unwrap_or(inst.host.comp_shutter);
    shutter.abs().clamp(1.0, 360.0) as u32 / 45 + 1
}

fn inverse_transform(
    x: f32,
    y: f32,
    anchor: [f64; 2],
    position: [f64; 2],
    scale: f32,
    rotation: f32,
    skew: f32,
    axis: f32,
) -> (f32, f32) {
    let mut dx = x - position[0] as f32;
    let mut dy = y - position[1] as f32;
    let cos = rotation.cos();
    let sin = rotation.sin();
    let rx = dx * cos + dy * sin;
    let ry = -dx * sin + dy * cos;
    dx = rx;
    dy = ry;
    let s = scale.abs().max(1e-4) * if scale < 0.0 { -1.0 } else { 1.0 };
    dx /= s;
    dy /= s;
    let shear = skew.tan();
    let ax = axis.cos();
    let ay = axis.sin();
    let along = dx * ax + dy * ay;
    let perp_x = dx - along * ax;
    let perp_y = dy - along * ay;
    dx = along * ax + perp_x - shear * along * ay;
    dy = along * ay + perp_y + shear * along * ax;
    (dx + anchor[0] as f32, dy + anchor[1] as f32)
}

pub fn turbulent(inst: &Instance, src: &Image, time: f64) -> ApplyResult<Image> {
    let kind = inst
        .choice("Displacement")
        .unwrap_or_else(|| "Turbulent".into());
    let amount = inst.num("Amount").unwrap_or(0.0) as f32;
    let size = inst.num("Size").unwrap_or(10.0).max(1.0) as f32;
    let offset = inst.role_point("offset").unwrap_or([0.0, 0.0]);
    let complexity = inst.num("Complexity").unwrap_or(1.0).clamp(1.0, 8.0) as u32;
    let evo = evolution_at(inst, time) as f32;
    let seed = inst.num("Random Seed").unwrap_or(0.0) as u32;
    let pin = inst.role_index("pin").unwrap_or(0);
    let resize = inst.flag("Resize Layer").unwrap_or(false);
    let pad = if resize {
        amount.abs().ceil() as u32
    } else {
        0
    };
    let mut out = blank_like(src, src.width + pad * 2, src.height + pad * 2);
    let smoother = kind.contains("Smoother");
    let octaves = if smoother {
        complexity.saturating_add(1)
    } else {
        complexity
    };
    let freq = if smoother { 0.45 } else { 1.0 };
    for y in 0..out.height {
        for x in 0..out.width {
            let px = x as f32 - pad as f32;
            let py = y as f32 - pad as f32;
            if pinned(pin, px, py, src) {
                out.set(
                    x as i32,
                    y as i32,
                    sample(src, px, py, Filter::Nearest, false),
                );
                continue;
            }
            let nx = (px + offset[0] as f32) / size;
            let ny = (py + offset[1] as f32 + evo * 4.0) / size;
            let n = fbm(nx * freq, ny * freq, octaves, seed) * 2.0 - 1.0;
            let n2 = fbm(ny * freq, nx * freq, octaves, seed.wrapping_add(5)) * 2.0 - 1.0;
            let (ox, oy) = match kind.as_str() {
                "Bulge" | "Bulge Smoother" => (
                    n * (px - src.width as f32 / 2.0).signum(),
                    n * (py - src.height as f32 / 2.0).signum(),
                ),
                "Twist" | "Twist Smoother" => (n2, -n),
                "Vertical Displacement" => (0.0, n),
                "Horizontal Displacement" => (n, 0.0),
                "Cross Displacement" => (n, n2),
                _ => (n, n2),
            };
            out.set(
                x as i32,
                y as i32,
                sample(
                    src,
                    px - ox * amount,
                    py - oy * amount,
                    Filter::Bilinear,
                    false,
                ),
            );
        }
    }
    Ok(out)
}

pub(crate) fn evolution_at(inst: &Instance, time: f64) -> f64 {
    let evo = inst.number_at("Evolution", time).unwrap_or(0.0);
    if inst.flag("Cycle Evolution").unwrap_or(false) {
        let cycle = inst.num("Cycle").unwrap_or(1.0).abs().max(1e-6);
        evo.rem_euclid(cycle)
    } else {
        evo
    }
}

fn pinned(pin: u32, x: f32, y: f32, src: &Image) -> bool {
    match pin {
        1 => y <= 0.5,
        2 => y >= src.height as f32 - 1.5,
        3 => x <= 0.5,
        4 => x >= src.width as f32 - 1.5,
        _ => false,
    }
}

pub fn twirl(inst: &Instance, src: &Image) -> ApplyResult<Image> {
    let angle = inst.num("Angle").unwrap_or(0.0).to_radians() as f32;
    let radius_pct = inst.num("Twirl Radius").unwrap_or(0.0) as f32;
    let radius = (radius_pct / 100.0) * src.width.max(src.height) as f32;
    let cx = src.width as f32 / 2.0;
    let cy = src.height as f32 / 2.0;
    Ok(map_px(src, |x, y| {
        let dx = x as f32 - cx;
        let dy = y as f32 - cy;
        let d = (dx * dx + dy * dy).sqrt();
        if radius <= 0.0 || d >= radius {
            return sample(src, x as f32, y as f32, Filter::Nearest, false);
        }
        let t = 1.0 - d / radius;
        let a = -angle * t;
        let cos = a.cos();
        let sin = a.sin();
        sample(
            src,
            cx + dx * cos - dy * sin,
            cy + dx * sin + dy * cos,
            Filter::Bilinear,
            false,
        )
    }))
}

pub fn warp_style(inst: &Instance, src: &Image) -> ApplyResult<Image> {
    let style = inst.role_index("style").unwrap_or(0);
    Ok(map_px(src, |x, y| {
        let u = x as f32 / src.width.max(1) as f32;
        let v = y as f32 / src.height.max(1) as f32;
        let (ox, oy) = match style {
            1 => ((v * TAU).sin() * 2.0, 0.0),
            2 => (0.0, (u - 0.5).abs() * 4.0),
            _ => (0.0, (u * TAU).sin() * 2.0),
        };
        sample(src, x as f32 - ox, y as f32 - oy, Filter::Bilinear, false)
    }))
}

pub fn stabilizer(inst: &Instance, src: &Image, time: f64) -> ApplyResult<Image> {
    if inst.runtime.analysis.cancelled {
        return Ok(src.clone());
    }
    if !inst.runtime.analysis.done {
        return Ok(src.clone());
    }
    let shift = shift_at(&inst.runtime.analysis.stab_shift, &inst.host.motion, time);
    let framing = inst
        .choice("Framing")
        .unwrap_or_else(|| "Stabilize, Crop, Auto-scale".into());
    let extra = inst.num("Additional Scale").unwrap_or(100.0) as f32 / 100.0;
    let max_scale = inst.num("Maximum Scale").unwrap_or(150.0) as f32 / 100.0;
    let margin = inst.num("Action-Safe Margin").unwrap_or(0.0) as f32 / 100.0;
    let preserve = inst.flag("Preserve Scale").unwrap_or(false);
    let mut scale = if preserve { 1.0 } else { extra };
    if framing == "Stabilize, Crop, Auto-scale" {
        scale = scale.max(1.05).min(max_scale);
    }
    if framing == "Stabilize Only"
        || framing == "Stabilize, Crop"
        || framing == "Stabilize, Synthesize Edges"
    {
        if framing != "Stabilize, Crop, Auto-scale" {
            scale = if preserve { 1.0 } else { extra };
        }
    }
    let crop = framing == "Stabilize, Crop" || framing == "Stabilize, Crop, Auto-scale";
    Ok(map_px(src, |x, y| {
        let cx = src.width as f32 / 2.0;
        let cy = src.height as f32 / 2.0;
        let mut sx = cx + (x as f32 - cx) / scale + shift[0];
        let mut sy = cy + (y as f32 - cy) / scale + shift[1];
        if margin > 0.0 && framing == "Stabilize, Crop, Auto-scale" {
            let inset_x = src.width as f32 * margin;
            let inset_y = src.height as f32 * margin;
            let xf = x as f32;
            let yf = y as f32;
            if xf < inset_x
                || yf < inset_y
                || xf > src.width as f32 - inset_x
                || yf > src.height as f32 - inset_y
            {
                return sample(src, xf + shift[0], yf + shift[1], Filter::Bilinear, false);
            }
        }
        if crop {
            sx = sx.clamp(0.0, src.width as f32 - 1.0);
            sy = sy.clamp(0.0, src.height as f32 - 1.0);
        }
        if framing == "Stabilize, Synthesize Edges" {
            let px = sample(src, sx, sy, Filter::Bilinear, false);
            if px[3] <= 0.01 {
                let feather = inst.num("Synthesis Edge Feather").unwrap_or(0.0) as f32;
                let other = sample(src, sx - shift[0], sy - shift[1], Filter::Bilinear, false);
                return crate::image::mix(other, px, (1.0 - feather / 10.0).clamp(0.0, 1.0));
            }
            return px;
        }
        sample(src, sx, sy, Filter::Bilinear, false)
    }))
}

fn shift_at(shifts: &[[f64; 2]], motion: &[crate::host::MotionSample], time: f64) -> [f32; 2] {
    if shifts.is_empty() {
        return [0.0; 2];
    }
    let mut index = 0;
    for (i, sample) in motion.iter().enumerate() {
        if (sample.time - time).abs() < 1e-4 {
            index = i;
            break;
        }
        if sample.time <= time {
            index = i;
        }
    }
    let shift = shifts.get(index).copied().unwrap_or(shifts[0]);
    [shift[0] as f32, shift[1] as f32]
}

pub fn wave(inst: &Instance, src: &Image, time: f64) -> ApplyResult<Image> {
    let height = inst.num("Wave Height").unwrap_or(1.0) as f32;
    let width = inst.num("Wave Width").unwrap_or(0.0) as f32;
    let direction = inst.num("Direction").unwrap_or(0.0);
    let speed = inst.num("Wave Speed").unwrap_or(0.0);
    let phase = inst.num("Phase").unwrap_or(0.0);
    let aa = inst.num("Antialiasing").unwrap_or(0.0);
    let pin = inst.role_index("pin").unwrap_or(0);
    let kind = inst.role_index("wave_type").unwrap_or(2);
    let filter = if inst.host.quality == Quality::Best && aa > 0.0 {
        Filter::Bilinear
    } else {
        Filter::Nearest
    };
    let (dx, dy) = wave_dir(direction);
    Ok(map_px(src, |x, y| {
        if pinned(pin, x as f32, y as f32, src) {
            return sample(src, x as f32, y as f32, Filter::Nearest, false);
        }
        let along = x as f32 * dx + y as f32 * dy;
        let mut off = wave_sample(along, height, phase, speed, time) * width;
        if kind == 0 {
            off = off.signum() * width;
        } else if kind == 1 {
            let radial = dist(
                x as f32,
                y as f32,
                src.width as f32 / 2.0,
                src.height as f32 / 2.0,
            );
            off = wave_sample(radial, height, phase, speed, time) * width;
        }
        let px = -dy;
        let py = dx;
        sample(src, x as f32 - px * off, y as f32 - py * off, filter, false)
    }))
}

pub fn wave_dir(degrees: f64) -> (f32, f32) {
    let r = degrees.to_radians();
    (r.cos() as f32, -r.sin() as f32)
}

/// 0° は下り坂の中点、90° は谷。速度はサイクル/秒。
pub fn wave_sample(along: f32, wavelength: f32, phase_deg: f64, speed: f64, time: f64) -> f32 {
    let wave = if wavelength.abs() < 1e-4 {
        0.0
    } else {
        along / wavelength * TAU
    };
    let phase = phase_deg.to_radians() as f32 + (time * speed) as f32 * TAU;
    -(wave + phase).sin()
}
