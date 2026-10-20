use crate::color::Rgba;

/// ポイントテキストは折り返さない。段落テキストはボックス幅で折り返す。
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TextLayout {
    Point,
    Paragraph { width: f64, height: f64 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum TextOrientation {
    #[default]
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum ParagraphAlignment {
    #[default]
    Left,
    Center,
    Right,
    JustifyLeft,
    JustifyCenter,
    JustifyRight,
    JustifyAll,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextStyle {
    pub font_family: String,
    pub font_style: String,
    pub size: f64,
    pub fill: Rgba,
    pub faux_bold: bool,
    pub faux_italic: bool,
    pub alignment: ParagraphAlignment,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font_family: String::new(),
            font_style: String::new(),
            size: 0.0,
            fill: Rgba::white(),
            faux_bold: false,
            faux_italic: false,
            alignment: ParagraphAlignment::Left,
        }
    }
}

/// ソーステキスト。書式の時間変化はプロパティ側のキーに載せる。
#[derive(Clone, Debug, PartialEq)]
pub struct Text {
    pub source: String,
    pub layout: TextLayout,
    pub orientation: TextOrientation,
    pub style: TextStyle,
}

impl Text {
    pub fn point(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            layout: TextLayout::Point,
            orientation: TextOrientation::Horizontal,
            style: TextStyle::default(),
        }
    }

    pub fn paragraph(source: impl Into<String>, width: f64, height: f64) -> Self {
        Self {
            source: source.into(),
            layout: TextLayout::Paragraph { width, height },
            orientation: TextOrientation::Horizontal,
            style: TextStyle::default(),
        }
    }
}

/// テキストアニメーター。セレクタの種類名は実装が後から足す。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextAnimator {
    pub name: String,
    pub selector: String,
}

impl TextAnimator {
    pub fn new(name: impl Into<String>, selector: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            selector: selector.into(),
        }
    }
}
