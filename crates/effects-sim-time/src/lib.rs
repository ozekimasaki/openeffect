//! Simulation、3D Channel、Audio、Time、Transition、Utility、Expression Controls、Obsolete。
//! 初期値が要確認の数値は置かない。

#![forbid(unsafe_code)]

mod apply;
mod bag;
mod catalog;
mod cx;
mod hosted;
mod image;
mod paint;
mod stack;
mod values;

pub use apply::{render, Rendered};
pub use catalog::{
    effect_api_params, effect_index, effects, Category, EffectDef, EffectId, ParamDef,
};
pub use cx::RenderCx;
pub use hosted::AppliedEffect;
pub use image::{image_from_frame, Audio, Image};
pub use stack::{add_effect, menu_effects};
pub use values::{EffectValues, SetError, Slot};

#[cfg(test)]
mod harness;

#[cfg(test)]
mod pass_tests;
