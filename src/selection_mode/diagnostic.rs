use lsp_types::DiagnosticSeverity;

use crate::selection::Selection;

use super::SelectionMode;

pub struct Diagnostic(pub DiagnosticSeverity);

impl SelectionMode for Diagnostic {
    fn iter<'a>(
        &'a self,
        _current_selection: &'a Selection,
        buffer: &'a crate::buffer::Buffer,
    ) -> anyhow::Result<Box<dyn Iterator<Item = super::ByteRange> + 'a>> {
        Ok(Box::new(
            buffer
                .diagnostics()
                .iter()
                .filter(|diagnostic| diagnostic.severity == Some(self.0))
                .filter_map(|diagnostic| {
                    Some(super::ByteRange::with_info(
                        buffer.position_to_byte(diagnostic.range.start).ok()?
                            ..buffer.position_to_byte(diagnostic.range.end).ok()?,
                        diagnostic.message.clone(),
                    ))
                }),
        ))
    }
}
