use std::{collections::HashSet, fmt};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum KeyEventKind {
    Press,
    Release,
}

impl From<crossterm::event::KeyEventKind> for KeyEventKind {
    fn from(value: crossterm::event::KeyEventKind) -> Self {
        match value {
            crossterm::event::KeyEventKind::Press => Self::Press,
            crossterm::event::KeyEventKind::Repeat => Self::Press,
            crossterm::event::KeyEventKind::Release => Self::Release,
        }
    }
}

/// This struct is created to enable pattern-matching
/// on combined modifier keys like Ctrl+Alt+Shift.
///
/// The `crossterm` crate does not support this out of the box.
///
/// It also replaces Repeat events with Press events
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct KeyEvent {
    pub code: crossterm::event::KeyCode,
    pub modifiers: KeyModifiers,
    pub kind: KeyEventKind,
}
impl fmt::Debug for KeyEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display())
    }
}
impl KeyEvent {
    pub const fn pressed(key: crossterm::event::KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code: key,
            modifiers,
            kind: KeyEventKind::Press,
        }
    }

    pub const fn released(key: crossterm::event::KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code: key,
            modifiers,
            kind: KeyEventKind::Release,
        }
    }

    pub fn to_rust_code(&self) -> String {
        format!(
            "event::KeyEvent {{ code: crossterm::event::KeyCode::{:#?}, modifiers: event::{:#?}, kind: event::KeyEventKind::{:#?} }}",
            self.code, self.modifiers, self.kind
        )
    }

    pub fn display(&self) -> String {
        use crossterm::event::KeyCode;
        let key_code = match self.code {
            KeyCode::Char(' ') => String::from("space"),
            KeyCode::Char(c) => c.to_string(),
            KeyCode::Backspace => String::from("backspace"),
            KeyCode::Enter => String::from("enter"),
            KeyCode::Left => String::from("left"),
            KeyCode::Right => String::from("right"),
            KeyCode::Up => String::from("up"),
            KeyCode::Down => String::from("down"),
            KeyCode::Home => String::from("home"),
            KeyCode::End => String::from("end"),
            KeyCode::PageUp => String::from("pageup"),
            KeyCode::PageDown => String::from("pagedown"),
            KeyCode::Tab => String::from("tab"),
            KeyCode::BackTab => String::from("backtab"),
            KeyCode::Delete => String::from("delete"),
            KeyCode::Insert => String::from("insert"),
            KeyCode::F(n) => format!("F{n}"),
            KeyCode::Null => String::from("Null"),
            KeyCode::Esc => String::from("esc"),
            // Add more cases as needed
            _ => String::from("Unknown"),
        };
        let modifier = self.modifiers.display();
        let modified = format!(
            "{}{key_code}",
            if !modifier.is_empty() {
                format!("{modifier}+")
            } else {
                "".to_string()
            }
        );
        format!(
            "{}{modified}",
            if self.kind == KeyEventKind::Release {
                "release-"
            } else {
                ""
            }
        )
    }

    pub fn set_event_kind(self, kind: KeyEventKind) -> KeyEvent {
        Self { kind, ..self }
    }
}

impl From<crossterm::event::KeyEvent> for KeyEvent {
    fn from(value: crossterm::event::KeyEvent) -> Self {
        Self {
            code: value.code,
            modifiers: value.modifiers.into(),
            kind: value.kind.into(),
        }
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct KeyModifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

impl KeyModifiers {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_ctrl(self, ctrl: bool) -> Self {
        Self { ctrl, ..self }
    }

    pub fn set_alt(self, alt: bool) -> Self {
        Self { alt, ..self }
    }

    pub(crate) fn add_shift(self, shift: bool) -> Self {
        if !shift {
            return self;
        }
        Self {
            shift: true,
            ..self
        }
    }

    pub fn set_shift(self, shift: bool) -> Self {
        Self { shift, ..self }
    }

    pub fn display(&self) -> String {
        [
            self.ctrl.then_some("ctrl"),
            self.alt.then_some("alt"),
            self.shift.then_some("shift"),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join("+")
    }
}

impl From<crossterm::event::KeyModifiers> for KeyModifiers {
    fn from(value: crossterm::event::KeyModifiers) -> Self {
        Self {
            shift: value.contains(crossterm::event::KeyModifiers::SHIFT),
            ctrl: value.contains(crossterm::event::KeyModifiers::CONTROL),
            alt: value.contains(crossterm::event::KeyModifiers::ALT),
        }
    }
}

impl From<HashSet<KeyModifiers>> for KeyModifiers {
    fn from(value: HashSet<KeyModifiers>) -> Self {
        Self {
            shift: value.iter().any(|x| x.shift),
            alt: value.iter().any(|x| x.alt),
            ctrl: value.iter().any(|x| x.ctrl),
        }
    }
}
