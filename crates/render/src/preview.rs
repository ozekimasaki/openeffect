use model::{Composition, Time};

/// プレビューの範囲。スペースキーの既定はワークエリアを現在時刻で延長する。
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RangeMode {
    WorkArea,
    WorkAreaExtendedByCurrentTime,
    EntireDuration,
    Around { preroll: f64, postroll: f64 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PreviewRequest {
    pub range: RangeMode,
    pub from_current_time: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimeSpan {
    pub start: f64,
    pub end: f64,
}

impl TimeSpan {
    pub fn contains(self, time: f64) -> bool {
        time >= self.start && time < self.end
    }
}

pub fn resolve_range(comp: &Composition, current: f64, request: PreviewRequest) -> TimeSpan {
    let duration = comp.duration.seconds();
    let current = current.clamp(0.0, duration);
    let wa_start = comp.work_area_start.seconds();
    let wa_end = wa_start + comp.work_area_duration.seconds();
    let mut span = match request.range {
        RangeMode::WorkArea => (wa_start, wa_end),
        RangeMode::EntireDuration => (0.0, duration),
        RangeMode::Around { preroll, postroll } => {
            ((current - preroll).max(0.0), (current + postroll).min(duration))
        }
        RangeMode::WorkAreaExtendedByCurrentTime => {
            if current < wa_start {
                (current, wa_end)
            } else if current > wa_end {
                if request.from_current_time {
                    (wa_start, duration)
                } else {
                    (wa_start, current)
                }
            } else {
                (wa_start, wa_end)
            }
        }
    };
    let extended_after = matches!(request.range, RangeMode::WorkAreaExtendedByCurrentTime) && current > wa_end;
    if request.from_current_time && !extended_after && current >= span.0 && current <= span.1 {
        span.0 = current;
    }
    TimeSpan {
        start: span.0,
        end: span.1.max(span.0),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreviewResolution {
    Full,
    Half,
    Third,
    Quarter,
    Custom { x: u32, y: u32 },
    Auto,
}

impl PreviewResolution {
    pub fn denominators(self) -> (u32, u32) {
        match self {
            Self::Full | Self::Auto => (1, 1),
            Self::Half => (2, 2),
            Self::Third => (3, 3),
            Self::Quarter => (4, 4),
            Self::Custom { x, y } => (x.max(1), y.max(1)),
        }
    }
}

pub fn preview_pixels(width: u32, height: u32, resolution: PreviewResolution) -> (u32, u32) {
    let (dx, dy) = resolution.denominators();
    ((width / dx).max(1), (height / dy).max(1))
}

pub fn effective_resolution(preview: PreviewResolution, viewer: PreviewResolution) -> PreviewResolution {
    match preview {
        PreviewResolution::Auto => match viewer {
            PreviewResolution::Auto => PreviewResolution::Full,
            other => other,
        },
        other => other,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InterestRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub enabled: bool,
}

impl InterestRegion {
    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
    }

    pub fn replace(&mut self, x: u32, y: u32, width: u32, height: u32) {
        self.x = x;
        self.y = y;
        self.width = width;
        self.height = height;
        self.enabled = true;
    }
}

pub fn preview_rect(comp: &Composition, region: &InterestRegion) -> (u32, u32, u32, u32) {
    if region.enabled {
        (region.x, region.y, region.width, region.height)
    } else {
        (0, 0, comp.width, comp.height)
    }
}

pub fn file_frame_size(comp: &Composition) -> (u32, u32) {
    (comp.width, comp.height)
}

pub fn crop_comp_to_region(comp: &mut Composition, region: &InterestRegion) {
    comp.width = region.width;
    comp.height = region.height;
}

pub fn set_work_area_start(comp: &mut Composition, current: Time) {
    let end = comp.work_area_start.seconds() + comp.work_area_duration.seconds();
    let start = current.seconds().clamp(0.0, end);
    comp.work_area_start = Time::from_seconds(start);
    comp.work_area_duration = Time::from_seconds(end - start);
}

pub fn set_work_area_end(comp: &mut Composition, current: Time) {
    let start = comp.work_area_start.seconds();
    let end = current.seconds().clamp(start, comp.duration.seconds());
    comp.work_area_duration = Time::from_seconds(end - start);
}

pub fn set_work_area_to_span(comp: &mut Composition, span: Option<(Time, Time)>) {
    match span {
        None => {
            comp.work_area_start = Time::ZERO;
            comp.work_area_duration = comp.duration;
        }
        Some((start, end)) => {
            comp.work_area_start = start;
            comp.work_area_duration = Time::from_seconds((end.seconds() - start.seconds()).max(0.0));
        }
    }
}

pub const ALTERNATE_PREVIEW_FRAMES: u32 = 5;

pub fn alternate_preview(current_frame: u32, frame_count: u32, frames: u32) -> (u32, u32) {
    let end = current_frame.saturating_add(frames).min(frame_count);
    (current_frame, end)
}

pub fn skipped_frames(start: u32, end: u32, skip: u32) -> Vec<u32> {
    let step = skip.saturating_add(1).max(1) as usize;
    (start..end).step_by(step).collect()
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PreviewFps {
    Auto,
    Fixed(f64),
}

pub fn preview_fps(comp_fps: f64, setting: PreviewFps) -> f64 {
    match setting {
        PreviewFps::Auto => comp_fps,
        PreviewFps::Fixed(value) => value,
    }
}

/// スペースキーの確認できた既定。Cache Before Playback はオフ。音声は含む。
#[derive(Clone, Debug, PartialEq)]
pub struct PreviewKey {
    pub include_audio: bool,
    pub cache_before_playback: bool,
    pub range: RangeMode,
    pub from_current_time: bool,
    pub frame_rate: PreviewFps,
    pub skip: u32,
    pub resolution: PreviewResolution,
    pub alternate_frames: u32,
}

impl PreviewKey {
    pub fn spacebar() -> Self {
        Self {
            include_audio: true,
            cache_before_playback: false,
            range: RangeMode::WorkAreaExtendedByCurrentTime,
            from_current_time: false,
            frame_rate: PreviewFps::Auto,
            skip: 0,
            resolution: PreviewResolution::Auto,
            alternate_frames: ALTERNATE_PREVIEW_FRAMES,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use model::ItemId;

    fn comp() -> Composition {
        let mut comp = Composition::new(ItemId::new(1), "Comp", 100, 50, 24.0, Time::from_seconds(10.0));
        comp.work_area_start = Time::from_seconds(2.0);
        comp.work_area_duration = Time::from_seconds(4.0);
        comp
    }

    #[test]
    fn extended_range_starts_at_current_time_before_the_work_area() {
        let span = resolve_range(
            &comp(),
            1.0,
            PreviewRequest {
                range: RangeMode::WorkAreaExtendedByCurrentTime,
                from_current_time: false,
            },
        );
        assert!((span.start - 1.0).abs() < 1e-9);
        assert!((span.end - 6.0).abs() < 1e-9);
    }

    #[test]
    fn work_area_does_not_include_outside_frames() {
        let span = resolve_range(
            &comp(),
            0.0,
            PreviewRequest {
                range: RangeMode::WorkArea,
                from_current_time: false,
            },
        );
        assert!(!span.contains(1.0));
        assert!(span.contains(2.0));
        assert!(!span.contains(6.0));
    }

    #[test]
    fn entire_duration_ignores_a_short_work_area() {
        let span = resolve_range(
            &comp(),
            3.0,
            PreviewRequest {
                range: RangeMode::EntireDuration,
                from_current_time: false,
            },
        );
        assert!((span.start - 0.0).abs() < 1e-9);
        assert!((span.end - 10.0).abs() < 1e-9);
    }

    #[test]
    fn around_current_time_uses_preroll_and_postroll() {
        let span = resolve_range(
            &comp(),
            5.0,
            PreviewRequest {
                range: RangeMode::Around { preroll: 1.0, postroll: 1.0 },
                from_current_time: false,
            },
        );
        assert!((span.start - 4.0).abs() < 1e-9);
        assert!((span.end - 6.0).abs() < 1e-9);
    }

    #[test]
    fn from_current_time_starts_at_the_middle() {
        let span = resolve_range(
            &comp(),
            5.0,
            PreviewRequest {
                range: RangeMode::EntireDuration,
                from_current_time: true,
            },
        );
        assert!((span.start - 5.0).abs() < 1e-9);
        assert!((span.end - 10.0).abs() < 1e-9);
    }

    #[test]
    fn work_area_shortcuts_move_the_edges() {
        let mut comp = comp();
        set_work_area_start(&mut comp, Time::from_seconds(3.0));
        assert!((comp.work_area_start.seconds() - 3.0).abs() < 1e-9);
        set_work_area_end(&mut comp, Time::from_seconds(7.0));
        assert!((comp.work_area_start.seconds() + comp.work_area_duration.seconds() - 7.0).abs() < 1e-9);
        set_work_area_to_span(&mut comp, Some((Time::from_seconds(1.0), Time::from_seconds(2.0))));
        assert!((comp.work_area_start.seconds() - 1.0).abs() < 1e-9);
        assert!((comp.work_area_duration.seconds() - 1.0).abs() < 1e-9);
        set_work_area_to_span(&mut comp, None);
        assert!((comp.work_area_duration.seconds() - comp.duration.seconds()).abs() < 1e-9);
    }

    #[test]
    fn resolutions_divide_both_axes() {
        assert_eq!(preview_pixels(100, 50, PreviewResolution::Full), (100, 50));
        assert_eq!(preview_pixels(100, 50, PreviewResolution::Half), (50, 25));
        assert_eq!(preview_pixels(90, 30, PreviewResolution::Third), (30, 10));
        assert_eq!(preview_pixels(80, 40, PreviewResolution::Quarter), (20, 10));
        assert_eq!(
            preview_pixels(100, 50, PreviewResolution::Custom { x: 5, y: 5 }),
            (20, 10)
        );
    }

    #[test]
    fn auto_resolution_follows_the_viewer() {
        let resolution = effective_resolution(PreviewResolution::Auto, PreviewResolution::Half);
        assert_eq!(preview_pixels(100, 50, resolution), (50, 25));
        assert!((preview_fps(24.0, PreviewFps::Auto) - 24.0).abs() < 1e-9);
    }

    #[test]
    fn region_of_interest_does_not_change_the_file_and_can_crop() {
        let mut comp = comp();
        let mut region = InterestRegion { x: 10, y: 5, width: 20, height: 8, enabled: true };
        assert_eq!(preview_rect(&comp, &region), (10, 5, 20, 8));
        assert_eq!(file_frame_size(&comp), (100, 50));
        region.toggle();
        assert_eq!(preview_rect(&comp, &region).2, 100);
        region.replace(0, 0, 12, 9);
        crop_comp_to_region(&mut comp, &region);
        assert_eq!((comp.width, comp.height), (12, 9));
    }

    #[test]
    fn alternate_preview_is_five_frames_and_skip_steps() {
        assert_eq!(alternate_preview(4, 30, ALTERNATE_PREVIEW_FRAMES), (4, 9));
        assert_eq!(skipped_frames(0, 6, 1), vec![0, 2, 4]);
    }

    #[test]
    fn spacebar_includes_audio_and_does_not_cache_first() {
        let key = PreviewKey::spacebar();
        assert!(key.include_audio);
        assert!(!key.cache_before_playback);
        assert!(matches!(key.range, RangeMode::WorkAreaExtendedByCurrentTime));
    }
}
