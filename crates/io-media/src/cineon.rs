use model::BitsPerChannel;

/// ダイアログの単位。細目の名称は要確認。10 bit コードを基準にする。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CineonUnits {
    TenBitCode,
}

/// Cineon / DPX の変換。10 bit の基準は黒 95、白 685。各欄の初期値は要確認。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CineonSettings {
    pub converted_black: f32,
    pub converted_white: f32,
    pub black_10bit: f32,
    pub white_10bit: f32,
    pub gamma: f32,
    pub highlight_rolloff: f32,
    pub logarithmic: bool,
    pub units: CineonUnits,
}

impl CineonSettings {
    /// 確認できた基準点だけを置く。ダイアログを開いたときの初期値ではない。
    pub fn reference_points() -> Self {
        Self {
            converted_black: 0.0,
            converted_white: 1.0,
            black_10bit: 95.0,
            white_10bit: 685.0,
            gamma: 1.0,
            highlight_rolloff: 0.0,
            logarithmic: false,
            units: CineonUnits::TenBitCode,
        }
    }
}

/// 10 bit コードを、プロジェクト深度のチャネル値へ変換する。
/// 32 bpc で Highlight Rolloff が 0 のとき、白点を超えた値は 1 より上に残る。
pub fn cineon_code_to_channel(code: f32, settings: &CineonSettings, depth: BitsPerChannel) -> f32 {
    let linear = code_to_unit(code, settings);
    match depth {
        BitsPerChannel::Bpc8 => linear.clamp(0.0, 1.0) * 255.0,
        BitsPerChannel::Bpc16 => linear.clamp(0.0, 1.0) * 32768.0,
        BitsPerChannel::Bpc32 => linear,
    }
}

fn code_to_unit(code: f32, settings: &CineonSettings) -> f32 {
    let span = (settings.white_10bit - settings.black_10bit).max(f32::EPSILON);
    let mut unit = if settings.logarithmic {
        let white = 10f32.powf(0.0);
        let black = 10f32.powf((settings.black_10bit - settings.white_10bit) * 0.002 / 0.6);
        let value = 10f32.powf((code - settings.white_10bit) * 0.002 / 0.6);
        (value - black) / (white - black)
    } else {
        (code - settings.black_10bit) / span
    };
    if settings.highlight_rolloff > 0.0 && unit > 1.0 {
        let over = unit - 1.0;
        unit = 1.0 + over / (1.0 + over * settings.highlight_rolloff);
    }
    if settings.gamma > 0.0 && (settings.gamma - 1.0).abs() > 1e-6 {
        let sign = unit.signum();
        unit = sign * unit.abs().powf(1.0 / settings.gamma);
    }
    settings.converted_black + unit * (settings.converted_white - settings.converted_black)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sixteen_and_thirty_two_keep_the_white_point() {
        let settings = CineonSettings::reference_points();
        let white_16 = cineon_code_to_channel(685.0, &settings, BitsPerChannel::Bpc16);
        let above_16 = cineon_code_to_channel(900.0, &settings, BitsPerChannel::Bpc16);
        let above_32 = cineon_code_to_channel(900.0, &settings, BitsPerChannel::Bpc32);
        assert!((white_16 - 32768.0).abs() < 1.0);
        assert!(above_16 + 0.5 >= white_16);
        assert!(above_32 > 1.0);
    }

    #[test]
    fn white_point_changes_highlight_code() {
        let mut narrow = CineonSettings::reference_points();
        let mut wide = narrow;
        narrow.white_10bit = 500.0;
        wide.white_10bit = 685.0;
        let narrow_value = cineon_code_to_channel(600.0, &narrow, BitsPerChannel::Bpc32);
        let wide_value = cineon_code_to_channel(600.0, &wide, BitsPerChannel::Bpc32);
        assert!(narrow_value > wide_value);
    }

    #[test]
    fn logarithmic_conversion_changes_mid_gray() {
        let mut linear = CineonSettings::reference_points();
        linear.logarithmic = false;
        let mut log = linear;
        log.logarithmic = true;
        let code = 390.0;
        let linear_value = cineon_code_to_channel(code, &linear, BitsPerChannel::Bpc32);
        let log_value = cineon_code_to_channel(code, &log, BitsPerChannel::Bpc32);
        assert!((linear_value - log_value).abs() > 0.02);
    }
}
