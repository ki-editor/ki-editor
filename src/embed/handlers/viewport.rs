use crate::embed::app::EmbeddedApp;
use anyhow::Result;
use itertools::Itertools;
use ki_protocol_types::ViewportParams;

impl EmbeddedApp {
    /// Handle viewport change request from Host
    pub(crate) fn handle_viewport_change_request(&mut self, params: ViewportParams) -> Result<()> {
        let component = self.app.lock().unwrap().current_component();
        let mut component_ref = component.borrow_mut();
        let editor = component_ref.editor_mut();

        let visible_line_ranges = params
            .visible_line_ranges
            .iter()
            .map(|line_range| line_range.start as usize..line_range.end as usize)
            .collect_vec();
        editor.set_visible_line_ranges(visible_line_ranges);

        Ok(())
    }
}
