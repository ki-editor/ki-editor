use shared::canonicalized_path::CanonicalizedPath;

use super::completion::PositionalEdit;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceEdit {
    pub edits: Vec<TextDocumentEdit>,
}

impl TryFrom<lsp_types::WorkspaceEdit> for WorkspaceEdit {
    type Error = anyhow::Error;
    fn try_from(value: lsp_types::WorkspaceEdit) -> Result<Self, Self::Error> {
        if let Some(changes) = value.changes {
            return Ok(WorkspaceEdit {
                edits: changes
                    .into_iter()
                    .map(|(url, edits)| {
                        Ok(TextDocumentEdit {
                            path: url
                                .to_file_path()
                                .map_err(|_| anyhow::anyhow!("Invalid URI"))?
                                .try_into()?,
                            edits: edits
                                .into_iter()
                                .map(|edit| edit.try_into())
                                .collect::<Result<Vec<_>, _>>()?,
                        })
                    })
                    .collect::<Result<Vec<_>, Self::Error>>()?,
            });
        }
        Ok(WorkspaceEdit {
            edits: value
                .document_changes
                .map(|changes| match changes {
                    lsp_types::DocumentChanges::Edits(edits) => edits
                        .into_iter()
                        .map(|edit| edit.try_into())
                        .collect::<Result<Vec<_>, _>>(),
                    lsp_types::DocumentChanges::Operations(_) => todo!(),
                })
                .unwrap_or_else(|| Ok(vec![]))?,
        })
    }
}

impl TryFrom<lsp_types::TextDocumentEdit> for TextDocumentEdit {
    type Error = anyhow::Error;

    fn try_from(value: lsp_types::TextDocumentEdit) -> Result<Self, Self::Error> {
        let path = CanonicalizedPath::try_from(
            value
                .text_document
                .uri
                .to_file_path()
                .map_err(|_| anyhow::anyhow!("Invalid URI"))?,
        )?;
        Ok(TextDocumentEdit {
            path,
            edits: value
                .edits
                .into_iter()
                .map(|edit| -> anyhow::Result<PositionalEdit> {
                    match edit {
                        lsp_types::OneOf::Left(text_edit) => text_edit.try_into(),
                        lsp_types::OneOf::Right(annotated_text_edit) => {
                            annotated_text_edit.try_into()
                        }
                    }
                })
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextDocumentEdit {
    pub path: CanonicalizedPath,
    pub edits: Vec<PositionalEdit>,
}
