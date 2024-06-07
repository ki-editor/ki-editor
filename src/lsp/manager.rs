use std::{collections::HashMap, sync::mpsc::Sender};

use crate::app::AppMessage;

use super::process::{FromEditor, LspServerProcessChannel};
use shared::{
    canonicalized_path::CanonicalizedPath,
    language::{self, Language, LanguageId},
};

pub(crate) struct LspManager {
    lsp_server_process_channels: HashMap<LanguageId, LspServerProcessChannel>,
    sender: Sender<AppMessage>,
    current_working_directory: CanonicalizedPath,
    #[cfg(test)]
    /// Used for testing the correctness of LSP requests
    /// We use HashMap instead of Vec because we only one to store the latest
    /// requests of the same kind
    history: HashMap</* request name */ &'static str, FromEditor>,
}

impl Drop for LspManager {
    fn drop(&mut self) {
        self.shutdown()
    }
}

impl LspManager {
    pub(crate) fn new(
        sender: Sender<AppMessage>,
        current_working_directory: CanonicalizedPath,
    ) -> LspManager {
        LspManager {
            lsp_server_process_channels: HashMap::new(),
            sender,
            current_working_directory,
            #[cfg(test)]
            history: Default::default(),
        }
    }

    fn invoke_channels(
        &self,
        path: &CanonicalizedPath,
        _error: &str,
        f: impl Fn(&LspServerProcessChannel) -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        language::from_path(path)
            .and_then(|language| self.lsp_server_process_channels.get(&language.id()?))
            .map(f)
            .unwrap_or_else(|| Ok(()))
    }

    pub(crate) fn send_message(
        &mut self,
        path: CanonicalizedPath,
        from_editor: FromEditor,
    ) -> anyhow::Result<()> {
        #[cfg(test)]
        self.history
            .insert(from_editor.variant(), from_editor.clone());

        self.invoke_channels(
            &path,
            &format!("Failed to send message '{}'", from_editor.variant()),
            |channel| channel.send_from_editor(from_editor.clone()),
        )
    }

    /// Open file can do one of the following:
    /// 1. Start a new LSP server process if it is not started yet.
    /// 2. Notify the LSP server process that a new file is opened.
    /// 3. Do nothing if the LSP server process is spawned but not yet initialized.

    pub(crate) fn open_file(&mut self, path: CanonicalizedPath) -> Result<(), anyhow::Error> {
        let Some(language) = language::from_path(&path) else {
            return Ok(());
        };
        let Some(language_id) = language.id() else {
            return Ok(());
        };

        if let Some(channel) = self.lsp_server_process_channels.get(&language_id) {
            if channel.is_initialized() {
                channel.document_did_open(path.clone())
            } else {
                Ok(())
            }
        } else {
            LspServerProcessChannel::new(
                language.clone(),
                self.sender.clone(),
                self.current_working_directory.clone(),
            )
            .map(|channel| {
                if let Some(channel) = channel {
                    self.lsp_server_process_channels
                        .insert(language.id()?, channel);
                }
                Some(())
            })?;
            Ok(())
        }
    }

    pub(crate) fn initialized(
        &mut self,
        language: Language,
        opened_documents: Vec<CanonicalizedPath>,
    ) {
        let Some(language_id) = language.id() else {
            return;
        };
        self.lsp_server_process_channels
            .get_mut(&language_id)
            .map(|channel| {
                channel.initialized();
                channel.documents_did_open(opened_documents)
            });
    }

    pub(crate) fn shutdown(&mut self) {
        for (_, channel) in self.lsp_server_process_channels.drain() {
            channel
                .shutdown()
                .unwrap_or_else(|error| log::error!("{:?}", error));
        }
    }

    #[cfg(test)]
    pub(crate) fn lsp_request_sent(&self, from_editor: &FromEditor) -> bool {
        self.history.get(from_editor.variant()) == Some(from_editor)
    }
}
