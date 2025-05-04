use itertools::Itertools;

use crate::{
    context::{LocalSearchConfig, LocalSearchConfigMode},
    list::grep::RegexConfig,
};

fn parse_search_config(input: &str) -> anyhow::Result<LocalSearchConfig> {
    let chars = input.chars().collect_vec();
    let mode_chars = chars
        .iter()
        .take_while(|c| c.is_ascii_alphanumeric())
        .collect_vec();
    let mode_chars_count = mode_chars.len();
    let mode_str: String = mode_chars.into_iter().map(|c| c.to_string()).join("");
    let mode = {
        match mode_str.as_str() {
            "" => LocalSearchConfigMode::Regex(RegexConfig::literal()),
            "c" => LocalSearchConfigMode::Regex(RegexConfig {
                escaped: true,
                match_whole_word: false,
                case_sensitive: true,
            }),
            "w" => LocalSearchConfigMode::Regex(RegexConfig {
                escaped: true,
                match_whole_word: true,
                case_sensitive: false,
            }),
            "s" | "cw" | "wc" => LocalSearchConfigMode::Regex(RegexConfig::strict()),
            "r" => LocalSearchConfigMode::Regex(RegexConfig {
                escaped: false,
                match_whole_word: false,
                case_sensitive: false,
            }),
            "rc" => LocalSearchConfigMode::Regex(RegexConfig {
                escaped: false,
                match_whole_word: false,
                case_sensitive: true,
            }),
            "rw" => LocalSearchConfigMode::Regex(RegexConfig {
                escaped: false,
                match_whole_word: true,
                case_sensitive: false,
            }),
            "rs" | "rcw" | "rwc" => LocalSearchConfigMode::Regex(RegexConfig {
                escaped: false,
                match_whole_word: true,
                case_sensitive: true,
            }),
            "n" => LocalSearchConfigMode::NamingConventionAgnostic,
            "a" => LocalSearchConfigMode::AstGrep,
            _ => return Err(anyhow::anyhow!("{mode_str:?} is not a valid search mode.")),
        }
    };
    let separator = chars.iter().skip(mode_chars_count).next().ok_or_else(|| {
        anyhow::anyhow!("Expected a non-alphanumeric separator after {mode_str:?}")
    })?;

    let chars = chars.iter().skip(mode_chars_count + 1).collect_vec();
    let mut search = Vec::new();
    let mut replacement = Vec::new();
    let mut escaped = false;
    let mut last_index = 0;
    for i in 0..chars.len() {
        let c = chars[i];

        if c == &'\\' && chars.get(i + 1) == Some(&separator) {
            escaped = true;
        } else if c != separator || escaped {
            escaped = false;
            search.push(c.to_string())
        } else {
            last_index = i;
            break;
        }
    }
    if chars.get(last_index) == Some(&separator) {
        for i in last_index + 1..chars.len() {
            let c = chars[i];

            if c == &'\\' && chars.get(i + 1) == Some(&separator) {
                escaped = true;
            } else if c != separator || escaped {
                escaped = false;
                replacement.push(c.to_string())
            } else {
                break;
            }
        }
    }
    let search = search.join("");
    let replacement = replacement.join("");

    Ok(LocalSearchConfig::new(mode)
        .set_search(search)
        .set_replacment(replacement)
        .clone())
}

#[cfg(test)]
mod test_parse_search_config {
    use super::*;

    #[test]
    fn test_mode() {
        fn run_test(mode_str: &str, expected_mode: LocalSearchConfigMode) {
            let actual = parse_search_config(&format!("{mode_str} hello")).unwrap();
            assert_eq!(actual.mode, expected_mode);
            assert_eq!(actual.search(), "hello")
        }
        run_test("", LocalSearchConfigMode::Regex(RegexConfig::literal()));
        run_test("", LocalSearchConfigMode::Regex(RegexConfig::literal()));
        run_test(
            "c",
            LocalSearchConfigMode::Regex(RegexConfig {
                escaped: true,
                match_whole_word: false,
                case_sensitive: true,
            }),
        );
        run_test(
            "w",
            LocalSearchConfigMode::Regex(RegexConfig {
                escaped: true,
                match_whole_word: true,
                case_sensitive: false,
            }),
        );
        run_test("s", LocalSearchConfigMode::Regex(RegexConfig::strict()));
        run_test("wc", LocalSearchConfigMode::Regex(RegexConfig::strict()));
        run_test("cw", LocalSearchConfigMode::Regex(RegexConfig::strict()));
        run_test(
            "r",
            LocalSearchConfigMode::Regex(RegexConfig {
                escaped: false,
                match_whole_word: false,
                case_sensitive: false,
            }),
        );
        run_test(
            "rc",
            LocalSearchConfigMode::Regex(RegexConfig {
                escaped: false,
                match_whole_word: false,
                case_sensitive: true,
            }),
        );
        run_test(
            "rw",
            LocalSearchConfigMode::Regex(RegexConfig {
                escaped: false,
                match_whole_word: true,
                case_sensitive: false,
            }),
        );
        run_test(
            "rs",
            LocalSearchConfigMode::Regex(RegexConfig {
                escaped: false,
                match_whole_word: true,
                case_sensitive: true,
            }),
        );
        run_test(
            "rcw",
            LocalSearchConfigMode::Regex(RegexConfig {
                escaped: false,
                match_whole_word: true,
                case_sensitive: true,
            }),
        );
        run_test(
            "rwc",
            LocalSearchConfigMode::Regex(RegexConfig {
                escaped: false,
                match_whole_word: true,
                case_sensitive: true,
            }),
        );
        run_test("n", LocalSearchConfigMode::NamingConventionAgnostic);
        run_test("a", LocalSearchConfigMode::AstGrep);
    }

    #[test]
    fn space_separator() {
        let actual = parse_search_config("r hello world").unwrap();
        assert_eq!(
            actual.mode,
            LocalSearchConfigMode::Regex(RegexConfig::regex())
        );
        assert_eq!(actual.search(), "hello");
        assert_eq!(actual.replacement(), "world")
    }

    #[test]
    fn slash_separator() {
        let actual = parse_search_config("r/hello world/bye bye").unwrap();
        assert_eq!(
            actual.mode,
            LocalSearchConfigMode::Regex(RegexConfig::regex())
        );
        assert_eq!(actual.search(), "hello world");
        assert_eq!(actual.replacement(), "bye bye")
    }

    #[test]
    fn search_and_replacement_contains_escaped_separator() {
        let actual = parse_search_config(r#"r hello\ wor\ld bye\ by\e"#).unwrap();
        assert_eq!(
            actual.mode,
            LocalSearchConfigMode::Regex(RegexConfig::regex())
        );
        assert_eq!(actual.search(), r#"hello wor\ld"#);
        assert_eq!(actual.replacement(), r#"bye by\e"#)
    }
}
