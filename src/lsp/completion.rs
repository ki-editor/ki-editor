use std::ops::Range;

use itertools::Itertools;
use lsp_types::CompletionItemKind;
use shared::icons::get_icon_config;

use crate::{
    components::{dropdown::DropdownItem, suggestive_editor::Info},
    position::Position,
};

use super::documentation::Documentation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Completion {
    pub(crate) items: Vec<DropdownItem>,
    pub(crate) trigger_characters: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompletionItem {
    pub(crate) label: String,
    pub(crate) kind: Option<CompletionItemKind>,
    pub(crate) detail: Option<String>,
    pub(crate) documentation: Option<Documentation>,
    pub(crate) sort_text: Option<String>,
    pub(crate) insert_text: Option<String>,
    pub(crate) edit: Option<CompletionItemEdit>,
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

impl PartialOrd for CompletionItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CompletionItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap_or(std::cmp::Ordering::Equal)
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
        }
    }

    pub(crate) fn label(&self) -> String {
        self.label.clone()
    }

    pub(crate) fn documentation(&self) -> Option<Documentation> {
        self.documentation.clone()
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
}

impl From<lsp_types::CompletionItem> for CompletionItem {
    fn from(item: lsp_types::CompletionItem) -> Self {
        Self {
            label: item.label,
            kind: item.kind,
            detail: item.detail,
            documentation: item.documentation.map(|doc| doc.into()),
            sort_text: item.sort_text,
            insert_text: item.insert_text,
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
