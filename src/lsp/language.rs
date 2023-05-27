use std::{path::PathBuf, sync::mpsc::Sender};

use crate::screen::ScreenMessage;

use super::process::LspServerProcessChannel;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub enum Language {
    Rust,
}

/// This returns a list because some files may have multiple languages,
/// for example, HTML file may contains CSS and JavaScript.
pub fn get_languages(path: &PathBuf) -> Vec<Language> {
    path.extension()
        .map(|extension| extension.to_str())
        .flatten()
        .map(|extension| match extension {
            "rs" => vec![Language::Rust],
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
        }
    }

    /// Refer https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocumentItem
    pub fn id(&self) -> String {
        match self {
            Language::Rust => "rust".to_string(),
        }
    }
}
