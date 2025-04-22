//! Selection-related handlers for VSCode IPC messages

use super::prelude::*;
use crate::components::component::Component;
use crate::components::editor::Movement;
use crate::vscode::VSCodeApp;
use ki_protocol_types::Selection;
use ki_protocol_types::{OutputMessage, OutputMessageWrapper, SelectionSet};

impl VSCodeApp {
    /// Send selection updates if changed
    pub fn check_for_selection_changes(&mut self) -> Result<()> {
        let id = self.next_id();

        let buffer_id = match self.get_current_file_uri() {
            Some(uri) => uri,
            None => return Ok(()), // No active buffer
        };

        // Get current selections from App
        let (current_selections, primary_selection_index) = {
            let app_guard = self.app.lock().unwrap();
            let component = app_guard.current_component();
            let component_ref = component.borrow();
            let editor = component_ref.editor();
            let buffer = editor.buffer();
            let selection_mode = &editor.selection_set.mode;
            let movement = Movement::Right;
            let current_path = self.get_current_file_path();
            let context = if let Some(ref path) = current_path {
                // CanonicalizedPath is constructed via TryFrom<PathBuf>
                let canonical_path = path.clone();
                Context::new(canonical_path)
            } else {
                Context::default()
            };
            if let Ok(Some(selection_set)) =
                editor.get_selection_set(selection_mode, movement, &context)
            {
                let selections: Vec<Selection> = selection_set
                    .selections
                    .iter()
                    .map(|sel| {
                        // Assume extended_range start/end map to anchor/active for now
                        let anchor_idx = sel.extended_range().start;
                        let active_idx = sel.extended_range().end;
                        let anchor_pos = buffer.char_to_position(anchor_idx).unwrap();
                        let active_pos = buffer.char_to_position(active_idx).unwrap();
                        // is_extended seems to track if an initial range was set
                        let is_extended = sel.initial_range.is_some();
                        Selection {
                            anchor: ki_position_to_vscode_position(&anchor_pos),
                            active: ki_position_to_vscode_position(&active_pos),
                            is_extended,
                        }
                    })
                    .collect();
                (selections, selection_set.cursor_index)
            } else if let Ok(pos) = editor.get_cursor_position() {
                // If no selection set, send the cursor position as anchor and active
                let vscode_pos = ki_position_to_vscode_position(&pos);
                (
                    vec![Selection {
                        anchor: vscode_pos.clone(),
                        active: vscode_pos,
                        is_extended: false,
                    }],
                    0,
                )
            } else {
                (vec![], 0)
            }
        };

        let params = SelectionSet {
            buffer_id: buffer_id.clone(),
            selections: current_selections.clone(),
            primary: primary_selection_index,
        };

        // DEBUG: Always send selection update for now
        info!(
            "[Ki→VSCode] Sending selection.update: buffer_id={}, selections={:?}, primary={}",
            buffer_id, current_selections, primary_selection_index
        );
        self.send_message_to_vscode(OutputMessageWrapper {
            id,
            message: OutputMessage::SelectionUpdate(params),
            error: None,
        })?;

        Ok(())
    }

    /// Handle selection.set notification from VSCode
    /// Note: This handler does NOT call check_for_changes() afterwards to prevent feedback loops
    pub fn handle_selection_set_notification(
        &mut self,
        params: SelectionSet,
        id: Option<u64>,
    ) -> Result<()> {
        // --- Suppression Check ---
        // This check is ONLY for cursor updates *initiated by Ki*.
        if self.suppress_next_cursor_update {
            // Check the flag
            info!(
                "Selection change suppressed due to internal update (ID: {:?}).",
                id
            );
            self.suppress_next_cursor_update = false; // Clear the flag

            // If this was a REQUEST (not a notification), we still need to reply
            // Send success even if suppressed, otherwise the request times out.
            if let Some(req_id) = id {
                // Use send_response helper which wraps the message
                match self.send_response(req_id, OutputMessage::Success(true)) {
                    Ok(_) => trace!("Sent suppressed success response for ID: {}", req_id),
                    Err(e) => error!(
                        "Failed to send suppressed success response for ID {}: {}",
                        req_id, e
                    ),
                }
            }
            return Ok(()); // Don't process further, don't send to Ki
        }

        // If not suppressed, process the selection change from VSCode
        trace!("Handling selection set update.");

        info!(
            "Received selection.set notification with id {}",
            id.unwrap_or(0)
        );

        info!(
            "Selection set: buffer_id={}, selections={}",
            params.buffer_id,
            params.selections.len()
        );

        // Process the selections - for now just log them
        for (i, sel) in params.selections.iter().enumerate() {
            debug!(
                "Selection {}: anchor=({},{}) active=({},{})",
                i, sel.anchor.line, sel.anchor.character, sel.active.line, sel.active.character
            );
        }

        // Update the selection set in the editor
        let current_path = self.get_current_file_path();
        if let Some(ref path) = current_path {
            let formatted_path = path_to_uri(path);

            if formatted_path == params.buffer_id {
                let component = self.app.lock().unwrap().current_component();
                let mut component_ref = component.borrow_mut();
                let editor = component_ref.editor_mut();
                if let Some(first_sel) = params.selections.first() {
                    let context = if let Some(ref path) = current_path {
                        let canonical_path = path.clone();
                        Context::new(canonical_path)
                    } else {
                        Context::default()
                    };
                    let active_vscode_pos = &first_sel.active;
                    let ki_position =
                        Position::new(active_vscode_pos.line, active_vscode_pos.character);

                    info!(
                        "Setting cursor from primary selection active: Line {}, Char {}",
                        ki_position.line, ki_position.column
                    );

                    if let Err(e) = editor.set_cursor_position(
                        ki_position.line as u16,
                        ki_position.column as u16,
                        &context,
                    ) {
                        error!("Failed to set cursor position from selection: {}", e);
                        // Optionally send error response if id is present
                        if let Some(response_id) = id {
                            self.send_error_response(
                                response_id,
                                &format!("Failed to set cursor: {}", e),
                            )?;
                        }
                        // Don't return Ok, let the error propagate if needed, or handle differently
                        // For a notification, maybe just log and continue?
                    } else {
                        // TODO: Handle extended selection range setting using anchor and active
                        if first_sel.is_extended {
                            let anchor_vscode_pos = &first_sel.anchor;
                            warn!(
                                "Extended selection handling not implemented yet. Anchor: {:?}, Active: {:?}",
                                anchor_vscode_pos, active_vscode_pos
                            );
                            // Example (needs Ki Editor API):
                            // let ki_anchor = Position::new(anchor_vscode_pos.line, anchor_vscode_pos.character);
                            // editor.set_selection_range(ki_anchor, ki_position, &context)?;
                        }

                        // Send success response only if it was a request (id is Some)
                        if let Some(response_id) = id {
                            // Use send_response with OutputMessage::Success
                            self.send_response(response_id, OutputMessage::Success(true))?;
                        }
                    }
                }
            }
        }
        // Track the last selection received from VSCode
        self.last_vscode_selection = Some(params);

        // IMPORTANT: We do NOT call check_for_changes() here to prevent feedback loops
        // where Ki sends back a selection.update immediately after receiving a selection.set

        Ok(())
    }

    /// Handle selection get request from VSCode
    pub fn handle_selection_get_request(&mut self, id: u64) -> Result<()> {
        let current_path = self.get_current_file_path();
        info!(
            "Handling selection get request: id={}, current_file={:?}",
            id, current_path
        );

        let component = self.app.lock().unwrap().current_component();
        let component_ref = component.borrow();
        let editor = component_ref.editor();

        // Format the path for VSCode
        let current_path = self.get_current_file_path();

        if let Some(ref path) = current_path {
            let formatted_path = path_to_uri(path);
            // No params in this function, just use formatted_path
            let buffer_id = &formatted_path;

            // If a specific buffer_id was requested, check if it matches the current buffer
            if !buffer_id.is_empty() && *buffer_id != formatted_path {
                info!(
                    "Requested buffer {} doesn't match current buffer {}",
                    buffer_id, formatted_path
                );

                // Return error for mismatched buffer
                self.send_message_to_vscode(OutputMessageWrapper {
                    id,
                    message: OutputMessage::Error(format!(
                        "Requested buffer not active: {} (current: {})",
                        buffer_id, formatted_path
                    )),
                    error: Some(ResponseError {
                        code: 5,
                        message: format!(
                            "Requested buffer not active: {} (current: {})",
                            buffer_id, formatted_path
                        ),
                        data: None,
                    }),
                })?;
                return Ok(());
            }

            // For now, create a selection based on the cursor position
            // In the future, this could be expanded to handle multiple selections
            if let Ok(position) = editor.get_cursor_position() {
                let vscode_position = ki_position_to_vscode_position(&position);

                info!(
                    "Sending selection at position: ({}, {}) for buffer {}",
                    position.line, position.column, formatted_path
                );

                // Create a selection that's just the cursor position (anchor = active)
                // TODO: Get actual anchor/active from editor if possible
                let selections = vec![Selection {
                    anchor: vscode_position.clone(),
                    active: vscode_position,
                    is_extended: false, // Assuming not extended if only cursor known
                }];

                // Create and send the response
                // Create a properly typed response
                let selection_result = SelectionSet {
                    buffer_id: formatted_path,
                    selections,
                    primary: 0,
                };

                let response = OutputMessageWrapper {
                    id,
                    message: OutputMessage::SelectionUpdate(selection_result),
                    error: None,
                };
                self.send_message_to_vscode(response)?;
            } else {
                // Couldn't get cursor position for selection
                self.send_message_to_vscode(OutputMessageWrapper {
                    id,
                    message: OutputMessage::Error(format!("Failed to get selection")),
                    error: Some(ResponseError {
                        code: 3,
                        message: "Failed to get selection".to_string(),
                        data: None,
                    }),
                })?;
            }
        } else {
            // No file path available
            self.send_message_to_vscode(OutputMessageWrapper {
                id,
                message: OutputMessage::Error(format!("No active buffer")),
                error: Some(ResponseError {
                    code: 4,
                    message: "No active buffer".to_string(),
                    data: None,
                }),
            })?;
        }

        Ok(())
    }

    /// Handle selection.set request
    pub fn handle_selection_set_request(&mut self, id: u64, params: SelectionSet) -> Result<()> {
        // Pass the ID to the notification handler so it knows to send a response
        self.handle_selection_set_notification(params, Some(id))
    }
}
