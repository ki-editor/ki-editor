use crate::buffer::Buffer;

pub struct WordLong;

impl WordLong {
    pub fn as_regex(buffer: &Buffer) -> anyhow::Result<super::Regex> {
        super::Regex::from_config(
            buffer,
            r"((\w|-)+)|([^a-zA-Z\d\s])",
            crate::list::grep::RegexConfig {
                escaped: false,
                case_sensitive: false,
                match_whole_word: false,
            },
        )
    }
}

#[cfg(test)]
mod test_word_long {
    use crate::{buffer::Buffer, selection::Selection, selection_mode::SelectionMode};

    use super::*;

    #[test]
    fn case_1() {
        let buffer = Buffer::new(
            tree_sitter_rust::language(),
            "snake_case camelCase PascalCase UPPER_SNAKE kebab-case ->() 123 <_>",
        );
        WordLong::as_regex(&buffer).unwrap().assert_all_selections(
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
