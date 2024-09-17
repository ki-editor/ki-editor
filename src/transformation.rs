use convert_case::Casing;
use shared::process_command::ProcessCommand;

use crate::{clipboard::CopiedTexts, selection_mode::CaseAgnostic, soft_wrap::soft_wrap};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Transformation {
    Case(convert_case::Case),
    Join,
    Wrap,
    PipeToShell { command: String },
    ReplaceWithCopiedText { copied_texts: CopiedTexts },
    RegexReplace { regex: MyRegex, replacement: String },
    CaseAgnosticReplace { search: String, replacement: String },
}

impl std::fmt::Display for Transformation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Transformation::Case(case) => write!(
                f,
                "{}",
                format!("{:?}", case).to_case(convert_case::Case::Title)
            ),
            Transformation::Join => write!(f, "Join",),
            Transformation::Wrap => write!(f, "Wrap",),
            Transformation::PipeToShell { command } => write!(f, "Pipe To Shell `{command}`",),
            Transformation::ReplaceWithCopiedText { .. } => {
                write!(f, "Replace With Copied Text",)
            }
            Transformation::RegexReplace { regex, replacement } => {
                write!(
                    f,
                    "Regex Replace /{}/ with /{replacement}/",
                    regex.0.as_str(),
                )
            }
            Transformation::CaseAgnosticReplace {
                search,
                replacement,
            } => write!(f, "Case-agnostic Replace `{search}` with `{replacement}`",),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct MyRegex(pub(crate) regex::Regex);

impl PartialEq for MyRegex {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_str() == other.0.as_str()
    }
}

impl Eq for MyRegex {}

impl Transformation {
    pub(crate) fn apply(&self, selection_index: usize, string: String) -> anyhow::Result<String> {
        match self {
            Transformation::Case(case) => Ok(string.to_case(*case)),
            Transformation::Join => Ok(regex::Regex::new(r"\s*\n+\s*")
                .unwrap()
                .replace_all(&string, " ")
                .to_string()),
            Transformation::Wrap => Ok(soft_wrap(&string, 80).to_string()),
            Transformation::PipeToShell { command } => {
                ProcessCommand::new("bash", &["-c", command]).run_with_input(&string)
            }
            Transformation::ReplaceWithCopiedText { copied_texts } => {
                Ok(copied_texts.get(selection_index))
            }
            Transformation::RegexReplace { regex, replacement } => {
                Ok(regex.0.replace(&string, replacement).to_string())
            }
            Transformation::CaseAgnosticReplace {
                search,
                replacement,
            } => CaseAgnostic::replace(&string, search, replacement),
        }
    }
}

#[cfg(test)]
mod test_transformation {
    use super::Transformation;

    #[test]
    fn join() {
        let result = Transformation::Join
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
        assert_eq!(result, "who lives in a pineapple under the sea? Spongebob Squarepants! absorbent and \nyellow and porous is he? Spongebob Squarepants")
    }
}
