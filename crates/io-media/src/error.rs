use std::fmt;

/// 読み込みと書き出しの失敗。
#[derive(Debug)]
pub enum MediaError {
    Io(std::io::Error),
    Decode(String),
    Encode(String),
    Unsupported { kind: String, detail: &'static str },
    Invalid(String),
    FrameRate(f64),
}

impl fmt::Display for MediaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::Decode(message) => write!(f, "{message}"),
            Self::Encode(message) => write!(f, "{message}"),
            Self::Unsupported { kind, detail } => write!(f, "{kind}: {detail}"),
            Self::Invalid(message) => write!(f, "{message}"),
            Self::FrameRate(value) => write!(f, "フレームレート {value} は 0 より大きく 999 以下"),
        }
    }
}

impl std::error::Error for MediaError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Decode(_)
            | Self::Encode(_)
            | Self::Unsupported { .. }
            | Self::Invalid(_)
            | Self::FrameRate(_) => None,
        }
    }
}

impl From<std::io::Error> for MediaError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<image::ImageError> for MediaError {
    fn from(error: image::ImageError) -> Self {
        Self::Decode(error.to_string())
    }
}

impl From<tiff::TiffError> for MediaError {
    fn from(error: tiff::TiffError) -> Self {
        Self::Decode(error.to_string())
    }
}

impl From<exr::error::Error> for MediaError {
    fn from(error: exr::error::Error) -> Self {
        Self::Decode(error.to_string())
    }
}
