use std::{path::PathBuf, sync::mpsc::Sender};

use crate::{canonicalized_path::CanonicalizedPath, screen::ScreenMessage};

use super::process::LspServerProcessChannel;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub enum Language {
    Rust,
    Typescript,
    TypescriptReact,
    JavaScript,
    JavaScriptReact,
}

/// This returns a list because some files may have multiple languages,
/// for example, HTML file may contains CSS and JavaScript.
pub fn get_languages(path: &CanonicalizedPath) -> Vec<Language> {
    path.extension()
        .map(|extension| match extension {
            "rs" => vec![Language::Rust],
            "ts" => vec![Language::Typescript],
            "tsx" => vec![Language::TypescriptReact],
            "js" => vec![Language::JavaScript],
            "jsx" => vec![Language::JavaScriptReact],
            _ => vec![],
        })
        .unwrap_or(vec![])
}

impl Language {
    pub fn spawn_lsp(
        self,
        screen_message_sender: Sender<ScreenMessage>,
    ) -> Result<LspServerProcessChannel, anyhow::Error> {
        LspServerProcessChannel::new(self, screen_message_sender)
    }

    pub fn get_command_args(&self) -> (&str, Vec<&str>) {
        match self {
            Language::Rust => ("rust-analyzer", vec![]),
            Language::Typescript
            | Language::TypescriptReact
            | Language::JavaScript
            | Language::JavaScriptReact => ("typescript-language-server", vec!["--stdio"]),
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
        }
    }
}
