//! メニュー、パネル、ドック、ツール、ショートカット。仕様のチェックをすべて置く。

#[rustfmt::skip]
#[path = "checks_data.rs"]
mod checks_data;

use ui_api::{Chord, Command, CommandId, Panel, Shortcut};

pub use checks_data::Binding;

pub const NEW_PROJECT: &str = "file.new-project";
pub const PROJECT: &str = "project";
pub const SELECTION: &str = "selection";
pub const SELECT_TOOL: &str = "tool.selection";

/// 仕様の節。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Menu,
    Panel,
    Dock,
    Tool,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Menu => "menu",
            Kind::Panel => "panel",
            Kind::Dock => "dock",
            Kind::Tool => "tool",
        }
    }
}

/// 仕様のチェック一つ。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Check {
    pub id: &'static str,
    pub kind: Kind,
    pub group: &'static str,
    pub title: &'static str,
    pub behavior: &'static str,
    pub parameters: &'static str,
    pub location: &'static str,
    pub shortcut: &'static str,
    pub acceptance: &'static str,
}

/// メニューの一つの項目。場所は仕様の文字列。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuItem {
    pub command: Command,
    pub location: &'static str,
}

impl MenuItem {
    /// `>` で区切った場所。並記された場所は `location` を見る。
    pub fn path(&self) -> Vec<&str> {
        if self.location.contains(" > ") {
            self.location.split(" > ").collect()
        } else {
            vec![self.location]
        }
    }
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

/// ウィンドウメニューから開くパネル。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PanelWindow {
    pub panel: Panel,
    pub open_command: &'static str,
}

pub fn checks() -> &'static [Check] {
    checks_data::CHECKS
}

pub fn bindings() -> Vec<Binding> {
    checks_data::bindings()
}

pub fn menu_items() -> Vec<MenuItem> {
    checks()
        .iter()
        .filter(|check| check.kind == Kind::Menu)
        .map(|check| MenuItem {
            command: Command::new(check.id, check.title),
            location: check.location,
        })
        .collect()
}

pub fn panel_windows() -> Vec<PanelWindow> {
    let mut windows = Vec::new();
    for check in checks() {
        if check.kind != Kind::Menu {
            continue;
        }
        let Some(name) = check.title.strip_suffix("パネルを開閉") else {
            continue;
        };
        let panel = if name == "プロジェクト" {
            Panel::new(PROJECT, "プロジェクト")
        } else {
            Panel::new(format!("window.{name}"), name)
        };
        windows.push(PanelWindow {
            panel,
            open_command: check.id,
        });
    }
    windows.sort_by_key(|window| window.panel.id.as_str() != PROJECT);
    windows
}

pub fn panels() -> Vec<Panel> {
    panel_windows()
        .into_iter()
        .map(|window| window.panel)
        .collect()
}

pub fn tools() -> Vec<Tool> {
    checks()
        .iter()
        .filter(|check| check.kind == Kind::Tool)
        .map(|check| {
            if check.id == SELECT_TOOL {
                Tool::new(SELECTION, check.title, SELECT_TOOL)
            } else {
                Tool::new(check.title, check.title, check.id)
            }
        })
        .collect()
}

pub fn shortcuts() -> Vec<Shortcut> {
    checks()
        .iter()
        .zip(bindings())
        .flat_map(|(check, binding)| {
            let id = check.id;
            binding
                .alternatives
                .into_iter()
                .map(move |chord| Shortcut::new(CommandId::new(id), chord))
        })
        .collect()
}

pub fn commands() -> Vec<Command> {
    checks()
        .iter()
        .map(|check| Command::new(check.id, check.title))
        .collect()
}

pub fn command(id: &CommandId) -> Option<Command> {
    checks()
        .iter()
        .find(|check| check.id == id.as_str())
        .map(|check| Command::new(check.id, check.title))
}

pub fn commands_for(chord: &Chord) -> Vec<CommandId> {
    checks()
        .iter()
        .zip(bindings())
        .filter_map(|(check, binding)| {
            binding
                .alternatives
                .iter()
                .any(|candidate| candidate == chord)
                .then(|| CommandId::new(check.id))
        })
        .collect()
}

pub fn command_for(chord: &Chord) -> Option<CommandId> {
    commands_for(chord).into_iter().next()
}

pub fn command_for_sequence(steps: &[Chord]) -> Option<CommandId> {
    checks()
        .iter()
        .zip(bindings())
        .find_map(|(check, binding)| (binding.sequence == steps).then(|| CommandId::new(check.id)))
}

pub fn project_panel() -> Panel {
    Panel::new(PROJECT, "プロジェクト")
}

#[cfg(test)]
mod tests {
    use ui_api::{Chord, CommandId, Key, Modifiers};

    use super::{
        bindings, checks, command_for, command_for_sequence, commands_for, menu_items, panels,
        shortcuts, tools, Kind, NEW_PROJECT, PROJECT, SELECTION, SELECT_TOOL,
    };

    const GESTURES: &[&str] = &[
        "環境設定の初期キーを戻す",
        "マーカーを削除",
        "ビット深度を循環",
        "表示中のプロパティへ追加",
        "プロパティを隠す",
        "ブラシの直径",
        "ブラシの硬さ",
        "マーカー間の長さ",
    ];

    fn ctrl_alt_n() -> Chord {
        Chord::new(
            Key::Character('N'),
            Modifiers {
                ctrl: true,
                alt: true,
                shift: false,
                meta: false,
            },
        )
    }

    fn plain(ch: char) -> Chord {
        Chord::new(ch_key(ch), Modifiers::default())
    }

    fn ch_key(ch: char) -> Key {
        Key::Character(ch)
    }

    #[test]
    fn menu_item_is_new_project() {
        let items = menu_items();
        assert_eq!(items.len(), 806);
        assert_eq!(items[0].command.id.as_str(), NEW_PROJECT);
        assert_eq!(items[0].command.title, "新規プロジェクト");
        assert_eq!(items[0].location, "ファイル > 新規 > 新規プロジェクト");
        assert_eq!(
            items[0].path(),
            ["ファイル", "新規", "新規プロジェクト"].as_slice()
        );
    }

    #[test]
    fn panel_is_project() {
        let panels = panels();
        assert_eq!(panels.len(), 28);
        assert_eq!(panels[0].id.as_str(), PROJECT);
        assert_eq!(panels[0].title, "プロジェクト");
    }

    #[test]
    fn tool_is_selection() {
        let tools = tools();
        assert_eq!(tools.len(), 36);
        assert_eq!(tools[0].id.as_str(), SELECTION);
        assert_eq!(tools[0].title, "選択ツール");
        assert_eq!(tools[0].command.id.as_str(), SELECT_TOOL);
        assert_eq!(tools[0].command.title, "選択ツール");
    }

    #[test]
    fn shortcut_is_ctrl_alt_n() {
        let matched: Vec<_> = shortcuts()
            .into_iter()
            .filter(|shortcut| shortcut.command.as_str() == NEW_PROJECT)
            .collect();
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].chord.key, Key::Character('N'));
        assert!(matched[0].chord.modifiers.ctrl);
        assert!(matched[0].chord.modifiers.alt);
        assert!(!matched[0].chord.modifiers.shift);
        assert!(!matched[0].chord.modifiers.meta);
        assert_eq!(
            command_for(&ctrl_alt_n()).expect("shortcut").as_str(),
            NEW_PROJECT
        );
        assert_eq!(commands_for(&ctrl_alt_n()).len(), 1);
        let without_alt = Chord::new(
            Key::Character('N'),
            Modifiers {
                ctrl: true,
                alt: false,
                shift: false,
                meta: false,
            },
        );
        assert!(commands_for(&without_alt)
            .iter()
            .all(|id| id.as_str() != NEW_PROJECT));
    }

    #[test]
    fn spec_counts_match_the_checklist() {
        assert_eq!(checks().len(), 1090);
        assert_eq!(bindings().len(), 1090);
        assert_eq!(count(Kind::Menu), 806);
        assert_eq!(count(Kind::Panel), 236);
        assert_eq!(count(Kind::Dock), 12);
        assert_eq!(count(Kind::Tool), 36);
        assert_eq!(checks()[0].title, "新規プロジェクト");
        assert_eq!(checks().last().expect("last").title, "ユニバーサルギズモ");
        assert_eq!(
            checks()
                .iter()
                .filter(|check| check.shortcut == "要確認")
                .count(),
            766
        );
        assert_eq!(
            checks()
                .iter()
                .filter(|check| check.shortcut.starts_with("なし"))
                .count(),
            15
        );
        assert_eq!(
            checks()
                .iter()
                .filter(|check| check.shortcut != "要確認" && !check.shortcut.starts_with("なし"))
                .count(),
            309
        );
    }

    #[test]
    fn every_check_matches_the_spec_list() {
        let golden = include_str!("spec_checks.golden");
        let mut lines = golden.lines();
        for check in checks() {
            let line = lines
                .next()
                .unwrap_or_else(|| panic!("missing {}", check.title));
            let fields: Vec<&str> = line.split('\t').collect();
            assert_eq!(fields.len(), 8, "{}", check.title);
            assert_eq!(fields[0], check.kind.as_str());
            assert_eq!(fields[1], check.group);
            assert_eq!(fields[2], check.title);
            assert_eq!(fields[3], check.behavior);
            assert_eq!(fields[4], check.parameters);
            assert_eq!(fields[5], check.location);
            assert_eq!(fields[6], check.shortcut);
            assert_eq!(fields[7], check.acceptance);
            assert!(!check.behavior.is_empty());
            assert!(!check.parameters.is_empty());
            assert!(!check.location.is_empty());
            assert!(!check.shortcut.is_empty());
            assert!(!check.acceptance.is_empty());
        }
        assert!(lines.next().is_none());
    }

    #[test]
    fn shortcuts_cover_assigned_keys_and_leave_gestures() {
        let mut keyed = 0;
        let mut gestures = Vec::new();
        for (check, binding) in checks().iter().zip(bindings()) {
            let assigned = check.shortcut != "要確認" && !check.shortcut.starts_with("なし");
            let has_chord = !binding.alternatives.is_empty() || !binding.sequence.is_empty();
            if !assigned {
                assert!(!has_chord, "{}", check.title);
            } else if has_chord {
                keyed += 1;
            } else {
                gestures.push(check.title);
            }
        }
        assert_eq!(gestures, GESTURES);
        assert_eq!(keyed, 301);
    }

    #[test]
    fn repeated_keys_are_sequences() {
        let mut sequences = 0;
        for (check, binding) in checks().iter().zip(bindings()) {
            if binding.sequence.is_empty() {
                continue;
            }
            sequences += 1;
            assert_eq!(binding.sequence.len(), 2, "{}", check.shortcut);
            assert_eq!(check.shortcut.chars().count(), 2);
            let mut chars = check.shortcut.chars();
            assert_eq!(chars.next(), chars.next());
        }
        assert_eq!(sequences, 10);
        let single = checks()
            .iter()
            .find(|check| check.shortcut == "U")
            .expect("U");
        let double = checks()
            .iter()
            .find(|check| check.shortcut == "UU")
            .expect("UU");
        let ids = commands_for(&plain('U'));
        assert!(ids.iter().any(|id| id.as_str() == single.id));
        assert!(ids.iter().all(|id| id.as_str() != double.id));
        let step = plain('U');
        assert_eq!(
            command_for_sequence(&[step.clone(), step])
                .expect("UU")
                .as_str(),
            double.id
        );
    }

    #[test]
    fn shared_shortcuts_keep_every_command() {
        let vee = commands_for(&plain('V'));
        let titles = titles_of(&vee);
        assert!(titles.contains(&"選択ツール"));
        assert!(titles.contains(&"ユニバーサルギズモ"));
        let find = Chord::new(
            Key::Character('F'),
            Modifiers {
                ctrl: true,
                alt: false,
                shift: false,
                meta: false,
            },
        );
        let found = commands_for(&find);
        let titles = titles_of(&found);
        assert!(titles.contains(&"プロジェクトを検索"));
        assert!(titles.contains(&"タイムラインを検索"));
    }

    fn titles_of(ids: &[CommandId]) -> Vec<&str> {
        ids.iter()
            .map(|id| {
                checks()
                    .iter()
                    .find(|check| check.id == id.as_str())
                    .expect("id")
                    .title
            })
            .collect()
    }

    fn count(kind: Kind) -> usize {
        checks().iter().filter(|check| check.kind == kind).count()
    }
}
