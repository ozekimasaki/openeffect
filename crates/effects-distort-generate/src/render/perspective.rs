use effect_api::ApplyResult;
use model::Rgba;

use crate::host::Instance;
use crate::image::{dist, luma, Filter, Image};

use super::{map_px, sample_at};

pub fn tracker(inst: &Instance, src: &Image) -> ApplyResult<Image> {
    let mut out = src.clone();
    if inst.flag("Render Track Points").unwrap_or(false) && inst.runtime.analysis.done {
        let size = inst.num("Track Point Size").unwrap_or(2.0) as i32;
        for track in &inst.host.tracks {
            stamp_box(
                &mut out,
                track.xy[0] as i32,
                track.xy[1] as i32,
                size.max(1),
            );
        }
    }
    Ok(out)
}

fn stamp_box(img: &mut Image, x: i32, y: i32, size: i32) {
    for oy in -size..=size {
        for ox in -size..=size {
            img.set(x + ox, y + oy, [1.0, 0.2, 0.2, 1.0]);
        }
    }
}

pub fn glasses(inst: &Instance, src: &Image) -> ApplyResult<Image> {
    let left = layer_image(inst, "Left View").unwrap_or_else(|| src.clone());
    let right = layer_image(inst, "Right View").unwrap_or_else(|| src.clone());
    let view = inst
        .choice("3D View")
        .unwrap_or_else(|| "Stereo Pair (Side by Side)".into());
    let swap = inst.flag("Swap Left-Right").unwrap_or(false);
    let (mut left, mut right) = if swap { (right, left) } else { (left, right) };
    let conv = if inst.enabled("Convergence Offset") {
        inst.num("Convergence Offset").unwrap_or(0.0) as f32
    } else {
        0.0
    };
    let vert = inst.num("Vertical Alignment").unwrap_or(0.0) as f32;
    let units = inst.choice("Units").unwrap_or_else(|| "Pixels".into());
    let conv = if units == "% of Source" {
        conv / 100.0 * src.width as f32
    } else {
        conv
    };
    let vert = if units == "% of Source" {
        vert / 100.0 * src.height as f32
    } else {
        vert
    };
    shift(&mut right, conv, vert);
    let balance = inst.num("Balance").unwrap_or(0.0) as f32;
    match view.as_str() {
        "Over Under" => {
            let mut out = super::blank_like(src, src.width, src.height);
            blit(&mut out, &left, 0, 0, src.width, src.height / 2, false);
            blit(
                &mut out,
                &right,
                0,
                (src.height / 2) as i32,
                src.width,
                src.height / 2,
                false,
            );
            Ok(out)
        }
        "Interlace Upper L Lower R" => Ok(map_px(src, |x, y| {
            if y % 2 == 0 {
                left.at(x, y)
            } else {
                right.at(x, y)
            }
        })),
        "Red Green LR" => Ok(anaglyph(
            &left,
            &right,
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            0.0,
        )),
        "Red Blue LR" => Ok(anaglyph(
            &left,
            &right,
            [1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            0.0,
        )),
        "Balanced Red Green LR" => Ok(anaglyph(
            &left,
            &right,
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            balance,
        )),
        "Balanced Red Blue LR" => Ok(anaglyph(
            &left,
            &right,
            [1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            balance,
        )),
        "Balanced Colored Red Blue" => Ok(anaglyph(
            &left,
            &right,
            [1.0, 0.2, 0.2],
            [0.2, 0.2, 1.0],
            balance,
        )),
        _ => {
            let mut out = super::blank_like(src, src.width, src.height);
            let half = src.width / 2;
            blit(&mut out, &left, 0, 0, half, src.height, true);
            blit(&mut out, &right, half as i32, 0, half, src.height, true);
            Ok(out)
        }
    }
}

fn layer_image(inst: &Instance, name: &str) -> Option<Image> {
    let id = inst.layer_value(name).flatten()?;
    let layer = inst.host.layers.iter().find(|layer| layer.id == id)?;
    Some(Image::from_frame(&layer.frame))
}

fn shift(img: &mut Image, dx: f32, dy: f32) {
    let copy = img.clone();
    for y in 0..img.height {
        for x in 0..img.width {
            img.set(
                x as i32,
                y as i32,
                sample_at(&copy, x as f32 - dx, y as f32 - dy, Filter::Bilinear, false),
            );
        }
    }
}

fn blit(dst: &mut Image, src: &Image, ox: i32, oy: i32, w: u32, h: u32, squeeze: bool) {
    for y in 0..h {
        for x in 0..w {
            let sx = if squeeze {
                x as f32 * src.width as f32 / w.max(1) as f32
            } else {
                x as f32
            };
            let sy = y as f32 * src.height as f32 / h.max(1) as f32;
            dst.set(
                ox + x as i32,
                oy + y as i32,
                sample_at(src, sx, sy, Filter::Bilinear, false),
            );
        }
    }
}

fn anaglyph(
    left: &Image,
    right: &Image,
    left_tint: [f32; 3],
    right_tint: [f32; 3],
    balance: f32,
) -> Image {
    let mut out = left.clone();
    for y in 0..left.height.min(right.height) {
        for x in 0..left.width.min(right.width) {
            let l = luma(left.at(x, y));
            let r = luma(right.at(x, y));
            let sat = 1.0 + balance;
            let mut px = [
                (r * right_tint[0] + l * left_tint[0]) * sat,
                (r * right_tint[1] + l * left_tint[1]) * sat,
                (r * right_tint[2] + l * left_tint[2]) * sat,
                1.0,
            ];
            if balance <= 0.0 {
                px = [
                    r * right_tint[0] + l * left_tint[0],
                    r * right_tint[1] + l * left_tint[1],
                    r * right_tint[2] + l * left_tint[2],
                    1.0,
                ];
            }
            out.set(x as i32, y as i32, px);
        }
    }
    out
}

pub fn bevel(src: &Image, soft: bool) -> ApplyResult<Image> {
    let radius = if soft { 2 } else { 1 };
    Ok(map_px(src, |x, y| {
        let center = src.at(x, y);
        if center[3] <= 0.01 {
            return center;
        }
        let mut edge = 0.0_f32;
        for oy in -radius..=radius {
            for ox in -radius..=radius {
                let a = src.get(x as i32 + ox, y as i32 + oy)[3];
                if a < 0.5 {
                    edge = 1.0;
                }
            }
        }
        if edge == 0.0 {
            return center;
        }
        let light = if soft {
            let dx = x as f32 / src.width.max(1) as f32;
            let dy = y as f32 / src.height.max(1) as f32;
            (1.2 - dx - dy).clamp(0.4, 1.3)
        } else if x == 0 || y == 0 {
            1.35
        } else {
            0.65
        };
        [
            (center[0] * light).min(if src.bits == model::BitsPerChannel::Bpc32 {
                4.0
            } else {
                1.0
            }),
            (center[1] * light).min(if src.bits == model::BitsPerChannel::Bpc32 {
                4.0
            } else {
                1.0
            }),
            (center[2] * light).min(if src.bits == model::BitsPerChannel::Bpc32 {
                4.0
            } else {
                1.0
            }),
            center[3],
        ]
    }))
}

pub fn drop_shadow(inst: &Instance, src: &Image) -> ApplyResult<Image> {
    let only = inst.flag("Shadow Only").unwrap_or(false);
    let distance = inst.role_number("distance").unwrap_or(4.0) as f32;
    let angle = inst.role_number("angle").unwrap_or(45.0).to_radians() as f32;
    let dx = angle.cos() * distance;
    let dy = angle.sin() * distance;
    let softness = inst.role_number("softness").unwrap_or(1.0) as i32;
    let color = inst.role_color("color").unwrap_or(Rgba::black());
    let opacity = inst.role_number("opacity").unwrap_or(0.75) as f32;
    let mut out = super::blank_like(src, src.width, src.height);
    for y in 0..src.height {
        for x in 0..src.width {
            let a = src.at(x, y)[3];
            if a <= 0.0 {
                continue;
            }
            for oy in -softness..=softness {
                for ox in -softness..=softness {
                    let px = x as f32 + dx + ox as f32;
                    let py = y as f32 + dy + oy as f32;
                    let cur = out.get(px as i32, py as i32);
                    let add = a * opacity / (1 + ox.abs() + oy.abs()) as f32;
                    out.set(
                        px as i32,
                        py as i32,
                        [color.r, color.g, color.b, (cur[3] + add).min(1.0)],
                    );
                }
            }
        }
    }
    if !only {
        for y in 0..src.height {
            for x in 0..src.width {
                let base = src.at(x, y);
                let shadow = out.at(x, y);
                let a = base[3] + shadow[3] * (1.0 - base[3]);
                out.set(
                    x as i32,
                    y as i32,
                    [
                        base[0] * base[3] + shadow[0] * shadow[3] * (1.0 - base[3]),
                        base[1] * base[3] + shadow[1] * shadow[3] * (1.0 - base[3]),
                        base[2] * base[3] + shadow[2] * shadow[3] * (1.0 - base[3]),
                        a,
                    ],
                );
            }
        }
    }
    let _ = dist;
    Ok(out)
}

pub fn radial_shadow(inst: &Instance, src: &Image) -> ApplyResult<Image> {
    let color = inst.color_at("Shadow Color", 0.0).unwrap_or(Rgba::black());
    let opacity = inst.num("Opacity").unwrap_or(1.0) as f32;
    let light = inst
        .point_at("Light Source", 0.0)
        .unwrap_or([src.width as f64 / 2.0, src.height as f64 / 2.0]);
    let distance = inst.num("Projection Distance").unwrap_or(0.0) as f32;
    let softness = inst.num("Softness").unwrap_or(0.0) as f32;
    let glass = inst.choice("Render").as_deref() == Some("Glass Edge");
    let influence = if glass {
        inst.num("Color Influence").unwrap_or(100.0) as f32 / 100.0
    } else {
        0.0
    };
    let only = inst.flag("Shadow Only").unwrap_or(false);
    let resize = inst.flag("Resize Layer").unwrap_or(false);
    let pad = if resize {
        distance.abs().ceil() as u32 + 2
    } else {
        0
    };
    let mut out = super::blank_like(src, src.width + pad * 2, src.height + pad * 2);
    for y in 0..src.height {
        for x in 0..src.width {
            let px = src.at(x, y);
            if px[3] <= 0.01 {
                continue;
            }
            let dx = x as f32 - light[0] as f32;
            let dy = y as f32 - light[1] as f32;
            let len = (dx * dx + dy * dy).sqrt().max(1.0);
            let sx = x as f32 + dx / len * distance;
            let sy = y as f32 + dy / len * distance;
            let mut shadow = if glass {
                let mixed = [
                    color.r * (1.0 - influence) + px[0] * influence,
                    color.g * (1.0 - influence) + px[1] * influence,
                    color.b * (1.0 - influence) + px[2] * influence,
                    opacity * px[3],
                ];
                mixed
            } else {
                [color.r, color.g, color.b, opacity * px[3]]
            };
            if softness > 0.0 {
                shadow[3] *= 0.65;
            }
            out.set((sx + pad as f32) as i32, (sy + pad as f32) as i32, shadow);
        }
    }
    if !only {
        for y in 0..src.height {
            for x in 0..src.width {
                let base = src.at(x, y);
                let i_x = x as i32 + pad as i32;
                let i_y = y as i32 + pad as i32;
                let shadow = out.get(i_x, i_y);
                let a = base[3] + shadow[3] * (1.0 - base[3]);
                out.set(
                    i_x,
                    i_y,
                    [
                        base[0] * base[3] + shadow[0] * shadow[3] * (1.0 - base[3]),
                        base[1] * base[3] + shadow[1] * shadow[3] * (1.0 - base[3]),
                        base[2] * base[3] + shadow[2] * shadow[3] * (1.0 - base[3]),
                        a,
                    ],
                );
            }
        }
    }
    Ok(out)
}
