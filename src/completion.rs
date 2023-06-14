#[derive(Debug, Clone)]
pub struct Completion {
    pub items: Vec<CompletionItem>,
    pub trigger_characters: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CompletionItem {
    label: String,
    documentation: Option<String>,
}

impl CompletionItem {
    pub fn label(&self) -> String {
        self.label.clone()
    }

    pub fn documentation(&self) -> String {
        self.documentation.clone().unwrap_or_default()
    }
}

impl From<lsp_types::CompletionItem> for CompletionItem {
    fn from(item: lsp_types::CompletionItem) -> Self {
        Self {
            label: item.label,
            documentation: item.documentation.map(|doc| match doc {
                lsp_types::Documentation::String(s) => s,
                lsp_types::Documentation::MarkupContent(content) => content.value,
            }),
        }
    }
}
