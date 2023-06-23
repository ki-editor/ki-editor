use std::{sync::mpsc::Sender};

use crate::{canonicalized_path::CanonicalizedPath, screen::ScreenMessage};

use super::{formatter::Formatter, process::LspServerProcessChannel};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub enum Language {
    Rust,
    Typescript,
    TypescriptReact,
    JavaScript,
    JavaScriptReact,
    Markdown,
}

/// This returns a list because some files may have multiple languages,
/// for example, HTML file may contains CSS and JavaScript.
pub fn get_language(path: &CanonicalizedPath) -> Option<Language> {
    Language::from_path(path)
}

impl Language {
    pub fn from_path(path: &CanonicalizedPath) -> Option<Language> {
        path.extension()
            .map(|extension| match extension {
                "rs" => Some(Language::Rust),
                "ts" => Some(Language::Typescript),
                "tsx" => Some(Language::TypescriptReact),
                "js" => Some(Language::JavaScript),
                "jsx" => Some(Language::JavaScriptReact),
                "md" => Some(Language::Markdown),
                _ => None,
            })
            .unwrap_or_default()
    }
    pub fn spawn_lsp(
        self,
        screen_message_sender: Sender<ScreenMessage>,
    ) -> Result<Option<LspServerProcessChannel>, anyhow::Error> {
        LspServerProcessChannel::new(self, screen_message_sender)
    }

    pub fn get_lsp_command_args(&self) -> Option<(&str, Vec<&str>)> {
        match self {
            Language::Rust => Some(("rust-analyzer", vec![])),
            Language::Typescript
            | Language::TypescriptReact
            | Language::JavaScript
            | Language::JavaScriptReact => Some(("typescript-language-server", vec!["--stdio"])),
            _ => None,
        }
    }

    /// Refer https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocumentItem
    pub fn id(&self) -> String {
        match self {
            Language::Rust => "rust".to_string(),
            Language::Typescript => "typescript".to_string(),
            Language::TypescriptReact => "typescriptreact".to_string(),
            Language::JavaScript => "javascript".to_string(),
            Language::JavaScriptReact => "javascriptreact".to_string(),
            Language::Markdown => "markdown".to_string(),
        }
    }

    pub fn tree_sitter_language(&self) -> tree_sitter::Language {
        match self {
            Language::JavaScript => tree_sitter_javascript::language(),
            Language::Typescript => tree_sitter_typescript::language_typescript(),
            Language::TypescriptReact => tree_sitter_typescript::language_tsx(),
            Language::Rust => tree_sitter_rust::language(),
            Language::Markdown => tree_sitter_md::language(),

            // By default use the Markdown language
            _ => tree_sitter_md::language(),
        }
    }

    pub fn formatter(&self) -> Option<Formatter> {
        Formatter::from_language(self)
    }
}
