use std::collections::HashSet;

#[derive(Debug)]
pub enum Event {
    Key(KeyEvent),
    FocusGained,
    FocusLost,
    Mouse(crossterm::event::MouseEvent),
    Paste(String),
    Resize(u16, u16),
}

impl From<crossterm::event::Event> for Event {
    fn from(value: crossterm::event::Event) -> Self {
        match value {
            crossterm::event::Event::Key(key) => Event::Key(key.into()),
            crossterm::event::Event::FocusGained => Event::FocusGained,
            crossterm::event::Event::FocusLost => Event::FocusLost,
            crossterm::event::Event::Mouse(mouse_event) => Event::Mouse(mouse_event),
            crossterm::event::Event::Paste(string) => Event::Paste(string),
            crossterm::event::Event::Resize(columns, rows) => Event::Resize(columns, rows),
        }
    }
}

/// This struct is created to enable pattern-matching
/// on combined modifier keys like Ctrl+Alt+Shift.
///
/// The `crossterm` crate does not support this out of the box.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyEvent {
    pub code: crossterm::event::KeyCode,
    pub modifiers: KeyModifiers,
}
impl KeyEvent {
    pub const fn new(key: crossterm::event::KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code: key,
            modifiers,
        }
    }

    pub fn to_rust_code(&self) -> String {
        format!(
            "event::KeyEvent {{ code: crossterm::event::KeyCode::{:#?}, modifiers: event::KeyModifiers::{:#?}, }}",
            self.code, self.modifiers
        )
    }
}

impl From<crossterm::event::KeyEvent> for KeyEvent {
    fn from(value: crossterm::event::KeyEvent) -> Self {
        Self {
            code: value.code,
            modifiers: value.modifiers.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum KeyModifiers {
    None,
    Ctrl,
    Alt,
    Shift,
    CtrlAlt,
    CtrlShift,
    AltShift,
    CtrlAltShift,
    Unknown,
}
impl KeyModifiers {
    pub(crate) fn add_shift(self, shift: bool) -> KeyModifiers {
        use KeyModifiers::*;
        if !shift {
            return self;
        }
        match self {
            None => Shift,
            Ctrl => CtrlShift,
            Alt => AltShift,
            CtrlAlt => CtrlAltShift,
            Unknown => Shift,
            _ => self,
        }
    }
}

impl From<crossterm::event::KeyModifiers> for KeyModifiers {
    fn from(value: crossterm::event::KeyModifiers) -> Self {
        use crossterm::event::KeyModifiers;
        if value == KeyModifiers::NONE {
            self::KeyModifiers::None
        } else if value == KeyModifiers::CONTROL {
            self::KeyModifiers::Ctrl
        } else if value == KeyModifiers::ALT {
            self::KeyModifiers::Alt
        } else if value == KeyModifiers::SHIFT {
            self::KeyModifiers::Shift
        } else if value == KeyModifiers::CONTROL | KeyModifiers::ALT {
            self::KeyModifiers::CtrlAlt
        } else if value == KeyModifiers::CONTROL | KeyModifiers::SHIFT {
            self::KeyModifiers::CtrlShift
        } else if value == KeyModifiers::ALT | KeyModifiers::SHIFT {
            self::KeyModifiers::AltShift
        } else if value == KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SHIFT {
            self::KeyModifiers::CtrlAltShift
        } else {
            self::KeyModifiers::Unknown
        }
    }
}

impl From<HashSet<KeyModifiers>> for KeyModifiers {
    fn from(value: HashSet<KeyModifiers>) -> Self {
        use KeyModifiers::*;
        if value == HashSet::from([None]) || value.is_empty() {
            None
        } else if value == HashSet::from([Ctrl]) {
            Ctrl
        } else if value == HashSet::from([Alt]) {
            Alt
        } else if value == HashSet::from([Shift]) {
            Shift
        } else if value == HashSet::from([Ctrl, Alt]) {
            CtrlAlt
        } else if value == HashSet::from([Ctrl, Shift]) {
            CtrlShift
        } else if value == HashSet::from([Alt, Shift]) {
            AltShift
        } else if value == HashSet::from([Ctrl, Alt, Shift]) {
            CtrlAltShift
        } else {
            Unknown
        }
    }
}
