use model::Time;

use crate::cineon::CineonSettings;
use crate::raster::Raster;

/// アルファの解釈。Guess は確定前の選択。
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AlphaMode {
    Guess,
    Ignore,
    Straight,
    Premultiplied { matte: [f32; 3] },
}

/// ラベルの無いアルファの読み込み時の扱い。初期選択は要確認。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnlabeledAlpha {
    AskUser,
    Guess,
    Ignore,
    Straight,
    PremultipliedBlack,
    PremultipliedWhite,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PixelAspect {
    Square,
    D1Ntsc,
    D1NtscWidescreen,
    D1Pal,
    D1PalWidescreen,
    /// HDV 1080、DVCPRO HD 720、HD Anamorphic 1080。
    Hdv1080,
    DvcproHd1080,
    Custom(f64),
}

impl PixelAspect {
    pub fn ratio(self) -> f64 {
        match self {
            Self::Square => 1.0,
            Self::D1Ntsc => 0.91,
            Self::D1NtscWidescreen => 1.21,
            Self::D1Pal => 1.09,
            Self::D1PalWidescreen => 1.46,
            Self::Hdv1080 => 1.33,
            Self::DvcproHd1080 => 1.5,
            Self::Custom(value) => value,
        }
    }
}

/// フィールド順のメニュー名は要確認。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldOrder {
    Off,
    UpperFirst,
    LowerFirst,
}

/// プルダウン位相の名称は要確認。位相は 0 からの番号で持つ。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pulldown {
    Off,
    Remove32 { phase: u8 },
    Remove24Pa,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Interpretation {
    pub alpha: AlphaMode,
    pub invert_alpha: bool,
    pub conform_frame_rate: Option<f64>,
    pub pixel_aspect: PixelAspect,
    pub fields: FieldOrder,
    pub pulldown: Pulldown,
    /// 繰り返し回数。1 は 1 周。ダイアログの初期値は要確認。
    pub loops: u32,
    pub color_profile: Option<String>,
    pub cineon: CineonSettings,
}

impl Interpretation {
    pub fn new() -> Self {
        Self {
            alpha: AlphaMode::Guess,
            invert_alpha: false,
            conform_frame_rate: None,
            pixel_aspect: PixelAspect::Square,
            fields: FieldOrder::Off,
            pulldown: Pulldown::Off,
            loops: 1,
            color_profile: None,
            cineon: CineonSettings::reference_points(),
        }
    }

    pub fn persist(&self) -> String {
        format!(
            "alpha={}\ninvert={}\nfps={}\npar={}\nfields={}\npulldown={}\nloops={}\nprofile={}\ncineon_white={}\ncineon_black={}\nrolloff={}\ngamma={}\nlog={}\n",
            alpha_token(self.alpha),
            self.invert_alpha,
            self.conform_frame_rate.unwrap_or(0.0),
            self.pixel_aspect.ratio(),
            field_token(self.fields),
            pulldown_token(self.pulldown),
            self.loops,
            self.color_profile.as_deref().unwrap_or(""),
            self.cineon.white_10bit,
            self.cineon.black_10bit,
            self.cineon.highlight_rolloff,
            self.cineon.gamma,
            self.cineon.logarithmic,
        )
    }

    pub fn from_persist(text: &str) -> Self {
        let mut value = Self::new();
        for line in text.lines() {
            let Some((key, raw)) = line.split_once('=') else {
                continue;
            };
            match key {
                "alpha" => value.alpha = alpha_from_token(raw),
                "invert" => value.invert_alpha = raw == "true",
                "fps" => {
                    let fps: f64 = raw.parse().unwrap_or(0.0);
                    value.conform_frame_rate = if fps > 0.0 { Some(fps) } else { None };
                }
                "par" => value.pixel_aspect = aspect_from_ratio(raw.parse().unwrap_or(1.0)),
                "fields" => value.fields = field_from_token(raw),
                "pulldown" => value.pulldown = pulldown_from_token(raw),
                "loops" => value.loops = raw.parse().unwrap_or(1),
                "profile" => {
                    value.color_profile = if raw.is_empty() { None } else { Some(raw.to_string()) };
                }
                "cineon_white" => value.cineon.white_10bit = raw.parse().unwrap_or(685.0),
                "cineon_black" => value.cineon.black_10bit = raw.parse().unwrap_or(95.0),
                "rolloff" => value.cineon.highlight_rolloff = raw.parse().unwrap_or(0.0),
                "gamma" => value.cineon.gamma = raw.parse().unwrap_or(1.0),
                "log" => value.cineon.logarithmic = raw == "true",
                _ => {}
            }
        }
        value
    }
}

impl Default for Interpretation {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedFootage {
    pub raster: Raster,
    pub duration: Time,
    pub frame_rate: f64,
    pub pixel_aspect: f64,
}

pub fn apply_interpretation(
    raster: &Raster,
    source_frames: u32,
    source_fps: f64,
    interpretation: &Interpretation,
) -> ResolvedFootage {
    let alpha = match interpretation.alpha {
        AlphaMode::Guess => guess_alpha(raster, None),
        other => other,
    };
    let mut pixels = apply_alpha(raster, alpha, interpretation.invert_alpha);
    if interpretation.fields != FieldOrder::Off {
        pixels = separate_fields(&pixels, interpretation.fields);
    }
    let frame_rate = interpretation.conform_frame_rate.unwrap_or(source_fps);
    let duration = looped_duration(conform_duration(source_frames, frame_rate), interpretation.loops.max(1));
    ResolvedFootage {
        raster: pixels,
        duration,
        frame_rate: remove_pulldown_frame_rate(frame_rate, interpretation.pulldown),
        pixel_aspect: interpretation.pixel_aspect.ratio(),
    }
}

pub fn guess_alpha(raster: &Raster, labeled: Option<AlphaMode>) -> AlphaMode {
    if let Some(labeled) = labeled {
        return labeled;
    }
    if !raster.has_alpha() {
        return AlphaMode::Ignore;
    }
    let mut straight = 0;
    let mut premul = 0;
    for y in 0..raster.height {
        for x in 0..raster.width {
            let pixel = raster.get(x, y);
            if pixel[3] > 0.02 && pixel[3] < 0.98 {
                let max_rgb = pixel[0].max(pixel[1]).max(pixel[2]);
                if max_rgb > pixel[3] + 0.05 {
                    straight += 1;
                } else {
                    premul += 1;
                }
            }
        }
    }
    if straight == 0 && premul == 0 {
        AlphaMode::Straight
    } else if straight >= premul {
        AlphaMode::Straight
    } else {
        AlphaMode::Premultiplied { matte: [0.0, 0.0, 0.0] }
    }
}

pub fn apply_alpha(raster: &Raster, mode: AlphaMode, invert: bool) -> Raster {
    raster.map_pixels(|pixel| {
        let mut next = match mode {
            AlphaMode::Ignore | AlphaMode::Guess => [pixel[0], pixel[1], pixel[2], 1.0],
            AlphaMode::Straight => pixel,
            AlphaMode::Premultiplied { matte } => unpremultiply(pixel, matte),
        };
        if invert {
            next[3] = 1.0 - next[3];
        }
        next
    })
}

fn unpremultiply(pixel: [f32; 4], matte: [f32; 3]) -> [f32; 4] {
    let alpha = pixel[3];
    if alpha <= 1e-5 {
        return [0.0, 0.0, 0.0, 0.0];
    }
    [
        (pixel[0] - matte[0] * (1.0 - alpha)) / alpha,
        (pixel[1] - matte[1] * (1.0 - alpha)) / alpha,
        (pixel[2] - matte[2] * (1.0 - alpha)) / alpha,
        alpha,
    ]
}

pub fn separate_fields(raster: &Raster, order: FieldOrder) -> Raster {
    let start = match order {
        FieldOrder::Off => return raster.clone(),
        FieldOrder::UpperFirst => 0,
        FieldOrder::LowerFirst => 1,
    };
    let height = (raster.height / 2).max(1);
    let mut out = Raster::solid(raster.width, height, raster.bits, [0.0, 0.0, 0.0, 0.0]);
    for y in 0..height {
        let source_y = y * 2 + start;
        if source_y >= raster.height {
            break;
        }
        for x in 0..raster.width {
            out.set(x, y, raster.get(x, source_y));
        }
    }
    out
}

pub fn conform_duration(frames: u32, frame_rate: f64) -> Time {
    Time::from_frames(f64::from(frames), frame_rate)
}

pub fn looped_duration(source: Time, loops: u32) -> Time {
    Time::from_seconds(source.seconds() * f64::from(loops.max(1)))
}

pub fn remove_pulldown_frame_rate(source_fps: f64, pulldown: Pulldown) -> f64 {
    match pulldown {
        Pulldown::Off => source_fps,
        Pulldown::Remove32 { .. } => {
            if (source_fps - 29.97).abs() < 0.02 {
                24.0
            } else {
                source_fps * 4.0 / 5.0
            }
        }
        Pulldown::Remove24Pa => 24.0,
    }
}

pub fn remove_repeated_frames(frames: &[u32]) -> Vec<u32> {
    let mut out = Vec::new();
    for frame in frames {
        if out.last() != Some(frame) {
            out.push(*frame);
        }
    }
    out
}

pub fn guess_pixel_aspect(width: u32, height: u32, tagged: Option<PixelAspect>) -> PixelAspect {
    if let Some(tagged) = tagged {
        return tagged;
    }
    match (width, height) {
        (720, 486) | (720, 480) => PixelAspect::D1Ntsc,
        (720, 576) => PixelAspect::D1Pal,
        _ => PixelAspect::Square,
    }
}

pub fn vector_pixel_size(width: f64, height: f64) -> (u32, u32) {
    (width.ceil().max(1.0) as u32, height.ceil().max(1.0) as u32)
}

#[derive(Clone, Debug, PartialEq)]
pub struct InterpretDialog {
    committed: Interpretation,
    draft: Interpretation,
}

impl InterpretDialog {
    pub fn new(interpretation: Interpretation) -> Self {
        Self {
            committed: interpretation.clone(),
            draft: interpretation,
        }
    }

    pub fn draft_mut(&mut self) -> &mut Interpretation {
        &mut self.draft
    }

    pub fn preview(&self, raster: &Raster, frames: u32, fps: f64) -> ResolvedFootage {
        apply_interpretation(raster, frames, fps, &self.draft)
    }

    pub fn committed_view(&self, raster: &Raster, frames: u32, fps: f64) -> ResolvedFootage {
        apply_interpretation(raster, frames, fps, &self.committed)
    }

    pub fn ok(&mut self) -> Interpretation {
        self.committed = self.draft.clone();
        self.committed.clone()
    }

    pub fn is_committed(&self) -> bool {
        self.committed == self.draft
    }
}

#[derive(Clone, Debug, Default)]
pub struct InterpretationMemory {
    stored: Option<Interpretation>,
}

impl InterpretationMemory {
    pub fn remember(&mut self, interpretation: &Interpretation) {
        self.stored = Some(interpretation.clone());
    }

    pub fn apply_to(&self, targets: &mut [&mut Interpretation]) -> bool {
        let Some(stored) = &self.stored else {
            return false;
        };
        for target in targets {
            **target = stored.clone();
        }
        true
    }
}

fn alpha_token(mode: AlphaMode) -> String {
    match mode {
        AlphaMode::Guess => "guess".into(),
        AlphaMode::Ignore => "ignore".into(),
        AlphaMode::Straight => "straight".into(),
        AlphaMode::Premultiplied { matte } => format!("premul:{},{},{}", matte[0], matte[1], matte[2]),
    }
}

fn alpha_from_token(token: &str) -> AlphaMode {
    match token {
        "ignore" => AlphaMode::Ignore,
        "straight" => AlphaMode::Straight,
        "guess" => AlphaMode::Guess,
        _ if token.starts_with("premul:") => {
            let mut parts = token.trim_start_matches("premul:").split(',');
            let r = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0.0);
            let g = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0.0);
            let b = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0.0);
            AlphaMode::Premultiplied { matte: [r, g, b] }
        }
        _ => AlphaMode::Guess,
    }
}

fn field_token(order: FieldOrder) -> &'static str {
    match order {
        FieldOrder::Off => "off",
        FieldOrder::UpperFirst => "upper",
        FieldOrder::LowerFirst => "lower",
    }
}

fn field_from_token(token: &str) -> FieldOrder {
    match token {
        "upper" => FieldOrder::UpperFirst,
        "lower" => FieldOrder::LowerFirst,
        _ => FieldOrder::Off,
    }
}

fn pulldown_token(pulldown: Pulldown) -> String {
    match pulldown {
        Pulldown::Off => "off".into(),
        Pulldown::Remove32 { phase } => format!("32:{phase}"),
        Pulldown::Remove24Pa => "24pa".into(),
    }
}

fn pulldown_from_token(token: &str) -> Pulldown {
    if token == "24pa" {
        Pulldown::Remove24Pa
    } else if let Some(phase) = token.strip_prefix("32:") {
        Pulldown::Remove32 {
            phase: phase.parse().unwrap_or(0),
        }
    } else {
        Pulldown::Off
    }
}

fn aspect_from_ratio(ratio: f64) -> PixelAspect {
    const KNOWN: [PixelAspect; 7] = [
        PixelAspect::Square,
        PixelAspect::D1Ntsc,
        PixelAspect::D1NtscWidescreen,
        PixelAspect::D1Pal,
        PixelAspect::D1PalWidescreen,
        PixelAspect::Hdv1080,
        PixelAspect::DvcproHd1080,
    ];
    KNOWN
        .into_iter()
        .find(|aspect| (aspect.ratio() - ratio).abs() < 0.001)
        .unwrap_or(PixelAspect::Custom(ratio))
}

#[cfg(test)]
mod tests {
    use super::*;
    use model::BitsPerChannel;

    fn edge() -> Raster {
        let mut raster = Raster::solid(2, 1, BitsPerChannel::Bpc8, [0.0, 0.0, 0.0, 1.0]);
        raster.set(0, 0, [1.0, 0.0, 0.0, 0.5]);
        raster
    }

    #[test]
    fn labeled_straight_png_keeps_the_edge_color() {
        let raster = edge();
        let guessed = guess_alpha(&raster, Some(AlphaMode::Straight));
        assert_eq!(guessed, AlphaMode::Straight);
        let applied = apply_alpha(&raster, guessed, false);
        let pixel = applied.get(0, 0);
        assert!((pixel[0] - 1.0).abs() < 1e-5);
        assert!((pixel[3] - 0.5).abs() < 1e-5);
    }

    #[test]
    fn ignore_alpha_makes_the_frame_opaque() {
        let applied = apply_alpha(&edge(), AlphaMode::Ignore, false);
        assert!((applied.get(0, 0)[3] - 1.0).abs() < 1e-5);
    }

    #[test]
    fn straight_edge_does_not_mix_a_background() {
        let applied = apply_alpha(&edge(), AlphaMode::Straight, false);
        assert!((applied.get(0, 0)[0] - 1.0).abs() < 1e-5);
    }

    #[test]
    fn black_matte_removes_the_dark_fringe() {
        let mut raster = Raster::solid(1, 1, BitsPerChannel::Bpc8, [0.0, 0.0, 0.0, 1.0]);
        raster.set(0, 0, [0.5, 0.0, 0.0, 0.5]);
        let applied = apply_alpha(
            &raster,
            AlphaMode::Premultiplied { matte: [0.0, 0.0, 0.0] },
            false,
        );
        assert!((applied.get(0, 0)[0] - 1.0).abs() < 1e-4);
    }

    #[test]
    fn invert_alpha_swaps_transparency() {
        let before = edge().get(0, 0)[3];
        let after = apply_alpha(&edge(), AlphaMode::Straight, true).get(0, 0)[3];
        assert!((after - (1.0 - before)).abs() < 1e-5);
    }

    #[test]
    fn conform_changes_duration_without_changing_frame_count() {
        let ten = conform_duration(10, 5.0);
        assert!((ten.seconds() - 2.0).abs() < 1e-9);
        let source = conform_duration(15, 15.0);
        let faster = conform_duration(15, 30.0);
        assert!((source.seconds() / faster.seconds() - 2.0).abs() < 1e-9);
    }

    #[test]
    fn unlabeled_720x480_is_d1_ntsc_and_can_return_to_square() {
        let guessed = guess_pixel_aspect(720, 480, None);
        assert_eq!(guessed, PixelAspect::D1Ntsc);
        assert!((guessed.ratio() - 0.91).abs() < 1e-9);
        let square = PixelAspect::Square;
        assert!((square.ratio() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn field_separation_halves_the_comb() {
        let mut raster = Raster::solid(1, 4, BitsPerChannel::Bpc8, [0.0, 0.0, 0.0, 1.0]);
        raster.set(0, 0, [1.0, 1.0, 1.0, 1.0]);
        raster.set(0, 2, [1.0, 1.0, 1.0, 1.0]);
        let field = separate_fields(&raster, FieldOrder::UpperFirst);
        assert_eq!(field.height, 2);
        assert!(field.get(0, 0)[0] > 0.9);
        assert!(field.get(0, 1)[0] > 0.9);
    }

    #[test]
    fn ntsc_pulldown_becomes_24() {
        let fps = remove_pulldown_frame_rate(29.97, Pulldown::Remove32 { phase: 0 });
        assert!((fps - 24.0).abs() < 1e-6);
    }

    #[test]
    fn twenty_four_pa_drops_repeated_fields() {
        let frames = remove_repeated_frames(&[1, 1, 2, 3, 3]);
        assert_eq!(frames, vec![1, 2, 3]);
        assert!((remove_pulldown_frame_rate(29.97, Pulldown::Remove24Pa) - 24.0).abs() < 1e-6);
    }

    #[test]
    fn loop_three_triples_duration() {
        let once = conform_duration(10, 10.0);
        let looped = looped_duration(once, 3);
        assert!((looped.seconds() / once.seconds() - 3.0).abs() < 1e-9);
    }

    #[test]
    fn profile_and_frame_rate_survive_reopen() {
        let mut interpretation = Interpretation::new();
        interpretation.conform_frame_rate = Some(24.0);
        interpretation.color_profile = Some("sRGB IEC61966-2.1".into());
        interpretation.cineon.white_10bit = 500.0;
        let restored = Interpretation::from_persist(&interpretation.persist());
        assert_eq!(restored.conform_frame_rate, Some(24.0));
        assert_eq!(restored.color_profile.as_deref(), Some("sRGB IEC61966-2.1"));
        assert!((restored.cineon.white_10bit - 500.0).abs() < 1e-4);
    }

    #[test]
    fn preview_does_not_commit_until_ok() {
        let raster = edge();
        let mut dialog = InterpretDialog::new(Interpretation::new());
        dialog.draft_mut().alpha = AlphaMode::Ignore;
        let preview = dialog.preview(&raster, 1, 30.0);
        let committed = dialog.committed_view(&raster, 1, 30.0);
        assert!(preview.raster.get(0, 0)[3] > 0.9);
        assert!(committed.raster.get(0, 0)[3] < 0.9);
        dialog.ok();
        assert!(dialog.committed_view(&raster, 1, 30.0).raster.get(0, 0)[3] > 0.9);
    }

    #[test]
    fn remembered_interpretation_matches_on_three_items() {
        let mut source = Interpretation::new();
        source.alpha = AlphaMode::Straight;
        source.conform_frame_rate = Some(12.0);
        let mut memory = InterpretationMemory::default();
        memory.remember(&source);
        let mut a = Interpretation::new();
        let mut b = Interpretation::new();
        let mut c = Interpretation::new();
        memory.apply_to(&mut [&mut a, &mut b, &mut c]);
        assert_eq!(a.alpha, AlphaMode::Straight);
        assert_eq!(b.conform_frame_rate, Some(12.0));
        assert_eq!(c.conform_frame_rate, a.conform_frame_rate);
    }

    #[test]
    fn fractional_vector_size_ceils_each_side() {
        assert_eq!(vector_pixel_size(216.5, 275.5), (217, 276));
    }
}
