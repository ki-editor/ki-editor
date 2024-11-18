use super::{ByteRange, SelectionMode, Token};
use crate::buffer::Buffer;

pub struct Word {
    normal_regex: super::Regex,
    symbol_skipping_regex: super::Regex,
}

const SUBWORD_REGEX: &str =
    r"[A-Z]{2,}(?=[A-Z][a-z])|[A-Z][a-z]+|[A-Z]{2,}|[a-z]+|[^\w\s]|_|[0-9]+";

const SUBWORD_SYMBOL_SKIPPING_REGEX: &str =
    r"[A-Z]{2,}(?=[A-Z][a-z])|[A-Z][a-z]+|[A-Z]{2,}|[a-z]+|[0-9]+";

impl Word {
    pub(crate) fn new(buffer: &Buffer) -> anyhow::Result<Self> {
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
        })
    }
}

impl SelectionMode for Word {
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
        get_subword(params, SelectionPosition::First)
    }

    fn last(
        &self,
        params: super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        get_subword(params, SelectionPosition::Last)
    }
}

#[cfg(test)]
mod test_subword {
    use super::*;
    use crate::{buffer::Buffer, selection::Selection, selection_mode::SelectionMode};

    #[test]
    fn case_1() {
        let buffer = Buffer::new(
            None,
            "snake_case camelCase PascalCase UPPER_SNAKE ->() 123 <_> HTTPNetwork",
        );
        Word::new(&buffer).unwrap().assert_all_selections(
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
            ],
        );
    }

    #[test]
    fn case_2() {
        let buffer = Buffer::new(None, "XMLParser JSONObject HTMLElement");
        Word::new(&buffer).unwrap().assert_all_selections(
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
enum SelectionPosition {
    First,
    Last,
}

fn get_subword(
    params: super::SelectionModeParams,
    position: SelectionPosition,
) -> anyhow::Result<Option<crate::selection::Selection>> {
    if let Some(current_word) = Token::new(params.buffer)?.current(
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
