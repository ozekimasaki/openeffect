use std::path::{Path, PathBuf};

/// フッテージの代わりに使うファイル。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Proxy {
    pub path: PathBuf,
    pub in_use: bool,
}

impl Proxy {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            in_use: true,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}
