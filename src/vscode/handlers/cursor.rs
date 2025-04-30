//! Cursor-related handlers for VSCode IPC messages

use crate::vscode::app::VSCodeApp;

impl VSCodeApp {
    // The check_for_cursor_changes function has been removed.
    // Instead, we now use the Dispatch::SelectionChanged event to send cursor updates.
    // This is a more reliable approach as it captures all cursor changes directly
    // when they happen, rather than polling for changes.

    // Suppression flag to prevent feedback loop when applying backend-driven updates
    // This should be a field on VSCodeApp if not already present
    // Example: self.suppress_next_cursor_update: bool

    // Note: The cursor update and get request handlers have been removed as they are no longer used.
    // Selection-related functionality is now handled by the selection.rs handlers.
}
