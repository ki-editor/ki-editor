use std::path::PathBuf;

use itertools::Itertools;
use shared::canonicalized_path::CanonicalizedPath;

use super::completion::PositionalEdit;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceEdit {
    pub edits: Vec<TextDocumentEdit>,
    pub resource_operations: Vec<ResourceOperation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceOperation {
    Create(String),
    Rename {
        old: CanonicalizedPath,
        new: PathBuf,
    },
    Delete(CanonicalizedPath),
}

impl TryFrom<lsp_types::ResourceOp> for ResourceOperation {
    type Error = anyhow::Error;

    fn try_from(value: lsp_types::ResourceOp) -> Result<Self, Self::Error> {
        match value {
            lsp_types::ResourceOp::Create(create) => {
                Ok(ResourceOperation::Create(create.uri.try_into()?))
            }
            lsp_types::ResourceOp::Rename(rename) => Ok(ResourceOperation::Rename {
                old: rename.old_uri.try_into()?,
                new: (rename
                        .new_uri
                        .to_file_path()
                        .map_err(|error| anyhow::anyhow!("{:?}", error))?),
            }),
            lsp_types::ResourceOp::Delete(delete) => {
                Ok(ResourceOperation::Delete(delete.uri.try_into()?))
            }
        }
    }
}

impl TryFrom<lsp_types::WorkspaceEdit> for WorkspaceEdit {
    type Error = anyhow::Error;
    fn try_from(value: lsp_types::WorkspaceEdit) -> Result<Self, Self::Error> {
        let edits1 = value
            .changes
            .map(|changes| {
                changes
                    .into_iter()
                    .map(|(url, edits)| {
                        Ok(TextDocumentEdit {
                            path: url.try_into()?,
                            edits: edits
                                .into_iter()
                                .map(|edit| edit.try_into())
                                .try_collect()?,
                        })
                    })
                    .collect::<Result<Vec<_>, Self::Error>>()
            })
            .transpose()?
            .unwrap_or_default();

        let (edits2, edits_or_operations): (Vec<_>, Vec<_>) = value
            .document_changes
            .into_iter()
            .partition_map(|changes| match changes {
                lsp_types::DocumentChanges::Edits(edits) => itertools::Either::Left(edits),
                lsp_types::DocumentChanges::Operations(operations) => {
                    let (edits, operations): (Vec<_>, Vec<_>) = operations
                        .into_iter()
                        .partition_map(|operation| match operation {
                            lsp_types::DocumentChangeOperation::Edit(edit) => {
                                itertools::Either::Left(edit)
                            }
                            lsp_types::DocumentChangeOperation::Op(operation) => {
                                itertools::Either::Right(operation)
                            }
                        });
                    itertools::Either::Right((edits, operations))
                }
            });
        let (edits3, operations): (Vec<_>, Vec<_>) = edits_or_operations.into_iter().unzip();
        let edits = edits1
            .into_iter()
            .chain(
                edits2
                    .into_iter()
                    .chain(edits3)
                    .flatten()
                    .map(|edit| edit.try_into())
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter(),
            )
            .collect_vec();
        Ok(WorkspaceEdit {
            edits,
            resource_operations: operations
                .into_iter()
                .flatten()
                .map(|operation| operation.try_into())
                .try_collect()?,
        })
    }
}

impl TryFrom<lsp_types::TextDocumentEdit> for TextDocumentEdit {
    type Error = anyhow::Error;

    fn try_from(value: lsp_types::TextDocumentEdit) -> Result<Self, Self::Error> {
        let path = value.text_document.uri.try_into()?;
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
