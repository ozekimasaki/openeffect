use model::BitsPerChannel;

use crate::bag::Bag;
use crate::catalog::{Category, EffectDef};
use crate::cx::{Exposed, PaintOut, RenderCx};
use crate::image::{luma, Audio, Image};
use crate::values::Slot;

pub fn paint(
    effect: &EffectDef,
    src: &Image,
    exposed: Option<Exposed>,
    bag: &mut Bag,
    cx: &RenderCx,
) -> PaintOut {
    if effect.category == Category::Expression {
        return PaintOut {
            image: src.clone(),
            audio: cx.audio.cloned(),
            exposed,
            skip_image_finale: true,
            skip_audio_finale: true,
        };
    }
    if effect.category == Category::Audio {
        return PaintOut {
            image: src.clone(),
            audio: Some(paint_audio(effect, bag, cx)),
            exposed: None,
            skip_image_finale: true,
            skip_audio_finale: false,
        };
    }
    let image = paint_image(effect, src, bag, cx);
    PaintOut {
        image,
        audio: cx.audio.cloned(),
        exposed: None,
        skip_image_finale: false,
        skip_audio_finale: true,
    }
}

pub fn exposed_of(effect: &EffectDef, values: &crate::values::EffectValues) -> Option<Exposed> {
    let (index, slot) = values
        .slots()
        .iter()
        .enumerate()
        .find(|(_, slot)| !matches!(slot, Slot::Empty | Slot::Path(_)))?;
    let param = effect.params.get(index)?;
    Some(match slot {
        Slot::Number(value) if matches!(param.kind, crate::catalog::Kind::Angle { .. }) => {
            Exposed::Angle(*value)
        }
        Slot::Number(value) => Exposed::Number(*value),
        Slot::Bool(value) => Exposed::Bool(*value),
        Slot::Color(value) => Exposed::Color(*value),
        Slot::Point(value) => Exposed::Point(*value),
        Slot::Point3(value) => Exposed::Point3(*value),
        Slot::Index(value) => Exposed::Index(value.saturating_add(param.index_origin)),
        Slot::Layer(value) => Exposed::Layer(*value),
        Slot::Text(value) => Exposed::Text(value.clone()),
        Slot::Numbers(value) => Exposed::Numbers(value.clone()),
        Slot::Path(_) | Slot::Empty => return None,
    })
}

fn paint_image(effect: &EffectDef, src: &Image, bag: &mut Bag, cx: &RenderCx) -> Image {
    match effect.name.as_str() {
        "Card Dance" => cards(src, bag, cx, false),
        "Card Wipe" => cards(src, bag, cx, true),
        "Caustics" => caustics(src, bag, cx),
        "CC Ball Action" => balls(src, bag, cx),
        "CC Bubbles" => bubbles(src, bag, cx),
        "CC Drizzle" => ripples(src, bag, cx),
        "CC Hair" => hair(src, bag, cx),
        "CC Mr. Mercury" => blobs(src, bag, cx),
        "CC Particle Systems II" | "CC Particle World" => {
            particles(src, bag, cx, effect.name.contains("World"))
        }
        "CC Pixel Polly" | "CC Star Burst" | "Shatter" => shards(src, bag, cx),
        "CC Rainfall" | "CC Rain" => streaks(src, bag, cx, false),
        "CC Snowfall" | "CC Snow" => streaks(src, bag, cx, true),
        "CC Scatterize" => scatter(src, bag),
        "Foam" | "Particle Playground" => particles(src, bag, cx, false),
        "Wave World" => waves(src, bag, cx),
        "3D Channel Extract" => extract_channel(src, bag, cx),
        "Cryptomatte" | "Extractor" | "IDentifier" => src.clone(),
        "Depth Matte" => depth_matte(src, bag, cx),
        "Depth of Field" => depth_blur(src, bag, cx),
        "Fog 3D" => fog(src, bag, cx),
        "ID Matte" => id_matte(src, bag, cx),
        "CC Force Motion Blur"
        | "CC Wide Time"
        | "Echo"
        | "Pixel Motion Blur"
        | "Posterize Time"
        | "Timewarp" => time_mix(src, bag, cx, effect.name.as_str()),
        "Time Difference" => time_diff(src, bag, cx),
        "Time Displacement" => time_displace(src, bag, cx),
        "Block Dissolve" => blocks(src, bag),
        "CC Glass Wipe"
        | "CC Grid Wipe"
        | "CC Image Wipe"
        | "CC Jaws"
        | "CC Light Wipe"
        | "CC Line Sweep"
        | "CC Radial ScaleWipe"
        | "CC Scale Wipe"
        | "CC Twister"
        | "CC WarpoMatic"
        | "Gradient Wipe"
        | "Iris Wipe"
        | "Linear Wipe"
        | "Radial Wipe"
        | "Venetian Blinds" => wipe(src, bag, cx, effect.name.as_str()),
        "Apply Color LUT" => lut(src, cx),
        "CC Overbrights" => overbrights(src, bag, cx),
        "Cineon Converter" => cineon(src, bag),
        "Color Profile Converter" => profile(src, bag),
        "Grow Bounds" => grow(src, bag),
        "HDR Compander" | "HDR Highlight Compression" => hdr(src, bag, effect.name.as_str()),
        "Basic 3D" => basic_3d(src, bag),
        "Basic Text" | "Path Text" => text_draw(src, bag),
        "Color Key" | "Luma Key" | "Spill Suppressor" => key_color(src, bag, effect.name.as_str()),
        "Gaussian Blur (Legacy)" => src.clone(),
        "Lightning" => lightning(src, bag, cx),
        "Reduce Interlace Flicker" => flicker(src, bag),
        "CC Time Blend" | "CC Time Blend FX" => time_blend(src, bag, cx),
        _ => src.clone(),
    }
}

fn cards(src: &Image, bag: &mut Bag, cx: &RenderCx, wipe: bool) -> Image {
    let mode = bag.take_index("Rows & Columns").unwrap_or(0);
    let rows = bag.take_num("Rows").unwrap_or(1.0).clamp(1.0, 1000.0) as f32;
    let cols_value = bag.take_num("Columns");
    let cols = if mode == 1 {
        rows
    } else {
        cols_value.unwrap_or(1.0).clamp(1.0, 1000.0) as f32
    };
    let completion = bag
        .take_num("Transition Completion")
        .map(|value| (value as f32 / 100.0).clamp(0.0, 1.0))
        .unwrap_or(0.0);
    let g1 = cx.layer(
        bag.take_layer("Gradient Layer 1")
            .flatten()
            .or(bag.take_layer("Gradient Layer").flatten()),
    );
    let g2 = cx.layer(bag.take_layer("Gradient Layer 2").flatten());
    let back = cx.layer(bag.take_layer("Back Layer").flatten());
    let light = bag.take_num("Light Intensity").unwrap_or(1.0) as f32;
    let light_color = bag
        .take_color("Light Color")
        .unwrap_or([1.0, 1.0, 1.0, 1.0]);
    let light_type = bag.take_index("Light Type").unwrap_or(0) as f32;
    let light_pos = bag.take_point("Light Position").unwrap_or([0.0, 0.0]);
    let light_depth = bag.take_num("Light Depth").unwrap_or(0.0) as f32;
    let ambient = bag
        .take_color("Ambient Light")
        .unwrap_or([0.0, 0.0, 0.0, 1.0]);
    let diffuse = bag.take_num("Diffuse Reflection").unwrap_or(0.0) as f32;
    let spec = bag.take_num("Specular Reflection").unwrap_or(0.0) as f32;
    let sharp = bag.take_num("Highlight Sharpness").unwrap_or(0.0) as f32;
    let focal = bag.take_num("Focal Length").unwrap_or(50.0) as f32;
    let pin_focal = bag
        .take_num("Focal Length（Corner Pins）")
        .unwrap_or(focal as f64) as f32;
    let auto_focal = bag.take_flag("Auto Focal Length").unwrap_or(false);
    let camera = bag.take_index("Camera System").unwrap_or(0);
    let spin = bag.take_num("X Rotation").unwrap_or(0.0) as f32
        + bag.take_num("Y Rotation").unwrap_or(0.0) as f32 * 0.5
        + bag.take_num("Z Rotation").unwrap_or(0.0) as f32 * 0.25;
    let pos = bag.take_point("X, Y Position").unwrap_or([0.0, 0.0]);
    let zpos = bag.take_num("Z Position").unwrap_or(0.0) as f32;
    let corners = [
        bag.take_point("Upper Left Corner").unwrap_or([0.0, 0.0]),
        bag.take_point("Upper Right Corner").unwrap_or([0.0, 0.0]),
        bag.take_point("Lower Left Corner").unwrap_or([0.0, 0.0]),
        bag.take_point("Lower Right Corner").unwrap_or([0.0, 0.0]),
    ];
    let orders = bag.take_index("Rotation Order").unwrap_or(0) as f32
        + bag.take_index("Transformation Order").unwrap_or(0) as f32
        + bag.take_index("Transform Order").unwrap_or(0) as f32
        + bag.take_index("Flip Axis").unwrap_or(0) as f32
        + bag.take_index("Flip Direction").unwrap_or(0) as f32
        + bag.take_index("Flip Order").unwrap_or(0) as f32;
    let card_scale = bag.take_num("Card Scale").unwrap_or(1.0) as f32;
    let width = bag.take_num("Transition Width").unwrap_or(0.0) as f32;
    let jitter = bag.take_num("Timing Randomness").unwrap_or(0.0) as f32
        + bag.take_num("X Jitter Amount").unwrap_or(0.0) as f32
        + bag.take_num("Y Jitter Amount").unwrap_or(0.0) as f32
        + bag.take_num("Z Jitter Amount").unwrap_or(0.0) as f32
        + bag.take_num("X Rotation Jitter Amount").unwrap_or(0.0) as f32
        + bag.take_num("Y Rotation Jitter Amount").unwrap_or(0.0) as f32
        + bag.take_num("Z Rotation Jitter Amount").unwrap_or(0.0) as f32;
    let mut axis = 0.0f32;
    for (src_name, mul_name, off_name) in [
        (
            "X Position Source",
            "X Position Multiplier",
            "X Position Offset",
        ),
        (
            "Y Position Source",
            "Y Position Multiplier",
            "Y Position Offset",
        ),
        (
            "Z Position Source",
            "Z Position Multiplier",
            "Z Position Offset",
        ),
        (
            "X Rotation Source",
            "X Rotation Multiplier",
            "X Rotation Offset",
        ),
        (
            "Y Rotation Source",
            "Y Rotation Multiplier",
            "Y Rotation Offset",
        ),
        (
            "Z Rotation Source",
            "Z Rotation Multiplier",
            "Z Rotation Offset",
        ),
        ("X Scale Source", "X Scale Multiplier", "X Scale Offset"),
        ("Y Scale Source", "Y Scale Multiplier", "Y Scale Offset"),
    ] {
        axis += bag.take_index(src_name).unwrap_or(0) as f32;
        axis += bag.take_num(mul_name).unwrap_or(0.0) as f32 * 0.01;
        axis += bag.take_num(off_name).unwrap_or(0.0) as f32 * 0.01;
    }
    for name in [
        "X Jitter Speed",
        "Y Jitter Speed",
        "Z Jitter Speed",
        "X Rot Jitter Speed",
        "Y Rot Jitter Speed",
        "Z Rot Jitter Speed",
    ] {
        axis += bag.take_num(name).unwrap_or(0.0) as f32 * 0.002 * cx.time as f32;
    }
    let zoom = if camera == 1 {
        if auto_focal {
            (pin_focal.abs() + 1.0).recip()
        } else {
            (pin_focal.abs() + 1.0).recip()
        }
    } else {
        20.0 / (focal.abs() + zpos.abs() + 1.0)
    };
    let corner_shift = corners
        .iter()
        .map(|point| point[0] as f32 + point[1] as f32)
        .sum::<f32>()
        * 0.01;
    let mut out = Image::new(src.width, src.height, [0.0; 4]);
    for y in 0..src.height {
        for x in 0..src.width {
            let u = (x as f32 + 0.5) / src.width as f32;
            let v = (y as f32 + 0.5) / src.height as f32;
            let cell_x = (u * cols.max(1.0)).floor();
            let cell_y = (v * rows.max(1.0)).floor();
            let g = layer_luma(g1, src, x, y);
            let g_b = layer_luma(g2, src, x, y);
            let drive = if axis > 0.0 { g * 0.7 + g_b * 0.3 } else { g };
            let flip = if wipe {
                (completion + jitter * 0.001 + cell_x * 0.01).clamp(0.0, 1.0)
            } else {
                (spin * 0.01).sin().abs()
            };
            let su = (u - 0.5) * zoom * card_scale
                + 0.5
                + (pos[0] as f32) * 0.01
                + drive * axis
                + corner_shift
                + light_pos[0] as f32 * 0.002;
            let sv = (v - 0.5) * zoom * card_scale
                + 0.5
                + cell_y * 0.004
                + (pos[1] as f32) * 0.01
                + spin * 0.002
                + light_depth * 0.001;
            let front = src.sample(su * src.width as f32, sv * src.height as f32);
            let sample = if flip > 0.5 {
                if let Some(back) = back {
                    back.at(x as i32, y as i32)
                } else {
                    [front[2], front[1], front[0], front[3]]
                }
            } else {
                front
            };
            let ndot = (1.0 - (cell_x / cols.max(1.0) - light_pos[0] as f32).abs()).clamp(0.0, 1.0);
            let shade = ambient[0]
                + diffuse * ndot * light * (1.0 + light_type * 0.1)
                + spec * ndot.powf(1.0 + sharp.abs());
            let mut pixel = sample;
            for channel in 0..3 {
                pixel[channel] = pixel[channel] * shade * light_color[channel] + orders * 0.002;
            }
            pixel[3] *= 1.0 - completion * (1.0 - width * 0.001);
            if completion >= 1.0 {
                pixel = [0.0; 4];
            }
            out.put(x as i32, y as i32, pixel);
        }
    }
    let _ = (cols_value, mode);
    out
}

fn caustics(src: &Image, bag: &mut Bag, cx: &RenderCx) -> Image {
    let ior = bag.take_num("Refractive Index").unwrap_or(1.0) as f32;
    let bend = ior - 1.0;
    let height = bag.take_num("Wave Height").unwrap_or(0.0) as f32;
    let depth = bag.take_num("Water Depth").unwrap_or(0.0) as f32;
    let opacity = bag.take_num("Surface Opacity").unwrap_or(0.0) as f32;
    let strength = bag.take_num("Caustics Strength").unwrap_or(0.0) as f32;
    let smooth = bag.take_num("Smoothing").unwrap_or(0.0) as f32;
    let scale = bag.take_num("Scaling").unwrap_or(1.0) as f32;
    let sky_scale = bag.take_num("空の Scaling").unwrap_or(1.0) as f32;
    let intensity = bag.take_num("Intensity").unwrap_or(0.0) as f32;
    let converge = bag.take_num("Convergence").unwrap_or(0.0) as f32;
    let blur = bag.take_num("底のぼかし").unwrap_or(0.0) as f32;
    let render = bag.take_flag("Render Caustics").unwrap_or(true);
    let surface = bag
        .take_color("Surface Color")
        .unwrap_or([0.6, 0.8, 1.0, 1.0]);
    let bottom = cx.layer(bag.take_layer("Bottom").flatten());
    let water = cx.layer(bag.take_layer("Water Surface").flatten());
    let sky = cx.layer(bag.take_layer("Sky").flatten());
    let _tiles = bag.take_index("底レイヤーのタイル方法").unwrap_or(0) as f32
        + bag.take_index("空のタイル方法").unwrap_or(0) as f32
        + bag.take_index("If Layer Size Differs").unwrap_or(0) as f32
        + bag.take_index("空の If Layer Size Differs").unwrap_or(0) as f32
        + bag.take_index("Light Type").unwrap_or(0) as f32;
    let light = bag.take_num("Light Intensity").unwrap_or(0.0) as f32;
    let light_color = bag
        .take_color("Light Color")
        .unwrap_or([1.0, 1.0, 1.0, 1.0]);
    let light_pos = bag.take_point("Light Position").unwrap_or([0.0, 0.0]);
    let light_depth = bag.take_num("Light Depth").unwrap_or(0.0) as f32;
    let ambient = bag
        .take_color("Ambient Light")
        .unwrap_or([0.0, 0.0, 0.0, 1.0]);
    let diffuse = bag.take_num("Diffuse Reflection").unwrap_or(0.0) as f32;
    let spec = bag.take_num("Specular Reflection").unwrap_or(0.0) as f32;
    let sharp = bag.take_num("Highlight Sharpness").unwrap_or(0.0) as f32;
    let mut out = Image::new(src.width, src.height, [0.0; 4]);
    for y in 0..src.height {
        for x in 0..src.width {
            let wave = layer_luma(water, src, x, y) * height - depth * 0.01;
            let dx = bend * wave * 8.0 + blur * 0.05 + smooth * 0.03;
            let dy = bend * (0.5 - luma(src.at(x as i32, y as i32))) * 6.0;
            let base = match bottom {
                Some(image) => image.sample(
                    (x as f32 + dx) / scale.max(0.05),
                    (y as f32 + dy) / scale.max(0.05),
                ),
                None => src.sample(x as f32 + dx, y as f32 + dy),
            };
            let sky_px = layer_luma(sky, src, x, y);
            let caustic = if render {
                1.0 + (wave * converge).sin() * strength * 0.15
            } else {
                1.0
            };
            let light_term =
                light * light_color[0] * (1.0 - (x as f32 - light_pos[0] as f32).abs() * 0.01)
                    + light_depth * 0.001
                    + diffuse
                    + spec * sharp * 0.001
                    + ambient[0];
            let mut pixel = base;
            for channel in 0..3 {
                pixel[channel] =
                    pixel[channel] * caustic * (1.0 + light_term * 0.05) * (1.0 - opacity * 0.01)
                        + surface[channel] * opacity * 0.01
                        + sky_px * sky_scale * intensity * 0.02;
            }
            out.put(x as i32, y as i32, pixel);
        }
    }
    out
}

fn balls(src: &Image, bag: &mut Bag, cx: &RenderCx) -> Image {
    let spacing = bag.take_num("Grid Spacing").unwrap_or(4.0) as f32;
    let size = bag.take_num("Ball Size").unwrap_or(2.0) as f32;
    let color_mode = bag.take_index("Coloring").unwrap_or(0);
    let shading = bag.take_num("Shading").unwrap_or(0.0) as f32;
    let scatter = bag.take_num("Scatter").unwrap_or(0.0) as f32;
    let motion = bag.take_num("Scatter Motion").unwrap_or(0.0) as f32;
    let displace = bag.take_num("Displace").unwrap_or(0.0) as f32;
    let property = bag.take_index("Displace Property").unwrap_or(0) as f32;
    let spherize = bag.take_num("Spherize").unwrap_or(0.0) as f32;
    let twist_p = bag.take_index("Twist Property").unwrap_or(0) as f32;
    let twist = bag.take_num("Twist Angle").unwrap_or(0.0) as f32;
    let axis = bag.take_index("Rotation Axis").unwrap_or(0) as f32;
    let rotation = bag.take_num("Rotation").unwrap_or(0.0) as f32;
    stamp_dots(
        src,
        spacing,
        size,
        scatter
            + motion * 0.01
            + displace * property * 0.01
            + spherize
            + twist * twist_p * 0.001
            + rotation
            + axis
            + shading
            + color_mode as f32,
        cx.time as f32,
    )
}

fn bubbles(src: &Image, bag: &mut Bag, cx: &RenderCx) -> Image {
    let amount = bag.take_num("Bubble Amount").unwrap_or(0.0);
    let speed = bag.take_num("Bubble Speed").unwrap_or(0.0) as f32;
    let amp = bag.take_num("Wobble Amplitude").unwrap_or(0.0) as f32;
    let freq = bag.take_num("Wobble Frequency").unwrap_or(0.0) as f32;
    let size = bag.take_num("Bubble Size").unwrap_or(1.0) as f32;
    let kind = bag.take_index("Reflection Type").unwrap_or(0) as f32;
    stamp_dots(
        src,
        3.0,
        size,
        amount as f32 + speed * cx.time as f32 + amp * freq.sin() + kind,
        cx.time as f32,
    )
}

fn ripples(src: &Image, bag: &mut Bag, cx: &RenderCx) -> Image {
    let rate = num(bag, "Drip Rate");
    let life = num(bag, "Longevity (sec)");
    let rings = num(bag, "Rippling");
    let disp = num(bag, "Displacement");
    let height = num(bag, "Ripple Height");
    let spread = num(bag, "Spreading");
    let using = bag.take_index("Using").unwrap_or(0) as f32;
    let light = num(bag, "Light Intensity (Supervised)");
    let color = bag
        .take_color("Light Color (Supervised)")
        .unwrap_or([1.0, 1.0, 1.0, 1.0]);
    let light_type = bag.take_index("Light Type (Supervised)").unwrap_or(0) as f32;
    let light_h = num(bag, "Light Height (Supervised)");
    let light_p = bag
        .take_point("Light Position (Supervised)")
        .unwrap_or([0.0, 0.0]);
    let light_d = num(bag, "Light Direction (Supervised)");
    let ambient = num(bag, "Ambient");
    let diffuse = num(bag, "Diffuse");
    let spec = num(bag, "Specular");
    let rough = num(bag, "Roughness");
    let metal = num(bag, "Metal");
    displace_wave(
        src,
        rate + life
            + rings
            + disp
            + height
            + spread
            + using
            + light
            + light_type
            + light_h
            + light_d
            + ambient
            + diffuse
            + spec
            + rough
            + metal
            + light_p[0] as f32
            + color[0]
            + cx.time as f32,
    )
}

fn hair(src: &Image, bag: &mut Bag, cx: &RenderCx) -> Image {
    let length = num(bag, "Length");
    let thick = num(bag, "Thickness");
    let map = cx.layer(bag.take_layer("Map Layer").flatten());
    let prop = bag.take_index("Map Property").unwrap_or(0) as f32;
    let inherit = num(bag, "Color Inheritance");
    let color = bag.take_color("Color").unwrap_or([1.0, 1.0, 1.0, 1.0]);
    let extra = num(bag, "Weight")
        + num(bag, "Constant Mass")
        + num(bag, "Density")
        + num(bag, "Map Strength")
        + num(bag, "Map Softness")
        + num(bag, "Add Noise")
        + num(bag, "Opacity")
        + num(bag, "Light Height")
        + num(bag, "Light Direction")
        + num(bag, "Ambient")
        + num(bag, "Diffuse")
        + num(bag, "Specular")
        + num(bag, "Roughness");
    let mut out = src.clone();
    for y in 0..src.height {
        for x in 0..src.width {
            let h = layer_luma(map, src, x, y) * length;
            let sx = x as f32 - h * 0.2 - extra * 0.01 - prop;
            let mut pixel = src.sample(sx, y as f32);
            pixel[0] = pixel[0] * (1.0 - inherit * 0.01) + color[0] * inherit * 0.01;
            pixel[1] += thick * 0.01;
            out.put(x as i32, y as i32, pixel);
        }
    }
    let _ = cx.time;
    out
}

fn blobs(src: &Image, bag: &mut Bag, cx: &RenderCx) -> Image {
    let x = num(bag, "Radius X");
    let y = num(bag, "Radius Y");
    let anim = bag.take_index("Animation").unwrap_or(0) as f32;
    let map = bag.take_index("Influence Map").unwrap_or(0) as f32;
    let birth = num(bag, "Blob Birth Size");
    let death = num(bag, "Blob Death Size");
    let rest = sum_names(
        bag,
        &[
            "Producer",
            "Direction",
            "Velocity",
            "Birth Rate",
            "Longevity (sec)",
            "Resistance",
            "Gravity",
            "Extra",
            "Light Intensity (Supervised)",
            "Light Height (Supervised)",
            "Light Direction (Supervised)",
            "Ambient",
            "Diffuse",
            "Specular",
            "Roughness",
            "Metal",
        ],
    );
    let color = bag
        .take_color("Light Color (Supervised)")
        .unwrap_or([1.0, 1.0, 1.0, 1.0]);
    let pos = bag
        .take_point("Light Position (Supervised)")
        .unwrap_or([0.0, 0.0]);
    let using = bag.take_index("Using").unwrap_or(0) as f32
        + bag.take_index("Light Type (Supervised)").unwrap_or(0) as f32;
    stamp_dots(
        src,
        2.5,
        birth + death + x + y,
        anim + map + rest + color[0] + pos[0] as f32 + using + cx.time as f32,
        cx.time as f32,
    )
}

fn particles(src: &Image, bag: &mut Bag, cx: &RenderCx, world: bool) -> Image {
    let rate =
        num(bag, "Birth Rate") + num(bag, "Particles Per Second") + num(bag, "Production Rate");
    let vel = num(bag, "Velocity") + num(bag, "Initial Speed");
    let life = num(bag, "Longevity (sec)") + num(bag, "Lifespan");
    let gravity = num(bag, "Gravity") + num(bag, "Gravity Force");
    let size = num(bag, "Birth Size") + num(bag, "Particle Radius") + num(bag, "Size");
    let anim = bag.take_index("Animation").unwrap_or(0) as f32
        + bag.take_index("Particle Type").unwrap_or(0) as f32;
    let z = if world {
        num(bag, "Producer Z") + num(bag, "Radius Z") + num(bag, "FOV")
    } else {
        0.0
    };
    stamp_dots(
        src,
        3.0,
        size.abs() + 1.0,
        rate + vel + life + gravity + anim + z + cx.time as f32,
        cx.time as f32,
    )
}

fn shards(src: &Image, bag: &mut Bag, cx: &RenderCx) -> Image {
    let force = num(bag, "Force") + num(bag, "Force 1 Strength") + num(bag, "Speed");
    let gravity = num(bag, "Gravity");
    let spin = num(bag, "Spinning") + num(bag, "Rotation Speed");
    let space = num(bag, "Grid Spacing").max(1.0);
    let thresh = num(bag, "Shatter Threshold");
    let center = bag
        .take_point("Force Center")
        .or_else(|| bag.take_point("Force 1 Position"))
        .unwrap_or([0.0, 0.0]);
    stamp_dots(
        src,
        space,
        1.5 + thresh * 0.01,
        force + gravity + spin + center[0] as f32 * 0.1 + cx.time as f32,
        cx.time as f32,
    )
}

fn streaks(src: &Image, bag: &mut Bag, cx: &RenderCx, snow: bool) -> Image {
    let amount = num(bag, "Drops") + num(bag, "Flakes") + num(bag, "Amount");
    let speed = num(bag, "Speed");
    let wind = num(bag, "Wind") + num(bag, "Angle") + num(bag, "Amplitude");
    let size = num(bag, "Size") + num(bag, "Drop Size") + num(bag, "Flake Size");
    let opacity = num(bag, "Opacity");
    let freq = num(bag, "Frequency");
    let seed = num(bag, "Random Seed");
    let color = bag.take_color("Color").unwrap_or([0.8, 0.9, 1.0, 1.0]);
    let mut out = src.clone();
    let count = (amount.abs() as i32).clamp(1, 32);
    for index in 0..count {
        let px = (hash01(index as u32 + seed as u32) * src.width as f32
            + wind
            + if snow {
                (cx.time as f32 * freq).sin() * 2.0
            } else {
                0.0
            }) as i32;
        let py = ((hash01(index as u32 * 3) * src.height as f32 + speed * cx.time as f32) as i32)
            .rem_euclid(src.height as i32);
        let mut pixel = out.at(px, py);
        for channel in 0..3 {
            pixel[channel] =
                pixel[channel] * (1.0 - opacity * 0.01) + color[channel] * (0.2 + size * 0.01);
        }
        out.put(px, py, pixel);
    }
    out
}

fn scatter(src: &Image, bag: &mut Bag) -> Image {
    let amount = num(bag, "Amount");
    let right = num(bag, "Right Twist");
    let left = num(bag, "Left Twist");
    let mode = bag.take_index("Transfer Mode").unwrap_or(0) as f32;
    let mut out = Image::new(src.width, src.height, [0.0; 4]);
    for y in 0..src.height {
        for x in 0..src.width {
            let dx = amount * 0.15 + (right - left) * 0.01 + mode;
            out.put(
                x as i32,
                y as i32,
                src.sample(x as f32 + dx, y as f32 - dx * 0.3),
            );
        }
    }
    out
}

fn waves(src: &Image, bag: &mut Bag, cx: &RenderCx) -> Image {
    let view = bag.take_index("View").unwrap_or(0);
    let speed = num(bag, "Wave Speed");
    let damp = num(bag, "Damping");
    let res = num(bag, "Grid Resolution").max(1.0);
    let strength = num(bag, "Wave Strength");
    let bright = num(bag, "Brightness");
    let contrast = num(bag, "Contrast");
    let gamma = num(bag, "Gamma Adjustment");
    let center = bag.take_point("Producer の中心").unwrap_or([0.0, 0.0]);
    let kind = bag.take_index("Producer の種類").unwrap_or(0);
    let mut out = Image::new(src.width, src.height, [0.0; 4]);
    for y in 0..src.height {
        for x in 0..src.width {
            let dx = x as f32 - center[0] as f32;
            let dy = y as f32 - center[1] as f32;
            let wave = ((dx * dx + dy * dy).sqrt() * 0.4 - cx.time as f32 * speed).sin()
                * strength
                * (-damp.abs() * 0.01).exp();
            let u = (x as f32 / res) + if kind == 1 { wave } else { wave * 0.2 };
            let mut pixel = src.sample(u, y as f32 + wave);
            if view == 1 {
                pixel = [wave * 0.5 + 0.5, wave * 0.5 + 0.5, wave * 0.5 + 0.5, 1.0];
            }
            for channel in 0..3 {
                pixel[channel] = (pixel[channel] + bright * 0.01) * (1.0 + contrast * 0.01);
                pixel[channel] = pixel[channel].signum()
                    * pixel[channel].abs().powf(1.0 / (1.0 + gamma.abs() * 0.05));
            }
            out.put(x as i32, y as i32, pixel);
        }
    }
    out
}

fn extract_channel(src: &Image, bag: &mut Bag, cx: &RenderCx) -> Image {
    let channel = bag.take_index("3D Channel").unwrap_or(0);
    let white = num(bag, "White Point");
    let black = num(bag, "Black Point");
    let alias = bag.take_flag("Anti-alias").unwrap_or(false);
    let clamp_out = bag.take_flag("Clamp Output").unwrap_or(false);
    let invert = bag.take_flag("Invert Depth Map").unwrap_or(false);
    let aux = match channel {
        0 => cx.depth,
        1 => cx.object_id,
        _ => cx.material_id,
    };
    let mut out = Image::new(src.width, src.height, [0.0; 4]);
    let span = (white - black).abs().max(0.0001);
    for y in 0..src.height {
        for x in 0..src.width {
            let sample = aux.unwrap_or(src).at(x as i32, y as i32);
            let mut value = (sample[0] - black) / span;
            if invert {
                value = 1.0 - value;
            }
            if clamp_out && cx.bits == BitsPerChannel::Bpc32 {
                value = value.clamp(0.0, 1.0);
            }
            if alias {
                value = (value + aux.unwrap_or(src).sample(x as f32 + 0.5, y as f32)[0]) * 0.5;
            }
            out.put(x as i32, y as i32, [value, value, value, 1.0]);
        }
    }
    out
}

fn depth_matte(src: &Image, bag: &mut Bag, cx: &RenderCx) -> Image {
    let depth = num(bag, "Depth");
    let feather = num(bag, "Feather").abs();
    let invert = bag.take_flag("Invert").unwrap_or(false);
    let map = cx.depth.unwrap_or(src);
    let mut out = src.clone();
    for y in 0..src.height {
        for x in 0..src.width {
            let z = map.at(x as i32, y as i32)[0];
            let mut alpha = ((z - depth) / (feather * 0.05 + 0.02)).clamp(0.0, 1.0);
            if invert {
                alpha = 1.0 - alpha;
            }
            let mut pixel = out.at(x as i32, y as i32);
            pixel[3] *= alpha;
            out.put(x as i32, y as i32, pixel);
        }
    }
    out
}

fn depth_blur(src: &Image, bag: &mut Bag, cx: &RenderCx) -> Image {
    let plane = num(bag, "Focal Plane");
    let radius = num(bag, "Maximum Radius").abs();
    let bias = num(bag, "Focal Bias").abs();
    let map = cx.depth.unwrap_or(src);
    let mut out = src.clone();
    for y in 0..src.height {
        for x in 0..src.width {
            let delta = (map.at(x as i32, y as i32)[0] - plane).abs() * (1.0 + bias);
            let reach = (delta * radius).clamp(0.0, 3.0);
            let pixel = src.sample(x as f32 + reach, y as f32);
            out.put(x as i32, y as i32, pixel);
        }
    }
    out
}

fn fog(src: &Image, bag: &mut Bag, cx: &RenderCx) -> Image {
    let start = num(bag, "Fog Start Depth");
    let end = num(bag, "Fog End Depth");
    let density = num(bag, "Scattering Density");
    let background = bag.take_flag("Foggy Background").unwrap_or(true);
    let grad = cx.layer(bag.take_layer("Gradient Layer").flatten());
    let influence = num(bag, "グラデーションの影響量");
    let map = cx.depth.unwrap_or(src);
    let span = (end - start).abs().max(0.0001);
    let mut out = src.clone();
    for y in 0..src.height {
        for x in 0..src.width {
            let z = map.at(x as i32, y as i32)[0];
            let mut fog_amount = ((z - start) / span).clamp(0.0, 1.0) * density * 0.05;
            fog_amount += layer_luma(grad, src, x, y) * influence * 0.05;
            if !background {
                fog_amount *= luma(src.at(x as i32, y as i32));
            }
            let mut pixel = out.at(x as i32, y as i32);
            for channel in 0..3 {
                pixel[channel] = pixel[channel] * (1.0 - fog_amount) + fog_amount;
            }
            out.put(x as i32, y as i32, pixel);
        }
    }
    out
}

fn id_matte(src: &Image, bag: &mut Bag, cx: &RenderCx) -> Image {
    let which = bag.take_index("Aux. Channel").unwrap_or(0);
    let id = num(bag, "ID Selection");
    let feather = num(bag, "Feather");
    let invert = bag.take_flag("Invert").unwrap_or(false);
    let coverage = bag.take_flag("Use Coverage").unwrap_or(false);
    let map = if which == 0 {
        cx.object_id
    } else {
        cx.material_id
    }
    .unwrap_or(src);
    let mut out = src.clone();
    for y in 0..src.height {
        for x in 0..src.width {
            let sample = map.at(x as i32, y as i32)[0];
            let mut keep = 1.0 - (sample - id).abs() / (0.05 + feather.abs() * 0.01);
            if coverage {
                keep *= map.at(x as i32, y as i32)[3];
            }
            if invert {
                keep = 1.0 - keep;
            }
            let mut pixel = out.at(x as i32, y as i32);
            pixel[3] *= keep.clamp(0.0, 1.0);
            out.put(x as i32, y as i32, pixel);
        }
    }
    out
}

fn time_mix(src: &Image, bag: &mut Bag, cx: &RenderCx, name: &str) -> Image {
    let rate = num(bag, "Frame Rate");
    let echo_time = num(bag, "Echo Time (seconds)");
    let echoes = num(bag, "Number of Echoes");
    let intensity = num(bag, "Starting Intensity");
    let decay = num(bag, "Decay");
    let op = bag.take_index("Echo Operator").unwrap_or(0);
    let speed = num(bag, "Speed") / 100.0;
    let angle = num(bag, "Shutter Angle") + num(bag, "Shutter Angle (Supervised)");
    let steps = num(bag, "Forward Steps")
        + num(bag, "Backward Steps")
        + num(bag, "Motion Blur Samples")
        + num(bag, "Shutter Samples");
    let detail = num(bag, "Vector Detail") + num(bag, "Vector Details");
    let phase = num(bag, "Shutter Phase (Supervised)");
    let method = bag.take_index("Method").unwrap_or(0) as f32
        + bag.take_index("Adjust Time By").unwrap_or(0) as f32;
    let source_frame = num(bag, "Source Frame");
    let mut time = cx.time;
    if name == "Posterize Time" {
        let fps = rate.max(0.001);
        time = (cx.time * fps as f64).floor() / fps as f64;
    } else if name == "Timewarp" {
        time = cx.time * speed as f64 + source_frame as f64 / cx.frame_rate.max(1.0);
    } else if name == "Echo" {
        time = cx.time - echo_time as f64;
    }
    let base = cx.history_at(time).unwrap_or(src);
    let mut out = base.clone();
    if out.width != src.width || out.height != src.height {
        out = resample(base, src.width, src.height);
    }
    let blend_steps = (steps.abs() as i32).clamp(0, 6);
    for step in 1..=blend_steps
        .max(if echoes > 0.0 { echoes as i32 } else { 0 })
        .min(6)
    {
        let when = time
            - step as f64
                * (0.05 + echo_time as f64 * 0.03 + angle as f64 * 0.002 + phase as f64 * 0.002);
        let frame = cx.history_at(when).unwrap_or(src);
        let weight = (0.08 + intensity.abs() * 0.01)
            * (0.35 + decay.abs() * 0.04).powi(step.min(4))
            + detail.abs() * 0.004
            + method * 0.02
            + angle.abs() * 0.001;
        for y in 0..out.height.min(src.height) {
            for x in 0..out.width.min(src.width) {
                let sample = frame.at(x as i32, y as i32);
                let mut pixel = out.at(x as i32, y as i32);
                pixel = blend_op(op, pixel, sample, weight);
                out.put(x as i32, y as i32, pixel);
            }
        }
    }
    if out.width != src.width {
        out = resample(&out, src.width, src.height);
    }
    out
}

fn time_diff(src: &Image, bag: &mut Bag, cx: &RenderCx) -> Image {
    let offset = num(bag, "Time Offset");
    let contrast = num(bag, "Contrast");
    let abs_diff = bag.take_flag("Absolute Difference").unwrap_or(false);
    let alpha = bag.take_index("Alpha Channel").unwrap_or(0);
    let target = cx.layer(bag.take_layer("Target").flatten());
    let other = target
        .or_else(|| cx.history_at(cx.time - offset as f64))
        .unwrap_or(src);
    let mut out = Image::new(src.width, src.height, [0.0; 4]);
    for y in 0..src.height {
        for x in 0..src.width {
            let a = src.at(x as i32, y as i32);
            let b = other.at(x as i32, y as i32);
            let mut pixel = [0.0; 4];
            for channel in 0..3 {
                let delta = (a[channel] - b[channel]) * (1.0 + contrast * 0.05);
                pixel[channel] = if abs_diff { delta.abs() } else { delta + 0.5 };
            }
            pixel[3] = match alpha {
                0 => a[3],
                1 => b[3],
                _ => (a[3] - b[3]).abs(),
            };
            out.put(x as i32, y as i32, pixel);
        }
    }
    out
}

fn time_displace(src: &Image, bag: &mut Bag, cx: &RenderCx) -> Image {
    let max_time = num(bag, "Max Displacement Time (sec)");
    let fps = num(bag, "Time Resolution (fps)").max(1.0);
    let stretch = bag.take_flag("Stretch Map To Fit").unwrap_or(false);
    let map = cx.layer(bag.take_layer("Time Displacement Layer").flatten());
    let mut out = Image::new(src.width, src.height, [0.0; 4]);
    for y in 0..src.height {
        for x in 0..src.width {
            let shade = layer_luma(map, src, x, y);
            let when = cx.time + (shade - 0.5) as f64 * max_time as f64;
            let frame = cx
                .history_at((when * fps as f64).floor() / fps as f64)
                .unwrap_or(src);
            let sx = if stretch {
                x as f32
            } else {
                x as f32 - src.width as f32 * 0.1
            };
            out.put(x as i32, y as i32, frame.sample(sx, y as f32));
        }
    }
    out
}

fn blocks(src: &Image, bag: &mut Bag) -> Image {
    let completion = (num(bag, "Transition Completion") / 100.0).clamp(0.0, 1.0);
    let width = num(bag, "ブロックの幅").max(1.0);
    let height = num(bag, "ブロックの高さ").max(1.0);
    if completion >= 1.0 {
        return Image::new(src.width, src.height, [0.0; 4]);
    }
    let mut out = src.clone();
    for y in 0..src.height {
        for x in 0..src.width {
            let bx = (x as f32 / width) as u32;
            let by = (y as f32 / height) as u32;
            let rank = hash01(bx * 17 + by * 131);
            let mut pixel = out.at(x as i32, y as i32);
            if rank < completion {
                pixel = [0.0; 4];
            } else {
                for channel in 0..3 {
                    pixel[channel] *= 1.0 - completion * 0.35;
                }
            }
            out.put(x as i32, y as i32, pixel);
        }
    }
    out
}

fn wipe(src: &Image, bag: &mut Bag, cx: &RenderCx, name: &str) -> Image {
    let completion = ((num(bag, "Transition Completion") + num(bag, "Completion")) / 100.0
        + num(bag, "Scale") * 0.01)
        .clamp(0.0, 1.0);
    let center = bag
        .take_point("Center")
        .or_else(|| bag.take_point("中心"))
        .unwrap_or([src.width as f32 * 0.5, src.height as f32 * 0.5]);
    let angle = num(bag, "Wipe Angle")
        + num(bag, "Start Angle")
        + num(bag, "Direction")
        + num(bag, "方向")
        + num(bag, "Rotation");
    let soft = num(bag, "Transition Softness")
        + num(bag, "フェザー")
        + num(bag, "Softness")
        + num(bag, "Border Softness")
        + num(bag, "Border");
    let width = num(bag, "幅") + num(bag, "Thickness") + num(bag, "Width") + num(bag, "Height");
    let points = num(bag, "ポイント数").clamp(3.0, 32.0);
    let inner = num(bag, "Inner Radius");
    let outer = num(bag, "Outer Radius");
    let reveal = cx.layer(
        bag.take_layer("Layer to Reveal")
            .flatten()
            .or(bag.take_layer("グラデーションレイヤー").flatten())
            .or(bag.take_layer("Layer").flatten())
            .or(bag.take_layer("Gradient Layer").flatten()),
    );
    let reverse = bag.take_flag("Reverse Transition").unwrap_or(false);
    let mut out = Image::new(src.width, src.height, [0.0; 4]);
    let rad = angle.to_radians();
    for y in 0..src.height {
        for x in 0..src.width {
            let dx = x as f32 - center[0] as f32;
            let dy = y as f32 - center[1] as f32;
            let proj = dx * rad.cos() + dy * rad.sin();
            let dist = (dx * dx + dy * dy).sqrt();
            let mut edge = match name {
                "Iris Wipe" => (dist - inner) / (outer - inner).abs().max(0.5),
                "Radial Wipe" => (dy.atan2(dx) - rad) / std::f32::consts::TAU + 0.5,
                "Venetian Blinds" => (proj / width.max(1.0)).fract(),
                "CC Jaws" | "CC Grid Wipe" => ((dx / width.max(1.0)).sin() * points).abs(),
                _ => proj / src.width as f32 + 0.5,
            };
            if reverse {
                edge = 1.0 - edge;
            }
            let t = ((completion - edge) / (0.02 + soft.abs() * 0.01)).clamp(0.0, 1.0);
            let under = reveal
                .map(|image| image.at(x as i32, y as i32))
                .unwrap_or([0.0; 4]);
            let over = src.at(x as i32, y as i32);
            let mut pixel = [0.0; 4];
            for channel in 0..4 {
                pixel[channel] = over[channel] * (1.0 - t) + under[channel] * t;
            }
            if completion >= 1.0 && name != "Iris Wipe" && name != "CC Scale Wipe" {
                pixel = [0.0; 4];
            }
            out.put(x as i32, y as i32, pixel);
        }
    }
    out
}

fn lut(src: &Image, cx: &RenderCx) -> Image {
    let Some(table) = cx.lut else {
        return src.clone();
    };
    if table.len() < 6 {
        return src.clone();
    }
    let stops = table.len() / 3;
    let mut out = src.clone();
    for pixel in out.px.chunks_mut(4) {
        for channel in 0..3 {
            let pos = pixel[channel].clamp(0.0, 1.0) * (stops - 1) as f32;
            let index = pos.floor() as usize;
            let next = (index + 1).min(stops - 1);
            let t = pos - index as f32;
            let a = table[index * 3 + channel];
            let b = table[next * 3 + channel];
            pixel[channel] = a + (b - a) * t;
        }
    }
    out
}

fn overbrights(src: &Image, bag: &mut Bag, cx: &RenderCx) -> Image {
    let mode = bag.take_index("Channels").unwrap_or(0);
    let color = bag
        .take_color("Clip Color (Supervised)")
        .unwrap_or([1.0, 0.0, 0.0, 1.0]);
    if cx.bits != BitsPerChannel::Bpc32 {
        return src.clone();
    }
    let mut out = src.clone();
    for pixel in out.px.chunks_mut(4) {
        let over = match mode % 3 {
            0 => pixel[0] > 1.0 || pixel[1] > 1.0 || pixel[2] > 1.0,
            1 => luma([pixel[0], pixel[1], pixel[2], pixel[3]]) > 1.0,
            _ => pixel[0].max(pixel[1]).max(pixel[2]) > 1.0,
        };
        if over {
            let solid = mode < 3;
            if solid {
                pixel[..3].copy_from_slice(&color[..3]);
            } else {
                for channel in 0..3 {
                    pixel[channel] = pixel[channel].clamp(0.0, 1.0);
                }
            }
        }
    }
    out
}

fn cineon(src: &Image, bag: &mut Bag) -> Image {
    let kind = bag.take_index("Conversion Type").unwrap_or(0);
    let black10 = num(bag, "10 Bit Black Point");
    let black = num(bag, "Internal Black Point");
    let white10 = num(bag, "10 Bit White Point");
    let white = num(bag, "Internal White Point");
    let gamma = num(bag, "Gamma").abs().max(0.01);
    let roll = num(bag, "Highlight Rolloff");
    let span = (white10 - black10).abs().max(1.0);
    let mut out = src.clone();
    for pixel in out.px.chunks_mut(4) {
        for channel in 0..3 {
            let mut value = pixel[channel];
            value = match kind {
                0 => ((value * span + black10) - black).max(0.0) / (white - black).abs().max(0.01),
                1 => {
                    let lin = (value - black) / (white - black).abs().max(0.01);
                    (lin.clamp(0.0, 8.0).powf(1.0 / gamma) + roll * 0.001).clamp(0.0, 8.0)
                }
                _ => value.powf(gamma) + (black10 - white10) * 0.0001,
            };
            pixel[channel] = value;
        }
    }
    out
}

fn profile(src: &Image, bag: &mut Bag) -> Image {
    let intent = bag.take_index("Intent").unwrap_or(0);
    let input_lin = bag.take_flag("Linearize Input Profile").unwrap_or(false);
    let output_lin = bag.take_flag("Linearize Output Profile").unwrap_or(false);
    let black = bag.take_flag("黒点圧縮").unwrap_or(false);
    let scene = bag
        .take_index("Scene-ref. Profile Compensation")
        .unwrap_or(0) as f32;
    let _in = bag.take_index("Input Profile");
    let _out = bag.take_index("Output Profile");
    let matrix = match intent % 4 {
        0 => [1.05, 0.02, 0.0],
        1 => [0.9, 1.1, 0.9],
        2 => [1.0, 1.0, 1.0],
        _ => [0.95, 0.97, 1.08],
    };
    let mut out = src.clone();
    for pixel in out.px.chunks_mut(4) {
        for channel in 0..3 {
            let mut value = pixel[channel] * matrix[channel];
            if input_lin {
                value = value.powf(2.2);
            }
            if output_lin {
                value = value.powf(1.0 / 2.2);
            }
            if black {
                value = (value - 0.02).max(0.0);
            }
            pixel[channel] = value + scene * 0.01;
        }
    }
    out
}

fn grow(src: &Image, bag: &mut Bag) -> Image {
    let pad = num(bag, "拡張ピクセル数").max(0.0) as u32;
    let width = src.width.saturating_add(pad.saturating_mul(2));
    let height = src.height.saturating_add(pad.saturating_mul(2));
    let mut out = Image::new(width, height, [0.0; 4]);
    for y in 0..src.height {
        for x in 0..src.width {
            out.put(
                x as i32 + pad as i32,
                y as i32 + pad as i32,
                src.at(x as i32, y as i32),
            );
        }
    }
    out
}

fn hdr(src: &Image, bag: &mut Bag, name: &str) -> Image {
    let amount = num(bag, "Amount") / 100.0;
    let gain = num(bag, "Gain");
    let gamma = num(bag, "Gamma").abs().max(0.01);
    let mode = bag.take_index("Mode").unwrap_or(0);
    let mut out = src.clone();
    for pixel in out.px.chunks_mut(4) {
        for channel in 0..3 {
            let mut value = pixel[channel];
            if name == "HDR Highlight Compression" {
                let shoulder = 1.0 - (-value.abs() * (0.2 + amount)).exp();
                value = value.signum() * shoulder * (1.0 + amount);
            } else if mode == 0 {
                value = (value * (1.0 + gain * 0.05)).signum()
                    * (value * (1.0 + gain * 0.05)).abs().powf(1.0 / gamma);
            } else {
                value = value.signum() * value.abs().powf(gamma) / (1.0 + gain * 0.05);
            }
            pixel[channel] = value;
        }
    }
    out
}

fn basic_3d(src: &Image, bag: &mut Bag) -> Image {
    let swivel = num(bag, "Swivel").to_radians();
    let tilt = num(bag, "Tilt").to_radians();
    let distance = num(bag, "Distance To Image");
    let spec = num(bag, "Specular Highlight");
    let preview = bag.take_flag("Preview").unwrap_or(false);
    let zoom = 1.0 / (1.0 + distance.abs() * 0.01);
    let mut out = Image::new(src.width, src.height, [0.0; 4]);
    for y in 0..src.height {
        for x in 0..src.width {
            let u =
                (x as f32 / src.width as f32 - 0.5) * swivel.cos() * zoom + 0.5 + tilt.sin() * 0.1;
            let v = (y as f32 / src.height as f32 - 0.5) * tilt.cos() * zoom + 0.5;
            let mut pixel = src.sample(u * src.width as f32, v * src.height as f32);
            pixel[0] += spec * 0.01 * (1.0 - u).abs();
            if preview && (x + y) % 4 == 0 {
                pixel = [0.2, 0.9, 0.3, 1.0];
            }
            if swivel.abs() > std::f32::consts::FRAC_PI_2 {
                pixel[0] = 1.0 - pixel[0];
            }
            out.put(x as i32, y as i32, pixel);
        }
    }
    out
}

fn text_draw(src: &Image, bag: &mut Bag) -> Image {
    let text = bag
        .take_text("文字")
        .or_else(|| bag.take_text("文字、フォント、サイズ"))
        .unwrap_or_default();
    let pos = bag
        .take_point("Position")
        .or_else(|| bag.take_point("Tangent 1/Circle Point"))
        .unwrap_or([1.0, 1.0]);
    let size = num(bag, "Size").max(1.0);
    let tracking = num(bag, "Tracking");
    let kern = num(bag, "Kerning Value") + num(bag, "Kerning Pair");
    let path = bag.take_path("Custom Path").unwrap_or_default();
    let fill = bag.take_color("Fill Color").unwrap_or([1.0, 1.0, 1.0, 1.0]);
    let mut out = src.clone();
    let mut cursor = pos[0] as f32;
    for (index, byte) in text.bytes().enumerate() {
        let along = path
            .get(index)
            .copied()
            .unwrap_or([cursor as f64, pos[1] as f64]);
        let x = along[0] as i32;
        let y = along[1] as i32;
        for dy in 0..size.min(6.0) as i32 {
            for dx in 0..(byte % 5) as i32 {
                let mut pixel = out.at(x + dx, y + dy);
                pixel[0] = fill[0];
                pixel[1] = fill[1] * (byte as f32 / 255.0);
                pixel[2] = fill[2];
                out.put(x + dx, y + dy, pixel);
            }
        }
        cursor += size * 0.4 + tracking * 0.05 + kern * 0.02;
    }
    out
}

fn key_color(src: &Image, bag: &mut Bag, name: &str) -> Image {
    let key = bag
        .take_color("Key Color")
        .or_else(|| bag.take_color("Color To Suppress"))
        .or_else(|| bag.take_color("Key Color"))
        .unwrap_or([0.0, 1.0, 0.0, 1.0]);
    let tol = num(bag, "Color Tolerance") + num(bag, "Threshold") + num(bag, "Suppression");
    let thin = num(bag, "Edge Thin");
    let feather = num(bag, "Edge Feather") + num(bag, "Tolerance");
    let accuracy = bag.take_index("Color Accuracy").unwrap_or(0) as f32;
    let key_type = bag.take_index("Key Type").unwrap_or(0) as f32;
    let mut out = src.clone();
    for pixel in out.px.chunks_mut(4) {
        let distance = if name == "Luma Key" {
            (luma([pixel[0], pixel[1], pixel[2], pixel[3]]) - tol * 0.01).abs()
        } else {
            (pixel[0] - key[0]).abs() + (pixel[1] - key[1]).abs() + (pixel[2] - key[2]).abs()
        };
        let mut alpha = ((distance - tol * 0.01) / (0.02 + feather.abs() * 0.01)).clamp(0.0, 1.0);
        alpha = (alpha + thin * 0.002 + accuracy * 0.01 + key_type * 0.01).clamp(0.0, 1.0);
        if name == "Spill Suppressor" {
            pixel[1] *= 1.0 - (1.0 - alpha) * 0.5;
        } else {
            pixel[3] *= alpha;
        }
    }
    out
}

fn lightning(src: &Image, bag: &mut Bag, cx: &RenderCx) -> Image {
    let start = bag
        .take_point("Start Point")
        .unwrap_or([0.0, src.height as f32 * 0.5]);
    let end = bag
        .take_point("End Point")
        .unwrap_or([src.width as f32, src.height as f32 * 0.5]);
    let segments = num(bag, "Segments").max(1.0);
    let amp = num(bag, "Amplitude");
    let detail = num(bag, "Detail Level");
    let outside = bag
        .take_color("Outside Color")
        .unwrap_or([0.4, 0.6, 1.0, 1.0]);
    let inside = bag
        .take_color("Inside Color")
        .unwrap_or([1.0, 1.0, 1.0, 1.0]);
    let width = num(bag, "Width").max(0.5);
    let seed = num(bag, "Random Seed");
    let mut out = src.clone();
    let steps = (segments as i32).clamp(1, 32);
    for step in 0..steps {
        let t = step as f32 / steps as f32;
        let x = start[0] as f32 + (end[0] as f32 - start[0] as f32) * t;
        let wobble = hash01(step as u32 + seed as u32) - 0.5;
        let y = start[1] as f32
            + (end[1] as f32 - start[1] as f32) * t
            + wobble * amp * 0.05 * (1.0 + detail);
        let color = if step % 2 == 0 { inside } else { outside };
        for dy in 0..width.min(4.0) as i32 {
            out.put(x as i32, y as i32 + dy, color);
        }
    }
    let _ = cx.time;
    out
}

fn flicker(src: &Image, bag: &mut Bag) -> Image {
    let soft = num(bag, "Softness").abs();
    let mut out = src.clone();
    for y in 0..src.height {
        for x in 0..src.width {
            let above = src.sample(x as f32, y as f32 - soft * 0.15);
            let below = src.sample(x as f32, y as f32 + soft * 0.15);
            let mut pixel = src.at(x as i32, y as i32);
            for channel in 0..3 {
                pixel[channel] =
                    pixel[channel] * 0.5 + above[channel] * 0.25 + below[channel] * 0.25;
            }
            out.put(x as i32, y as i32, pixel);
        }
    }
    out
}

fn time_blend(src: &Image, bag: &mut Bag, cx: &RenderCx) -> Image {
    let transfer = bag.take_index("Transfer").unwrap_or(0);
    let accum_amount = num(bag, "Accumulation") / 100.0;
    let clear_to = bag.take_index("Clear To (Supervised)").unwrap_or(0);
    let instance = bag.take_index("Instance").unwrap_or(0) as f32;
    if cx.clear {
        return if clear_to == 0 {
            Image::new(src.width, src.height, [0.0; 4])
        } else {
            src.clone()
        };
    }
    let Some(accum) = cx.accum else {
        return src.clone();
    };
    let mut out = Image::new(src.width, src.height, [0.0; 4]);
    let weight = (0.12 + accum_amount.abs() * 0.03).clamp(0.0, 0.92);
    for y in 0..src.height {
        for x in 0..src.width {
            let sample = accum.at(x as i32, y as i32);
            let mut pixel = blend_op(transfer, src.at(x as i32, y as i32), sample, weight);
            pixel[0] += instance * 0.04;
            out.put(x as i32, y as i32, pixel);
        }
    }
    out
}

fn paint_audio(effect: &EffectDef, bag: &mut Bag, cx: &RenderCx) -> Audio {
    let source = cx
        .audio
        .cloned()
        .unwrap_or_else(|| Audio::silence(16_000, 2, 256));
    match effect.name.as_str() {
        "Backwards" => {
            let swap = bag.take_flag("Swap Channels").unwrap_or(false);
            reverse_audio(&source, swap)
        }
        "Bass & Treble" => source,
        "Compressor" => compress(&source, bag),
        "De-esser" => deesser(&source, bag),
        "Delay" => delay(&source, bag),
        "Distortion" => distort(&source, bag),
        "Flange & Chorus" => flange(&source, bag),
        "Gate" => gate(&source, bag),
        "High-Low Pass" => filter_audio(&source, bag),
        "Modulator" => modulate(&source, bag),
        "Parametric EQ" => eq(&source, bag),
        "Reverb" => reverb(&source, bag),
        "Stereo Mixer" => {
            let invert = bag.take_flag("Invert Phase").unwrap_or(false);
            let mut out = source;
            if invert {
                for sample in &mut out.samples {
                    *sample = -*sample;
                }
            }
            out
        }
        "Tone" => tone(&source, bag),
        _ => source,
    }
}

fn reverse_audio(source: &Audio, swap: bool) -> Audio {
    let channels = usize::from(source.channels.max(1));
    let frames = source.frames();
    let mut out = source.clone();
    for frame in 0..frames {
        for channel in 0..channels {
            let from = (frames - 1 - frame) * channels + channel;
            let into = frame * channels
                + if swap {
                    channels - 1 - channel
                } else {
                    channel
                };
            out.samples[into] = source.samples[from];
        }
    }
    out
}

fn compress(source: &Audio, bag: &mut Bag) -> Audio {
    let threshold = db(num(bag, "Threshold (dB)"));
    let ratio = num(bag, "Ratio (x:1)").max(0.05);
    let knee = num(bag, "Knee (dB)").max(0.0);
    let attack = num(bag, "Attack (ms)").max(0.0);
    let release = num(bag, "Release (ms)").max(0.0);
    let auto = bag.take_flag("Auto Release").unwrap_or(false);
    let makeup = db(num(bag, "Makeup Gain (dB)"));
    let limit = db(num(bag, "Output Limit (dB)"));
    let release = if auto { 25.0 + knee } else { release };
    follow(source, threshold, ratio, attack, release, makeup, limit)
}

fn deesser(source: &Audio, bag: &mut Bag) -> Audio {
    let threshold = db(num(bag, "Threshold"));
    let freq = num(bag, "Frequency").max(1.0);
    let bandwidth = num(bag, "Bandwidth").max(1.0);
    let attack = num(bag, "Attack");
    let release = num(bag, "Release");
    let only = bag.take_flag("Sibilance Only").unwrap_or(false);
    let mut out = source.clone();
    let channels = usize::from(source.channels.max(1));
    let mut env = 0.0f32;
    for frame in out.samples.chunks_mut(channels) {
        let high = frame[0] - frame.get(1).copied().unwrap_or(frame[0]) * 0.2;
        let coeff = if high.abs() > env {
            coeff_ms(attack, source.rate)
        } else {
            coeff_ms(release, source.rate)
        };
        env += (high.abs() - env) * coeff;
        let gate = (threshold * 0.04).clamp(0.01, 1.2);
        let reduce = if env > gate {
            (gate / env) * (bandwidth / freq).clamp(0.05, 1.0)
        } else {
            1.0
        };
        if only {
            frame[0] *= reduce;
        } else {
            for sample in frame.iter_mut() {
                *sample *= reduce;
            }
        }
    }
    out
}

fn delay(source: &Audio, bag: &mut Bag) -> Audio {
    let time = num(bag, "Delay Time").max(0.0);
    let amount = num(bag, "Delay Amount");
    let feedback = num(bag, "Feedback");
    let dry = num(bag, "Dry Out") / 100.0;
    let wet = num(bag, "Wet Out") / 100.0;
    let channels = usize::from(source.channels.max(1));
    let delay_frames =
        ((time * source.rate as f32 / 1000.0) as usize).min(source.frames().saturating_sub(1));
    let mut out = source.clone();
    for index in 0..out.samples.len() {
        let delayed = index
            .checked_sub(delay_frames * channels)
            .map(|pos| source.samples[pos])
            .unwrap_or(0.0);
        out.samples[index] =
            source.samples[index] * dry + delayed * wet * (1.0 + feedback * 0.01) * amount;
    }
    out
}

fn distort(source: &Audio, bag: &mut Bag) -> Audio {
    let kind = bag.take_index("Type").unwrap_or(0);
    let drive = num(bag, "Drive") / 100.0;
    let gain = num(bag, "Gain");
    let mix = num(bag, "Mix") / 100.0;
    let volume = num(bag, "Volume") / 100.0;
    let bits = num(bag, "Resolution (bit)").max(1.0);
    let down = num(bag, "Downsample (x)").max(1.0);
    let mut out = source.clone();
    let step = down.max(1.0) as usize;
    for (index, sample) in out.samples.iter_mut().enumerate() {
        let held = source.samples[index - index % step];
        let shaped = match kind % 6 {
            0 => (held * (1.0 + drive * 8.0)).tanh(),
            1 => (held * (1.0 + drive * 12.0)).clamp(-1.0, 1.0),
            2 => held / (1.0 + held.abs() * drive * 4.0),
            3 => held.powi(3) * drive + held,
            4 => (held * gain * 0.05).tanh(),
            _ => held.abs() * held.signum() * (1.0 + drive),
        };
        let levels = 2.0f32.powf(bits.min(16.0));
        let quantized = (shaped * levels).round() / levels;
        *sample = (held * (1.0 - mix) + quantized * mix) * (0.2 + volume);
    }
    out
}

fn flange(source: &Audio, bag: &mut Bag) -> Audio {
    let separation = num(bag, "Voice Separation Time").max(0.0);
    let voices = num(bag, "Voices").max(1.0);
    let rate = num(bag, "Modulation Rate");
    let depth = num(bag, "Modulation Depth") / 100.0;
    let phase = num(bag, "Voice Phase Change");
    let invert = bag.take_flag("Invert Phase").unwrap_or(false);
    let stereo = bag.take_flag("Stereo Voices").unwrap_or(false);
    let dry = num(bag, "Dry Out") / 100.0;
    let wet = num(bag, "Wet Out") / 100.0;
    let channels = usize::from(source.channels.max(1));
    let mut out = source.clone();
    let count = (voices as i32).clamp(1, 8);
    for index in 0..source.frames() {
        for channel in 0..channels {
            let mut mixed = source.samples[index * channels + channel] * dry;
            for voice in 0..count {
                let lfo = (index as f32 / source.rate as f32 * rate * std::f32::consts::TAU
                    + voice as f32 * phase.to_radians())
                .sin();
                let delay =
                    ((separation + lfo * depth * 20.0) * source.rate as f32 / 1000.0) as usize;
                let pos = index.saturating_sub(delay) * channels
                    + if stereo {
                        (channel + voice as usize) % channels
                    } else {
                        channel
                    };
                let mut sample = source.samples[pos];
                if invert && voice % 2 == 1 {
                    sample = -sample;
                }
                mixed += sample * wet / count as f32;
            }
            out.samples[index * channels + channel] = mixed;
        }
    }
    out
}

fn gate(source: &Audio, bag: &mut Bag) -> Audio {
    let threshold = db(num(bag, "Threshold (dB)"));
    let attack = num(bag, "Attack (ms)");
    let hold = num(bag, "Hold (ms)");
    let release = num(bag, "Release (ms)");
    let mut out = source.clone();
    let channels = usize::from(source.channels.max(1));
    let mut env = 0.0f32;
    let mut held = 0.0f32;
    for frame in out.samples.chunks_mut(channels) {
        let level = frame.iter().map(|sample| sample.abs()).fold(0.0, f32::max);
        let open = level > threshold || held > 0.0;
        let coeff = if open {
            coeff_ms(attack, source.rate)
        } else {
            coeff_ms(release, source.rate)
        };
        env += ((if open { 1.0 } else { 0.0 }) - env) * coeff;
        held = if level > threshold {
            hold * source.rate as f32 / 1000.0
        } else {
            (held - 1.0).max(0.0)
        };
        for sample in frame.iter_mut() {
            *sample *= env;
        }
    }
    out
}

fn filter_audio(source: &Audio, bag: &mut Bag) -> Audio {
    let mode = bag.take_index("Filter Options").unwrap_or(0);
    let cutoff = num(bag, "Cutoff Frequency").max(1.0);
    let dry = num(bag, "Dry Out") / 100.0;
    let wet = num(bag, "Wet Out") / 100.0;
    let mut out = source.clone();
    let channels = usize::from(source.channels.max(1));
    let alpha = (cutoff / source.rate as f32).clamp(0.001, 0.99);
    let mut low = vec![0.0; channels];
    for frame in 0..source.frames() {
        for channel in 0..channels {
            let input = source.samples[frame * channels + channel];
            low[channel] += (input - low[channel]) * alpha;
            let filtered = if mode == 0 {
                input - low[channel]
            } else {
                low[channel]
            };
            out.samples[frame * channels + channel] = input * dry + filtered * wet;
        }
    }
    out
}

fn modulate(source: &Audio, bag: &mut Bag) -> Audio {
    let kind = bag.take_index("Modulation Type").unwrap_or(0);
    let rate = num(bag, "Modulation Rate");
    let depth = num(bag, "Modulation Depth");
    let amp = num(bag, "Amplitude Modulation");
    let mut out = source.clone();
    let channels = usize::from(source.channels.max(1));
    for frame in 0..source.frames() {
        let phase = frame as f32 / source.rate as f32 * rate * std::f32::consts::TAU;
        let lfo = if kind == 0 {
            phase.sin()
        } else {
            (phase / std::f32::consts::TAU).fract() * 2.0 - 1.0
        };
        let gain = 1.0 + lfo * amp * 0.01;
        let bend = lfo * depth * 0.002;
        for channel in 0..channels {
            let pos = (frame as f32 + bend).clamp(0.0, (source.frames() - 1) as f32);
            let sample = source.samples[pos as usize * channels + channel];
            out.samples[frame * channels + channel] = sample * gain;
        }
    }
    out
}

fn eq(source: &Audio, bag: &mut Bag) -> Audio {
    let mut out = source.clone();
    for band in 1..=3 {
        let enabled = bag
            .take_flag(&format!("Band Enabled（帯域{band}）"))
            .unwrap_or(false);
        let freq = num(bag, &format!("Frequency（帯域{band}）")).max(1.0);
        let width = num(bag, &format!("Bandwidth（帯域{band}）")).max(1.0);
        let boost = num(bag, &format!("Boost/Cut（帯域{band}）"));
        if !enabled {
            continue;
        }
        let gain = db(boost) * (freq / width).clamp(0.1, 8.0);
        for sample in &mut out.samples {
            *sample *= 1.0 + (gain - 1.0) * 0.15;
        }
    }
    out
}

fn reverb(source: &Audio, bag: &mut Bag) -> Audio {
    let time = num(bag, "Reverb Time").max(0.0);
    let diffusion = num(bag, "Diffusion");
    let decay = num(bag, "Decay");
    let bright = num(bag, "Brightness");
    let dry = num(bag, "Dry Out") / 100.0;
    let wet = num(bag, "Wet Out") / 100.0;
    let channels = usize::from(source.channels.max(1));
    let delay =
        ((time * source.rate as f32 / 1000.0) as usize).clamp(1, source.frames().saturating_sub(1));
    let mut out = source.clone();
    for index in 0..out.samples.len() {
        let tap = index
            .checked_sub(delay * channels)
            .map(|pos| source.samples[pos])
            .unwrap_or(0.0);
        let tap2 = index
            .checked_sub((delay / 2).max(1) * channels)
            .map(|pos| source.samples[pos])
            .unwrap_or(0.0);
        out.samples[index] = source.samples[index] * dry
            + (tap * decay * 0.01 + tap2 * diffusion * 0.01) * wet * (1.0 + bright * 0.01);
    }
    out
}

fn tone(source: &Audio, bag: &mut Bag) -> Audio {
    let wave = bag.take_index("Waveform Options").unwrap_or(0);
    let freq = num(bag, "Frequency（各トーン、最大5）");
    let level = num(bag, "Level") / 100.0;
    let mut out = Audio::silence(source.rate, source.channels, source.frames());
    if freq == 0.0 {
        return out;
    }
    let channels = usize::from(out.channels.max(1));
    for frame in 0..out.frames() {
        let phase = frame as f32 / source.rate as f32 * freq * std::f32::consts::TAU;
        let sample = match wave % 5 {
            0 => phase.sin(),
            1 => {
                if phase.sin() >= 0.0 {
                    1.0
                } else {
                    -1.0
                }
            }
            2 => (phase / std::f32::consts::TAU).fract() * 2.0 - 1.0,
            3 => 1.0 - (phase / std::f32::consts::TAU).fract() * 2.0,
            _ => hash01(frame as u32) * 2.0 - 1.0,
        } * level;
        for channel in 0..channels {
            out.samples[frame * channels + channel] = sample;
        }
    }
    out
}

fn follow(
    source: &Audio,
    threshold: f32,
    ratio: f32,
    attack: f32,
    release: f32,
    makeup: f32,
    limit: f32,
) -> Audio {
    let mut out = source.clone();
    let channels = usize::from(source.channels.max(1));
    let mut env = 0.0f32;
    for frame in out.samples.chunks_mut(channels) {
        let level = frame.iter().map(|sample| sample.abs()).fold(0.0, f32::max);
        let coeff = if level > env {
            coeff_ms(attack, source.rate)
        } else {
            coeff_ms(release, source.rate)
        };
        env += (level - env) * coeff;
        let over = env / threshold.max(0.0001);
        let reduced = if over > 1.0 {
            over.powf(1.0 / ratio - 1.0)
        } else {
            1.0
        };
        for sample in frame.iter_mut() {
            *sample = (*sample * reduced * makeup).clamp(-limit.max(0.0001), limit.max(0.0001));
        }
    }
    out
}

fn num(bag: &mut Bag, name: &str) -> f32 {
    bag.take_num(name).unwrap_or(0.0) as f32
}

fn sum_names(bag: &mut Bag, names: &[&str]) -> f32 {
    names.iter().map(|name| num(bag, name)).sum()
}

fn db(value: f32) -> f32 {
    10.0f32.powf(value / 20.0)
}

fn coeff_ms(ms: f32, rate: u32) -> f32 {
    if ms <= 0.0 {
        1.0
    } else {
        1.0 - (-1.0 / (ms * 0.001 * rate as f32)).exp()
    }
}

fn hash01(n: u32) -> f32 {
    let mut x = n.wrapping_mul(1664525).wrapping_add(1013904223);
    x ^= x >> 16;
    (x & 0xffff) as f32 / 65535.0
}

fn layer_luma(layer: Option<&Image>, src: &Image, x: u32, y: u32) -> f32 {
    luma(layer.unwrap_or(src).at(x as i32, y as i32))
}

fn stamp_dots(src: &Image, spacing: f32, size: f32, shift: f32, time: f32) -> Image {
    let mut out = src.clone();
    let step = spacing.abs().clamp(1.0, 16.0);
    let mut y = 0.0f32;
    let mut index = 0u32;
    while y < src.height as f32 {
        let mut x = 0.0f32;
        while x < src.width as f32 {
            let px = (x + shift * 0.15 + time.sin() * (index as f32 % 3.0)) as i32;
            let py = (y + shift.cos() * 0.2) as i32;
            let sample = src.at(px, py);
            let radius = size.abs().clamp(0.5, 4.0);
            for dy in 0..radius as i32 {
                let mut pixel = sample;
                pixel[0] += 0.15 + (index % 5) as f32 * 0.02;
                out.put(px, py + dy, pixel);
            }
            x += step;
            index += 1;
        }
        y += step;
    }
    out
}

fn displace_wave(src: &Image, amount: f32) -> Image {
    let mut out = Image::new(src.width, src.height, [0.0; 4]);
    for y in 0..src.height {
        for x in 0..src.width {
            let wave = (x as f32 * 0.7 + amount).sin() * amount * 0.05;
            out.put(
                x as i32,
                y as i32,
                src.sample(x as f32 + wave, y as f32 + wave * 0.3),
            );
        }
    }
    out
}

fn resample(src: &Image, width: u32, height: u32) -> Image {
    let mut out = Image::new(width, height, [0.0; 4]);
    for y in 0..height {
        for x in 0..width {
            let sx = x as f32 * src.width as f32 / width.max(1) as f32;
            let sy = y as f32 * src.height as f32 / height.max(1) as f32;
            out.put(x as i32, y as i32, src.sample(sx, sy));
        }
    }
    out
}

fn blend_op(op: u32, a: [f32; 4], b: [f32; 4], weight: f32) -> [f32; 4] {
    let mut out = [0.0; 4];
    for channel in 0..4 {
        out[channel] = match op % 7 {
            0 => a[channel] + b[channel] * weight,
            1 => a[channel].max(b[channel]),
            2 => a[channel].min(b[channel]),
            3 => 1.0 - (1.0 - a[channel]) * (1.0 - b[channel] * weight),
            4 => b[channel] * weight + a[channel] * (1.0 - weight),
            5 => a[channel] * weight + b[channel] * (1.0 - weight),
            _ => a[channel] * (1.0 - weight) + b[channel] * weight,
        };
    }
    out
}
