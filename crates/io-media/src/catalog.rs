//! 拡張子と、読み書きの可否。公式ヘルプで文として確認できたものだけを載せる。

/// コンテナや静止画の種類。重なる拡張子は `kinds_for_extension` が複数返す。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MediaKind {
    Png,
    Jpeg,
    Tiff,
    Psd,
    Bmp,
    Gif,
    OpenExr,
    Cineon,
    Dpx,
    Radiance,
    Sgi,
    Targa,
    MayaIff,
    SoftimagePic,
    Pcx,
    Heif,
    CameraRaw,
    Pict,
    ElectricImage,
    Svg,
    QuickTime,
    Avi,
    Mpeg4,
    Hevc,
    Mpeg1,
    Mpeg2,
    Mxf,
    WindowsMedia,
    FlashVideo,
    ThreeGpp,
    Avchd,
    RedR3d,
    CanonCrm,
    Swf,
    Omf,
    Wav,
    Aiff,
    Mp3,
    Aac,
    Asnd,
    Project,
}

/// 純 Rust で画素を往復できる範囲。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PureRust {
    None,
    Read,
    ReadWrite,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FormatInfo {
    pub kind: MediaKind,
    pub import: bool,
    pub export: bool,
    pub still: bool,
    pub video: bool,
    pub audio: bool,
    pub windows_only: bool,
    pub pure_rust: PureRust,
}

struct KindRow {
    kind: MediaKind,
    extensions: &'static [&'static str],
    import: bool,
    export: bool,
    still: bool,
    video: bool,
    audio: bool,
    windows_only: bool,
    pure_rust: PureRust,
}

const KINDS: &[KindRow] = &[
    row(MediaKind::Png, &["png"], true, true, true, false, false, false, PureRust::ReadWrite),
    row(MediaKind::Jpeg, &["jpg", "jpe"], true, true, true, false, false, false, PureRust::ReadWrite),
    row(MediaKind::Tiff, &["tif", "tiff"], true, true, true, false, false, false, PureRust::ReadWrite),
    row(MediaKind::Psd, &["psd"], true, false, true, false, false, false, PureRust::Read),
    row(MediaKind::Bmp, &["bmp", "rle", "dib"], true, false, true, false, false, false, PureRust::Read),
    row(MediaKind::Gif, &["gif"], true, false, true, false, false, false, PureRust::Read),
    row(MediaKind::OpenExr, &["exr", "sxr", "mxr"], true, true, true, false, false, false, PureRust::ReadWrite),
    row(MediaKind::Cineon, &["cin"], true, true, true, false, false, false, PureRust::ReadWrite),
    row(MediaKind::Dpx, &["dpx"], true, true, true, false, false, false, PureRust::ReadWrite),
    row(MediaKind::Radiance, &["hdr", "rgbe", "xyze"], true, true, true, false, false, false, PureRust::ReadWrite),
    row(MediaKind::Sgi, &["sgi", "bw", "rgb"], true, true, true, false, false, false, PureRust::None),
    row(MediaKind::Targa, &["tga", "vda", "icb", "vst"], true, true, true, false, false, false, PureRust::ReadWrite),
    row(MediaKind::MayaIff, &["iff", "tdi"], true, true, true, false, false, false, PureRust::None),
    row(MediaKind::SoftimagePic, &["pic"], true, false, true, false, false, false, PureRust::None),
    row(MediaKind::Pcx, &["pcx"], true, false, true, false, false, true, PureRust::None),
    row(MediaKind::Heif, &["heif", "heic"], true, false, true, false, false, false, PureRust::None),
    row(
        MediaKind::CameraRaw,
        &["crw", "nef", "raf", "orf", "mrw", "dcr", "mos", "raw", "pef", "srf", "dng", "x3f", "cr2", "erf", "sr2", "mfw", "mef", "arw"],
        true,
        false,
        true,
        false,
        false,
        false,
        PureRust::None,
    ),
    row(MediaKind::Pict, &["pct"], true, false, true, false, false, false, PureRust::None),
    row(MediaKind::ElectricImage, &["img", "ei"], true, false, true, false, false, false, PureRust::None),
    row(MediaKind::Svg, &["svg"], true, false, true, false, false, false, PureRust::Read),
    row(MediaKind::QuickTime, &["mov"], true, true, false, true, false, false, PureRust::None),
    row(MediaKind::Avi, &["avi"], true, true, false, true, false, false, PureRust::None),
    row(MediaKind::Mpeg4, &["mp4", "m4v"], true, false, false, true, false, false, PureRust::None),
    row(MediaKind::Hevc, &["hevc"], true, false, false, true, false, false, PureRust::None),
    row(MediaKind::Mpeg1, &["mpg", "mpe", "mpa", "mpv", "mod"], true, false, false, true, false, false, PureRust::None),
    row(MediaKind::Mpeg2, &["mpg", "m2p", "m2v", "m2a", "m2t", "m2ts"], true, false, false, true, false, false, PureRust::None),
    row(MediaKind::Mxf, &["mxf"], true, false, false, true, false, false, PureRust::None),
    row(MediaKind::WindowsMedia, &["wmv", "wma", "asf", "asx"], true, false, false, true, true, true, PureRust::None),
    row(MediaKind::FlashVideo, &["flv", "f4v"], true, false, false, true, false, false, PureRust::None),
    row(MediaKind::ThreeGpp, &["3gp", "3g2", "amc"], true, false, false, true, false, false, PureRust::None),
    row(MediaKind::Avchd, &["m2ts"], true, false, false, true, false, false, PureRust::None),
    row(MediaKind::RedR3d, &["r3d"], true, false, false, true, false, false, PureRust::None),
    row(MediaKind::CanonCrm, &["crm"], true, false, false, true, false, false, PureRust::None),
    row(MediaKind::Swf, &["swf"], true, false, false, true, false, false, PureRust::None),
    row(MediaKind::Omf, &["omf"], true, true, false, true, false, true, PureRust::None),
    row(MediaKind::Wav, &["wav", "bwf"], true, true, false, false, true, false, PureRust::ReadWrite),
    row(MediaKind::Aiff, &["aif", "aiff"], true, true, false, false, true, false, PureRust::ReadWrite),
    row(MediaKind::Mp3, &["mp3", "mpeg", "mpg", "mpa", "mpe"], true, true, false, false, true, false, PureRust::None),
    row(MediaKind::Aac, &["aac", "m4a"], true, true, false, false, true, false, PureRust::None),
    row(MediaKind::Asnd, &["asnd"], true, false, false, false, true, false, PureRust::None),
    row(MediaKind::Project, &["aep", "aepx"], false, false, false, false, false, false, PureRust::None),
];

const fn row(
    kind: MediaKind,
    extensions: &'static [&'static str],
    import: bool,
    export: bool,
    still: bool,
    video: bool,
    audio: bool,
    windows_only: bool,
    pure_rust: PureRust,
) -> KindRow {
    KindRow {
        kind,
        extensions,
        import,
        export,
        still,
        video,
        audio,
        windows_only,
        pure_rust,
    }
}

pub fn format_info(kind: MediaKind) -> FormatInfo {
    match kind {
        MediaKind::Png => info_of(MediaKind::Png),
        MediaKind::Jpeg => info_of(MediaKind::Jpeg),
        MediaKind::Tiff => info_of(MediaKind::Tiff),
        MediaKind::Psd => info_of(MediaKind::Psd),
        MediaKind::Bmp => info_of(MediaKind::Bmp),
        MediaKind::Gif => info_of(MediaKind::Gif),
        MediaKind::OpenExr => info_of(MediaKind::OpenExr),
        MediaKind::Cineon => info_of(MediaKind::Cineon),
        MediaKind::Dpx => info_of(MediaKind::Dpx),
        MediaKind::Radiance => info_of(MediaKind::Radiance),
        MediaKind::Sgi => info_of(MediaKind::Sgi),
        MediaKind::Targa => info_of(MediaKind::Targa),
        MediaKind::MayaIff => info_of(MediaKind::MayaIff),
        MediaKind::SoftimagePic => info_of(MediaKind::SoftimagePic),
        MediaKind::Pcx => info_of(MediaKind::Pcx),
        MediaKind::Heif => info_of(MediaKind::Heif),
        MediaKind::CameraRaw => info_of(MediaKind::CameraRaw),
        MediaKind::Pict => info_of(MediaKind::Pict),
        MediaKind::ElectricImage => info_of(MediaKind::ElectricImage),
        MediaKind::Svg => info_of(MediaKind::Svg),
        MediaKind::QuickTime => info_of(MediaKind::QuickTime),
        MediaKind::Avi => info_of(MediaKind::Avi),
        MediaKind::Mpeg4 => info_of(MediaKind::Mpeg4),
        MediaKind::Hevc => info_of(MediaKind::Hevc),
        MediaKind::Mpeg1 => info_of(MediaKind::Mpeg1),
        MediaKind::Mpeg2 => info_of(MediaKind::Mpeg2),
        MediaKind::Mxf => info_of(MediaKind::Mxf),
        MediaKind::WindowsMedia => info_of(MediaKind::WindowsMedia),
        MediaKind::FlashVideo => info_of(MediaKind::FlashVideo),
        MediaKind::ThreeGpp => info_of(MediaKind::ThreeGpp),
        MediaKind::Avchd => info_of(MediaKind::Avchd),
        MediaKind::RedR3d => info_of(MediaKind::RedR3d),
        MediaKind::CanonCrm => info_of(MediaKind::CanonCrm),
        MediaKind::Swf => info_of(MediaKind::Swf),
        MediaKind::Omf => info_of(MediaKind::Omf),
        MediaKind::Wav => info_of(MediaKind::Wav),
        MediaKind::Aiff => info_of(MediaKind::Aiff),
        MediaKind::Mp3 => info_of(MediaKind::Mp3),
        MediaKind::Aac => info_of(MediaKind::Aac),
        MediaKind::Asnd => info_of(MediaKind::Asnd),
        MediaKind::Project => info_of(MediaKind::Project),
    }
}

fn info_of(kind: MediaKind) -> FormatInfo {
    let row = KINDS.iter().find(|row| row.kind == kind).expect("kind row");
    FormatInfo {
        kind: row.kind,
        import: row.import,
        export: row.export,
        still: row.still,
        video: row.video,
        audio: row.audio,
        windows_only: row.windows_only,
        pure_rust: row.pure_rust,
    }
}

pub fn extension_of(path: &std::path::Path) -> Option<String> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
}

pub fn kinds_for_extension(extension: &str) -> Vec<MediaKind> {
    let extension = extension.trim_start_matches('.').to_ascii_lowercase();
    KINDS
        .iter()
        .filter(|row| row.extensions.iter().any(|candidate| *candidate == extension))
        .map(|row| row.kind)
        .collect()
}

pub fn primary_kind(path: &std::path::Path) -> Option<MediaKind> {
    let extension = extension_of(path)?;
    kinds_for_extension(&extension).into_iter().next()
}

pub fn is_footage_extension(extension: &str) -> bool {
    kinds_for_extension(extension)
        .into_iter()
        .any(|kind| format_info(kind).import)
}

/// Files Of Type の All Footage Files。プロジェクトファイルは候補に入らない。
pub fn is_all_footage_file(path: &std::path::Path) -> bool {
    match extension_of(path) {
        Some(extension) => is_footage_extension(&extension),
        None => false,
    }
}

pub fn host_can_import(kind: MediaKind) -> bool {
    let info = format_info(kind);
    info.import && (!info.windows_only || cfg!(windows))
}

/// Camera Raw と CinemaDNG で確認できた色空間。CinemaDNG の拡張子は要確認。
pub const CAMERA_RAW_COLOR_SPACES: [&str; 4] = [
    "Adobe RGB",
    "sRGB IEC61966-2.1",
    "ColorMatch RGB",
    "ProPhoto RGB",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn footage_filter_excludes_project_files() {
        assert!(is_all_footage_file(std::path::Path::new("shot.png")));
        assert!(!is_all_footage_file(std::path::Path::new("edit.aep")));
        assert!(!is_all_footage_file(std::path::Path::new("edit.aepx")));
    }

    #[test]
    fn vob_is_not_recognized_until_renamed() {
        assert!(kinds_for_extension("vob").is_empty());
        assert!(is_footage_extension("mpg"));
    }

    #[test]
    fn shared_extensions_cover_each_listed_kind() {
        assert!(kinds_for_extension("tif").contains(&MediaKind::Tiff));
        assert!(kinds_for_extension("tiff").contains(&MediaKind::Tiff));
        assert!(kinds_for_extension("jpg").contains(&MediaKind::Jpeg));
        assert!(kinds_for_extension("jpe").contains(&MediaKind::Jpeg));
        assert!(kinds_for_extension("sxr").contains(&MediaKind::OpenExr));
        assert!(kinds_for_extension("mxr").contains(&MediaKind::OpenExr));
        assert_eq!(
            kinds_for_extension("bmp"),
            vec![MediaKind::Bmp]
        );
        for extension in ["bmp", "rle", "dib"] {
            assert!(kinds_for_extension(extension).contains(&MediaKind::Bmp), "{extension}");
        }
        for extension in ["hdr", "rgbe", "xyze"] {
            assert!(kinds_for_extension(extension).contains(&MediaKind::Radiance));
        }
        for extension in ["sgi", "bw", "rgb"] {
            assert!(kinds_for_extension(extension).contains(&MediaKind::Sgi));
        }
        for extension in ["tga", "vda", "icb", "vst"] {
            assert!(kinds_for_extension(extension).contains(&MediaKind::Targa));
        }
        assert!(kinds_for_extension("iff").contains(&MediaKind::MayaIff));
        assert!(kinds_for_extension("wav").contains(&MediaKind::Wav));
        assert!(kinds_for_extension("bwf").contains(&MediaKind::Wav));
        assert!(kinds_for_extension("aif").contains(&MediaKind::Aiff));
        assert!(kinds_for_extension("aiff").contains(&MediaKind::Aiff));
        assert!(kinds_for_extension("mp4").contains(&MediaKind::Mpeg4));
        assert!(kinds_for_extension("m4v").contains(&MediaKind::Mpeg4));
        assert!(kinds_for_extension("mov").contains(&MediaKind::QuickTime));
        assert!(kinds_for_extension("mpg").contains(&MediaKind::Mpeg1));
        assert!(kinds_for_extension("mpg").contains(&MediaKind::Mp3));
        assert!(kinds_for_extension("m2v").contains(&MediaKind::Mpeg2));
        assert!(kinds_for_extension("m2t").contains(&MediaKind::Mpeg2));
        assert!(kinds_for_extension("m2ts").contains(&MediaKind::Avchd));
        assert!(format_info(MediaKind::Pcx).windows_only);
        assert!(format_info(MediaKind::WindowsMedia).windows_only);
        assert!(!host_can_import(MediaKind::Pcx) || cfg!(windows));
        assert_eq!(CAMERA_RAW_COLOR_SPACES.len(), 4);
        assert!(kinds_for_extension("dng").contains(&MediaKind::CameraRaw));
    }
}
