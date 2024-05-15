use crate::{
    buffer::Buffer, char_index_range::CharIndexRange, position::Position, quickfix_list::Location,
};

use lsp_types::DiagnosticSeverity;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub range: CharIndexRange,
    pub message: String,
    pub severity: Option<DiagnosticSeverity>,
    pub related_information: Option<Vec<DiagnosticRelatedInformation>>,
    pub code_description: Option<lsp_types::CodeDescription>,
    pub original_value: Option<lsp_types::Diagnostic>,
}

impl Diagnostic {
    pub(crate) fn try_from(buffer: &Buffer, value: lsp_types::Diagnostic) -> anyhow::Result<Self> {
        Ok(Self {
            range: buffer.position_range_to_char_index_range(
                &(Position::from(value.range.start)..Position::from(value.range.end)),
            )?,
            message: value.message.clone(),
            severity: value.severity,
            code_description: value.code_description.clone(),
            related_information: if let Some(related_information) =
                value.related_information.clone()
            {
                Some(
                    related_information
                        .into_iter()
                        .map(DiagnosticRelatedInformation::try_from)
                        .collect::<Result<Vec<_>, _>>()?,
                )
            } else {
                None
            },
            original_value: Some(value),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticRelatedInformation {
    location: Location,
    message: String,
}

impl TryFrom<lsp_types::DiagnosticRelatedInformation> for DiagnosticRelatedInformation {
    type Error = anyhow::Error;

    fn try_from(value: lsp_types::DiagnosticRelatedInformation) -> Result<Self, Self::Error> {
        Ok(Self {
            location: Location::try_from(value.location)?,
            message: value.message,
        })
    }
}
