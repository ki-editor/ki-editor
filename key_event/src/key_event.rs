use std::collections::HashSet;

/// This struct is created to enable pattern-matching
/// on combined modifier keys like Ctrl+Alt+Shift.
///
/// The `crossterm` crate does not support this out of the box.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyEvent {
    pub code: crossterm::event::KeyCode,
    pub modifiers: KeyModifiers,
}
impl KeyEvent {
    pub fn new(key: crossterm::event::KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code: key,
            modifiers,
        }
    }

    pub fn to_rust_code(&self) -> String {
        format!(
            "key_event::KeyEvent {{ code: crossterm::event::KeyCode::{:#?}, modifiers: key_event::KeyModifiers::{:#?}, }}",
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
            unreachable!()
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
            unreachable!()
        }
    }
}
