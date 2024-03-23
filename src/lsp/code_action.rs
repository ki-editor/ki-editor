use crate::components::dropdown::DropdownItem;

use super::workspace_edit::WorkspaceEdit;

#[derive(Debug, Clone, PartialEq, Eq)]
/// Refer https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#codeAction
pub struct CodeAction {
    pub title: String,
    pub kind: Option<String>,
    pub edit: Option<WorkspaceEdit>,
    pub command: Option<Command>,
}

#[derive(Debug, Clone)]
pub struct Command(lsp_types::Command);
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
    pub fn title(&self) -> String {
        self.title.clone()
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
