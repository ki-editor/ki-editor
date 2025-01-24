use crate::buffer::Buffer;

use super::{ByteRange, SelectionMode};

pub struct Token {
    normal_regex: super::Regex,
    symbol_skipping_regex: super::Regex,
    skip_symbols: bool,
}

impl Token {
    pub(crate) fn new(buffer: &Buffer, skip_symbols: bool) -> anyhow::Result<Self> {
        let config = crate::list::grep::RegexConfig {
            escaped: false,
            case_sensitive: false,
            match_whole_word: false,
        };
        Ok(Self {
            normal_regex: super::Regex::from_config(buffer, r"((\w|-)+)|([^a-zA-Z\d\s])", config)?,
            symbol_skipping_regex: super::Regex::from_config(buffer, r"(\w|-)+", config)?,
            skip_symbols,
        })
    }
}
impl SelectionMode for Token {
    fn iter<'a>(
        &'a self,
        params: super::SelectionModeParams<'a>,
    ) -> anyhow::Result<Box<dyn Iterator<Item = ByteRange> + 'a>> {
        if self.skip_symbols {
            self.symbol_skipping_regex.iter(params)
        } else {
            self.normal_regex.iter(params)
        }
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
