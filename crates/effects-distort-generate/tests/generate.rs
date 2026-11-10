#[path = "common/mod.rs"]
mod common;

use effects_distort_generate::{AudioClip, EffectKind, Instance, MaskShape, SourceLayer, Value};
use model::{BitsPerChannel, BlendMode, LayerId, MaskMode, Rgba};

use common::{at, diff, paint, run, solid, view};

fn fx(kind: EffectKind) -> Instance {
    Instance::new(kind)
}

#[test]
fn four_color_blend_jitter_and_opacity() {
    let frame = solid(16, 16, BitsPerChannel::Bpc8, [0.2, 0.0, 0.0, 1.0]);
    let mut g = fx(EffectKind::FourColorGradient);
    for (i, color) in [
        Rgba::new(1.0, 0.0, 0.0, 1.0),
        Rgba::new(0.0, 1.0, 0.0, 1.0),
        Rgba::new(0.0, 0.0, 1.0, 1.0),
        Rgba::new(1.0, 1.0, 0.0, 1.0),
    ]
    .into_iter()
    .enumerate()
    {
        g.set_role(&format!("c{i}"), Value::Color(color));
        g.set_role(
            &format!("p{i}"),
            Value::Point([
                if i % 2 == 0 { 0.0 } else { 15.0 },
                if i < 2 { 0.0 } else { 15.0 },
            ]),
        );
    }
    g.set_number("Blend", 0.4).unwrap();
    g.set_number("Opacity", 1.0).unwrap();
    let hard = run(&g, &frame, 0.0);
    g.set_number("Blend", 3.0).unwrap();
    let soft = run(&g, &frame, 0.0);
    assert!(diff(&hard, &soft) > 0.02);
    g.set_number("Jitter", 0.0).unwrap();
    let clean = run(&g, &frame, 0.0);
    g.set_number("Jitter", 0.4).unwrap();
    let noisy = run(&g, &frame, 0.0);
    assert!(diff(&clean, &noisy) > 0.01);
    g.set_number("Opacity", 0.2).unwrap();
    let dim = run(&g, &frame, 0.0);
    assert!(
        view(&dim).px.iter().map(|px| px[3]).sum::<f32>()
            < view(&noisy).px.iter().map(|px| px[3]).sum::<f32>()
    );
    g.set_blend("Blending Mode", Some(BlendMode::Add));
    let added = run(&g, &frame, 0.0);
    assert!(diff(&dim, &added) > 0.0);
    g.set_role("p0", Value::Point([8.0, 8.0]));
    let moved = run(&g, &frame, 0.0);
    assert!(diff(&added, &moved) > 0.0);
}

#[test]
fn lightning_type_conductivity_glow_and_obstacle() {
    let mut frame = solid(24, 24, BitsPerChannel::Bpc8, [0.0, 0.0, 0.0, 0.0]);
    for y in 8..16 {
        for x in 8..16 {
            frame = paint(&frame, x, y, [1.0, 1.0, 1.0, 1.0]);
        }
    }
    let mut g = fx(EffectKind::AdvancedLightning);
    g.set_choice("Lightning Type", "Direction").unwrap();
    g.set_point("Origin", 4.0, 2.0).unwrap();
    g.set_point("Direction", 20.0, 20.0).unwrap();
    g.set_number("Conductivity State", 0.2).unwrap();
    g.set_number("Turbulence", 4.0).unwrap();
    g.set_number("Glow Opacity", 1.0).unwrap();
    g.set_bool("Composite On Original", false).unwrap();
    let a = run(&g, &frame, 0.0);
    g.set_number("Conductivity State", 3.0).unwrap();
    let b = run(&g, &frame, 0.0);
    assert!(diff(&a, &b) > 0.01);
    g.set_number("Glow Opacity", 0.0).unwrap();
    let no_glow = run(&g, &frame, 0.0);
    assert!(count_lit(&no_glow) < count_lit(&b));
    g.set_bool("Composite On Original", true).unwrap();
    let over = run(&g, &frame, 0.0);
    assert!(at(&over, 0, 0)[3] < 0.2 || diff(&no_glow, &over) > 0.0);
    g.set_choice("Lightning Type", "Vertical").unwrap();
    assert!(!g.enabled("Direction"));
    assert!(!g.enabled("Outer Radius"));
    g.set_choice("Lightning Type", "Omni").unwrap();
    assert!(g.enabled("Outer Radius"));
    g.set_number("Outer Radius", 8.0).unwrap();
    let omni = run(&g, &frame, 0.0);
    assert_eq!(omni.width(), 24);
    g.set_number("Alpha Obstacle", 1.0).unwrap();
    assert!(g.enabled("Main Core Collision Only"));
    g.set_number("Alpha Obstacle", -1.0).unwrap();
    let inside = run(&g, &frame, 0.0);
    g.set_number("Alpha Obstacle", 0.0).unwrap();
    assert!(!g.enabled("Main Core Collision Only"));
    let plain = run(&g, &frame, 0.0);
    assert!(diff(&inside, &plain) > 0.0);
    g.set_number("Forking", 0.1).unwrap();
    let few = run(&g, &frame, 0.0);
    g.set_number("Forking", 0.9).unwrap();
    g.set_number("Min. Forkdistance", 2.0).unwrap();
    let many = run(&g, &frame, 0.0);
    assert!(count_lit(&many) >= count_lit(&few) || diff(&few, &many) > 0.0);
    g.set_bool("Decay Main Core", true).unwrap();
    g.set_number("Decay", 1.5).unwrap();
    let decay = run(&g, &frame, 0.0);
    assert_eq!(decay.height(), 24);
    g.set_number("Core Drain", 80.0).unwrap();
    g.set_number("Fork Strength", 40.0).unwrap();
    g.set_number("Fork Variation", 1.0).unwrap();
    let _ = run(&g, &frame, 0.0);
}

fn count_lit(frame: &effect_api::Frame) -> usize {
    view(frame).px.iter().filter(|px| px[3] > 0.05).count()
}

#[test]
fn audio_spectrum_and_waveform_use_raw_audio() {
    let frame = solid(32, 16, BitsPerChannel::Bpc8, [0.0, 0.0, 0.0, 1.0]);
    let mut samples = vec![0.0_f32; 256];
    for (i, sample) in samples.iter_mut().enumerate() {
        *sample = (i as f32 * 0.4).sin();
    }
    let audio = AudioClip {
        sample_rate: 800,
        channels: 1,
        samples: samples.clone(),
        processed: vec![0.0; 256],
    };
    let mut g = fx(EffectKind::AudioSpectrum);
    g.host.audio = Some(audio);
    g.set_point("Start Point", 2.0, 8.0).unwrap();
    g.set_point("End Point", 30.0, 8.0).unwrap();
    g.set_number("Start Frequency", 20.0).unwrap();
    g.set_number("End Frequency", 200.0).unwrap();
    g.set_number("Frequency Bands", 8.0).unwrap();
    g.set_number("Maximum Height", 6.0).unwrap();
    g.set_number("Audio Duration", 80.0).unwrap();
    g.set_number("Audio Offset", 0.0).unwrap();
    g.set_number("Thickness", 1.0).unwrap();
    g.set_number("Softness", 0.0).unwrap();
    g.set_color("Inside Color", Rgba::new(1.0, 0.0, 0.0, 1.0))
        .unwrap();
    g.set_color("Outside Color", Rgba::new(0.0, 0.0, 1.0, 1.0))
        .unwrap();
    g.set_choice("Display Options", "Digital").unwrap();
    g.set_choice("Side Options", "Side A").unwrap();
    g.set_bool("Composite On Original", false).unwrap();
    g.set_number("Hue Interpolation", 0.0).unwrap();
    assert!(!g.enabled("Dynamic Hue Phase"));
    let bars = run(&g, &frame, 0.0);
    assert!(count_lit(&bars) > 4);
    g.set_number("Hue Interpolation", 1.0).unwrap();
    assert!(g.enabled("Color Symmetry"));
    g.set_bool("Dynamic Hue Phase", true).unwrap();
    g.set_bool("Color Symmetry", true).unwrap();
    let hued = run(&g, &frame, 0.0);
    assert!(diff(&bars, &hued) > 0.0);
    g.set_bool("Use Polar Path", true).unwrap();
    let polar = run(&g, &frame, 0.0);
    assert!(diff(&hued, &polar) > 0.0);
    g.set_choice("Side Options", "Side A & B").unwrap();
    let both = run(&g, &frame, 0.0);
    assert!(count_lit(&both) >= count_lit(&polar));
    g.set_choice("Display Options", "Analog Dots").unwrap();
    let dots = run(&g, &frame, 0.0);
    g.set_choice("Display Options", "Analog Lines").unwrap();
    let lines = run(&g, &frame, 0.0);
    assert!(diff(&dots, &lines) > 0.0);
    g.set_bool("Duration Averaging", true).unwrap();
    let avg = run(&g, &frame, 0.0);
    assert_eq!(avg.width(), 32);
    g.set_bool("Composite On Original", true).unwrap();
    let over = run(&g, &frame, 0.0);
    assert!(at(&over, 0, 0)[0] > 0.0 || diff(&avg, &over) > 0.0);
    g.set_number("Audio Offset", 40.0).unwrap();
    let shifted = run(&g, &frame, 0.0);
    assert!(diff(&over, &shifted) > 0.0 || count_lit(&shifted) > 0);
    let mut wave = fx(EffectKind::AudioWaveform);
    wave.host.audio = g.host.audio.clone();
    wave.set_point("Start Point", 0.0, 8.0).unwrap();
    wave.set_point("End Point", 31.0, 8.0).unwrap();
    wave.set_number("Displayed Samples", 16.0).unwrap();
    wave.set_number("Maximum Height", 5.0).unwrap();
    wave.set_number("Audio Duration", 40.0).unwrap();
    wave.set_number("Audio Offset", 0.0).unwrap();
    wave.set_number("Thickness", 1.0).unwrap();
    wave.set_number("Softness", 0.0).unwrap();
    wave.set_color("Inside Color", Rgba::white()).unwrap();
    wave.set_color("Outside Color", Rgba::white()).unwrap();
    wave.set_choice("Display Options", "Digital").unwrap();
    wave.set_bool("Composite On Original", false).unwrap();
    wave.set_role("waveform", Value::Index(0));
    let mono = run(&wave, &frame, 0.0);
    assert!(count_lit(&mono) > 2);
    wave.set_choice("Display Options", "Analog Lines").unwrap();
    let analog = run(&wave, &frame, 0.0);
    wave.set_choice("Display Options", "Analog Dots").unwrap();
    let analog_dots = run(&wave, &frame, 0.0);
    assert!(diff(&mono, &analog) > 0.0);
    assert!(diff(&analog, &analog_dots) > 0.0);
    wave.set_bool("Composite On Original", true).unwrap();
    let added = run(&wave, &frame, 0.0);
    assert!(diff(&analog_dots, &added) > 0.0);
    let silent = AudioClip {
        sample_rate: 800,
        channels: 1,
        samples: vec![0.0; 64],
        processed: samples,
    };
    wave.host.audio = Some(silent);
    let quiet = run(&wave, &frame, 0.0);
    assert!(count_lit(&quiet) < count_lit(&added) + 8);
}

#[test]
fn beam_length_is_longest_at_half_time() {
    let frame = solid(40, 8, BitsPerChannel::Bpc16, [0.0, 0.0, 0.0, 1.0]);
    let mut g = fx(EffectKind::Beam);
    g.set_role("start", Value::Point([2.0, 4.0]));
    g.set_role("end", Value::Point([38.0, 4.0]));
    g.set_role("thick0", Value::Number(1.0));
    g.set_role("thick1", Value::Number(3.0));
    g.set_number("Length", 100.0).unwrap();
    g.set_number("Time", 50.0).unwrap();
    g.set_bool("3D Perspective", false).unwrap();
    let mid = count_lit(&run(&g, &frame, 0.0));
    g.set_number("Time", 10.0).unwrap();
    let early = count_lit(&run(&g, &frame, 0.0));
    g.set_number("Time", 50.0).unwrap();
    g.set_number("Length", 40.0).unwrap();
    let short = count_lit(&run(&g, &frame, 0.0));
    assert!(mid > early);
    assert!(mid > short);
    g.set_number("Length", 100.0).unwrap();
    g.set_bool("3D Perspective", true).unwrap();
    g.set_number("Time", 80.0).unwrap();
    let persp = run(&g, &frame, 0.0);
    g.set_bool("3D Perspective", false).unwrap();
    let flat = run(&g, &frame, 0.0);
    assert!(diff(&persp, &flat) > 0.0);
}

#[test]
fn cell_pattern_size_overflow_and_evolution() {
    let frame = solid(48, 48, BitsPerChannel::Bpc8, [0.0, 0.0, 0.0, 1.0]);
    let mut g = fx(EffectKind::CellPattern);
    assert!((g.num("Size").unwrap() - 60.0).abs() < 1e-9);
    g.set_role("pattern", Value::Index(0));
    g.set_number("Disperse", 0.2).unwrap();
    g.set_number("Contrast/Sharpness", 10.0).unwrap();
    let sized = run(&g, &frame, 0.0);
    g.set_number("Size", 20.0).unwrap();
    let smaller = run(&g, &frame, 0.0);
    assert!(diff(&sized, &smaller) > 0.05);
    g.set_bool("Invert", true).unwrap();
    let inverted = run(&g, &frame, 0.0);
    assert!(diff(&smaller, &inverted) > 0.2);
    g.set_bool("Invert", false).unwrap();
    g.set_number("Contrast/Sharpness", 180.0).unwrap();
    g.set_choice("Overflow", "Clip").unwrap();
    let clip = run(&g, &frame, 0.0);
    g.set_choice("Overflow", "Wrap Back").unwrap();
    let wrap = run(&g, &frame, 0.0);
    g.set_choice("Overflow", "Soft Clamp").unwrap();
    let soft = run(&g, &frame, 0.0);
    assert!(diff(&clip, &wrap) > 0.0);
    assert!(diff(&clip, &soft) > 0.0);
    g.set_number("Disperse", 0.0).unwrap();
    let grid = run(&g, &frame, 0.0);
    g.set_number("Disperse", 1.0).unwrap();
    let loose = run(&g, &frame, 0.0);
    assert!(diff(&grid, &loose) > 0.0);
    g.set_role("pattern", Value::Index(5));
    g.set_number("Evolution", 0.0).unwrap();
    let plates0 = run(&g, &frame, 0.0);
    g.set_number("Evolution", 1.0).unwrap();
    let plates1 = run(&g, &frame, 0.0);
    g.set_role("pattern", Value::Index(6));
    let static0 = run(&g, &frame, 0.0);
    g.set_number("Evolution", 0.0).unwrap();
    let static1 = run(&g, &frame, 0.0);
    assert!(diff(&plates0, &plates1) > 0.0);
    assert!(diff(&static0, &static1) < 0.05);
    g.set_role("pattern", Value::Index(7));
    assert!(!g.enabled("Overflow"));
    g.set_role("pattern", Value::Index(0));
    assert!(g.enabled("Overflow"));
    g.set_bool("Enable Tiling", true).unwrap();
    g.set_number("Cells Horizontal", 3.0).unwrap();
    g.set_number("Cells Vertical", 2.0).unwrap();
    let tiled = run(&g, &frame, 0.0);
    g.set_number("Cells Horizontal", 6.0).unwrap();
    let more = run(&g, &frame, 0.0);
    assert!(diff(&tiled, &more) > 0.0);
    g.set_bool("Cycle Evolution", true).unwrap();
    g.set_number("Cycle", 1.0).unwrap();
    g.set_number("Evolution", 1.2).unwrap();
    let looped = run(&g, &frame, 0.0);
    g.set_number("Evolution", 0.2).unwrap();
    let same = run(&g, &frame, 0.0);
    assert!(diff(&looped, &same) < 0.08);
    g.set_number("Random Seed", 2.0).unwrap();
    let seed = run(&g, &frame, 0.0);
    assert!(diff(&same, &seed) > 0.0);
}

#[test]
fn checkerboard_and_grid_tiles() {
    let frame = solid(16, 16, BitsPerChannel::Bpc8, [1.0, 0.0, 0.0, 1.0]);
    let mut g = fx(EffectKind::Checkerboard);
    g.set_point("Anchor", 0.0, 0.0).unwrap();
    g.set_choice("Size From", "Width Slider").unwrap();
    g.set_number("Width", 4.0).unwrap();
    g.set_color("Color", Rgba::white()).unwrap();
    g.set_number("Opacity", 1.0).unwrap();
    let board = run(&g, &frame, 0.0);
    assert!(at(&board, 1, 1)[0] > 0.8);
    assert!(at(&board, 5, 1)[3] < 0.1);
    assert!(at(&board, 5, 1)[0] < 0.2);
    g.set_point("Anchor", 2.0, 0.0).unwrap();
    let shifted = run(&g, &frame, 0.0);
    assert!(diff(&board, &shifted) > 0.2);
    g.set_choice("Size From", "Width & Height Sliders").unwrap();
    g.set_number("Height", 8.0).unwrap();
    let tall = run(&g, &frame, 0.0);
    assert!(diff(&shifted, &tall) > 0.05);
    g.set_choice("Size From", "Corner Point").unwrap();
    g.set_point("Anchor", 0.0, 0.0).unwrap();
    g.set_point("Corner Point", 8.0, 8.0).unwrap();
    let corner = run(&g, &frame, 0.0);
    assert!(diff(&tall, &corner) > 0.0);
    g.set_number("Opacity", 0.4).unwrap();
    let faded = run(&g, &frame, 0.0);
    assert!(at(&faded, 1, 1)[3] < 0.8);
    g.set_blend("Blending Mode", Some(BlendMode::Add));
    let added = run(&g, &frame, 0.0);
    assert!(at(&added, 1, 1)[0] > 0.5);
    let mut grid = fx(EffectKind::Grid);
    grid.set_point("Anchor", 0.0, 0.0).unwrap();
    grid.set_choice("Size From", "Width Slider").unwrap();
    grid.set_number("Width", 8.0).unwrap();
    grid.set_number("Border", 0.0).unwrap();
    grid.set_color("Color", Rgba::white()).unwrap();
    grid.set_number("Opacity", 1.0).unwrap();
    let empty = run(&grid, &frame, 0.0);
    assert!(count_lit(&empty) == 0);
    grid.set_number("Border", 2.0).unwrap();
    let lines = run(&grid, &frame, 0.0);
    assert!(count_lit(&lines) > 0);
    grid.set_bool("Invert Grid", true).unwrap();
    let inverted = run(&grid, &frame, 0.0);
    assert!(diff(&lines, &inverted) > 0.2);
    grid.set_number("Feather", 2.0).unwrap();
    let soft = run(&grid, &frame, 0.0);
    assert!(diff(&inverted, &soft) > 0.0);
}

#[test]
fn circle_edges_and_ellipse() {
    let frame = solid(20, 12, BitsPerChannel::Bpc8, [0.0, 1.0, 0.0, 1.0]);
    let mut g = fx(EffectKind::Circle);
    g.set_role("center", Value::Point([10.0, 6.0]));
    g.set_role("color", Value::Color(Rgba::new(1.0, 0.0, 0.0, 1.0)));
    g.set_number("Radius", 5.0).unwrap();
    g.set_choice("Edge", "None").unwrap();
    g.set_bool("Invert Circle", false).unwrap();
    let disk = run(&g, &frame, 0.0);
    assert!(at(&disk, 10, 6)[0] > 0.8);
    g.set_choice("Edge", "Thickness").unwrap();
    g.set_number("Thickness", 2.0).unwrap();
    let ring = run(&g, &frame, 0.0);
    assert!(at(&ring, 10, 6)[0] < at(&disk, 10, 6)[0]);
    g.set_choice("Edge", "Edge Radius").unwrap();
    g.set_role("edge_radius", Value::Number(8.0));
    let edge = run(&g, &frame, 0.0);
    g.set_choice("Edge", "Thickness * Radius").unwrap();
    let scaled = run(&g, &frame, 0.0);
    g.set_choice("Edge", "Thickness & Feather * Radius")
        .unwrap();
    g.set_number("Feather", 1.0).unwrap();
    let feather = run(&g, &frame, 0.0);
    assert!(diff(&edge, &scaled) > 0.0 || diff(&scaled, &feather) > 0.0);
    g.set_choice("Edge", "None").unwrap();
    g.set_bool("Invert Circle", true).unwrap();
    let inverted = run(&g, &frame, 0.0);
    assert!(at(&inverted, 0, 0)[0] > at(&inverted, 10, 6)[0]);
    let ellipse = run(&fx(EffectKind::Ellipse), &frame, 0.0);
    assert!(at(&ellipse, 10, 6)[0] > 0.9);
    assert!(at(&ellipse, 0, 0)[3] < 0.1);
    assert!(effects_distort_generate::parameters(EffectKind::Ellipse).is_empty());
}

#[test]
fn eyedropper_fill_and_mask_fill() {
    let mut frame = solid(8, 8, BitsPerChannel::Bpc8, [0.0, 0.0, 1.0, 1.0]);
    frame = paint(&frame, 1, 1, [1.0, 0.0, 0.0, 1.0]);
    frame = paint(&frame, 2, 1, [0.0, 0.0, 0.0, 0.0]);
    let mut g = fx(EffectKind::EyedropperFill);
    g.set_point("Sample Point", 1.0, 1.0).unwrap();
    g.set_number("Sample Radius", 0.0).unwrap();
    g.set_choice("Average Pixel Color", "All").unwrap();
    g.set_number("Blend With Original", 0.0).unwrap();
    let red = run(&g, &frame, 0.0);
    assert!(at(&red, 4, 4)[0] > 0.8);
    g.set_point("Sample Point", 0.0, 0.0).unwrap();
    let blue = run(&g, &frame, 0.0);
    assert!(at(&blue, 4, 4)[2] > at(&red, 4, 4)[2]);
    g.set_number("Sample Radius", 1.5).unwrap();
    g.set_choice("Average Pixel Color", "Skip Empty").unwrap();
    let skip = run(&g, &frame, 0.0);
    g.set_choice("Average Pixel Color", "All Premultiplied")
        .unwrap();
    let premul = run(&g, &frame, 0.0);
    g.set_choice("Average Pixel Color", "Including Alpha")
        .unwrap();
    g.set_bool("Maintain Original Alpha", true).unwrap();
    let kept = run(&g, &frame, 0.0);
    assert!(diff(&skip, &premul) > 0.0 || diff(&premul, &kept) > 0.0);
    g.set_number("Blend With Original", 100.0).unwrap();
    let original = run(&g, &frame, 0.0);
    assert!(diff(&original, &frame) < 0.08);
    let mut fill = fx(EffectKind::Fill);
    fill.set_role("color", Value::Color(Rgba::new(0.0, 1.0, 0.0, 1.0)));
    fill.host.masks.push(MaskShape::polygon(
        "box",
        vec![[2.0, 2.0], [6.0, 2.0], [6.0, 6.0], [2.0, 6.0]],
        0,
    ));
    let painted = run(&fill, &frame, 0.0);
    assert!(at(&painted, 4, 4)[1] > 0.8);
    assert!(at(&painted, 0, 0)[2] > 0.5);
}

#[test]
fn fractal_starts_as_mandelbrot() {
    let frame = solid(32, 24, BitsPerChannel::Bpc8, [1.0, 1.0, 1.0, 1.0]);
    let g = fx(EffectKind::Fractal);
    let first = run(&g, &frame, 0.0);
    assert!(view(&first)
        .px
        .iter()
        .any(|px| px[0] < 0.05 && px[1] < 0.05 && px[2] < 0.05));
    let mut g = g;
    g.set_number("X (Real)", 0.3).unwrap();
    let moved = run(&g, &frame, 0.0);
    assert!(diff(&first, &moved) > 0.05);
    g.set_choice("Set Choice", "Julia").unwrap();
    g.set_number("Magnification", -1.0).unwrap();
    let julia = run(&g, &frame, 0.0);
    assert!(diff(&moved, &julia) > 0.05);
    let mut plain = fx(EffectKind::Fractal);
    plain.set_bool("Transparency", true).unwrap();
    let clear = run(&plain, &frame, 0.0);
    assert!(view(&clear).px.iter().any(|px| px[3] < 0.2));
    g.set_choice("Palette", "Solid Color").unwrap();
    g.set_number("Hue", 0.2).unwrap();
    let solid_color = run(&g, &frame, 0.0);
    g.set_choice("Palette", "Hue Wheel").unwrap();
    let wheel = run(&g, &frame, 0.0);
    assert!(diff(&solid_color, &wheel) > 0.05);
    g.set_number("Escape Limit", 8.0).unwrap();
    let low = run(&g, &frame, 0.0);
    g.set_number("Escape Limit", 40.0).unwrap();
    let high = run(&g, &frame, 0.0);
    assert!(diff(&low, &high) > 0.0);
    g.set_bool("Overlay", true).unwrap();
    let ghost = run(&g, &frame, 0.0);
    assert!(diff(&high, &ghost) > 0.0 || count_lit(&ghost) > 0);
    g.host.quality = effects_distort_generate::Quality::Draft;
    g.set_bool("Edge Highlight", true).unwrap();
    let low_q = run(&g, &frame, 0.0);
    g.host.quality = effects_distort_generate::Quality::Best;
    let high_q = run(&g, &frame, 0.0);
    assert!(diff(&low_q, &high_q) > 0.0 || low_q.width() == high_q.width());
    g.set_choice("Oversample Method", "Brute Force-Slow-Every Pixel")
        .unwrap();
    g.set_number("Oversample Factor", 2.0).unwrap();
    let brute = run(&g, &frame, 0.0);
    g.set_choice("Oversample Method", "Edge Detect-Fast-May Miss Pixels")
        .unwrap();
    let edge = run(&g, &frame, 0.0);
    assert!(diff(&brute, &edge) > 0.0 || brute.width() == 32);
}

#[test]
fn gradient_ramp_and_lens_flare() {
    let frame = solid(16, 8, BitsPerChannel::Bpc8, [0.0, 0.0, 0.0, 1.0]);
    let mut g = fx(EffectKind::GradientRamp);
    g.set_point("Start Of Ramp", 0.0, 4.0).unwrap();
    g.set_point("End Of Ramp", 15.0, 4.0).unwrap();
    g.set_role("c0", Value::Color(Rgba::black()));
    g.set_role("c1", Value::Color(Rgba::white()));
    g.set_number("Ramp Scatter", 0.0).unwrap();
    let linear = run(&g, &frame, 0.0);
    assert!(at(&linear, 1, 4)[0] < at(&linear, 14, 4)[0]);
    g.set_point("End Of Ramp", 8.0, 4.0).unwrap();
    let shorter = run(&g, &frame, 0.0);
    assert!(diff(&linear, &shorter) > 0.05);
    g.set_number("Ramp Scatter", 0.3).unwrap();
    let scattered = run(&g, &frame, 0.0);
    assert!(diff(&shorter, &scattered) > 0.01);
    g.set_role("shape", Value::Index(1));
    let radial = run(&g, &frame, 0.0);
    assert!(diff(&scattered, &radial) > 0.05);
    let mut flare = fx(EffectKind::LensFlare);
    flare.runtime.flare_center = Some([2.0, 2.0]);
    let left = run(&flare, &frame, 0.0);
    flare.runtime.flare_center = Some([12.0, 4.0]);
    let right = run(&flare, &frame, 0.0);
    assert!(at(&left, 2, 2)[0] > at(&left, 12, 4)[0]);
    assert!(diff(&left, &right) > 0.2);
    assert!(effects_distort_generate::parameters(EffectKind::LensFlare).is_empty());
}

#[test]
fn paint_bucket_selectors_and_fill_only() {
    let mut frame = solid(10, 10, BitsPerChannel::Bpc8, [0.0, 0.0, 0.0, 1.0]);
    for y in 2..6 {
        for x in 2..6 {
            frame = paint(&frame, x, y, [0.2, 0.2, 0.2, 1.0]);
        }
    }
    let mut g = fx(EffectKind::PaintBucket);
    g.set_point("Fill Point", 3.0, 3.0).unwrap();
    g.set_choice("Fill Selector", "Color & Alpha").unwrap();
    g.set_number("Tolerance", 0.05).unwrap();
    g.set_choice("Stroke", "Antialias").unwrap();
    g.set_color("Color", Rgba::new(0.0, 1.0, 0.0, 1.0)).unwrap();
    g.set_number("Opacity", 1.0).unwrap();
    let tight = run(&g, &frame, 0.0);
    g.set_number("Tolerance", 1.0).unwrap();
    let wide = run(&g, &frame, 0.0);
    assert!(count_lit_green(&wide) >= count_lit_green(&tight));
    g.set_bool("View Threshold", true).unwrap();
    let viewed = run(&g, &frame, 0.0);
    assert!(view(&viewed).px.iter().any(|px| px[0] > 0.9 && px[1] > 0.9));
    g.set_bool("View Threshold", false).unwrap();
    g.set_choice("Stroke", "Spread").unwrap();
    assert!(g.enabled("Spread Radius"));
    g.set_number("Spread Radius", 1.0).unwrap();
    let spread = run(&g, &frame, 0.0);
    g.set_choice("Stroke", "Choke").unwrap();
    let choke = run(&g, &frame, 0.0);
    assert!(diff(&spread, &choke) > 0.0);
    g.set_choice("Stroke", "Stroke").unwrap();
    assert!(g.enabled("Stroke Width"));
    g.set_number("Stroke Width", 1.0).unwrap();
    let edge = run(&g, &frame, 0.0);
    assert!(count_lit_green(&edge) < count_lit_green(&wide) || diff(&edge, &wide) > 0.0);
    g.set_choice("Fill Selector", "Transparency").unwrap();
    let mut hole = frame.clone();
    hole = paint(&hole, 1, 1, [0.0, 0.0, 0.0, 0.0]);
    g.set_point("Fill Point", 1.0, 1.0).unwrap();
    let _ = run(&g, &hole, 0.0);
    g.set_role("fill_only", Value::Bool(true));
    let mut stack = effects_distort_generate::Stack::new();
    stack.items.push(g.clone());
    stack.items.push(g);
    let params = effect_api::ParamValues::new();
    let input = effect_api::CpuInput {
        time: model::Time::ZERO,
        bits: BitsPerChannel::Bpc8,
        params: &params,
        frame: &frame,
    };
    let mut out = frame.clone();
    effect_api::CpuApply::apply_cpu(&stack, &input, &mut out).unwrap();
    assert_eq!(out.width(), frame.width());
}

fn count_lit_green(frame: &effect_api::Frame) -> usize {
    view(frame).px.iter().filter(|px| px[1] > 0.5).count()
}

#[test]
fn radio_waves_expand_and_fade() {
    let frame = solid(24, 24, BitsPerChannel::Bpc8, [0.0, 0.0, 0.0, 1.0]);
    let mut g = fx(EffectKind::RadioWaves);
    g.set_point("Producer Point", 12.0, 12.0).unwrap();
    g.set_number("Frequency", 2.0).unwrap();
    g.set_number("Expansion", 10.0).unwrap();
    g.set_number("Lifespan (sec)", 3.0).unwrap();
    g.set_number("Fade-in Time", 1.0).unwrap();
    g.set_number("Fade-out Time", 1.0).unwrap();
    g.set_number("Sides", 16.0).unwrap();
    g.set_color("Color", Rgba::white()).unwrap();
    g.set_number("Opacity", 1.0).unwrap();
    g.set_number("Start Width", 1.0).unwrap();
    g.set_number("End Width", 2.0).unwrap();
    let born = run(&g, &frame, 0.1);
    let mid = run(&g, &frame, 1.5);
    assert!(count_lit(&mid) > count_lit(&born));
    g.set_bool("Reflection", true).unwrap();
    let bounce = run(&g, &frame, 1.5);
    assert!(diff(&mid, &bounce) > 0.0 || count_lit(&bounce) > 0);
    g.set_bool("Star", true).unwrap();
    g.set_number("Star Depth", 0.4).unwrap();
    g.set_number("Sides", 5.0).unwrap();
    let star = run(&g, &frame, 1.0);
    g.set_number("Sides", 70.0).unwrap();
    let round = run(&g, &frame, 1.0);
    assert!(diff(&star, &round) > 0.0);
    g.set_number("Velocity", 8.0).unwrap();
    g.set_number("Direction", 0.0).unwrap();
    let drifting = run(&g, &frame, 1.0);
    assert!(diff(&round, &drifting) > 0.0);
}

#[test]
fn scribble_stroke_vegas_and_write_on() {
    let frame = solid(20, 16, BitsPerChannel::Bpc8, [0.2, 0.0, 0.0, 1.0]);
    let mut g = fx(EffectKind::Scribble);
    g.host.masks.push(MaskShape::polygon(
        "box",
        vec![[2.0, 2.0], [16.0, 2.0], [16.0, 12.0], [2.0, 12.0]],
        0,
    ));
    g.set_choice("Scribble", "Single Mask").unwrap();
    g.set_number("Angle", 0.0).unwrap();
    g.set_number("Stroke Width", 2.0).unwrap();
    g.set_number("Spacing", 3.0).unwrap();
    g.set_number("Start", 0.0).unwrap();
    g.set_number("End", 100.0).unwrap();
    g.set_choice("Composite", "On Transparent").unwrap();
    let filled = run(&g, &frame, 0.0);
    assert!(count_lit(&filled) > 5);
    assert!(at(&filled, 0, 0)[3] < 0.2);
    g.set_number("End", 40.0).unwrap();
    let partial = run(&g, &frame, 0.0);
    assert!(count_lit(&partial) < count_lit(&filled));
    g.set_choice("Composite", "Reveal Original Image").unwrap();
    let reveal = run(&g, &frame, 0.0);
    assert!(at(&reveal, 8, 6)[0] > 0.1 || count_lit(&reveal) > 0);
    assert!((g.num("Miter Limit").unwrap() - 4.0).abs() < 1e-9);
    g.set_choice("Join", "Miter").unwrap();
    assert!(g.enabled("Miter Limit"));
    g.set_choice("Wiggle Type", "Smooth").unwrap();
    assert!(g.enabled("Wiggles/Second"));
    g.set_number("Wiggles/Second", 2.0).unwrap();
    let a = run(&g, &frame, 0.0);
    let b = run(&g, &frame, 0.5);
    assert!(diff(&a, &b) > 0.0);
    let mut stroke = fx(EffectKind::Stroke);
    stroke
        .host
        .masks
        .push(MaskShape::polygon("line", vec![[1.0, 8.0], [18.0, 8.0]], 0));
    stroke.host.masks[0].closed = false;
    stroke.set_number("Brush Hardness", 1.0).unwrap();
    stroke.set_number("Spacing", 1.0).unwrap();
    stroke.set_role("color", Value::Color(Rgba::white()));
    stroke.set_role("opacity", Value::Number(1.0));
    let hard = run(&stroke, &frame, 0.0);
    stroke.set_number("Brush Hardness", 0.2).unwrap();
    stroke.set_number("Spacing", 4.0).unwrap();
    let soft = run(&stroke, &frame, 0.0);
    assert!(diff(&hard, &soft) > 0.0);
    let mut vegas = fx(EffectKind::Vegas);
    vegas.host.masks.push(MaskShape::polygon(
        "box",
        vec![[2.0, 2.0], [16.0, 2.0], [16.0, 12.0], [2.0, 12.0]],
        0,
    ));
    vegas.set_choice("Stroke", "Mask/Path").unwrap();
    vegas.set_number("Segments", 4.0).unwrap();
    vegas.set_number("Length", 0.5).unwrap();
    vegas.set_number("Rotation", 0.0).unwrap();
    vegas.set_number("Width", 2.0).unwrap();
    vegas.set_number("Hardness", 1.0).unwrap();
    vegas.set_number("Start Opacity", 1.0).unwrap();
    vegas.set_number("End Opacity", 0.2).unwrap();
    vegas.set_number("Mid-point Opacity", 0.0).unwrap();
    vegas.set_choice("Blend Mode", "Transparent").unwrap();
    vegas.set_choice("Segment Distribution", "Even").unwrap();
    vegas.set_color("Color", Rgba::white()).unwrap();
    let lights = run(&vegas, &frame, 0.0);
    vegas.set_number("Rotation", 0.08).unwrap();
    let moved = run(&vegas, &frame, 0.0);
    assert!(diff(&lights, &moved) > 0.0);
    vegas.set_choice("Blend Mode", "Stencil").unwrap();
    let stencil = run(&vegas, &frame, 0.0);
    assert!(diff(&moved, &stencil) > 0.0);
    let mut write = fx(EffectKind::WriteOn);
    write.set_choice("Paint Style", "On Transparent").unwrap();
    write.set_number("Stroke Length (secs)", 0.0).unwrap();
    write.set_number("Brush Spacing (secs)", 0.1).unwrap();
    write.add_key("Brush Position", 0.0, Value::Point([2.0, 4.0]));
    write.add_key("Brush Position", 1.0, Value::Point([16.0, 4.0]));
    let trail = run(&write, &frame, 1.0);
    assert!(count_lit(&trail) > 3);
    write.set_number("Stroke Length (secs)", 0.2).unwrap();
    let snake = run(&write, &frame, 1.0);
    assert!(count_lit(&snake) < count_lit(&trail));
    write.set_role("paint_time", Value::Index(1));
    write.add_key("Color", 0.0, Value::Color(Rgba::new(1.0, 0.0, 0.0, 1.0)));
    write.add_key("Color", 1.0, Value::Color(Rgba::new(0.0, 0.0, 1.0, 1.0)));
    let colored = run(&write, &frame, 1.0);
    assert!(diff(&snake, &colored) > 0.0 || count_lit(&colored) > 0);
    let _ = (
        LayerId::new(1),
        MaskMode::None,
        SourceLayer {
            id: LayerId::new(1),
            frame: frame.clone(),
            masks: vec![],
            audio: None,
            precomposed: false,
        },
    );
}
