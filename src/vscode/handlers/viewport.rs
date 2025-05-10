//! Viewport change handlers for VSCode IPC messages

use crate::vscode::app::VSCodeApp;
use anyhow::Result;
use ki_protocol_types::{OutputMessage, ViewportParams};
use log::{debug, error, info};

impl VSCodeApp {
    /// Handle viewport change request from VSCode
    pub fn handle_viewport_change_request(
        &mut self,
        id: u64,
        params: ViewportParams,
        trace_id: &str,
    ) -> Result<()> {
        info!(
            "[{}] Received viewport.change request: {:?}",
            trace_id, params
        );

        // First, send the success response immediately to prevent VSCode from timing out
        debug!(
            "[{}] Sending success response before processing viewport change",
            trace_id
        );
        let response_result = self.send_response(id, OutputMessage::Success(true));
        if let Err(e) = response_result {
            error!("[{}] Failed to send success response: {}", trace_id, e);
            return Err(e);
        }

        // For now, we'll just log the viewport change since there's no direct SetViewport dispatch
        // In a real implementation, we might want to add a SetViewport variant to DispatchEditor
        info!(
            "[{}] Viewport change: start_line={}, end_line={}",
            trace_id, params.start_line, params.end_line
        );

        // Return success without creating a dispatch
        Ok(())
    }
}
