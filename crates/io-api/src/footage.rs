use std::path::{Path, PathBuf};

use model::ItemId;

use crate::proxy::Proxy;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LinkState {
    Linked,
    Unlinked,
}

/// フッテージ項目とファイルの対応。未リンクでも最後のパスは残る。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FootageReference {
    pub item: ItemId,
    pub path: PathBuf,
    pub link: LinkState,
    pub proxy: Option<Proxy>,
}

impl FootageReference {
    pub fn linked(item: ItemId, path: impl Into<PathBuf>) -> Self {
        Self {
            item,
            path: path.into(),
            link: LinkState::Linked,
            proxy: None,
        }
    }

    pub fn unlinked(item: ItemId, path: impl Into<PathBuf>) -> Self {
        Self {
            item,
            path: path.into(),
            link: LinkState::Unlinked,
            proxy: None,
        }
    }

    pub fn is_unlinked(&self) -> bool {
        matches!(self.link, LinkState::Unlinked)
    }

    pub fn set_unlinked(&mut self) {
        self.link = LinkState::Unlinked;
    }

    pub fn relink(&mut self, path: impl Into<PathBuf>) {
        self.path = path.into();
        self.link = LinkState::Linked;
    }

    pub fn set_proxy(&mut self, proxy: Proxy) {
        self.proxy = Some(proxy);
    }

    pub fn clear_proxy(&mut self) {
        self.proxy = None;
    }

    pub fn set_proxy_in_use(&mut self, in_use: bool) -> bool {
        match &mut self.proxy {
            Some(proxy) => {
                proxy.in_use = in_use;
                true
            }
            None => false,
        }
    }

    /// 使用中のプロキシがあればそのパス。未リンクでプロキシが無ければ None。
    pub fn active_path(&self) -> Option<&Path> {
        if let Some(proxy) = &self.proxy {
            if proxy.in_use {
                return Some(proxy.path());
            }
        }
        match self.link {
            LinkState::Linked => Some(self.path.as_path()),
            LinkState::Unlinked => None,
        }
    }

    pub fn last_path(&self) -> &Path {
        self.path.as_path()
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;
    use std::path::Path;

    use super::FootageReference;
    use model::ItemId;

    #[test]
    fn unlinked_keeps_the_last_path() {
        let mut reference = FootageReference::linked(ItemId::new(1), "/media/shot.png");
        reference.set_unlinked();
        assert!(reference.is_unlinked());
        assert!(reference.active_path().is_none());
        assert_eq!(reference.last_path().as_os_str(), "/media/shot.png");
        reference.relink("/media/shot-fixed.png");
        assert!(!reference.is_unlinked());
        assert_eq!(
            reference.active_path().map(Path::as_os_str),
            Some(OsStr::new("/media/shot-fixed.png"))
        );
    }
}
