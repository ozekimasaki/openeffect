use crate::panel::PanelId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Axis {
    Horizontal,
    Vertical,
}

/// ドックの木。タブ、分割、フローティング。
#[derive(Clone, Debug, PartialEq)]
pub enum DockNode {
    Split {
        axis: Axis,
        ratio: f32,
        first: Box<DockNode>,
        second: Box<DockNode>,
    },
    Tabs {
        panels: Vec<PanelId>,
        active: usize,
    },
    Floating {
        panels: Vec<PanelId>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Dock {
    pub root: DockNode,
}

impl Dock {
    pub fn new(root: DockNode) -> Self {
        Self { root }
    }

    pub fn tabs(panels: Vec<PanelId>) -> DockNode {
        DockNode::Tabs { panels, active: 0 }
    }

    /// 比率は 0 から 1 に収める。
    pub fn split(axis: Axis, ratio: f32, first: DockNode, second: DockNode) -> DockNode {
        DockNode::Split {
            axis,
            ratio: ratio.clamp(0.0, 1.0),
            first: Box::new(first),
            second: Box::new(second),
        }
    }

    pub fn active_panel(node: &DockNode) -> Option<&PanelId> {
        match node {
            DockNode::Tabs { panels, active } => panels.get(*active),
            DockNode::Split { first, second, .. } => {
                Self::active_panel(first).or_else(|| Self::active_panel(second))
            }
            DockNode::Floating { panels } => panels.first(),
        }
    }
}
