use itertools::Itertools;

use crate::buffer::Buffer;

use super::{ByteRange, SelectionMode};

pub struct Token {
    normal_regex: super::Regex,
    symbol_skipping_regex: super::Regex,
}

impl Token {
    pub(crate) fn new(buffer: &Buffer) -> anyhow::Result<Self> {
        let config = crate::list::grep::RegexConfig {
            escaped: false,
            case_sensitive: false,
            match_whole_word: false,
        };
        Ok(Self {
            normal_regex: super::Regex::from_config(buffer, r"((\w|-)+)|([^a-zA-Z\d\s])", config)?,
            symbol_skipping_regex: super::Regex::from_config(buffer, r"(\w|-)+", config)?,
        })
    }
}
impl SelectionMode for Token {
    fn iter<'a>(
        &'a self,
        params: super::SelectionModeParams<'a>,
    ) -> anyhow::Result<Box<dyn Iterator<Item = ByteRange> + 'a>> {
        self.normal_regex.iter(params)
    }
    fn next(
        &self,
        params: super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        self.symbol_skipping_regex.next(params)
    }
    fn previous(
        &self,
        params: super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        self.symbol_skipping_regex.previous(params)
    }
    fn first(
        &self,
        params: super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        let buffer = params.buffer;
        let byte_range = buffer.char_index_range_to_byte_range(params.current_selection.range())?;
        let current_selection = params.current_selection.clone();
        Ok(self
            .iter_filtered(params)?
            .take_while(|range| range.range != byte_range)
            .collect_vec()
            .into_iter()
            .rev()
            .take_while(|range| is_alphanumeric_word(buffer, range))
            .last()
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
        let byte_range = buffer.char_index_range_to_byte_range(params.current_selection.range())?;
        let current_selection = params.current_selection.clone();
        Ok(self
            .iter_filtered(params)?
            .skip_while(|range| range.range != byte_range)
            .take_while(|range| is_alphanumeric_word(buffer, range))
            .last()
            .and_then(|range| {
                Some(
                    current_selection
                        .set_range(buffer.byte_range_to_char_index_range(&range.range).ok()?),
                )
            }))
    }
}

fn is_alphanumeric_word(buffer: &Buffer, range: &ByteRange) -> bool {
    let Ok(char_index_range) = buffer.byte_range_to_char_index_range(&range.range) else {
        return false;
    };
    let Ok(content) = buffer.slice(&char_index_range) else {
        return false;
    };
    content
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
}

#[cfg(test)]
mod test_word {
    use crate::{buffer::Buffer, selection::Selection, selection_mode::SelectionMode};

    use super::*;

    #[test]
    fn case_1() {
        let buffer = Buffer::new(
            None,
            "snake_case camelCase PascalCase UPPER_SNAKE kebab-case ->() 123 <_>",
        );
        Token::new(&buffer).unwrap().assert_all_selections(
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
