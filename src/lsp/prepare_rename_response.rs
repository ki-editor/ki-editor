use std::ops::Range;

use crate::position::Position;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PrepareRenameResponse {
    pub(crate) range: Option<Range<Position>>,
    pub(crate) placeholder: Option<String>,
}

impl From<lsp_types::PrepareRenameResponse> for PrepareRenameResponse {
    fn from(value: lsp_types::PrepareRenameResponse) -> PrepareRenameResponse {
        match value {
            lsp_types::PrepareRenameResponse::Range(range) => PrepareRenameResponse {
                range: Some(range.start.into()..range.end.into()),
                placeholder: None,
            },
            lsp_types::PrepareRenameResponse::RangeWithPlaceholder { range, placeholder } => {
                PrepareRenameResponse {
                    range: Some(range.start.into()..range.end.into()),
                    placeholder: Some(placeholder),
                }
            }

            lsp_types::PrepareRenameResponse::DefaultBehavior { .. } => PrepareRenameResponse {
                range: None,
                placeholder: None,
            },
        }
    }
}
