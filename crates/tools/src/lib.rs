//! トラッカー、ペイント、パペット、ロトブラシ、コンテントアウェア塗り。
//! 契約クレートの公開型はここでは変えない。

#![forbid(unsafe_code)]

pub mod camera;
mod error;
pub mod fill;
mod linalg;
mod match2d;
pub mod paint;
mod persist;
pub mod planar;
pub mod point_track;
pub mod puppet;
mod raster;
pub mod roto;
pub mod stabilize;

pub use error::ToolError;
pub use point_track::{
    ApplyAxes, Corner, Direction, LowConfidenceAction, MatchEnhance, MotionTrackers, NewPoint,
    PointTracker, SelectedProperty, StepControl, TrackChannels, TrackKind, TrackPlugin,
    TrackSample,
};
pub use raster::{CompareChannel, FrameClip, Raster};
