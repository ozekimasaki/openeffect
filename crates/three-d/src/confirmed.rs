/// 公式ヘルプに数値が無い値。推測で埋めない。
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Confirmed<T> {
    Known(T),
    Unconfirmed,
}

impl<T: Eq> Eq for Confirmed<T> {}

impl<T> Confirmed<T> {
    pub const fn is_unconfirmed(&self) -> bool {
        matches!(self, Self::Unconfirmed)
    }

    pub const fn is_known(&self) -> bool {
        matches!(self, Self::Known(_))
    }

    pub fn known(self) -> Option<T> {
        match self {
            Self::Known(value) => Some(value),
            Self::Unconfirmed => None,
        }
    }

    pub const fn as_ref(&self) -> Confirmed<&T> {
        match self {
            Self::Known(value) => Confirmed::Known(value),
            Self::Unconfirmed => Confirmed::Unconfirmed,
        }
    }
}

/// キーフレームの可否。公式が書いていないものは未確定。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Keyframes {
    Allowed,
    Forbidden,
    Unconfirmed,
}

/// 範囲外。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RangeError;

/// `.aep` / `.aepx` の格納位置は公式ヘルプに無い。
pub const fn project_file_location() -> Confirmed<&'static str> {
    Confirmed::Unconfirmed
}
