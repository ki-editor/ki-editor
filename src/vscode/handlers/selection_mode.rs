//! Selection mode change handlers for VSCode IPC messages

use super::prelude::*;
use crate::selection::SelectionMode as KiSelectionMode;
use crate::vscode::app::VSCodeApp;
use anyhow::Result;
use ki_protocol_types::{ModeParams, OutputMessage, OutputMessageWrapper};
use log::debug;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum VSCodeSelectionMode {
    Word,
    Token,
    Character,
    LineFull,
    LineTrimmed,
}

impl From<KiSelectionMode> for VSCodeSelectionMode {
    fn from(mode: KiSelectionMode) -> Self {
        match mode {
            KiSelectionMode::Word { .. } => VSCodeSelectionMode::Word,
            KiSelectionMode::Token => VSCodeSelectionMode::Token,
            KiSelectionMode::Character => VSCodeSelectionMode::Character,
            KiSelectionMode::LineFull => VSCodeSelectionMode::LineFull,
            KiSelectionMode::Line => VSCodeSelectionMode::LineTrimmed,
            // Map other modes to appropriate VSCode modes or default to Word
            _ => VSCodeSelectionMode::Word,
        }
    }
}

impl fmt::Display for VSCodeSelectionMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VSCodeSelectionMode::Word => write!(f, "Word"),
            VSCodeSelectionMode::Token => write!(f, "Token"),
            VSCodeSelectionMode::Character => write!(f, "Character"),
            VSCodeSelectionMode::LineFull => write!(f, "Line"),
            VSCodeSelectionMode::LineTrimmed => write!(f, "Line (Trimmed)"),
        }
    }
}

/// Parameters for selection mode change notification with VSCode-specific mode type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VSCodeSelectionModeChangeParams {
    pub mode: VSCodeSelectionMode,
    pub buffer_id: String,
}

impl VSCodeApp {
    // Implement a simple version without using the params.selection_mode field
    pub fn _handle_selection_mode_set_request(
        &mut self,
        id: u64,
        params: ModeParams,
    ) -> Result<()> {
        info!("Received selection_mode.set request: {}", params.mode);

        // Convert mode string to Ki selection mode
        let ki_mode = match params.mode.as_str() {
            "Word" => KiSelectionMode::Word {
                skip_symbols: false,
            },
            "Token" => KiSelectionMode::Token,
            "Character" => KiSelectionMode::Character,
            "Line" => KiSelectionMode::Line,
            "Line (Trimmed)" => KiSelectionMode::Line,
            _ => {
                warn!("Unknown selection mode: {}", params.mode);
                return Ok(());
            }
        };

        debug!("Setting selection mode to: {}", params.mode);

        // Lock the app and set the selection mode
        let component = self.app.lock().unwrap().current_component();

        // Use a default path if none is available
        let path = self
            .get_current_file_path()
            .unwrap_or_else(|| ".".try_into().unwrap());
        let context = Context::new(path);

        {
            let mut component_ref = component.borrow_mut();
            let editor = component_ref.editor_mut();
            if let Err(e) =
                editor.set_selection_mode(IfCurrentNotFound::LookForward, ki_mode.clone(), &context)
            {
                error!("Failed to set selection mode: {}", e);
            }
        }

        // Send success response
        let response = OutputMessageWrapper {
            id,
            message: OutputMessage::Success(true),
            error: None,
        };
        self.send_message_to_vscode(response)?;

        Ok(())
    }
}
