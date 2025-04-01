use super::{get_current_selection_by_cursor_via_iter, ByteRange, SelectionMode};
use crate::components::editor::IfCurrentNotFound;
use crate::{buffer::Buffer, list::grep::RegexConfig};
use anyhow::Result;
use itertools::Itertools;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub(crate) struct Regex {
    regex: fancy_regex::Regex,
    content: String,
}

// Define a struct to use as the cache key
#[derive(Hash, Eq, PartialEq, Clone)]
struct RegexCacheKey {
    pattern: String,
    escaped: bool,
    match_whole_word: bool,
    case_sensitive: bool,
}

// Create a global cache using Lazy
static REGEX_CACHE: Lazy<Arc<Mutex<HashMap<RegexCacheKey, fancy_regex::Regex>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

pub(crate) fn get_regex(pattern: &str, config: RegexConfig) -> Result<fancy_regex::Regex> {
    let key = RegexCacheKey {
        pattern: pattern.to_string(),
        escaped: config.escaped,
        match_whole_word: config.match_whole_word,
        case_sensitive: config.case_sensitive,
    };

    // Try to get from cache first
    {
        let cache = REGEX_CACHE.lock().unwrap();
        if let Some(regex) = cache.get(&key) {
            return Ok((*regex).clone());
        }
    }

    // If not in cache, create the regex
    let pattern = if config.escaped {
        regex::escape(pattern)
    } else {
        pattern.to_string()
    };
    let pattern = if config.match_whole_word {
        format!("\\b{}\\b", pattern)
    } else {
        pattern
    };
    let pattern = if config.case_sensitive {
        pattern
    } else {
        format!("(?i){}", pattern)
    };
    let pattern = format!("(?m){}", pattern);

    let regex = fancy_regex::Regex::new(&pattern)?;

    // Store in cache
    {
        let mut cache = REGEX_CACHE.lock().unwrap();
        cache.insert(key, regex.clone());
    }

    Ok(regex)
}

impl Regex {
    pub(crate) fn from_config(
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

impl SelectionMode for Regex {
    fn get_current_selection_by_cursor(
        &self,
        buffer: &crate::buffer::Buffer,
        cursor_char_index: crate::selection::CharIndex,
        if_current_not_found: IfCurrentNotFound,
    ) -> anyhow::Result<Option<super::ByteRange>> {
        get_current_selection_by_cursor_via_iter(
            buffer,
            cursor_char_index,
            if_current_not_found,
            Rc::new(
                self.regex
                    .find_iter(&self.content)
                    .filter_map(|match_| Some(ByteRange::new(match_.ok()?.range())))
                    .collect(),
            ),
        )
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
