use model::{LayerId, Time, TrackMatteMode};

/// サンプル済みのトラックマット。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrackMatteSample {
    pub mode: TrackMatteMode,
    pub source: Option<LayerId>,
}

impl Default for TrackMatteSample {
    fn default() -> Self {
        Self {
            mode: TrackMatteMode::None,
            source: None,
        }
    }
}

/// トラックマットの入力を受ける。既定はそのまま返す。
pub trait TrackMatteEval {
    fn with_track_matte(&self, time: Time, matte: &TrackMatteSample) -> TrackMatteSample {
        let _ = (self, time);
        *matte
    }
}
