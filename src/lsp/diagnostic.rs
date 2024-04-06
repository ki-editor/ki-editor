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
    pub fn message(&self) -> String {
        let severity = self.severity.map(|severity| match severity {
            DiagnosticSeverity::ERROR => "ERROR",
            DiagnosticSeverity::WARNING => "WARNING",
            DiagnosticSeverity::INFORMATION => "INFO",
            DiagnosticSeverity::HINT => "HINT",
            _ => "UNKNOWN",
        });
        format!(
            "[{}]\n{}\n\n[RELATED INFORMATION]\n{}\n\n[REFERENCE]\n{}",
            severity.unwrap_or("UNKNOWN"),
            self.message,
            self.related_information
                .as_ref()
                .map(|related_information| {
                    related_information
                        .iter()
                        .map(|related_information| {
                            format!(
                                "{}\n  {}\n\n{}",
                                related_information.location.display(),
                                related_information.message,
                                related_information
                                    .location
                                    .read()
                                    .unwrap_or_else(|error| error.to_string())
                                    .lines()
                                    .map(|line| format!("    {}", line))
                                    .collect::<Vec<_>>()
                                    .join("\n")
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n\n")
                })
                .unwrap_or_else(|| "N/A".to_string()),
            self.code_description
                .as_ref()
                .map(|description| description.href.to_string())
                .unwrap_or_else(|| "N/A".to_string())
        )
    }

    pub fn try_from(buffer: &Buffer, value: lsp_types::Diagnostic) -> anyhow::Result<Self> {
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
