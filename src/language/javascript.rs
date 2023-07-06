use super::*;

#[derive(Debug, Clone)]
pub struct Javascript;

impl Language for Javascript {
    fn extension(&self) -> &'static str {
        "js"
    }

    fn lsp_process_command(&self) -> Option<ProcessCommand> {
        Some(ProcessCommand::new(
            "typescript-language-server",
            &["--stdio"],
        ))
    }

    fn id(&self) -> LanguageId {
        LanguageId::new("javascript")
    }

    fn tree_sitter_language(&self) -> Option<tree_sitter::Language> {
        Some(tree_sitter_javascript::language())
    }

    fn highlight_query(&self) -> Option<&'static str> {
        Some(tree_sitter_javascript::HIGHLIGHT_QUERY)
    }

    fn formatter_command(&self) -> Option<(ProcessCommand, FormatterTestCase)> {
        Some((
            ProcessCommand::new("prettierd", &[".js"]),
            FormatterTestCase {
                input: "let x=1",
                expected: "let x = 1;\n",
            },
        ))
    }
}
