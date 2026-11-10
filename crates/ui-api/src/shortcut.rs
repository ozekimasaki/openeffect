use crate::command::CommandId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct Modifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    Character(char),
    Named(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Chord {
    pub key: Key,
    pub modifiers: Modifiers,
}

impl Chord {
    pub fn new(key: Key, modifiers: Modifiers) -> Self {
        Self { key, modifiers }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shortcut {
    pub command: CommandId,
    pub chord: Chord,
}

impl Shortcut {
    pub fn new(command: CommandId, chord: Chord) -> Self {
        Self { command, chord }
    }
}
