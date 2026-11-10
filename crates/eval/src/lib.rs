//! 時刻からの値と、レイヤー合成の入力。画素の合成は持たない。

#![forbid(unsafe_code)]

pub mod blend;
pub mod blend_pixel;
pub mod collapse;
pub mod color_space;
pub mod edit;
pub mod input;
pub mod interpolate;
pub mod motion_blur;
pub mod parent;
pub mod picture;
pub mod render;
pub mod sampler;
pub mod track_matte;

pub use blend::{BlendEval, BlendSample};
pub use collapse::{CollapseTransformEval, CollapseTransformSample};
pub use input::{CompositeInputs, DefaultEvaluator, EvalFrame, LayerEval};
pub use motion_blur::{MotionBlurEval, MotionBlurSample};
pub use parent::{ParentEval, ParentSample};
pub use sampler::{sample_keyframes, sample_property, Constant, PropertySampler, Sampler};
pub use track_matte::{TrackMatteEval, TrackMatteSample};
