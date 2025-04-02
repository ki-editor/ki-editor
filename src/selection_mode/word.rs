use ropey::Rope;

use super::{ByteRange, SelectionMode, Token};
use crate::{buffer::Buffer, components::editor::IfCurrentNotFound, selection::CharIndex};

pub struct Word {
    skip_symbols: bool,
}

const SUBWORD_REGEX: &str =
    r"[A-Z]{2,}(?=[A-Z][a-z])|[A-Z]{2,}|[A-Z][a-z]+|[A-Z]|[a-z]+|[^\w\s]|_|[0-9]+";

const SUBWORD_SYMBOL_SKIPPING_REGEX: &str =
    r"[A-Z]{2,}(?=[A-Z][a-z])|[A-Z]{2,}|[A-Z][a-z]+|[A-Z]|[a-z]+|[0-9]+";

impl Word {
    pub(crate) fn new(skip_symbols: bool) -> anyhow::Result<Self> {
        Ok(Self { skip_symbols })
    }
}

impl SelectionMode for Word {
    fn first(
        &self,
        params: &super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        get_word(params, SelectionPosition::First, self.skip_symbols)
    }

    fn last(
        &self,
        params: &super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        get_word(params, SelectionPosition::Last, self.skip_symbols)
    }

    fn get_current_selection_by_cursor(
        &self,
        buffer: &Buffer,
        cursor_char_index: CharIndex,
        if_current_not_found: IfCurrentNotFound,
    ) -> anyhow::Result<Option<ByteRange>> {
        let rope = buffer.rope();
        let last_char_index = CharIndex(rope.len_chars().saturating_sub(1));

        if cursor_char_index > last_char_index {
            return Ok(None);
        }

        let predicate = |c: char| {
            if self.skip_symbols {
                c.is_ascii_alphanumeric()
            } else {
                !c.is_whitespace()
            }
        };
        let current = {
            let mut current = cursor_char_index;
            loop {
                if (CharIndex(0)..=last_char_index).contains(&current) {
                    if predicate(buffer.char(current)) {
                        break current;
                    } else {
                        match if_current_not_found {
                            IfCurrentNotFound::LookForward if current < last_char_index => {
                                current = current + 1
                            }
                            IfCurrentNotFound::LookBackward if current > CharIndex(0) => {
                                current = current - 1
                            }
                            _ => break current,
                        }
                    }
                } else {
                    return Ok(None);
                }
            }
        };

        let current_char = buffer.char(current);

        if current_char.is_whitespace()
            || (self.skip_symbols && !current_char.is_ascii_alphanumeric())
        {
            return Ok(None);
        }

        let range: crate::char_index_range::CharIndexRange = if current_char.is_ascii_lowercase() {
            let start = {
                let mut index = current;
                loop {
                    if index > CharIndex(0) && buffer.char(index - 1).is_ascii_lowercase() {
                        index = index - 1;
                    } else if index > CharIndex(0) && buffer.char(index - 1).is_ascii_uppercase() {
                        break index - 1;
                    } else {
                        break index;
                    }
                }
            };
            let end = {
                let mut index = current;
                loop {
                    if index < last_char_index && buffer.char(index + 1).is_ascii_lowercase() {
                        index = index + 1;
                    } else {
                        break index;
                    }
                }
            };
            start..end + 1
        } else if current_char.is_ascii_uppercase() {
            let start = {
                let mut index = current;
                if index < last_char_index && buffer.char(index + 1).is_lowercase() {
                    index
                } else {
                    loop {
                        if index == CharIndex(0) {
                            break index;
                        }
                        let char = buffer.char(index - 1);
                        if char.is_ascii_uppercase() {
                            index = index - 1;
                        } else {
                            break index;
                        }
                    }
                }
            };
            let end = {
                let mut previous_is_uppercase = buffer.char(current).is_ascii_uppercase();
                let mut index = current;
                loop {
                    if index >= last_char_index {
                        break index;
                    }
                    let char = buffer.char(index + 1);
                    if char.is_ascii_lowercase() {
                        previous_is_uppercase = char.is_ascii_uppercase();
                        index = index + 1;
                    } else if previous_is_uppercase && char.is_ascii_uppercase() {
                        if index < last_char_index - 1
                            && buffer.char(index + 2).is_ascii_lowercase()
                        {
                            break index;
                        } else {
                            previous_is_uppercase = char.is_ascii_uppercase();
                            index = index + 1;
                        }
                    } else {
                        break index;
                    }
                }
            };
            start..end + 1
        } else if current_char.is_digit(10) {
            let start = {
                let mut index = current;
                loop {
                    if index > CharIndex(0) && buffer.char(index - 1).is_digit(10) {
                        index = index - 1;
                    } else {
                        break index;
                    }
                }
            };
            let end = {
                let mut index = current;
                loop {
                    if index < last_char_index && buffer.char(index + 1).is_digit(10) {
                        index = index + 1;
                    } else {
                        break index;
                    }
                }
            };
            start..end + 1
        } else {
            current..current + 1
        }
        .into();

        Ok(Some(ByteRange::new(
            buffer.char_index_range_to_byte_range(range)?,
        )))
    }
}

#[cfg(test)]
mod test_word {
    use super::*;
    use crate::{buffer::Buffer, selection::Selection, selection_mode::SelectionMode};

    #[test]
    fn simple_case() {
        let buffer = Buffer::new(None, "snake Case camel");
        Word::new(true).unwrap().assert_all_selections(
            &buffer,
            Selection::default(),
            &[(0..5, "snake"), (6..10, "Case"), (11..16, "camel")],
        );
    }

    #[test]
    fn skip_symbols() {
        let buffer = Buffer::new(
            None,
            "snake_case camelCase PascalCase UPPER_SNAKE ->() 123 <_> HTTPNetwork X",
        );
        Word::new(true).unwrap().assert_all_selections(
            &buffer,
            Selection::default(),
            &[
                (0..5, "snake"),
                (6..10, "case"),
                (11..16, "camel"),
                (16..20, "Case"),
                (21..27, "Pascal"),
                (27..31, "Case"),
                (32..37, "UPPER"),
                (38..43, "SNAKE"),
                (49..52, "123"),
                (57..61, "HTTP"),
                (61..68, "Network"),
                (69..70, "X"),
            ],
        );
    }

    #[test]
    fn no_skip_symbols() {
        let buffer = Buffer::new(
            None,
            "snake_case camelCase PascalCase UPPER_SNAKE ->() 123 <_> HTTPNetwork X",
        );
        Word::new(false).unwrap().assert_all_selections(
            &buffer,
            Selection::default(),
            &[
                (0..5, "snake"),
                (5..6, "_"),
                (6..10, "case"),
                (11..16, "camel"),
                (16..20, "Case"),
                (21..27, "Pascal"),
                (27..31, "Case"),
                (32..37, "UPPER"),
                (37..38, "_"),
                (38..43, "SNAKE"),
                (44..45, "-"),
                (45..46, ">"),
                (46..47, "("),
                (47..48, ")"),
                (49..52, "123"),
                (53..54, "<"),
                (54..55, "_"),
                (55..56, ">"),
                (57..61, "HTTP"),
                (61..68, "Network"),
                (69..70, "X"),
            ],
        );
    }

    #[test]
    fn consecutive_uppercase_letters() {
        let buffer = Buffer::new(None, "XMLParser JSONObject HTMLElement");
        Word::new(true).unwrap().assert_all_selections(
            &buffer,
            Selection::default(),
            &[
                (0..3, "XML"),
                (3..9, "Parser"),
                (10..14, "JSON"),
                (14..20, "Object"),
                (21..25, "HTML"),
                (25..32, "Element"),
            ],
        );
    }
}

#[derive(Clone, Copy)]
pub(crate) enum SelectionPosition {
    First,
    Last,
}

fn get_word(
    params: &super::SelectionModeParams,
    position: SelectionPosition,
    skip_symbols: bool,
) -> anyhow::Result<Option<crate::selection::Selection>> {
    if let Some(current_word) = Token::new(skip_symbols)?.current(
        params.clone(),
        crate::components::editor::IfCurrentNotFound::LookForward,
    )? {
        let content = params.buffer.slice(&current_word.range())?.to_string();
        let regex = fancy_regex::Regex::new(SUBWORD_REGEX)?;
        let mut captures = regex.captures_iter(&content);
        if let Some(match_) = match position {
            SelectionPosition::First => captures.next(),
            SelectionPosition::Last => captures.last(),
        } {
            let start = current_word.range().start;
            if let Some(range) = match_?.get(0).map(|m| start + m.start()..start + m.end()) {
                return Ok(Some(current_word.set_range(range.into())));
            }
        }
    }
    Ok(None)
}
