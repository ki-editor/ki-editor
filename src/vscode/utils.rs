use crate::position::Position as KiPosition;
use ki_protocol_types::Position as VSCodePosition;
use shared::canonicalized_path::CanonicalizedPath;
use url::Url;

// Convert Ki editor position to VSCode protocol position
pub(crate) fn ki_position_to_vscode_position(pos: &KiPosition) -> VSCodePosition {
    VSCodePosition {
        line: pos.line,
        character: pos.column, // Ki uses 'column', VSCode uses 'character'
    }
}

// Convert VSCode protocol position to Ki editor position
pub(crate) fn vscode_position_to_ki_position(pos: &VSCodePosition) -> KiPosition {
    KiPosition {
        line: pos.line,
        column: pos.character, // VSCode uses 'character', Ki uses 'column'
    }
}

// Convert a CanonicalizedPath to a file URI string
pub(crate) fn path_to_uri(path: &CanonicalizedPath) -> String {
    Url::from_file_path(path.as_ref())
        .map(|url| url.to_string())
        .unwrap_or_else(|_| path.display_absolute()) // Fallback to absolute path if URL conversion fails
}

// Convert a file URI string back to a CanonicalizedPath
pub(crate) fn uri_to_path(uri: &str) -> anyhow::Result<CanonicalizedPath> {
    // First, check if the URI is already in the form of a CanonicalizedPath string
    if let Some(path_str) = extract_canonicalized_path(uri) {
        return path_str.try_into().map_err(|e| {
            anyhow::anyhow!(
                "Failed to convert extracted path to CanonicalizedPath: {}",
                e
            )
        });
    }

    // Try to parse as a URL first
    if let Ok(url) = Url::parse(uri) {
        if let Ok(path_buf) = url.to_file_path() {
            return path_buf.try_into().map_err(|e| {
                anyhow::anyhow!("Failed to convert URL path to CanonicalizedPath: {}", e)
            });
        }
    }

    // If URL parsing fails, try direct conversion
    // Remove the file:// prefix if present
    let path_str = if uri.starts_with("file://") {
        &uri[7..]
    } else {
        uri
    };

    // Convert to CanonicalizedPath
    path_str
        .try_into()
        .map_err(|e| anyhow::anyhow!("Failed to convert URI to path: {}", e))
}

/// Extract a path string from a CanonicalizedPath representation
fn extract_canonicalized_path(s: &str) -> Option<&str> {
    // Match patterns like CanonicalizedPath("/path/to/file")
    let re = regex::Regex::new(r#"CanonicalizedPath\("([^"]+)"\)"#).ok()?;
    re.captures(s)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str())
}

// These functions have been removed as they are no longer used.
// Mode and selection mode conversion is now handled directly in the integration event handler.

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
