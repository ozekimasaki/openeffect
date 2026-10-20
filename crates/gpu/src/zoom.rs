/// ヘルプが例示した固定段階。完全な一覧ではない。
pub const DOCUMENTED_FIXED_STEPS: [f64; 4] = [0.33, 0.5, 1.0, 2.0];

/// ヘルプが隣り合う上限の例として書いた 3200% と 6400%。
pub const DOCUMENTED_UPPER_STEPS: [f64; 2] = [32.0, 64.0];

/// 0 除算を避ける数値上の下限。ヘルプの最小倍率ではない。
const MIN_RATIO: f64 = 1.0e-6;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameSize {
    pub width: f64,
    pub height: f64,
    pub pixel_aspect: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PanelSize {
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PanelRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZoomMode {
    Legacy,
    Smooth,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZoomFocus {
    /// パネル中心。カーソル位置は使わない。
    Centered,
    FollowCursor,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ZoomTool {
    Click { cursor: [f64; 2], alt: bool },
    Drag { rect: PanelRect },
    DoubleClick,
}

/// 表示倍率。1.0 が 100%。レイヤーのスケールや書き出しサイズは持たない。
///
/// 初期モードは Smooth。フィット倍率が固定段階の外に出るためで、
/// 環境設定の初期値が Smooth だとは扱わない。
#[derive(Clone, Debug, PartialEq)]
pub struct ZoomState {
    pub mode: ZoomMode,
    pub focus: ZoomFocus,
    pub ratio: f64,
    /// パネル左上に対応するコンポジション座標。
    pub origin: [f64; 2],
    pub frame: FrameSize,
    pub panel: PanelSize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompPanel {
    pub views: Vec<ZoomState>,
    pub active: usize,
}

impl ZoomState {
    pub fn fitted(frame: FrameSize, panel: PanelSize) -> Self {
        let ratio = fit_ratio(frame, panel).max(MIN_RATIO);
        Self {
            mode: ZoomMode::Smooth,
            focus: ZoomFocus::Centered,
            ratio,
            origin: centered_origin(frame, panel, ratio),
            frame,
            panel,
        }
    }

    pub fn zoom_in(&mut self, steps: &[f64], smooth_factor: f64) {
        let next = self.next_ratio(false, steps, smooth_factor);
        self.set_ratio_about_center(next);
    }

    pub fn zoom_out(&mut self, steps: &[f64], smooth_factor: f64) {
        let next = self.next_ratio(true, steps, smooth_factor);
        self.set_ratio_about_center(next);
    }

    pub fn show_100(&mut self) {
        self.set_ratio_about_center(1.0);
    }

    pub fn fit(&mut self) {
        self.ratio = fit_ratio(self.frame, self.panel).max(MIN_RATIO);
        self.center_frame();
    }

    pub fn fit_up_to_100(&mut self) {
        self.ratio = fit_up_to_100(self.frame, self.panel).max(MIN_RATIO);
        self.center_frame();
    }

    pub fn set_from_menu(&mut self, ratio: f64) {
        if ratio.is_finite() && ratio > 0.0 {
            self.set_ratio_about_center(ratio);
        }
    }

    /// Legacy は一覧へ吸着する。Smooth は入力した倍率を保つ。
    pub fn set_hot_text(&mut self, ratio: f64, steps: &[f64]) {
        if !ratio.is_finite() || ratio <= 0.0 {
            return;
        }
        let ratio = match self.mode {
            ZoomMode::Legacy => nearest_step(ratio, steps).unwrap_or(ratio),
            ZoomMode::Smooth => ratio,
        };
        self.set_ratio_about_center(ratio);
    }

    /// `notches` の大きさは呼び出し側が決める。Shift の加速量は要確認。
    pub fn wheel(&mut self, notches: f64, cursor: [f64; 2], steps: &[f64]) {
        if !notches.is_finite() || notches == 0.0 {
            return;
        }
        let next = match self.mode {
            ZoomMode::Legacy => {
                let direction = if notches > 0.0 { 1 } else { -1 };
                step_ratio(self.ratio, steps, direction)
            }
            ZoomMode::Smooth => (self.ratio * (1.0 + notches)).max(MIN_RATIO),
        };
        self.apply_focus(next, cursor);
    }

    pub fn zoom_tool(&mut self, action: ZoomTool, steps: &[f64], smooth_factor: f64) {
        match action {
            ZoomTool::DoubleClick => self.show_100(),
            ZoomTool::Click { cursor, alt } => {
                let next = self.next_ratio(alt, steps, smooth_factor);
                self.set_ratio_about_panel_point(next, cursor);
            }
            ZoomTool::Drag { rect } => self.zoom_to_rect(rect),
        }
    }

    pub fn pan_panel(&mut self, dx: f64, dy: f64) {
        let aspect = self.frame.pixel_aspect.max(MIN_RATIO);
        self.origin[0] -= dx / (self.ratio * aspect);
        self.origin[1] -= dy / self.ratio;
    }

    pub fn center_frame(&mut self) {
        self.origin = centered_origin(self.frame, self.panel, self.ratio);
    }

    pub fn comp_pixels_per_screen_pixel(&self) -> [f64; 2] {
        let aspect = self.frame.pixel_aspect.max(MIN_RATIO);
        [1.0 / (self.ratio * aspect), 1.0 / self.ratio]
    }

    pub fn comp_at_panel(&self, panel: [f64; 2]) -> [f64; 2] {
        let aspect = self.frame.pixel_aspect.max(MIN_RATIO);
        [
            self.origin[0] + panel[0] / (self.ratio * aspect),
            self.origin[1] + panel[1] / self.ratio,
        ]
    }

    pub fn panel_center(&self) -> [f64; 2] {
        [self.panel.width * 0.5, self.panel.height * 0.5]
    }

    pub fn frame_fits(&self) -> bool {
        let aspect = self.frame.pixel_aspect.max(MIN_RATIO);
        let displayed_w = self.frame.width * aspect * self.ratio;
        let displayed_h = self.frame.height * self.ratio;
        if displayed_w > self.panel.width + 1.0e-6 || displayed_h > self.panel.height + 1.0e-6 {
            return false;
        }
        let top_left = self.comp_at_panel([0.0, 0.0]);
        let bottom_right = self.comp_at_panel([self.panel.width, self.panel.height]);
        top_left[0] <= 1.0e-6
            && top_left[1] <= 1.0e-6
            && bottom_right[0] >= self.frame.width - 1.0e-6
            && bottom_right[1] >= self.frame.height - 1.0e-6
    }

    fn next_ratio(&self, shrink: bool, steps: &[f64], smooth_factor: f64) -> f64 {
        match self.mode {
            ZoomMode::Legacy => {
                let direction = if shrink { -1 } else { 1 };
                step_ratio(self.ratio, steps, direction)
            }
            ZoomMode::Smooth => {
                let factor = if smooth_factor.is_finite() && smooth_factor > 1.0 {
                    smooth_factor
                } else {
                    2.0
                };
                let next = if shrink {
                    self.ratio / factor
                } else {
                    self.ratio * factor
                };
                next.max(MIN_RATIO)
            }
        }
    }

    fn apply_focus(&mut self, ratio: f64, cursor: [f64; 2]) {
        match self.focus {
            ZoomFocus::Centered => self.set_ratio_about_center(ratio),
            ZoomFocus::FollowCursor => self.set_ratio_about_panel_point(ratio, cursor),
        }
    }

    fn set_ratio_about_center(&mut self, ratio: f64) {
        self.set_ratio_about_panel_point(ratio, self.panel_center());
    }

    fn set_ratio_about_panel_point(&mut self, ratio: f64, panel: [f64; 2]) {
        let ratio = if ratio.is_finite() && ratio > 0.0 {
            ratio.max(MIN_RATIO)
        } else {
            self.ratio
        };
        let comp = self.comp_at_panel(panel);
        let aspect = self.frame.pixel_aspect.max(MIN_RATIO);
        self.ratio = ratio;
        self.origin = [
            comp[0] - panel[0] / (ratio * aspect),
            comp[1] - panel[1] / ratio,
        ];
    }

    fn zoom_to_rect(&mut self, rect: PanelRect) {
        if !rect.width.is_finite()
            || !rect.height.is_finite()
            || rect.width.abs() < 1.0e-6
            || rect.height.abs() < 1.0e-6
        {
            return;
        }
        let start = self.comp_at_panel([rect.x, rect.y]);
        let end = self.comp_at_panel([rect.x + rect.width, rect.y + rect.height]);
        let comp_w = (end[0] - start[0]).abs().max(1.0e-6);
        let comp_h = (end[1] - start[1]).abs().max(1.0e-6);
        let aspect = self.frame.pixel_aspect.max(MIN_RATIO);
        let ratio = (self.panel.width / (comp_w * aspect))
            .min(self.panel.height / comp_h)
            .max(MIN_RATIO);
        let center = [(start[0] + end[0]) * 0.5, (start[1] + end[1]) * 0.5];
        self.ratio = ratio;
        self.origin = [
            center[0] - (self.panel.width * 0.5) / (ratio * aspect),
            center[1] - (self.panel.height * 0.5) / ratio,
        ];
    }
}

impl CompPanel {
    pub fn set_menu_ratio(&mut self, ratio: f64, ctrl: bool) {
        if self.views.is_empty() {
            return;
        }
        if ctrl {
            for view in &mut self.views {
                view.set_from_menu(ratio);
            }
        } else if let Some(view) = self.views.get_mut(self.active) {
            view.set_from_menu(ratio);
        }
    }
}

pub fn fit_ratio(frame: FrameSize, panel: PanelSize) -> f64 {
    if frame.width <= 0.0 || frame.height <= 0.0 || panel.width <= 0.0 || panel.height <= 0.0 {
        return 1.0;
    }
    let aspect = if frame.pixel_aspect.is_finite() && frame.pixel_aspect > 0.0 {
        frame.pixel_aspect
    } else {
        1.0
    };
    let sx = panel.width / (frame.width * aspect);
    let sy = panel.height / frame.height;
    sx.min(sy)
}

/// Windows の Fit up to 100%。100% を超えない。
pub fn fit_up_to_100(frame: FrameSize, panel: PanelSize) -> f64 {
    fit_ratio(frame, panel).min(1.0)
}

fn centered_origin(frame: FrameSize, panel: PanelSize, ratio: f64) -> [f64; 2] {
    let aspect = frame.pixel_aspect.max(MIN_RATIO);
    let ratio = ratio.max(MIN_RATIO);
    [
        frame.width * 0.5 - (panel.width * 0.5) / (ratio * aspect),
        frame.height * 0.5 - (panel.height * 0.5) / ratio,
    ]
}

fn step_ratio(current: f64, steps: &[f64], direction: i32) -> f64 {
    let mut ordered: Vec<f64> = steps
        .iter()
        .copied()
        .filter(|step| step.is_finite() && *step > 0.0)
        .collect();
    ordered.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    ordered.dedup_by(|a, b| (*a - *b).abs() < 1.0e-9);
    if ordered.is_empty() || direction == 0 {
        return current;
    }
    if direction > 0 {
        ordered
            .iter()
            .copied()
            .find(|step| *step > current + 1.0e-9)
            .unwrap_or(ordered[ordered.len() - 1])
    } else {
        ordered
            .iter()
            .rev()
            .copied()
            .find(|step| *step < current - 1.0e-9)
            .unwrap_or(ordered[0])
    }
}

fn nearest_step(value: f64, steps: &[f64]) -> Option<f64> {
    steps
        .iter()
        .copied()
        .filter(|step| step.is_finite() && *step > 0.0)
        .min_by(|a, b| {
            let da = (*a - value).abs();
            let db = (*b - value).abs();
            da.partial_cmp(&db)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        })
}

#[cfg(test)]
mod tests {
    use super::{
        fit_ratio, fit_up_to_100, CompPanel, FrameSize, PanelRect, PanelSize, ZoomFocus, ZoomMode,
        ZoomState, ZoomTool, DOCUMENTED_FIXED_STEPS, DOCUMENTED_UPPER_STEPS,
    };
    use crate::export_size;

    fn frame(width: f64, height: f64) -> FrameSize {
        FrameSize {
            width,
            height,
            pixel_aspect: 1.0,
        }
    }

    fn panel(width: f64, height: f64) -> PanelSize {
        PanelSize { width, height }
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1.0e-9
    }

    #[test]
    fn initial_ratio_fits_the_panel() {
        let view = ZoomState::fitted(frame(200.0, 100.0), panel(100.0, 100.0));
        assert!(close(view.ratio, 0.5));
        assert!(view.frame_fits());
        assert!(close(view.frame.width / view.frame.height, 2.0));
    }

    #[test]
    fn zoom_in_and_out_do_not_change_composition_pixels() {
        let mut view = ZoomState::fitted(frame(100.0, 80.0), panel(100.0, 80.0));
        view.mode = ZoomMode::Legacy;
        view.ratio = 1.0;
        let before = view.ratio;
        view.zoom_in(&DOCUMENTED_FIXED_STEPS, 2.0);
        assert!(view.ratio > before);
        view.zoom_out(&DOCUMENTED_FIXED_STEPS, 2.0);
        assert!(close(view.ratio, before));
        assert_eq!(export_size(100, 80), (100, 80));
    }

    #[test]
    fn one_hundred_percent_maps_one_screen_pixel() {
        let mut view = ZoomState::fitted(frame(40.0, 20.0), panel(80.0, 80.0));
        view.show_100();
        assert!(close(view.ratio, 1.0));
        let scale = view.comp_pixels_per_screen_pixel();
        assert!(close(scale[0], 1.0));
        assert!(close(scale[1], 1.0));
    }

    #[test]
    fn fit_has_no_upper_bound_and_keeps_aspect() {
        let mut view = ZoomState::fitted(frame(100.0, 50.0), panel(400.0, 400.0));
        view.fit();
        assert!(close(view.ratio, fit_ratio(view.frame, view.panel)));
        assert!(close(view.ratio, 4.0));
        assert!(view.frame_fits());
    }

    #[test]
    fn fit_up_to_100_never_exceeds_full_size() {
        let mut large = ZoomState::fitted(frame(100.0, 100.0), panel(400.0, 400.0));
        large.fit_up_to_100();
        assert!(close(large.ratio, 1.0));
        assert!(large.frame_fits());
        assert!(close(
            fit_up_to_100(frame(100.0, 100.0), panel(400.0, 400.0)),
            1.0
        ));

        let mut small = ZoomState::fitted(frame(100.0, 100.0), panel(50.0, 40.0));
        small.fit_up_to_100();
        assert!(small.ratio <= 1.0);
        assert!(small.frame_fits());
        assert!(close(small.ratio, 0.4));
    }

    #[test]
    fn zoom_tool_double_click_is_100_and_alt_click_shrinks() {
        let mut view = ZoomState::fitted(frame(100.0, 100.0), panel(100.0, 100.0));
        view.mode = ZoomMode::Legacy;
        view.ratio = 1.0;
        view.zoom_tool(
            ZoomTool::Click {
                cursor: [20.0, 20.0],
                alt: false,
            },
            &DOCUMENTED_FIXED_STEPS,
            2.0,
        );
        assert!(close(view.ratio, 2.0));
        view.zoom_tool(
            ZoomTool::Click {
                cursor: [20.0, 20.0],
                alt: true,
            },
            &DOCUMENTED_FIXED_STEPS,
            2.0,
        );
        assert!(close(view.ratio, 1.0));
        view.ratio = 2.0;
        view.zoom_tool(ZoomTool::DoubleClick, &DOCUMENTED_FIXED_STEPS, 2.0);
        assert!(close(view.ratio, 1.0));
    }

    #[test]
    fn drag_rect_zooms_into_that_range() {
        let mut view = ZoomState::fitted(frame(100.0, 100.0), panel(100.0, 100.0));
        view.ratio = 1.0;
        view.origin = [0.0, 0.0];
        view.zoom_tool(
            ZoomTool::Drag {
                rect: PanelRect {
                    x: 0.0,
                    y: 0.0,
                    width: 50.0,
                    height: 50.0,
                },
            },
            &[],
            2.0,
        );
        assert!(close(view.ratio, 2.0));
        let center = view.comp_at_panel(view.panel_center());
        assert!(close(center[0], 25.0));
        assert!(close(center[1], 25.0));
    }

    #[test]
    fn pan_changes_only_the_view_origin() {
        let mut view = ZoomState::fitted(frame(100.0, 100.0), panel(50.0, 50.0));
        view.show_100();
        let layer_position = [12.0, 34.0, 0.0];
        let origin = view.origin;
        view.pan_panel(10.0, -4.0);
        assert_ne!(view.origin, origin);
        assert_eq!(layer_position, [12.0, 34.0, 0.0]);
        assert!(close(view.ratio, 1.0));
    }

    #[test]
    fn hand_double_click_centers_the_frame() {
        let mut view = ZoomState::fitted(frame(100.0, 80.0), panel(40.0, 40.0));
        view.show_100();
        view.pan_panel(15.0, 6.0);
        view.center_frame();
        let center = view.comp_at_panel(view.panel_center());
        assert!(close(center[0], 50.0));
        assert!(close(center[1], 40.0));
    }

    #[test]
    fn legacy_wheel_and_hot_text_stay_on_the_step_list() {
        let mut view = ZoomState::fitted(frame(100.0, 100.0), panel(100.0, 100.0));
        view.mode = ZoomMode::Legacy;
        view.ratio = 1.0;
        view.wheel(1.0, [0.0, 0.0], &DOCUMENTED_FIXED_STEPS);
        assert!(DOCUMENTED_FIXED_STEPS.contains(&view.ratio));
        assert!(close(view.ratio, 2.0));
        view.set_hot_text(0.4, &DOCUMENTED_FIXED_STEPS);
        assert!(DOCUMENTED_FIXED_STEPS.contains(&view.ratio));
        assert!(!close(view.ratio, 0.4));
        assert!(close(view.ratio, 0.33));
    }

    #[test]
    fn smooth_keeps_arbitrary_ratios_and_centered_ignores_the_cursor() {
        let mut view = ZoomState::fitted(frame(100.0, 100.0), panel(100.0, 100.0));
        view.mode = ZoomMode::Smooth;
        view.focus = ZoomFocus::Centered;
        view.ratio = 1.0;
        view.origin = [0.0, 0.0];
        view.set_hot_text(0.4, &DOCUMENTED_FIXED_STEPS);
        assert!(close(view.ratio, 0.4));
        view.ratio = 1.0;
        view.origin = [0.0, 0.0];
        let center_before = view.comp_at_panel(view.panel_center());
        view.wheel(1.0, [0.0, 0.0], &DOCUMENTED_FIXED_STEPS);
        let center_after = view.comp_at_panel(view.panel_center());
        assert!(close(center_before[0], center_after[0]));
        assert!(close(center_before[1], center_after[1]));
        assert!(close(view.ratio, 2.0));

        view.focus = ZoomFocus::FollowCursor;
        view.ratio = 1.0;
        view.origin = [0.0, 0.0];
        let cursor = [10.0, 20.0];
        let pointed = view.comp_at_panel(cursor);
        view.wheel(1.0, cursor, &[]);
        let pointed_after = view.comp_at_panel(cursor);
        assert!(close(pointed[0], pointed_after[0]));
        assert!(close(pointed[1], pointed_after[1]));
    }

    #[test]
    fn fixed_steps_can_move_from_3200_to_6400() {
        let mut view = ZoomState::fitted(frame(10.0, 10.0), panel(10.0, 10.0));
        view.mode = ZoomMode::Legacy;
        view.ratio = 32.0;
        view.zoom_in(&DOCUMENTED_UPPER_STEPS, 2.0);
        assert!(close(view.ratio, 64.0));
    }

    #[test]
    fn ctrl_changes_every_view_in_the_panel() {
        let frame = frame(100.0, 100.0);
        let panel = panel(100.0, 100.0);
        let mut views = CompPanel {
            views: vec![
                ZoomState::fitted(frame, panel),
                ZoomState::fitted(frame, panel),
            ],
            active: 0,
        };
        views.views[0].ratio = 1.0;
        views.views[1].ratio = 1.0;
        views.set_menu_ratio(2.0, false);
        assert!(close(views.views[0].ratio, 2.0));
        assert!(close(views.views[1].ratio, 1.0));
        views.set_menu_ratio(0.5, true);
        assert!(close(views.views[0].ratio, 0.5));
        assert!(close(views.views[1].ratio, 0.5));
    }
}
