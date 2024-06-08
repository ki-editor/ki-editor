use std::ops::Range;

use itertools::Itertools;
use lsp_types::CompletionItemKind;
use shared::icons::get_icon_config;

use crate::{
    app::{Dispatch, Dispatches},
    components::{dropdown::DropdownItem, editor::DispatchEditor, suggestive_editor::Info},
    position::Position,
};

use super::documentation::Documentation;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Completion {
    pub(crate) items: Vec<DropdownItem>,
    pub(crate) trigger_characters: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CompletionItem {
    pub(crate) label: String,
    pub(crate) kind: Option<CompletionItemKind>,
    pub(crate) detail: Option<String>,
    pub(crate) documentation: Option<Documentation>,
    pub(crate) sort_text: Option<String>,
    pub(crate) insert_text: Option<String>,
    pub(crate) edit: Option<CompletionItemEdit>,
    pub(crate) completion_item: lsp_types::CompletionItem,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CompletionItemEdit {
    PositionalEdit(PositionalEdit),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PositionalEdit {
    pub(crate) range: Range<Position>,
    pub(crate) new_text: String,
}

impl TryFrom<lsp_types::AnnotatedTextEdit> for PositionalEdit {
    type Error = anyhow::Error;

    fn try_from(value: lsp_types::AnnotatedTextEdit) -> Result<Self, Self::Error> {
        value.text_edit.try_into()
    }
}

impl TryFrom<lsp_types::TextEdit> for PositionalEdit {
    type Error = anyhow::Error;

    fn try_from(value: lsp_types::TextEdit) -> Result<Self, Self::Error> {
        Ok(PositionalEdit {
            range: value.range.start.into()..value.range.end.into(),
            new_text: value.new_text,
        })
    }
}

impl CompletionItem {
    pub(crate) fn emoji(&self) -> String {
        self.kind
            .map(|kind| {
                get_icon_config()
                    .completion
                    .get(&format!("{:?}", kind))
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("({:?})", kind))
            })
            .unwrap_or_default()
    }
    pub(crate) fn info(&self) -> Option<Info> {
        let kind = self.kind.map(|kind| {
            convert_case::Casing::to_case(&format!("{:?}", kind), convert_case::Case::Title)
        });
        let detail = self.detail.clone();
        let documentation = self.documentation().map(|d| d.content);
        let result = []
            .into_iter()
            .chain(kind)
            .chain(detail)
            .chain(documentation)
            .collect_vec()
            .join("\n==========\n");
        if result.is_empty() {
            None
        } else {
            Some(Info::new("Completion Info".to_string(), result))
        }
    }
    #[cfg(test)]
    pub(crate) fn from_label(label: String) -> Self {
        Self {
            label,
            kind: None,
            detail: None,
            documentation: None,
            sort_text: None,
            edit: None,
            insert_text: None,
            completion_item: Default::default(),
        }
    }

    pub(crate) fn label(&self) -> String {
        self.label.clone()
    }

    pub(crate) fn documentation(&self) -> Option<Documentation> {
        self.documentation.clone()
    }

    pub(crate) fn additional_text_edits(&self) -> Vec<CompletionItemEdit> {
        self.completion_item
            .additional_text_edits
            .as_ref()
            .map(|edits| {
                edits
                    .iter()
                    .map(|edit| {
                        CompletionItemEdit::PositionalEdit(PositionalEdit {
                            range: edit.range.start.into()..edit.range.end.into(),
                            new_text: edit.new_text.clone(),
                        })
                    })
                    .collect_vec()
            })
            .unwrap_or_default()
    }

    #[cfg(test)]
    pub(crate) fn set_documentation(self, description: Option<Documentation>) -> CompletionItem {
        CompletionItem {
            documentation: description,
            ..self
        }
    }

    #[cfg(test)]
    pub(crate) fn set_insert_text(self, insert_text: Option<String>) -> CompletionItem {
        CompletionItem {
            insert_text,
            ..self
        }
    }

    pub(crate) fn insert_text(&self) -> Option<String> {
        self.insert_text.clone()
    }

    pub(crate) fn dispatches(&self) -> crate::app::Dispatches {
        match &self.edit {
            None => Dispatches::one(Dispatch::ToEditor(
                DispatchEditor::TryReplaceCurrentLongWord(
                    self.insert_text().unwrap_or_else(|| self.label()),
                ),
            ))
            .append(Dispatch::ToEditor(DispatchEditor::ApplyPositionalEdits(
                self.additional_text_edits(),
            ))),
            Some(edit) => {
                Dispatches::one(Dispatch::ToEditor(DispatchEditor::ApplyPositionalEdits(
                    Some(edit.clone())
                        .into_iter()
                        .chain(self.additional_text_edits())
                        .collect_vec(),
                )))
            }
        }
        .append_some(
            self.command()
                .clone()
                .map(|command| Dispatch::LspExecuteCommand {
                    command: command.into(),
                }),
        )
    }

    pub(crate) fn completion_item(&self) -> lsp_types::CompletionItem {
        self.completion_item.clone()
    }

    fn command(&self) -> Option<lsp_types::Command> {
        self.completion_item.command.clone()
    }
}

impl From<lsp_types::CompletionItem> for CompletionItem {
    fn from(item: lsp_types::CompletionItem) -> Self {
        Self {
            label: item.label.clone(),
            kind: item.kind,
            detail: item.detail.clone(),
            documentation: item.documentation.clone().map(|doc| doc.into()),
            sort_text: item.sort_text.clone(),
            insert_text: item.insert_text.clone(),
            edit: item.text_edit.clone().and_then(|edit| match edit {
                lsp_types::CompletionTextEdit::Edit(edit) => {
                    Some(CompletionItemEdit::PositionalEdit(PositionalEdit {
                        range: edit.range.start.into()..edit.range.end.into(),
                        new_text: edit.new_text,
                    }))
                }
                lsp_types::CompletionTextEdit::InsertAndReplace(_) => None,
            }),
            completion_item: item,
        }
    }
}
