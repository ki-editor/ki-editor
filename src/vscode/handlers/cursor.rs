//! Cursor-related handlers for VSCode IPC messages

use super::prelude::*;
use crate::buffer::BufferOwner;
use crate::components::component::Component;
use crate::vscode::app::VSCodeApp;
use ki_protocol_types::{CursorParams, OutputMessage, OutputMessageWrapper};

impl VSCodeApp {
    /// Check for cursor changes and send updates if needed
    pub fn check_for_cursor_changes(&mut self) -> Result<bool> {
        // Prevent feedback loop: only send update if not suppressed
        if self.suppress_next_cursor_update {
            self.suppress_next_cursor_update = false;
            return Ok(false);
        }
        let current_path = self.get_current_file_path();
        if current_path.is_none() {
            trace!("No active file path, skipping cursor check.");
            return Ok(false); // No active file path
        }

        let component = self.app.lock().unwrap().current_component();
        let component_ref = component.borrow();
        let editor = component_ref.editor();
        let buffer = editor.buffer(); // Get buffer from the editor instance

        // Get all selections from the editor
        let ki_selections = editor.selection_set.selections(); // Access via selection_set field

        if !ki_selections.is_empty() {
            let mut anchors = Vec::with_capacity(ki_selections.len());
            let mut actives = Vec::with_capacity(ki_selections.len());

            for selection in ki_selections {
                // Determine anchor and active character indices from the internal selection
                // Use the logic similar to ki_selection_to_vscode_selection
                let active_char_index = selection.range().end;
                let anchor_char_index = selection
                    .initial_range
                    .map_or(selection.range().start, |r| r.start);

                // Convert char indices to internal Ki Positions
                let Ok(ki_active_pos) = buffer.char_to_position(active_char_index) else {
                    warn!(
                        "Failed to convert active char index {:?} to position",
                        active_char_index
                    );
                    continue; // Skip this selection if conversion fails
                };
                let Ok(ki_anchor_pos) = buffer.char_to_position(anchor_char_index) else {
                    warn!(
                        "Failed to convert anchor char index {:?} to position",
                        anchor_char_index
                    );
                    continue; // Skip this selection if conversion fails
                };

                // Convert Ki Positions to VSCode Positions
                anchors.push(ki_position_to_vscode_position(&ki_anchor_pos));
                actives.push(ki_position_to_vscode_position(&ki_active_pos));
            }

            // Only send if we successfully processed at least one selection
            if !anchors.is_empty() {
                info!(
                    "Sending cursor update with {} anchors and {} actives.",
                    anchors.len(),
                    actives.len()
                );

                // Create properly typed notification parameters
                let cursor_params = CursorParams {
                    buffer_id: self.get_current_file_uri().unwrap(),
                    anchors, // Use the collected anchors
                    actives, // Use the collected actives
                };

                let notification = OutputMessageWrapper {
                    id: 0, // Use ID 0 for notifications
                    message: OutputMessage::CursorUpdate(cursor_params),
                    error: None,
                };

                self.send_message_to_vscode(notification)?;
                return Ok(true); // Indicate that an update was sent
            } else {
                warn!("Could not convert any editor selections to VSCode positions.");
                return Ok(false); // No valid selections to send
            }
        } else {
            trace!("No selections found in editor, not sending cursor update.");
        }
        Ok(false) // No update sent
    }

    // Suppression flag to prevent feedback loop when applying backend-driven updates
    // This should be a field on VSCodeApp if not already present
    // Example: self.suppress_next_cursor_update: bool

    /// Handle cursor update request
    pub fn handle_cursor_update_request(&mut self, id: u64, params: CursorParams) -> Result<()> {
        info!(
            "Received cursor.update request for buffer: {}",
            params.buffer_id
        );

        // Get the buffer ID and position from the params
        let buffer_id = params.buffer_id.clone();
        // Use the 'actives' field for cursor positions
        let active_positions = params.actives.clone();

        // Important: Get the file path first, before any borrowing happens
        let file_path = match uri_to_path(&buffer_id) {
            Some(path) => path,
            None => return Err(anyhow!("Invalid URI: {}", buffer_id)),
        };

        // Get current file path first
        let current_file = self.get_current_file_path();
        let different_file = current_file
            .as_ref()
            .map_or(true, |path| *path != file_path);

        // If we need to switch files, do that first
        if different_file {
            info!(
                "Cursor update for different file. Current file: {}, Update for: {}",
                current_file.map_or("None".to_string(), |p| p
                    .as_ref()
                    .to_string_lossy()
                    .to_string()),
                file_path.as_ref().to_string_lossy()
            );

            // Open the file using dispatch with correct field names
            if let Err(e) =
                self.app
                    .lock()
                    .unwrap()
                    .handle_dispatch(crate::app::Dispatch::OpenFile {
                        path: file_path.clone(),
                        owner: BufferOwner::User,
                        focus: true,
                    })
            {
                error!("Failed to open file: {}", e);
                return Err(anyhow!("Failed to open file: {}", e));
            }
        }

        // Now get the component after potentially switching files
        let component = self.app.lock().unwrap().current_component();
        let mut component_ref = component.borrow_mut();
        let editor = component_ref.editor_mut();

        // Prevent feedback loop: set suppression flag before applying update
        self.suppress_next_cursor_update = true;
        // Apply cursor update - make sure we handle only the first cursor for now
        if let Some(first_active) = active_positions.first() {
            let vscode_position = first_active.clone();
            info!(
                "Setting cursor position from VSCode: Line {}, Char {}",
                vscode_position.line, vscode_position.character
            );

            // Get the context needed for set_cursor_position
            let app_guard = self.app.lock().unwrap();
            let context = app_guard.context();

            if let Err(e) = editor.set_cursor_position(
                vscode_position.line as u16,
                vscode_position.character as u16,
                &context,
            ) {
                error!("Failed to set cursor position: {}", e);
                drop(app_guard); // Explicitly drop the guard
                return Err(anyhow!("Failed to set cursor position: {}", e));
            }

            // Release the app lock before the end of this branch
            drop(app_guard);
        } else {
            info!("No cursor positions in update");
            return Ok(());
        }

        // Send success response
        let message = OutputMessage::Success(true);
        let wrapper = OutputMessageWrapper {
            id,
            message,
            error: None,
        };
        self.send_message_to_vscode(wrapper)?;

        Ok(())
    }

    /// Handle cursor get request from VSCode
    pub fn handle_cursor_get_request(&mut self, id: u64) -> Result<()> {
        info!("Received cursor.get request");

        // Get the current file path
        let buffer_id = match self.get_current_file_uri() {
            Some(uri) => uri,
            None => {
                error!("Failed to get current file URI");

                // Send error response
                let response = OutputMessageWrapper {
                    id,
                    message: OutputMessage::Error("No active buffer".to_string()),
                    error: Some(ResponseError {
                        code: -1,
                        message: "No active buffer".to_string(),
                        data: None,
                    }),
                };
                self.send_message_to_vscode(response)?;

                return Err(anyhow!("No active buffer"));
            }
        };

        // Get the current component
        let component = self.app.lock().unwrap().current_component();
        let component_borrowed = component.borrow();
        let editor = component_borrowed.editor();

        // Get the cursor position
        let position = editor.get_cursor_position();

        if let Ok(position) = position {
            // Convert Ki position to VSCode position
            let vscode_position = ki_protocol_types::Position {
                line: position.line,
                character: position.column,
            };

            // Create response message
            let cursor_params = CursorParams {
                buffer_id,
                // For now, assume single cursor; anchor and active are the same
                anchors: vec![vscode_position.clone()],
                actives: vec![vscode_position],
            };

            let response = OutputMessageWrapper {
                id,
                message: OutputMessage::CursorUpdate(cursor_params),
                error: None,
            };

            // Send response
            self.send_message_to_vscode(response)?;

            Ok(())
        } else {
            // Handle error case
            let error_msg = "Failed to get cursor position";
            error!("{}", error_msg);

            // Send error response
            let response = OutputMessageWrapper {
                id,
                message: OutputMessage::Error(error_msg.to_string()),
                error: Some(ResponseError {
                    code: -1,
                    message: error_msg.to_string(),
                    data: None,
                }),
            };
            self.send_message_to_vscode(response)?;

            Err(anyhow!(error_msg))
        }
    }
}
