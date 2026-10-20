use effect_api::Frame32;

use crate::channel::{present_channels, ChannelView, ViewerQuality};
use crate::composite::{composite_export, preview_composite, CompositeRequest};
use crate::guide::{GuideAppearance, ViewGuides};
use crate::resolution::{CompResolution, ResolutionError};
use crate::zoom::{FrameSize, PanelSize, ZoomState};

/// 一つのコンポジションビュー。ガイドとチャンネル表示は書き出しに入らない。
#[derive(Clone, Debug)]
pub struct CompositionViewer {
    pub resolution: CompResolution,
    pub zoom: ZoomState,
    pub guides: ViewGuides,
    pub channels: ChannelView,
    pub quality: ViewerQuality,
}

impl CompositionViewer {
    pub fn new(frame: FrameSize, panel: PanelSize, guides: GuideAppearance) -> Self {
        Self {
            resolution: CompResolution::full(),
            zoom: ZoomState::fitted(frame, panel),
            guides: ViewGuides::new(guides),
            channels: ChannelView::default(),
            quality: ViewerQuality::default(),
        }
    }

    pub fn effective_quality(&self) -> ViewerQuality {
        self.channels.effective_quality(self.quality)
    }

    /// `other_zooms` は同じパネルの他ビュー。Auto は最も拡大されているビューに合わせる。
    pub fn preview(
        &self,
        request: &CompositeRequest<'_>,
        other_zooms: &[f64],
    ) -> Result<Frame32, ResolutionError> {
        let mut zooms = Vec::with_capacity(other_zooms.len() + 1);
        zooms.push(self.zoom.ratio);
        zooms.extend_from_slice(other_zooms);
        let image = preview_composite(request, &self.resolution, &zooms)?;
        Ok(present_channels(self.channels, &image))
    }

    pub fn export(&self, request: &CompositeRequest<'_>) -> Frame32 {
        composite_export(request)
    }
}

#[cfg(test)]
mod tests {
    use super::CompositionViewer;
    use crate::channel::ChannelDisplay;
    use crate::composite::{CompositeRequest, LayerPlate};
    use crate::guide::{overlay_guides, GuideAppearance, GuideStyle, RulerEdge};
    use crate::resolution::Resolution;
    use crate::zoom::{FrameSize, PanelSize};
    use effect_api::Frame32;
    use model::{BitsPerChannel, Rgba};

    #[test]
    fn export_ignores_zoom_channels_resolution_and_guides() {
        let image = {
            let mut frame = Frame32::new(8, 4, 4, 0.0);
            for chunk in frame.samples.chunks_mut(4) {
                chunk.copy_from_slice(&[1.0, 0.0, 0.0, 1.0]);
            }
            frame
        };
        let layer = LayerPlate::new(&image);
        let request = CompositeRequest {
            width: 8,
            height: 4,
            bits: BitsPerChannel::Bpc8,
            background: Rgba::transparent(),
            layers: &[layer],
        };
        let mut viewer = CompositionViewer::new(
            FrameSize {
                width: 8.0,
                height: 4.0,
                pixel_aspect: 1.0,
            },
            PanelSize {
                width: 8.0,
                height: 4.0,
            },
            GuideAppearance {
                color: Rgba::new(0.0, 1.0, 0.0, 1.0),
                style: GuideStyle::Lines,
            },
        );
        viewer.zoom.ratio = 0.25;
        viewer.resolution.value = Resolution::Auto;
        viewer.channels.display = ChannelDisplay::Red;
        viewer.guides.push_from_ruler(RulerEdge::Top, 0.0);
        let exported = viewer.export(&request);
        assert_eq!((exported.width, exported.height), (8, 4));
        assert_eq!(exported.samples, image.samples);
        let over = overlay_guides(&exported, &viewer.guides);
        assert_ne!(over.samples, exported.samples);
        let preview = viewer.preview(&request, &[]).unwrap();
        assert_eq!((preview.width, preview.height), (2, 1));
        assert_ne!(preview.samples, exported.samples);
    }
}
