use super::language::{Command, GrammarConfig, Language, LanguageId, LspCommand};

pub const LANGUAGES: &[&Language] = &[
    &common_lisp(),
    &csv(),
    &graphql(),
    &javascript(true),
    &javascript(false),
    &json(),
    &markdown(),
    &rust(),
    &sql(),
    &toml(),
    &tree_sitter_query(),
    &typescript(false),
    &typescript(true),
    &xml(),
    &yaml(),
];

const fn common_lisp() -> Language {
    Language {
        lsp_language_id: None,
        lsp_command: None,
        extensions: &["lisp", "lsp", "l", "cl", "fasl", "sbcl", "el"],
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "commonlisp",
            url: "https://github.com/theHamsta/tree-sitter-commonlisp",
            commit: "5153dbbc70e4cc2324320c1bdae020d31079c7c0",
            subpath: None,
        }),
        highlight_query: None,
        formatter_command: None,
    }
}
const fn csv() -> Language {
    Language {
        extensions: &["csv"],
        lsp_language_id: None,
        lsp_command: None,
        highlight_query: None,
        formatter_command: None,
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "csv",
            url: "https://github.com/arnau/tree-sitter-csv",
            commit: "ae0728a5f00ad8f02357c20e61249af1a52e89b4",
            subpath: None,
        }),
    }
}

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

const fn json() -> Language {
    Language {
        extensions: &["json"],
        lsp_language_id: None,
        lsp_command: None,
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "json",
            url: "https://github.com/tree-sitter/tree-sitter-json",
            commit: "ca3f8919800e3c1ad4508de3bfd7b0b860ce434f",
            subpath: None,
        }),
        highlight_query: None,
        formatter_command: Some(Command("prettierd", &[".json"])),
    }
}

const fn markdown() -> Language {
    Language {
        lsp_language_id: None,
        extensions: &["md"],
        lsp_command: None,
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "markdown",
            url: "https://github.com/MDeiml/tree-sitter-markdown",
            commit: "aaf76797aa8ecd9a5e78e0ec3681941de6c945ee",
            subpath: Some("tree-sitter-markdown"),
        }),
        formatter_command: Some(Command("prettierd", &[".md"])),
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
        highlight_query: None,
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

const fn toml() -> Language {
    Language {
        extensions: &["toml"],
        lsp_language_id: None,
        lsp_command: None,
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "toml",
            url: "https://github.com/ikatyang/tree-sitter-toml",
            commit: "8bd2056818b21860e3d756b5a58c4f6e05fb744e",
            subpath: None,
        }),
        highlight_query: None,
        formatter_command: None,
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
            id: if tsx { "tsx" } else { "typescript" },
            url: "https://github.com/tree-sitter/tree-sitter-typescript",
            commit: "b1bf4825d9eaa0f3bdeb1e52f099533328acfbdf",
            subpath: Some(if tsx { "tsx" } else { "typescript" }),
        }),
        formatter_command: Some(Command("prettierd", if tsx { &[".tsx"] } else { &[".ts"] })),
        ..Language::new()
    }
}

const fn xml() -> Language {
    Language {
        lsp_language_id: Some(LanguageId::new("xml")),
        extensions: &["xml", "svg"],
        lsp_command: None,
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "xml",
            url: "https://github.com/ObserverOfTime/tree-sitter-xml",
            subpath: Some("tree-sitter-xml"),
            commit: "a3bfa1ae7e8400ab81a6358f5e8d2983f5dd0697",
        }),
        formatter_command: None,
        highlight_query: None,
    }
}

const fn yaml() -> Language {
    Language {
        lsp_language_id: Some(LanguageId::new("yaml")),
        extensions: &["yaml", "yml"],
        lsp_command: None,
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "yaml",
            url: "https://github.com/ikatyang/tree-sitter-yaml",
            subpath: None,
            commit: "0e36bed171768908f331ff7dff9d956bae016efb",
        }),
        formatter_command: None,
        highlight_query: None,
    }
}
