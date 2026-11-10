/// プロジェクト内の項目。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ItemId(u64);

/// コンポジション内のレイヤー。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LayerId(u64);

/// プロパティ、マスク、シェイプノード。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PropertyId(u64);

impl ItemId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

impl LayerId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

impl PropertyId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

/// 実装クレートが項目を足すときに使う採番。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ids {
    next: u64,
}

impl Ids {
    pub const fn new() -> Self {
        Self { next: 1 }
    }

    pub fn item(&mut self) -> ItemId {
        ItemId::new(self.bump())
    }

    pub fn layer(&mut self) -> LayerId {
        LayerId::new(self.bump())
    }

    pub fn property(&mut self) -> PropertyId {
        PropertyId::new(self.bump())
    }

    fn bump(&mut self) -> u64 {
        let value = self.next;
        self.next = self.next.saturating_add(1);
        value
    }
}

impl Default for Ids {
    fn default() -> Self {
        Self::new()
    }
}
