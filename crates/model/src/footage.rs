use crate::color::Rgba;
use crate::id::ItemId;
use crate::time::Time;

/// 平面の一辺の上限。
pub const SOLID_MAX_PIXELS: u32 = 30_000;

/// フッテージ項目の中身。パスは io-api が持つ。
#[derive(Clone, Debug, PartialEq)]
pub enum FootageKind {
    File,
    Solid { color: Rgba },
    Placeholder,
    /// レンダーキューの出力モジュールから作った項目。
    OutputPlaceholder { comp: ItemId },
    Photoshop,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AlphaMode {
    Straight,
    Premultiplied,
    Ignored,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AlphaInterpretation {
    pub mode: AlphaMode,
    /// Premultiplied の背景色。それ以外では使わない。
    pub matte: Rgba,
}

impl AlphaInterpretation {
    pub const fn straight() -> Self {
        Self {
            mode: AlphaMode::Straight,
            matte: Rgba::black(),
        }
    }

    pub const fn ignored() -> Self {
        Self {
            mode: AlphaMode::Ignored,
            matte: Rgba::black(),
        }
    }

    pub const fn premultiplied(matte: Rgba) -> Self {
        Self {
            mode: AlphaMode::Premultiplied,
            matte,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FieldSeparation {
    Off,
    UpperFirst,
    LowerFirst,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Pulldown {
    Off,
    /// 位相の表示名は要確認。番号だけ持つ。
    ThreeTwo { phase: u8 },
    TwentyFourPa { phase: u8 },
}

/// ヘルプの表に数値がある比率。アナモフィック 2:1 の数値は要確認。
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PixelAspect {
    Square,
    D1DvNtsc,
    D1DvNtscWidescreen,
    D1DvPal,
    D1DvPalWidescreen,
    Anamorphic2to1,
    Hdv1080,
    DvcproHd1080,
    Custom(f64),
}

impl PixelAspect {
    pub const NAMED: [Self; 8] = [
        Self::Square,
        Self::D1DvNtsc,
        Self::D1DvNtscWidescreen,
        Self::D1DvPal,
        Self::D1DvPalWidescreen,
        Self::Anamorphic2to1,
        Self::Hdv1080,
        Self::DvcproHd1080,
    ];

    /// 数値がヘルプにあるものだけ Some。
    pub fn ratio(self) -> Option<f64> {
        match self {
            Self::Square => Some(1.0),
            Self::D1DvNtsc => Some(0.91),
            Self::D1DvNtscWidescreen => Some(1.21),
            Self::D1DvPal => Some(1.09),
            Self::D1DvPalWidescreen => Some(1.46),
            Self::Anamorphic2to1 => None,
            Self::Hdv1080 => Some(1.33),
            Self::DvcproHd1080 => Some(1.5),
            Self::Custom(value) => Some(value),
        }
    }
}

/// 作業用スペースへ変換するときの入力プロファイル。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InputProfile {
    Srgb,
    HdtvRec709,
    AdobeRgb,
    ColorMatchRgb,
    ProPhotoRgb,
    /// RGB 以外。入力プロファイルは割り当てられない。
    NativeNonRgb { name: String },
    Other(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CinemaDngSpace {
    AdobeRgb,
    Srgb,
    ColorMatchRgb,
    ProPhotoRgb,
}

impl CinemaDngSpace {
    pub const ALL: [Self; 4] = [
        Self::AdobeRgb,
        Self::Srgb,
        Self::ColorMatchRgb,
        Self::ProPhotoRgb,
    ];
}

/// スライダの名前と範囲は要確認。色空間だけが確定している。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CinemaDngOptions {
    pub color_space: CinemaDngSpace,
}

/// 露出の初期値は要確認。None はソース設定を触っていない状態。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RedSourceSettings {
    pub exposure_stops: Option<f32>,
}

impl Default for RedSourceSettings {
    fn default() -> Self {
        Self {
            exposure_stops: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Interpretation {
    pub alpha: Option<AlphaInterpretation>,
    pub invert_alpha: bool,
    /// None はファイル側のフレームレート。
    pub conform_frame_rate: Option<f64>,
    pub fields: FieldSeparation,
    pub preserve_edges: bool,
    pub pulldown: Pulldown,
    /// 繰り返し回数。1 は繰り返さない。上限の規格値は要確認。
    pub loop_count: u32,
    pub pixel_aspect: PixelAspect,
    pub override_color_space: Option<InputProfile>,
    pub interpret_as_linear_light: bool,
    pub preserve_rgb: bool,
    pub cinema_dng: Option<CinemaDngOptions>,
    pub red: RedSourceSettings,
}

impl Default for Interpretation {
    fn default() -> Self {
        Self {
            alpha: None,
            invert_alpha: false,
            conform_frame_rate: None,
            fields: FieldSeparation::Off,
            preserve_edges: false,
            pulldown: Pulldown::Off,
            loop_count: 1,
            pixel_aspect: PixelAspect::Square,
            override_color_space: None,
            interpret_as_linear_light: false,
            preserve_rgb: false,
            cinema_dng: None,
            red: RedSourceSettings::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FootageProxy {
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<Rgba>,
    pub in_use: bool,
    pub still: bool,
    pub interpretation: Interpretation,
}

/// フッテージ項目。
#[derive(Clone, Debug, PartialEq)]
pub struct Footage {
    pub id: ItemId,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub duration: Time,
    pub frame_rate: f64,
    /// 音声の長さ。conform では変えない。
    pub audio_duration: Time,
    pub kind: FootageKind,
    pub pixels: Vec<Rgba>,
    pub interpretation: Interpretation,
    pub proxy: Option<FootageProxy>,
    pub missing: bool,
    pub path: String,
    /// ソースの開始タイムコードをフレーム数で持つ。無いときは None。
    pub media_start_frame: Option<i64>,
    pub label: Option<u8>,
}

impl Footage {
    pub fn file(
        id: ItemId,
        name: impl Into<String>,
        width: u32,
        height: u32,
        duration: Time,
        frame_rate: f64,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            width,
            height,
            duration,
            frame_rate,
            audio_duration: duration,
            kind: FootageKind::File,
            pixels: Vec::new(),
            interpretation: Interpretation::default(),
            proxy: None,
            missing: false,
            path: String::new(),
            media_start_frame: None,
            label: None,
        }
    }

    pub fn name_is_italic(&self) -> bool {
        self.missing
    }

    /// ループを掛けた尺。フレーム数はソースのフレーム数に回数を掛ける。
    pub fn looped_duration(&self) -> Time {
        let frames = self.duration.to_frames(self.frame_rate);
        let count = f64::from(self.interpretation.loop_count.max(1));
        Time::from_frames(frames * count, self.frame_rate)
    }

    /// conform 後の絵の尺。音声の尺は変えない。上限は 999 fps。
    pub fn conformed_picture_duration(&self) -> Option<Time> {
        let Some(fps) = self.interpretation.conform_frame_rate else {
            return Some(self.looped_duration());
        };
        if !(fps > 0.0 && fps <= 999.0) || self.frame_rate <= 0.0 {
            return None;
        }
        let frames = self.duration.to_frames(self.frame_rate) * f64::from(self.interpretation.loop_count.max(1));
        Some(Time::from_frames(frames, fps))
    }

    pub fn active_pixels(&self) -> (&[Rgba], u32, u32) {
        if let Some(proxy) = &self.proxy {
            if proxy.in_use {
                return (&proxy.pixels, self.width, self.height);
            }
        }
        (&self.pixels, self.width, self.height)
    }

    pub fn active_interpretation(&self) -> &Interpretation {
        if let Some(proxy) = &self.proxy {
            if proxy.in_use {
                return &proxy.interpretation;
            }
        }
        &self.interpretation
    }
}

pub fn solid_size_allowed(width: u32, height: u32) -> bool {
    width > 0 && height > 0 && width <= SOLID_MAX_PIXELS && height <= SOLID_MAX_PIXELS
}

/// RED のソース設定で露出を変えると、白を超えた値は残らない。
pub fn red_source_exposure(value: f32, stops: f32) -> f32 {
    (value * 2f32.powf(stops)).clamp(0.0, 1.0)
}

/// 32 bpc のプロジェクト側で露出を変えると、白を超えた値が残る。
pub fn project_exposure_32(value: f32, stops: f32) -> f32 {
    value * 2f32.powf(stops)
}
