//! Viewport change handlers for VSCode IPC messages

use crate::vscode::app::VSCodeApp;
use anyhow::Result;
use itertools::Itertools;
use ki_protocol_types::{OutputMessage, ViewportParams};

impl VSCodeApp {
    /// Handle viewport change request from VSCode
    pub fn handle_viewport_change_request(
        &mut self,
        id: u64,
        params: ViewportParams,
    ) -> Result<()> {
        let component = self.app.lock().unwrap().current_component();
        let mut component_ref = component.borrow_mut();
        let editor = component_ref.editor_mut();

        let visible_line_ranges = params
            .visible_line_ranges
            .iter()
            .map(|line_range| line_range.start..line_range.end)
            .collect_vec();
        editor.set_visible_line_ranges(visible_line_ranges);

        // Return success without creating a dispatch
        self.send_response(id, OutputMessage::Success(true))
    }
}
