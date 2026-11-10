mod distort;
mod generate;
mod perspective;

use effect_api::ApplyResult;

use crate::host::Instance;
use crate::image::{Filter, Image};
use crate::spec::EffectKind;

pub use distort::{polar_pair, wave_dir, wave_sample};

pub fn render(inst: &Instance, src: &Image, time: f64) -> ApplyResult<Image> {
    match inst.kind {
        EffectKind::BezierWarp => distort::bezier(inst, src),
        EffectKind::Bulge => distort::bulge(inst, src),
        EffectKind::CcBendIt
        | EffectKind::CcBender
        | EffectKind::CcBlobbylize
        | EffectKind::CcFloMotion
        | EffectKind::CcGriddler
        | EffectKind::CcLens
        | EffectKind::CcPageTurn
        | EffectKind::CcPowerPin
        | EffectKind::CcRipplePulse
        | EffectKind::CcSlant
        | EffectKind::CcSmear
        | EffectKind::CcSplit
        | EffectKind::CcSplit2
        | EffectKind::CcTiler => Ok(src.clone()),
        EffectKind::CornerPin => distort::corner_pin(inst, src),
        EffectKind::DetailPreservingUpscale => distort::upscale(inst, src),
        EffectKind::DisplacementMap => distort::displacement(inst, src),
        EffectKind::Liquify => distort::liquify(inst, src),
        EffectKind::Magnify => distort::magnify(inst, src),
        EffectKind::MeshWarp => distort::mesh(inst, src),
        EffectKind::Mirror => distort::mirror(inst, src),
        EffectKind::Offset => distort::offset(inst, src),
        EffectKind::OpticsCompensation => distort::optics(inst, src),
        EffectKind::PolarCoordinates => distort::polar(inst, src),
        EffectKind::Reshape => distort::reshape(inst, src, time),
        EffectKind::Ripple => distort::ripple(inst, src, time),
        EffectKind::RollingShutterRepair => distort::rolling(inst, src),
        EffectKind::Smear => distort::smear(inst, src, time),
        EffectKind::Spherize => distort::spherize(inst, src),
        EffectKind::Transform => distort::transform(inst, src, time),
        EffectKind::TurbulentDisplace => distort::turbulent(inst, src, time),
        EffectKind::Twirl => distort::twirl(inst, src),
        EffectKind::Warp => distort::warp_style(inst, src),
        EffectKind::WarpStabilizer => distort::stabilizer(inst, src, time),
        EffectKind::WaveWarp => distort::wave(inst, src, time),
        EffectKind::FourColorGradient => generate::four_color(inst, src),
        EffectKind::AdvancedLightning => generate::lightning(inst, src),
        EffectKind::AudioSpectrum => generate::spectrum(inst, src, time),
        EffectKind::AudioWaveform => generate::waveform(inst, src, time),
        EffectKind::Beam => generate::beam(inst, src),
        EffectKind::CcGlueGun
        | EffectKind::CcLightBurst
        | EffectKind::CcLightRays
        | EffectKind::CcLightSweep
        | EffectKind::CcThreads => Ok(src.clone()),
        EffectKind::CellPattern => generate::cells(inst, src, time),
        EffectKind::Checkerboard => generate::checker(inst, src),
        EffectKind::Circle => generate::circle(inst, src),
        EffectKind::Ellipse => generate::ellipse(src),
        EffectKind::EyedropperFill => generate::eyedropper(inst, src),
        EffectKind::Fill => generate::fill(inst, src),
        EffectKind::Fractal => generate::fractal(inst, src),
        EffectKind::GradientRamp => generate::ramp(inst, src),
        EffectKind::Grid => generate::grid(inst, src),
        EffectKind::LensFlare => generate::flare(inst, src),
        EffectKind::PaintBucket => generate::bucket(inst, src),
        EffectKind::RadioWaves => generate::radio(inst, src, time),
        EffectKind::Scribble => generate::scribble(inst, src, time),
        EffectKind::Stroke => generate::stroke(inst, src),
        EffectKind::Vegas => generate::vegas(inst, src, time),
        EffectKind::WriteOn => generate::write_on(inst, src, time),
        EffectKind::CameraTracker => perspective::tracker(inst, src),
        EffectKind::Glasses3d => perspective::glasses(inst, src),
        EffectKind::BevelAlpha => perspective::bevel(src, true),
        EffectKind::BevelEdges => perspective::bevel(src, false),
        EffectKind::CcCylinder
        | EffectKind::CcEnvironment
        | EffectKind::CcSphere
        | EffectKind::CcSpotlight => Ok(src.clone()),
        EffectKind::DropShadow => perspective::drop_shadow(inst, src),
        EffectKind::RadialShadow => perspective::radial_shadow(inst, src),
    }
}

pub(super) fn sample_at(src: &Image, x: f32, y: f32, filter: Filter, wrap: bool) -> [f32; 4] {
    if !x.is_finite() || !y.is_finite() {
        return [0.0; 4];
    }
    src.sample(x + 0.5, y + 0.5, filter, wrap)
}

pub(super) fn map_px(src: &Image, mut f: impl FnMut(u32, u32) -> [f32; 4]) -> Image {
    let mut out = src.clone();
    for y in 0..src.height {
        for x in 0..src.width {
            let i = (y * src.width + x) as usize;
            out.px[i] = f(x, y);
        }
    }
    out
}

pub(super) fn blank_like(src: &Image, w: u32, h: u32) -> Image {
    Image::new(w, h, src.channels, src.bits, [0.0; 4])
}
