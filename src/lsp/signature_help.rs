use crate::selection_mode::ByteRange;

use super::documentation::Documentation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureHelp {
    pub signatures: Vec<SignatureInformation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureInformation {
    pub label: String,
    pub documentation: Option<Documentation>,
    pub active_parameter_byte_range: Option<ByteRange>,
}

impl From<lsp_types::SignatureHelp> for SignatureHelp {
    fn from(value: lsp_types::SignatureHelp) -> Self {
        Self {
            signatures: value
                .signatures
                .into_iter()
                .map(SignatureInformation::from)
                .collect(),
        }
    }
}

impl From<lsp_types::SignatureInformation> for SignatureInformation {
    fn from(value: lsp_types::SignatureInformation) -> Self {
        Self {
            label: value.label,
            documentation: value.documentation.map(Documentation::from),
            active_parameter_byte_range: value.parameters.and_then(|parameters| {
                let label = parameters
                    .get(value.active_parameter? as usize)?
                    .label
                    .to_owned();
                match label {
                    lsp_types::ParameterLabel::LabelOffsets([start, end]) => {
                        Some(ByteRange::new(start as usize..end as usize))
                    }
                    _ => None,
                }
            }),
        }
    }
}
