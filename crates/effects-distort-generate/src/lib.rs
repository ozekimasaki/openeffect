//! Distort、Generate、Perspective の CPU 効果。
//!
//! 解説が初期値を書いていない項目は既定値を持たない。表示名が要確認の役割は
//! [`Instance::set_role`] に置き、Effect Controls の項目名としては出さない。

#![forbid(unsafe_code)]

mod host;
mod image;
mod render;
mod spec;

pub use host::{
    api_params, fallback_method, run, run_bits, AudioClip, Host, Instance, LiquifyStroke,
    LiquifyTool, MaskShape, MeshPoint, MotionSample, Quality, SourceLayer, Stack, TrackObs, Value,
};
pub use image::{displace_amount, Image};
pub use render::{polar_pair, wave_dir, wave_sample};
pub use spec::{
    parameters, unconfirmed_note, Category, EffectKind, Keyframe, ParamKind, ParamSpec,
};
