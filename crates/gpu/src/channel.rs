use effect_api::Frame32;

/// Show Channel。Alpha Boundary と Alpha Overlay はレイヤーパネル専用なので置かない。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ChannelDisplay {
    #[default]
    Rgb,
    Red,
    Green,
    Blue,
    Alpha,
    RgbStraight,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ViewerQuality {
    Faster,
    #[default]
    MoreAccurateExceptCached,
    MoreAccurate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct ChannelView {
    pub display: ChannelDisplay,
    pub colorize: bool,
}

impl ChannelView {
    /// Show Channel ボタンの Alt+クリック。アルファと全 RGB を往復する。
    pub fn alt_click(&mut self) {
        self.display = match self.display {
            ChannelDisplay::Alpha => ChannelDisplay::Rgb,
            ChannelDisplay::Rgb
            | ChannelDisplay::Red
            | ChannelDisplay::Green
            | ChannelDisplay::Blue
            | ChannelDisplay::RgbStraight => ChannelDisplay::Alpha,
        };
    }

    /// RGB Straight のあいだは Viewer Quality を Faster として扱う。
    pub fn effective_quality(self, requested: ViewerQuality) -> ViewerQuality {
        match self.display {
            ChannelDisplay::RgbStraight => ViewerQuality::Faster,
            ChannelDisplay::Rgb
            | ChannelDisplay::Red
            | ChannelDisplay::Green
            | ChannelDisplay::Blue
            | ChannelDisplay::Alpha => requested,
        }
    }
}

/// Alt+1 から Alt+4、および Shift 付き。アルファの色付きショートカットは無い。
pub fn channel_from_shortcut(alt: bool, shift: bool, digit: u8) -> Option<ChannelView> {
    if !alt {
        return None;
    }
    match (shift, digit) {
        (false, 1) => Some(ChannelView {
            display: ChannelDisplay::Red,
            colorize: false,
        }),
        (false, 2) => Some(ChannelView {
            display: ChannelDisplay::Green,
            colorize: false,
        }),
        (false, 3) => Some(ChannelView {
            display: ChannelDisplay::Blue,
            colorize: false,
        }),
        (false, 4) => Some(ChannelView {
            display: ChannelDisplay::Alpha,
            colorize: false,
        }),
        (true, 1) => Some(ChannelView {
            display: ChannelDisplay::Red,
            colorize: true,
        }),
        (true, 2) => Some(ChannelView {
            display: ChannelDisplay::Green,
            colorize: true,
        }),
        (true, 3) => Some(ChannelView {
            display: ChannelDisplay::Blue,
            colorize: true,
        }),
        (true, 4) => Some(ChannelView {
            display: ChannelDisplay::RgbStraight,
            colorize: false,
        }),
        _ => None,
    }
}

/// プレビュー用の表示バッファ。入力フレームは変えない。
pub fn present_channels(view: ChannelView, frame: &Frame32) -> Frame32 {
    let mut output = Frame32::new(frame.width, frame.height, 4, 0.0);
    if frame.channels != 4 {
        return output;
    }
    let pixels = frame.width.saturating_mul(frame.height) as usize;
    for index in 0..pixels {
        let src = read_pixel(frame, index);
        let shown = present_pixel(view, src);
        write_pixel(&mut output, index, shown);
    }
    output
}

fn present_pixel(view: ChannelView, pixel: [f32; 4]) -> [f32; 4] {
    let [r, g, b, a] = pixel;
    match view.display {
        ChannelDisplay::Rgb => [r * a, g * a, b * a, a],
        ChannelDisplay::RgbStraight => [r, g, b, a],
        ChannelDisplay::Alpha => gray(a),
        ChannelDisplay::Red => channel_color(r, [1.0, 0.0, 0.0], view.colorize),
        ChannelDisplay::Green => channel_color(g, [0.0, 1.0, 0.0], view.colorize),
        ChannelDisplay::Blue => channel_color(b, [0.0, 0.0, 1.0], view.colorize),
    }
}

fn channel_color(value: f32, tint: [f32; 3], colorize: bool) -> [f32; 4] {
    if colorize {
        [value * tint[0], value * tint[1], value * tint[2], 1.0]
    } else {
        gray(value)
    }
}

fn gray(value: f32) -> [f32; 4] {
    [value, value, value, 1.0]
}

fn read_pixel(frame: &Frame32, index: usize) -> [f32; 4] {
    let start = index.saturating_mul(4);
    if start + 3 >= frame.samples.len() {
        return [0.0; 4];
    }
    [
        frame.samples[start],
        frame.samples[start + 1],
        frame.samples[start + 2],
        frame.samples[start + 3],
    ]
}

fn write_pixel(frame: &mut Frame32, index: usize, pixel: [f32; 4]) {
    let start = index.saturating_mul(4);
    if start + 3 >= frame.samples.len() {
        return;
    }
    frame.samples[start..start + 4].copy_from_slice(&pixel);
}

#[cfg(test)]
mod tests {
    use super::{
        channel_from_shortcut, present_channels, ChannelDisplay, ChannelView, ViewerQuality,
    };
    use effect_api::Frame32;

    fn solid(pixel: [f32; 4]) -> Frame32 {
        let mut frame = Frame32::new(1, 1, 4, 0.0);
        frame.samples.copy_from_slice(&pixel);
        frame
    }

    fn shown(view: ChannelView, pixel: [f32; 4]) -> [f32; 4] {
        let frame = solid(pixel);
        let output = present_channels(view, &frame);
        output.samples.as_slice().try_into().unwrap()
    }

    #[test]
    fn rgb_shows_all_channels_and_alt_click_toggles_alpha() {
        let mut view = ChannelView::default();
        let pixel = shown(view, [0.2, 0.4, 0.6, 1.0]);
        assert_eq!(pixel, [0.2, 0.4, 0.6, 1.0]);
        view.alt_click();
        assert_eq!(view.display, ChannelDisplay::Alpha);
        view.alt_click();
        assert_eq!(view.display, ChannelDisplay::Rgb);
    }

    #[test]
    fn primary_channels_are_grayscale() {
        let red = ChannelView {
            display: ChannelDisplay::Red,
            colorize: false,
        };
        assert_eq!(shown(red, [1.0, 0.0, 0.0, 1.0]), [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(shown(red, [0.0, 1.0, 0.0, 1.0]), [0.0, 0.0, 0.0, 1.0]);
        let green = ChannelView {
            display: ChannelDisplay::Green,
            colorize: false,
        };
        assert_eq!(shown(green, [0.0, 1.0, 0.0, 1.0]), [1.0, 1.0, 1.0, 1.0]);
        let blue = ChannelView {
            display: ChannelDisplay::Blue,
            colorize: false,
        };
        assert_eq!(shown(blue, [0.0, 0.0, 1.0, 1.0]), [1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn alpha_is_grayscale_without_color() {
        let view = ChannelView {
            display: ChannelDisplay::Alpha,
            colorize: false,
        };
        assert_eq!(shown(view, [1.0, 0.0, 0.0, 0.0]), [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(shown(view, [0.0, 1.0, 0.0, 1.0]), [1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn straight_rgb_differs_from_premultiplied_rgb() {
        let source = solid([1.0, 0.0, 0.0, 0.5]);
        let before = source.samples.clone();
        let rgb = present_channels(
            ChannelView {
                display: ChannelDisplay::Rgb,
                colorize: false,
            },
            &source,
        );
        let straight = present_channels(
            ChannelView {
                display: ChannelDisplay::RgbStraight,
                colorize: false,
            },
            &source,
        );
        assert_eq!(source.samples, before);
        assert_eq!(rgb.samples.as_slice(), &[0.5, 0.0, 0.0, 0.5]);
        assert_eq!(straight.samples.as_slice(), &[1.0, 0.0, 0.0, 0.5]);
        assert_ne!(rgb.samples, straight.samples);
    }

    #[test]
    fn colorize_shows_the_channel_hue() {
        let gray = shown(
            ChannelView {
                display: ChannelDisplay::Red,
                colorize: false,
            },
            [1.0, 0.0, 0.0, 1.0],
        );
        let tinted = shown(
            ChannelView {
                display: ChannelDisplay::Red,
                colorize: true,
            },
            [1.0, 0.0, 0.0, 1.0],
        );
        assert_eq!(tinted, [1.0, 0.0, 0.0, 1.0]);
        assert_ne!(gray, tinted);
        assert!(
            channel_from_shortcut(true, true, 4).unwrap().display == ChannelDisplay::RgbStraight
        );
        assert!(channel_from_shortcut(false, true, 1).is_none());
    }

    #[test]
    fn straight_display_forces_faster_quality() {
        let straight = ChannelView {
            display: ChannelDisplay::RgbStraight,
            colorize: false,
        };
        assert_eq!(
            straight.effective_quality(ViewerQuality::MoreAccurate),
            ViewerQuality::Faster
        );
        let rgb = ChannelView::default();
        assert_eq!(
            rgb.effective_quality(ViewerQuality::MoreAccurate),
            ViewerQuality::MoreAccurate
        );
    }
}
