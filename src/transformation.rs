use convert_case::Casing;
use itertools::Itertools;
use shared::process_command::ProcessCommand;

use crate::{clipboard::Texts, selection_mode::NamingConventionAgnostic, soft_wrap::soft_wrap};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Transformation {
    Case(convert_case::Case),
    Unwrap,
    Wrap,
    PipeToShell { command: String },
    ReplaceWithCopiedText { copied_texts: Texts },
    RegexReplace { regex: MyRegex, replacement: String },
    NamingConventionAgnosticReplace { search: String, replacement: String },
    ToggleLineComment { prefix: String },
    ToggleBlockComment { open: String, close: String },
}

impl std::fmt::Display for Transformation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Transformation::Case(case) => write!(
                f,
                "{}",
                format!("{case:?}").to_case(convert_case::Case::Title)
            ),
            Transformation::Unwrap => write!(f, "Unwrap",),
            Transformation::Wrap => write!(f, "Wrap",),
            Transformation::PipeToShell { command } => write!(f, "Pipe To Shell `{command}`",),
            Transformation::ReplaceWithCopiedText { .. } => {
                write!(f, "Replace With Copied Text",)
            }
            Transformation::RegexReplace { regex, replacement } => {
                write!(
                    f,
                    "Regex: Replace /{}/ with /{replacement}/",
                    regex.0.as_str(),
                )
            }
            Transformation::NamingConventionAgnosticReplace {
                search,
                replacement,
            } => write!(
                f,
                "Naming convention-Agnostic: Replace `{search}` with `{replacement}`",
            ),
            Transformation::ToggleLineComment { prefix } => {
                write!(f, "Toggle Line Comment `{prefix}`")
            }
            Transformation::ToggleBlockComment { open, close } => {
                write!(f, "Toggle Block Comment `{open} {close}`")
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct MyRegex(pub fancy_regex::Regex);

impl PartialEq for MyRegex {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_str() == other.0.as_str()
    }
}

impl Eq for MyRegex {}

impl Transformation {
    pub fn apply(&self, selection_index: usize, string: String) -> anyhow::Result<String> {
        match self {
            Transformation::Case(case) => Ok(string.to_case(*case)),
            Transformation::Unwrap => Ok(regex::Regex::new(r"\s*\n+\s*")
                .unwrap()
                .replace_all(&string, " ")
                .to_string()),
            Transformation::Wrap => Ok({
                let result = soft_wrap(&string, 80)
                    .to_string()
                    .lines()
                    .map(|line| line.trim_end())
                    .join("\n");

                debug_assert!(result.lines().all(|line| !line
                    .chars()
                    .last()
                    .unwrap()
                    .is_whitespace()));

                result
            }),
            Transformation::PipeToShell { command } => {
                ProcessCommand::new("bash", ["-c".to_string(), command.to_string()].as_ref())
                    .run_with_input(&string)
            }
            Transformation::ReplaceWithCopiedText { copied_texts } => {
                Ok(copied_texts.get(selection_index))
            }
            Transformation::RegexReplace { regex, replacement } => {
                Ok(regex.0.replace(&string, replacement).to_string())
            }
            Transformation::NamingConventionAgnosticReplace {
                search,
                replacement,
            } => NamingConventionAgnostic::replace(&string, search, replacement),
            Transformation::ToggleLineComment { prefix } => Ok(if string.starts_with(prefix) {
                string.trim_start_matches(prefix).trim_start().to_string()
            } else {
                format!("{prefix} {string}")
            }),
            Transformation::ToggleBlockComment { open, close } => {
                Ok(if string.starts_with(open) && string.ends_with(close) {
                    string
                        .trim_start_matches(open)
                        .trim_end_matches(close)
                        .trim()
                        .to_string()
                } else {
                    format!("{open} {string} {close}")
                })
            }
        }
    }
}

#[cfg(test)]
mod test_transformation {
    use super::Transformation;

    #[test]
    fn unwrap() {
        let result = Transformation::Unwrap
            .apply(
                0,
                "
who 
  lives
    in 
      a

pineapple?
"
                .trim()
                .to_string(),
            )
            .unwrap();
        assert_eq!(result, "who lives in a pineapple?")
    }

    #[test]
    fn wrap() {
        let result = Transformation::Wrap
            .apply(0,"
who lives in a pineapple under the sea? Spongebob Squarepants! absorbent and yellow and porous is he? Spongebob Squarepants
"
            .trim().to_string()).unwrap();
        assert_eq!(result, "who lives in a pineapple under the sea? Spongebob Squarepants! absorbent and\nyellow and porous is he? Spongebob Squarepants")
    }

    #[test]
    fn toggle_line_comment() {
        let transformation = Transformation::ToggleLineComment {
            prefix: "//".to_string(),
        };
        assert_eq!(
            transformation.apply(0, "hello".to_string()).unwrap(),
            "// hello"
        );
        assert_eq!(
            transformation.apply(0, "// hello".to_string()).unwrap(),
            "hello"
        );
    }
    #[test]
    fn toggle_block_comment() {
        let transformation = Transformation::ToggleBlockComment {
            open: "/*".to_string(),
            close: "*/".to_string(),
        };
        assert_eq!(
            transformation.apply(0, "hello".to_string()).unwrap(),
            "/* hello */"
        );
        assert_eq!(
            transformation.apply(0, "/* hello */".to_string()).unwrap(),
            "hello"
        );
    }
}
