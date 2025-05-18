//! Selection-related handlers for VSCode IPC messages

use super::prelude::*;
use crate::vscode::VSCodeApp;
use itertools::Itertools;
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
                let ki_selections = {
                    let buffer = editor.buffer();
                    params
                        .selections
                        .into_iter()
                        .map(|selection| {
                            let range = buffer.position_range_to_char_index_range(
                                &(to_ki_position(&selection.active)
                                    ..to_ki_position(&selection.anchor)),
                            )?;
                            Ok(crate::selection::Selection::new(range))
                        })
                        .collect::<anyhow::Result<Vec<_>>>()?
                };

                let selection_set = match ki_selections.split_first() {
                    Some((head, tail)) => crate::selection::SelectionSet::new(nonempty::NonEmpty {
                        head: head.clone(),
                        tail: tail.to_vec(),
                    }),
                    None => return Ok(()),
                };
                editor.set_selection_set(selection_set, &Context::default());
                if let Some(response_id) = id {
                    // Use send_response with OutputMessage::Success
                    self.send_response(response_id, OutputMessage::Success(true))?;
                }
            }
        }
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

pub(crate) fn to_ki_position(position: &ki_protocol_types::Position) -> Position {
    Position {
        line: position.line,
        column: position.character,
    }
}
