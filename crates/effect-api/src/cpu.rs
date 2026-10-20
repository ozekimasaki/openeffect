use model::{BitsPerChannel, Time};

use crate::frame::{copy_frame, Frame};
use crate::param::ParamValues;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApplyError {
    pub message: String,
}

impl ApplyError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

pub type ApplyResult<T> = Result<T, ApplyError>;

/// CPU 適用の入力。
#[derive(Clone, Copy, Debug)]
pub struct CpuInput<'a> {
    pub time: Time,
    pub bits: BitsPerChannel,
    pub params: &'a ParamValues,
    pub frame: &'a Frame,
}

/// CPU で 1 フレームを出力バッファへ書く。
pub trait CpuApply {
    fn apply_cpu(&self, input: &CpuInput<'_>, output: &mut Frame) -> ApplyResult<()>;
}

/// 入力を出力の深度へコピーする。
#[derive(Clone, Copy, Debug, Default)]
pub struct IdentityEffect;

impl CpuApply for IdentityEffect {
    fn apply_cpu(&self, input: &CpuInput<'_>, output: &mut Frame) -> ApplyResult<()> {
        let _ = (self, input.time, input.bits, input.params);
        copy_frame(input.frame, output);
        Ok(())
    }
}
