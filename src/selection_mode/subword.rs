use crate::buffer::Buffer;

use super::{ByteRange, SelectionMode};

pub struct Subword {
    normal_regex: super::Regex,
    symbol_skipping_regex: super::Regex,
}

impl Subword {
    pub(crate) fn new(buffer: &Buffer) -> anyhow::Result<Self> {
        Ok(Self {
            normal_regex: super::Regex::from_config(
                buffer,
                r"((([a-z]+)|(([A-Z]{2,})+)|([A-Z][a-z]*)))|([^\w\s]|_)|[0-9]+",
                crate::list::grep::RegexConfig {
                    escaped: false,
                    case_sensitive: true,
                    match_whole_word: false,
                },
            )?,
            symbol_skipping_regex: super::Regex::from_config(
                buffer,
                r"((([a-z]+)|(([A-Z]{2,})+)|([A-Z][a-z]*)))|[0-9]+",
                crate::list::grep::RegexConfig {
                    escaped: false,
                    case_sensitive: true,
                    match_whole_word: false,
                },
            )?,
        })
    }
}

impl SelectionMode for Subword {
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
}

#[cfg(test)]
mod test_subword {
    use crate::{buffer::Buffer, selection::Selection, selection_mode::SelectionMode};

    use super::*;

    #[test]
    fn case_1() {
        let buffer = Buffer::new(
            None,
            "snake_case camelCase PascalCase UPPER_SNAKE ->() 123 <_>",
        );
        Subword::new(&buffer).unwrap().assert_all_selections(
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
            ],
        );
    }
}
