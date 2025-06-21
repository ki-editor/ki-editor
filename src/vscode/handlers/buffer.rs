//! Buffer-related handlers for VSCode IPC messages

use super::prelude::*;
use crate::{
    app::Dispatch,
    buffer::BufferOwner,
    components::editor::DispatchEditor,
    context::Context,
    edit::{Action, ActionGroup, Edit, EditTransaction},
    vscode::{
        app::VSCodeApp,
        utils::{uri_to_path, vscode_position_to_ki_position}, // Use position conversion util
    },
};
use itertools::Itertools;
use ki_protocol_types::{BufferActiveParams, BufferDiffParams, BufferOpenParams, BufferParams};
use log::{error, info}; // Added debug
use ropey::Rope; // Added Rope

impl VSCodeApp {
    /// Handle buffer open request from VSCode
    pub fn handle_buffer_open_request(&mut self, params: BufferOpenParams) -> Result<()> {
        let BufferOpenParams {
            uri,
            content,
            language_id: _,
            version,
            selections,
        } = params;

        let path = uri_to_path(&uri)?;

        // Store the original buffer_id for versioning
        let buffer_id = uri.to_string();

        // Update buffer version
        self.buffer_versions
            .insert(buffer_id.clone(), version.unwrap_or(0) as u64);

        // Open the file
        let dispatch = Dispatch::OpenFile {
            path: path.clone(),
            owner: BufferOwner::User,
            focus: true,
        };
        self.app.lock().unwrap().handle_dispatch(dispatch)?;

        self.handle_selection_set_notification(ki_protocol_types::SelectionSet {
            buffer_id: buffer_id.clone(),
            selections,
            primary: 0,
        })?;

        let app_guard = self.app.lock().unwrap();
        let comp = app_guard.current_component();
        let context = Context::new(path.clone(), true);

        // Scope the mutable borrow to avoid borrow checker issues
        {
            let mut comp_ref = comp.borrow_mut();
            comp_ref.set_content(&content, &context)?;
        }

        Ok(())
    }

    /// Handle buffer close request from VSCode
    pub fn handle_buffer_close_request(&mut self, params: BufferParams) -> Result<()> {
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

        Ok(())
    }

    /// Handle buffer save request from VSCode
    pub fn handle_buffer_save_request(&mut self, params: BufferParams) -> Result<()> {
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

        Ok(())
    }

    /// Handle buffer active request from VSCode
    pub fn handle_buffer_active_request(&mut self, params: BufferActiveParams) -> Result<()> {
        let BufferActiveParams { uri, content, .. } = params;
        let path = uri_to_path(&uri)?;
        self.app
            .lock()
            .unwrap()
            .handle_dispatch(Dispatch::OpenFile {
                path: path.clone(),
                owner: BufferOwner::User,
                focus: true,
            })?;

        // Update the content, this is to prevent buffer desync issues that happens randomly
        let app_guard = self.app.lock().unwrap();
        let comp = app_guard.current_component();
        let context = Context::new(path.clone(), true);

        // Scope the mutable borrow to avoid borrow checker issues
        {
            let mut comp_ref = comp.borrow_mut();
            comp_ref.set_content(&content, &context)?;
        };

        Ok(())
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

        // Ignore the dispatches, as we should not send a buffer updated modification
        // back to VS Code again, otherwise it will be an infinite loop
        let _ = editor_rc
            .borrow_mut()
            .editor_mut()
            .apply_edit_transaction(transaction, &Context::default())?;

        Ok(())
    }
}
