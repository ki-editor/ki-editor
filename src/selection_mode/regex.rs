use crate::{buffer::Buffer, list::grep::RegexConfig};

use super::{ByteRange, IterBasedSelectionMode};

pub struct Regex {
    regex: fancy_regex::Regex,
    content: String,
}

/// BOTTLENECK 3
pub fn get_regex(pattern: &str, config: RegexConfig) -> anyhow::Result<fancy_regex::Regex> {
    let pattern = if config.escaped {
        regex::escape(pattern)
    } else {
        pattern.to_string()
    };
    let pattern = if config.match_whole_word {
        format!("\\b{pattern}\\b")
    } else {
        pattern
    };
    let pattern = if config.case_sensitive {
        pattern
    } else {
        format!("(?i){pattern}")
    };
    let pattern = format!("(?m){pattern}");
    Ok(fancy_regex::Regex::new(&pattern)?)
}

impl Regex {
    pub fn from_config(
        buffer: &Buffer,
        pattern: &str,
        config: RegexConfig,
    ) -> anyhow::Result<Self> {
        let regex = get_regex(pattern, config)?;
        Ok(Self {
            regex,
            content: buffer.rope().to_string(),
        })
    }
}

impl IterBasedSelectionMode for Regex {
    fn iter<'a>(
        &'a self,
        _params: &super::SelectionModeParams<'a>,
    ) -> anyhow::Result<Box<dyn Iterator<Item = ByteRange> + 'a>> {
        let matches = self.regex.find_iter(&self.content);
        Ok(Box::new(matches.filter_map(move |matches| {
            let matches = matches.ok()?;
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
        let buffer = Buffer::new(None, "fn main() { let x = m.in; }");
        crate::selection_mode::Regex::from_config(
            &buffer,
            "m.in",
            RegexConfig {
                escaped: true,
                case_sensitive: false,
                match_whole_word: false,
            },
        )
        .unwrap()
        .assert_all_selections(&buffer, Selection::default(), &[(20..24, "m.in")]);
    }

    #[test]
    fn unescaped() {
        let buffer = Buffer::new(None, "fn main() { let x = m.in; }");
        crate::selection_mode::Regex::from_config(
            &buffer,
            "m.in",
            RegexConfig {
                escaped: false,
                case_sensitive: false,
                match_whole_word: false,
            },
        )
        .unwrap()
        .assert_all_selections(
            &buffer,
            Selection::default(),
            &[(3..7, "main"), (20..24, "m.in")],
        );
    }

    #[test]
    fn ignore_case() {
        let buffer = Buffer::new(None, "fn Main() { let x = m.in; }");
        crate::selection_mode::Regex::from_config(
            &buffer,
            "m.in",
            RegexConfig {
                escaped: false,
                case_sensitive: false,
                match_whole_word: false,
            },
        )
        .unwrap()
        .assert_all_selections(
            &buffer,
            Selection::default(),
            &[(3..7, "Main"), (20..24, "m.in")],
        );
    }

    #[test]
    fn match_whole_word() {
        let buffer = Buffer::new(None, "fn Main() { let x = main_war; }");
        crate::selection_mode::Regex::from_config(
            &buffer,
            "m.in",
            RegexConfig {
                escaped: false,
                case_sensitive: false,
                match_whole_word: true,
            },
        )
        .unwrap()
        .assert_all_selections(&buffer, Selection::default(), &[(3..7, "Main")]);
    }

    #[test]
    fn multiline_mode_enabled_by_default() {
        let buffer = Buffer::new(
            None,
            "
- [ ] a
- [ ] b
  - [ ]  c
- [ ] d
",
        );
        crate::selection_mode::Regex::from_config(
            &buffer,
            r"^- \[ \](.*)$",
            RegexConfig {
                escaped: false,
                case_sensitive: false,
                match_whole_word: false,
            },
        )
        .unwrap()
        .assert_all_selections(
            &buffer,
            Selection::default(),
            &[(1..8, "- [ ] a"), (9..16, "- [ ] b"), (28..35, "- [ ] d")],
        );
    }
}
