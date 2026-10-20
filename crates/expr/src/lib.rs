//! After Effects 互換の JavaScript 式。
//!
//! 公開入口は [`Engine::evaluate`]。中身はこの環境では純 Rust 評価器。

#![forbid(unsafe_code)]

mod ast;
mod edit;
mod eval;
mod host;
mod legacy;
mod ops;
mod parse;
mod scene;
mod value;

pub use edit::{Banner, ComponentDrop};
pub use ops::orientation_forward as orientation_z_axis;
pub use scene::{
    CompAttr, EngineMode, Image, LayerAttr, LayerError, Panel, PropExtra, Rect, Reveal, Scene,
    TimeDisplay,
};
pub use value::Value;

use model::{ItemId, LayerId, PropertyId};

/// 式を評価するレイヤーとプロパティと時刻。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Target {
    pub comp: ItemId,
    pub layer: LayerId,
    pub property: PropertyId,
    pub time: f64,
}

/// 評価に失敗したとき。成功するまでレイヤーに残る。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExprFailure {
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Backend {
    PureRust,
}

/// 式エンジン。JavaScript と Legacy のどちらも、共通構文はこの評価器で実行する。
#[derive(Clone, Debug)]
pub struct Engine {
    backend: Backend,
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    pub fn new() -> Self {
        Self {
            backend: Backend::PureRust,
        }
    }

    pub fn evaluate(
        &self,
        scene: &mut Scene,
        target: Target,
        source: &str,
    ) -> Result<Value, ExprFailure> {
        let result = match self.backend {
            Backend::PureRust => eval::execute(scene, target, source, false),
        };
        match result {
            Ok(value) => {
                scene.clear_error(target.property);
                Ok(value)
            }
            Err(error) => {
                scene.record_error(target, error.message.clone());
                Err(error)
            }
        }
    }
}

impl Scene {
    pub fn eval(&mut self, target: Target, source: &str) -> Result<Value, ExprFailure> {
        Engine::new().evaluate(self, target, source)
    }
}
