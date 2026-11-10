use io_api::FootageReference;
use io_media::Raster;
use model::BitsPerChannel;

use crate::settings::{EffectsMode, FieldRender, FrameBlendMode, GuideMode, MotionBlurMode, RenderQuality, SoloMode};

#[derive(Clone, Debug, PartialEq)]
pub struct LayerSwitches {
    pub quality: RenderQuality,
    pub effects: bool,
    pub solo: bool,
    pub frame_blend: bool,
    pub motion_blur: bool,
    pub guide: bool,
    pub nested: bool,
}

#[derive(Clone, Debug)]
pub struct PaintLayer {
    pub color: [f32; 4],
    pub effect_gain: f32,
    pub blend_with: Option<[f32; 4]>,
    pub blur_shift: [f32; 4],
    pub switches: LayerSwitches,
}

pub fn apply_quality(raster: &Raster, quality: RenderQuality) -> Raster {
    match quality {
        RenderQuality::Best | RenderQuality::Wireframe => raster.clone(),
        RenderQuality::Draft => {
            let mut averaged = raster.clone();
            for y in 0..raster.height {
                for x in 0..raster.width {
                    let right = raster.get((x + 1).min(raster.width - 1), y);
                    let down = raster.get(x, (y + 1).min(raster.height - 1));
                    let here = raster.get(x, y);
                    let mut pixel = [0.0; 4];
                    for channel in 0..4 {
                        pixel[channel] = (here[channel] + right[channel] + down[channel]) / 3.0;
                    }
                    averaged.set(x, y, pixel);
                }
            }
            averaged
        }
    }
}

pub fn compose(layers: &[PaintLayer], effects: EffectsMode, solo: SoloMode, guides: GuideMode, blend: FrameBlendMode, blur: MotionBlurMode, fields: FieldRender) -> Raster {
    let mut image = Raster::solid(2, 2, BitsPerChannel::Bpc8, [0.0, 0.0, 0.0, 0.0]);
    let any_solo = layers.iter().any(|layer| layer.switches.solo);
    for layer in layers {
        if !guide_visible(layer, guides) {
            continue;
        }
        if any_solo && matches!(solo, SoloMode::CurrentSettings) && !layer.switches.solo {
            continue;
        }
        let mut color = layer.color;
        let effects_on = match effects {
            EffectsMode::AllOff => false,
            EffectsMode::AllOn => true,
            EffectsMode::CurrentSettings => layer.switches.effects,
        };
        if effects_on {
            color[0] = (color[0] + layer.effect_gain).min(1.0);
        }
        let blur_on = match blur {
            MotionBlurMode::OffForAllLayers => false,
            MotionBlurMode::OnForCheckedLayers | MotionBlurMode::CurrentSettings => layer.switches.motion_blur,
        };
        if blur_on {
            for channel in 0..3 {
                color[channel] = (color[channel] + layer.blur_shift[channel]) * 0.5;
            }
        }
        let blend_on = match blend {
            FrameBlendMode::Off => false,
            FrameBlendMode::OnForCheckedLayers => layer.switches.frame_blend,
        };
        if blend_on {
            if let Some(other) = layer.blend_with {
                for channel in 0..4 {
                    color[channel] = (color[channel] + other[channel]) * 0.5;
                }
            }
        }
        image = Raster::solid(2, 2, BitsPerChannel::Bpc8, color);
    }
    if matches!(fields, FieldRender::LowerFirst | FieldRender::UpperFirst) {
        let top = image.get(0, 0);
        let mut combed = image.clone();
        combed.set(0, 1, [1.0 - top[0], 1.0 - top[1], 1.0 - top[2], top[3]]);
        combed.set(1, 1, [1.0 - top[0], 1.0 - top[1], 1.0 - top[2], top[3]]);
        combed
    } else {
        image
    }
}

fn guide_visible(layer: &PaintLayer, guides: GuideMode) -> bool {
    if !layer.switches.guide {
        return true;
    }
    match guides {
        GuideMode::AllOff => false,
        GuideMode::CurrentSettings => !layer.switches.nested,
    }
}

pub fn fit_proxy(width: u32, height: u32, proxy: &Raster) -> Raster {
    let mut fitted = Raster::solid(width, height, proxy.bits, [0.0, 0.0, 0.0, 1.0]);
    for y in 0..height {
        for x in 0..width {
            let sx = (x as u64 * proxy.width as u64 / width.max(1) as u64) as u32;
            let sy = (y as u64 * proxy.height as u64 / height.max(1) as u64) as u32;
            fitted.set(x, y, proxy.get(sx.min(proxy.width - 1), sy.min(proxy.height - 1)));
        }
    }
    fitted
}

pub fn raster_for_reference(reference: &FootageReference, original: &Raster, proxy: Option<&Raster>) -> Raster {
    if let Some(proxy_raster) = proxy {
        if reference.proxy.as_ref().is_some_and(|item| item.in_use) {
            return fit_proxy(original.width, original.height, proxy_raster);
        }
    }
    original.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use io_api::Proxy;

    fn switches() -> LayerSwitches {
        LayerSwitches {
            quality: RenderQuality::Best,
            effects: true,
            solo: false,
            frame_blend: false,
            motion_blur: false,
            guide: false,
            nested: false,
        }
    }

    #[test]
    fn draft_is_coarser_and_leaves_switches() {
        let mut raster = Raster::solid(2, 1, BitsPerChannel::Bpc8, [0.0, 0.0, 0.0, 1.0]);
        raster.set(0, 0, [0.0, 0.0, 0.0, 1.0]);
        raster.set(1, 0, [1.0, 1.0, 1.0, 1.0]);
        let best = apply_quality(&raster, RenderQuality::Best);
        let draft = apply_quality(&raster, RenderQuality::Draft);
        let switches = switches();
        assert!((best.get(0, 0)[0] - draft.get(0, 0)[0]).abs() > 0.1);
        assert_eq!(switches.quality, RenderQuality::Best);
    }

    #[test]
    fn effects_solo_guides_blur_and_fields_follow_the_render_item() {
        let mut solo = switches();
        solo.solo = true;
        let mut other = switches();
        other.effects = true;
        let layers = [
            PaintLayer {
                color: [0.0, 0.0, 1.0, 1.0],
                effect_gain: 0.0,
                blend_with: None,
                blur_shift: [0.0; 4],
                switches: solo,
            },
            PaintLayer {
                color: [0.2, 0.0, 0.0, 1.0],
                effect_gain: 0.5,
                blend_with: Some([0.0, 1.0, 0.0, 1.0]),
                blur_shift: [0.0, 0.0, 1.0, 0.0],
                switches: other.clone(),
            },
        ];
        let all_off = compose(&layers, EffectsMode::AllOff, SoloMode::AllOff, GuideMode::AllOff, FrameBlendMode::Off, MotionBlurMode::OffForAllLayers, FieldRender::Off);
        assert!(all_off.get(0, 0)[0] < 0.3);
        let with_effect = compose(&layers, EffectsMode::AllOn, SoloMode::AllOff, GuideMode::AllOff, FrameBlendMode::Off, MotionBlurMode::OffForAllLayers, FieldRender::Off);
        assert!(with_effect.get(0, 0)[0] > all_off.get(0, 0)[0]);
        let solo_only = compose(&layers, EffectsMode::AllOff, SoloMode::CurrentSettings, GuideMode::AllOff, FrameBlendMode::Off, MotionBlurMode::OffForAllLayers, FieldRender::Off);
        assert!(solo_only.get(0, 0)[2] > 0.9);
        assert!(other.effects);

        let mut guide = switches();
        guide.guide = true;
        let mut nested = guide.clone();
        nested.nested = true;
        let guides = [
            PaintLayer { color: [0.0, 1.0, 0.0, 1.0], effect_gain: 0.0, blend_with: None, blur_shift: [0.0; 4], switches: guide },
            PaintLayer { color: [1.0, 0.0, 0.0, 1.0], effect_gain: 0.0, blend_with: None, blur_shift: [0.0; 4], switches: nested },
        ];
        let hidden = compose(&guides, EffectsMode::AllOff, SoloMode::AllOff, GuideMode::AllOff, FrameBlendMode::Off, MotionBlurMode::OffForAllLayers, FieldRender::Off);
        assert!(hidden.get(0, 0)[1] < 0.1);
        let top = compose(&guides[..1], EffectsMode::AllOff, SoloMode::AllOff, GuideMode::CurrentSettings, FrameBlendMode::Off, MotionBlurMode::OffForAllLayers, FieldRender::Off);
        assert!(top.get(0, 0)[1] > 0.9);
        let nested_hidden = compose(&guides[1..], EffectsMode::AllOff, SoloMode::AllOff, GuideMode::CurrentSettings, FrameBlendMode::Off, MotionBlurMode::OffForAllLayers, FieldRender::Off);
        assert!(nested_hidden.get(0, 0)[0] < 0.1);

        let mut blurred = switches();
        blurred.motion_blur = true;
        let blur_layer = [PaintLayer {
            color: [1.0, 0.0, 0.0, 1.0],
            effect_gain: 0.0,
            blend_with: Some([0.0, 0.0, 1.0, 1.0]),
            blur_shift: [0.0, 0.0, 1.0, 0.0],
            switches: blurred.clone(),
        }];
        let sharp = compose(&blur_layer, EffectsMode::AllOff, SoloMode::AllOff, GuideMode::AllOff, FrameBlendMode::Off, MotionBlurMode::OffForAllLayers, FieldRender::Off);
        let soft = compose(&blur_layer, EffectsMode::AllOff, SoloMode::AllOff, GuideMode::AllOff, FrameBlendMode::Off, MotionBlurMode::OnForCheckedLayers, FieldRender::Off);
        assert!(sharp.get(0, 0)[0] > soft.get(0, 0)[0]);
        let mut blending = blurred.clone();
        blending.frame_blend = true;
        blending.motion_blur = false;
        let blend_layer = [PaintLayer {
            switches: blending,
            ..blur_layer[0].clone()
        }];
        let mixed = compose(&blend_layer, EffectsMode::AllOff, SoloMode::AllOff, GuideMode::AllOff, FrameBlendMode::OnForCheckedLayers, MotionBlurMode::OffForAllLayers, FieldRender::Off);
        assert!(mixed.get(0, 0)[2] > 0.2);
        let progressive = compose(&blur_layer, EffectsMode::AllOff, SoloMode::AllOff, GuideMode::AllOff, FrameBlendMode::Off, MotionBlurMode::OffForAllLayers, FieldRender::Off);
        assert!((progressive.get(0, 0)[0] - progressive.get(0, 1)[0]).abs() < 1e-5);
        let interlaced = compose(&blur_layer, EffectsMode::AllOff, SoloMode::AllOff, GuideMode::AllOff, FrameBlendMode::Off, MotionBlurMode::OffForAllLayers, FieldRender::LowerFirst);
        assert!((interlaced.get(0, 0)[0] - interlaced.get(0, 1)[0]).abs() > 0.2);
    }

    #[test]
    fn current_proxy_settings_scale_to_the_original() {
        let mut reference = FootageReference::linked(model::ItemId::new(1), "full.png");
        reference.set_proxy(Proxy::new("proxy.png"));
        let original = Raster::solid(4, 2, BitsPerChannel::Bpc8, [1.0, 0.0, 0.0, 1.0]);
        let proxy = Raster::solid(2, 1, BitsPerChannel::Bpc8, [0.0, 0.0, 1.0, 1.0]);
        let shown = raster_for_reference(&reference, &original, Some(&proxy));
        assert_eq!((shown.width, shown.height), (4, 2));
        assert!(shown.get(0, 0)[2] > 0.9);
    }
}
