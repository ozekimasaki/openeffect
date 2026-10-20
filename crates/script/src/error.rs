/// スクリプティングの失敗。未対応、範囲外、読み取り専用を区別する。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptError {
    pub message: String,
}

impl ScriptError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn unsupported(what: impl std::fmt::Display) -> Self {
        Self::new(format!("{what} は未対応"))
    }

    pub fn readonly(what: impl std::fmt::Display) -> Self {
        Self::new(format!("{what} は読み取り専用"))
    }

    pub fn range(what: impl std::fmt::Display) -> Self {
        Self::new(format!("範囲外: {what}"))
    }

    pub fn missing(what: impl std::fmt::Display) -> Self {
        Self::new(format!("見つからない: {what}"))
    }

    pub fn invalid(what: impl std::fmt::Display) -> Self {
        Self::new(format!("不正: {what}"))
    }

    pub fn unconfirmed(what: impl std::fmt::Display) -> Self {
        Self::new(format!("要確認のまま未対応: {what}"))
    }
}

pub type ScriptResult<T> = Result<T, ScriptError>;
