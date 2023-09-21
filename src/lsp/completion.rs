use std::ops::Range;

use lsp_types::CompletionItemKind;

use crate::position::Position;

use super::documentation::Documentation;

#[derive(Debug, Clone)]
pub struct Completion {
    pub items: Vec<CompletionItem>,
    pub trigger_characters: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionItem {
    pub label: String,
    pub kind: Option<CompletionItemKind>,
    pub detail: Option<String>,
    pub documentation: Option<Documentation>,
    pub sort_text: Option<String>,
    pub edit: Option<CompletionItemEdit>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionItemEdit {
    PositionalEdit(PositionalEdit),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionalEdit {
    pub range: Range<Position>,
    pub new_text: String,
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

impl PartialOrd for CompletionItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self.sort_text.as_ref(), other.sort_text.as_ref()) {
            (Some(a), Some(b)) => a.partial_cmp(b),
            _ => self.label.partial_cmp(&other.label),
        }
    }
}

impl Ord for CompletionItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap_or(std::cmp::Ordering::Equal)
    }
}

impl CompletionItem {
    pub fn from_label(label: String) -> Self {
        Self {
            label,
            kind: None,
            detail: None,
            documentation: None,
            sort_text: None,
            edit: None,
        }
    }

    pub fn label(&self) -> String {
        self.label.clone()
    }

    pub fn documentation(&self) -> Option<Documentation> {
        self.documentation.clone()
    }

    pub fn set_documentation(self, description: Option<Documentation>) -> CompletionItem {
        CompletionItem {
            documentation: description,
            ..self
        }
    }
}

impl From<lsp_types::CompletionItem> for CompletionItem {
    fn from(item: lsp_types::CompletionItem) -> Self {
        Self {
            label: item.label,
            kind: item.kind,
            detail: item.detail,
            documentation: item.documentation.map(|doc| doc.into()),
            sort_text: item.sort_text,
            edit: item.text_edit.and_then(|edit| match edit {
                lsp_types::CompletionTextEdit::Edit(edit) => {
                    Some(CompletionItemEdit::PositionalEdit(PositionalEdit {
                        range: edit.range.start.into()..edit.range.end.into(),
                        new_text: edit.new_text,
                    }))
                }
                lsp_types::CompletionTextEdit::InsertAndReplace(_) => None,
            }),
        }
    }
}
