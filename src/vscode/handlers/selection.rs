//! Selection-related handlers for VSCode IPC messages

use super::prelude::*;
use crate::vscode::VSCodeApp;
use ki_protocol_types::{OutputMessage, SelectionSet};

impl VSCodeApp {
    // The check_for_selection_changes method has been removed as it's no longer used.
    // Selection updates are now sent via the IntegrationEvent::SelectionChanged event.

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

    // The handle_selection_get_request method has been removed as it's no longer used.
    // Selection information is now sent via the IntegrationEvent::SelectionChanged event.

    /// Handle selection.set request
    pub fn handle_selection_set_request(&mut self, id: u64, params: SelectionSet) -> Result<()> {
        // Pass the ID to the notification handler so it knows to send a response
        self.handle_selection_set_notification(params, Some(id))
    }
}
