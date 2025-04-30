//! Buffer-related handlers for VSCode IPC messages

use super::prelude::*;
use crate::{
    app::Dispatch,
    buffer::BufferOwner,
    components::editor::DispatchEditor,
    context::Context,
    edit::{Action, ActionGroup, Edit, EditTransaction}, // Added Edit types
    vscode::{
        app::VSCodeApp,
        utils::{uri_to_path, vscode_position_to_ki_position}, // Use position conversion util
    },
};
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

        // Convert URI to path
        if let Ok(path) = uri_to_path(&uri) {
            // Store the original buffer_id for versioning
            let buffer_id = uri.to_string();

            // Update buffer version
            self.buffer_versions
                .insert(buffer_id.clone(), version.unwrap_or(0) as u64);

            // First check if the file is already open
            let current_path = self.get_current_file_path();
            if current_path.as_ref() == Some(&path) {
                info!("Requested file is already open");
            } else {
                // Open the file
                let dispatch = Dispatch::OpenFile {
                    path: path.clone(),
                    owner: BufferOwner::User,
                    focus: true,
                };
                let open_result = self.app.lock().unwrap().handle_dispatch(dispatch);

                if let Err(e) = open_result {
                    error!("Failed to open file {}: {}", uri, e);
                    // Create the error response
                    let error_message = format!("Failed to open file: {}", e);
                    let response = OutputMessageWrapper {
                        id,
                        message: OutputMessage::Error(error_message.clone()),
                        error: Some(ResponseError {
                            code: 1,
                            message: error_message,
                            data: None,
                        }),
                    };
                    self.send_message_to_vscode(response)?;
                    return Ok(());
                }
            }

            // Set the content if provided
            if let Some(content_val) = content {
                let app_guard = self.app.lock().unwrap();
                let comp = app_guard.current_component();
                let context = Context::new(path.clone());

                // Scope the mutable borrow to avoid borrow checker issues
                {
                    let mut comp_ref = comp.borrow_mut();
                    if let Err(e) = comp_ref.set_content(&content_val, &context) {
                        error!("Failed to set buffer content: {}", e);
                        // Release the app lock before sending the error response
                        drop(app_guard);

                        // Create error message outside any borrows
                        let error_message = format!("Failed to set buffer content: {}", e);
                        let response = OutputMessageWrapper {
                            id,
                            message: OutputMessage::Error(error_message.clone()),
                            error: Some(ResponseError {
                                code: 2,
                                message: error_message,
                                data: None,
                            }),
                        };
                        self.send_message_to_vscode(response)?;
                        return Ok(());
                    }
                }
            }

            // Send success response
            let response = OutputMessageWrapper {
                id,
                message: OutputMessage::Success(true),
                error: None,
            };
            self.send_message_to_vscode(response)?;

            // Send cursor position update after buffer is opened
            // This ensures VSCode has the correct cursor position from the start
            self.send_cursor_position_for_current_buffer()?;

            Ok(())
        } else {
            error!("Failed to convert URI to path: {}", uri);
            // Send error response for bad URI
            let error_message = format!("Failed to convert URI to path: {}", uri);
            let response = OutputMessageWrapper {
                id,
                message: OutputMessage::Error(error_message.clone()),
                error: Some(ResponseError {
                    code: 3,
                    message: error_message,
                    data: None,
                }),
            };
            self.send_message_to_vscode(response)?;
            Ok(())
        }
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
    /// This is treated as a notification (_id might not be relevant for response)
    pub fn handle_buffer_change_request(
        &mut self,
        _id: u64,                 // Typically unused for notifications/updates like this
        params: BufferDiffParams, // Use BufferDiffParams
    ) -> Result<()> {
        let BufferDiffParams { buffer_id, edits } = params;

        debug!(
            "Handling buffer change request: buffer_id={}, edits_count={}",
            buffer_id,
            edits.len()
        );

        // Convert buffer_id (URI) to path
        if let Ok(path) = uri_to_path(&buffer_id) {
            // Find the corresponding editor/buffer in the app's layout
            // Use the helper method to get the editor component
            if let Some(editor_rc) = self.get_editor_component_by_path(&path) {
                // Create a scope for the editor borrow to ensure it's dropped before we dispatch
                let ki_edits_result: Result<Vec<Edit>, _> = {
                    let editor_borrow = editor_rc.borrow();
                    let buffer = editor_borrow.editor().buffer();
                    // let context = app_lock.context(); // Context not needed here

                    // Convert protocol edits to Ki Edits
                    edits
                        .into_iter()
                        .map(|diff_edit| {
                            // Convert VSCode Position to Ki Position
                            let start_ki_pos =
                                vscode_position_to_ki_position(&diff_edit.range.start);
                            let end_ki_pos = vscode_position_to_ki_position(&diff_edit.range.end);

                            // Convert Ki Position to Ki CharIndex, handling potential errors
                            let start_char_index = buffer.position_to_char(start_ki_pos)?;
                            let end_char_index = buffer.position_to_char(end_ki_pos)?;

                            let range = (start_char_index..end_char_index).into();

                            Ok::<Edit, anyhow::Error>(Edit::new(
                                buffer.rope(), // Pass rope reference
                                range,
                                Rope::from_str(&diff_edit.new_text),
                            ))
                        })
                        .collect()
                    // editor_borrow is dropped at the end of this scope
                };

                match ki_edits_result {
                    Ok(ki_edits) => {
                        if ki_edits.is_empty() {
                            debug!("No actual edits to apply for buffer {}", buffer_id);
                            return Ok(()); // Nothing to do
                        }

                        // Create an EditTransaction
                        let transaction =
                            EditTransaction::from_action_groups(vec![ActionGroup::new(
                                ki_edits.into_iter().map(Action::Edit).collect(),
                            )]);

                        // Get the number of edits for logging
                        let num_edits = transaction.edits().len();

                        // Dispatch the transaction to the specific editor component
                        let component_id = editor_rc.borrow().id();
                        // Use ApplyEditTransaction variant
                        let dispatch = Dispatch::ToEditor(DispatchEditor::ApplyEditTransaction {
                            transaction,
                            component_id,            // Target the specific editor
                            reparse_tree: true,      // Assume reparse needed
                            update_undo_stack: true, // Assume undo needed
                        });

                        info!(
                            "Dispatching ApplyEditTransaction for buffer {} with {} edits",
                            buffer_id, num_edits
                        );

                        // Re-lock app to dispatch
                        if let Err(e) = self.app.lock().unwrap().handle_dispatch(dispatch) {
                            error!(
                                "Failed to dispatch ApplyTransaction for buffer {}: {}",
                                buffer_id, e
                            );
                            // Consider sending an error back to VSCode if needed
                        }
                    }
                    Err(e) => {
                        error!(
                            "Failed to convert VSCode edits to Ki edits for buffer {}: {}",
                            buffer_id, e
                        );
                        // Consider sending an error back to VSCode
                    }
                }
            } else {
                warn!(
                    "Buffer not found in Ki layout for change request: {}",
                    buffer_id
                );
                // Buffer might be closed in Ki but open in VSCode, or not yet opened by Ki
            }
            Ok(())
        } else {
            error!(
                "Failed to convert URI to path for change request: {}",
                buffer_id
            );
            Err(anyhow!(
                "Invalid URI for buffer change request: {}",
                buffer_id
            ))
        }
    }
}
