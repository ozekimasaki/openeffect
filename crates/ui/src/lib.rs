//! ドック、パネル、コマンド、ショートカット。ウィンドウは開かない。

#![forbid(unsafe_code)]

pub mod catalog;
pub mod dock;
pub mod workspace;

pub use catalog::{MenuItem, Tool, ToolId};
pub use workspace::Workspace;
