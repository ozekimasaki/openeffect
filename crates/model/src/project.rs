use crate::bits::BitsPerChannel;
use crate::composition::Composition;
use crate::folder::{Folder, FolderChild};
use crate::footage::Footage;
use crate::id::{Ids, ItemId};

/// プロジェクト。色深度は 8、16、32 のいずれか。
#[derive(Clone, Debug, PartialEq)]
pub struct Project {
    pub bits_per_channel: BitsPerChannel,
    pub root: Folder,
    pub footage: Vec<Footage>,
    pub compositions: Vec<Composition>,
    ids: Ids,
}

impl Project {
    pub fn new(bits_per_channel: BitsPerChannel) -> Self {
        let mut ids = Ids::new();
        let root = Folder::new(ids.item(), "Root");
        Self {
            bits_per_channel,
            root,
            footage: Vec::new(),
            compositions: Vec::new(),
            ids,
        }
    }

    pub fn alloc_item(&mut self) -> ItemId {
        self.ids.item()
    }

    pub fn alloc_layer(&mut self) -> crate::id::LayerId {
        self.ids.layer()
    }

    pub fn alloc_property(&mut self) -> crate::id::PropertyId {
        self.ids.property()
    }

    pub fn add_footage(&mut self, footage: Footage) {
        let id = footage.id;
        self.footage.push(footage);
        self.root.children.push(FolderChild::Footage(id));
    }

    pub fn add_composition(&mut self, composition: Composition) {
        let id = composition.id;
        self.compositions.push(composition);
        self.root.children.push(FolderChild::Composition(id));
    }

    pub fn add_folder(&mut self, folder: Folder) {
        self.root.children.push(FolderChild::Folder(folder));
    }

    pub fn footage(&self, id: ItemId) -> Option<&Footage> {
        self.footage.iter().find(|item| item.id == id)
    }

    pub fn footage_mut(&mut self, id: ItemId) -> Option<&mut Footage> {
        self.footage.iter_mut().find(|item| item.id == id)
    }

    pub fn composition(&self, id: ItemId) -> Option<&Composition> {
        self.compositions.iter().find(|item| item.id == id)
    }

    pub fn composition_mut(&mut self, id: ItemId) -> Option<&mut Composition> {
        self.compositions.iter_mut().find(|item| item.id == id)
    }
}
