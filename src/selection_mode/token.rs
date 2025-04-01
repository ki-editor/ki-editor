use ropey::Rope;

use crate::{
    char_index_range::CharIndexRange, components::editor::IfCurrentNotFound, selection::CharIndex,
};

use super::{ByteRange, SelectionMode};

pub struct Token {
    skip_symbols: bool,
}

impl Token {
    pub(crate) fn new(skip_symbols: bool) -> anyhow::Result<Self> {
        Ok(Self { skip_symbols })
    }
}

fn current_impl(
    rope: &Rope,
    cursor_char_index: CharIndex,
    skip_symbols: bool,
) -> anyhow::Result<Option<CharIndexRange>> {
    let last_char_index = CharIndex(rope.len_chars().saturating_sub(1));

    // Define predicates once
    let is_word = |char: char| char.is_alphanumeric() || char == '_' || char == '-';
    let is_symbol = |char: char| !is_word(char) && !char.is_whitespace();
    let is_target = |char: char| {
        if skip_symbols {
            is_word(char)
        } else {
            is_word(char) || is_symbol(char)
        }
    };

    if cursor_char_index > last_char_index {
        return Ok(None);
    }

    if !is_target(rope.char(cursor_char_index.0)) {
        return Ok(None);
    }

    // Handle single symbol case
    if !skip_symbols && is_symbol(rope.char(cursor_char_index.0)) {
        return Ok(Some((cursor_char_index..cursor_char_index + 1).into()));
    }

    // Find word boundaries
    let start = find_word_start(rope, cursor_char_index, is_word);
    let end = find_word_end(rope, cursor_char_index, last_char_index, is_word) + 1;

    // Validate results
    debug_assert!(is_word(rope.char(cursor_char_index.0)));
    debug_assert!(is_word(rope.char(start.0)));
    debug_assert!(is_word(rope.char((end - 1).0)));

    Ok(Some((start..end).into()))
}

fn find_current_position(
    rope: &Rope,
    cursor_char_index: CharIndex,
    if_current_not_found: IfCurrentNotFound,
    is_target: impl Fn(char) -> bool,
) -> anyhow::Result<Option<CharIndex>> {
    let last_char_index = CharIndex(rope.len_chars().saturating_sub(1));

    match if_current_not_found {
        IfCurrentNotFound::LookForward => {
            let mut index = cursor_char_index;
            while index <= last_char_index {
                if is_target(rope.char(index.0)) {
                    return Ok(Some(index));
                }
                index = index + 1;
                if index >= last_char_index {
                    return Ok(None);
                }
            }
            Ok(None)
        }
        IfCurrentNotFound::LookBackward => {
            let mut index = cursor_char_index;
            while index >= CharIndex(0) {
                if is_target(rope.char(index.0)) {
                    return Ok(Some(index));
                }
                if index == CharIndex(0) {
                    return Ok(None);
                }
                index = index - 1;
            }
            Ok(None)
        }
    }
}

fn find_word_start(rope: &Rope, current: CharIndex, is_word: impl Fn(char) -> bool) -> CharIndex {
    // Create a reverse range from current.0 down to 1 (not including 0)
    for i in (1..=current.0).rev() {
        let prev_char = rope.char(i - 1);
        if !is_word(prev_char) {
            return CharIndex(i);
        }
    }
    // If we've examined all characters to the start, return index 0
    CharIndex(0)
}

fn find_word_end(
    rope: &Rope,
    current: CharIndex,
    last_char_index: CharIndex,
    is_word: impl Fn(char) -> bool,
) -> CharIndex {
    // Create a range from current.0+1 to last_char_index.0
    for i in (current.0 + 1)..=last_char_index.0 {
        let char = rope.char(i);
        if !is_word(char) {
            return CharIndex(i - 1);
        }
    }
    // If we've examined all characters to the end, return the last index
    last_char_index
}
impl SelectionMode for Token {
    fn first(
        &self,
        params: &super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        let buffer = params.buffer;
        let current_line_index = buffer.char_to_line(params.cursor_char_index())?;
        let line_start_char_index = buffer.line_to_char(current_line_index)?;
        let current_selection = params.current_selection.clone();
        if let Some(range) = self.get_current_selection_by_cursor(
            &params.buffer,
            line_start_char_index,
            IfCurrentNotFound::LookForward,
        )? {
            if buffer.byte_to_line(range.range.start)? == current_line_index {
                return Ok(Some(range.to_selection(buffer, &current_selection)?));
            }
        }
        Ok(None)
    }
    fn last(
        &self,
        params: &super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        let buffer = params.buffer;
        let current_line_index = buffer.char_to_line(params.cursor_char_index())?;
        let next_line_index = current_line_index + 1;
        let line_end_char_index = buffer.line_to_char(next_line_index)? - 1;
        let current_selection = params.current_selection.clone();
        if let Some(range) = self.get_current_selection_by_cursor(
            &params.buffer,
            line_end_char_index,
            IfCurrentNotFound::LookBackward,
        )? {
            if buffer.byte_to_line(range.range.start)? == current_line_index {
                return Ok(Some(range.to_selection(buffer, &current_selection)?));
            }
        }
        Ok(None)
    }
    fn get_current_selection_by_cursor(
        &self,
        buffer: &crate::buffer::Buffer,
        cursor_char_index: crate::selection::CharIndex,
        if_current_not_found: IfCurrentNotFound,
    ) -> anyhow::Result<Option<super::ByteRange>> {
        let last_char_index = CharIndex(buffer.len_chars().saturating_sub(1));

        // Define predicates once
        let is_word = |char: char| char.is_alphanumeric() || char == '_' || char == '-';
        let is_symbol = |char: char| !is_word(char) && !char.is_whitespace();
        let is_target = |char: char| {
            if self.skip_symbols {
                is_word(char)
            } else {
                is_word(char) || is_symbol(char)
            }
        };

        if cursor_char_index > last_char_index {
            return Ok(None);
        }

        let last_char_index = CharIndex(buffer.len_chars().saturating_sub(1));

        let current = {
            let mut current = cursor_char_index;
            loop {
                if (CharIndex(0)..=last_char_index).contains(&current) {
                    if is_target(buffer.char(current)) {
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

        let rope = buffer.rope();
        if !is_target(rope.char(current.0)) {
            return Ok(None);
        }

        // Handle single symbol case
        if !self.skip_symbols && is_symbol(rope.char(current.0)) {
            let current_byte = rope.try_char_to_byte(current.0)?;
            return Ok(Some(ByteRange::new(current_byte..current_byte + 1)));
        }

        // Find word boundaries
        let start = find_word_start(rope, current, is_word);
        let end = find_word_end(rope, current, last_char_index, is_word) + 1;

        // Validate results
        debug_assert!(is_word(rope.char(current.0)));
        debug_assert!(is_word(rope.char(start.0)));
        debug_assert!(is_word(rope.char((end - 1).0)));

        Ok(Some(ByteRange::new(
            rope.try_char_to_byte(start.0)?..rope.try_char_to_byte(end.0)?,
        )))
    }
}

#[cfg(test)]
mod test_token {
    use crate::{buffer::Buffer, selection::Selection, selection_mode::SelectionMode};

    use super::*;

    #[test]
    fn skip_symbols() {
        let buffer = Buffer::new(
            None,
            "snake_case camelCase PascalCase UPPER_SNAKE kebab-case ->() 123 <_>",
        );
        Token::new(true).unwrap().assert_all_selections(
            &buffer,
            Selection::default(),
            &[
                (0..10, "snake_case"),
                (11..20, "camelCase"),
                (21..31, "PascalCase"),
                (32..43, "UPPER_SNAKE"),
                (44..54, "kebab-case"),
                (55..56, "-"),
                (60..63, "123"),
                (65..66, "_"),
            ],
        );
    }
    #[test]
    fn no_skip_symbols() {
        let buffer = Buffer::new(
            None,
            "snake_case camelCase PascalCase UPPER_SNAKE kebab-case ->() 123 <_>",
        );
        Token::new(false).unwrap().assert_all_selections(
            &buffer,
            Selection::default(),
            &[
                (0..10, "snake_case"),
                (11..20, "camelCase"),
                (21..31, "PascalCase"),
                (32..43, "UPPER_SNAKE"),
                (44..54, "kebab-case"),
                (55..56, "-"),
                (56..57, ">"),
                (57..58, "("),
                (58..59, ")"),
                (60..63, "123"),
                (64..65, "<"),
                (65..66, "_"),
                (66..67, ">"),
            ],
        );
    }
}
