use super::prelude::*;
use crate::{components::component::Component, embed::EmbeddedApp};
use ki_protocol_types::SelectionSetParams;

impl EmbeddedApp {
    /// Handle selection.set notification from Host
    pub(crate) fn handle_selection_set_notification(
        &mut self,
        params: SelectionSetParams,
    ) -> Result<()> {
        let Some(uri) = params.uri else {
            log::info!("EmbeddedApp::handle_selection_set_notification: params.uri is None");
            return Ok(());
        };
        let path = uri_to_path(&uri)?;
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
                    let active = to_ki_position(&selection.active);
                    let anchor = to_ki_position(&selection.anchor);

                    // The sorting is necessary, because `active` might not always be the smaller position
                    // If start is not always smaller, than Swap Primary Cursor with Secondary Cursor will not work properly
                    let start = active.min(anchor);
                    let end = active.max(anchor);

                    let range = buffer.position_range_to_char_index_range(&(start..end))?;
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
        .set_mode(editor.selection_set.mode().clone());

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

        editor.set_selection_set(selection_set, &self.context);

        Ok(())
    }

    pub(crate) fn handle_selection_set_request(
        &mut self,
        params: SelectionSetParams,
    ) -> Result<()> {
        self.handle_selection_set_notification(params)
    }
}

pub(crate) fn to_ki_position(position: &ki_protocol_types::Position) -> Position {
    Position {
        line: position.line as usize,
        column: position.character as usize,
    }
}
