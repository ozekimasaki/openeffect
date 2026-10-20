//! メニュー、パネル、ツール、ショートカットを一つずつ置く。

use ui_api::{Chord, Command, CommandId, Key, Modifiers, Panel, Shortcut};

pub const NEW_PROJECT: &str = "file.new-project";
pub const PROJECT: &str = "project";
pub const SELECTION: &str = "selection";
pub const SELECT_TOOL: &str = "tool.selection";

const NEW_PROJECT_LOCATION: &[&str] = &["ファイル", "新規", "新規プロジェクト"];

/// ツールの安定名。
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ToolId(String);

impl ToolId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// メニューの一つの項目。場所は根からその項目までの名前。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuItem {
    pub command: Command,
    pub location: &'static [&'static str],
}

/// ツールパネルの一つのツール。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tool {
    pub id: ToolId,
    pub title: String,
    pub command: Command,
}

impl Tool {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        command_id: impl Into<String>,
    ) -> Self {
        let title = title.into();
        Self {
            id: ToolId::new(id),
            title: title.clone(),
            command: Command::new(command_id, title),
        }
    }
}

pub fn menu_items() -> [MenuItem; 1] {
    [MenuItem {
        command: Command::new(NEW_PROJECT, "新規プロジェクト"),
        location: NEW_PROJECT_LOCATION,
    }]
}

pub fn panels() -> [Panel; 1] {
    [Panel::new(PROJECT, "プロジェクト")]
}

pub fn tools() -> [Tool; 1] {
    [Tool::new(SELECTION, "選択ツール", SELECT_TOOL)]
}

/// 新規プロジェクト。Ctrl+Alt+N。
pub fn shortcuts() -> [Shortcut; 1] {
    [Shortcut::new(
        CommandId::new(NEW_PROJECT),
        Chord::new(
            Key::Character('N'),
            Modifiers {
                ctrl: true,
                alt: true,
                shift: false,
                meta: false,
            },
        ),
    )]
}

pub fn command_for(chord: &Chord) -> Option<CommandId> {
    shortcuts()
        .into_iter()
        .find(|shortcut| &shortcut.chord == chord)
        .map(|shortcut| shortcut.command)
}

pub fn commands() -> Vec<Command> {
    menu_items()
        .into_iter()
        .map(|item| item.command)
        .chain(tools().into_iter().map(|tool| tool.command))
        .collect()
}

#[cfg(test)]
mod tests {
    use ui_api::{Chord, Key, Modifiers};

    use super::{
        command_for, menu_items, panels, shortcuts, tools, NEW_PROJECT, PROJECT, SELECTION,
        SELECT_TOOL,
    };

    #[test]
    fn menu_item_is_new_project() {
        let items = menu_items();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].command.id.as_str(), NEW_PROJECT);
        assert_eq!(items[0].command.title, "新規プロジェクト");
        assert_eq!(
            items[0].location,
            ["ファイル", "新規", "新規プロジェクト"].as_slice()
        );
    }

    #[test]
    fn panel_is_project() {
        let panels = panels();
        assert_eq!(panels.len(), 1);
        assert_eq!(panels[0].id.as_str(), PROJECT);
        assert_eq!(panels[0].title, "プロジェクト");
    }

    #[test]
    fn tool_is_selection() {
        let tools = tools();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].id.as_str(), SELECTION);
        assert_eq!(tools[0].title, "選択ツール");
        assert_eq!(tools[0].command.id.as_str(), SELECT_TOOL);
        assert_eq!(tools[0].command.title, "選択ツール");
    }

    #[test]
    fn shortcut_is_ctrl_alt_n() {
        let shortcuts = shortcuts();
        assert_eq!(shortcuts.len(), 1);
        assert_eq!(shortcuts[0].command.as_str(), NEW_PROJECT);
        assert_eq!(shortcuts[0].chord.key, Key::Character('N'));
        assert!(shortcuts[0].chord.modifiers.ctrl);
        assert!(shortcuts[0].chord.modifiers.alt);
        assert!(!shortcuts[0].chord.modifiers.shift);
        assert!(!shortcuts[0].chord.modifiers.meta);

        let chord = Chord::new(
            Key::Character('N'),
            Modifiers {
                ctrl: true,
                alt: true,
                shift: false,
                meta: false,
            },
        );
        assert_eq!(command_for(&chord).expect("shortcut").as_str(), NEW_PROJECT);
        let without_alt = Chord::new(
            Key::Character('N'),
            Modifiers {
                ctrl: true,
                alt: false,
                shift: false,
                meta: false,
            },
        );
        assert!(command_for(&without_alt).is_none());
    }
}
