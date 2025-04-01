use std::rc::Rc;

use itertools::Itertools;

use crate::{components::suggestive_editor::Info, quickfix_list::DiagnosticSeverityRange};

use super::{get_current_selection_by_cursor_via_iter, SelectionMode};

// TODO: change this to custom selections, so it can also hold references, definitions etc
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

impl SelectionMode for Diagnostic {
    fn get_current_selection_by_cursor(
        &self,
        buffer: &crate::buffer::Buffer,
        cursor_char_index: crate::selection::CharIndex,
        if_current_not_found: crate::components::editor::IfCurrentNotFound,
    ) -> anyhow::Result<Option<super::ByteRange>> {
        get_current_selection_by_cursor_via_iter(
            buffer,
            cursor_char_index,
            if_current_not_found,
            Rc::new(
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
            ),
        )
    }
}
