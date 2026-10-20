use model::Time;

/// コラップストランスフォーム、またはベクトルレイヤーの連続ラスタライズ。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct CollapseTransformSample {
    pub enabled: bool,
}

/// コラップストランスフォームの入力を受ける。
pub trait CollapseTransformEval {
    fn with_collapse_transformations(
        &self,
        time: Time,
        collapse: &CollapseTransformSample,
    ) -> bool {
        let _ = (self, time);
        collapse.enabled
    }
}
