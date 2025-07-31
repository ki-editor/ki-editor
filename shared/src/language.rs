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
    /// This will be used when we can't load the language file using `tree_sitter_grammar_config`.
    pub(crate) language_fallback: Option<CargoLinkedTreesitterLanguage>,
    pub(crate) highlight_query: Option<&'static str>,
    pub(crate) formatter_command: Option<Command>,
    pub(crate) line_comment_prefix: Option<&'static str>,
    pub(crate) block_comment_affixes: Option<(&'static str, &'static str)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CargoLinkedTreesitterLanguage {
    Typescript,
    TSX,
    Python,
    Rust,
    Graphql,
    Javascript,
    JSX,
    JSON,
    YAML,
    HTML,
    XML,
    Zig,
    Markdown,
    Go,
    Lua,
    Gleam,
    Bash,
    C,
    CPP,
    CSS,
    Ruby,
    Nix,
    Fish,
    Diff,
    Elixir,
    Swift,
    Heex,
    Toml,
}

impl CargoLinkedTreesitterLanguage {
    pub(crate) fn to_tree_sitter_language(&self) -> tree_sitter::Language {
        match self {
            CargoLinkedTreesitterLanguage::Typescript => {
                tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
            }
            CargoLinkedTreesitterLanguage::TSX => tree_sitter_typescript::LANGUAGE_TSX.into(),
            CargoLinkedTreesitterLanguage::Python => tree_sitter_python::LANGUAGE.into(),
            CargoLinkedTreesitterLanguage::Rust => tree_sitter_rust::LANGUAGE.into(),
            CargoLinkedTreesitterLanguage::Graphql => tree_sitter_graphql::LANGUAGE.into(),
            CargoLinkedTreesitterLanguage::Javascript => tree_sitter_javascript::LANGUAGE.into(),
            CargoLinkedTreesitterLanguage::JSX => tree_sitter_javascript::LANGUAGE.into(),
            CargoLinkedTreesitterLanguage::JSON => tree_sitter_json::LANGUAGE.into(),
            CargoLinkedTreesitterLanguage::YAML => tree_sitter_yaml::LANGUAGE.into(),
            CargoLinkedTreesitterLanguage::HTML => tree_sitter_html::LANGUAGE.into(),
            CargoLinkedTreesitterLanguage::XML => tree_sitter_xml::LANGUAGE_XML.into(),
            CargoLinkedTreesitterLanguage::Zig => tree_sitter_zig::LANGUAGE.into(),
            CargoLinkedTreesitterLanguage::Markdown => tree_sitter_md::LANGUAGE.into(),
            CargoLinkedTreesitterLanguage::Go => tree_sitter_go::LANGUAGE.into(),
            CargoLinkedTreesitterLanguage::Lua => tree_sitter_lua::LANGUAGE.into(),
            CargoLinkedTreesitterLanguage::Gleam => tree_sitter_gleam::LANGUAGE.into(),
            CargoLinkedTreesitterLanguage::Bash => tree_sitter_bash::LANGUAGE.into(),
            CargoLinkedTreesitterLanguage::C => tree_sitter_c::LANGUAGE.into(),
            CargoLinkedTreesitterLanguage::CPP => tree_sitter_cpp::LANGUAGE.into(),
            CargoLinkedTreesitterLanguage::CSS => tree_sitter_css::LANGUAGE.into(),
            CargoLinkedTreesitterLanguage::Ruby => tree_sitter_ruby::LANGUAGE.into(),
            CargoLinkedTreesitterLanguage::Nix => tree_sitter_nix::LANGUAGE.into(),
            CargoLinkedTreesitterLanguage::Fish => tree_sitter_fish::language().into(),
            CargoLinkedTreesitterLanguage::Diff => tree_sitter_diff::LANGUAGE.into(),
            CargoLinkedTreesitterLanguage::Elixir => tree_sitter_elixir::LANGUAGE.into(),
            CargoLinkedTreesitterLanguage::Swift => tree_sitter_swift::LANGUAGE.into(),
            CargoLinkedTreesitterLanguage::Heex => tree_sitter_heex::LANGUAGE.into(),
            CargoLinkedTreesitterLanguage::Toml => tree_sitter_toml_ng::LANGUAGE.into(),
        }
    }
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
            language_fallback: None,
            line_comment_prefix: None,
            block_comment_affixes: None,
        }
    }

    fn file_names(&self) -> &'static [&'static str] {
        self.file_names
    }

    pub fn line_comment_prefix(&self) -> Option<&'static str> {
        self.line_comment_prefix
    }

    pub fn block_comment_affixes(&self) -> Option<(&'static str, &'static str)> {
        self.block_comment_affixes
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
        grammar::grammar::get_language(&self.tree_sitter_grammar_config()?.grammar_id)
            .map_err(|err| {
                log::error!(
                    "Language::tree_sitter_language: unable to obtain language due to {err:?}"
                );
                err
            })
            .ok()
            .or_else(|| Some(self.language_fallback.clone()?.to_tree_sitter_language()))
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

use regex::Regex;

/// Detect the language from the first line of the file content.
///
/// Standard shebang format is checked as well as vim's `ft=` method and various
/// other editors supporting `mode:`.
///
/// For example, a file opened that has any of the following first lines will be
/// detected as bash.
///
/// - `#!/bin/bash`
/// - `# vim: ft=bash`
/// - `# mode: bash
///
/// Spaces and other content on the line do not matter.
pub fn from_content_directive(content: &str) -> Option<Language> {
    let first_line = content.lines().next()?;

    let re = Regex::new(r"(?:(?:^#!.*/)|(?:mode:)|(?:ft\s*=))\s*(\w+)").unwrap();
    let language_id = re
        .captures(first_line)
        .and_then(|captures| captures.get(1).map(|mode| mode.as_str().to_string()));

    language_id.and_then(|id| {
        LANGUAGES
            .iter()
            .find(|language| {
                language
                    .lsp_language_id
                    .is_some_and(|lsp_id| lsp_id.0 == id)
            })
            .map(|language| (*language).clone())
    })
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

    #[test]
    fn test_from_content_directive() -> anyhow::Result<()> {
        fn run_test_case(content: &str, expected_language_id: &'static str) -> anyhow::Result<()> {
            let result = from_content_directive(content).unwrap();
            assert_eq!(
                result.tree_sitter_grammar_id().unwrap(),
                expected_language_id
            );
            Ok(())
        }

        run_test_case("#!/bin/bash", "bash")?;
        run_test_case("#!/usr/local/bin/bash", "bash")?;
        run_test_case("// mode: python", "python")?;
        run_test_case("-- tab_spaces: 5, mode: bash, use_tabs: false", "bash")?;
        run_test_case("-- tab_spaces: 5, mode:bash, use_tabs: false", "bash")?;
        run_test_case("-- vim: ft = bash", "bash")?;

        Ok(())
    }
}
