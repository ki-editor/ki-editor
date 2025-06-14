//! Editor action handlers for VSCode IPC messages

use super::prelude::*;
use crate::vscode::app::VSCodeApp;
use ki_protocol_types::{EditorAction, EditorActionParams};

impl VSCodeApp {
    /// Handle editor.action request
    pub fn handle_editor_action_request(
        &self,
        id: u64,
        params: EditorActionParams,
        trace_id: &str,
    ) -> Result<()> {
        debug!(
            "[{}] Processing editor action: {:?}",
            trace_id, params.action
        );

        // First, send the success response immediately to prevent VSCode from timing out
        // This ensures VSCode knows the command was received and is being processed
        debug!(
            "[{}] Sending success response before processing editor action",
            trace_id
        );
        // Create a dispatch directly for the action
        // This is much more robust than using key events, which depend on keyboard layout
        let dispatch = match params.action {
            EditorAction::Undo => {
                info!("[{}] Creating dispatch for undo", trace_id);
                crate::app::Dispatch::ToEditor(crate::components::editor::DispatchEditor::Undo)
            }
            EditorAction::Redo => {
                info!("[{}] Creating dispatch for redo", trace_id);
                crate::app::Dispatch::ToEditor(crate::components::editor::DispatchEditor::Redo)
            }
            EditorAction::Save => {
                info!("[{}] Creating dispatch for save", trace_id);
                crate::app::Dispatch::ToEditor(crate::components::editor::DispatchEditor::Save)
            }
            EditorAction::ForceSave => {
                info!("[{}] Creating dispatch for force save", trace_id);
                crate::app::Dispatch::ToEditor(crate::components::editor::DispatchEditor::ForceSave)
            }
            EditorAction::Copy => {
                info!("[{}] Creating dispatch for copy", trace_id);
                crate::app::Dispatch::ToEditor(crate::components::editor::DispatchEditor::Copy {
                    use_system_clipboard: true,
                })
            }
            EditorAction::Cut => {
                info!("[{}] Creating dispatch for cut", trace_id);
                crate::app::Dispatch::ToEditor(
                    crate::components::editor::DispatchEditor::ChangeCut {
                        use_system_clipboard: true,
                    },
                )
            }
            EditorAction::Paste => {
                info!("[{}] Creating dispatch for paste", trace_id);
                // This would need to be handled differently as it requires clipboard content
                // For now, we'll just log a warning
                warn!("[{}] Paste action not fully implemented yet", trace_id);
                return self.send_error_response(id, "Paste action not fully implemented yet");
            }
            EditorAction::SelectAll => {
                info!("[{}] Creating dispatch for select all", trace_id);
                crate::app::Dispatch::ToEditor(crate::components::editor::DispatchEditor::SelectAll)
            }
        };

        // Create an AppMessage::ExternalDispatch to send to the App thread
        let app_message = crate::app::AppMessage::ExternalDispatch(dispatch);

        // Send the message to the App thread via the app_sender
        // This will be processed in the main event loop without blocking
        info!(
            "[{}] Sending dispatch for action '{:?}' via app_sender",
            trace_id, params.action
        );

        self.app_sender.send(app_message)?;

        Ok(())
    }
}
