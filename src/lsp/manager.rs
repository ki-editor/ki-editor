use std::collections::HashMap;

use crate::app::AppMessage;

use super::process::{FromEditor, LspNotification, LspServerProcessChannel};
use shared::{
    absolute_path::AbsolutePath,
    language::{Language, LanguageId},
};

pub struct LspManager {
    lsp_server_process_channels: HashMap<LanguageId, LspServerProcessChannel>,
    sender: crossbeam_channel::Sender<AppMessage>,
    current_working_directory: AbsolutePath,
    #[cfg(test)]
    /// Used for testing the correctness of LSP requests
    /// We use HashMap instead of Vec because we only one to store the latest
    /// requests of the same kind
    history: HashMap</* request name */ &'static str, FromEditor>,

    #[cfg(test)]
    /// Used for testing the correctness of initialization
    lsp_server_initialized_args_history: Vec<(LanguageId, Vec<AbsolutePath>)>,
}

impl Drop for LspManager {
    fn drop(&mut self) {
        self.shutdown();
    }
}

impl LspManager {
    pub fn new(
        sender: crossbeam_channel::Sender<AppMessage>,
        current_working_directory: AbsolutePath,
    ) -> LspManager {
        LspManager {
            lsp_server_process_channels: HashMap::new(),
            sender,
            current_working_directory,
            #[cfg(test)]
            history: HashMap::default(),
            #[cfg(test)]
            lsp_server_initialized_args_history: Vec::default(),
        }
    }

    fn invoke_channels(
        &self,
        path: &AbsolutePath,
        _error: &str,
        f: impl Fn(&LspServerProcessChannel) -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        crate::config::from_path(path)
            .and_then(|language| self.lsp_server_process_channels.get(&language.id()?))
            .map(f)
            .unwrap_or_else(|| Ok(()))
    }

    pub fn send_message(
        &mut self,
        path: AbsolutePath,
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
    pub fn open_file(&mut self, path: AbsolutePath) -> Result<(), anyhow::Error> {
        let Some(language) = crate::config::from_path(&path) else {
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

    pub fn initialized(&mut self, language: Language, opened_documents: Vec<AbsolutePath>) {
        let Some(language_id) = language.id() else {
            return;
        };

        #[cfg(test)]
        self.lsp_server_initialized_args_history
            .push((language_id.clone(), opened_documents.clone()));

        self.lsp_server_process_channels
            .get_mut(&language_id)
            .map(|channel| {
                channel.initialized();
                channel.documents_did_open(opened_documents)
            });
    }

    pub fn shutdown(&mut self) {
        for (_, channel) in self.lsp_server_process_channels.drain() {
            channel
                .shutdown()
                .unwrap_or_else(|error| log::error!("{error:?}"));
        }
    }

    /// Restarts the LSP server process for the given `language`, if one is running.
    ///
    /// The existing process (if any) is shut down and a fresh one is spawned.
    /// Once the new process reports that it is initialized, `documents_did_open`
    /// will be replayed for currently open buffers (see `App::handle_lsp_notification`),
    /// so callers do not need to re-open any documents themselves.
    pub fn restart_language(&mut self, language: &Language) -> anyhow::Result<()> {
        let Some(language_id) = language.id() else {
            return Ok(());
        };

        if let Some(channel) = self.lsp_server_process_channels.remove(&language_id) {
            // `shutdown` blocks until the old process has actually stopped (or failed
            // to), so a failure is reported here before the replacement process is
            // spawned below, rather than racing with it. Success is not reported —
            // it's the expected outcome and not worth surfacing to the user.
            if let Err(error) = channel.shutdown() {
                let _ = self.sender.send(AppMessage::LspNotification(Box::new(
                    LspNotification::Error(format!(
                        "LSP server for {language_id} failed to shut down cleanly: {error:?}"
                    )),
                )));
            }
        }

        LspServerProcessChannel::new(
            language.clone(),
            self.sender.clone(),
            self.current_working_directory.clone(),
        )
        .map(|channel| {
            if let Some(channel) = channel {
                self.lsp_server_process_channels
                    .insert(language_id, channel);
            }
        })
    }

    /// Returns the distinct `Language`s that currently have a running LSP server process.
    pub fn running_languages(&self) -> Vec<Language> {
        self.lsp_server_process_channels
            .values()
            .map(|channel| channel.language().clone())
            .collect()
    }

    #[cfg(test)]
    pub fn lsp_request_sent(&self, from_editor: &FromEditor) -> bool {
        self.history.get(from_editor.variant()) == Some(from_editor)
    }

    #[cfg(test)]
    pub fn lsp_server_initialized_args(&self) -> Option<(LanguageId, Vec<AbsolutePath>)> {
        self.lsp_server_initialized_args_history.last().cloned()
    }
}
