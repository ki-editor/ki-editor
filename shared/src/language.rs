use grammar::grammar::GrammarConfiguration;
use serde_json::Value;
use tree_sitter::Query;

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
    pub(crate) line_comment_prefix: Option<&'static str>,
    pub(crate) block_comment_affixes: Option<(&'static str, &'static str)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CargoLinkedTreesitterLanguage {
    Typescript,
    TSX,
    Python,
    Scheme,
    OCaml,
    OCamlInterface,
    Rust,
    Graphql,
    Javascript,
    JSX,
    Svelte,
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
    KiQuickfix,
    Haskell,
}

impl CargoLinkedTreesitterLanguage {
    pub(crate) fn to_tree_sitter_language(&self) -> tree_sitter::Language {
        match self {
            CargoLinkedTreesitterLanguage::Typescript => {
                tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
            }
            CargoLinkedTreesitterLanguage::TSX => tree_sitter_typescript::LANGUAGE_TSX.into(),
            CargoLinkedTreesitterLanguage::Python => tree_sitter_python::LANGUAGE.into(),
            CargoLinkedTreesitterLanguage::Scheme => tree_sitter_scheme::LANGUAGE.into(),
            CargoLinkedTreesitterLanguage::OCaml => tree_sitter_ocaml::LANGUAGE_OCAML.into(),
            CargoLinkedTreesitterLanguage::OCamlInterface => {
                tree_sitter_ocaml::LANGUAGE_OCAML_INTERFACE.into()
            }
            CargoLinkedTreesitterLanguage::Rust => tree_sitter_rust::LANGUAGE.into(),
            CargoLinkedTreesitterLanguage::Graphql => tree_sitter_graphql::LANGUAGE.into(),
            CargoLinkedTreesitterLanguage::Javascript => tree_sitter_javascript::LANGUAGE.into(),
            CargoLinkedTreesitterLanguage::JSX => tree_sitter_javascript::LANGUAGE.into(),
            CargoLinkedTreesitterLanguage::Svelte => tree_sitter_svelte_ng::LANGUAGE.into(),
            CargoLinkedTreesitterLanguage::JSON => tree_sitter_json::LANGUAGE.into(),
            CargoLinkedTreesitterLanguage::YAML => tree_sitter_yaml::LANGUAGE.into(),
            CargoLinkedTreesitterLanguage::HTML => tree_sitter_html::LANGUAGE.into(),
            CargoLinkedTreesitterLanguage::Haskell => tree_sitter_haskell::LANGUAGE.into(),
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
            CargoLinkedTreesitterLanguage::Fish => tree_sitter_fish::language(),
            CargoLinkedTreesitterLanguage::Diff => tree_sitter_diff::LANGUAGE.into(),
            CargoLinkedTreesitterLanguage::Elixir => tree_sitter_elixir::LANGUAGE.into(),
            CargoLinkedTreesitterLanguage::Swift => tree_sitter_swift::LANGUAGE.into(),
            CargoLinkedTreesitterLanguage::Heex => tree_sitter_heex::LANGUAGE.into(),
            CargoLinkedTreesitterLanguage::Toml => tree_sitter_toml_ng::LANGUAGE.into(),
            CargoLinkedTreesitterLanguage::KiQuickfix => tree_sitter_quickfix::language(),
        }
    }

    fn default_highlight_query(&self) -> Option<&str> {
        match self {
            CargoLinkedTreesitterLanguage::Typescript => {
                Some(tree_sitter_typescript::HIGHLIGHTS_QUERY)
            }
            CargoLinkedTreesitterLanguage::TSX => Some(tree_sitter_typescript::HIGHLIGHTS_QUERY),
            CargoLinkedTreesitterLanguage::Python => Some(tree_sitter_python::HIGHLIGHTS_QUERY),
            CargoLinkedTreesitterLanguage::Scheme => Some(tree_sitter_scheme::HIGHLIGHTS_QUERY),
            CargoLinkedTreesitterLanguage::OCaml => Some(tree_sitter_ocaml::HIGHLIGHTS_QUERY),
            CargoLinkedTreesitterLanguage::OCamlInterface => {
                Some(tree_sitter_ocaml::HIGHLIGHTS_QUERY)
            }
            CargoLinkedTreesitterLanguage::Rust => Some(tree_sitter_rust::HIGHLIGHTS_QUERY),
            CargoLinkedTreesitterLanguage::Graphql => None,
            CargoLinkedTreesitterLanguage::Javascript => {
                Some(tree_sitter_javascript::HIGHLIGHT_QUERY)
            }
            CargoLinkedTreesitterLanguage::JSX => Some(tree_sitter_javascript::HIGHLIGHT_QUERY),
            CargoLinkedTreesitterLanguage::Svelte => Some(tree_sitter_svelte_ng::HIGHLIGHTS_QUERY),
            CargoLinkedTreesitterLanguage::JSON => Some(tree_sitter_json::HIGHLIGHTS_QUERY),
            CargoLinkedTreesitterLanguage::YAML => Some(tree_sitter_yaml::HIGHLIGHTS_QUERY),
            CargoLinkedTreesitterLanguage::HTML => Some(tree_sitter_html::HIGHLIGHTS_QUERY),
            CargoLinkedTreesitterLanguage::Haskell => Some(tree_sitter_haskell::HIGHLIGHTS_QUERY),
            CargoLinkedTreesitterLanguage::XML => Some(tree_sitter_xml::XML_HIGHLIGHT_QUERY),
            CargoLinkedTreesitterLanguage::Zig => Some(tree_sitter_zig::HIGHLIGHTS_QUERY),
            CargoLinkedTreesitterLanguage::Markdown => Some(tree_sitter_md::HIGHLIGHT_QUERY_BLOCK),
            CargoLinkedTreesitterLanguage::Go => Some(tree_sitter_go::HIGHLIGHTS_QUERY),
            CargoLinkedTreesitterLanguage::Lua => Some(tree_sitter_lua::HIGHLIGHTS_QUERY),
            CargoLinkedTreesitterLanguage::Gleam => Some(tree_sitter_gleam::HIGHLIGHT_QUERY),
            CargoLinkedTreesitterLanguage::Bash => Some(tree_sitter_bash::HIGHLIGHT_QUERY),
            CargoLinkedTreesitterLanguage::C => Some(tree_sitter_c::HIGHLIGHT_QUERY),
            CargoLinkedTreesitterLanguage::CPP => Some(tree_sitter_cpp::HIGHLIGHT_QUERY),
            CargoLinkedTreesitterLanguage::CSS => Some(tree_sitter_css::HIGHLIGHTS_QUERY),
            CargoLinkedTreesitterLanguage::Ruby => Some(tree_sitter_ruby::HIGHLIGHTS_QUERY),
            CargoLinkedTreesitterLanguage::Nix => Some(tree_sitter_nix::HIGHLIGHTS_QUERY),
            CargoLinkedTreesitterLanguage::Fish => Some(tree_sitter_fish::HIGHLIGHTS_QUERY),
            CargoLinkedTreesitterLanguage::Diff => Some(tree_sitter_diff::HIGHLIGHTS_QUERY),
            CargoLinkedTreesitterLanguage::Elixir => Some(tree_sitter_elixir::HIGHLIGHTS_QUERY),
            CargoLinkedTreesitterLanguage::Swift => Some(tree_sitter_swift::HIGHLIGHTS_QUERY),
            CargoLinkedTreesitterLanguage::Heex => Some(tree_sitter_heex::HIGHLIGHTS_QUERY),
            CargoLinkedTreesitterLanguage::Toml => Some(tree_sitter_toml_ng::HIGHLIGHTS_QUERY),
            CargoLinkedTreesitterLanguage::KiQuickfix => Some(r#" (header) @keyword"#),
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
    pub id: &'static str,
    pub kind: GrammarConfigKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrammarConfigKind {
    /// This is the recommended over `FromSource`, as `FromSource`
    /// is not reliable across different operating system.
    CargoLinked(CargoLinkedTreesitterLanguage),
    FromSource {
        url: &'static str,
        commit: &'static str,
        subpath: Option<&'static str>,
    },
}

impl Language {
    fn extensions(&self) -> &'static [&'static str] {
        self.extensions
    }

    pub fn initialization_options(&self) -> Option<Value> {
        serde_json::from_str(self.lsp_command.clone()?.initialization_options?).ok()
    }

    pub fn tree_sitter_language(&self) -> Option<tree_sitter::Language> {
        let config = self.tree_sitter_grammar_config.as_ref()?;
        match &config.kind {
            GrammarConfigKind::CargoLinked(language) => Some(language.to_tree_sitter_language()),
            GrammarConfigKind::FromSource { .. } => grammar::grammar::get_language(config.id).ok(),
        }
    }

    pub(crate) fn tree_sitter_grammar_config(&self) -> Option<GrammarConfiguration> {
        match self.tree_sitter_grammar_config.as_ref()?.kind {
            GrammarConfigKind::CargoLinked(_) => None,
            GrammarConfigKind::FromSource {
                url,
                commit,
                subpath,
            } => self
                .tree_sitter_grammar_config
                .as_ref()
                .map(|config| GrammarConfiguration::remote(config.id, url, commit, subpath)),
        }
    }

    /// We prioritize using highlight queries from nvim-treesitter
    /// over the default highlight queries provided by each Treesitter grammar
    /// repositories because the former produces better syntax highlighting.
    ///
    /// However, in the event that the tree-sitter-highlight crates cannot
    /// handle the nvim-treesitter query due to issues like Neovim-specific directives
    /// (this is validated through the use of `tree_sitter::Query::new`),
    /// we will fallback to the default highlight queries.
    pub fn highlight_query(&self) -> Option<String> {
        if let Some(query) = self.highlight_query_nvim_treesitter() {
            match Query::new(&self.tree_sitter_language()?, &query) {
                Ok(_) => return Some(query),
                Err(error) => {
                    log::error!(
                        "[Language::highlight_query]: Falling back to default query; unable to use highlight query of {} from nvim-treesitter due to error: {error:?}",
                        self.tree_sitter_grammar_config.clone()?.id
                    )
                }
            }
        }
        self.highlight_query_default()
    }

    pub fn highlight_query_nvim_treesitter(&self) -> Option<String> {
        get_highlight_query(self.tree_sitter_grammar_config.clone()?.id)
            .ok()
            .map(|result| {
                result
                    .query
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

    fn highlight_query_default(&self) -> Option<String> {
        let config = self.tree_sitter_grammar_config.as_ref()?;
        match &config.kind {
            GrammarConfigKind::CargoLinked(language) => {
                Some(language.default_highlight_query()?.to_string())
            }
            GrammarConfigKind::FromSource { .. } => grammar::grammar::load_runtime_file(
                &self.tree_sitter_grammar_id()?,
                "highlights.scm",
            )
            .ok(),
        }
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
        Some(self.tree_sitter_grammar_config.as_ref()?.id.to_string())
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
