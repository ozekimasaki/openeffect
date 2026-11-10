use model::{BlendMode, Time};

/// サンプル済みのブレンド。不透明度 1 が不透明。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlendSample {
    pub mode: BlendMode,
    pub opacity: f64,
}

impl Default for BlendSample {
    fn default() -> Self {
        Self {
            mode: BlendMode::Normal,
            opacity: 1.0,
        }
    }
}

/// ブレンドの入力を受ける。既定はそのまま返す。
pub trait BlendEval {
    fn with_blend(&self, time: Time, blend: &BlendSample) -> BlendSample {
        let _ = (self, time);
        *blend
    }
}
