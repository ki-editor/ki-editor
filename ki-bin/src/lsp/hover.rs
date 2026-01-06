#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Hover {
    pub(crate) contents: Vec<String>,
}

impl From<lsp_types::Hover> for Hover {
    fn from(hover: lsp_types::Hover) -> Self {
        let contents = match hover.contents {
            lsp_types::HoverContents::Scalar(marked_string) => {
                vec![marked_string_to_string(marked_string)]
            }
            lsp_types::HoverContents::Array(contents) => contents
                .into_iter()
                .map(marked_string_to_string)
                .collect::<Vec<_>>(),
            lsp_types::HoverContents::Markup(content) => vec![content.value],
        };
        Hover { contents }
    }
}

pub(crate) fn marked_string_to_string(marked_string: lsp_types::MarkedString) -> String {
    match marked_string {
        lsp_types::MarkedString::String(string) => string,
        lsp_types::MarkedString::LanguageString(language_string) => language_string.value,
    }
}
