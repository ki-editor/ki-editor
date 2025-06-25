use super::prelude::*;
use crate::embed::app::EmbeddedApp;
use ki_protocol_types::OutputMessage;

/// Handle ping requests from Host
///
pub(crate) fn handle_ping_request(app: &mut EmbeddedApp, id: u32, value: String) -> Result<()> {
    info!("Received ping: {}", value);
    let response_message = format!("pong: {}", value);
    app.send_response(id, OutputMessage::Ping(response_message))
}
