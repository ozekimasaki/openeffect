//! フッテージ参照、未リンク、プロキシ、レンダーキューの出力。

#![forbid(unsafe_code)]

pub mod footage;
pub mod proxy;
pub mod render;

pub use footage::{FootageReference, LinkState};
pub use proxy::Proxy;
pub use render::{OutputFormat, PostRenderAction, RenderOutput, RenderQueue};
