use convert_case::Casing;
use shared::process_command::ProcessCommand;

use crate::{clipboard::CopiedTexts, soft_wrap::soft_wrap};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Transformation {
    Case(convert_case::Case),
    Join,
    Wrap,
    PipeToShell { command: String },
    ReplaceWithCopiedText { copied_texts: CopiedTexts },
}
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
                ProcessCommand::new("bash", &["-c", &command]).run_with_input(&string)
            }
            Transformation::ReplaceWithCopiedText { copied_texts } => {
                Ok(copied_texts.get(selection_index))
            }
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
