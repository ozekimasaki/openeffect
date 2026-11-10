use crate::confirmed::{Confirmed, Keyframes};

/// 現行ボタンは 1 つ。古い「ライトを消す」挙動は別モードにしない。中身のどちらを指すかは要確認。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DraftEffect {
    SingleUnconfirmedMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Draft3d {
    pub enabled: Confirmed<bool>,
    pub effect: DraftEffect,
}

impl Draft3d {
    pub const fn new() -> Self {
        Self {
            enabled: Confirmed::Unconfirmed,
            effect: DraftEffect::SingleUnconfirmedMode,
        }
    }
}

impl Default for Draft3d {
    fn default() -> Self {
        Self::new()
    }
}

pub const fn draft_button_visible(has_3d_content: bool) -> bool {
    has_3d_content
}

pub const fn draft_shortcut() -> Confirmed<&'static str> {
    Confirmed::Unconfirmed
}

pub const fn preview_is_faster(enabled: bool) -> bool {
    enabled
}

pub const fn draft_keyframes() -> Keyframes {
    Keyframes::Forbidden
}

pub const fn separate_disable_lights_mode() -> bool {
    false
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GroundPlane {
    pub visible: bool,
    pub in_render_output: bool,
    pub center_axes: bool,
}

pub const fn ground_plane(draft_enabled: bool, button_on: bool) -> Option<GroundPlane> {
    if !draft_enabled {
        None
    } else {
        Some(GroundPlane {
            visible: button_on,
            in_render_output: false,
            center_axes: button_on,
        })
    }
}

pub const fn grid_step_thresholds() -> Confirmed<()> {
    Confirmed::Unconfirmed
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExtendedViewer {
    pub show_outside_frame: bool,
    pub render_includes_outside: bool,
}

pub const fn extended_viewer(enabled: bool) -> ExtendedViewer {
    ExtendedViewer {
        show_outside_frame: enabled,
        render_includes_outside: false,
    }
}

pub const fn preview_shows_point(extended: bool, outside_frame: bool) -> bool {
    if outside_frame {
        extended
    } else {
        true
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReferenceAxes {
    pub visible: bool,
}

impl Default for ReferenceAxes {
    fn default() -> Self {
        Self { visible: true }
    }
}

pub const fn reference_axes_keyframes() -> Keyframes {
    Keyframes::Forbidden
}

#[cfg(test)]
mod tests {
    use super::{
        draft_button_visible, draft_shortcut, extended_viewer, grid_step_thresholds, ground_plane,
        preview_is_faster, preview_shows_point, separate_disable_lights_mode, Draft3d, DraftEffect,
        ReferenceAxes,
    };

    #[test]
    fn draft_button_needs_3d_and_is_one_mode() {
        assert!(!draft_button_visible(false));
        assert!(draft_button_visible(true));
        assert!(preview_is_faster(true));
        assert!(!preview_is_faster(false));
        assert_eq!(Draft3d::new().effect, DraftEffect::SingleUnconfirmedMode);
        assert!(!separate_disable_lights_mode());
        assert!(draft_shortcut().is_unconfirmed());
        assert!(Draft3d::new().enabled.is_unconfirmed());
    }

    #[test]
    fn ground_plane_requires_draft_and_stays_out_of_the_render() {
        assert!(ground_plane(false, true).is_none());
        let plane = ground_plane(true, true).unwrap();
        assert!(plane.visible);
        assert!(plane.center_axes);
        assert!(!plane.in_render_output);
        assert!(grid_step_thresholds().is_unconfirmed());
    }

    #[test]
    fn extended_viewer_shows_outside_the_frame_only_in_preview() {
        assert!(preview_shows_point(true, true));
        assert!(!preview_shows_point(false, true));
        assert!(preview_shows_point(false, false));
        assert!(!extended_viewer(true).render_includes_outside);
    }

    #[test]
    fn reference_axes_start_visible() {
        assert!(ReferenceAxes::default().visible);
        let hidden = ReferenceAxes { visible: false };
        assert!(!hidden.visible);
    }
}
