//! エフェクトのパラメータ定義と、CPU / GPU の適用入口。

#![forbid(unsafe_code)]

pub mod cpu;
pub mod frame;
pub mod gpu;
pub mod param;

pub use cpu::{ApplyError, ApplyResult, CpuApply, CpuInput, IdentityEffect};
pub use frame::{
    channel_16_to_32, channel_32_to_8, channel_8_to_16, convert_16_to_32, convert_32_to_8,
    convert_8_to_16, copy_frame, Frame, Frame16, Frame32, Frame8,
};
pub use gpu::{GpuApply, GpuBuffer, GpuInput, GpuPass};
pub use param::{defaults_from, Param, ParamIndex, ParamList, ParamValue, ParamValues};
