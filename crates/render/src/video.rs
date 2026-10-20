/// H.264 と ProRes の設定。符号化そのものは FFmpeg が行うので、ここでは呼ばない。

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum H264EncoderKind {
    Hardware,
    Software,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct H264Options {
    pub bitrate_mbps: u32,
    pub profile: &'static str,
    pub level: &'static str,
    pub encoder: H264EncoderKind,
    pub audio: bool,
}

pub const MOVIE_DEFAULT_NAME: &str = "H.264 - Match Render Settings - 15 Mbps";

/// 解像度ごとの対応表は要確認。1080 と 2160 で Level が違うことだけを固定する。
pub fn h264_level(width: u32, height: u32) -> &'static str {
    if width as u64 * height as u64 > 1920 * 1080 {
        "5.2"
    } else if width as u64 * height as u64 > 1280 * 720 {
        "4.2"
    } else {
        "4.0"
    }
}

pub fn h264_options(bitrate_mbps: u32, width: u32, height: u32, encoder: H264EncoderKind, audio: bool) -> H264Options {
    H264Options {
        bitrate_mbps,
        profile: "High",
        level: h264_level(width, height),
        encoder,
        audio,
    }
}

pub fn estimated_bytes(megabits_per_second: u32, seconds: f64) -> u64 {
    (f64::from(megabits_per_second) * 1_000_000.0 / 8.0 * seconds) as u64
}

pub fn even_dimension(value: u32) -> u32 {
    if value <= 2 {
        2
    } else if value % 2 == 0 {
        value
    } else {
        value - 1
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProResVariant {
    Proxy422,
    Lt422,
    Standard422,
    Hq422,
    FourFourFour,
    FourFourFourXq,
}

impl ProResVariant {
    pub fn name(self) -> &'static str {
        match self {
            Self::Proxy422 => "Apple ProRes 422 Proxy",
            Self::Lt422 => "Apple ProRes 422 LT",
            Self::Standard422 => "Apple ProRes 422",
            Self::Hq422 => "Apple ProRes 422 HQ",
            Self::FourFourFour => "Apple ProRes 4444",
            Self::FourFourFourXq => "Apple ProRes 4444 XQ",
        }
    }

    pub fn weight(self) -> u32 {
        match self {
            Self::Proxy422 => 1,
            Self::Lt422 => 2,
            Self::Standard422 => 3,
            Self::Hq422 => 4,
            Self::FourFourFour => 5,
            Self::FourFourFourXq => 6,
        }
    }

    pub fn keeps_alpha(self) -> bool {
        matches!(self, Self::FourFourFour | Self::FourFourFourXq)
    }

    pub fn windows_only(self) -> bool {
        matches!(self, Self::FourFourFourXq)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProResOptions {
    pub variant: ProResVariant,
    pub hardware: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_differ_between_1080_and_2160() {
        assert_ne!(h264_level(1920, 1080), h264_level(3840, 2160));
    }

    #[test]
    fn forty_megabits_is_larger_than_five() {
        assert!(estimated_bytes(40, 2.0) > estimated_bytes(5, 2.0));
    }

    #[test]
    fn both_encoders_keep_the_planned_size() {
        let hardware = h264_options(15, 1920, 1080, H264EncoderKind::Hardware, true);
        let software = h264_options(15, 1920, 1080, H264EncoderKind::Software, true);
        assert_eq!(hardware.level, software.level);
        assert!(hardware.audio && software.audio);
        assert_eq!(even_dimension(1919), 1918);
    }

    #[test]
    fn prores_variants_have_official_names() {
        assert_eq!(ProResVariant::Proxy422.name(), "Apple ProRes 422 Proxy");
        assert_eq!(ProResVariant::Lt422.name(), "Apple ProRes 422 LT");
        assert_eq!(ProResVariant::Standard422.name(), "Apple ProRes 422");
        assert_eq!(ProResVariant::Hq422.name(), "Apple ProRes 422 HQ");
        assert_eq!(ProResVariant::FourFourFour.name(), "Apple ProRes 4444");
        assert_eq!(ProResVariant::FourFourFourXq.name(), "Apple ProRes 4444 XQ");
        assert!(ProResVariant::Hq422.weight() > ProResVariant::Proxy422.weight());
        assert!(ProResVariant::FourFourFour.keeps_alpha());
        assert!(ProResVariant::FourFourFourXq.windows_only());
        let hardware = ProResOptions { variant: ProResVariant::Standard422, hardware: true };
        let software = ProResOptions { variant: hardware.variant, hardware: false };
        assert_eq!(hardware.variant.name(), software.variant.name());
    }
}
