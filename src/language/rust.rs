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

    fn tree_sitter_grammar_config(&self) -> Option<GrammarConfiguration> {
        Some(GrammarConfiguration::remote(
            "rust",
            "https://github.com/tree-sitter/tree-sitter-rust",
            "afb6000a71fb9dff3f47f90d412ec080ae12bbb4",
            None,
        ))
    }

    fn highlight_query(&self) -> Option<&'static str> {
        Some(tree_sitter_rust::HIGHLIGHT_QUERY)
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
