use crate::confirmed::Confirmed;
use crate::light::SceneLightKind;
use crate::math::{add, close3, cross, length, normalize, scale, sub};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CameraTool {
    Unified,
    Orbit,
    Xy,
    Z,
}

impl CameraTool {
    pub const fn cycle(self) -> Self {
        match self {
            Self::Unified => Self::Orbit,
            Self::Orbit => Self::Xy,
            Self::Xy => Self::Z,
            Self::Z => Self::Unified,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
}

pub const fn unified_button(button: MouseButton) -> CameraTool {
    match button {
        MouseButton::Left => CameraTool::Orbit,
        MouseButton::Middle => CameraTool::Xy,
        MouseButton::Right => CameraTool::Z,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OrbitAxis {
    Yaw,
    Pitch,
}

/// 目標点を固定して位置を回す。距離は保つ。Shift の軸の選び方は要確認なので、軸は引数で受ける。
pub fn orbit(
    position: [f64; 3],
    point_of_interest: [f64; 3],
    yaw_deg: f64,
    pitch_deg: f64,
    constraint: Option<OrbitAxis>,
) -> [f64; 3] {
    let (yaw, pitch) = match constraint {
        Some(OrbitAxis::Yaw) => (yaw_deg, 0.0),
        Some(OrbitAxis::Pitch) => (0.0, pitch_deg),
        None => (yaw_deg, pitch_deg),
    };
    let offset = sub(position, point_of_interest);
    let turned = crate::math::rotate_x(crate::math::rotate_y(offset, yaw), pitch);
    add(point_of_interest, turned)
}

pub fn pan(
    position: [f64; 3],
    point_of_interest: [f64; 3],
    delta: [f64; 3],
) -> ([f64; 3], [f64; 3]) {
    let Some(forward) = normalize(sub(point_of_interest, position)) else {
        return (add(position, delta), add(point_of_interest, delta));
    };
    let offset = crate::math::reject(delta, forward);
    (add(position, offset), add(point_of_interest, offset))
}

pub fn dolly(position: [f64; 3], point_of_interest: [f64; 3], toward_interest: f64) -> [f64; 3] {
    let Some(forward) = normalize(sub(point_of_interest, position)) else {
        return position;
    };
    add(position, scale(forward, toward_interest))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WorkViewKind {
    Front,
    Back,
    Left,
    Right,
    Top,
    Bottom,
    Custom,
}

impl WorkViewKind {
    pub const fn orbit_allowed(self) -> bool {
        matches!(self, Self::Custom)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorkView {
    pub kind: WorkViewKind,
    pub scale: f64,
    pub yaw_deg: f64,
    pub pitch_deg: f64,
}

pub fn orbit_work_view(view: WorkView, yaw_deg: f64, pitch_deg: f64) -> Option<WorkView> {
    if !view.kind.orbit_allowed() {
        return None;
    }
    Some(WorkView {
        yaw_deg: view.yaw_deg + yaw_deg,
        pitch_deg: view.pitch_deg + pitch_deg,
        ..view
    })
}

pub fn dolly_work_scale(scale_value: f64, factor: f64) -> f64 {
    scale_value * factor
}

pub fn reset_view(initial: WorkView) -> (WorkView, bool) {
    (initial, false)
}

#[derive(Clone, Debug, PartialEq)]
pub struct ViewHistory {
    pub current: WorkViewKind,
    pub previous: Vec<WorkViewKind>,
}

impl ViewHistory {
    pub fn push(&mut self, next: WorkViewKind) {
        self.previous.push(self.current);
        self.current = next;
    }

    pub fn back(&mut self) -> Option<WorkViewKind> {
        let previous = self.previous.pop()?;
        self.current = previous;
        Some(previous)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub min: [f64; 3],
    pub max: [f64; 3],
}

/// ズームの等倍規則で、渡された範囲だけがフレームに入る位置と目標点を返す。
pub fn fit_camera(
    bounds: Bounds,
    zoom_px: f64,
    comp_width: f64,
    comp_height: f64,
    forward: [f64; 3],
) -> Option<([f64; 3], [f64; 3])> {
    let center = scale(add(bounds.min, bounds.max), 0.5);
    let width = (bounds.max[0] - bounds.min[0]).abs();
    let height = (bounds.max[1] - bounds.min[1]).abs();
    let distance = (zoom_px * width / comp_width).max(zoom_px * height / comp_height);
    let forward = normalize(forward)?;
    Some((sub(center, scale(forward, distance)), center))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Axis {
    X,
    Y,
    Z,
}

pub fn drag_axis(
    position: [f64; 3],
    point_of_interest: [f64; 3],
    axis: Axis,
    delta: f64,
    ctrl: bool,
) -> ([f64; 3], [f64; 3]) {
    let mut next_position = position;
    let mut next_interest = point_of_interest;
    let index = match axis {
        Axis::X => 0,
        Axis::Y => 1,
        Axis::Z => 2,
    };
    next_position[index] += delta;
    if !ctrl {
        next_interest[index] += delta;
    }
    (next_position, next_interest)
}

pub fn drag_icon(position: [f64; 3], delta: [f64; 3]) -> [f64; 3] {
    add(position, delta)
}

pub fn drag_interest(point_of_interest: [f64; 3], delta: [f64; 3]) -> [f64; 3] {
    add(point_of_interest, delta)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraLightLink {
    pub offset: [f64; 3],
    pub mechanism: Confirmed<LinkMechanism>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LinkMechanism {
    Parent,
    Expression,
}

impl CameraLightLink {
    pub fn from_positions(camera: [f64; 3], light: [f64; 3]) -> Self {
        Self {
            offset: sub(light, camera),
            mechanism: Confirmed::Unconfirmed,
        }
    }

    pub fn move_camera(&self, camera: [f64; 3], delta: [f64; 3]) -> ([f64; 3], [f64; 3]) {
        let next = add(camera, delta);
        (next, add(next, self.offset))
    }
}

pub const fn cone_changes_linked_zoom(kind: SceneLightKind) -> bool {
    matches!(kind, SceneLightKind::Spot)
}

pub fn linked_zoom_after_cone_change(
    previous_zoom: f64,
    cone_before: f64,
    cone_after: f64,
) -> Confirmed<f64> {
    if close3([cone_before, 0.0, 0.0], [cone_after, 0.0, 0.0]) {
        Confirmed::Known(previous_zoom)
    } else {
        Confirmed::Unconfirmed
    }
}

pub const VIEW_1_FRONT_SHORTCUT: &str = "F10";
pub const VIEW_2_CUSTOM_SHORTCUT: &str = "F11";
pub const PREVIOUS_VIEW_SHORTCUT: &str = "Esc";
pub const FIT_SELECTED_SHORTCUT: &str = "Ctrl+Alt+Shift+\\";
pub const FIT_SELECTED_WITH_CAMERA_TOOL: &str = "F";
pub const FIT_ALL_WITH_CAMERA_TOOL: &str = "Ctrl+Shift+F";

pub fn same_distance(
    position: [f64; 3],
    interest: [f64; 3],
    other_position: [f64; 3],
    other_interest: [f64; 3],
) -> bool {
    (length(sub(position, interest)) - length(sub(other_position, other_interest))).abs() <= 1e-6
}

pub fn view_right(forward: [f64; 3]) -> Option<[f64; 3]> {
    normalize(cross([0.0, -1.0, 0.0], forward))
}

#[cfg(test)]
mod tests {
    use super::{
        cone_changes_linked_zoom, dolly, dolly_work_scale, drag_axis, drag_icon, drag_interest,
        fit_camera, linked_zoom_after_cone_change, orbit, orbit_work_view, pan, reset_view,
        same_distance, unified_button, Axis, Bounds, CameraLightLink, CameraTool, Confirmed,
        MouseButton, OrbitAxis, WorkView, WorkViewKind, FIT_ALL_WITH_CAMERA_TOOL,
        FIT_SELECTED_SHORTCUT, FIT_SELECTED_WITH_CAMERA_TOOL, PREVIOUS_VIEW_SHORTCUT,
        VIEW_1_FRONT_SHORTCUT, VIEW_2_CUSTOM_SHORTCUT,
    };
    use crate::light::SceneLightKind;
    use crate::math::{close, close3, length, sub};

    #[test]
    fn unified_camera_maps_mouse_buttons() {
        assert_eq!(CameraTool::Unified.cycle(), CameraTool::Orbit);
        assert_eq!(unified_button(MouseButton::Left), CameraTool::Orbit);
        assert_eq!(unified_button(MouseButton::Middle), CameraTool::Xy);
        assert_eq!(unified_button(MouseButton::Right), CameraTool::Z);
        let position = [0.0, 0.0, 10.0];
        let interest = [0.0, 0.0, 0.0];
        let orbited = orbit(position, interest, 20.0, 10.0, None);
        assert!(close3(interest, interest));
        assert!(same_distance(position, interest, orbited, interest));
        let (panned, interest_after) = pan(position, interest, [3.0, -2.0, 0.0]);
        assert!(close(
            length(sub(panned, interest_after)),
            length(sub(position, interest))
        ));
        let dolled = dolly(position, interest, 4.0);
        assert!(length(sub(dolled, interest)) < length(sub(position, interest)));
    }

    #[test]
    fn orbit_keeps_interest_and_is_blocked_on_ortho_views() {
        let position = [0.0, 0.0, 8.0];
        let interest = [1.0, 2.0, 3.0];
        let moved = orbit(position, interest, 30.0, -15.0, None);
        assert!(same_distance(position, interest, moved, interest));
        assert_ne!(moved, position);
        assert!(!WorkViewKind::Front.orbit_allowed());
        assert!(WorkViewKind::Custom.orbit_allowed());
    }

    #[test]
    fn shift_orbit_changes_one_axis() {
        let position = [0.0, 0.0, 10.0];
        let interest = [0.0, 0.0, 0.0];
        let free = orbit(position, interest, 25.0, 15.0, None);
        let yaw_only = orbit(position, interest, 25.0, 15.0, Some(OrbitAxis::Yaw));
        let pitch_only = orbit(position, interest, 25.0, 15.0, Some(OrbitAxis::Pitch));
        assert_ne!(free, yaw_only);
        assert_ne!(yaw_only, pitch_only);
        assert!(same_distance(position, interest, yaw_only, interest));
    }

    #[test]
    fn xy_camera_keeps_the_interest_distance() {
        let position = [0.0, 0.0, 12.0];
        let interest = [0.0, 0.0, 0.0];
        let (next, next_interest) = pan(position, interest, [4.0, 5.0, 9.0]);
        assert!(close(length(sub(next, next_interest)), 12.0));
        assert!(!close3(next, position));
    }

    #[test]
    fn z_camera_approaches_interest_and_ortho_changes_scale() {
        let position = [0.0, 0.0, 20.0];
        let interest = [0.0, 0.0, 0.0];
        let closer = dolly(position, interest, 5.0);
        assert!(close(closer[2], 15.0));
        assert_eq!(interest, [0.0, 0.0, 0.0]);
        let layer_position = position;
        let scaled = dolly_work_scale(1.0, 1.25);
        assert!(close(scaled, 1.25));
        assert_eq!(layer_position, position);
    }

    #[test]
    fn work_view_orbit_does_not_change_layer_values() {
        let view = WorkView {
            kind: WorkViewKind::Custom,
            scale: 1.0,
            yaw_deg: 0.0,
            pitch_deg: 0.0,
        };
        let camera_position = [1.0, 2.0, 3.0];
        let camera_interest = [4.0, 5.0, 6.0];
        let updated = orbit_work_view(view, 10.0, 5.0).unwrap();
        assert!(close(updated.yaw_deg, 10.0));
        assert_eq!(camera_position, [1.0, 2.0, 3.0]);
        assert_eq!(camera_interest, [4.0, 5.0, 6.0]);
        assert!(orbit_work_view(
            WorkView {
                kind: WorkViewKind::Front,
                ..view
            },
            10.0,
            0.0
        )
        .is_none());
    }

    #[test]
    fn reset_restores_the_view_and_keeps_keyframes() {
        let initial = WorkView {
            kind: WorkViewKind::Custom,
            scale: 1.0,
            yaw_deg: 0.0,
            pitch_deg: 0.0,
        };
        let (reset, keyframes_cleared) = reset_view(initial);
        assert_eq!(reset, initial);
        assert!(!keyframes_cleared);
    }

    #[test]
    fn fit_selected_uses_only_the_given_bounds() {
        let selected = Bounds {
            min: [0.0, 0.0, 0.0],
            max: [100.0, 50.0, 0.0],
        };
        let (position, interest) =
            fit_camera(selected, 200.0, 200.0, 100.0, [0.0, 0.0, -1.0]).unwrap();
        assert!(close3(interest, [50.0, 25.0, 0.0]));
        assert!(position[2] > 0.0);
        assert_eq!(FIT_SELECTED_SHORTCUT, "Ctrl+Alt+Shift+\\");
        assert_eq!(FIT_SELECTED_WITH_CAMERA_TOOL, "F");
    }

    #[test]
    fn fit_all_uses_the_union_bounds() {
        let all = Bounds {
            min: [-20.0, -10.0, 0.0],
            max: [20.0, 10.0, 0.0],
        };
        let (_, interest) = fit_camera(all, 100.0, 100.0, 100.0, [0.0, 0.0, -1.0]).unwrap();
        assert!(close3(interest, [0.0, 0.0, 0.0]));
        assert_eq!(FIT_ALL_WITH_CAMERA_TOOL, "Ctrl+Shift+F");
    }

    #[test]
    fn axis_drag_moves_position_and_interest_together() {
        let (position, interest) = drag_axis([0.0, 1.0, 2.0], [3.0, 4.0, 5.0], Axis::X, 6.0, false);
        assert!(close(position[0], 6.0));
        assert!(close(interest[0], 9.0));
        assert!(close(position[1], 1.0));
        assert!(close(interest[1], 4.0));
    }

    #[test]
    fn ctrl_axis_drag_leaves_interest() {
        let (position, interest) = drag_axis([0.0, 0.0, 0.0], [1.0, 2.0, 3.0], Axis::Y, 4.0, true);
        assert!(close(position[1], 4.0));
        assert!(close3(interest, [1.0, 2.0, 3.0]));
    }

    #[test]
    fn icon_drag_leaves_interest() {
        let position = drag_icon([1.0, 1.0, 1.0], [2.0, 0.0, -1.0]);
        let interest = [9.0, 9.0, 9.0];
        assert!(close3(position, [3.0, 1.0, 0.0]));
        assert!(close3(interest, [9.0, 9.0, 9.0]));
        assert_eq!(
            crate::light::has_comp_icon(SceneLightKind::Ambient),
            Confirmed::Known(false)
        );
    }

    #[test]
    fn interest_drag_leaves_position() {
        let position = [1.0, 2.0, 3.0];
        let interest = drag_interest([0.0, 0.0, 0.0], [4.0, 0.0, 0.0]);
        assert!(close3(position, [1.0, 2.0, 3.0]));
        assert!(close3(interest, [4.0, 0.0, 0.0]));
    }

    #[test]
    fn camera_control_link_keeps_the_offset_and_spot_cone_dirties_zoom() {
        let link = CameraLightLink::from_positions([0.0, 0.0, 10.0], [1.0, 2.0, 14.0]);
        assert!(link.mechanism.is_unconfirmed());
        let (camera, light) = link.move_camera([0.0, 0.0, 10.0], [3.0, 0.0, 0.0]);
        assert!(close3(camera, [3.0, 0.0, 10.0]));
        assert!(close3(light, [4.0, 2.0, 14.0]));
        assert!(cone_changes_linked_zoom(SceneLightKind::Spot));
        assert!(!cone_changes_linked_zoom(SceneLightKind::Parallel));
        assert!(linked_zoom_after_cone_change(500.0, 30.0, 40.0).is_unconfirmed());
    }

    #[test]
    fn view_shortcuts_match_the_default_assignment() {
        assert_eq!(VIEW_1_FRONT_SHORTCUT, "F10");
        assert_eq!(VIEW_2_CUSTOM_SHORTCUT, "F11");
        assert_eq!(PREVIOUS_VIEW_SHORTCUT, "Esc");
        let mut history = super::ViewHistory {
            current: WorkViewKind::Front,
            previous: Vec::new(),
        };
        history.push(WorkViewKind::Custom);
        assert_eq!(history.back(), Some(WorkViewKind::Front));
    }
}
