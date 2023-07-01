pub mod key_event;

use std::collections::HashSet;

pub use crate::key_event::{KeyEvent, KeyModifiers};

use crossterm::event::KeyCode;

#[derive(Debug, PartialEq)]
struct Token(String);

pub fn parse_key_events(input: &str) -> Result<Vec<KeyEvent>, ParseError> {
    input
        .split(' ')
        .map(parse_key_event)
        .collect::<Result<Vec<_>, _>>()
}

pub fn parse_key_event(input: &str) -> Result<KeyEvent, ParseError> {
    Token(input.into()).to_key_event()
}

impl Token {
    fn to_key_event(self) -> Result<KeyEvent, ParseError> {
        match self.0.split('+').collect::<Vec<_>>().split_last() {
            Some((key, modifiers)) => Ok(KeyEvent::new(
                Token::parse_key_code(key)?,
                Token::parse_modifiers(modifiers)?,
            )),
            _ => Ok(KeyEvent::new(
                Token::parse_key_code(&self.0)?,
                KeyModifiers::None,
            )),
        }
    }

    fn parse_modifiers(modifiers: &[&str]) -> Result<KeyModifiers, ParseError> {
        let set: HashSet<_> = modifiers
            .iter()
            .map(|m| Token::parse_modifier(m))
            .collect::<Result<HashSet<_>, _>>()?;

        Ok(set.into())
    }

    fn parse_modifier(s: &str) -> Result<KeyModifiers, ParseError> {
        match s {
            "ctrl" => Ok(KeyModifiers::Ctrl),
            "alt" => Ok(KeyModifiers::Alt),
            "shift" => Ok(KeyModifiers::Shift),
            _ => Err(ParseError::UnknownModifier(s.to_string())),
        }
    }

    fn parse_key_code(s: &str) -> Result<KeyCode, ParseError> {
        match s {
            "enter" => Ok(KeyCode::Enter),
            "esc" => Ok(KeyCode::Esc),
            "backspace" => Ok(KeyCode::Backspace),
            "left" => Ok(KeyCode::Left),
            "right" => Ok(KeyCode::Right),
            "up" => Ok(KeyCode::Up),
            "down" => Ok(KeyCode::Down),
            "home" => Ok(KeyCode::Home),
            "end" => Ok(KeyCode::End),
            "pageup" => Ok(KeyCode::PageUp),
            "pagedown" => Ok(KeyCode::PageDown),
            "tab" => Ok(KeyCode::Tab),
            "backtab" => Ok(KeyCode::BackTab),
            "delete" => Ok(KeyCode::Delete),
            "insert" => Ok(KeyCode::Insert),
            "space" => Ok(KeyCode::Char(' ')),
            _ if s.len() == 1 => Ok(KeyCode::Char(s.chars().next().unwrap())),
            _ => Err(ParseError::UnknownKeyCode(s.to_string())),
        }
    }
}

#[derive(Debug)]
pub enum ParseError {
    UnknownKeyCode(String),
    UnknownModifier(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for ParseError {}

#[cfg(test)]
mod test_key_event {
    use crossterm::event::KeyCode;

    use crate::{KeyEvent, KeyModifiers};

    use super::parse_key_events;
    use pretty_assertions::assert_eq;

    #[test]
    fn alphabetic_char() {
        assert_eq!(
            parse_key_events("a").unwrap(),
            vec![KeyEvent::new(KeyCode::Char('a'), KeyModifiers::None)]
        );
    }

    #[test]
    fn modifier() {
        assert_eq!(
            parse_key_events("ctrl+a").unwrap(),
            vec![KeyEvent::new(KeyCode::Char('a'), KeyModifiers::Ctrl)]
        );

        assert_eq!(
            parse_key_events("alt+a").unwrap(),
            vec![KeyEvent::new(KeyCode::Char('a'), KeyModifiers::Alt)]
        );

        assert_eq!(
            parse_key_events("shift+a").unwrap(),
            vec![KeyEvent::new(KeyCode::Char('a'), KeyModifiers::Shift)]
        );

        assert_eq!(
            parse_key_events("ctrl+alt+a").unwrap(),
            vec![KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CtrlAlt)]
        );

        assert_eq!(
            parse_key_events("ctrl+shift+a").unwrap(),
            vec![KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CtrlShift)]
        );

        assert_eq!(
            parse_key_events("alt+shift+a").unwrap(),
            vec![KeyEvent::new(KeyCode::Char('a'), KeyModifiers::AltShift)]
        );

        assert_eq!(
            parse_key_events("ctrl+alt+shift+a").unwrap(),
            vec![KeyEvent::new(
                KeyCode::Char('a'),
                KeyModifiers::CtrlAltShift
            )]
        );
    }

    #[test]
    fn invisible_keys() {
        assert_eq!(
            parse_key_events("enter").unwrap(),
            vec![KeyEvent::new(KeyCode::Enter, KeyModifiers::None)]
        );

        assert_eq!(
            parse_key_events("esc").unwrap(),
            vec![KeyEvent::new(KeyCode::Esc, KeyModifiers::None)]
        );

        assert_eq!(
            parse_key_events("backspace").unwrap(),
            vec![KeyEvent::new(KeyCode::Backspace, KeyModifiers::None)]
        );

        assert_eq!(
            parse_key_events("up").unwrap(),
            vec![KeyEvent::new(KeyCode::Up, KeyModifiers::None)]
        );

        assert_eq!(
            parse_key_events("down").unwrap(),
            vec![KeyEvent::new(KeyCode::Down, KeyModifiers::None)]
        );

        assert_eq!(
            parse_key_events("left").unwrap(),
            vec![KeyEvent::new(KeyCode::Left, KeyModifiers::None)]
        );

        assert_eq!(
            parse_key_events("right").unwrap(),
            vec![KeyEvent::new(KeyCode::Right, KeyModifiers::None)]
        );

        assert_eq!(
            parse_key_events("home").unwrap(),
            vec![KeyEvent::new(KeyCode::Home, KeyModifiers::None)]
        );

        assert_eq!(
            parse_key_events("end").unwrap(),
            vec![KeyEvent::new(KeyCode::End, KeyModifiers::None)]
        );

        assert_eq!(
            parse_key_events("pageup").unwrap(),
            vec![KeyEvent::new(KeyCode::PageUp, KeyModifiers::None)]
        );

        assert_eq!(
            parse_key_events("pagedown").unwrap(),
            vec![KeyEvent::new(KeyCode::PageDown, KeyModifiers::None)]
        );

        assert_eq!(
            parse_key_events("delete").unwrap(),
            vec![KeyEvent::new(KeyCode::Delete, KeyModifiers::None)]
        );

        assert_eq!(
            parse_key_events("tab").unwrap(),
            vec![KeyEvent::new(KeyCode::Tab, KeyModifiers::None)]
        );
    }

    #[test]
    fn multiple() {
        assert_eq!(
            parse_key_events("a b c alt+enter").unwrap(),
            vec![
                KeyEvent::new(KeyCode::Char('a'), KeyModifiers::None),
                KeyEvent::new(KeyCode::Char('b'), KeyModifiers::None),
                KeyEvent::new(KeyCode::Char('c'), KeyModifiers::None),
                KeyEvent::new(KeyCode::Enter, KeyModifiers::Alt),
            ]
        );
    }
}
