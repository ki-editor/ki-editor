use crate::components::editor::Mode as KiMode;
use crate::position::Position as KiPosition;
use ki_protocol_types::Position as VSCodePosition; // Assuming Range might be needed later
use shared::canonicalized_path::CanonicalizedPath;
use url::Url;

// Convert Ki editor position to VSCode protocol position
pub(crate) fn ki_position_to_vscode_position(pos: &KiPosition) -> VSCodePosition {
    VSCodePosition {
        line: pos.line,
        character: pos.column, // Ki uses 'column', VSCode uses 'character'
    }
}

// Convert a CanonicalizedPath to a file URI string
pub(crate) fn path_to_uri(path: &CanonicalizedPath) -> String {
    Url::from_file_path(path.as_ref())
        .map(|url| url.to_string())
        .unwrap_or_else(|_| path.display_absolute()) // Fallback to absolute path if URL conversion fails
}

// Convert a file URI string back to a CanonicalizedPath
pub(crate) fn uri_to_path(uri: &str) -> Option<CanonicalizedPath> {
    Url::parse(uri)
        .ok()
        .and_then(|url| url.to_file_path().ok())
        .and_then(|path_buf| path_buf.try_into().ok())
}

// Convert Ki editor mode to VSCode protocol mode string
pub(crate) fn mode_to_protocol(mode: &KiMode) -> String {
    match mode {
        KiMode::Normal => "normal".to_string(),
        KiMode::Insert => "insert".to_string(),
        KiMode::MultiCursor => "multi_cursor".to_string(),
        KiMode::FindOneChar(_) => "find_one_char".to_string(),
        KiMode::Swap => "swap".to_string(),
        KiMode::Replace => "replace".to_string(),
        KiMode::Extend => "extend".to_string(),
        // Add other specific modes if they exist and need distinct protocol names
        // _ => "unknown".to_string(), // Optional catch-all
    }
}

// Convert Ki editor selection mode display string to VSCode protocol string
// Note: This assumes the input `mode_str` is already the display representation.
pub(crate) fn selection_mode_to_protocol(mode_str: &str) -> String {
    // Directly use the string representation from `display_selection_mode`
    // This might need adjustment if the protocol expects specific identifiers
    mode_str.to_lowercase().replace(' ', "_") // Example basic transformation
}

// // Convert Ki editor SelectionSet to VSCode protocol SelectionSet (Example - Needs Ki's SelectionSet definition)
// use crate::selection::SelectionSet as KiSelectionSet; // Hypothetical import
// pub(crate) fn selection_set_to_protocol(ki_set: &KiSelectionSet) -> ki_protocol_types::SelectionSet {
//     ki_protocol_types::SelectionSet {
//         buffer_id: ki_set.buffer_id.clone(), // Assuming buffer_id exists and is String
//         primary: ki_set.primary_index(), // Assuming a method to get primary index
//         selections: ki_set.selections().iter().map(|sel| { // Assuming selections() returns iterator
//             ki_protocol_types::Selection {
//                 start: ki_position_to_vscode_position(&sel.start()), // Assuming start() method
//                 end: ki_position_to_vscode_position(&sel.end()), // Assuming end() method
//                 is_extended: sel.is_extended(), // Assuming is_extended() method
//             }
//         }).collect(),
//     }
// }
