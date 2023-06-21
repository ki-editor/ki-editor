use std::error::Error;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, PartialEq)]
struct Token(String);

pub fn parse_key_events(input: &str) -> Result<Vec<KeyEvent>, ParseError> {
    input
        .split(' ')
        .map(|s| Token(s.into()).to_key_event())
        .collect::<Result<Vec<_>, _>>()
}

impl Token {
    fn to_key_event(self) -> Result<KeyEvent, ParseError> {
        match self.0.split('-').collect::<Vec<_>>().split_first() {
            Some((modifier, [key])) => Ok(KeyEvent::new(
                Token::parse_key_code(key)?,
                Token::parse_modifier(modifier)?,
            )),
            _ => Ok(KeyEvent::new(
                Token::parse_key_code(&self.0)?,
                KeyModifiers::NONE,
            )),
        }
    }

    fn parse_modifier(s: &str) -> Result<KeyModifiers, ParseError> {
        match s {
            "c" => Ok(KeyModifiers::CONTROL),
            "a" => Ok(KeyModifiers::ALT),
            "s" => Ok(KeyModifiers::SHIFT),
            "ca" => Ok(KeyModifiers::CONTROL | KeyModifiers::ALT),
            "cs" => Ok(KeyModifiers::CONTROL | KeyModifiers::SHIFT),
            "as" => Ok(KeyModifiers::ALT | KeyModifiers::SHIFT),
            "cas" => Ok(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SHIFT),
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
mod test_parse_keys {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::parse_key_events;

    #[test]
    fn alphabetic_char() {
        assert_eq!(
            parse_key_events("a").unwrap(),
            vec![KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE)]
        );
    }

    #[test]
    fn modifier() {
        assert_eq!(
            parse_key_events("c-a").unwrap(),
            vec![KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL)]
        );

        assert_eq!(
            parse_key_events("a-a").unwrap(),
            vec![KeyEvent::new(KeyCode::Char('a'), KeyModifiers::ALT)]
        );

        assert_eq!(
            parse_key_events("s-a").unwrap(),
            vec![KeyEvent::new(KeyCode::Char('a'), KeyModifiers::SHIFT)]
        );

        assert_eq!(
            parse_key_events("ca-a").unwrap(),
            vec![KeyEvent::new(
                KeyCode::Char('a'),
                KeyModifiers::CONTROL | KeyModifiers::ALT
            )]
        );

        assert_eq!(
            parse_key_events("cs-a").unwrap(),
            vec![KeyEvent::new(
                KeyCode::Char('a'),
                KeyModifiers::CONTROL | KeyModifiers::SHIFT
            )]
        );

        assert_eq!(
            parse_key_events("as-a").unwrap(),
            vec![KeyEvent::new(
                KeyCode::Char('a'),
                KeyModifiers::ALT | KeyModifiers::SHIFT
            )]
        );

        assert_eq!(
            parse_key_events("cas-a").unwrap(),
            vec![KeyEvent::new(
                KeyCode::Char('a'),
                KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SHIFT
            )]
        );
    }

    #[test]
    fn invisible_keys() {
        assert_eq!(
            parse_key_events("enter").unwrap(),
            vec![KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)]
        );

        assert_eq!(
            parse_key_events("esc").unwrap(),
            vec![KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)]
        );

        assert_eq!(
            parse_key_events("backspace").unwrap(),
            vec![KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE)]
        );

        assert_eq!(
            parse_key_events("up").unwrap(),
            vec![KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)]
        );

        assert_eq!(
            parse_key_events("down").unwrap(),
            vec![KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)]
        );

        assert_eq!(
            parse_key_events("left").unwrap(),
            vec![KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)]
        );

        assert_eq!(
            parse_key_events("right").unwrap(),
            vec![KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)]
        );

        assert_eq!(
            parse_key_events("home").unwrap(),
            vec![KeyEvent::new(KeyCode::Home, KeyModifiers::NONE)]
        );

        assert_eq!(
            parse_key_events("end").unwrap(),
            vec![KeyEvent::new(KeyCode::End, KeyModifiers::NONE)]
        );

        assert_eq!(
            parse_key_events("pageup").unwrap(),
            vec![KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE)]
        );

        assert_eq!(
            parse_key_events("pagedown").unwrap(),
            vec![KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE)]
        );

        assert_eq!(
            parse_key_events("delete").unwrap(),
            vec![KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE)]
        );

        assert_eq!(
            parse_key_events("tab").unwrap(),
            vec![KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)]
        );
    }

    fn multiple() {
        assert_eq!(
            parse_key_events("a b c a-enter").unwrap(),
            vec![
                KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE),
                KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE),
                KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE),
                KeyEvent::new(KeyCode::Char('a'), KeyModifiers::ALT),
            ]
        );
    }
}
