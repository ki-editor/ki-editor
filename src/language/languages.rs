use super::{Command, GrammarConfig, Language, LanguageId, LspCommand};

pub const LANGUAGES: &[&Language] = &[
    &graphql(),
    &javascript(true),
    &javascript(false),
    &rust(),
    &sql(),
    &tree_sitter_query(),
    &typescript(false),
    &typescript(true),
];

const fn graphql() -> Language {
    Language {
        lsp_language_id: Some(LanguageId::new("graphql")),
        extensions: &["graphql", "gql"],
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "graphql",
            url: "https://github.com/bkegley/tree-sitter-graphql",
            commit: "5e66e961eee421786bdda8495ed1db045e06b5fe",
            subpath: None,
        }),
        formatter_command: Some(Command("prettierd", &[".graphql", ".gql"])),
        ..Language::new()
    }
}

const fn javascript(jsx: bool) -> Language {
    Language {
        lsp_language_id: Some(LanguageId::new(if jsx {
            "javascriptreact"
        } else {
            "javascript"
        })),
        extensions: if jsx { &["jsx"] } else { &["js"] },
        lsp_command: Some(LspCommand {
            command: Command("typescript-language-server", &["--stdio"]),
            ..LspCommand::default()
        }),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: if jsx { "jsx" } else { "javascript" },
            url: "https://github.com/tree-sitter/tree-sitter-javascript",
            commit: "c69aabab53609d00e8e198ab902e4fde4b8e449f",
            subpath: None,
        }),
        formatter_command: Some(Command("prettierd", if jsx { &[".jsx"] } else { &[".js"] })),
        ..Language::new()
    }
}

const fn rust() -> Language {
    Language {
        lsp_language_id: Some(LanguageId::new("rust")),
        extensions: &["rs"],
        lsp_command: Some(LspCommand {
            command: Command("rust-analyzer", &[]),
            ..LspCommand::default()
        }),
        highlight_query: Some(include_str!("../../contrib/queries/rust/highlights.scm")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "rust",
            url: "https://github.com/tree-sitter/tree-sitter-rust",
            commit: "afb6000a71fb9dff3f47f90d412ec080ae12bbb4",
            subpath: None,
        }),
        formatter_command: Some(Command("rustfmt", &[])),
        ..Language::new()
    }
}

const fn sql() -> Language {
    Language {
        lsp_language_id: Some(LanguageId::new("sql")),
        extensions: &["sql"],
        lsp_command: None,
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "sql",
            url: "https://github.com/m-novikov/tree-sitter-sql",
            commit: "63cd04238b18c7f55987465b3252597da47b6924",
            subpath: None,
        }),
        formatter_command: Some(Command("sql-formatter", &[])),
        ..Language::new()
    }
}
const fn tree_sitter_query() -> Language {
    Language {
        extensions: &["scm"],
        lsp_language_id: None,
        lsp_command: None,
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "tsq",
            url: "https://github.com/tree-sitter/tree-sitter-tsq",
            commit: "b665659d3238e6036e22ed0e24935e60efb39415",
            subpath: None,
        }),
        highlight_query: None,
        formatter_command: None,
    }
}

const fn typescript(tsx: bool) -> Language {
    Language {
        lsp_language_id: Some(LanguageId::new(if tsx {
            "typescriptreact"
        } else {
            "typescript"
        })),
        extensions: if tsx { &["tsx"] } else { &["ts"] },
        lsp_command: Some(LspCommand {
            command: Command("typescript-language-server", &["--stdio"]),
            ..LspCommand::default()
        }),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "typescript",
            url: "https://github.com/tree-sitter/tree-sitter-typescript",
            commit: "b1bf4825d9eaa0f3bdeb1e52f099533328acfbdf",
            subpath: Some(if tsx { "tsx" } else { "typescript" }),
        }),
        formatter_command: Some(Command("prettierd", if tsx { &[".tsx"] } else { &[".ts"] })),
        ..Language::new()
    }
}
