use itertools::Itertools;

use crate::{
    components::suggestive_editor::{Decoration, Info},
    grid::StyleKey,
    selection_range::SelectionRange,
};

use super::documentation::Documentation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureHelp {
    pub signatures: Vec<SignatureInformation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureInformation {
    pub label: String,
    pub documentation: Option<Documentation>,
    pub active_parameter_byte_range: Option<SelectionRange>,
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

impl SignatureHelp {
    pub fn into_info(self) -> Option<Info> {
        self.signatures
            .into_iter()
            .map(|signature| {
                let signature_label_len = signature.label.len();
                let content = [signature.label]
                    .into_iter()
                    .chain(signature.documentation.map(|doc| doc.content))
                    .collect_vec()
                    .join(&format!("\n{}\n", "-".repeat(signature_label_len)));

                let decoration = signature
                    .active_parameter_byte_range
                    .map(|selection_range| {
                        Decoration::new(selection_range, StyleKey::UiPrimarySelection)
                    });
                Info::new("Signature Help".to_string(), content)
                    .set_decorations(decoration.into_iter().collect_vec())
            })
            .reduce(Info::join)
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
                        Some(SelectionRange::Byte(start as usize..end as usize))
                    }
                    _ => None,
                }
            }),
        }
    }
}
