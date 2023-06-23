use super::*;

#[derive(Debug, Clone)]
pub struct Rust;

impl Language for Rust {
    fn extension(&self) -> &'static str {
        "rs"
    }

    fn lsp_process_command(&self) -> Option<ProcessCommand> {
        Some(ProcessCommand::new("rust-analyzer", &[]))
    }

    fn id(&self) -> LanguageId {
        LanguageId::new("rust")
    }

    fn tree_sitter_language(&self) -> Option<tree_sitter::Language> {
        Some(tree_sitter_rust::language())
    }

    fn formatter_command(&self) -> Option<(ProcessCommand, FormatterTestCase)> {
        Some((
            ProcessCommand::new("rustfmt", &[]),
            FormatterTestCase {
                input: "fn main(){}",
                expected: "fn main() {}\n",
            },
        ))
    }
}
