//! Ping handler for VSCode IPC messages

use super::prelude::*;
use crate::vscode::app::VSCodeApp;
use ki_protocol_types::OutputMessage;

/// Handle ping requests from VSCode
pub fn handle_ping_request(app: &mut VSCodeApp, id: u64, value: String) -> Result<()> {
    info!("Received ping: {}", value);
    let response_message = format!("pong: {}", value);
    app.send_response(id, OutputMessage::Ping(response_message))
}
