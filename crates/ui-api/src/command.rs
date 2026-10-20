/// コマンドの安定名。メニューとショートカットは後からこの名前へ結ぶ。
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CommandId(String);

impl CommandId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Command {
    pub id: CommandId,
    pub title: String,
}

impl Command {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: CommandId::new(id),
            title: title.into(),
        }
    }
}
