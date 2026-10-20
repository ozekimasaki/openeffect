use std::path::{Path, PathBuf};

use crate::error::MediaError;

/// 連番の既定フレームレート。
pub const SEQUENCE_FPS_DEFAULT: f64 = 30.0;
/// 連番と Conform の上限。
pub const MAX_SEQUENCE_FPS: f64 = 999.0;

#[derive(Clone, Debug, PartialEq)]
pub struct SequenceOptions {
    pub sequence: bool,
    pub multiple_sequences: bool,
    pub force_alphabetical: bool,
    pub report_missing_frames: bool,
    pub verify_individual_files: bool,
    pub frame_rate: f64,
}

impl Default for SequenceOptions {
    fn default() -> Self {
        Self {
            sequence: true,
            multiple_sequences: false,
            force_alphabetical: false,
            report_missing_frames: false,
            verify_individual_files: false,
            frame_rate: SEQUENCE_FPS_DEFAULT,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SequenceFrame {
    File(PathBuf),
    Missing { number: u32 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SequenceWarning {
    MissingFrames { missing: Vec<u32> },
}

#[derive(Clone, Debug, PartialEq)]
pub struct SequenceGroup {
    pub frames: Vec<SequenceFrame>,
    pub frame_rate: f64,
    pub warnings: Vec<SequenceWarning>,
}

#[derive(Clone, Debug)]
struct NumberedFile {
    path: PathBuf,
    prefix: String,
    number: u32,
    padding: usize,
    extension: String,
    broken_link: bool,
}

pub fn checked_frame_rate(frame_rate: f64) -> Result<f64, MediaError> {
    if frame_rate <= 0.0 || frame_rate > MAX_SEQUENCE_FPS {
        Err(MediaError::FrameRate(frame_rate))
    } else {
        Ok(frame_rate)
    }
}

/// 選択ファイルを連番へ分ける。`sequence` がオフならファイルごとに 1 項目。
pub fn group_files(paths: &[PathBuf], options: &SequenceOptions) -> Result<Vec<SequenceGroup>, MediaError> {
    let frame_rate = checked_frame_rate(options.frame_rate)?;
    if !options.sequence {
        return Ok(paths
            .iter()
            .map(|path| SequenceGroup {
                frames: vec![SequenceFrame::File(path.clone())],
                frame_rate,
                warnings: Vec::new(),
            })
            .collect());
    }
    let mut numbered = Vec::new();
    let mut plain = Vec::new();
    for path in paths {
        if let Some(parsed) = parse_numbered(path) {
            let broken_link = is_broken_link(path);
            if options.verify_individual_files && broken_link {
                numbered.push(NumberedFile {
                    broken_link: true,
                    ..parsed
                });
            } else {
                numbered.push(NumberedFile {
                    broken_link: false,
                    ..parsed
                });
            }
        } else {
            plain.push(path.clone());
        }
    }
    let mut groups = Vec::new();
    if options.multiple_sequences {
        let mut keys: Vec<(String, String, usize)> = numbered
            .iter()
            .map(|file| (file.prefix.clone(), file.extension.clone(), file.padding))
            .collect();
        keys.sort();
        keys.dedup();
        for key in keys {
            let members: Vec<_> = numbered
                .iter()
                .filter(|file| file.prefix == key.0 && file.extension == key.1 && file.padding == key.2)
                .cloned()
                .collect();
            groups.push(build_group(members, options, frame_rate));
        }
    } else if !numbered.is_empty() {
        groups.push(build_group(numbered, options, frame_rate));
    }
    for path in plain {
        groups.push(SequenceGroup {
            frames: vec![SequenceFrame::File(path)],
            frame_rate,
            warnings: Vec::new(),
        });
    }
    Ok(groups)
}

pub fn frame_count(group: &SequenceGroup) -> u32 {
    group.frames.len() as u32
}

fn build_group(mut files: Vec<NumberedFile>, options: &SequenceOptions, frame_rate: f64) -> SequenceGroup {
    if options.force_alphabetical {
        files.sort_by(|left, right| left.path.file_name().cmp(&right.path.file_name()));
        let frames = files
            .into_iter()
            .map(|file| {
                if file.broken_link {
                    SequenceFrame::Missing { number: file.number }
                } else {
                    SequenceFrame::File(file.path)
                }
            })
            .collect();
        return SequenceGroup {
            frames,
            frame_rate,
            warnings: Vec::new(),
        };
    }
    files.sort_by_key(|file| file.number);
    let mut warnings = Vec::new();
    let mut missing = Vec::new();
    if files.len() >= 2 {
        let first = files[0].number;
        let last = files[files.len() - 1].number;
        let present: std::collections::BTreeSet<u32> = files.iter().map(|file| file.number).collect();
        for number in first..=last {
            if !present.contains(&number) {
                missing.push(number);
            }
        }
    }
    if !missing.is_empty() {
        warnings.push(SequenceWarning::MissingFrames {
            missing: missing.clone(),
        });
    }
    let mut frames = Vec::new();
    if options.report_missing_frames && !missing.is_empty() {
        let first = files[0].number;
        let last = files[files.len() - 1].number;
        for number in first..=last {
            if let Some(file) = files.iter().find(|file| file.number == number) {
                if file.broken_link {
                    frames.push(SequenceFrame::Missing { number });
                } else {
                    frames.push(SequenceFrame::File(file.path.clone()));
                }
            } else {
                frames.push(SequenceFrame::Missing { number });
            }
        }
    } else {
        for file in files {
            if file.broken_link {
                frames.push(SequenceFrame::Missing { number: file.number });
            } else {
                frames.push(SequenceFrame::File(file.path));
            }
        }
    }
    SequenceGroup {
        frames,
        frame_rate,
        warnings,
    }
}

fn parse_numbered(path: &Path) -> Option<NumberedFile> {
    let name = path.file_name()?.to_str()?;
    let (stem, extension) = match name.rsplit_once('.') {
        Some((stem, extension)) => (stem, extension.to_ascii_lowercase()),
        None => return None,
    };
    let digits = stem.chars().rev().take_while(|ch| ch.is_ascii_digit()).count();
    if digits == 0 {
        return None;
    }
    let split = stem.len() - digits;
    let number: u32 = stem[split..].parse().ok()?;
    Some(NumberedFile {
        path: path.to_path_buf(),
        prefix: stem[..split].to_string(),
        number,
        padding: digits,
        extension,
        broken_link: false,
    })
}

pub fn is_broken_link(path: &Path) -> bool {
    match std::fs::symlink_metadata(path) {
        Ok(meta) => meta.file_type().is_symlink() && !path.exists(),
        Err(_) => !path.exists(),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FolderDrop {
    Sequence,
    SeparateItems,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_dir;

    fn touch(path: &Path) {
        std::fs::write(path, b"x").unwrap();
    }

    #[test]
    fn three_numbered_files_are_one_item_at_thirty_fps() {
        let paths = vec![
            PathBuf::from("Seq1.png"),
            PathBuf::from("Seq2.png"),
            PathBuf::from("Seq3.png"),
        ];
        let groups = group_files(&paths, &SequenceOptions::default()).unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(frame_count(&groups[0]), 3);
        assert_eq!(groups[0].frame_rate, 30.0);
        let duration = frame_count(&groups[0]) as f64 / groups[0].frame_rate;
        assert!((duration - 0.1).abs() < 1e-9);
    }

    #[test]
    fn sequence_off_keeps_a_single_file() {
        let paths = vec![PathBuf::from("Seq1.png"), PathBuf::from("Seq2.png")];
        let mut options = SequenceOptions::default();
        options.sequence = false;
        let groups = group_files(&paths, &options).unwrap();
        assert_eq!(groups.len(), 2);
        assert!(groups.iter().all(|group| frame_count(group) == 1));
    }

    #[test]
    fn range_selection_uses_only_the_chosen_files() {
        let paths: Vec<_> = (3..=6).map(|index| PathBuf::from(format!("Seq{index}.png"))).collect();
        let groups = group_files(&paths, &SequenceOptions::default()).unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(frame_count(&groups[0]), 4);
        match &groups[0].frames[0] {
            SequenceFrame::File(path) => assert!(path.ends_with("Seq3.png")),
            SequenceFrame::Missing { .. } => panic!("file"),
        }
    }

    #[test]
    fn multiple_sequences_become_two_items() {
        let paths = vec![
            PathBuf::from("A1.png"),
            PathBuf::from("A2.png"),
            PathBuf::from("B1.png"),
            PathBuf::from("B2.png"),
        ];
        let mut options = SequenceOptions::default();
        options.multiple_sequences = true;
        let groups = group_files(&paths, &options).unwrap();
        assert_eq!(groups.len(), 2);
    }

    #[test]
    fn alphabetical_force_counts_gaps_as_present_files() {
        let paths = vec![
            PathBuf::from("Seq1.png"),
            PathBuf::from("Seq2.png"),
            PathBuf::from("Seq3.png"),
            PathBuf::from("Seq5.png"),
        ];
        let mut forced = SequenceOptions::default();
        forced.force_alphabetical = true;
        let forced_groups = group_files(&paths, &forced).unwrap();
        assert_eq!(frame_count(&forced_groups[0]), 4);
        assert!(forced_groups[0].warnings.is_empty());

        let warned = group_files(&paths, &SequenceOptions::default()).unwrap();
        assert!(matches!(
            warned[0].warnings.first(),
            Some(SequenceWarning::MissingFrames { missing }) if missing == &vec![4]
        ));
    }

    #[test]
    fn report_missing_frames_inserts_a_placeholder() {
        let paths = vec![
            PathBuf::from("Seq1.png"),
            PathBuf::from("Seq2.png"),
            PathBuf::from("Seq3.png"),
            PathBuf::from("Seq5.png"),
        ];
        let mut options = SequenceOptions::default();
        options.report_missing_frames = true;
        let groups = group_files(&paths, &options).unwrap();
        assert_eq!(frame_count(&groups[0]), 5);
        assert!(matches!(groups[0].frames[3], SequenceFrame::Missing { number: 4 }));
    }

    #[test]
    fn verify_individual_files_marks_a_broken_link() {
        let dir = temp_dir("verify");
        let real = dir.join("Seq1.png");
        let broken = dir.join("Seq2.png");
        touch(&real);
        std::os::unix::fs::symlink(dir.join("missing.png"), &broken).unwrap();
        let paths = vec![real, broken];
        let mut off = SequenceOptions::default();
        off.verify_individual_files = false;
        let open = group_files(&paths, &off).unwrap();
        assert!(open[0].frames.iter().all(|frame| matches!(frame, SequenceFrame::File(_))));

        let mut on = SequenceOptions::default();
        on.verify_individual_files = true;
        let checked = group_files(&paths, &on).unwrap();
        assert!(matches!(checked[0].frames[1], SequenceFrame::Missing { number: 2 }));
    }

    #[test]
    fn frame_rate_above_999_is_rejected() {
        let mut options = SequenceOptions::default();
        options.frame_rate = 1000.0;
        assert!(group_files(&[PathBuf::from("A1.png")], &options).is_err());
    }

    #[test]
    fn turning_sequence_off_is_remembered() {
        let mut preference = SequenceOptions::default();
        assert!(preference.sequence);
        preference.sequence = false;
        assert!(!preference.sequence);
    }
}
