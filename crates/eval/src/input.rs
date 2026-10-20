use model::{Time, Transform3d};

use crate::blend::{BlendEval, BlendSample};
use crate::collapse::{CollapseTransformEval, CollapseTransformSample};
use crate::motion_blur::{MotionBlurEval, MotionBlurSample};
use crate::parent::{ParentEval, ParentSample};
use crate::track_matte::{TrackMatteEval, TrackMatteSample};

/// 1 レイヤーの合成入力。
#[derive(Clone, Debug, PartialEq, Default)]
pub struct CompositeInputs {
    pub parent: Option<ParentSample>,
    pub blend: BlendSample,
    pub track_matte: TrackMatteSample,
    pub collapse_transformations: CollapseTransformSample,
    pub motion_blur: MotionBlurSample,
}

/// 合成入力をすべて受けた評価結果。画素は含まない。
#[derive(Clone, Debug, PartialEq)]
pub struct EvalFrame {
    pub time: Time,
    pub transform: Transform3d,
    pub blend: BlendSample,
    pub track_matte: TrackMatteSample,
    pub collapse_transformations: bool,
    pub motion_blur: MotionBlurSample,
}

/// 親子、ブレンド、トラックマット、コラップストランスフォーム、モーションブラーを受ける。
pub trait LayerEval:
    ParentEval + BlendEval + TrackMatteEval + CollapseTransformEval + MotionBlurEval
{
    fn evaluate(&self, time: Time, inputs: &CompositeInputs) -> EvalFrame {
        EvalFrame {
            time,
            transform: self.with_parent(time, inputs.parent.as_ref()),
            blend: self.with_blend(time, &inputs.blend),
            track_matte: self.with_track_matte(time, &inputs.track_matte),
            collapse_transformations: self
                .with_collapse_transformations(time, &inputs.collapse_transformations),
            motion_blur: self.with_motion_blur(time, &inputs.motion_blur),
        }
    }
}

/// 入力をそのまま評価結果にする。
#[derive(Clone, Copy, Debug, Default)]
pub struct DefaultEvaluator;

impl ParentEval for DefaultEvaluator {}
impl BlendEval for DefaultEvaluator {}
impl TrackMatteEval for DefaultEvaluator {}
impl CollapseTransformEval for DefaultEvaluator {}
impl MotionBlurEval for DefaultEvaluator {}
impl LayerEval for DefaultEvaluator {}

#[cfg(test)]
mod tests {
    use super::{CompositeInputs, DefaultEvaluator, LayerEval};
    use crate::blend::BlendSample;
    use model::{BlendMode, Time};

    #[test]
    fn default_evaluator_returns_inputs() {
        let inputs = CompositeInputs {
            blend: BlendSample {
                mode: BlendMode::Multiply,
                opacity: 0.5,
            },
            ..CompositeInputs::default()
        };
        let frame = DefaultEvaluator.evaluate(Time::from_seconds(1.0), &inputs);
        assert_eq!(frame.blend.mode, BlendMode::Multiply);
        assert_eq!(frame.blend.opacity, 0.5);
        assert!(!frame.collapse_transformations);
        assert!(!frame.motion_blur.enabled);
    }
}
