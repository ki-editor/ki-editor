use std::ops::Range;

use crate::position::Position;

use lsp_types::DiagnosticSeverity;

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub range: Range<Position>,
    pub message: String,
    pub severity: Option<DiagnosticSeverity>,
}

impl Diagnostic {
    pub fn message(&self) -> String {
        let severity = self.severity.map(|severity| match severity {
            DiagnosticSeverity::ERROR => "ERROR",
            DiagnosticSeverity::WARNING => "WARNING",
            DiagnosticSeverity::INFORMATION => "INFO",
            DiagnosticSeverity::HINT => "HINT",
            _ => "UNKNOWN",
        });
        if let Some(severity) = severity {
            format!("[{}]\n{}", severity, self.message)
        } else {
            self.message.clone()
        }
    }
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
