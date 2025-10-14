//! Buffer-related handlers for Host-Ki IPC messages

use super::prelude::*;
use crate::{
    app::Dispatch,
    buffer::BufferOwner,
    edit::{Action, ActionGroup, Edit, EditTransaction},
    embed::{
        app::EmbeddedApp,
        utils::{host_position_to_ki_position, uri_to_path},
    },
};
use itertools::Itertools;
use ki_protocol_types::{
    BufferDiffParams, BufferOpenParams, BufferParams, SyncBufferResponseParams,
};
use ropey::Rope;

impl EmbeddedApp {
    /// Handle buffer open request from Host
    pub(crate) fn handle_buffer_open_request(&mut self, params: BufferOpenParams) -> Result<()> {
        let BufferOpenParams {
            uri,
            content,
            selections,
        } = params;

        let path = uri_to_path(&uri)?;

        // Open the file
        let dispatch = Dispatch::OpenFile {
            path: path.clone(),
            owner: BufferOwner::User,
            focus: true,
        };
        self.app.lock().unwrap().handle_dispatch(dispatch)?;

        self.handle_selection_set_notification(ki_protocol_types::SelectionSet {
            uri: Some(uri.to_string()),
            selections,
        })?;

        let app_guard = self.app.lock().unwrap();
        let comp = app_guard.current_component();

        // Scope the mutable borrow to avoid borrow checker issues
        {
            let mut comp_ref = comp.borrow_mut();
            comp_ref.set_content(&content, &self.context)?;
        }

        Ok(())
    }

    /// Handle buffer active request from Host
    pub(crate) fn handle_buffer_active_request(&mut self, params: BufferParams) -> Result<()> {
        let path = uri_to_path(&params.uri)?;
        self.app
            .lock()
            .unwrap()
            .handle_dispatch(Dispatch::OpenFile {
                path: path.clone(),
                owner: BufferOwner::User,
                focus: true,
            })?;

        Ok(())
    }

    /// Handle buffer change request from Host
    pub(crate) fn handle_buffer_change_request(
        &mut self,
        params: BufferDiffParams,
    ) -> anyhow::Result<()> {
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
                    let start_ki_pos = host_position_to_ki_position(&diff_edit.range.start);
                    let end_ki_pos = host_position_to_ki_position(&diff_edit.range.end);

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

        // Ignore the dispatches, as we should not send a buffer updated modification
        // back to VS Code again, otherwise it will be an infinite loop
        let _ = editor_rc
            .borrow_mut()
            .editor_mut()
            .apply_edit_transaction(transaction, &self.context)?;

        Ok(())
    }

    pub(crate) fn handle_sync_buffer_response(
        &self,
        params: SyncBufferResponseParams,
    ) -> std::result::Result<(), anyhow::Error> {
        let SyncBufferResponseParams { uri, content, .. } = params;
        let path = uri_to_path(&uri)?;
        self.app
            .lock()
            .unwrap()
            .handle_dispatch(Dispatch::OpenFile {
                path: path.clone(),
                owner: BufferOwner::User,
                focus: false,
            })?;

        // Update the content, this is to prevent buffer desync issues that happens randomly
        let mut app_guard = self.app.lock().unwrap();
        let comp = app_guard.current_component();
        // Scope the mutable borrow to avoid borrow checker issues
        {
            let mut comp_ref = comp.borrow_mut();
            let dispatches = comp_ref
                .editor_mut()
                .update_content(&content, &self.context)?;
            app_guard.handle_dispatches(dispatches)?;
        };

        for event in app_guard.take_queued_events() {
            app_guard.handle_event(event)?;
        }

        Ok(())
    }
}
