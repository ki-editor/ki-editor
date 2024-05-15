use itertools::Itertools;

use crate::{
    app::{Dispatch, RequestParams},
    components::dropdown::DropdownItem,
};

use super::workspace_edit::WorkspaceEdit;

#[derive(Debug, Clone, PartialEq, Eq)]
/// Refer https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#codeAction
pub(crate) struct CodeAction {
    pub(crate) title: String,
    pub(crate) kind: Option<String>,
    pub(crate) edit: Option<WorkspaceEdit>,
    pub(crate) command: Option<Command>,
}

#[derive(Debug, Clone)]
pub(crate) struct Command(lsp_types::Command);
impl Command {
    pub(crate) fn arguments(&self) -> Vec<serde_json::Value> {
        self.0.arguments.clone().unwrap_or_default()
    }

    pub(crate) fn command(&self) -> String {
        self.0.command.clone()
    }
}

impl PartialEq for Command {
    fn eq(&self, other: &Self) -> bool {
        self.0.command.eq(&other.0.command)
    }
}

impl Eq for Command {}

impl CodeAction {
    pub(crate) fn into_dropdown_item(self, params: Option<RequestParams>) -> DropdownItem {
        let value = self;
        DropdownItem::new(value.title)
            .set_group(Some(
                value
                    .kind
                    .and_then(|kind| if kind.is_empty() { None } else { Some(kind) })
                    .unwrap_or("Misc.".to_string()),
            ))
            .set_dispatches(
                value
                    .edit
                    .map(Dispatch::ApplyWorkspaceEdit)
                    .into_iter()
                    // A command this code action executes. If a code action
                    // provides an edit and a command, first the edit is
                    // executed and then the command.
                    // Refer https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#codeAction
                    .chain(params.and_then(|params| {
                        value
                            .command
                            .map(|command| Dispatch::LspExecuteCommand { command, params })
                    }))
                    .collect_vec()
                    .into(),
            )
    }
}

impl PartialOrd for CodeAction {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CodeAction {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.title.cmp(&other.title)
    }
}

impl TryFrom<lsp_types::CodeAction> for CodeAction {
    type Error = anyhow::Error;

    fn try_from(value: lsp_types::CodeAction) -> Result<Self, Self::Error> {
        log::info!("CodeAction: {:#?}", value);

        let title = value.title;
        Ok(CodeAction {
            title,
            kind: value.kind.map(|kind| kind.as_str().to_string()),
            edit: value.edit.map(WorkspaceEdit::try_from).transpose()?,
            command: value.command.map(Command),
        })
    }
}
