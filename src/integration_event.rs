use crate::{
    components::{component::ComponentId, editor::Mode},
    selection::{CharIndex, Selection, SelectionMode},
};
use shared::canonicalized_path::CanonicalizedPath;
use std::sync::mpsc::Sender;

/// Events emitted by Ki for external integrations
#[derive(Debug, Clone)]
pub enum IntegrationEvent {
    // Buffer events
    BufferChanged {
        path: CanonicalizedPath,
        edits: Vec<ki_protocol_types::DiffEdit>,
    },
    BufferSaved {
        path: CanonicalizedPath,
    },
    // Editor state events
    ModeChanged {
        component_id: ComponentId,
        mode: Mode,
    },
    SelectionChanged {
        component_id: ComponentId,
        selections: Vec<Selection>,
    },
    JumpsChanged {
        component_id: ComponentId,
        jumps: Vec<(char, CharIndex)>,
    },
    SelectionModeChanged {
        component_id: ComponentId,
        selection_mode: SelectionMode,
    },
    PromptOpened {
        title: String,
        items: Vec<ki_protocol_types::PromptItem>,
    },
    MarksChanged {
        component_id: ComponentId,
        marks: Vec<crate::char_index_range::CharIndexRange>,
    },
    RequestLspDefinition,
    RequestLspHover,
    RequestLspReferences,
    RequestLspDeclaration,
    RequestLspImplementation,
    RequestLspTypeDefinition,
    KeyboardLayoutChanged(&'static str),
    ShowInfo {
        /// Set to `None` to hide the info
        info: Option<String>,
    },
    RequestLspRename,
    RequestLspCodeAction,
    RequestLspDocumentSymbols,
    SyncBufferRequest {
        path: CanonicalizedPath,
    },
}

/// Trait for components that can emit integration events
pub trait IntegrationEventEmitter {
    fn emit_event(&self, event: IntegrationEvent);
}

/// Implementation for Option<Sender<IntegrationEvent>>
impl IntegrationEventEmitter for Option<Sender<IntegrationEvent>> {
    fn emit_event(&self, event: IntegrationEvent) {
        if let Some(sender) = self {
            // Use try_send to avoid blocking if the receiver is not ready
            // This is important to prevent deadlocks in the integration
            if let Err(e) = sender.send(event) {
                log::warn!("Failed to send integration event: {e}");
            }
        }
    }
}
