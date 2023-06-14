use std::ops::Range;

use crate::position::Position;

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub range: Range<Position>,
    pub message: String,
    pub severity: Option<lsp_types::DiagnosticSeverity>,
}

impl From<lsp_types::Diagnostic> for Diagnostic {
    fn from(value: lsp_types::Diagnostic) -> Self {
        Self {
            range: Position::from(value.range.start)..Position::from(value.range.end),
            message: value.message,
            severity: value.severity,
        }
    }
}
