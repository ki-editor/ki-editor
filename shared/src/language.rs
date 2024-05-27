use grammar::grammar::GrammarConfiguration;
use serde_json::Value;

pub(crate) use crate::process_command::ProcessCommand;
use crate::{
    canonicalized_path::CanonicalizedPath, formatter::Formatter,
    ts_highlight_query::get_highlight_query,
};

pub(crate) use crate::languages::LANGUAGES;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
/// As defined by the LSP protocol.
/// See sections below https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#range
pub struct LanguageId(&'static str);

impl std::fmt::Display for LanguageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl LanguageId {
    pub const fn new(id: &'static str) -> Self {
        Self(id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command(pub &'static str, pub &'static [&'static str]);
impl Command {
    pub const fn default() -> Command {
        Command("", &[])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Language {
    pub(crate) extensions: &'static [&'static str],
    pub(crate) file_names: &'static [&'static str],
    pub(crate) lsp_language_id: Option<LanguageId>,
    pub(crate) lsp_command: Option<LspCommand>,
    pub(crate) tree_sitter_grammar_config: Option<GrammarConfig>,
    pub(crate) highlight_query: Option<&'static str>,
    pub(crate) formatter_command: Option<Command>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspCommand {
    pub(crate) command: Command,
    pub(crate) initialization_options: Option<&'static str>,
}
impl LspCommand {
    pub const fn default() -> LspCommand {
        LspCommand {
            command: Command::default(),
            initialization_options: None,
        }
    }
}

impl Language {
    pub const fn new() -> Self {
        Self {
            extensions: &[""],
            file_names: &[""],
            lsp_language_id: None,
            highlight_query: None,
            lsp_command: None,
            tree_sitter_grammar_config: None,
            formatter_command: None,
        }
    }

    fn file_names(&self) -> &'static [&'static str] {
        self.file_names
    }
}

impl Default for Language {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrammarConfig {
    pub(crate) id: &'static str,
    pub(crate) url: &'static str,
    pub(crate) commit: &'static str,
    pub(crate) subpath: Option<&'static str>,
}

impl Language {
    fn extensions(&self) -> &'static [&'static str] {
        self.extensions
    }

    pub fn initialization_options(&self) -> Option<Value> {
        serde_json::from_str(self.lsp_command.clone()?.initialization_options?).ok()
    }

    pub fn tree_sitter_language(&self) -> Option<tree_sitter::Language> {
        grammar::grammar::get_language(&self.tree_sitter_grammar_config()?.grammar_id).ok()
    }

    pub(crate) fn tree_sitter_grammar_config(&self) -> Option<GrammarConfiguration> {
        self.tree_sitter_grammar_config.as_ref().map(|config| {
            GrammarConfiguration::remote(config.id, config.url, config.commit, config.subpath)
        })
    }

    pub fn highlight_query(&self) -> Option<String> {
        // Get highlight query from `nvim-treesitter` first
        get_highlight_query(self.tree_sitter_grammar_config.clone()?.id)
            .ok()
            .map(|result| result.query)
            .or(
                // Otherwise, get from the default highlight queries defined in the grammar repo
                grammar::grammar::load_runtime_file(
                    &self.tree_sitter_grammar_config()?.grammar_id,
                    "highlights.scm",
                )
                .ok(),
            )
            .map(|query| {
                query
                    // Replace `nvim-treesitter`-specific predicates with builtin predicates supported by `tree-sitter-highlight` crate
                    // Reference: https://github.com/nvim-treesitter/nvim-treesitter/blob/23ba63028c6acca29be6462c0a291fc4a1b9eae8/CONTRIBUTING.md#predicates
                    .replace("lua-match", "match")
                    .replace("vim-match", "match")
                    // Remove non-highlight captures, as they are not handled by this editor
                    // See https://github.com/nvim-treesitter/nvim-treesitter/blob/23ba63028c6acca29be6462c0a291fc4a1b9eae8/CONTRIBUTING.md#non-highlighting-captures
                    .replace("@none", "")
                    .replace("@conceal", "")
                    .replace("@spell", "")
                    .replace("@nospell", "")
            })
    }

    pub fn locals_query(&self) -> Option<&'static str> {
        None
    }

    pub fn injection_query(&self) -> Option<&'static str> {
        None
    }

    pub fn lsp_process_command(&self) -> Option<ProcessCommand> {
        self.lsp_command
            .as_ref()
            .map(|command| ProcessCommand::new(command.command.0, command.command.1))
    }

    pub fn tree_sitter_grammar_id(&self) -> Option<String> {
        Some(self.tree_sitter_grammar_config()?.grammar_id)
    }

    pub fn id(&self) -> Option<LanguageId> {
        self.lsp_language_id
    }

    fn formatter_command(&self) -> Option<ProcessCommand> {
        self.formatter_command
            .as_ref()
            .map(|command| ProcessCommand::new(command.0, command.1))
    }

    pub fn formatter(&self) -> Option<Formatter> {
        self.formatter_command().map(Formatter::from)
    }
}

pub fn from_path(path: &CanonicalizedPath) -> Option<Language> {
    path.extension()
        .and_then(from_extension)
        .or_else(|| from_filename(path))
}

pub fn from_extension(extension: &str) -> Option<Language> {
    LANGUAGES
        .iter()
        .find(|language| language.extensions().contains(&extension))
        .map(|language| (*language).clone())
}

pub(crate) fn from_filename(path: &CanonicalizedPath) -> Option<Language> {
    let file_name = path.file_name()?;
    LANGUAGES
        .iter()
        .find(|language| language.file_names().contains(&file_name.as_str()))
        .map(|language| (*language).clone())
}

#[cfg(test)]
mod test_language {
    use super::*;
    use std::fs::File;
    #[test]
    fn test_from_path() -> anyhow::Result<()> {
        fn run_test_case(filename: &str, expected_language_id: &'static str) -> anyhow::Result<()> {
            let tempdir = tempfile::tempdir()?;
            let path = tempdir.path().join(filename);
            File::create(path.clone())?;
            let result = from_path(&path.to_string_lossy().to_string().try_into()?).unwrap();
            assert_eq!(
                result.tree_sitter_grammar_id().unwrap(),
                expected_language_id
            );
            Ok(())
        }
        run_test_case("hello.rs", "rust")?;
        run_test_case("justfile", "just")?;
        Ok(())
    }
}
