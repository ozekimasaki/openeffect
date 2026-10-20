//! ドックの木を組み替える。フローティングも木の節であり、ウィンドウは開かない。

use std::mem;

use ui_api::{Axis, Dock, DockNode, PanelId};

pub fn contains(node: &DockNode, id: &PanelId) -> bool {
    match node {
        DockNode::Tabs { panels, .. } | DockNode::Floating { panels } => panels.contains(id),
        DockNode::Split { first, second, .. } => contains(first, id) || contains(second, id),
    }
}

pub fn is_empty(node: &DockNode) -> bool {
    match node {
        DockNode::Tabs { panels, .. } | DockNode::Floating { panels } => panels.is_empty(),
        DockNode::Split { first, second, .. } => is_empty(first) && is_empty(second),
    }
}

pub fn panel_ids(node: &DockNode) -> Vec<PanelId> {
    let mut ids = Vec::new();
    collect(node, &mut ids);
    ids
}

/// パネルを枠から外す。空になった分割は隣の枠へ畳む。
pub fn close(node: &mut DockNode, id: &PanelId) -> bool {
    take(node, id).is_some()
}

/// タブの前面をそのパネルにする。
pub fn activate(node: &mut DockNode, id: &PanelId) -> bool {
    match node {
        DockNode::Tabs { panels, active } => {
            let Some(index) = panels.iter().position(|panel| panel == id) else {
                return false;
            };
            *active = index;
            true
        }
        DockNode::Floating { panels } => move_to_front(panels, id),
        DockNode::Split { first, second, .. } => activate(first, id) || activate(second, id),
    }
}

/// ホストの枠へタブとして積む。積んだタブを前面にする。
pub fn tabify(node: &mut DockNode, host: &PanelId, panel: PanelId) -> bool {
    if host == &panel || !contains(node, host) {
        return false;
    }
    let panel = take(node, &panel).unwrap_or(panel);
    if !contains(node, host) {
        place_orphaned(node, panel);
        return false;
    }
    match insert_tab(node, host, panel) {
        Ok(()) => true,
        Err(panel) => {
            place_orphaned(node, panel);
            false
        }
    }
}

/// ホストの枠の隣にパネルを置く。比率は 0 から 1 に収める。
pub fn split_beside(
    node: &mut DockNode,
    host: &PanelId,
    panel: PanelId,
    axis: Axis,
    ratio: f32,
) -> bool {
    if host == &panel || !contains(node, host) {
        return false;
    }
    let ratio = clamp_ratio(ratio);
    let panel = take(node, &panel).unwrap_or(panel);
    if !contains(node, host) {
        place_orphaned(node, panel);
        return false;
    }
    match split_frame(node, host, panel, axis, ratio) {
        Ok(()) => true,
        Err(panel) => {
            place_orphaned(node, panel);
            false
        }
    }
}

/// パネル一つを Floating の節へ移す。
pub fn float_panel(node: &mut DockNode, id: &PanelId) -> bool {
    if sole_floating(node, id) {
        return true;
    }
    let Some(panel) = take(node, id) else {
        return false;
    };
    attach_floating(node, panel);
    true
}

/// パネルがいる枠ごと Floating にする。
pub fn float_frame(node: &mut DockNode, id: &PanelId) -> bool {
    match node {
        DockNode::Tabs { panels, .. } if panels.contains(id) => {
            let panels = mem::take(panels);
            *node = DockNode::Floating { panels };
            true
        }
        DockNode::Floating { panels } if panels.contains(id) => true,
        DockNode::Split { first, second, .. } => float_frame(first, id) || float_frame(second, id),
        DockNode::Tabs { .. } | DockNode::Floating { .. } => false,
    }
}

/// パネルが直接入っている分割の比率を変える。
pub fn set_ratio(node: &mut DockNode, id: &PanelId, ratio: f32) -> bool {
    let ratio = clamp_ratio(ratio);
    match node {
        DockNode::Split {
            ratio: slot,
            first,
            second,
            ..
        } => {
            if holds_panel(first, id) || holds_panel(second, id) {
                *slot = ratio;
                return true;
            }
            set_ratio(first, id, ratio) || set_ratio(second, id, ratio)
        }
        DockNode::Tabs { .. } | DockNode::Floating { .. } => false,
    }
}

fn collect(node: &DockNode, ids: &mut Vec<PanelId>) {
    match node {
        DockNode::Tabs { panels, .. } | DockNode::Floating { panels } => ids.extend(panels.clone()),
        DockNode::Split { first, second, .. } => {
            collect(first, ids);
            collect(second, ids);
        }
    }
}

fn holds_panel(node: &DockNode, id: &PanelId) -> bool {
    match node {
        DockNode::Tabs { panels, .. } | DockNode::Floating { panels } => panels.contains(id),
        DockNode::Split { .. } => false,
    }
}

fn sole_floating(node: &DockNode, id: &PanelId) -> bool {
    match node {
        DockNode::Floating { panels } => panels.len() == 1 && panels.first() == Some(id),
        DockNode::Split { first, second, .. } => {
            sole_floating(first, id) || sole_floating(second, id)
        }
        DockNode::Tabs { .. } => false,
    }
}

fn empty_tabs() -> DockNode {
    Dock::tabs(Vec::new())
}

fn clamp_ratio(ratio: f32) -> f32 {
    if ratio.is_finite() {
        ratio.clamp(0.0, 1.0)
    } else {
        0.5
    }
}

fn take(node: &mut DockNode, id: &PanelId) -> Option<PanelId> {
    if !contains(node, id) {
        return None;
    }
    let current = mem::replace(node, empty_tabs());
    let (next, found) = extract(current, id);
    *node = next.unwrap_or_else(empty_tabs);
    found
}

fn extract(node: DockNode, id: &PanelId) -> (Option<DockNode>, Option<PanelId>) {
    match node {
        DockNode::Tabs {
            mut panels,
            mut active,
        } => {
            let Some(index) = panels.iter().position(|panel| panel == id) else {
                return (Some(DockNode::Tabs { panels, active }), None);
            };
            let found = panels.remove(index);
            if panels.is_empty() {
                return (None, Some(found));
            }
            if index < active {
                active -= 1;
            }
            if active >= panels.len() {
                active = panels.len() - 1;
            }
            (Some(DockNode::Tabs { panels, active }), Some(found))
        }
        DockNode::Floating { mut panels } => {
            let Some(index) = panels.iter().position(|panel| panel == id) else {
                return (Some(DockNode::Floating { panels }), None);
            };
            let found = panels.remove(index);
            if panels.is_empty() {
                (None, Some(found))
            } else {
                (Some(DockNode::Floating { panels }), Some(found))
            }
        }
        DockNode::Split {
            axis,
            ratio,
            first,
            second,
        } => {
            let (first, found) = extract(*first, id);
            if let Some(found) = found {
                return (join(axis, ratio, first, Some(*second)), Some(found));
            }
            let (second, found) = extract(*second, id);
            (join(axis, ratio, first, second), found)
        }
    }
}

fn join(
    axis: Axis,
    ratio: f32,
    first: Option<DockNode>,
    second: Option<DockNode>,
) -> Option<DockNode> {
    match (first, second) {
        (Some(first), Some(second)) => Some(DockNode::Split {
            axis,
            ratio,
            first: Box::new(first),
            second: Box::new(second),
        }),
        (Some(only), None) | (None, Some(only)) => Some(only),
        (None, None) => None,
    }
}

fn insert_tab(node: &mut DockNode, host: &PanelId, panel: PanelId) -> Result<(), PanelId> {
    match node {
        DockNode::Tabs { panels, active } if panels.contains(host) => {
            panels.push(panel);
            *active = panels.len() - 1;
            Ok(())
        }
        DockNode::Floating { panels } if panels.contains(host) => {
            panels.insert(0, panel);
            Ok(())
        }
        DockNode::Split { first, second, .. } => match insert_tab(first, host, panel) {
            Ok(()) => Ok(()),
            Err(panel) => insert_tab(second, host, panel),
        },
        DockNode::Tabs { .. } | DockNode::Floating { .. } => Err(panel),
    }
}

fn split_frame(
    node: &mut DockNode,
    host: &PanelId,
    panel: PanelId,
    axis: Axis,
    ratio: f32,
) -> Result<(), PanelId> {
    match node {
        DockNode::Tabs { panels, .. } if panels.contains(host) => {
            let frame = mem::replace(node, empty_tabs());
            *node = Dock::split(axis, ratio, frame, Dock::tabs(vec![panel]));
            Ok(())
        }
        DockNode::Floating { panels } if panels.contains(host) => {
            let frame = mem::replace(node, empty_tabs());
            *node = Dock::split(axis, ratio, frame, Dock::tabs(vec![panel]));
            Ok(())
        }
        DockNode::Split { first, second, .. } => match split_frame(first, host, panel, axis, ratio)
        {
            Ok(()) => Ok(()),
            Err(panel) => split_frame(second, host, panel, axis, ratio),
        },
        DockNode::Tabs { .. } | DockNode::Floating { .. } => Err(panel),
    }
}

fn attach_floating(node: &mut DockNode, panel: PanelId) {
    let floating = DockNode::Floating {
        panels: vec![panel],
    };
    if is_empty(node) {
        *node = floating;
        return;
    }
    let current = mem::replace(node, empty_tabs());
    *node = Dock::split(Axis::Horizontal, 0.75, current, floating);
}

fn place_orphaned(node: &mut DockNode, panel: PanelId) {
    if is_empty(node) {
        *node = Dock::tabs(vec![panel]);
        return;
    }
    let current = mem::replace(node, empty_tabs());
    *node = Dock::split(Axis::Horizontal, 0.5, current, Dock::tabs(vec![panel]));
}

fn move_to_front(panels: &mut Vec<PanelId>, id: &PanelId) -> bool {
    let Some(index) = panels.iter().position(|panel| panel == id) else {
        return false;
    };
    let panel = panels.remove(index);
    panels.insert(0, panel);
    true
}
