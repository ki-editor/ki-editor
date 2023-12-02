use crate::buffer::Buffer;

use super::{ByteRange, SelectionMode};

pub struct Regex {
    regex: regex::Regex,
    content: String,
}
pub fn get_regex(
    pattern: &str,
    escape: bool,
    case_sensitive: bool,
) -> anyhow::Result<regex::Regex> {
    let pattern = if escape {
        regex::escape(pattern)
    } else {
        pattern.to_string()
    };
    let pattern = if case_sensitive {
        pattern
    } else {
        format!("(?i){}", pattern)
    };
    Ok(regex::Regex::new(&pattern)?)
}

impl Regex {
    pub fn new(
        buffer: &Buffer,
        pattern: &str,
        escape: bool,
        case_sensitive: bool,
    ) -> anyhow::Result<Self> {
        let regex = get_regex(pattern, escape, case_sensitive)?;
        Ok(Self {
            regex,
            content: buffer.rope().to_string(),
        })
    }

    pub fn regex(buffer: &Buffer, pattern: &str) -> anyhow::Result<Self> {
        let regex = get_regex(pattern, false, false)?;
        Ok(Self {
            regex,
            content: buffer.rope().to_string(),
        })
    }
}

impl SelectionMode for Regex {
    fn name(&self) -> &'static str {
        "REGEX"
    }
    fn iter<'a>(
        &'a self,
        _params: super::SelectionModeParams<'a>,
    ) -> anyhow::Result<Box<dyn Iterator<Item = ByteRange> + 'a>> {
        let matches = self.regex.find_iter(&self.content);
        Ok(Box::new(matches.filter_map(move |matches| {
            Some(ByteRange::new(matches.start()..matches.end()))
        })))
    }
}

#[cfg(test)]
mod test_regex {
    use crate::{buffer::Buffer, selection::Selection};

    use super::*;

    #[test]
    fn escaped() {
        let buffer = Buffer::new(tree_sitter_rust::language(), "fn main() { let x = m.in; }");
        crate::selection_mode::Regex::new(&buffer, "m.in", true, false)
            .unwrap()
            .assert_all_selections(&buffer, Selection::default(), &[(20..24, "m.in")]);
    }

    #[test]
    fn unescaped() {
        let buffer = Buffer::new(tree_sitter_rust::language(), "fn main() { let x = m.in; }");
        crate::selection_mode::Regex::new(&buffer, "m.in", false, false)
            .unwrap()
            .assert_all_selections(
                &buffer,
                Selection::default(),
                &[(3..7, "main"), (20..24, "m.in")],
            );
    }

    #[test]
    fn ignore_case() {
        let buffer = Buffer::new(tree_sitter_rust::language(), "fn Main() { let x = m.in; }");
        crate::selection_mode::Regex::new(&buffer, "m.in", false, false)
            .unwrap()
            .assert_all_selections(
                &buffer,
                Selection::default(),
                &[(3..7, "Main"), (20..24, "m.in")],
            );
    }
}
