use model::{BitsPerChannel, Time};

use crate::cpu::{ApplyResult, IdentityEffect};
use crate::param::ParamValues;

/// GPU リソースの不透明な参照。実行環境はホストが決める。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GpuBuffer {
    pub width: u32,
    pub height: u32,
    pub bits: BitsPerChannel,
    pub channels: u8,
    pub id: u64,
}

/// 入力から出力へのコピーを表すパス。このクレートは GPU を実行しない。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GpuPass {
    pub input: GpuBuffer,
    pub output: GpuBuffer,
}

#[derive(Clone, Copy, Debug)]
pub struct GpuInput<'a> {
    pub time: Time,
    pub bits: BitsPerChannel,
    pub params: &'a ParamValues,
    pub input: GpuBuffer,
    pub output: GpuBuffer,
}

/// GPU 適用の入口。戻りはホストが実行するパス。
pub trait GpuApply {
    fn apply_gpu(&self, input: &GpuInput<'_>) -> ApplyResult<GpuPass>;
}

impl GpuApply for IdentityEffect {
    fn apply_gpu(&self, input: &GpuInput<'_>) -> ApplyResult<GpuPass> {
        let _ = (self, input.time, input.bits, input.params);
        Ok(GpuPass {
            input: input.input,
            output: input.output,
        })
    }
}
