use std::path::PathBuf;

use model::{BitsPerChannel, ItemId};

/// この契約が出力先として名前を持つ形式。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OutputFormat {
    Png,
    Tiff,
    OpenExr,
    Dpx,
    Jpeg,
    Wav,
    H264,
    ProRes,
}

impl OutputFormat {
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Tiff => "tif",
            Self::OpenExr => "exr",
            Self::Dpx => "dpx",
            Self::Jpeg => "jpg",
            Self::Wav => "wav",
            Self::H264 => "mp4",
            Self::ProRes => "mov",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum PostRenderAction {
    #[default]
    None,
    Import,
    ImportAndReplace,
    SetProxy,
}

/// レンダーキューの 1 出力。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderOutput {
    pub comp: ItemId,
    pub path: PathBuf,
    pub format: OutputFormat,
    pub bits_per_channel: BitsPerChannel,
    pub post_render: PostRenderAction,
}

impl RenderOutput {
    pub fn new(
        comp: ItemId,
        path: impl Into<PathBuf>,
        format: OutputFormat,
        bits_per_channel: BitsPerChannel,
    ) -> Self {
        Self {
            comp,
            path: path.into(),
            format,
            bits_per_channel,
            post_render: PostRenderAction::None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct RenderQueue {
    pub outputs: Vec<RenderOutput>,
}

impl RenderQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, output: RenderOutput) {
        self.outputs.push(output);
    }

    pub fn len(&self) -> usize {
        self.outputs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.outputs.is_empty()
    }
}
