mod javascript;
mod javascript_react;
pub mod rust;
mod typescript;
mod typescript_react;

use grammar::grammar::GrammarConfiguration;
use serde_json::Value;

pub use crate::process_command::ProcessCommand;
use crate::{canonicalized_path::CanonicalizedPath, lsp::formatter::Formatter};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LanguageId(&'static str);

impl std::fmt::Display for LanguageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl LanguageId {
    pub fn new(id: &'static str) -> Self {
        Self(id)
    }
}

pub trait Language: dyn_clone::DynClone + std::fmt::Debug + Send + Sync {
    /// For example, "rs" for Rust, "cpp" for C++.
    fn extension(&self) -> &'static str;
    fn lsp_process_command(&self) -> Option<ProcessCommand>;

    /// Refer https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocumentItem
    fn id(&self) -> LanguageId;

    fn tree_sitter_grammar_config(&self) -> Option<GrammarConfiguration> {
        None
    }

    fn tree_sitter_language(&self) -> Option<tree_sitter::Language> {
        grammar::grammar::get_language(&self.tree_sitter_grammar_config()?.grammar_id).ok()
    }

    /// Used for tree-sitter syntax highlighting.
    fn highlight_query(&self) -> Option<&'static str> {
        None
    }

    /// Used for tree-sitter language injection.
    fn injection_query(&self) -> Option<&'static str> {
        None
    }

    /// Used for tree-sitter locals.
    fn locals_query(&self) -> Option<&'static str> {
        None
    }

    fn formatter_command(&self) -> Option<(ProcessCommand, FormatterTestCase)>;

    fn formatter(&self) -> Option<Formatter> {
        self.formatter_command()
            .map(|(command, _)| Formatter::from(command))
    }
    fn initialization_options(&self) -> Option<Value> {
        None
    }
}

dyn_clone::clone_trait_object!(Language);

pub fn languages() -> Vec<Box<dyn Language>> {
    use self::*;
    vec![
        Box::new(rust::Rust),
        Box::new(typescript::Typescript),
        Box::new(typescript_react::TypescriptReact),
        Box::new(javascript::Javascript),
        Box::new(javascript_react::JavascriptReact),
    ]
}

pub fn from_path(path: &CanonicalizedPath) -> Option<Box<dyn Language>> {
    path.extension()
        .map(|extension| {
            languages()
                .into_iter()
                .find(|language| language.extension().eq(extension))
        })
        .unwrap_or_default()
}

pub struct FormatterTestCase {
    /// The unformatted input.
    pub input: &'static str,

    /// The formatted output.
    pub expected: &'static str,
}

#[cfg(test)]
mod test_language {
    use crate::lsp::formatter::Formatter;

    use super::languages;

    #[test]
    fn test_formatter() {
        for language in languages() {
            if let Some((formatter_command, test_case)) = language.formatter_command() {
                let actual = Formatter::from(formatter_command)
                    .format(test_case.input)
                    .unwrap();

                assert_eq!(actual, test_case.expected, "language: {}", language.id());
            }
        }
    }
}
