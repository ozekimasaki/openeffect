/// パネルの安定名。メニュー項目は後からこの名前で開く。
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PanelId(String);

impl PanelId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Panel {
    pub id: PanelId,
    pub title: String,
}

impl Panel {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: PanelId::new(id),
            title: title.into(),
        }
    }
}
