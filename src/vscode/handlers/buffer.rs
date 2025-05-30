//! Buffer-related handlers for VSCode IPC messages

use super::prelude::*;
use crate::{
    app::Dispatch,
    buffer::BufferOwner,
    components::editor::DispatchEditor,
    context::Context,
    edit::{Action, ActionGroup, Edit, EditTransaction},
    selection::SelectionSet,
    vscode::{
        app::VSCodeApp,
        utils::{uri_to_path, vscode_position_to_ki_position}, // Use position conversion util
    },
};
use itertools::Itertools;
use ki_protocol_types::{
    BufferDiffParams, // Use BufferDiffParams instead of InputBufferChangeParams
    BufferParams,

    OutputMessage,
    OutputMessageWrapper,
    ResponseError,
};
use log::{debug, error, info, warn}; // Added debug
use ropey::Rope; // Added Rope

impl VSCodeApp {
    /// Handle buffer open request from VSCode
    pub fn handle_buffer_open_request(&mut self, id: u64, params: BufferParams) -> Result<()> {
        let BufferParams {
            uri,
            content,
            language_id: _,
            version,
        } = params;

        let path = uri_to_path(&uri)?;

        // Store the original buffer_id for versioning
        let buffer_id = uri.to_string();

        // Update buffer version
        self.buffer_versions
            .insert(buffer_id.clone(), version.unwrap_or(0) as u64);

        // First check if the file is already open
        let current_path = self.get_current_file_path();
        if current_path.as_ref() == Some(&path) {
            info!("Requested file is already open");
            return Ok(());
        }

        // Open the file
        let dispatch = Dispatch::OpenFile {
            path: path.clone(),
            owner: BufferOwner::User,
            focus: true,
        };
        self.app.lock().unwrap().handle_dispatch(dispatch)?;

        // Set the content if provided
        if let Some(content_val) = content {
            let app_guard = self.app.lock().unwrap();
            let comp = app_guard.current_component();
            let context = Context::new(path.clone());

            // Scope the mutable borrow to avoid borrow checker issues
            {
                let mut comp_ref = comp.borrow_mut();
                comp_ref.set_content(&content_val, &context)?;
            }
        }

        // Send success response
        let response = OutputMessageWrapper {
            id,
            message: OutputMessage::Success(true),
            error: None,
        };
        self.send_message_to_vscode(response)?;

        Ok(())
    }

    /// Handle buffer close request from VSCode
    pub fn handle_buffer_close_request(&mut self, id: u64, params: BufferParams) -> Result<()> {
        let BufferParams { uri, .. } = params;
        info!("Buffer closed: uri={}", uri);

        // Convert to CanonicalizedPath
        let _path = match uri_to_path(&uri) {
            Ok(p) => p,
            Err(e) => {
                error!("Failed to convert URI to path: {}: {}", uri, e);
                return Ok(());
            }
        };

        // Close the file in Ki
        // Using CloseCurrentWindow instead of CloseFile
        self.app
            .lock()
            .unwrap()
            .handle_dispatch(Dispatch::CloseCurrentWindow)?;

        // Remove from version tracking
        self.buffer_versions.remove(&uri);

        // Send success response
        let response = OutputMessageWrapper {
            id,
            message: OutputMessage::Success(true),
            error: None,
        };
        self.send_message_to_vscode(response)?;
        Ok(())
    }

    /// Handle buffer save request from VSCode
    pub fn handle_buffer_save_request(&mut self, id: u64, params: BufferParams) -> Result<()> {
        let BufferParams {
            uri: _,
            content: _,
            language_id: _,
            version: _,
            ..
        } = params;
        self.app
            .lock()
            .unwrap()
            .handle_dispatch(Dispatch::ToEditor(DispatchEditor::ForceSave))?;

        // Send success response
        let response = OutputMessageWrapper {
            id,
            message: OutputMessage::Success(true),
            error: None,
        };
        self.send_message_to_vscode(response)?;
        Ok(())
    }

    /// Handle buffer active request from VSCode
    pub fn handle_buffer_active_request(&mut self, _id: u64, params: BufferParams) -> Result<()> {
        let BufferParams { uri, .. } = params;
        let buffer_id = uri.to_string();
        info!("Handling buffer.active: {}", buffer_id);

        // Convert URI to path
        if let Ok(path) = uri_to_path(&uri) {
            // Use the app's dispatch system to focus the file
            info!("Focusing file through dispatch: {:?}", path);

            let focus_result = self
                .app
                .lock()
                .unwrap()
                .handle_dispatch(Dispatch::OpenFile {
                    path: path.clone(),
                    owner: BufferOwner::User,
                    focus: true,
                });

            match focus_result {
                Ok(_) => {
                    info!("Successfully focused file: {:?}", path);

                    // Send cursor position update after buffer is activated
                    // This ensures VSCode has the correct cursor position from the start
                    self.send_cursor_position_for_current_buffer()?;

                    Ok(())
                }
                Err(err) => {
                    error!("Failed to focus file {:?}: {}", path, err);
                    Ok(())
                }
            }
        } else {
            warn!("Failed to convert URI to path: {}", uri);
            Ok(())
        }
    }

    /// Handle buffer change request from VSCode
    pub fn handle_buffer_change_request(&mut self, params: BufferDiffParams) -> anyhow::Result<()> {
        let BufferDiffParams { buffer_id, edits } = params;

        let path = uri_to_path(&buffer_id)?;
        let Some(editor_rc) = self.get_editor_component_by_path(&path) else {
            return Err(anyhow::anyhow!(
                "Buffer not found in Ki layout for change request: {}",
                buffer_id
            ));
        };

        let ki_edits = {
            let editor_borrow = editor_rc.borrow();
            let buffer = editor_borrow.editor().buffer();

            // Convert VS Code edits to Ki Edits
            edits
                .into_iter()
                .map(|diff_edit| -> anyhow::Result<_> {
                    let start_ki_pos = vscode_position_to_ki_position(&diff_edit.range.start);
                    let end_ki_pos = vscode_position_to_ki_position(&diff_edit.range.end);

                    let start_char_index = buffer.position_to_char(start_ki_pos)?;
                    let end_char_index = buffer.position_to_char(end_ki_pos)?;

                    let range = (start_char_index..end_char_index).into();

                    Ok(Edit::new(
                        buffer.rope(),
                        range,
                        Rope::from_str(&diff_edit.new_text),
                    ))
                })
                .collect::<Result<Vec<_>, _>>()?
        };

        let transaction = EditTransaction::from_action_groups(
            // Each edit should be in its own ActionGroup
            // because the edits sent from VS Code are non-offseted.
            ki_edits
                .into_iter()
                .map(|edit| ActionGroup::new([Action::Edit(edit)].to_vec()))
                .collect_vec(),
        );
        let component_id = editor_rc.borrow().id();

        // Ignore the dispatches, as we should not send a buffer updated modification
        // back to VS Code again, otherwise it will be an infinite loop
        let _ = editor_rc
            .borrow_mut()
            .editor_mut()
            .apply_edit_transaction(transaction, &Context::default())?;

        Ok(())
    }
}
