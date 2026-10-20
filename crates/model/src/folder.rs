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
    pub label: Option<u8>,
}

impl Folder {
    pub fn new(id: ItemId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            children: Vec::new(),
            label: None,
        }
    }

    pub fn child_folder(&self, name: &str) -> Option<&Folder> {
        self.children.iter().find_map(|child| match child {
            FolderChild::Folder(folder) if folder.name == name => Some(folder),
            FolderChild::Folder(folder) => folder.child_folder(name),
            FolderChild::Footage(_) | FolderChild::Composition(_) => None,
        })
    }

    pub fn child_folder_mut(&mut self, name: &str) -> Option<&mut Folder> {
        let index = self.children.iter().position(|child| match child {
            FolderChild::Folder(folder) => folder.name == name || folder.child_folder(name).is_some(),
            FolderChild::Footage(_) | FolderChild::Composition(_) => false,
        })?;
        let FolderChild::Folder(folder) = &mut self.children[index] else {
            return None;
        };
        if folder.name == name {
            Some(folder)
        } else {
            folder.child_folder_mut(name)
        }
    }
}
