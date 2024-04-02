use crate::app::RequestParams;
use std::{collections::HashMap, sync::mpsc::Sender};

use crate::app::AppMessage;

use super::process::LspServerProcessChannel;
use shared::{
    canonicalized_path::CanonicalizedPath,
    language::{self, Language, LanguageId},
};
use tokio::sync::mpsc::UnboundedSender;

pub struct LspManager {
    lsp_server_process_channels: HashMap<LanguageId, LspServerProcessChannel>,
    sender: UnboundedSender<AppMessage>,
    current_working_directory: CanonicalizedPath,
}

impl Drop for LspManager {
    fn drop(&mut self) {
        self.shutdown()
    }
}

impl LspManager {
    pub fn new(
        sender: UnboundedSender<AppMessage>,
        current_working_directory: CanonicalizedPath,
    ) -> LspManager {
        LspManager {
            lsp_server_process_channels: HashMap::new(),
            sender,
            current_working_directory,
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

    pub fn request_references(
        &self,
        params: RequestParams,
        include_declaration: bool,
    ) -> anyhow::Result<()> {
        self.invoke_channels(&params.path, "Failed to find references", |channel| {
            channel.request_references(params.clone(), include_declaration)
        })
    }

    pub fn request_declaration(&self, params: RequestParams) -> anyhow::Result<()> {
        self.invoke_channels(&params.path, "Failed to go to declaration", |channel| {
            channel.request_declaration(params.clone())
        })
    }

    pub fn request_implementation(&self, params: RequestParams) -> anyhow::Result<()> {
        self.invoke_channels(&params.path, "Failed to go to implementation", |channel| {
            channel.request_implementation(params.clone())
        })
    }

    pub fn request_type_definition(&self, params: RequestParams) -> anyhow::Result<()> {
        self.invoke_channels(&params.path, "Failed to go to type definition", |channel| {
            channel.request_type_definition(params.clone())
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

    pub fn request_code_action(
        &self,
        action: RequestParams,
        diagnostics: Vec<lsp_types::Diagnostic>,
    ) -> anyhow::Result<()> {
        self.invoke_channels(&action.path, "Failed to request code action", |channel| {
            channel.request_code_action(action.clone(), diagnostics.clone())
        })
    }

    pub fn request_signature_help(&self, params: RequestParams) -> anyhow::Result<()> {
        self.invoke_channels(
            &params.path,
            "Failed to request signature help",
            |channel| channel.request_signature_help(params.clone()),
        )
    }

    pub fn request_document_symbols(&self, params: RequestParams) -> anyhow::Result<()> {
        self.invoke_channels(
            &params.path,
            "Failed to request document symbols",
            |channel| channel.request_document_symbols(params.clone()),
        )
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

    pub fn document_did_rename(
        &mut self,
        old: CanonicalizedPath,
        new: CanonicalizedPath,
    ) -> anyhow::Result<()> {
        self.invoke_channels(&old, "Failed to notify document did rename", |channel| {
            channel.document_did_rename(old.clone(), new.clone())
        })
    }
    /// Open file can do one of the following:
    /// 1. Start a new LSP server process if it is not started yet.
    /// 2. Notify the LSP server process that a new file is opened.
    /// 3. Do nothing if the LSP server process is spawned but not yet initialized.

    pub fn open_file(&mut self, path: CanonicalizedPath) -> Result<(), anyhow::Error> {
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

    pub fn initialized(&mut self, language: Language, opened_documents: Vec<CanonicalizedPath>) {
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

    pub fn shutdown(&mut self) {
        for (_, channel) in self.lsp_server_process_channels.drain() {
            channel
                .shutdown()
                .unwrap_or_else(|error| log::error!("{:?}", error));
        }
    }

    pub(crate) fn workspace_execute_command(
        &self,
        params: RequestParams,
        command: super::code_action::Command,
    ) -> Result<(), anyhow::Error> {
        self.invoke_channels(
            &params.path.clone(),
            "Failed to execute command",
            |channel| channel.workspace_execute_command(params.clone(), command.clone()),
        )
    }
}
