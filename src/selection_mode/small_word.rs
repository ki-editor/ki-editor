use crate::buffer::Buffer;

pub struct SmallWord;

impl SmallWord {
    pub fn new(buffer: &Buffer) -> anyhow::Result<super::Regex> {
        super::Regex::new(
            buffer,
            r"((([a-z]+)|(([A-Z]{2,})+)|([A-Z][a-z]*))_*)|([^\w\s]|_)|[0-9]+",
            crate::list::grep::RegexConfig {
                escaped: false,
                case_sensitive: true,
                match_whole_word: false,
            },
        )
    }
}

#[cfg(test)]
mod test_small_word {
    use crate::{buffer::Buffer, selection::Selection, selection_mode::SelectionMode};

    use super::*;

    #[test]
    fn case_1() {
        let buffer = Buffer::new(
            tree_sitter_rust::language(),
            "snake_case camelCase PascalCase UPPER_SNAKE ->() 123 <_>",
        );
        SmallWord::new(&buffer).unwrap().assert_all_selections(
            &buffer,
            Selection::default(),
            &[
                (0..6, "snake_"),
                (6..10, "case"),
                (11..16, "camel"),
                (16..20, "Case"),
                (21..27, "Pascal"),
                (27..31, "Case"),
                (32..38, "UPPER_"),
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
