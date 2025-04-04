use std::rc::Rc;

use itertools::Itertools;

use crate::{components::suggestive_editor::Info, quickfix_list::DiagnosticSeverityRange};

use super::{VectorBased, VectorBasedSelectionMode};

pub(crate) struct Diagnostic {
    severity_range: DiagnosticSeverityRange,
    diagnostics: Vec<crate::lsp::diagnostic::Diagnostic>,
}

impl Diagnostic {
    pub(crate) fn new(
        severity_range: DiagnosticSeverityRange,
        params: super::SelectionModeParams<'_>,
    ) -> Self {
        Self {
            severity_range,
            diagnostics: params.buffer.diagnostics(),
        }
    }
}

impl VectorBasedSelectionMode for Diagnostic {
    fn get_byte_ranges(
        &self,
        buffer: &crate::buffer::Buffer,
    ) -> Result<Rc<Vec<super::ByteRange>>, anyhow::Error> {
        Ok(Rc::new(
            self.diagnostics
                .iter()
                .filter(|diagnostic| self.severity_range.contains(diagnostic.severity))
                .map(|diagnostic| -> anyhow::Result<_> {
                    Ok(super::ByteRange::with_info(
                        buffer.char_index_range_to_byte_range(diagnostic.range)?,
                        Info::new("Diagnostics".to_string(), diagnostic.message.clone()),
                    ))
                })
                .try_collect()?,
        ))
    }
}
