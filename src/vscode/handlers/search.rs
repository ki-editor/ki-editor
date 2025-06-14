//! Search handlers for VSCode IPC messages

use crate::vscode::app::VSCodeApp;
use anyhow::Result;
use ki_protocol_types::SearchParams;
use log::{debug, info};

impl VSCodeApp {
    /// Handle search find request from VSCode
    pub fn handle_search_find_request(
        &mut self,
        params: SearchParams,
        trace_id: &str,
    ) -> Result<()> {
        info!("[{}] Received search.find request: {:?}", trace_id, params);

        // First, send the success response immediately to prevent VSCode from timing out
        debug!(
            "[{}] Sending success response before processing search find",
            trace_id
        );

        // Create a dispatch to perform the search
        // We'll use the UpdateLocalSearchConfig dispatch to set the search pattern
        // and then trigger a search
        let update = crate::app::LocalSearchConfigUpdate::Search(params.query);
        let scope = crate::app::Scope::Local; // Use local scope for now
        let dispatch = crate::app::Dispatch::UpdateLocalSearchConfig {
            update,
            scope,
            show_config_after_enter: false,
            if_current_not_found: crate::components::editor::IfCurrentNotFound::LookForward,
            run_search_after_config_updated: true,
        };

        // Create an AppMessage::ExternalDispatch to send to the App thread
        let app_message = crate::app::AppMessage::ExternalDispatch(dispatch);

        // Send the message to the App thread via the app_sender
        self.app_sender.send(app_message)?;

        Ok(())
    }
}
