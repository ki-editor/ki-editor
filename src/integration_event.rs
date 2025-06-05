use crate::{
    components::editor::Mode,
    selection::{CharIndex, Selection, SelectionMode},
};
use shared::canonicalized_path::CanonicalizedPath;
use std::sync::mpsc::Sender;

/// Component ID used to identify which component an event is related to
pub type ComponentId = usize;

/// Convert from components::component::ComponentId to integration_event::ComponentId
pub fn component_id_to_usize(id: &crate::components::component::ComponentId) -> ComponentId {
    // Since we can't access the private field directly, we'll use the debug representation
    // This is a bit of a hack, but it's the simplest way to get the value without modifying
    // the original ComponentId struct
    let debug_str = format!("{:?}", id);
    let value_str = debug_str
        .trim_start_matches("ComponentId(")
        .trim_end_matches(")");
    value_str.parse::<usize>().unwrap_or(0)
}

/// Events emitted by Ki for external integrations
#[derive(Debug, Clone)]
pub enum IntegrationEvent {
    // Buffer events
    BufferChanged {
        #[allow(dead_code)]
        component_id: ComponentId,
        path: CanonicalizedPath,
        edits: Vec<ki_protocol_types::DiffEdit>,
    },
    BufferOpened {
        #[allow(dead_code)]
        component_id: ComponentId,
        path: CanonicalizedPath,
        language_id: Option<String>,
    },
    #[allow(dead_code)]
    BufferClosed {
        #[allow(dead_code)]
        component_id: ComponentId,
        path: CanonicalizedPath,
    },
    BufferSaved {
        #[allow(dead_code)]
        component_id: ComponentId,
        path: CanonicalizedPath,
    },
    BufferActivated {
        #[allow(dead_code)]
        component_id: ComponentId,
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
    // External buffer events
    #[allow(dead_code)]
    ExternalBufferCreated {
        component_id: ComponentId,
        buffer_id: String,
        content: String,
    },
    #[allow(dead_code)]
    ExternalBufferUpdated {
        component_id: ComponentId,
        buffer_id: String,
        content: String,
    },

    // Other events
    #[allow(dead_code)]
    CommandExecuted {
        command: String,
        success: bool,
    },
    JumpsChanged {
        component_id: usize,
        jumps: Vec<(char, CharIndex)>,
    },
    SelectionModeChanged {
        component_id: usize,
        selection_mode: SelectionMode,
    },
    PromptOpened {
        title: String,
        items: Vec<ki_protocol_types::PromptItem>,
    },
    MarksChanged {
        component_id: usize,
        marks: Vec<crate::char_index_range::CharIndexRange>,
    },
    RequestLspDefinition,
    RequestLspHover,
    RequestLspReferences,
    RequestLspDeclaration,
    RequestLspImplementation,
    RequestLspTypeDefinition,
    KeyboardLayoutChanged(&'static str),
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
                log::warn!("Failed to send integration event: {}", e);
            }
        }
    }
}
