//! プレビュー範囲、レンダー設定、出力モジュール。
//! H.264 と ProRes の符号化は FFmpeg に渡す想定で、このクレートは設定だけを持つ。

#![forbid(unsafe_code)]

pub mod compose;
pub mod output;
pub mod preview;
pub mod queue;
pub mod settings;
pub mod video;

pub use compose::{compose, fit_proxy, raster_for_reference, LayerSwitches, PaintLayer};
pub use output::{
    output_size, reconcile_h264, sequence_name, sequence_numbers, unique_output_name, write_audio, write_frame,
    AudioSettings, Channels, OutputAlpha, OutputModule, MAX_SEQUENCE_FRAMES,
};
pub use preview::{
    alternate_preview, crop_comp_to_region, preview_fps, preview_pixels, resolve_range, set_work_area_end,
    set_work_area_start, set_work_area_to_span, skipped_frames, InterestRegion, PreviewKey, PreviewRequest, RangeMode,
};
pub use queue::{apply_post_render, pre_render, ProjectQueue, QueueStatus, RenderItem};
pub use settings::{frame_count, render_log, render_resolution, RenderSettings, RenderTemplate};
pub use video::{h264_level, h264_options, ProResVariant, MOVIE_DEFAULT_NAME};
