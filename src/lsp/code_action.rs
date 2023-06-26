use super::workspace_edit::WorkspaceEdit;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeAction {
    pub title: String,
    pub kind: Option<String>,
    pub edit: WorkspaceEdit,
}

impl CodeAction {
    pub fn title(&self) -> String {
        match &self.kind {
            Some(kind) => format!("({}) {}", kind, self.title),
            None => self.title.clone(),
        }
    }
}

impl PartialOrd for CodeAction {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.title().partial_cmp(&other.title())
    }
}

impl Ord for CodeAction {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap_or(std::cmp::Ordering::Equal)
    }
}

impl TryFrom<lsp_types::CodeAction> for CodeAction {
    type Error = anyhow::Error;

    fn try_from(value: lsp_types::CodeAction) -> Result<Self, Self::Error> {
        log::info!("CodeAction: {:#?}", value);

        let title = value.title;
        let edit = value
            .edit
            .ok_or_else(|| anyhow::anyhow!("CodeAction edit is missing"))?;
        Ok(CodeAction {
            title,
            kind: value.kind.map(|kind| kind.as_str().to_string()),
            edit: WorkspaceEdit::try_from(edit)?,
        })
    }
}
