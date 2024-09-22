use super::language::{Command, GrammarConfig, Language, LanguageId, LspCommand};

pub const LANGUAGES: &[&Language] = &[
    &bash(),
    &common_lisp(),
    &css(),
    &csv(),
    &dockerfile(),
    &graphql(),
    &html(),
    &javascript(true),
    &javascript(false),
    &just(),
    &json(),
    &nix(),
    &markdown(),
    &python(),
    &rescript(),
    &rust(),
    &sql(),
    &swift(),
    &toml(),
    &tree_sitter_query(),
    &typescript(false),
    &typescript(true),
    &xml(),
    &yaml(),
    &zig(),
];

const fn bash() -> Language {
    Language {
        lsp_language_id: Some(LanguageId::new("bash")),
        file_names: &[".bashrc", ".bash_profile", "bashrc", "bash_profile"],
        extensions: &["sh", "bash"],
        lsp_command: Some(LspCommand {
            command: Command("bash-language-server", &["start"]),
            ..LspCommand::default()
        }),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "bash",
            url: "https://github.com/tree-sitter/tree-sitter-bash",
            commit: "master",
            subpath: None,
        }),
        formatter_command: Some(Command("shfmt", &[".sh", ".bash"])),
        ..Language::new()
    }
}

const fn common_lisp() -> Language {
    Language {
        file_names: &[],
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
        file_names: &[],
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

const fn css() -> Language {
    Language {
        file_names: &[],
        extensions: &["css"],
        lsp_language_id: None,
        lsp_command: None,
        highlight_query: None,
        formatter_command: Some(Command("prettierd", &[".css"])),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "css",
            url: "https://github.com/tree-sitter/tree-sitter-css",
            commit: "master",
            subpath: None,
        }),
    }
}

const fn dockerfile() -> Language {
    Language {
        file_names: &["Dockerfile"],
        extensions: &[],
        lsp_language_id: None,
        lsp_command: None,
        highlight_query: None,
        formatter_command: None,
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "dockerfile",
            url: "https://github.com/camdencheek/tree-sitter-dockerfile",
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
        formatter_command: Some(Command("prettierd", &[".graphql"])),
        lsp_command: Some(LspCommand {
            command: Command("graphql-lsp", &["server", "-m", "stream"]),
            initialization_options: Some(r#"{ "graphql-config.load.legacy": true }"#),
        }),
        ..Language::new()
    }
}

const fn html() -> Language {
    Language {
        lsp_language_id: Some(LanguageId::new("html")),
        extensions: &["htm", "html"],
        lsp_command: Some(LspCommand {
            command: Command("emmet-language-server", &["--stdio"]),
            ..LspCommand::default()
        }),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "html",
            url: "https://github.com/tree-sitter/tree-sitter-html",
            commit: "master",
            subpath: None,
        }),
        formatter_command: Some(Command("prettierd", &[".html"])),
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
        file_names: &[],
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

const fn just() -> Language {
    Language {
        file_names: &["justfile"],
        extensions: &[],
        lsp_language_id: None,
        lsp_command: None,
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "just",
            url: "https://github.com/IndianBoy42/tree-sitter-just",
            commit: "main",
            subpath: None,
        }),
        highlight_query: None,
        formatter_command: None,
    }
}

const fn nix() -> Language {
    Language {
        file_names: &[],
        lsp_language_id: Some(LanguageId::new("nix")),
        lsp_command: Some(LspCommand {
            command: Command("nil", &[]),
            ..LspCommand::default()
        }),
        extensions: &["nix"],
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "nix",
            url: "https://github.com/nix-community/tree-sitter-nix",
            commit: "master",
            subpath: None,
        }),
        highlight_query: None,
        formatter_command: Some(Command("nixfmt", &[])),
    }
}

const fn markdown() -> Language {
    Language {
        lsp_language_id: Some(LanguageId::new("markdown")),
        extensions: &["md"],
        lsp_command: Some(LspCommand {
            command: Command("marksman", &["server"]),
            ..LspCommand::default()
        }),
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

const fn python() -> Language {
    Language {
        lsp_language_id: Some(LanguageId::new("python")),
        extensions: &["py"],
        lsp_command: Some(LspCommand {
            command: Command("pyright-langserver", &["--stdio"]),
            ..LspCommand::default()
        }),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "python",
            url: "https://github.com/tree-sitter/tree-sitter-python",
            commit: "master",
            subpath: None,
        }),
        formatter_command: Some(Command("ruff", &["format", "--stdin-filename", ".py"])),
        ..Language::new()
    }
}

const fn rescript() -> Language {
    Language {
        file_names: &[],
        lsp_language_id: Some(LanguageId::new("rescript")),
        lsp_command: Some(LspCommand {
            command: Command("./node_modules/.bin/rescript-language-server", &["--stdio"]),
            ..LspCommand::default()
        }),
        extensions: &["res"],
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "rescript",
            url: "https://github.com/rescript-lang/tree-sitter-rescript",
            commit: "main",
            subpath: None,
        }),
        highlight_query: None,
        formatter_command: Some(Command(
            "./node_modules/.bin/rescript",
            &["format", "-stdin", ".res"],
        )),
    }
}

const fn rust() -> Language {
    Language {
        file_names: &[],
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
        formatter_command: Some(Command("rustfmt", &["--edition=2021"])),
    }
}

const fn sql() -> Language {
    Language {
        lsp_language_id: Some(LanguageId::new("sql")),
        extensions: &["sql"],
        lsp_command: None,
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "sql",
            url: "https://github.com/DerekStride/tree-sitter-sql",
            commit: "25f94f998de79bae9df28add9782f9ea6ea0e2b8",
            subpath: None,
        }),
        formatter_command: Some(Command("sql-formatter", &["--language", "postgresql"])),
        ..Language::new()
    }
}

const fn swift() -> Language {
    Language {
        lsp_language_id: Some(LanguageId::new("swift")),
        extensions: &["swift"],
        lsp_command: Some(LspCommand {
            command: Command("sourcekit-lsp", &[]),
            ..LspCommand::default()
        }),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "swift",
            url: "https://github.com/alex-pinkus/tree-sitter-swift",
            commit: "with-generated-files",
            subpath: None,
        }),
        formatter_command: Some(Command("swiftformat", &[])),
        ..Language::new()
    }
}

const fn toml() -> Language {
    Language {
        file_names: &[],
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
        file_names: &[],
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
        file_names: &[],
        lsp_language_id: Some(LanguageId::new("xml")),
        extensions: &["xml"],
        lsp_command: None,
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "xml",
            url: "https://github.com/tree-sitter-grammars/tree-sitter-xml",
            subpath: Some("xml"),
            commit: "master",
        }),
        formatter_command: None,
        highlight_query: None,
    }
}

const fn yaml() -> Language {
    Language {
        file_names: &[],
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

const fn zig() -> Language {
    Language {
        file_names: &[],
        lsp_language_id: Some(LanguageId::new("zig")),
        extensions: &["zig"],
        lsp_command: Some(LspCommand {
            command: Command("zls", &[]),
            ..LspCommand::default()
        }),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "zig",
            url: "https://github.com/tree-sitter-grammars/tree-sitter-zig",
            subpath: None,
            commit: "master",
        }),
        formatter_command: Some(Command("zig", &["fmt", "--stdin"])),
        highlight_query: None,
    }
}
