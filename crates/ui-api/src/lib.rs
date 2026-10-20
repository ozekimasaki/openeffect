//! ドック、パネル、コマンド、ショートカット、ヒストリ。

#![forbid(unsafe_code)]

pub mod command;
pub mod dock;
pub mod history;
pub mod panel;
pub mod shortcut;

pub use command::{Command, CommandId};
pub use dock::{Axis, Dock, DockNode};
pub use history::{History, HistoryEntry};
pub use panel::{Panel, PanelId};
pub use shortcut::{Chord, Key, Modifiers, Shortcut};
