//! Search handlers for VSCode IPC messages

use crate::vscode::app::VSCodeApp;
use anyhow::Result;
use ki_protocol_types::{OutputMessage, SearchParams};
use log::{debug, error, info};

impl VSCodeApp {
    /// Handle search find request from VSCode
    pub fn handle_search_find_request(
        &mut self,
        id: u64,
        params: SearchParams,
        trace_id: &str,
    ) -> Result<()> {
        info!("[{}] Received search.find request: {:?}", trace_id, params);

        // First, send the success response immediately to prevent VSCode from timing out
        debug!(
            "[{}] Sending success response before processing search find",
            trace_id
        );
        let response_result = self.send_response(id, OutputMessage::Success(true));
        if let Err(e) = response_result {
            error!("[{}] Failed to send success response: {}", trace_id, e);
            return Err(e);
        }

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
        match self.app_sender.send(app_message) {
            Ok(_) => {
                info!(
                    "[{}] Successfully sent search find dispatch to app_sender",
                    trace_id
                );
            }
            Err(e) => {
                error!(
                    "[{}] Failed to send search find dispatch to app_sender: {}",
                    trace_id, e
                );
                return self
                    .send_error_response(id, &format!("Failed to send dispatch to app: {}", e));
            }
        }

        Ok(())
    }
}
