use ropey::Rope;

use crate::{
    buffer::Buffer, char_index_range::CharIndexRange, components::editor::IfCurrentNotFound,
    selection::CharIndex,
};

use super::{ByteRange, SelectionMode};

pub struct Token {
    skip_symbols: bool,
}

impl Token {
    pub(crate) fn new(buffer: &Buffer, skip_symbols: bool) -> anyhow::Result<Self> {
        let config = crate::list::grep::RegexConfig {
            escaped: false,
            case_sensitive: false,
            match_whole_word: false,
        };
        Ok(Self { skip_symbols })
    }
}

fn current_impl(
    rope: &Rope,
    cursor_char_index: CharIndex,
    if_current_not_found: IfCurrentNotFound,
    skip_symbols: bool,
) -> anyhow::Result<Option<CharIndexRange>> {
    let last_char_index = CharIndex(rope.len_chars().saturating_sub(1));
    let is_word = |char: char| char.is_alphanumeric() || char == '_' || char == '-';
    let is_symbol = |char: char| !is_word(char) && !char.is_whitespace();
    if cursor_char_index > last_char_index {
        return Ok(None);
    }
    let Some(current) = ({
        let predicate = |char: char| {
            if skip_symbols {
                is_word(char)
            } else {
                is_word(char) || is_symbol(char)
            }
        };
        match if_current_not_found {
            IfCurrentNotFound::LookForward => {
                let mut index = cursor_char_index;
                loop {
                    let char = rope.char(index.0);
                    if !predicate(char) {
                        index = index + 1
                    } else {
                        break Some(index);
                    }
                    if index >= last_char_index {
                        break None;
                    }
                }
            }
            IfCurrentNotFound::LookBackward => {
                let mut index = cursor_char_index;
                loop {
                    if index == CharIndex(0) {
                        break None;
                    }
                    let char = rope.char(index.0);
                    if !predicate(char) {
                        index = index - 1
                    } else {
                        break Some(index);
                    }
                }
            }
        }
    }) else {
        return Ok(None);
    };
    if current.0 >= rope.len_chars() {
        return Ok(None);
    }
    if !skip_symbols && is_symbol(rope.char(current.0)) {
        return Ok(Some((current..current + 1).into()));
    }
    let start = {
        let mut index = current;
        loop {
            if index.0 == 0 {
                break index;
            }
            let char = rope.char(index.0.saturating_sub(1));
            if is_word(char) {
                index = index - 1
            } else {
                break index;
            }
        }
    };
    let end = {
        let mut index = current;
        loop {
            if index == last_char_index {
                break index;
            }
            let char = rope.char(index.0 + 1);
            if is_word(char) {
                index = index + 1
            } else {
                break index;
            }
        }
    } + 1;
    debug_assert!(is_word(rope.char(current.0)));
    debug_assert!(is_word(rope.char(start.0)));
    debug_assert!(is_word(rope.char((end - 1).0)));
    Ok(Some((start..end).into()))
}
impl SelectionMode for Token {
    fn iter<'a>(
        &'a self,
        params: super::SelectionModeParams<'a>,
    ) -> anyhow::Result<Box<dyn Iterator<Item = ByteRange> + 'a>> {
        struct MyIterator<'a> {
            params: super::SelectionModeParams<'a>,
            cursor_char_index: CharIndex,
            skip_symbols: bool,
        }
        impl<'a> Iterator for MyIterator<'a> {
            type Item = ByteRange;

            fn next(&mut self) -> Option<Self::Item> {
                let next_char_index_range = current_impl(
                    self.params.buffer.rope(),
                    self.cursor_char_index,
                    IfCurrentNotFound::LookForward,
                    self.skip_symbols,
                )
                .ok()??;
                let next_byte_range = ByteRange::new(
                    self.params
                        .buffer
                        .char_index_range_to_byte_range(next_char_index_range)
                        .ok()?,
                );

                self.cursor_char_index = next_char_index_range.end;

                Some(next_byte_range)
            }
        }
        Ok(Box::new(MyIterator {
            params,
            cursor_char_index: CharIndex(0),
            skip_symbols: self.skip_symbols,
        }))
    }
    fn first(
        &self,
        params: super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        let buffer = params.buffer;
        let line = buffer.char_to_line(params.current_selection.range().start)?;
        let byte_range = buffer.line_to_byte_range(line)?.range;
        let current_selection = params.current_selection.clone();
        Ok(self
            .iter_filtered(params)?
            .find(|range| {
                byte_range.start <= range.range.start && range.range.end <= byte_range.end
            })
            .and_then(|range| {
                Some(
                    current_selection
                        .set_range(buffer.byte_range_to_char_index_range(&range.range).ok()?),
                )
            }))
    }
    fn last(
        &self,
        params: super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        let buffer = params.buffer;
        let line = buffer.char_to_line(params.current_selection.range().start)?;
        let byte_range = buffer.line_to_byte_range(line)?.range;
        let current_selection = params.current_selection.clone();
        Ok(self
            .iter_filtered(params)?
            .filter(|range| {
                byte_range.start <= range.range.start && range.range.end <= byte_range.end + 1
            })
            .last()
            .and_then(|range| {
                Some(
                    current_selection
                        .set_range(buffer.byte_range_to_char_index_range(&range.range).ok()?),
                )
            }))
    }
    fn current(
        &self,
        params: super::SelectionModeParams,
        if_current_not_found: IfCurrentNotFound,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        let rope = params.buffer.rope();
        let cursor = params.cursor_char_index();
        Ok(
            current_impl(rope, cursor, if_current_not_found, self.skip_symbols)?
                .map(|range| params.current_selection.clone().set_range(range)),
        )
    }
    fn right(
        &self,
        params: super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        Ok(current_impl(
            params.buffer.rope(),
            params.current_selection.range().end,
            IfCurrentNotFound::LookForward,
            self.skip_symbols,
        )?
        .map(|range| params.current_selection.clone().set_range(range)))
    }
    fn left(
        &self,
        params: super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        Ok(current_impl(
            params.buffer.rope(),
            params.current_selection.range().start - 1,
            IfCurrentNotFound::LookForward,
            self.skip_symbols,
        )?
        .map(|range| params.current_selection.clone().set_range(range)))
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
        Token::new(&buffer, true).unwrap().assert_all_selections(
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
        Token::new(&buffer, false).unwrap().assert_all_selections(
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
