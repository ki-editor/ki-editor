use crate::{components::suggestive_editor::Info, quickfix_list::DiagnosticSeverityRange};

use super::SelectionMode;

// TODO: change this to custom selections, so it can also hold references, definitions etc
pub struct Diagnostic {
    severity_range: DiagnosticSeverityRange,
    diagnostics: Vec<crate::lsp::diagnostic::Diagnostic>,
}

impl Diagnostic {
    pub fn new(
        severity_range: DiagnosticSeverityRange,
        params: super::SelectionModeParams<'_>,
    ) -> Self {
        Self {
            severity_range,
            diagnostics: params.buffer.diagnostics(),
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
                .filter(|diagnostic| self.severity_range.contains(diagnostic.severity))
                .filter_map(|diagnostic| {
                    Some(super::ByteRange::with_info(
                        buffer.char_index_range_to_byte_range(diagnostic.range)?,
                        Info::new("Diagnostics".to_string(), diagnostic.message.clone()),
                    ))
                }),
        ))
    }
}
