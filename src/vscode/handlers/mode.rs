//! Mode change handlers for VSCode IPC messages

use crate::components::editor::Mode as KiMode;
use crate::vscode::app::VSCodeApp;
use anyhow::Result;
use ki_protocol_types::{EditorMode, OutputMessage, TypedModeParams};
use log::{debug, error, info};

impl VSCodeApp {
    /// Handle mode set request from VSCode
    pub fn handle_mode_set_request(
        &mut self,
        id: u64,
        params: TypedModeParams,
        trace_id: &str,
    ) -> Result<()> {
        info!(
            "[{}] Received mode.set request: {:?}",
            trace_id, params.mode
        );

        // Convert protocol mode to Ki mode
        let ki_mode = match params.mode {
            EditorMode::Normal => KiMode::Normal,
            EditorMode::Insert => KiMode::Insert,
            EditorMode::MultiCursor => KiMode::MultiCursor,
            EditorMode::FindOneChar => {
                KiMode::FindOneChar(crate::components::editor::IfCurrentNotFound::LookForward)
            }
            EditorMode::Swap => KiMode::Swap,
            EditorMode::Replace => KiMode::Replace,
            EditorMode::Extend => KiMode::Extend,
        };

        debug!("[{}] Setting mode to: {:?}", trace_id, ki_mode);

        // First, send the success response immediately to prevent VSCode from timing out
        debug!(
            "[{}] Sending success response before processing mode set",
            trace_id
        );
        let response_result = self.send_response(id, OutputMessage::Success(true));
        if let Err(e) = response_result {
            error!("[{}] Failed to send success response: {}", trace_id, e);
            return Err(e);
        }

        // Create a dispatch to set the mode
        let dispatch = match ki_mode {
            KiMode::Normal => crate::app::Dispatch::ToEditor(
                crate::components::editor::DispatchEditor::EnterNormalMode,
            ),
            KiMode::Insert => crate::app::Dispatch::ToEditor(
                crate::components::editor::DispatchEditor::EnterInsertMode(
                    crate::components::editor::Direction::End,
                ),
            ),
            KiMode::MultiCursor => crate::app::Dispatch::ToEditor(
                crate::components::editor::DispatchEditor::EnterMultiCursorMode,
            ),
            KiMode::FindOneChar(if_current_not_found) => crate::app::Dispatch::ToEditor(
                crate::components::editor::DispatchEditor::FindOneChar(if_current_not_found),
            ),
            KiMode::Swap => crate::app::Dispatch::ToEditor(
                crate::components::editor::DispatchEditor::EnterSwapMode,
            ),
            KiMode::Replace => crate::app::Dispatch::ToEditor(
                crate::components::editor::DispatchEditor::EnterReplaceMode,
            ),
            KiMode::Extend => crate::app::Dispatch::ToEditor(
                crate::components::editor::DispatchEditor::EnterExtendMode,
            ),
        };

        // Create an AppMessage::ExternalDispatch to send to the App thread
        let app_message = crate::app::AppMessage::ExternalDispatch(dispatch);

        // Send the message to the App thread via the app_sender
        match self.app_sender.send(app_message) {
            Ok(_) => {
                info!(
                    "[{}] Successfully sent mode change dispatch to app_sender",
                    trace_id
                );
            }
            Err(e) => {
                error!(
                    "[{}] Failed to send mode change dispatch to app_sender: {}",
                    trace_id, e
                );
                return self
                    .send_error_response(id, &format!("Failed to send dispatch to app: {}", e));
            }
        }

        Ok(())
    }
}
