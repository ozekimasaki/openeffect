use std::path::{Path, PathBuf};

use io_api::{OutputFormat, PostRenderAction};
use io_media::audio::{write_wav, AudioClip};
use io_media::dpx::FilmContainer;
use io_media::encode::{ExrCompression, PngCompression, TiffCompression};
use io_media::{encode_still, CineonSettings, EncodeOptions, MediaError, Raster};
use model::BitsPerChannel;

use crate::video::{h264_options, H264EncoderKind, H264Options, ProResOptions, ProResVariant, MOVIE_DEFAULT_NAME};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channels {
    Rgb,
    RgbAlpha,
    Alpha,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputAlpha {
    Straight,
    Premultiplied,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResizeQuality {
    Low,
    High,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Resize {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub lock_aspect: bool,
    pub quality: ResizeQuality,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Crop {
    pub top: i32,
    pub left: i32,
    pub bottom: i32,
    pub right: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioLayout {
    Mono,
    Stereo,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AudioSettings {
    pub enabled: bool,
    pub bits: u16,
    pub layout: AudioLayout,
    /// 選択肢の初期値は要確認。呼び出し側がレートを渡す。
    pub sample_rate: u32,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            bits: 16,
            layout: AudioLayout::Stereo,
            sample_rate: 48_000,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct OutputModule {
    pub template_name: String,
    pub format: OutputFormat,
    pub path: PathBuf,
    pub channels: Channels,
    pub depth: BitsPerChannel,
    pub alpha: OutputAlpha,
    pub resize: Option<Resize>,
    pub crop: Crop,
    pub audio: AudioSettings,
    pub starting_number: u32,
    pub use_comp_frame_number: bool,
    pub include_project_link: bool,
    pub include_source_xmp: bool,
    pub post_render: PostRenderAction,
    pub png_compression: PngCompression,
    pub jpeg_quality: u8,
    pub tiff_compression: TiffCompression,
    pub exr_compression: ExrCompression,
    pub extra_channel: bool,
    pub film: FilmContainer,
    pub cineon: CineonSettings,
    pub h264: Option<H264Options>,
    pub prores: Option<ProResOptions>,
}

impl OutputModule {
    pub fn high_quality(path: impl Into<PathBuf>) -> Self {
        Self::base("High Quality", OutputFormat::Png, path)
    }

    pub fn movie_default(path: impl Into<PathBuf>, width: u32, height: u32) -> Self {
        let mut module = Self::base(MOVIE_DEFAULT_NAME, OutputFormat::H264, path);
        module.h264 = Some(h264_options(15, width, height, H264EncoderKind::Software, false));
        module.channels = Channels::Rgb;
        module
    }

    fn base(name: &str, format: OutputFormat, path: impl Into<PathBuf>) -> Self {
        Self {
            template_name: name.into(),
            format,
            path: path.into(),
            channels: Channels::RgbAlpha,
            depth: BitsPerChannel::Bpc8,
            alpha: OutputAlpha::Straight,
            resize: None,
            crop: Crop::default(),
            audio: AudioSettings::default(),
            starting_number: 0,
            use_comp_frame_number: false,
            include_project_link: false,
            include_source_xmp: false,
            post_render: PostRenderAction::None,
            png_compression: PngCompression::Fast,
            jpeg_quality: 90,
            tiff_compression: TiffCompression::None,
            exr_compression: ExrCompression::Zip,
            extra_channel: false,
            film: FilmContainer::Dpx,
            cineon: CineonSettings::reference_points(),
            h264: None,
            prores: None,
        }
    }

    pub fn extension(&self) -> &'static str {
        if self.format == OutputFormat::Dpx && self.film == FilmContainer::Cineon45 {
            "cin"
        } else {
            self.format.extension()
        }
    }

    pub fn set_format(&mut self, format: OutputFormat) {
        self.format = format;
        if format == OutputFormat::Jpeg {
            self.channels = Channels::Rgb;
        }
        if format == OutputFormat::OpenExr {
            self.depth = BitsPerChannel::Bpc32;
        }
        if format == OutputFormat::H264 {
            self.h264 = Some(h264_options(15, 1920, 1080, H264EncoderKind::Software, self.audio.enabled));
            self.prores = None;
        }
        if format == OutputFormat::ProRes {
            self.prores = Some(ProResOptions { variant: ProResVariant::Standard422, hardware: false });
            self.h264 = None;
        }
        self.path = replace_extension(&self.path, self.extension());
    }

    pub fn format_dialog(&self) -> &'static str {
        match self.format {
            OutputFormat::Png => "png-compression",
            OutputFormat::H264 => "h264-video-audio-multiplexer",
            OutputFormat::Tiff => "tiff-compression",
            OutputFormat::OpenExr => "exr-channels-compression",
            OutputFormat::Dpx => "cineon-settings",
            OutputFormat::Jpeg => "jpeg-quality",
            OutputFormat::Wav => "wav-sample",
            OutputFormat::ProRes => "prores-variant",
        }
    }
}

pub fn output_size(comp_w: u32, comp_h: u32, module: &OutputModule) -> (u32, u32) {
    let (mut width, mut height) = (comp_w, comp_h);
    if let Some(resize) = module.resize {
        match (resize.width, resize.height, resize.lock_aspect) {
            (Some(next), _, true) => {
                height = ((u64::from(comp_h) * u64::from(next)) / u64::from(comp_w.max(1))) as u32;
                width = next;
            }
            (_, Some(next), true) => {
                width = ((u64::from(comp_w) * u64::from(next)) / u64::from(comp_h.max(1))) as u32;
                height = next;
            }
            (Some(next_w), Some(next_h), false) => {
                width = next_w;
                height = next_h;
            }
            (Some(next_w), None, false) => width = next_w,
            (None, Some(next_h), false) => height = next_h,
            (None, None, _) => {}
        }
    }
    let width = (width as i32 - module.crop.left - module.crop.right).max(1) as u32;
    let height = (height as i32 - module.crop.top - module.crop.bottom).max(1) as u32;
    (width, height)
}

pub const MAX_SEQUENCE_FRAMES: u32 = 32_766;

pub fn sequence_name(template: &str, number: u32) -> String {
    if let Some(start) = template.find('[') {
        if let Some(end) = template[start..].find(']') {
            let token = &template[start..start + end + 1];
            let digits = token.chars().filter(|ch| *ch == '#').count();
            if digits > 0 {
                return format!(
                    "{}{:0width$}{}",
                    &template[..start],
                    number,
                    &template[start + end + 1..],
                    width = digits
                );
            }
        }
    }
    template.to_string()
}

pub fn sequence_numbers(start: u32, count: u32, use_comp_frame: bool, comp_frame: u32) -> Vec<u32> {
    let origin = if use_comp_frame { start.saturating_add(comp_frame) } else { start };
    (0..count)
        .take(MAX_SEQUENCE_FRAMES as usize)
        .map(|index| origin.saturating_add(index))
        .filter(|number| *number <= MAX_SEQUENCE_FRAMES || origin > MAX_SEQUENCE_FRAMES)
        .collect()
}

pub fn pending_frames(existing: &[u32], wanted: &[u32], skip_existing: bool) -> Vec<u32> {
    if !skip_existing {
        return wanted.to_vec();
    }
    wanted.iter().copied().filter(|number| !existing.contains(number)).collect()
}

pub fn unique_output_name(stem: &str, extension: &str, existing: &[String]) -> String {
    let base = format!("{stem}.{extension}");
    if !existing.iter().any(|name| name == &base) {
        return base;
    }
    let mut index = 1u32;
    loop {
        let name = format!("{stem}_{index}.{extension}");
        if !existing.iter().any(|item| item == &name) {
            return name;
        }
        index += 1;
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsMismatch {
    pub message: String,
    pub width: u32,
    pub height: u32,
}

pub fn reconcile_h264(width: u32, height: u32) -> (u32, u32, Option<SettingsMismatch>) {
    let even_w = crate::video::even_dimension(width);
    let even_h = crate::video::even_dimension(height);
    if even_w == width && even_h == height {
        (width, height, None)
    } else {
        (
            even_w,
            even_h,
            Some(SettingsMismatch {
                message: "Settings Mismatch".into(),
                width: even_w,
                height: even_h,
            }),
        )
    }
}

pub fn write_frame(path: &Path, raster: &Raster, module: &OutputModule) -> Result<(), MediaError> {
    let mut options = EncodeOptions {
        bits: module.depth,
        include_alpha: matches!(module.channels, Channels::RgbAlpha | Channels::Alpha),
        straight: matches!(module.alpha, OutputAlpha::Straight),
        png_compression: module.png_compression,
        jpeg_quality: module.jpeg_quality,
        tiff_compression: module.tiff_compression,
        exr_compression: module.exr_compression,
        extra_channel: if module.extra_channel {
            Some(vec![0.5; raster.width as usize * raster.height as usize])
        } else {
            None
        },
    };
    if module.format == OutputFormat::OpenExr {
        options.bits = BitsPerChannel::Bpc32;
    }
    encode_still(path, raster, &options)
}

pub fn write_audio(path: &Path, module: &OutputModule, samples: &[i16]) -> Result<(), MediaError> {
    if !module.audio.enabled {
        return Ok(());
    }
    let channels = match module.audio.layout {
        AudioLayout::Mono => 1,
        AudioLayout::Stereo => 2,
    };
    write_wav(
        path,
        &AudioClip {
            sample_rate: module.audio.sample_rate,
            channels,
            bits: module.audio.bits,
            samples: samples.to_vec(),
        },
    )
}

fn replace_extension(path: &Path, extension: &str) -> PathBuf {
    let mut next = path.to_path_buf();
    next.set_extension(extension);
    next
}

#[cfg(test)]
mod tests {
    use super::*;
    use io_media::decode_still;
    use model::BitsPerChannel;
    use std::path::PathBuf;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("openeffect-render-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn format_changes_the_extension_and_dialogs_differ() {
        let mut module = OutputModule::high_quality("shot.png");
        assert!(!module.include_source_xmp);
        module.include_project_link = true;
        module.set_format(OutputFormat::Jpeg);
        assert_eq!(module.path.extension().unwrap(), "jpg");
        assert_eq!(module.channels, Channels::Rgb);
        let png = OutputModule::high_quality("a.png").format_dialog();
        let h264 = OutputModule::movie_default("a.mp4", 1920, 1080).format_dialog();
        assert_ne!(png, h264);
        assert!(module.include_project_link);
    }

    #[test]
    fn resize_lock_and_crop() {
        let mut module = OutputModule::high_quality("shot.png");
        module.resize = Some(Resize {
            width: Some(50),
            height: None,
            lock_aspect: true,
            quality: ResizeQuality::High,
        });
        assert_eq!(output_size(100, 40, &module), (50, 20));
        module.resize = Some(Resize {
            width: Some(50),
            height: Some(40),
            lock_aspect: false,
            quality: ResizeQuality::Low,
        });
        assert_eq!(output_size(100, 40, &module), (50, 40));
        module.resize = None;
        module.crop.top = 10;
        assert_eq!(output_size(100, 40, &module).1, 30);
    }

    #[test]
    fn sequence_names_and_the_frame_limit() {
        assert_eq!(sequence_name("shot_[#####].png", 1), "shot_00001.png");
        assert!(sequence_name("clip_[#####].png", 38).ends_with("00038.png"));
        let numbers = sequence_numbers(0, 1, true, 25);
        assert_eq!(numbers, vec![25]);
        assert_eq!(sequence_name("shot_[#####].png", numbers[0]), "shot_00025.png");
        let limited = sequence_numbers(1, 32_767, false, 0);
        assert_eq!(limited.len(), 32_766);
        assert!(!limited.contains(&32_767));
        assert_eq!(unique_output_name("Comp", "png", &["Comp.png".into()]), "Comp_1.png");
        assert_eq!(pending_frames(&[1, 3], &[1, 2, 3], true), vec![2]);
    }

    #[test]
    fn odd_h264_width_warns_and_becomes_even() {
        let (width, _, mismatch) = reconcile_h264(1919, 1080);
        assert_eq!(width % 2, 0);
        assert!(mismatch.unwrap().message.contains("Settings Mismatch"));
    }

    #[test]
    fn dpx_keeps_alpha_and_cineon_uses_cin() {
        let mut module = OutputModule::high_quality("shot.mov");
        module.channels = Channels::RgbAlpha;
        module.set_format(OutputFormat::Dpx);
        assert_eq!(module.channels, Channels::RgbAlpha);
        assert_eq!(module.extension(), "dpx");
        module.film = FilmContainer::Cineon45;
        assert_eq!(module.extension(), "cin");
    }

    #[test]
    fn movie_default_is_the_fifteen_megabit_preset_without_xmp() {
        let module = OutputModule::movie_default("movie.mp4", 1920, 1080);
        assert_eq!(module.template_name, MOVIE_DEFAULT_NAME);
        assert!(!module.include_source_xmp);
        let preset = module.h264.clone().unwrap();
        assert_eq!(preset.bitrate_mbps, 15);
        let four_k = h264_options(40, 3840, 2160, H264EncoderKind::Software, true);
        assert_ne!(preset.level, four_k.level);
        assert!(four_k.audio);
    }

    #[test]
    fn png_sequence_depth_and_straight_alpha_roundtrip() {
        let dir = temp_dir("pngseq");
        let mut module = OutputModule::high_quality(dir.join("shot_[#####].png"));
        let raster = Raster::solid(2, 2, BitsPerChannel::Bpc8, [1.0, 0.0, 0.0, 0.5]);
        for index in 1..=10 {
            let path = dir.join(sequence_name("shot_[#####].png", index));
            write_frame(&path, &raster, &module).unwrap();
        }
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 10);
        let decoded = decode_still(&dir.join("shot_00001.png")).unwrap();
        assert!(decoded.raster.has_alpha());
        assert!((decoded.raster.get(0, 0)[0] - 1.0).abs() < 0.02);

        module.depth = BitsPerChannel::Bpc16;
        let deep = dir.join("deep.png");
        write_frame(&deep, &raster, &module).unwrap();
        assert_eq!(decode_still(&deep).unwrap().raster.bits, BitsPerChannel::Bpc16);
    }

    #[test]
    fn wav_roundtrip_matches_depth_and_channels() {
        let dir = temp_dir("wavout");
        let mut module = OutputModule::high_quality(dir.join("mix.wav"));
        module.set_format(OutputFormat::Wav);
        module.audio.enabled = true;
        module.audio.bits = 16;
        module.audio.layout = AudioLayout::Stereo;
        module.audio.sample_rate = 48_000;
        let samples = vec![0; 48_000];
        write_audio(&module.path, &module, &samples).unwrap();
        let clip = io_media::read_audio(&module.path).unwrap();
        assert_eq!(clip.bits, 16);
        assert_eq!(clip.channels, 2);
        assert!((clip.duration().seconds() - 0.5).abs() < 1e-3);
    }
}
