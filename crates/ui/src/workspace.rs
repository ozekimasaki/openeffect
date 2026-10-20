//! 標準配置、パネルの開閉、コマンドとショートカット。

use ui_api::{Axis, Chord, Command, CommandId, Dock, DockNode, PanelId};

use crate::catalog::{self, ToolId};
use crate::dock;

#[derive(Clone, Debug, PartialEq)]
pub struct Workspace {
    dock: Dock,
    maximized: Option<PanelId>,
    restore: Option<DockNode>,
    active_tool: ToolId,
    last_command: Option<CommandId>,
}

impl Workspace {
    /// データにあるパネル一つを開いた配置。
    pub fn standard() -> Self {
        let panel = catalog::panels()[0].id.clone();
        let active_tool = catalog::tools()[0].id.clone();
        Self {
            dock: Dock::new(Dock::tabs(vec![panel])),
            maximized: None,
            restore: None,
            active_tool,
            last_command: None,
        }
    }

    pub fn dock(&self) -> &Dock {
        &self.dock
    }

    pub fn maximized(&self) -> Option<&PanelId> {
        self.maximized.as_ref()
    }

    pub fn active_tool(&self) -> &ToolId {
        &self.active_tool
    }

    pub fn last_command(&self) -> Option<&CommandId> {
        self.last_command.as_ref()
    }

    pub fn active_panel(&self) -> Option<&PanelId> {
        Dock::active_panel(&self.dock.root)
    }

    /// 開いていれば閉じ、閉じていればタブとして戻す。
    pub fn toggle_panel(&mut self, id: &PanelId) -> bool {
        if self.is_open(id) {
            self.close_panel(id)
        } else {
            self.open_panel(id.clone())
        }
    }

    pub fn close_panel(&mut self, id: &PanelId) -> bool {
        let in_view = dock::contains(&self.dock.root, id);
        let in_saved = self
            .restore
            .as_ref()
            .is_some_and(|node| dock::contains(node, id));
        if !in_view && !in_saved {
            return false;
        }
        if self.maximized.as_ref() == Some(id) {
            self.end_maximize();
        }
        let closed_view = if dock::contains(&self.dock.root, id) {
            dock::close(&mut self.dock.root, id)
        } else {
            false
        };
        if let Some(saved) = self.restore.as_mut() {
            dock::close(saved, id);
        }
        closed_view || in_saved
    }

    pub fn split_beside(&mut self, host: &PanelId, panel: PanelId, axis: Axis, ratio: f32) -> bool {
        self.resume_layout();
        dock::split_beside(&mut self.dock.root, host, panel, axis, ratio)
    }

    pub fn tabify(&mut self, host: &PanelId, panel: PanelId) -> bool {
        self.resume_layout();
        dock::tabify(&mut self.dock.root, host, panel)
    }

    pub fn float_panel(&mut self, id: &PanelId) -> bool {
        self.resume_layout();
        dock::float_panel(&mut self.dock.root, id)
    }

    pub fn float_frame(&mut self, id: &PanelId) -> bool {
        self.resume_layout();
        dock::float_frame(&mut self.dock.root, id)
    }

    pub fn set_ratio(&mut self, id: &PanelId, ratio: f32) -> bool {
        self.resume_layout();
        dock::set_ratio(&mut self.dock.root, id, ratio)
    }

    pub fn activate(&mut self, id: &PanelId) -> bool {
        dock::activate(&mut self.dock.root, id)
    }

    /// 枠をそのパネルだけにする。もう一度で戻す。
    pub fn toggle_maximized(&mut self, id: &PanelId) -> bool {
        if self.maximized.as_ref() == Some(id) {
            self.end_maximize();
            return true;
        }
        if self.maximized.is_some() {
            self.end_maximize();
        }
        if !dock::contains(&self.dock.root, id) {
            return false;
        }
        self.restore = Some(self.dock.root.clone());
        self.dock.root = Dock::tabs(vec![id.clone()]);
        self.maximized = Some(id.clone());
        true
    }

    /// パネル配置を標準へ戻す。選択中のツールは残す。
    pub fn reset(&mut self) {
        let active_tool = self.active_tool.clone();
        *self = Self::standard();
        self.active_tool = active_tool;
    }

    pub fn select_tool(&mut self, id: &ToolId) -> bool {
        let Some(tool) = catalog::tools().into_iter().find(|tool| &tool.id == id) else {
            return false;
        };
        self.active_tool = tool.id;
        true
    }

    /// 登録されたコマンドを受ける。ダイアログは出さない。
    pub fn invoke(&mut self, id: &CommandId) -> Option<Command> {
        let command = catalog::commands()
            .into_iter()
            .find(|command| &command.id == id)?;
        self.select_matching_tool(id);
        self.last_command = Some(command.id.clone());
        Some(command)
    }

    pub fn invoke_chord(&mut self, chord: &Chord) -> Option<Command> {
        let id = catalog::command_for(chord)?;
        self.invoke(&id)
    }

    fn is_open(&self, id: &PanelId) -> bool {
        dock::contains(&self.dock.root, id)
            || self
                .restore
                .as_ref()
                .is_some_and(|node| dock::contains(node, id))
    }

    fn open_panel(&mut self, id: PanelId) -> bool {
        self.resume_layout();
        if dock::contains(&self.dock.root, &id) {
            return false;
        }
        match dock::panel_ids(&self.dock.root).into_iter().next() {
            None => {
                self.dock.root = Dock::tabs(vec![id]);
                true
            }
            Some(host) => dock::tabify(&mut self.dock.root, &host, id),
        }
    }

    fn resume_layout(&mut self) {
        if self.maximized.is_some() {
            self.end_maximize();
        }
    }

    fn end_maximize(&mut self) {
        if let Some(saved) = self.restore.take() {
            self.dock.root = saved;
        }
        self.maximized = None;
    }

    fn select_matching_tool(&mut self, id: &CommandId) {
        for tool in catalog::tools() {
            if &tool.command.id == id {
                self.active_tool = tool.id;
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use ui_api::{Axis, Chord, CommandId, DockNode, Key, Modifiers, PanelId};

    use super::Workspace;
    use crate::catalog::{self, NEW_PROJECT, PROJECT, SELECT_TOOL};
    use crate::dock::{self, panel_ids};
    use crate::ToolId;

    fn project_id() -> PanelId {
        catalog::panels()[0].id.clone()
    }

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

    #[test]
    fn standard_layout_shows_the_project_panel() {
        let workspace = Workspace::standard();
        let ids = panel_ids(&workspace.dock().root);
        assert_eq!(ids.len(), 1);
        assert_eq!(ids[0].as_str(), PROJECT);
        assert_eq!(workspace.active_panel().map(PanelId::as_str), Some(PROJECT));
        assert!(workspace.maximized().is_none());
        assert_eq!(workspace.active_tool().as_str(), catalog::SELECTION);
    }

    #[test]
    fn shortcut_invokes_new_project() {
        let mut workspace = Workspace::standard();
        let command = workspace.invoke_chord(&ctrl_alt_n()).expect("command");
        assert_eq!(command.id.as_str(), NEW_PROJECT);
        assert_eq!(command.title, "新規プロジェクト");
        assert_eq!(
            workspace.last_command().map(CommandId::as_str),
            Some(NEW_PROJECT)
        );
        assert!(workspace.invoke(&CommandId::new("missing")).is_none());
    }

    #[test]
    fn toggle_project_panel() {
        let mut workspace = Workspace::standard();
        let id = project_id();
        assert!(workspace.toggle_panel(&id));
        assert!(panel_ids(&workspace.dock().root).is_empty());
        assert!(workspace.toggle_panel(&id));
        assert_eq!(panel_ids(&workspace.dock().root).as_slice(), &[id]);
    }

    #[test]
    fn selecting_the_tool() {
        let mut workspace = Workspace::standard();
        let tool = &catalog::tools()[0];
        assert!(!workspace.select_tool(&ToolId::new("missing")));
        assert!(workspace.select_tool(&tool.id));
        assert_eq!(workspace.active_tool().as_str(), catalog::SELECTION);
        let command = workspace
            .invoke(&CommandId::new(SELECT_TOOL))
            .expect("tool");
        assert_eq!(command.id.as_str(), SELECT_TOOL);
        assert_eq!(workspace.active_tool().as_str(), catalog::SELECTION);
    }

    #[test]
    fn close_collapses_a_split() {
        let mut workspace = Workspace::standard();
        let host = project_id();
        let extra = PanelId::new("dock-target");
        assert!(workspace.split_beside(&host, extra.clone(), Axis::Horizontal, 0.25));
        assert!(workspace.close_panel(&extra));
        match &workspace.dock().root {
            DockNode::Tabs { panels, active } => {
                assert_eq!(panels.as_slice(), &[host]);
                assert_eq!(*active, 0);
            }
            DockNode::Split { .. } | DockNode::Floating { .. } => {
                panic!("split should collapse");
            }
        }
    }

    #[test]
    fn split_tab_float_and_maximize() {
        let mut workspace = Workspace::standard();
        let host = project_id();
        let extra = PanelId::new("dock-target");
        assert!(workspace.split_beside(&host, extra.clone(), Axis::Vertical, 2.0));
        match &workspace.dock().root {
            DockNode::Split { axis, ratio, .. } => {
                assert_eq!(*axis, Axis::Vertical);
                assert_eq!(*ratio, 1.0);
            }
            DockNode::Tabs { .. } | DockNode::Floating { .. } => panic!("expected a split"),
        }
        assert!(workspace.set_ratio(&host, 0.25));
        match &workspace.dock().root {
            DockNode::Split { ratio, .. } => assert_eq!(*ratio, 0.25),
            DockNode::Tabs { .. } | DockNode::Floating { .. } => panic!("expected a split"),
        }
        assert!(workspace.tabify(&host, extra.clone()));
        match &workspace.dock().root {
            DockNode::Tabs { panels, active } => {
                assert_eq!(panels.as_slice(), &[host.clone(), extra.clone()]);
                assert_eq!(*active, 1);
            }
            DockNode::Split { .. } | DockNode::Floating { .. } => panic!("expected tabs"),
        }
        assert!(workspace.float_frame(&host));
        match &workspace.dock().root {
            DockNode::Floating { panels } => {
                assert_eq!(panels.as_slice(), &[host.clone(), extra.clone()]);
            }
            DockNode::Tabs { .. } | DockNode::Split { .. } => panic!("expected floating"),
        }
        assert!(workspace.float_panel(&extra));
        assert!(dock::contains(&workspace.dock().root, &host));
        assert!(dock::contains(&workspace.dock().root, &extra));
        assert!(workspace.toggle_maximized(&host));
        assert_eq!(workspace.maximized().map(PanelId::as_str), Some(PROJECT));
        assert_eq!(
            panel_ids(&workspace.dock().root).as_slice(),
            &[host.clone()]
        );
        assert!(workspace.toggle_maximized(&host));
        assert!(workspace.maximized().is_none());
        assert!(dock::contains(&workspace.dock().root, &extra));
        workspace.reset();
        assert_eq!(panel_ids(&workspace.dock().root).as_slice(), &[host]);
        assert!(workspace.maximized().is_none());
    }
}
