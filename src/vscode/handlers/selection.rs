//! Selection-related handlers for VSCode IPC messages

use super::prelude::*;
use crate::{
    components::component::{self, Component},
    vscode::VSCodeApp,
};
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

        log::info!("xxx selection_set = {selection_set:?}");

        editor.set_selection_set(selection_set, &Context::default());

        if let Some(response_id) = id {
            // Use send_response with OutputMessage::Success
            self.send_response(response_id, OutputMessage::Success(true))?;
        };

        return Ok(());
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
