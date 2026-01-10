pub mod event;

use std::collections::HashSet;

pub use crate::event::{KeyEvent, KeyModifiers};

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
    fn to_key_event(&self) -> Result<KeyEvent, ParseError> {
        fn new_key_event(
            key_code: KeyCode,
            modifiers: KeyModifiers,
            is_released: bool,
        ) -> KeyEvent {
            if is_released {
                KeyEvent::released(key_code, modifiers)
            } else {
                KeyEvent::pressed(key_code, modifiers)
            }
        }

        fn to_key_event(token: &Token, is_released: bool) -> Result<KeyEvent, ParseError> {
            if token.0.starts_with("release-") {
                return to_key_event(
                    &Token(token.0.trim_start_matches("release-").to_string()),
                    true,
                );
            }
            match token.0.split('+').collect::<Vec<_>>().split_last() {
                Some((key, modifiers)) => {
                    let result = Token::parse_key_code(key)?;
                    Ok(new_key_event(
                        result.key_code,
                        Token::parse_modifiers(modifiers)?.add_shift(result.shift),
                        is_released,
                    ))
                }
                _ => {
                    let result = Token::parse_key_code(&token.0)?;
                    Ok(new_key_event(
                        result.key_code,
                        KeyModifiers::None.add_shift(result.shift),
                        is_released,
                    ))
                }
            }
        }

        to_key_event(self, false)
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

    fn parse_key_code(s: &str) -> Result<ParseKeyCodeResult, ParseError> {
        match s {
            "enter" => Ok(ParseKeyCodeResult::from_key_code(KeyCode::Enter)),
            "esc" => Ok(ParseKeyCodeResult::from_key_code(KeyCode::Esc)),
            "backspace" => Ok(ParseKeyCodeResult::from_key_code(KeyCode::Backspace)),
            "left" => Ok(ParseKeyCodeResult::from_key_code(KeyCode::Left)),
            "right" => Ok(ParseKeyCodeResult::from_key_code(KeyCode::Right)),
            "up" => Ok(ParseKeyCodeResult::from_key_code(KeyCode::Up)),
            "down" => Ok(ParseKeyCodeResult::from_key_code(KeyCode::Down)),
            "home" => Ok(ParseKeyCodeResult::from_key_code(KeyCode::Home)),
            "end" => Ok(ParseKeyCodeResult::from_key_code(KeyCode::End)),
            "pageup" => Ok(ParseKeyCodeResult::from_key_code(KeyCode::PageUp)),
            "pagedown" => Ok(ParseKeyCodeResult::from_key_code(KeyCode::PageDown)),
            "tab" => Ok(ParseKeyCodeResult::from_key_code(KeyCode::Tab)),
            "backtab" => Ok(ParseKeyCodeResult::from_key_code(KeyCode::BackTab)),
            "delete" => Ok(ParseKeyCodeResult::from_key_code(KeyCode::Delete)),
            "insert" => Ok(ParseKeyCodeResult::from_key_code(KeyCode::Insert)),
            "space" => Ok(ParseKeyCodeResult::from_key_code(KeyCode::Char(' '))),
            "backslash" => Ok(ParseKeyCodeResult::from_key_code(KeyCode::Char('\\'))),
            "pipe" => Ok(ParseKeyCodeResult::from_key_code(KeyCode::Char('|'))),
            _ if s.len() == 1 => {
                let c = s.chars().next().unwrap();
                Ok(ParseKeyCodeResult {
                    key_code: KeyCode::Char(c),
                    shift: c.is_uppercase(),
                })
            }
            _ => Err(ParseError::UnknownKeyCode(s.to_string())),
        }
    }
}

struct ParseKeyCodeResult {
    key_code: KeyCode,
    shift: bool,
}

impl ParseKeyCodeResult {
    fn from_key_code(key_code: KeyCode) -> Self {
        Self {
            key_code,
            shift: false,
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
        write!(f, "{self:?}")
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
    fn reciprocity() {
        fn run_test(input: &'static str) {
            assert_eq!(parse_key_events(input).unwrap()[0].display(), input)
        }
        run_test("space");
        run_test("ctrl+a");
        run_test("ctrl+shift+t");
        run_test("alt+shift+backspace");
    }

    #[test]
    fn alphabetic_char() {
        assert_eq!(
            parse_key_events("a").unwrap(),
            vec![KeyEvent::pressed(KeyCode::Char('a'), KeyModifiers::None)]
        );
    }

    #[test]
    fn uppercase_char_should_have_shift() {
        assert_eq!(
            parse_key_events("A").unwrap(),
            vec![KeyEvent::pressed(KeyCode::Char('A'), KeyModifiers::Shift)]
        )
    }

    #[test]
    fn released_events() {
        assert_eq!(
            parse_key_events("release-A release-ctrl+g").unwrap(),
            vec![
                KeyEvent::released(KeyCode::Char('A'), KeyModifiers::Shift),
                KeyEvent::released(KeyCode::Char('g'), KeyModifiers::Ctrl)
            ]
        )
    }

    #[test]
    fn modifier() {
        assert_eq!(
            parse_key_events("ctrl+a").unwrap(),
            vec![KeyEvent::pressed(KeyCode::Char('a'), KeyModifiers::Ctrl)]
        );

        assert_eq!(
            parse_key_events("alt+a").unwrap(),
            vec![KeyEvent::pressed(KeyCode::Char('a'), KeyModifiers::Alt)]
        );

        assert_eq!(
            parse_key_events("shift+a").unwrap(),
            vec![KeyEvent::pressed(KeyCode::Char('a'), KeyModifiers::Shift)]
        );

        assert_eq!(
            parse_key_events("ctrl+alt+a").unwrap(),
            vec![KeyEvent::pressed(KeyCode::Char('a'), KeyModifiers::CtrlAlt)]
        );

        assert_eq!(
            parse_key_events("ctrl+shift+a").unwrap(),
            vec![KeyEvent::pressed(
                KeyCode::Char('a'),
                KeyModifiers::CtrlShift
            )]
        );

        assert_eq!(
            parse_key_events("alt+shift+a").unwrap(),
            vec![KeyEvent::pressed(
                KeyCode::Char('a'),
                KeyModifiers::AltShift
            )]
        );

        assert_eq!(
            parse_key_events("ctrl+alt+shift+a").unwrap(),
            vec![KeyEvent::pressed(
                KeyCode::Char('a'),
                KeyModifiers::CtrlAltShift
            )]
        );
    }

    #[test]
    fn invisible_keys() {
        assert_eq!(
            parse_key_events("enter").unwrap(),
            vec![KeyEvent::pressed(KeyCode::Enter, KeyModifiers::None)]
        );

        assert_eq!(
            parse_key_events("esc").unwrap(),
            vec![KeyEvent::pressed(KeyCode::Esc, KeyModifiers::None)]
        );

        assert_eq!(
            parse_key_events("backspace").unwrap(),
            vec![KeyEvent::pressed(KeyCode::Backspace, KeyModifiers::None)]
        );

        assert_eq!(
            parse_key_events("up").unwrap(),
            vec![KeyEvent::pressed(KeyCode::Up, KeyModifiers::None)]
        );

        assert_eq!(
            parse_key_events("down").unwrap(),
            vec![KeyEvent::pressed(KeyCode::Down, KeyModifiers::None)]
        );

        assert_eq!(
            parse_key_events("left").unwrap(),
            vec![KeyEvent::pressed(KeyCode::Left, KeyModifiers::None)]
        );

        assert_eq!(
            parse_key_events("right").unwrap(),
            vec![KeyEvent::pressed(KeyCode::Right, KeyModifiers::None)]
        );

        assert_eq!(
            parse_key_events("home").unwrap(),
            vec![KeyEvent::pressed(KeyCode::Home, KeyModifiers::None)]
        );

        assert_eq!(
            parse_key_events("end").unwrap(),
            vec![KeyEvent::pressed(KeyCode::End, KeyModifiers::None)]
        );

        assert_eq!(
            parse_key_events("pageup").unwrap(),
            vec![KeyEvent::pressed(KeyCode::PageUp, KeyModifiers::None)]
        );

        assert_eq!(
            parse_key_events("pagedown").unwrap(),
            vec![KeyEvent::pressed(KeyCode::PageDown, KeyModifiers::None)]
        );

        assert_eq!(
            parse_key_events("delete").unwrap(),
            vec![KeyEvent::pressed(KeyCode::Delete, KeyModifiers::None)]
        );

        assert_eq!(
            parse_key_events("tab").unwrap(),
            vec![KeyEvent::pressed(KeyCode::Tab, KeyModifiers::None)]
        );
    }

    #[test]
    fn multiple() {
        assert_eq!(
            parse_key_events("a b c alt+enter").unwrap(),
            vec![
                KeyEvent::pressed(KeyCode::Char('a'), KeyModifiers::None),
                KeyEvent::pressed(KeyCode::Char('b'), KeyModifiers::None),
                KeyEvent::pressed(KeyCode::Char('c'), KeyModifiers::None),
                KeyEvent::pressed(KeyCode::Enter, KeyModifiers::Alt),
            ]
        );
    }
}
