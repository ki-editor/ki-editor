//! Selection mode change handlers for VSCode IPC messages

use super::prelude::*;
use crate::context::Search;
use crate::vscode::app::VSCodeApp;
use crate::{search::parse_search_config, selection::SelectionMode as KiSelectionMode};
use anyhow::Result;
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
            ki_protocol_types::SelectionMode::Word => KiSelectionMode::Word { skip_symbols: true },
            ki_protocol_types::SelectionMode::WordFine => KiSelectionMode::Word {
                skip_symbols: false,
            },
            ki_protocol_types::SelectionMode::Token => KiSelectionMode::Token,
            ki_protocol_types::SelectionMode::Custom => KiSelectionMode::Custom,
            ki_protocol_types::SelectionMode::SyntaxNode => KiSelectionMode::SyntaxNode,
            ki_protocol_types::SelectionMode::SyntaxNodeFine => KiSelectionMode::SyntaxNodeFine,
            ki_protocol_types::SelectionMode::Mark => KiSelectionMode::Mark,
            // Simplified modes - use reasonable defaults
            ki_protocol_types::SelectionMode::Find { search } => {
                // Default to a simple search for the word under cursor
                let search = parse_search_config(&search)?;
                KiSelectionMode::Find {
                    search: Search {
                        mode: search.local_config().mode,
                        search: search.local_config().search(),
                    },
                }
            }
            ki_protocol_types::SelectionMode::Diagnostic(kind) => {
                // Default to all diagnostics
                use crate::quickfix_list::DiagnosticSeverityRange::*;
                KiSelectionMode::Diagnostic(match kind {
                    ki_protocol_types::DiagnosticKind::Error => Error,
                    ki_protocol_types::DiagnosticKind::Information => Information,
                    ki_protocol_types::DiagnosticKind::Warning => Warning,
                    ki_protocol_types::DiagnosticKind::All => All,
                    ki_protocol_types::DiagnosticKind::Hint => Hint,
                })
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
        self.app_sender.send(app_message)?;

        Ok(())
    }
}
