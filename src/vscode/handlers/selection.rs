//! Selection-related handlers for VSCode IPC messages

use super::prelude::*;
use crate::{components::component::Component, vscode::VSCodeApp};
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
        let path = uri_to_path(&params.buffer_id)?;
        let Some(editor) = self.app.lock().unwrap().get_editor_by_file_path(&path) else {
            return Err(anyhow::anyhow!(
                "Editor not found for path: {}",
                path.clone().display_absolute()
            ));
        };
        let mut editor_ref = editor.borrow_mut();
        let editor = editor_ref.editor_mut();
        let ki_selections = {
            let buffer = editor.buffer();
            params
                .selections
                .into_iter()
                .map(|selection| {
                    let range = buffer.position_range_to_char_index_range(
                        &(to_ki_position(&selection.active)..to_ki_position(&selection.anchor)),
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
        }
        .set_mode(editor.selection_set.mode.clone());

        // Skip setting selection if the extended ranges of both selection sets are the same.
        // This is necessary so that selection extension can work.
        // Because from the VS Code side,
        // selection changes due to mouse or custom commands will be sent to Ki.
        //
        // However, the selection changes sent from Ki to VS Code are from custom commands,
        // so when VS Code receives the selection changes from Ki, it will send back the
        // received selections to Ki.
        //
        // We need to send selection changes caused by custom commands so that
        // actions like LSP Go to Definition can work properly, otherwise
        // once we reach the definition, pressing any movement will send the cursor
        // to the first line of the file, as if the cursor was not on the definition before,
        // since Ki was not notified.
        //
        // This hack cannot be removed until we figure out how to distinguish
        // Ki-iniated selection changes from LSP-initiated selection changes via VS Code Extension API.
        {
            let current_extended_ranges = editor.selection_set.map(|s| s.extended_range());
            let new_extended_ranges = selection_set.map(|s| s.extended_range());
            if current_extended_ranges == new_extended_ranges {
                log::info!("Skipping setting selection as the extended ranges are the same");
                return Ok(());
            }
        }

        editor.set_selection_set(selection_set, &Context::default());

        if let Some(response_id) = id {
            self.send_response(response_id, OutputMessage::Success(true))?;
        };

        Ok(())
    }

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
