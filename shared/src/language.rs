use grammar::grammar::GrammarConfiguration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tree_sitter::Query;

pub(crate) use crate::process_command::ProcessCommand;
use crate::{formatter::Formatter, ts_highlight_query::get_highlight_query};

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
/// As defined by the LSP protocol.
/// See sections below https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#range
pub struct LanguageId(String);

impl std::fmt::Display for LanguageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl LanguageId {
    pub fn new(id: &'static str) -> Self {
        Self(id.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Command {
    pub command: String,
    pub arguments: Vec<String>,
}
impl Command {
    pub fn new(command: &'static str, arguments: &[&'static str]) -> Self {
        Self {
            command: command.to_string(),
            arguments: arguments.iter().map(|arg| arg.to_string()).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Language {
    #[schemars(example = &["ts", "tsx"])]
    pub(crate) extensions: Vec<String>,
    /// For files without extensions.
    #[schemars(example = &["Dockerfile"])]
    pub(crate) file_names: Vec<String>,
    pub(crate) lsp_language_id: Option<LanguageId>,
    pub(crate) lsp_command: Option<LspCommand>,
    pub(crate) tree_sitter_grammar_config: Option<GrammarConfig>,
    /// The formatter command will receive the content from STDIN
    /// and is expected to return the formatted output to STDOUT.
    pub(crate) formatter: Option<Command>,
    #[schemars(example = "//")]
    pub(crate) line_comment_prefix: Option<String>,
    #[schemars(example = ("/*", "*/"))]
    pub(crate) block_comment_affixes: Option<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum CargoLinkedTreesitterLanguage {
    Typescript,
    TSX,
    Python,
    Julia,
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
    Hcl,
    Odin,
    CSharp,
}

impl CargoLinkedTreesitterLanguage {
    pub(crate) fn to_tree_sitter_language(&self) -> tree_sitter::Language {
        match self {
            CargoLinkedTreesitterLanguage::Typescript => {
                tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
            }
            CargoLinkedTreesitterLanguage::TSX => tree_sitter_typescript::LANGUAGE_TSX.into(),
            CargoLinkedTreesitterLanguage::Python => tree_sitter_python::LANGUAGE.into(),
            CargoLinkedTreesitterLanguage::Julia => tree_sitter_julia::LANGUAGE.into(),
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
            CargoLinkedTreesitterLanguage::Hcl => tree_sitter_hcl::LANGUAGE.into(),
            CargoLinkedTreesitterLanguage::Odin => tree_sitter_odin::LANGUAGE.into(),
            CargoLinkedTreesitterLanguage::CSharp => tree_sitter_c_sharp::LANGUAGE.into(),
        }
    }

    fn default_highlight_query(&self) -> Option<&str> {
        match self {
            CargoLinkedTreesitterLanguage::Typescript => {
                Some(tree_sitter_typescript::HIGHLIGHTS_QUERY)
            }
            CargoLinkedTreesitterLanguage::TSX => Some(tree_sitter_typescript::HIGHLIGHTS_QUERY),
            CargoLinkedTreesitterLanguage::Python => Some(tree_sitter_python::HIGHLIGHTS_QUERY),
            CargoLinkedTreesitterLanguage::Julia => None,
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
            CargoLinkedTreesitterLanguage::Hcl => None,
            CargoLinkedTreesitterLanguage::Odin => Some(tree_sitter_odin::HIGHLIGHTS_QUERY),
            CargoLinkedTreesitterLanguage::CSharp => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct LspCommand {
    pub(crate) command: Command,
    pub(crate) initialization_options: Option<serde_json::Value>,
}

impl Language {
    pub const fn new() -> Self {
        Self {
            extensions: Vec::new(),
            file_names: Vec::new(),
            lsp_language_id: None,
            lsp_command: None,
            tree_sitter_grammar_config: None,
            formatter: None,
            line_comment_prefix: None,
            block_comment_affixes: None,
        }
    }

    pub fn file_names(&self) -> &Vec<String> {
        &self.file_names
    }

    pub fn lsp_language_id(&self) -> &Option<LanguageId> {
        &self.lsp_language_id
    }

    pub fn line_comment_prefix(&self) -> Option<String> {
        self.line_comment_prefix.clone()
    }

    pub fn block_comment_affixes(&self) -> Option<(String, String)> {
        self.block_comment_affixes.clone()
    }
}

impl Default for Language {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GrammarConfig {
    pub id: String,
    pub kind: GrammarConfigKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum GrammarConfigKind {
    /// This is the recommended over `FromSource`, as `FromSource`
    /// is not reliable across different operating system.
    CargoLinked(CargoLinkedTreesitterLanguage),
    FromSource {
        url: String,
        commit: String,
        subpath: Option<String>,
    },
}

impl Language {
    pub fn extensions(&self) -> &Vec<String> {
        &self.extensions
    }

    pub fn initialization_options(&self) -> Option<Value> {
        self.lsp_command.clone()?.initialization_options
    }

    pub fn tree_sitter_language(&self) -> Option<tree_sitter::Language> {
        let config = self.tree_sitter_grammar_config.as_ref()?;
        match &config.kind {
            GrammarConfigKind::CargoLinked(language) => Some(language.to_tree_sitter_language()),
            GrammarConfigKind::FromSource { .. } => grammar::grammar::get_language(&config.id).ok(),
        }
    }

    pub fn tree_sitter_grammar_config(&self) -> Option<GrammarConfiguration> {
        match &self.tree_sitter_grammar_config.as_ref()?.kind {
            GrammarConfigKind::CargoLinked(_) => None,
            GrammarConfigKind::FromSource {
                url,
                commit,
                subpath,
            } => self.tree_sitter_grammar_config.as_ref().map(|config| {
                GrammarConfiguration::remote(&config.id, url, commit, subpath.clone())
            }),
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
        get_highlight_query(&self.tree_sitter_grammar_config.clone()?.id).map(|result| {
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
        self.lsp_command.as_ref().map(|command| {
            ProcessCommand::new(&command.command.command, &command.command.arguments)
        })
    }

    pub fn tree_sitter_grammar_id(&self) -> Option<String> {
        Some(self.tree_sitter_grammar_config.as_ref()?.id.to_string())
    }

    pub fn id(&self) -> Option<LanguageId> {
        self.lsp_language_id.clone()
    }

    fn formatter_command(&self) -> Option<ProcessCommand> {
        self.formatter
            .as_ref()
            .map(|command| ProcessCommand::new(&command.command, &command.arguments))
    }

    pub fn formatter(&self) -> Option<Formatter> {
        self.formatter_command().map(Formatter::from)
    }
}
