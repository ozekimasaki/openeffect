//! ドック、パネル、コマンド、ショートカット。ウィンドウは開かない。

#![forbid(unsafe_code)]

pub mod catalog;
pub mod dock;
pub mod workspace;

pub use catalog::{Check, Kind, MenuItem, Tool, ToolId};
pub use workspace::Workspace;
