/// 秒で表す時刻。
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Time {
    seconds: f64,
}

impl Time {
    pub const ZERO: Self = Self { seconds: 0.0 };

    pub const fn from_seconds(seconds: f64) -> Self {
        Self { seconds }
    }

    pub const fn seconds(self) -> f64 {
        self.seconds
    }

    /// フレームレートが 0 以下のときは 0 秒を返す。
    pub fn from_frames(frame: f64, frame_rate: f64) -> Self {
        if frame_rate <= 0.0 {
            Self::ZERO
        } else {
            Self {
                seconds: frame / frame_rate,
            }
        }
    }

    pub fn to_frames(self, frame_rate: f64) -> f64 {
        if frame_rate <= 0.0 {
            0.0
        } else {
            self.seconds * frame_rate
        }
    }
}

impl Default for Time {
    fn default() -> Self {
        Self::ZERO
    }
}
