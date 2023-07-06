use super::*;

#[derive(Debug, Clone)]
pub struct Typescript;

impl Language for Typescript {
    fn extension(&self) -> &'static str {
        "ts"
    }

    fn lsp_process_command(&self) -> Option<ProcessCommand> {
        Some(ProcessCommand::new(
            "typescript-language-server",
            &["--stdio"],
        ))
    }

    fn id(&self) -> LanguageId {
        LanguageId::new("typescript")
    }

    fn tree_sitter_language(&self) -> Option<tree_sitter::Language> {
        Some(tree_sitter_typescript::language_typescript())
    }

    fn highlight_query(&self) -> Option<&'static str> {
        Some(tree_sitter_typescript::HIGHLIGHT_QUERY)
    }

    fn formatter_command(&self) -> Option<(ProcessCommand, FormatterTestCase)> {
        Some((
            ProcessCommand::new("prettierd", &[".ts"]),
            FormatterTestCase {
                input: "let x:Int=1",
                expected: "let x: Int = 1;\n",
            },
        ))
    }
}
