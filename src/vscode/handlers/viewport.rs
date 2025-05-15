//! Viewport change handlers for VSCode IPC messages

use crate::{
    app::{AppMessage, Dimension, Dispatch},
    vscode::app::VSCodeApp,
};
use anyhow::Result;
use itertools::Itertools;
use ki_protocol_types::{OutputMessage, ViewportParams};
use log::{debug, error, info};
use shared::canonicalized_path::CanonicalizedPath;

impl VSCodeApp {
    /// Handle viewport change request from VSCode
    pub fn handle_viewport_change_request(
        &mut self,
        id: u64,
        params: ViewportParams,
        trace_id: &str,
    ) -> Result<()> {
        let component = self.app.lock().unwrap().current_component();
        let mut component_ref = component.borrow_mut();
        let editor = component_ref.editor_mut();

        let visible_line_ranges = params
            .visible_line_ranges
            .iter()
            .map(|(start, end)| (*start)..(*end))
            .collect_vec();
        editor.set_visible_line_ranges(visible_line_ranges);

        // Return success without creating a dispatch
        let response_result = self.send_response(id, OutputMessage::Success(true));
        Ok(())
    }
}
