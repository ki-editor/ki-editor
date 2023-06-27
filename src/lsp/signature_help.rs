use super::documentation::Documentation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureHelp {
    pub signatures: Vec<SignatureInformation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureInformation {
    pub label: String,
    pub documentation: Option<Documentation>,
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
        }
    }
}
