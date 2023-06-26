use crate::{
    canonicalized_path::CanonicalizedPath,
    language::{self, Language, LanguageId},
    screen::RequestParams,
};
use std::{collections::HashMap, sync::mpsc::Sender};

use crate::screen::ScreenMessage;

use super::process::LspServerProcessChannel;

pub struct LspManager {
    lsp_server_process_channels: HashMap<LanguageId, LspServerProcessChannel>,
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
        path: &CanonicalizedPath,
        _error: &str,
        f: impl Fn(&LspServerProcessChannel) -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        language::from_path(path)
            .and_then(|language| self.lsp_server_process_channels.get(&language.id()))
            .map(f)
            .unwrap_or_else(|| Ok(()))
    }

    pub fn request_completion(&self, params: RequestParams) -> anyhow::Result<()> {
        self.invoke_channels(&params.path, "Failed to request completion", |channel| {
            channel.request_completion(params.clone())
        })
    }

    pub fn request_hover(&self, params: RequestParams) -> anyhow::Result<()> {
        self.invoke_channels(&params.path, "Failed to request hover", |channel| {
            channel.request_hover(params.clone())
        })
    }

    pub fn request_definition(&self, params: RequestParams) -> anyhow::Result<()> {
        self.invoke_channels(&params.path, "Failed to go to definition", |channel| {
            channel.request_definition(params.clone())
        })
    }

    pub fn request_references(&self, params: RequestParams) -> anyhow::Result<()> {
        self.invoke_channels(&params.path, "Failed to find references", |channel| {
            channel.request_references(params.clone())
        })
    }

    pub fn prepare_rename_symbol(&self, params: RequestParams) -> anyhow::Result<()> {
        self.invoke_channels(&params.path, "Failed to prepare rename symbol", |channel| {
            channel.prepare_rename_symbol(params.clone())
        })
    }

    pub fn rename_symbol(&self, params: RequestParams, new_name: String) -> anyhow::Result<()> {
        self.invoke_channels(&params.path, "Failed to rename symbol", |channel| {
            channel.rename_symbol(params.clone(), new_name.clone())
        })
    }

    pub fn document_did_change(
        &self,
        path: CanonicalizedPath,
        content: String,
    ) -> anyhow::Result<()> {
        self.invoke_channels(&path, "Failed to notify document did change", |channel| {
            channel.document_did_change(&path, &content)
        })
    }

    pub fn document_did_save(&self, path: CanonicalizedPath) -> anyhow::Result<()> {
        self.invoke_channels(&path, "Failed to notify document did save", |channel| {
            channel.document_did_save(&path)
        })
    }

    /// Open file can do one of the following:
    /// 1. Start a new LSP server process if it is not started yet.
    /// 2. Notify the LSP server process that a new file is opened.
    /// 3. Do nothing if the LSP server process is spawned but not yet initialized.
    pub fn open_file(&mut self, path: CanonicalizedPath) -> Result<(), anyhow::Error> {
        let languages = language::from_path(&path);

        languages
            .map(|language| {
                if let Some(channel) = self.lsp_server_process_channels.get(&language.id()) {
                    if channel.is_initialized() {
                        channel.document_did_open(path.clone())
                    } else {
                        Ok(())
                    }
                } else {
                    LspServerProcessChannel::new(language.clone(), self.sender.clone()).map(
                        |channel| {
                            if let Some(channel) = channel {
                                self.lsp_server_process_channels
                                    .insert(language.id(), channel);
                            }
                        },
                    )
                }
            })
            .unwrap_or_else(|| Ok(()))
    }

    pub fn initialized(
        &mut self,
        language: Box<dyn Language>,
        opened_documents: Vec<CanonicalizedPath>,
    ) {
        self.lsp_server_process_channels
            .get_mut(&language.id())
            .map(|channel| {
                channel.initialized();
                channel.documents_did_open(opened_documents)
            });
    }
}
