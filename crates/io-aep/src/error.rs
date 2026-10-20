use std::fmt;

use crate::FourCc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Truncated,
    BadMagic,
    BadFileSize { declared: u32, actual: u64 },
    UnsupportedForm(FourCc),
    BadList,
    TooDeep,
    TooLarge,
    BadHex,
    BadNumber,
    Xml(String),
    XmlName(FourCc),
    UnmappedXml(String),
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Truncated => write!(formatter, "チャンクが途中で切れています"),
            Error::BadMagic => write!(formatter, "先頭が RIFX ではありません"),
            Error::BadFileSize { declared, actual } => {
                write!(
                    formatter,
                    "ファイルサイズが一致しません: 宣言 {declared}、実際 {actual}"
                )
            }
            Error::UnsupportedForm(form) => {
                write!(formatter, "フォームタイプ {form} は読みません")
            }
            Error::BadList => write!(formatter, "LIST のタイプが足りません"),
            Error::TooDeep => write!(formatter, "チャンクの入れ子が深すぎます"),
            Error::TooLarge => write!(formatter, "チャンクが大きすぎます"),
            Error::BadHex => write!(formatter, "bdata の 16 進が不正です"),
            Error::BadNumber => write!(formatter, "数値が読み取れません"),
            Error::Xml(message) => write!(formatter, "XML を読めません: {message}"),
            Error::XmlName(id) => {
                write!(formatter, "fourcc {id} の .aepx 要素名は未確定です")
            }
            Error::UnmappedXml(name) => {
                write!(formatter, "要素 {name} に対応する fourcc は未確定です")
            }
        }
    }
}

impl std::error::Error for Error {}
