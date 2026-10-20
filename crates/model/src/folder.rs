use crate::id::ItemId;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FolderChild {
    Folder(Folder),
    Footage(ItemId),
    Composition(ItemId),
}

/// プロジェクトパネルのフォルダ。ディスク上のディレクトリではない。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Folder {
    pub id: ItemId,
    pub name: String,
    pub children: Vec<FolderChild>,
}

impl Folder {
    pub fn new(id: ItemId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            children: Vec::new(),
        }
    }
}
