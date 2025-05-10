//! Selection mode change handlers for VSCode IPC messages

use super::prelude::*;
use crate::selection::SelectionMode as KiSelectionMode;
use crate::vscode::app::VSCodeApp;
use anyhow::Result;
use ki_protocol_types::OutputMessage;
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
    /// Handle selection mode set request from VSCode
    pub fn handle_selection_mode_set_request(
        &mut self,
        id: u64,
        params: ki_protocol_types::SelectionModeParams,
        trace_id: &str,
    ) -> Result<()> {
        info!(
            "[{}] Received selection_mode.set request: {:?}",
            trace_id, params.mode
        );

        // Convert protocol selection mode to Ki selection mode
        let ki_mode = match params.mode {
            ki_protocol_types::SelectionMode::Character => KiSelectionMode::Character,
            ki_protocol_types::SelectionMode::Line => KiSelectionMode::Line,
            ki_protocol_types::SelectionMode::LineFull => KiSelectionMode::LineFull,
            ki_protocol_types::SelectionMode::CoarseWord => {
                KiSelectionMode::Word { skip_symbols: true }
            }
            ki_protocol_types::SelectionMode::FineWord => KiSelectionMode::Word {
                skip_symbols: false,
            },
            ki_protocol_types::SelectionMode::Token => KiSelectionMode::Token,
            ki_protocol_types::SelectionMode::Custom => KiSelectionMode::Custom,
            ki_protocol_types::SelectionMode::SyntaxNode => KiSelectionMode::SyntaxNode,
            ki_protocol_types::SelectionMode::SyntaxNodeFine => KiSelectionMode::SyntaxNodeFine,
            ki_protocol_types::SelectionMode::Mark => KiSelectionMode::Mark,
            // Simplified modes - use reasonable defaults
            ki_protocol_types::SelectionMode::Find => {
                // Default to a simple search for the word under cursor
                let search = crate::context::Search {
                    search: "".to_string(),
                    mode: crate::context::LocalSearchConfigMode::Regex(
                        crate::list::grep::RegexConfig::default(),
                    ),
                };
                KiSelectionMode::Find { search }
            }
            ki_protocol_types::SelectionMode::Diagnostic => {
                // Default to all diagnostics
                KiSelectionMode::Diagnostic(crate::quickfix_list::DiagnosticSeverityRange::All)
            }
            ki_protocol_types::SelectionMode::GitHunk => {
                // Default to unstaged changes against current branch
                KiSelectionMode::GitHunk(crate::git::DiffMode::UnstagedAgainstCurrentBranch)
            }
            ki_protocol_types::SelectionMode::LocalQuickfix => {
                // Default title
                KiSelectionMode::LocalQuickfix {
                    title: "Quickfix".to_string(),
                }
            }
        };

        debug!("[{}] Setting selection mode to: {:?}", trace_id, ki_mode);

        // First, send the success response immediately to prevent VSCode from timing out
        debug!(
            "[{}] Sending success response before processing selection mode set",
            trace_id
        );
        let response_result = self.send_response(id, OutputMessage::Success(true));
        if let Err(e) = response_result {
            error!("[{}] Failed to send success response: {}", trace_id, e);
            return Err(e);
        }

        // Create a dispatch to set the selection mode
        let dispatch = crate::app::Dispatch::ToEditor(
            crate::components::editor::DispatchEditor::SetSelectionMode(
                crate::components::editor::IfCurrentNotFound::LookForward,
                ki_mode,
            ),
        );

        // Create an AppMessage::ExternalDispatch to send to the App thread
        let app_message = crate::app::AppMessage::ExternalDispatch(dispatch);

        // Send the message to the App thread via the app_sender
        match self.app_sender.send(app_message) {
            Ok(_) => {
                info!(
                    "[{}] Successfully sent selection mode change dispatch to app_sender",
                    trace_id
                );
            }
            Err(e) => {
                error!(
                    "[{}] Failed to send selection mode change dispatch to app_sender: {}",
                    trace_id, e
                );
                return self
                    .send_error_response(id, &format!("Failed to send dispatch to app: {}", e));
            }
        }

        Ok(())
    }
}
