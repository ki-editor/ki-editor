use lsp_types::DiagnosticSeverity;

use crate::components::suggestive_editor::Info;

use super::SelectionMode;

// TODO: change this to custom selections, so it can also hold references, definitions etc
pub struct Diagnostic {
    severity: Option<DiagnosticSeverity>,
    diagnostics: Vec<crate::lsp::diagnostic::Diagnostic>,
}

impl Diagnostic {
    pub fn new(
        severity: Option<DiagnosticSeverity>,
        params: super::SelectionModeParams<'_>,
    ) -> Self {
        let buffer = params.buffer;
        let diagnostics = params.context.get_diagnostics(buffer.path());
        Self {
            severity,
            diagnostics: diagnostics.into_iter().cloned().collect(),
        }
    }
}

impl SelectionMode for Diagnostic {
    fn name(&self) -> &'static str {
        "DIAGNOSTIC"
    }
    fn iter<'a>(
        &'a self,
        params: super::SelectionModeParams<'a>,
    ) -> anyhow::Result<Box<dyn Iterator<Item = super::ByteRange> + 'a>> {
        let buffer = params.buffer;

        Ok(Box::new(
            self.diagnostics
                .iter()
                .filter(|diagnostic| {
                    self.severity.is_none() || diagnostic.severity == self.severity
                })
                .filter_map(|diagnostic| {
                    Some(super::ByteRange::with_info(
                        buffer
                            .position_range_to_byte_range(&diagnostic.range)
                            .ok()?,
                        Info::new(diagnostic.message.clone()),
                    ))
                }),
        ))
    }
}
