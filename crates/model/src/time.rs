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

/// フレーム表示の数え始め。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FrameNumbering {
    /// タイムコードが無ければ 0。
    TimecodeConversion,
    StartAt0,
    StartAt1,
}

/// SMPTE の時:分:秒:フレーム。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Timecode {
    pub hours: u32,
    pub minutes: u32,
    pub seconds: u32,
    pub frames: u32,
    pub drop_frame: bool,
}

impl Timecode {
    pub fn format(self) -> String {
        format!(
            "{:02}:{:02}:{:02}:{:02}",
            self.hours, self.minutes, self.seconds, self.frames
        )
    }
}

/// 29.97 は公称 30 フレームで数える。ドロップは毎分 2 フレーム、10 分ごとは落とさない。
pub fn frame_to_timecode(frame: i64, drop_frame: bool) -> Timecode {
    let frame = frame.max(0);
    let nominal = if drop_frame {
        let frames_per_min = 30 * 60 - 2;
        let frames_per_10_min = frames_per_min * 10 + 2;
        let tens = frame / frames_per_10_min;
        let rem = frame % frames_per_10_min;
        let mut dropped = 2 * 9 * tens;
        if rem > 2 {
            dropped += 2 * ((rem - 2) / frames_per_min);
        }
        frame + dropped
    } else {
        frame
    };
    let frames = (nominal % 30) as u32;
    let mut seconds_total = nominal / 30;
    let seconds = (seconds_total % 60) as u32;
    seconds_total /= 60;
    let minutes = (seconds_total % 60) as u32;
    let hours = (seconds_total / 60) as u32;
    Timecode {
        hours,
        minutes,
        seconds,
        frames,
        drop_frame,
    }
}

pub fn frame_number(index: i64, numbering: FrameNumbering, media_start_frame: Option<i64>) -> i64 {
    match numbering {
        FrameNumbering::StartAt0 => index,
        FrameNumbering::StartAt1 => index + 1,
        FrameNumbering::TimecodeConversion => index + media_start_frame.unwrap_or(0),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FeetFrames {
    pub feet: i64,
    pub frames: i64,
}

/// 1 フィートのフレーム数は呼び出し側が渡す。規格の初期値は推測しない。
pub fn feet_frames(frame_index: i64, frames_per_foot: u32) -> Option<FeetFrames> {
    if frames_per_foot == 0 {
        return None;
    }
    let span = i64::from(frames_per_foot);
    let frame_index = frame_index.max(0);
    Some(FeetFrames {
        feet: frame_index / span,
        frames: frame_index % span,
    })
}

/// 露光時間。24 fps で 90 度なら 1/96 秒。
pub fn exposure_seconds(shutter_angle_degrees: f64, frame_rate: f64) -> Option<f64> {
    if frame_rate <= 0.0 || shutter_angle_degrees < 0.0 {
        return None;
    }
    Some((shutter_angle_degrees / 360.0) / frame_rate)
}

/// シャッターが開いている時刻の区間。位相はフレーム頭からの度。
pub fn shutter_window(
    frame_time: f64,
    frame_rate: f64,
    shutter_angle_degrees: f64,
    shutter_phase_degrees: f64,
) -> Option<(f64, f64)> {
    if frame_rate <= 0.0 || shutter_angle_degrees < 0.0 {
        return None;
    }
    let exposure = shutter_angle_degrees / 360.0 / frame_rate;
    let shift = shutter_phase_degrees / 360.0 / frame_rate;
    let start = frame_time + shift;
    Some((start, start + exposure))
}

/// 1 秒間にサンプルされるフレーム。時刻が 1 秒未満のコマ数。上限は 999 fps。
pub fn samples_in_one_second(frame_rate: f64) -> Option<u32> {
    if !(frame_rate > 0.0 && frame_rate <= 999.0) {
        return None;
    }
    let mut count = 0u32;
    let mut index = 0u32;
    while f64::from(index) / frame_rate < 1.0 {
        count += 1;
        index += 1;
        if index > 999 {
            break;
        }
    }
    Some(count)
}
