use std::{collections::HashMap, path::PathBuf, sync::mpsc::Sender};

use itertools::Itertools;

use crate::{lsp::language::get_languages, screen::ScreenMessage, utils::consolidate_errors};

use super::{language::Language, process::LspServerProcessChannel};

pub struct LspManager {
    lsp_server_process_channels: HashMap<Language, LspServerProcessChannel>,
    sender: Sender<ScreenMessage>,
}

impl Drop for LspManager {
    fn drop(&mut self) {
        for (_, channel) in self.lsp_server_process_channels.drain() {
            channel
                .shutdown()
                .unwrap_or_else(|error| log::error!("{:?}", error));
        }
    }
}

impl LspManager {
    pub fn new(clone: Sender<ScreenMessage>) -> LspManager {
        LspManager {
            lsp_server_process_channels: HashMap::new(),
            sender: clone,
        }
    }

    fn invoke_channels(
        &self,
        path: &PathBuf,
        error: &str,
        f: impl Fn(&LspServerProcessChannel) -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        let languages = get_languages(&path);
        let results = languages
            .into_iter()
            .filter_map(|language| self.lsp_server_process_channels.get(&language))
            .map(|channel| f(channel))
            .collect_vec();
        consolidate_errors(error, results)
    }

    pub fn request_completion(
        &self,
        path: std::path::PathBuf,
        position: lsp_types::Position,
    ) -> anyhow::Result<()> {
        let languages = get_languages(&path);
        let channels = languages
            .into_iter()
            .filter_map(|language| self.lsp_server_process_channels.get(&language))
            .collect_vec();
        log::info!("Requesting completion for {:?}", path);
        consolidate_errors(
            "Failed to request completion",
            channels
                .into_iter()
                .map(|channel| channel.request_completion(&path, position))
                .collect_vec(),
        )
    }

    pub fn document_did_change(&self, path: PathBuf, content: String) -> anyhow::Result<()> {
        self.invoke_channels(&path, "Failed to notify document did change", |channel| {
            channel.document_did_change(&path, &content)
        })
    }

    /// Open file can do one of the following:
    /// 1. Start a new LSP server process if it is not started yet.
    /// 2. Notify the LSP server process that a new file is opened.
    /// 3. Do nothing if the LSP server process is spawned but not yet initialized.
    pub fn open_file(&mut self, path: PathBuf) -> Result<(), anyhow::Error> {
        let languages = get_languages(&path);

        consolidate_errors(
            "Failed to start language server",
            languages
                .into_iter()
                .map(|language| {
                    if let Some(channel) = self.lsp_server_process_channels.get(&language) {
                        if channel.is_initialized() {
                            channel.document_did_open(path.clone())
                        } else {
                            Ok(())
                        }
                    } else {
                        language.spawn_lsp(self.sender.clone()).map(|channel| {
                            self.lsp_server_process_channels.insert(language, channel);
                        })
                    }
                })
                .collect_vec(),
        )
    }

    pub fn initialized(&mut self, language: Language, opened_documents: Vec<PathBuf>) {
        self.lsp_server_process_channels
            .get_mut(&language)
            .map(|channel| {
                channel.initialized();
                channel.documents_did_open(opened_documents)
            });
    }
}
