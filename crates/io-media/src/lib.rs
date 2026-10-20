//! フッテージの読み込みと解釈。ソースファイルは書き換えない。

#![forbid(unsafe_code)]

pub mod audio;
pub mod catalog;
pub mod cineon;
pub mod decode;
pub mod dpx;
pub mod encode;
pub mod error;
pub mod import;
pub mod interpret;
pub mod raster;
pub mod rules;
pub mod sequence;

#[cfg(test)]
pub mod test_support;

pub use audio::{read_audio, AudioClip};
pub use catalog::{kinds_for_extension, primary_kind, FormatInfo, MediaKind};
pub use cineon::{cineon_code_to_channel, CineonSettings, CineonUnits};
pub use decode::{decode_still, DecodedStill};
pub use encode::{encode_still, EncodeOptions};
pub use error::MediaError;
pub use import::{
    decide_alpha, import_folder, import_layered, import_paths, place_still, reload_decision,
    replace_source, DocumentColor, ImportAs, ImportPrefs, LayeredDocument, MediaProject, PanelInfo,
    ReplacementPolicy, StillDurationPolicy,
};
pub use interpret::{
    apply_interpretation, conform_duration, guess_pixel_aspect, looped_duration,
    remove_pulldown_frame_rate, separate_fields, AlphaMode, FieldOrder, InterpretDialog,
    Interpretation, InterpretationMemory, PixelAspect, Pulldown, UnlabeledAlpha,
};
pub use raster::Raster;
pub use rules::{parse_rules, InterpretationRules};
pub use sequence::{
    group_files, SequenceFrame, SequenceGroup, SequenceOptions, SequenceWarning, MAX_SEQUENCE_FPS,
    SEQUENCE_FPS_DEFAULT,
};
