use globset::Glob;
use itertools::Itertools;

use crate::{
    context::{GlobalSearchConfig, LocalSearchConfig, LocalSearchConfigMode},
    list::grep::RegexConfig,
};

pub(crate) fn parse_search_config(input: &str) -> anyhow::Result<GlobalSearchConfig> {
    let default = || {
        Ok(GlobalSearchConfig {
            include_glob: None,
            exclude_glob: None,
            local_config: LocalSearchConfig::default()
                .set_search(input.to_string())
                .clone(),
        })
    };
    let chars = input.chars().collect_vec();
    let mode_chars = chars
        .iter()
        .take_while(|c| c.is_ascii_alphanumeric() || c == &&'\\')
        .collect_vec();
    let mode_chars_count = mode_chars.len();
    let mode_str: String = mode_chars.into_iter().map(|c| c.to_string()).join("");
    let mode = {
        match mode_str.as_str() {
            "" => LocalSearchConfigMode::Regex(RegexConfig::literal()),
            "c" => LocalSearchConfigMode::Regex(RegexConfig::case_sensitive()),
            "w" => LocalSearchConfigMode::Regex(RegexConfig::match_whole_word()),
            "s" | "cw" | "wc" => LocalSearchConfigMode::Regex(RegexConfig::strict()),
            "r" => LocalSearchConfigMode::Regex(RegexConfig::regex()),
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
            _ => return default(),
        }
    };
    let Some(separator) = chars.iter().skip(mode_chars_count).next() else {
        return default();
    };

    let chars = chars.iter().skip(mode_chars_count + 1).collect_vec();
    let parse_component = |start_index: usize| {
        let mut escaped = false;
        let mut last_index = 0;
        let mut result = Vec::new();
        for i in start_index..chars.len() {
            let c = chars[i];

            if c == &'\\' && chars.get(i + 1) == Some(&separator) {
                escaped = true;
            } else if c != separator || escaped {
                escaped = false;
                result.push(c.to_string())
            } else {
                last_index = i;
                break;
            }
        }
        (last_index, result.join(""))
    };
    let (last_index, search) = parse_component(0);
    let parse_next_component = |last_index: usize| {
        if chars.get(last_index) == Some(&separator) {
            let (last_index, result) = parse_component(last_index + 1);
            (last_index, result)
        } else {
            (last_index, "".to_string())
        }
    };
    let (last_index, replacement) = parse_next_component(last_index);
    let (last_index, include_glob) = parse_next_component(last_index);
    let (_, exclude_glob) = parse_next_component(last_index);

    let make_glob = |input: &str| {
        if input.is_empty() {
            Ok(None)
        } else {
            Some(Glob::new(&input)).transpose()
        }
    };
    Ok(GlobalSearchConfig {
        include_glob: make_glob(&include_glob)?,
        exclude_glob: make_glob(&exclude_glob)?,
        local_config: LocalSearchConfig::new(mode)
            .set_search(search)
            .set_replacment(replacement)
            .clone(),
    })
}

#[cfg(test)]
mod test_parse_search_config {
    use super::*;
    use LocalSearchConfigMode::*;

    #[test]
    fn test_mode() {
        fn run_test(mode_str: &str, expected_mode: LocalSearchConfigMode) {
            let actual = parse_search_config(&format!("{mode_str} hello"))
                .unwrap()
                .local_config;
            assert_eq!(actual.mode, expected_mode);
            assert_eq!(actual.search(), "hello")
        }
        run_test("", Regex(RegexConfig::literal()));
        run_test("", Regex(RegexConfig::literal()));
        run_test("c", Regex(RegexConfig::case_sensitive()));
        run_test("w", Regex(RegexConfig::match_whole_word()));
        run_test("s", Regex(RegexConfig::strict()));
        run_test("wc", Regex(RegexConfig::strict()));
        run_test("cw", Regex(RegexConfig::strict()));
        run_test(
            "r",
            Regex(RegexConfig {
                escaped: false,
                match_whole_word: false,
                case_sensitive: false,
            }),
        );
        run_test(
            "rc",
            Regex(RegexConfig {
                escaped: false,
                match_whole_word: false,
                case_sensitive: true,
            }),
        );
        run_test(
            "rw",
            Regex(RegexConfig {
                escaped: false,
                match_whole_word: true,
                case_sensitive: false,
            }),
        );
        run_test(
            "rs",
            Regex(RegexConfig {
                escaped: false,
                match_whole_word: true,
                case_sensitive: true,
            }),
        );
        run_test(
            "rcw",
            Regex(RegexConfig {
                escaped: false,
                match_whole_word: true,
                case_sensitive: true,
            }),
        );
        run_test(
            "rwc",
            Regex(RegexConfig {
                escaped: false,
                match_whole_word: true,
                case_sensitive: true,
            }),
        );
        run_test("n", NamingConventionAgnostic);
        run_test("a", AstGrep);
    }

    #[test]
    fn space_separator() {
        let actual = parse_search_config("r hello world").unwrap().local_config;
        assert_eq!(actual.mode, Regex(RegexConfig::regex()));
        assert_eq!(actual.search(), "hello");
        assert_eq!(actual.replacement(), "world")
    }

    #[test]
    fn slash_separator() {
        let actual = parse_search_config("r/hello world/bye bye")
            .unwrap()
            .local_config;
        assert_eq!(actual.mode, Regex(RegexConfig::regex()));
        assert_eq!(actual.search(), "hello world");
        assert_eq!(actual.replacement(), "bye bye")
    }

    #[test]
    fn search_and_replacement_contains_escaped_separator() {
        let actual = parse_search_config(r#"r hello\ wor\ld bye\ by\e"#)
            .unwrap()
            .local_config;
        assert_eq!(actual.mode, Regex(RegexConfig::regex()));
        assert_eq!(actual.search(), r#"hello wor\ld"#);
        assert_eq!(actual.replacement(), r#"bye by\e"#)
    }

    #[test]
    fn use_default_if_cannot_parse_mode() {
        let actual = parse_search_config("hello_world").unwrap().local_config;
        assert_eq!(actual.mode, Regex(RegexConfig::literal()));
        assert_eq!(actual.search(), "hello_world");
        assert_eq!(actual.replacement(), "")
    }

    #[test]
    fn backslash_cannot_be_treated_as_separator() {
        let actual = parse_search_config(r#"w\hello\world"#)
            .unwrap()
            .local_config;
        assert_eq!(actual.mode, Regex(RegexConfig::literal()));
        assert_eq!(actual.search(), r#"w\hello\world"#);
        assert_eq!(actual.replacement(), "")
    }

    #[test]
    fn include_glob_exclude_glob() {
        let actual = parse_search_config("a/search/replacement/*.include/*.exclude").unwrap();
        assert_eq!(actual.include_glob, Some(Glob::new("*.include").unwrap()));
        assert_eq!(actual.exclude_glob, Some(Glob::new("*.exclude").unwrap()));
    }
}
