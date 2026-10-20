use crate::command::CommandId;

/// ヒストリの 1 手。中身の巻き戻しは実装クレートが行う。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoryEntry {
    pub command: CommandId,
    pub label: String,
}

impl HistoryEntry {
    pub fn new(command: CommandId, label: impl Into<String>) -> Self {
        Self {
            command,
            label: label.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct History {
    undo: Vec<HistoryEntry>,
    redo: Vec<HistoryEntry>,
}

impl History {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&mut self, entry: HistoryEntry) {
        self.undo.push(entry);
        self.redo.clear();
    }

    pub fn undo(&mut self) -> Option<HistoryEntry> {
        let entry = self.undo.pop()?;
        self.redo.push(entry.clone());
        Some(entry)
    }

    pub fn redo(&mut self) -> Option<HistoryEntry> {
        let entry = self.redo.pop()?;
        self.undo.push(entry.clone());
        Some(entry)
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::{History, HistoryEntry};
    use crate::command::CommandId;

    #[test]
    fn undo_then_redo() {
        let mut history = History::new();
        history.record(HistoryEntry::new(CommandId::new("edit.opacity"), "opacity"));
        let undone = history.undo().expect("undo");
        assert_eq!(undone.command.as_str(), "edit.opacity");
        assert!(!history.can_undo());
        assert!(history.can_redo());
        let redone = history.redo().expect("redo");
        assert_eq!(redone.label, "opacity");
    }
}
