use model::{LayerId, Time, Transform3d};

/// 親をサンプルしたあとの姿勢。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParentSample {
    pub parent_id: LayerId,
    pub world: Transform3d,
}

/// 親子の入力を受ける。親が無いときは恒等変換。
pub trait ParentEval {
    fn with_parent(&self, time: Time, parent: Option<&ParentSample>) -> Transform3d {
        let _ = (self, time);
        match parent {
            Some(parent) => parent.world,
            None => Transform3d::identity(),
        }
    }
}
