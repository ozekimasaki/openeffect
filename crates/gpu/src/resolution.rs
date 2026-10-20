/// コンポジションパネルの Resolution / Down Sample Factor。
///
/// Third と Quarter は、ヘルプが保証している画素数が 1/9 と 1/16 になるよう、
/// 縦横を同じ整数で割る。列と行の比そのものはヘルプが要確認のままなので、
/// それ以上の比率は決めない。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resolution {
    Auto,
    Full,
    Half,
    Third,
    Quarter,
    Custom { horizontal: u32, vertical: u32 },
}

/// プレビューが実際に使う解像度の表示名。
///
/// Auto で分母が 4 のときだけヘルプの例に従い Quarter と表示する。
/// それ以外の Auto 分母のメニュー名は要確認。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreviewLabel {
    Full,
    Half,
    Third,
    Quarter,
    Custom { horizontal: u32, vertical: u32 },
    Auto { factor: u32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompResolution {
    pub value: Resolution,
    pub preserve_when_nested: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResolutionError {
    ZeroDenominator,
}

impl CompResolution {
    pub const fn full() -> Self {
        Self {
            value: Resolution::Full,
            preserve_when_nested: false,
        }
    }

    pub const fn auto() -> Self {
        Self {
            value: Resolution::Auto,
            preserve_when_nested: false,
        }
    }

    pub fn effective(self) -> Resolution {
        if self.preserve_when_nested && matches!(self.value, Resolution::Auto) {
            Resolution::Full
        } else {
            self.value
        }
    }

    pub fn denominators(self, view_zooms: &[f64]) -> Result<(u32, u32), ResolutionError> {
        match self.effective() {
            Resolution::Auto => {
                let factor = auto_factor_for_views(view_zooms);
                Ok((factor, factor))
            }
            Resolution::Full => Ok((1, 1)),
            Resolution::Half => Ok((2, 2)),
            Resolution::Third => Ok((3, 3)),
            Resolution::Quarter => Ok((4, 4)),
            Resolution::Custom {
                horizontal,
                vertical,
            } => {
                if horizontal == 0 || vertical == 0 {
                    Err(ResolutionError::ZeroDenominator)
                } else {
                    Ok((horizontal, vertical))
                }
            }
        }
    }

    pub fn buffer_size(
        self,
        comp_width: u32,
        comp_height: u32,
        view_zooms: &[f64],
    ) -> Result<(u32, u32), ResolutionError> {
        let (horizontal, vertical) = self.denominators(view_zooms)?;
        Ok((
            div_dim(comp_width, horizontal),
            div_dim(comp_height, vertical),
        ))
    }

    pub fn preview_label(self, view_zooms: &[f64]) -> PreviewLabel {
        match self.effective() {
            Resolution::Auto => {
                let factor = auto_factor_for_views(view_zooms);
                if factor == 4 {
                    PreviewLabel::Quarter
                } else {
                    PreviewLabel::Auto { factor }
                }
            }
            Resolution::Full => PreviewLabel::Full,
            Resolution::Half => PreviewLabel::Half,
            Resolution::Third => PreviewLabel::Third,
            Resolution::Quarter => PreviewLabel::Quarter,
            Resolution::Custom {
                horizontal,
                vertical,
            } => PreviewLabel::Custom {
                horizontal,
                vertical,
            },
        }
    }
}

/// 書き出しの縦横。プレビュー解像度では変わらない。
pub fn export_size(comp_width: u32, comp_height: u32) -> (u32, u32) {
    (comp_width, comp_height)
}

/// ズームで必要な画素だけを描くときの分母。
///
/// 25% は 1/0.25 = 4 で、ヘルプの Quarter の例と一致する。
/// 固定の分母一覧は要確認なので、名前の付いた段階表にはしない。
pub fn auto_factor(zoom: f64) -> u32 {
    if !zoom.is_finite() || zoom <= 0.0 {
        return 1;
    }
    let inverse = (1.0 / zoom).floor();
    if inverse < 1.0 {
        1
    } else if inverse >= f64::from(u32::MAX) {
        u32::MAX
    } else {
        inverse as u32
    }
}

/// 複数ビューでは、いちばん拡大されているビュー（分母が最小）に合わせる。
pub fn auto_factor_for_views(view_zooms: &[f64]) -> u32 {
    view_zooms
        .iter()
        .copied()
        .map(auto_factor)
        .min()
        .unwrap_or(1)
}

fn div_dim(size: u32, denominator: u32) -> u32 {
    if denominator == 0 {
        return size.max(1);
    }
    (size / denominator).max(1)
}

#[cfg(test)]
mod tests {
    use super::{export_size, CompResolution, PreviewLabel, Resolution, ResolutionError};

    #[test]
    fn full_matches_composition_size() {
        let size = CompResolution::full()
            .buffer_size(1920, 1080, &[1.0])
            .unwrap();
        assert_eq!(size, (1920, 1080));
        assert_eq!(
            CompResolution::full().preview_label(&[1.0]),
            PreviewLabel::Full
        );
    }

    #[test]
    fn half_draws_a_quarter_of_the_pixels() {
        let (width, height) = CompResolution {
            value: Resolution::Half,
            preserve_when_nested: false,
        }
        .buffer_size(100, 80, &[1.0])
        .unwrap();
        assert_eq!((width, height), (50, 40));
        assert_eq!(width * height * 4, 100 * 80);
    }

    #[test]
    fn third_draws_one_ninth() {
        let (width, height) = CompResolution {
            value: Resolution::Third,
            preserve_when_nested: false,
        }
        .buffer_size(90, 45, &[1.0])
        .unwrap();
        assert_eq!((width, height), (30, 15));
        assert_eq!(width * height * 9, 90 * 45);
    }

    #[test]
    fn quarter_draws_one_sixteenth() {
        let (width, height) = CompResolution {
            value: Resolution::Quarter,
            preserve_when_nested: false,
        }
        .buffer_size(80, 40, &[1.0])
        .unwrap();
        assert_eq!((width, height), (20, 10));
        assert_eq!(width * height * 16, 80 * 40);
    }

    #[test]
    fn custom_keeps_only_the_requested_axes() {
        let (width, height) = CompResolution {
            value: Resolution::Custom {
                horizontal: 2,
                vertical: 4,
            },
            preserve_when_nested: false,
        }
        .buffer_size(80, 80, &[1.0])
        .unwrap();
        assert_eq!((width, height), (40, 20));
    }

    #[test]
    fn auto_at_25_percent_is_quarter() {
        let resolution = CompResolution::auto();
        let (width, height) = resolution.buffer_size(80, 40, &[0.25]).unwrap();
        assert_eq!((width, height), (20, 10));
        assert_eq!(width * height * 16, 80 * 40);
        assert_eq!(resolution.preview_label(&[0.25]), PreviewLabel::Quarter);
    }

    #[test]
    fn auto_follows_the_most_magnified_view() {
        let resolution = CompResolution::auto();
        let size = resolution.buffer_size(80, 40, &[0.25, 1.0]).unwrap();
        assert_eq!(size, (80, 40));
        assert_eq!(
            resolution.preview_label(&[0.25, 1.0]),
            PreviewLabel::Auto { factor: 1 }
        );
    }

    #[test]
    fn auto_is_ignored_when_nested_resolution_is_preserved() {
        let mut resolution = CompResolution::auto();
        resolution.preserve_when_nested = true;
        assert_eq!(resolution.value, Resolution::Auto);
        let size = resolution.buffer_size(80, 40, &[0.25]).unwrap();
        assert_eq!(size, export_size(80, 40));
        assert_eq!(resolution.preview_label(&[0.25]), PreviewLabel::Full);
    }

    #[test]
    fn stored_half_still_applies_when_nested_resolution_is_preserved() {
        let resolution = CompResolution {
            value: Resolution::Half,
            preserve_when_nested: true,
        };
        assert_eq!(resolution.buffer_size(100, 40, &[0.25]).unwrap(), (50, 20));
    }

    #[test]
    fn export_pixel_count_ignores_preview_resolution() {
        assert_eq!(export_size(1920, 1080), (1920, 1080));
        let preview = CompResolution {
            value: Resolution::Half,
            preserve_when_nested: false,
        }
        .buffer_size(1920, 1080, &[0.25])
        .unwrap();
        assert_ne!(preview, export_size(1920, 1080));
    }

    #[test]
    fn custom_zero_denominator_is_rejected() {
        let resolution = CompResolution {
            value: Resolution::Custom {
                horizontal: 0,
                vertical: 2,
            },
            preserve_when_nested: false,
        };
        assert_eq!(
            resolution.buffer_size(10, 10, &[1.0]),
            Err(ResolutionError::ZeroDenominator)
        );
    }
}
