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
            commit: "master",
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
            commit: "main",
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
            commit: "master",
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
            commit: "master",
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
            commit: "master",
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
            commit: "split_parser",
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
            commit: "master",
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
            commit: "main",
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
            commit: "master",
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
            commit: "main",
            subpath: None,
        }),
        highlight_query: None,
        formatter_command: None,
    }
}

const fn choice<T: Copy>(condition: bool, left: T, right: T) -> T {
    if condition {
        left
    } else {
        right
    }
}

const fn typescript(tsx: bool) -> Language {
    Language {
        lsp_language_id: Some(LanguageId::new(choice(
            tsx,
            "typescriptreact",
            "typescript",
        ))),
        extensions: choice(tsx, &["tsx"], &["ts"]),
        lsp_command: Some(LspCommand {
            command: Command("typescript-language-server", &["--stdio"]),
            ..LspCommand::default()
        }),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: choice(tsx, "tsx", "typescript"),
            url: "https://github.com/tree-sitter/tree-sitter-typescript",
            commit: "master",
            subpath: Some(choice(tsx, "tsx", "typescript")),
        }),
        formatter_command: Some(Command("prettierd", choice(tsx, &[".tsx"], &[".ts"]))),
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
            commit: "master",
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
            commit: "master",
        }),
        formatter_command: None,
        highlight_query: None,
    }
}
