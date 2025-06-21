use crate::position::Position as KiPosition;
use ki_protocol_types::Position as VSCodePosition;
use shared::canonicalized_path::CanonicalizedPath;
use url::Url;

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
    let path_str = uri.strip_prefix("file://").unwrap_or(uri);

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
