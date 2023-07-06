use super::*;

#[derive(Debug, Clone)]
pub struct JavascriptReact;

impl Language for JavascriptReact {
    fn extension(&self) -> &'static str {
        "jsx"
    }

    fn lsp_process_command(&self) -> Option<ProcessCommand> {
        Some(ProcessCommand::new(
            "typescript-language-server",
            &["--stdio"],
        ))
    }

    fn id(&self) -> LanguageId {
        LanguageId::new("javascriptreact")
    }

    fn tree_sitter_language(&self) -> Option<tree_sitter::Language> {
        Some(tree_sitter_javascript::language())
    }

    fn highlight_query(&self) -> Option<&'static str> {
        Some(tree_sitter_javascript::JSX_HIGHLIGHT_QUERY)
    }

    fn formatter_command(&self) -> Option<(ProcessCommand, FormatterTestCase)> {
        Some((
            ProcessCommand::new("prettierd", &[".jsx"]),
            FormatterTestCase {
                input: "let x=<x ></ x>",
                expected: "let x = <x></x>;\n",
            },
        ))
    }
}
