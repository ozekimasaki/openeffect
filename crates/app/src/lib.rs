//! ヘッドレスの結合。
//!
//! コンポジションを作り、1 フレーム評価し、`.aep` と `.aepx` を往復し、
//! GPU クレートの CPU 参照実装で合成する。ウィンドウは開かない。

#![forbid(unsafe_code)]

use effect_api::Frame32;
use eval::render::{render, Image, RenderPurpose};
use gpu::{composite, CompositeRequest, LayerPlate};
use io_aep::{read_aep, read_aepx, write_aep, write_aepx, Chunk, Document, FourCc};
use model::{
    BitsPerChannel, Composition, Footage, FootageKind, Layer, LayerKind, Project, Rgba, Time,
};

/// 4×2、24 fps、1 秒のコンポジションに赤い平面を 1 枚置く。
pub fn small_composition() -> (Project, model::ItemId) {
    let mut project = Project::new(BitsPerChannel::Bpc8);
    let footage_id = project.alloc_item();
    let mut footage = Footage::file(footage_id, "Red Solid", 4, 2, Time::ZERO, 24.0);
    footage.kind = FootageKind::Solid {
        color: Rgba::new(1.0, 0.0, 0.0, 1.0),
    };
    project.add_footage(footage);

    let comp_id = project.alloc_item();
    let mut composition = Composition::new(comp_id, "Small", 4, 2, 24.0, Time::from_seconds(1.0));
    let layer_id = project.alloc_layer();
    let mut layer = Layer::new(layer_id, "Red Solid", LayerKind::Solid { source: footage_id });
    layer.in_point = Time::ZERO;
    layer.out_point = composition.duration;
    composition.layers.push(layer);
    project.add_composition(composition);
    (project, comp_id)
}

/// 時刻 0 の 1 フレーム。
pub fn evaluate_frame(project: &Project, comp: model::ItemId) -> Option<Image> {
    render(project, comp, Time::ZERO, RenderPurpose::Preview)
}

/// コンポジションの幅、高さ、フレームレート、名前をチャンクにして `.aep` と `.aepx` を往復する。
pub fn roundtrip_project_files(composition: &Composition) -> Document {
    let document = document_for(composition);
    let aep = write_aep(&document).expect("write .aep");
    let from_aep = read_aep(&aep).expect("read .aep");
    assert_eq!(from_aep, document);
    assert_eq!(write_aep(&from_aep).expect("rewrite .aep"), aep);

    let aepx = write_aepx(&document).expect("write .aepx");
    let from_aepx = read_aepx(&aepx).expect("read .aepx");
    assert_eq!(from_aepx, document);
    assert_eq!(write_aepx(&from_aepx).expect("rewrite .aepx"), aepx);
    from_aep
}

/// 評価済みフレームを CPU 参照実装で合成する。デバイスもウィンドウも作らない。
pub fn composite_on_cpu(image: &Image, bits: BitsPerChannel) -> Frame32 {
    let plate = frame_from_image(image);
    let layer = LayerPlate::new(&plate);
    composite(&CompositeRequest {
        width: image.width,
        height: image.height,
        bits,
        background: Rgba::black(),
        layers: &[layer],
    })
}

fn document_for(composition: &Composition) -> Document {
    let mut idta = vec![0_u8; 64];
    idta[0..2].copy_from_slice(&io_aep::fields::idta::COMPOSITION.to_be_bytes());
    let item_id = u32::try_from(composition.id.get()).unwrap_or(u32::MAX);
    idta[16..20].copy_from_slice(&item_id.to_be_bytes());

    let mut cdta = vec![0_u8; 208];
    let width = u16::try_from(composition.width).unwrap_or(u16::MAX);
    let height = u16::try_from(composition.height).unwrap_or(u16::MAX);
    let rate = u16::try_from(composition.frame_rate.round() as u32).unwrap_or(u16::MAX);
    cdta[io_aep::fields::cdta::WIDTH..io_aep::fields::cdta::WIDTH + 2]
        .copy_from_slice(&width.to_be_bytes());
    cdta[io_aep::fields::cdta::HEIGHT..io_aep::fields::cdta::HEIGHT + 2]
        .copy_from_slice(&height.to_be_bytes());
    cdta[io_aep::fields::cdta::FRAME_RATE..io_aep::fields::cdta::FRAME_RATE + 2]
        .copy_from_slice(&rate.to_be_bytes());

    Document {
        chunks: vec![
            Chunk::data(FourCc::new(b"idta"), idta),
            Chunk::data(FourCc::new(b"cdta"), cdta),
            Chunk::data(FourCc::new(b"Utf8"), composition.name.as_bytes()),
        ],
    }
}

fn frame_from_image(image: &Image) -> Frame32 {
    let mut frame = Frame32::new(image.width, image.height, 4, 0.0);
    for (pixel, chunk) in image.pixels.iter().zip(frame.samples.chunks_mut(4)) {
        chunk.copy_from_slice(&[pixel.r, pixel.g, pixel.b, pixel.a]);
    }
    frame
}

#[cfg(test)]
mod tests {
    use super::{composite_on_cpu, evaluate_frame, roundtrip_project_files, small_composition};
    use io_aep::fields::{cdta, idta, utf8};
    use io_aep::{Chunk, Document};

    #[test]
    fn small_comp_evaluates_one_frame_roundtrips_and_composites_on_cpu() {
        let (project, comp_id) = small_composition();
        let composition = project.composition(comp_id).expect("comp");
        assert_eq!((composition.width, composition.height), (4, 2));
        assert_eq!(composition.layers.len(), 1);

        let image = evaluate_frame(&project, comp_id).expect("frame");
        assert_eq!((image.width, image.height), (4, 2));
        assert_eq!(image.pixels.len(), 8);
        assert!(image
            .pixels
            .iter()
            .all(|pixel| *pixel == model::Rgba::new(1.0, 0.0, 0.0, 1.0)));

        let document = roundtrip_project_files(composition);
        let idta_bytes = data_payload(&document, b"idta");
        let cdta_bytes = data_payload(&document, b"cdta");
        let name = data_payload(&document, b"Utf8");
        assert_eq!(idta::kind(&idta_bytes), Some(idta::COMPOSITION));
        assert_eq!(cdta::width(&cdta_bytes), Some(4));
        assert_eq!(cdta::height(&cdta_bytes), Some(2));
        assert_eq!(cdta::frame_rate(&cdta_bytes), Some(24));
        assert_eq!(utf8::as_str(&name), Some("Small"));

        let composited = composite_on_cpu(&image, project.bits_per_channel);
        assert_eq!((composited.width, composited.height, composited.channels), (4, 2, 4));
        for chunk in composited.samples.chunks(4) {
            assert_eq!(chunk, &[1.0, 0.0, 0.0, 1.0]);
        }
    }

    fn data_payload(document: &Document, id: &[u8; 4]) -> Vec<u8> {
        for chunk in &document.chunks {
            if let Chunk::Data { id: tag, data, .. } = chunk {
                if tag.0 == *id {
                    return data.clone();
                }
            }
        }
        panic!("missing chunk {}", std::str::from_utf8(id).unwrap_or("????"));
    }
}
