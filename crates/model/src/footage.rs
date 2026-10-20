use crate::color::Rgba;
use crate::id::ItemId;
use crate::time::Time;

/// フッテージ項目の中身。パスは io-api が持つ。
#[derive(Clone, Debug, PartialEq)]
pub enum FootageKind {
    File,
    Solid { color: Rgba },
    Placeholder,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Footage {
    pub id: ItemId,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub duration: Time,
    pub frame_rate: f64,
    pub kind: FootageKind,
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
            kind: FootageKind::File,
        }
    }
}
