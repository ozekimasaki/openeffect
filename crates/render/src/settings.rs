use model::{BitsPerChannel, Composition};

use crate::preview::preview_pixels;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderTemplate {
    Best,
    Draft,
    Dv,
    MultiMachine,
}

impl RenderTemplate {
    pub fn name(self) -> &'static str {
        match self {
            Self::Best => "Best Settings",
            Self::Draft => "Draft Settings",
            Self::Dv => "DV Settings",
            Self::MultiMachine => "Multi-Machine Settings",
        }
    }
}

/// レイヤー品質として確認できた名前。ダイアログの文言が一致するかは要確認。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderQuality {
    Best,
    Draft,
    Wireframe,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResolutionChoice {
    Full,
    Half,
    Third,
    Quarter,
    Custom(u32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiskCacheMode {
    CurrentSettings,
    ReadOnly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProxyUse {
    CurrentSettings,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectsMode {
    CurrentSettings,
    AllOn,
    AllOff,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoloMode {
    CurrentSettings,
    AllOff,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuideMode {
    CurrentSettings,
    AllOff,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderDepth {
    CurrentSettings,
    Fixed(BitsPerChannel),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameBlendMode {
    OnForCheckedLayers,
    Off,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldRender {
    Off,
    LowerFirst,
    /// 表記は要確認。
    UpperFirst,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MotionBlurMode {
    CurrentSettings,
    OnForCheckedLayers,
    OffForAllLayers,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TimeSpanMode {
    LengthOfComp,
    WorkAreaOnly,
    Custom { start: f64, end: f64 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FrameRateMode {
    UseComp,
    UseThis(f64),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderLog {
    ErrorsOnly,
    PlusSettings,
    PlusPerFrameInfo,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RenderSettings {
    pub template: RenderTemplate,
    pub quality: RenderQuality,
    pub resolution: ResolutionChoice,
    pub disk_cache: DiskCacheMode,
    pub proxy_use: ProxyUse,
    pub effects: EffectsMode,
    pub solo: SoloMode,
    pub guide_layers: GuideMode,
    pub color_depth: RenderDepth,
    pub frame_blending: FrameBlendMode,
    pub field_render: FieldRender,
    pub pulldown_phase: Option<u8>,
    pub motion_blur: MotionBlurMode,
    pub time_span: TimeSpanMode,
    pub frame_rate: FrameRateMode,
    pub skip_existing_files: bool,
    pub log: RenderLog,
}

impl RenderSettings {
    pub fn from_template(template: RenderTemplate) -> Self {
        let mut settings = Self {
            template,
            quality: RenderQuality::Best,
            resolution: ResolutionChoice::Full,
            disk_cache: DiskCacheMode::CurrentSettings,
            proxy_use: ProxyUse::CurrentSettings,
            effects: EffectsMode::CurrentSettings,
            solo: SoloMode::CurrentSettings,
            guide_layers: GuideMode::AllOff,
            color_depth: RenderDepth::CurrentSettings,
            frame_blending: FrameBlendMode::OnForCheckedLayers,
            field_render: FieldRender::Off,
            pulldown_phase: None,
            motion_blur: MotionBlurMode::CurrentSettings,
            time_span: TimeSpanMode::LengthOfComp,
            frame_rate: FrameRateMode::UseComp,
            skip_existing_files: false,
            log: RenderLog::ErrorsOnly,
        };
        match template {
            RenderTemplate::Best => {}
            RenderTemplate::Draft => settings.quality = RenderQuality::Draft,
            RenderTemplate::Dv => settings.field_render = FieldRender::LowerFirst,
            RenderTemplate::MultiMachine => settings.skip_existing_files = true,
        }
        settings
    }
}

pub fn render_resolution(width: u32, height: u32, choice: ResolutionChoice) -> (u32, u32) {
    let resolution = match choice {
        ResolutionChoice::Full => crate::preview::PreviewResolution::Full,
        ResolutionChoice::Half => crate::preview::PreviewResolution::Half,
        ResolutionChoice::Third => crate::preview::PreviewResolution::Third,
        ResolutionChoice::Quarter => crate::preview::PreviewResolution::Quarter,
        ResolutionChoice::Custom(divisor) => crate::preview::PreviewResolution::Custom { x: divisor, y: divisor },
    };
    preview_pixels(width, height, resolution)
}

pub fn time_span(comp: &Composition, settings: &RenderSettings) -> (f64, f64) {
    match settings.time_span {
        TimeSpanMode::LengthOfComp => (0.0, comp.duration.seconds()),
        TimeSpanMode::WorkAreaOnly => {
            let start = comp.work_area_start.seconds();
            (start, start + comp.work_area_duration.seconds())
        }
        TimeSpanMode::Custom { start, end } => (start, end),
    }
}

pub fn sampling_rate(comp: &Composition, settings: &RenderSettings) -> f64 {
    let rate = match settings.frame_rate {
        FrameRateMode::UseComp => comp.frame_rate,
        FrameRateMode::UseThis(rate) => rate,
    };
    let rate = if settings.pulldown_phase.is_some() && (comp.frame_rate - 24.0).abs() < 0.1 {
        29.97
    } else {
        rate
    };
    rate.clamp(0.0, 999.0)
}

pub fn frame_count(comp: &Composition, settings: &RenderSettings) -> u32 {
    let (start, end) = time_span(comp, settings);
    let frames = ((end - start).max(0.0) * sampling_rate(comp, settings)).round();
    frames as u32
}

pub fn resolved_depth(project: BitsPerChannel, settings: &RenderSettings) -> BitsPerChannel {
    match settings.color_depth {
        RenderDepth::CurrentSettings => project,
        RenderDepth::Fixed(bits) => bits,
    }
}

#[derive(Clone, Debug, Default)]
pub struct FrameCache {
    frames: Vec<u32>,
    pub read_only: bool,
}

impl FrameCache {
    pub fn store(&mut self, frame: u32) -> bool {
        if self.read_only {
            return false;
        }
        self.frames.push(frame);
        true
    }

    pub fn len(&self) -> usize {
        self.frames.len()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }
}

pub fn render_log(comp: &Composition, settings: &RenderSettings, had_error: bool) -> Option<String> {
    match settings.log {
        RenderLog::ErrorsOnly if !had_error => None,
        RenderLog::ErrorsOnly => Some("error".into()),
        RenderLog::PlusSettings | RenderLog::PlusPerFrameInfo => {
            let (width, height) = render_resolution(comp.width, comp.height, settings.resolution);
            Some(format!("{width}x{height} {fps}", fps = sampling_rate(comp, settings)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use model::{ItemId, Time};

    fn comp() -> Composition {
        let mut comp = Composition::new(ItemId::new(1), "Comp", 1920, 1080, 30.0, Time::from_seconds(2.0));
        comp.work_area_start = Time::from_seconds(0.5);
        comp.work_area_duration = Time::from_seconds(1.0);
        comp
    }

    #[test]
    fn templates_expose_the_confirmed_names_and_switches() {
        assert_eq!(RenderTemplate::Best.name(), "Best Settings");
        assert_eq!(RenderTemplate::Draft.name(), "Draft Settings");
        assert_eq!(RenderSettings::from_template(RenderTemplate::Dv).field_render, FieldRender::LowerFirst);
        assert!(RenderSettings::from_template(RenderTemplate::MultiMachine).skip_existing_files);
        assert_eq!(
            RenderSettings::from_template(RenderTemplate::Best).guide_layers,
            GuideMode::AllOff
        );
    }

    #[test]
    fn half_resolution_does_not_change_the_composition() {
        let comp = comp();
        assert_eq!(render_resolution(comp.width, comp.height, ResolutionChoice::Half), (960, 540));
        assert_eq!((comp.width, comp.height), (1920, 1080));
    }

    #[test]
    fn work_area_span_matches_and_custom_rate_counts_frames() {
        let comp = comp();
        let mut settings = RenderSettings::from_template(RenderTemplate::Best);
        settings.time_span = TimeSpanMode::WorkAreaOnly;
        let (start, end) = time_span(&comp, &settings);
        assert!((end - start - comp.work_area_duration.seconds()).abs() < 1e-9);
        settings.time_span = TimeSpanMode::LengthOfComp;
        settings.frame_rate = FrameRateMode::UseThis(12.0);
        assert_eq!(frame_count(&comp, &settings), 24);
    }

    #[test]
    fn pulldown_raises_a_24_fps_comp() {
        let mut comp = comp();
        comp.frame_rate = 24.0;
        comp.duration = Time::from_seconds(1.0);
        let mut settings = RenderSettings::from_template(RenderTemplate::Best);
        settings.pulldown_phase = Some(0);
        assert!(frame_count(&comp, &settings) > 24);
    }

    #[test]
    fn current_depth_follows_the_project() {
        let settings = RenderSettings::from_template(RenderTemplate::Best);
        assert_eq!(resolved_depth(BitsPerChannel::Bpc16, &settings), BitsPerChannel::Bpc16);
    }

    #[test]
    fn plus_settings_log_contains_resolution_and_rate() {
        let comp = comp();
        let mut settings = RenderSettings::from_template(RenderTemplate::Best);
        settings.log = RenderLog::PlusSettings;
        let text = render_log(&comp, &settings, false).unwrap();
        assert!(text.contains("1920x1080"));
        assert!(text.contains("30"));
    }

    #[test]
    fn read_only_cache_does_not_grow() {
        let mut cache = FrameCache { read_only: true, ..FrameCache::default() };
        assert!(!cache.store(1));
        assert!(cache.is_empty());
        cache.read_only = false;
        assert!(cache.store(1));
        assert_eq!(cache.len(), 1);
    }
}
