use super::*;

#[derive(Debug, Clone)]
pub struct TypescriptReact;

impl Language for TypescriptReact {
    fn extension(&self) -> &'static str {
        "tsx"
    }

    fn lsp_process_command(&self) -> Option<ProcessCommand> {
        Some(ProcessCommand::new(
            "typescript-language-server",
            &["--stdio"],
        ))
    }

    fn id(&self) -> LanguageId {
        LanguageId::new("typescriptreact")
    }

    fn tree_sitter_language(&self) -> Option<tree_sitter::Language> {
        Some(tree_sitter_typescript::language_tsx())
    }

    fn highlight_query(&self) -> Option<&'static str> {
        Some(tree_sitter_typescript::HIGHLIGHT_QUERY)
    }

    fn formatter_command(&self) -> Option<(ProcessCommand, FormatterTestCase)> {
        Some((
            ProcessCommand::new("prettierd", &[".tsx"]),
            FormatterTestCase {
                input: "let x:Int=<x ></ x>",
                expected: "let x: Int = <x></x>;\n",
            },
        ))
    }
}
