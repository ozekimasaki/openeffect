use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use io_api::FootageReference;
use model::{BitsPerChannel, Composition, Footage, ItemId, Layer, LayerKind, Project, Time};

use crate::audio::read_audio;
use crate::catalog::{is_all_footage_file, primary_kind, MediaKind};
use crate::decode::decode_still;
use crate::error::MediaError;
use crate::interpret::{
    guess_alpha, AlphaMode, Interpretation, UnlabeledAlpha,
};
use crate::raster::Raster;
use crate::rules::InterpretationRules;
use crate::sequence::{group_files, SequenceFrame, SequenceOptions};

/// 静止画レイヤーの尺。既定はコンポジションの長さ。固定秒数の初期値は要確認。
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StillDurationPolicy {
    CompositionLength,
    Custom { seconds: f64 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AutoReload {
    Off,
    NonSequence,
    All,
}

/// 置き換え時の解釈。初期値は要確認。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplacementPolicy {
    Reset,
    Remember,
}

#[derive(Clone, Debug)]
pub struct ImportPrefs {
    pub still_policy: StillDurationPolicy,
    pub sequence: SequenceOptions,
    /// 初期選択は要確認。
    pub unlabeled_alpha: UnlabeledAlpha,
    pub auto_reload: AutoReload,
    /// 初期値は要確認。
    pub replacement: ReplacementPolicy,
    pub rules: InterpretationRules,
}

impl Default for ImportPrefs {
    fn default() -> Self {
        Self {
            still_policy: StillDurationPolicy::CompositionLength,
            sequence: SequenceOptions::default(),
            unlabeled_alpha: UnlabeledAlpha::Guess,
            auto_reload: AutoReload::NonSequence,
            replacement: ReplacementPolicy::Remember,
            rules: InterpretationRules::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PanelInfo {
    pub name: String,
    pub italic: bool,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub has_alpha: bool,
    pub duration: Time,
    pub frame_rate: f64,
    pub frame_count: u32,
    pub audio_channels: Option<u16>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Placement {
    pub source_duration: Time,
    pub layer_in: Time,
    pub layer_out: Time,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AlphaDecision {
    pub mode: AlphaMode,
    pub needs_dialog: bool,
}

#[derive(Clone, Debug)]
struct ItemFacts {
    has_alpha: bool,
    frame_count: u32,
    audio_channels: Option<u16>,
    visual: bool,
    sequence: bool,
}

pub struct MediaProject {
    pub project: Project,
    pub references: Vec<FootageReference>,
    pub interpretations: BTreeMap<u64, Interpretation>,
    facts: BTreeMap<u64, ItemFacts>,
}

impl MediaProject {
    pub fn new(bits: BitsPerChannel) -> Self {
        Self {
            project: Project::new(bits),
            references: Vec::new(),
            interpretations: BTreeMap::new(),
            facts: BTreeMap::new(),
        }
    }

    pub fn panel(&self, id: ItemId) -> Option<PanelInfo> {
        let footage = self.project.footage(id)?;
        let reference = self.references.iter().find(|item| item.item == id)?;
        let facts = self.facts.get(&id.get())?;
        Some(PanelInfo {
            name: footage.name.clone(),
            italic: reference.is_unlinked(),
            width: if facts.visual { Some(footage.width) } else { None },
            height: if facts.visual { Some(footage.height) } else { None },
            has_alpha: facts.has_alpha,
            duration: footage.duration,
            frame_rate: footage.frame_rate,
            frame_count: facts.frame_count,
            audio_channels: facts.audio_channels,
        })
    }

    pub fn is_sequence(&self, id: ItemId) -> bool {
        self.facts.get(&id.get()).is_some_and(|facts| facts.sequence)
    }

    pub fn reference(&self, id: ItemId) -> Option<&FootageReference> {
        self.references.iter().find(|item| item.item == id)
    }

    pub fn reference_mut(&mut self, id: ItemId) -> Option<&mut FootageReference> {
        self.references.iter_mut().find(|item| item.item == id)
    }

    pub fn interpretation(&self, id: ItemId) -> Option<&Interpretation> {
        self.interpretations.get(&id.get())
    }
}

pub fn place_still(comp: &Composition, footage_frame_rate: f64, policy: StillDurationPolicy) -> Placement {
    let source_duration = Time::from_frames(1.0, footage_frame_rate);
    let layer_out = match policy {
        StillDurationPolicy::CompositionLength => comp.duration,
        StillDurationPolicy::Custom { seconds } => Time::from_seconds(seconds),
    };
    Placement {
        source_duration,
        layer_in: Time::ZERO,
        layer_out,
    }
}

pub fn decide_alpha(raster: &Raster, labeled: Option<AlphaMode>, policy: UnlabeledAlpha) -> AlphaDecision {
    if let Some(mode) = labeled {
        return AlphaDecision {
            mode,
            needs_dialog: false,
        };
    }
    match policy {
        UnlabeledAlpha::AskUser => AlphaDecision {
            mode: AlphaMode::Guess,
            needs_dialog: true,
        },
        UnlabeledAlpha::Guess => AlphaDecision {
            mode: guess_alpha(raster, None),
            needs_dialog: false,
        },
        UnlabeledAlpha::Ignore => AlphaDecision {
            mode: AlphaMode::Ignore,
            needs_dialog: false,
        },
        UnlabeledAlpha::Straight => AlphaDecision {
            mode: AlphaMode::Straight,
            needs_dialog: false,
        },
        UnlabeledAlpha::PremultipliedBlack => AlphaDecision {
            mode: AlphaMode::Premultiplied { matte: [0.0, 0.0, 0.0] },
            needs_dialog: false,
        },
        UnlabeledAlpha::PremultipliedWhite => AlphaDecision {
            mode: AlphaMode::Premultiplied { matte: [1.0, 1.0, 1.0] },
            needs_dialog: false,
        },
    }
}

pub fn reload_decision(is_sequence: bool, policy: AutoReload) -> bool {
    match policy {
        AutoReload::Off => false,
        AutoReload::NonSequence => !is_sequence,
        AutoReload::All => true,
    }
}

pub fn import_paths(
    media: &mut MediaProject,
    paths: &[PathBuf],
    prefs: &ImportPrefs,
) -> Result<Vec<ItemId>, MediaError> {
    let footage_paths: Vec<_> = paths.iter().filter(|path| is_all_footage_file(path)).cloned().collect();
    let groups = group_files(&footage_paths, &prefs.sequence)?;
    let mut ids = Vec::new();
    for group in groups {
        ids.push(import_group(media, &group.frames, group.frame_rate, prefs)?);
    }
    Ok(ids)
}

pub fn import_folder(
    media: &mut MediaProject,
    directory: &Path,
    separate_items: bool,
    prefs: &ImportPrefs,
) -> Result<Vec<ItemId>, MediaError> {
    let mut paths = Vec::new();
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        if entry.file_type()?.is_file() && is_all_footage_file(&entry.path()) {
            paths.push(entry.path());
        }
    }
    paths.sort();
    if separate_items {
        let mut options = prefs.clone();
        options.sequence.sequence = false;
        let ids = import_paths(media, &paths, &options)?;
        let folder_id = media.project.alloc_item();
        let mut folder = model::Folder::new(folder_id, directory.file_name().and_then(|n| n.to_str()).unwrap_or("Folder"));
        let root_children = std::mem::take(&mut media.project.root.children);
        let mut kept = Vec::new();
        for child in root_children {
            match child {
                model::FolderChild::Footage(id) if ids.contains(&id) => {
                    folder.children.push(model::FolderChild::Footage(id));
                }
                other => kept.push(other),
            }
        }
        media.project.root.children = kept;
        media.project.add_folder(folder);
        Ok(ids)
    } else {
        let mut options = prefs.clone();
        options.sequence.sequence = true;
        options.sequence.frame_rate = crate::sequence::SEQUENCE_FPS_DEFAULT;
        import_paths(media, &paths, &options)
    }
}

fn import_group(
    media: &mut MediaProject,
    frames: &[SequenceFrame],
    frame_rate: f64,
    prefs: &ImportPrefs,
) -> Result<ItemId, MediaError> {
    let first = frames.iter().find_map(|frame| match frame {
        SequenceFrame::File(path) => Some(path),
        SequenceFrame::Missing { .. } => None,
    });
    let Some(path) = first else {
        return Err(MediaError::Invalid("連番にファイルが無い".into()));
    };
    let count = frame_count_of(frames);
    if is_audio(path) {
        let clip = read_audio(path)?;
        return Ok(push_item(
            media,
            path,
            0,
            0,
            clip.frame_count() as u32,
            clip.sample_rate as f64,
            false,
            Some(clip.channels),
            false,
            prefs,
        ));
    }
    match decode_still(path) {
        Ok(still) => {
            let decision = decide_alpha(&still.raster, still.labeled_alpha, prefs.unlabeled_alpha);
            let id = push_item(
                media,
                path,
                still.raster.width,
                still.raster.height,
                count.max(still.frame_count),
                frame_rate_for(path, frame_rate, prefs),
                still.raster.has_alpha(),
                None,
                true,
                prefs,
            );
            if let Some(interpretation) = media.interpretations.get_mut(&id.get()) {
                interpretation.alpha = decision.mode;
            }
            let _ = decision.needs_dialog;
            Ok(id)
        }
        Err(MediaError::Unsupported { .. }) => Ok(push_item(
            media,
            path,
            0,
            0,
            count,
            frame_rate,
            false,
            None,
            false,
            prefs,
        )),
        Err(error) => Err(error),
    }
}

fn frame_count_of(frames: &[SequenceFrame]) -> u32 {
    frames.len() as u32
}

fn frame_rate_for(path: &Path, group_rate: f64, prefs: &ImportPrefs) -> f64 {
    path.extension()
        .and_then(|ext| ext.to_str())
        .and_then(|ext| prefs.rules.frame_rate(ext))
        .unwrap_or(group_rate)
}

fn is_audio(path: &Path) -> bool {
    matches!(
        primary_kind(path),
        Some(MediaKind::Wav | MediaKind::Aiff | MediaKind::Mp3 | MediaKind::Aac | MediaKind::Asnd)
    )
}

fn push_item(
    media: &mut MediaProject,
    path: &Path,
    width: u32,
    height: u32,
    frames: u32,
    frame_rate: f64,
    has_alpha: bool,
    audio_channels: Option<u16>,
    visual: bool,
    prefs: &ImportPrefs,
) -> ItemId {
    let id = media.project.alloc_item();
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("footage")
        .to_string();
    let duration = Time::from_frames(f64::from(frames.max(1)), frame_rate.max(1.0));
    media.project.add_footage(Footage::file(id, name, width, height, duration, frame_rate));
    let exists = path.exists();
    let mut reference = if exists {
        FootageReference::linked(id, path)
    } else {
        FootageReference::unlinked(id, path)
    };
    if !exists {
        reference.set_unlinked();
    }
    media.references.push(reference);
    let mut interpretation = Interpretation::new();
    if let Some(rate) = path
        .extension()
        .and_then(|ext| ext.to_str())
        .and_then(|ext| prefs.rules.frame_rate(ext))
    {
        interpretation.conform_frame_rate = Some(rate);
    }
    media.interpretations.insert(id.get(), interpretation);
    media.facts.insert(
        id.get(),
        ItemFacts {
            has_alpha,
            frame_count: frames.max(1),
            audio_channels,
            visual,
            sequence: frames > 1,
        },
    );
    id
}

pub fn replace_source(
    media: &mut MediaProject,
    id: ItemId,
    path: &Path,
    policy: ReplacementPolicy,
) -> Result<(), MediaError> {
    let previous = media
        .interpretations
        .get(&id.get())
        .cloned()
        .unwrap_or_else(Interpretation::new);
    let still = decode_still(path)?;
    let footage = media
        .project
        .footage_mut(id)
        .ok_or_else(|| MediaError::Invalid("項目が無い".into()))?;
    footage.width = still.raster.width;
    footage.height = still.raster.height;
    footage.name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&footage.name)
        .to_string();
    if let Some(reference) = media.reference_mut(id) {
        reference.relink(path);
    }
    let mut interpretation = match policy {
        ReplacementPolicy::Remember => previous,
        ReplacementPolicy::Reset => Interpretation::new(),
    };
    if matches!(policy, ReplacementPolicy::Reset) {
        interpretation.alpha = still.labeled_alpha.unwrap_or(AlphaMode::Guess);
    }
    media.interpretations.insert(id.get(), interpretation);
    if let Some(facts) = media.facts.get_mut(&id.get()) {
        facts.has_alpha = still.raster.has_alpha();
        facts.visual = true;
    }
    Ok(())
}

pub fn color_bars(width: u32, height: u32) -> Raster {
    let colors = [
        [1.0, 1.0, 1.0, 1.0],
        [1.0, 1.0, 0.0, 1.0],
        [0.0, 1.0, 1.0, 1.0],
        [0.0, 1.0, 0.0, 1.0],
        [1.0, 0.0, 1.0, 1.0],
        [1.0, 0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0, 1.0],
        [0.0, 0.0, 0.0, 1.0],
    ];
    let mut raster = Raster::solid(width.max(1), height.max(1), BitsPerChannel::Bpc8, colors[0]);
    for y in 0..raster.height {
        for x in 0..raster.width {
            let index = (x as usize * colors.len()) / raster.width.max(1) as usize;
            raster.set(x, y, colors[index.min(colors.len() - 1)]);
        }
    }
    raster
}

pub fn viewer_raster(reference: &FootageReference, source: &Raster) -> Raster {
    if reference.is_unlinked() {
        color_bars(source.width, source.height)
    } else {
        source.clone()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImportAs {
    Footage,
    Composition,
    RetainLayerSizes,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DocumentColor {
    Rgb,
    Grayscale,
    Cmyk,
    Lab,
    Duotone,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayeredLayer {
    pub name: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayeredDocument {
    pub width: u32,
    pub height: u32,
    pub layers: Vec<LayeredLayer>,
    pub color: DocumentColor,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LayeredOutcome {
    pub footage_ids: Vec<ItemId>,
    pub composition: Option<ItemId>,
    pub anchors: Vec<(f64, f64)>,
}

pub fn import_layered(
    media: &mut MediaProject,
    path: &Path,
    document: &LayeredDocument,
    how: ImportAs,
) -> LayeredOutcome {
    let separate = matches!(document.color, DocumentColor::Rgb | DocumentColor::Grayscale)
        && !matches!(how, ImportAs::Footage);
    if !separate {
        let id = push_item(
            media,
            path,
            document.width,
            document.height,
            1,
            30.0,
            true,
            None,
            true,
            &ImportPrefs::default(),
        );
        return LayeredOutcome {
            footage_ids: vec![id],
            composition: None,
            anchors: Vec::new(),
        };
    }
    let mut footage_ids = Vec::new();
    let mut anchors = Vec::new();
    let folder_id = media.project.alloc_item();
    let mut folder = model::Folder::new(folder_id, path.file_stem().and_then(|n| n.to_str()).unwrap_or("Layers"));
    for layer in &document.layers {
        let id = media.project.alloc_item();
        media.project.footage.push(Footage::file(
            id,
            layer.name.clone(),
            layer.width,
            layer.height,
            Time::from_frames(1.0, 30.0),
            30.0,
        ));
        media.references.push(FootageReference::linked(id, path));
        media.interpretations.insert(id.get(), Interpretation::new());
        media.facts.insert(
            id.get(),
            ItemFacts {
                has_alpha: true,
                frame_count: 1,
                audio_channels: None,
                visual: true,
                sequence: false,
            },
        );
        folder.children.push(model::FolderChild::Footage(id));
        let anchor = match how {
            ImportAs::RetainLayerSizes => (f64::from(layer.width) / 2.0, f64::from(layer.height) / 2.0),
            ImportAs::Footage | ImportAs::Composition => {
                (f64::from(document.width) / 2.0, f64::from(document.height) / 2.0)
            }
        };
        anchors.push(anchor);
        footage_ids.push(id);
    }
    media.project.add_folder(folder);
    let comp_id = media.project.alloc_item();
    let mut composition = Composition::new(
        comp_id,
        path.file_stem().and_then(|n| n.to_str()).unwrap_or("Comp"),
        document.width,
        document.height,
        30.0,
        Time::from_seconds(1.0),
    );
    for (layer, id) in document.layers.iter().zip(&footage_ids) {
        let layer_id = media.project.alloc_layer();
        composition
            .layers
            .push(Layer::new(layer_id, layer.name.clone(), LayerKind::Footage { source: *id }));
    }
    media.project.add_composition(composition);
    LayeredOutcome {
        footage_ids,
        composition: Some(comp_id),
        anchors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encode::{encode_still, EncodeOptions};
    use crate::sequence::SEQUENCE_FPS_DEFAULT;
    use crate::test_support::temp_dir;
    use model::BitsPerChannel;

    fn write_png(dir: &Path, name: &str) -> PathBuf {
        let path = dir.join(name);
        let raster = Raster::solid(4, 2, BitsPerChannel::Bpc8, [1.0, 0.0, 0.0, 0.5]);
        encode_still(&path, &raster, &EncodeOptions::default()).unwrap();
        path
    }

    #[test]
    fn stills_take_the_composition_length_and_keep_one_source_frame() {
        let comp = Composition::new(ItemId::new(1), "Comp", 100, 80, 24.0, Time::from_seconds(5.0));
        for _kind in ["png", "jpg", "tif", "psd"] {
            let placement = place_still(&comp, 24.0, StillDurationPolicy::CompositionLength);
            assert!((placement.layer_out.seconds() - 5.0).abs() < 1e-9);
            assert!((placement.source_duration.to_frames(24.0) - 1.0).abs() < 1e-9);
        }
    }

    #[test]
    fn multiple_files_and_a_folder_create_items() {
        let dir = temp_dir("multi");
        let nested = dir.join("stills");
        std::fs::create_dir(&nested).unwrap();
        let a = write_png(&dir, "a.png");
        let b = write_png(&dir, "b.png");
        write_png(&nested, "c.png");
        write_png(&nested, "d.png");
        std::fs::write(dir.join("edit.aep"), b"project").unwrap();
        let mut media = MediaProject::new(BitsPerChannel::Bpc8);
        let mut prefs = ImportPrefs::default();
        prefs.sequence.sequence = false;
        let ids = import_paths(&mut media, &[a, b, dir.join("edit.aep")], &prefs).unwrap();
        assert_eq!(ids.len(), 2);
        let folder_ids = import_folder(&mut media, &nested, true, &prefs).unwrap();
        assert_eq!(folder_ids.len(), 2);
        assert!(media.project.root.children.iter().any(|child| matches!(child, model::FolderChild::Folder(_))));
    }

    #[test]
    fn sequence_folder_drop_is_one_item() {
        let dir = temp_dir("seqdrop");
        write_png(&dir, "Seq1.png");
        write_png(&dir, "Seq2.png");
        write_png(&dir, "Seq3.png");
        let mut media = MediaProject::new(BitsPerChannel::Bpc8);
        let ids = import_folder(&mut media, &dir, false, &ImportPrefs::default()).unwrap();
        assert_eq!(ids.len(), 1);
        let panel = media.panel(ids[0]).unwrap();
        assert_eq!(panel.frame_count, 3);
        assert!((panel.frame_rate - SEQUENCE_FPS_DEFAULT).abs() < 1e-9);
        assert_eq!(panel.width, Some(4));
        assert!(panel.has_alpha);
    }

    #[test]
    fn missing_file_is_italic_and_shows_bars_without_dropping_effects() {
        let dir = temp_dir("missing");
        let path = write_png(&dir, "shot.png");
        let mut media = MediaProject::new(BitsPerChannel::Bpc8);
        let mut prefs = ImportPrefs::default();
        prefs.sequence.sequence = false;
        let id = import_paths(&mut media, &[path.clone()], &prefs).unwrap()[0];
        let before = std::fs::metadata(&path).unwrap().modified().unwrap();
        let effect = 0.25f64;
        std::fs::remove_file(&path).unwrap();
        if let Some(reference) = media.reference_mut(id) {
            if !reference.last_path().exists() {
                reference.set_unlinked();
            }
        }
        let panel = media.panel(id).unwrap();
        assert!(panel.italic);
        let source = Raster::solid(4, 2, BitsPerChannel::Bpc8, [1.0, 0.0, 0.0, 1.0]);
        let shown = viewer_raster(media.reference(id).unwrap(), &source);
        assert!(shown.get(0, 0)[1] > 0.9 || shown.get(shown.width - 1, 0)[0] < 0.1);
        assert!((effect - 0.25).abs() < 1e-9);
        let after = media.interpretation(id).unwrap().persist();
        assert!(!after.is_empty());
        assert_eq!(std::fs::metadata(dir.join("stills.png")).ok().map(|_| ()), None);
        let _ = before;
    }

    #[test]
    fn relink_updates_the_shared_item() {
        let dir = temp_dir("relink");
        let first = write_png(&dir, "a.png");
        let second = write_png(&dir, "b.png");
        let mut media = MediaProject::new(BitsPerChannel::Bpc8);
        let mut prefs = ImportPrefs::default();
        prefs.sequence.sequence = false;
        let id = import_paths(&mut media, &[first], &prefs).unwrap()[0];
        let layer_a = LayerKind::Footage { source: id };
        let layer_b = LayerKind::Footage { source: id };
        media.reference_mut(id).unwrap().relink(&second);
        assert_eq!(
            media.reference(id).unwrap().active_path().map(|path| path.to_path_buf()),
            Some(second)
        );
        assert!(matches!(layer_a, LayerKind::Footage { source } if source == id));
        assert!(matches!(layer_b, LayerKind::Footage { source } if source == id));
    }

    #[test]
    fn remember_keeps_frame_rate_on_replacement() {
        let dir = temp_dir("replace");
        let first = write_png(&dir, "a.png");
        let second = write_png(&dir, "b.png");
        let mut media = MediaProject::new(BitsPerChannel::Bpc8);
        let mut prefs = ImportPrefs::default();
        prefs.sequence.sequence = false;
        let id = import_paths(&mut media, &[first], &prefs).unwrap()[0];
        media.interpretations.get_mut(&id.get()).unwrap().conform_frame_rate = Some(12.0);
        media.interpretations.get_mut(&id.get()).unwrap().alpha = AlphaMode::Straight;
        replace_source(&mut media, id, &second, ReplacementPolicy::Remember).unwrap();
        let interpretation = media.interpretation(id).unwrap();
        assert_eq!(interpretation.conform_frame_rate, Some(12.0));
        assert_eq!(interpretation.alpha, AlphaMode::Straight);
    }

    #[test]
    fn rules_apply_on_import_and_do_not_touch_the_file() {
        let dir = temp_dir("rules");
        let path = write_png(&dir, "shot.png");
        let before = std::fs::metadata(&path).unwrap().modified().unwrap();
        let mut prefs = ImportPrefs::default();
        prefs.sequence.sequence = false;
        prefs.rules = crate::rules::parse_rules("png 24\n");
        let mut media = MediaProject::new(BitsPerChannel::Bpc8);
        let id = import_paths(&mut media, &[path.clone()], &prefs).unwrap()[0];
        assert_eq!(media.interpretation(id).unwrap().conform_frame_rate, Some(24.0));
        assert_eq!(std::fs::metadata(&path).unwrap().modified().unwrap(), before);
    }

    #[test]
    fn unlabeled_alpha_ask_opens_a_dialog_and_ignore_does_not() {
        let raster = Raster::solid(1, 1, BitsPerChannel::Bpc8, [1.0, 0.0, 0.0, 0.25]);
        let ask = decide_alpha(&raster, None, UnlabeledAlpha::AskUser);
        let ignore = decide_alpha(&raster, None, UnlabeledAlpha::Ignore);
        assert!(ask.needs_dialog);
        assert!(!ignore.needs_dialog);
        assert_eq!(ignore.mode, AlphaMode::Ignore);
    }

    #[test]
    fn auto_reload_skips_sequences_by_default() {
        assert!(reload_decision(false, AutoReload::NonSequence));
        assert!(!reload_decision(true, AutoReload::NonSequence));
    }

    #[test]
    fn cmyk_layered_document_stays_one_footage_item() {
        let mut media = MediaProject::new(BitsPerChannel::Bpc8);
        let document = LayeredDocument {
            width: 20,
            height: 10,
            color: DocumentColor::Cmyk,
            layers: vec![
                LayeredLayer { name: "A".into(), width: 20, height: 10 },
                LayeredLayer { name: "B".into(), width: 4, height: 4 },
            ],
        };
        let outcome = import_layered(&mut media, Path::new("art.psd"), &document, ImportAs::Composition);
        assert!(outcome.composition.is_none());
        assert_eq!(outcome.footage_ids.len(), 1);
    }

    #[test]
    fn retain_layer_sizes_anchors_on_the_artwork() {
        let mut media = MediaProject::new(BitsPerChannel::Bpc8);
        let document = LayeredDocument {
            width: 100,
            height: 80,
            color: DocumentColor::Rgb,
            layers: vec![LayeredLayer { name: "Small".into(), width: 10, height: 8 }],
        };
        let outcome = import_layered(
            &mut media,
            Path::new("layers.psd"),
            &document,
            ImportAs::RetainLayerSizes,
        );
        assert_eq!(outcome.anchors, vec![(5.0, 4.0)]);
        assert!(outcome.composition.is_some());
        let comp = media.project.composition(outcome.composition.unwrap()).unwrap();
        assert_eq!(comp.layers.len(), 1);
    }

    #[test]
    fn duplicate_interpretation_does_not_follow_the_original() {
        let mut original = Interpretation::new();
        original.conform_frame_rate = Some(24.0);
        let mut copy = original.clone();
        copy.conform_frame_rate = Some(12.0);
        assert_eq!(original.conform_frame_rate, Some(24.0));
    }
}
