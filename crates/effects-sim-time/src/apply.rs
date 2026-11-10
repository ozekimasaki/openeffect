use crate::bag::{finale_audio, finale_image, Bag};
use crate::catalog::{effect, EffectId};
use crate::cx::{Exposed, RenderCx};
use crate::image::{image_from_frame, image_to_frame, Audio, Image};
use crate::paint::{exposed_of, paint};
use crate::values::EffectValues;
use effect_api::{ApplyResult, CpuInput, Frame};

/// CPU で適用した結果。
#[derive(Clone, Debug, PartialEq)]
pub struct Rendered {
    pub image: Image,
    pub audio: Option<Audio>,
    pub exposed: Option<Exposed>,
}

pub fn render(
    id: EffectId,
    src: &Image,
    values: &EffectValues,
    cx: &RenderCx<'_>,
) -> ApplyResult<Rendered> {
    let effect = effect(id).ok_or_else(|| effect_api::ApplyError::new("未知の効果"))?;
    let exposed = exposed_of(effect, values);
    let bag = Bag::from_values(effect, values);
    let mut working = bag.clone();
    let painted = paint(effect, src, exposed, &mut working, cx);
    let image = if painted.skip_image_finale {
        painted.image
    } else {
        finale_image(&painted.image, &bag, cx.layers)
    };
    let audio = match painted.audio {
        Some(audio) if !painted.skip_audio_finale => Some(finale_audio(&audio, &bag)),
        other => other,
    };
    Ok(Rendered {
        image,
        audio,
        exposed: painted.exposed,
    })
}

pub fn render_frame(
    id: EffectId,
    input: &CpuInput<'_>,
    values: &EffectValues,
    output: &mut Frame,
) -> ApplyResult<()> {
    let src = image_from_frame(input.frame);
    let cx = RenderCx {
        time: input.time.seconds(),
        frame_rate: 24.0,
        bits: input.bits,
        layers: &[],
        history: &[],
        audio: None,
        depth: None,
        object_id: None,
        material_id: None,
        accum: None,
        lut: None,
        clear: false,
    };
    let rendered = render(id, &src, values, &cx)?;
    *output = image_to_frame(&rendered.image, input.bits);
    Ok(())
}
