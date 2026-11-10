use effect_api::{ApplyError, ApplyResult, CpuApply, CpuInput, Frame, GpuApply, GpuInput, GpuPass};

use crate::apply::render_frame;
use crate::catalog::EffectId;
use crate::values::EffectValues;

/// 効果スタックに載った 1 つの効果。
#[derive(Clone, Debug, PartialEq)]
pub struct AppliedEffect {
    pub id: EffectId,
    pub values: EffectValues,
}

impl CpuApply for AppliedEffect {
    fn apply_cpu(&self, input: &CpuInput<'_>, output: &mut Frame) -> ApplyResult<()> {
        render_frame(self.id, input, &self.values, output)
    }
}

impl GpuApply for AppliedEffect {
    fn apply_gpu(&self, input: &GpuInput<'_>) -> ApplyResult<GpuPass> {
        let _ = input;
        Err(ApplyError::new("effects-sim-time は CPU で適用する"))
    }
}
