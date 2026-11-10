//! レイヤー合成とコンポジションビューア。
//!
//! 画素は CPU 参照実装で決める。テストは GPU デバイスもウィンドウも作らない。
//! Windows の wgpu バックエンドは DX12 だけを有効にする。

#![forbid(unsafe_code)]

mod blend;
mod cache;
mod channel;
mod composite;
mod device;
mod guide;
mod resolution;
mod viewer;
mod zoom;

pub use cache::{frame_index, CacheError, CachePresence, FrameCache, FrameKey};
pub use channel::{
    channel_from_shortcut, present_channels, ChannelDisplay, ChannelView, ViewerQuality,
};
pub use composite::{composite, composite_export, preview_composite, CompositeRequest, LayerPlate};
pub use device::{instance_descriptor, preferred_backends};
pub use guide::{
    export_json, import_json, overlay_guides, snap_edge, Edge, Guide, GuideAppearance,
    GuideFileError, GuideStyle, GuideUnit, Orientation, RulerEdge, ViewGuides,
};
pub use resolution::{export_size, CompResolution, PreviewLabel, Resolution, ResolutionError};
pub use viewer::CompositionViewer;
pub use zoom::{
    fit_ratio, fit_up_to_100, CompPanel, FrameSize, PanelRect, PanelSize, ZoomFocus, ZoomMode,
    ZoomState, ZoomTool, DOCUMENTED_FIXED_STEPS, DOCUMENTED_UPPER_STEPS,
};
