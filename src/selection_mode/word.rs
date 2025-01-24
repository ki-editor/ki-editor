use super::{ByteRange, SelectionMode, Token};
use crate::buffer::Buffer;

pub struct Word {
    normal_regex: super::Regex,
    symbol_skipping_regex: super::Regex,
    skip_symbols: bool,
}

const SUBWORD_REGEX: &str =
    r"[A-Z]{2,}(?=[A-Z][a-z])|[A-Z]{2,}|[A-Z][a-z]+|[A-Z]|[a-z]+|[^\w\s]|_|[0-9]+";

const SUBWORD_SYMBOL_SKIPPING_REGEX: &str =
    r"[A-Z]{2,}(?=[A-Z][a-z])|[A-Z]{2,}|[A-Z][a-z]+|[A-Z]|[a-z]+|[0-9]+";

impl Word {
    pub(crate) fn new(buffer: &Buffer, skip_symbols: bool) -> anyhow::Result<Self> {
        Ok(Self {
            normal_regex: super::Regex::from_config(
                buffer,
                SUBWORD_REGEX,
                crate::list::grep::RegexConfig {
                    escaped: false,
                    case_sensitive: true,
                    match_whole_word: false,
                },
            )?,
            symbol_skipping_regex: super::Regex::from_config(
                buffer,
                SUBWORD_SYMBOL_SKIPPING_REGEX,
                crate::list::grep::RegexConfig {
                    escaped: false,
                    case_sensitive: true,
                    match_whole_word: false,
                },
            )?,
            skip_symbols,
        })
    }
}

impl SelectionMode for Word {
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
        get_word(params, SelectionPosition::First, self.skip_symbols)
    }

    fn last(
        &self,
        params: super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        get_word(params, SelectionPosition::Last, self.skip_symbols)
    }
}

#[cfg(test)]
mod test_word {
    use super::*;
    use crate::{buffer::Buffer, selection::Selection, selection_mode::SelectionMode};

    #[test]
    fn skip_symbols() {
        let buffer = Buffer::new(
            None,
            "snake_case camelCase PascalCase UPPER_SNAKE ->() 123 <_> HTTPNetwork X",
        );
        Word::new(&buffer, true).unwrap().assert_all_selections(
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
        Word::new(&buffer, false).unwrap().assert_all_selections(
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
        Word::new(&buffer, true).unwrap().assert_all_selections(
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
    params: super::SelectionModeParams,
    position: SelectionPosition,
    skip_symbols: bool,
) -> anyhow::Result<Option<crate::selection::Selection>> {
    if let Some(current_word) = Token::new(params.buffer, skip_symbols)?.current(
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
