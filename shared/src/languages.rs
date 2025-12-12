use crate::language::{CargoLinkedTreesitterLanguage, GrammarConfigKind};

use super::language::{Command, GrammarConfig, Language, LanguageId, LspCommand};

pub const LANGUAGES: &[&Language] = &[
    &bash(),
    &fish(),
    &unison(),
    &c(),
    &racket(),
    &scheme(),
    &common_lisp(),
    &cpp(),
    &c_sharp(),
    &css(),
    &csv(),
    &diff(),
    &dockerfile(),
    &elixir(),
    &f_sharp(),
    &gitattributes(),
    &gitcommit(),
    &gitconfig(),
    &gitignore(),
    &gitrebase(),
    &gleam(),
    &golang(),
    &graphql(),
    &hare(),
    &heex(),
    &html(),
    &idris(),
    &haskell(),
    &javascript(),
    &javascript_react(),
    &svelte(),
    &json(),
    &julia(),
    &just(),
    &ki_quickfix(),
    &lua(),
    &markdown(),
    &nix(),
    &ocaml(),
    &ocaml_interface(),
    &odin(),
    &dune(),
    &python(),
    &rescript(),
    &roc(),
    &ruby(),
    &rust(),
    &sql(),
    &swift(),
    &typst(),
    &toml(),
    &tree_sitter_query(),
    &typescript(),
    &typescript_react(),
    &xml(),
    &yaml(),
    &zig(),
];

const fn bash() -> Language {
    Language {
        extensions: &["sh", "bash"],
        file_names: &[".bashrc", ".bash_profile", "bashrc", "bash_profile"],
        formatter_command: Some(Command("shfmt", &[".sh", ".bash"])),
        lsp_command: Some(LspCommand {
            command: Command("bash-language-server", &["start"]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("bash")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "bash",
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Bash),
        }),
        line_comment_prefix: Some("#"),
        ..Language::new()
    }
}

const fn fish() -> Language {
    Language {
        extensions: &["fish"],
        formatter_command: Some(Command("fish --no-execute ", &[".fish"])),
        lsp_command: Some(LspCommand {
            command: Command("fish-lsp", &["start"]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("fish")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "fish",
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Fish),
        }),
        line_comment_prefix: Some("#"),
        ..Language::new()
    }
}

const fn c() -> Language {
    Language {
        extensions: &["c", "h"],
        formatter_command: Some(Command("clang-format", &[])),
        lsp_command: Some(LspCommand {
            command: Command("clangd", &[]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("c")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "c",
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::C),
        }),
        line_comment_prefix: Some("//"),
        block_comment_affixes: Some(("/*", "*/")),
        ..Language::new()
    }
}

const fn racket() -> Language {
    Language {
        extensions: &["rkt", "rktd", "rktl", "scrbl", "zuo"],
        lsp_command: Some(LspCommand {
            command: Command("racket", &[]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("racket")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "racket",
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Scheme),
        }),
        line_comment_prefix: Some(";"),
        block_comment_affixes: Some(("#|", "|#")),
        ..Language::new()
    }
}

const fn scheme() -> Language {
    Language {
        extensions: &["ss", "scm", "sld"],
        // lsp_language_id: Some(LanguageId::new("scheme")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "scheme",
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Scheme),
        }),
        line_comment_prefix: Some(";"),
        block_comment_affixes: Some(("#|", "|#")),
        ..Language::new()
    }
}

const fn common_lisp() -> Language {
    Language {
        extensions: &[
            "lisp", "lsp", "l", "cl", "fasl", "sbcl", "el", "asd", "ny", "podsl", "sexp",
        ],
        lsp_command: Some(LspCommand {
            command: Command("cl-lsp", &[]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("commonlisp")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "commonlisp",
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Scheme),
        }),
        line_comment_prefix: Some(";"),
        ..Language::new()
    }
}

const fn cpp() -> Language {
    Language {
        extensions: &[
            "cc", "hh", "c++", "cpp", "hpp", "h", "ipp", "tpp", "cxx", "hxx", "ixx", "txx", "ino",
            "cu", "cuh", "cppm", "h++", "ii", "inl",
        ],
        formatter_command: Some(Command("clang-format", &[])),
        lsp_command: Some(LspCommand {
            command: Command("clangd", &[]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("cpp")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "cpp",
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::CPP),
        }),
        line_comment_prefix: Some("//"),
        block_comment_affixes: Some(("/*", "*/")),
        ..Language::new()
    }
}

const fn csv() -> Language {
    Language {
        extensions: &["csv"],
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "csv",
            kind: GrammarConfigKind::FromSource {
                url: "https://github.com/arnau/tree-sitter-csv",
                commit: "main",
                subpath: None,
            },
        }),
        ..Language::new()
    }
}

const fn c_sharp() -> Language {
    Language {
        extensions: &["cs", "csx", "cake"],
        formatter_command: Some(Command("csharpier", &["format", "--write-stdout"])),
        lsp_command: Some(LspCommand {
            command: Command("omnisharp", &["--languageserver"]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("c_sharp")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "c_sharp",
            kind: GrammarConfigKind::FromSource {
                url: "https://github.com/tree-sitter/tree-sitter-c-sharp",
                subpath: None,
                commit: "master",
            },
        }),
        line_comment_prefix: Some("//"),
        block_comment_affixes: Some(("/*", "*/")),
        ..Language::new()
    }
}

const fn css() -> Language {
    Language {
        extensions: &["css"],
        formatter_command: Some(Command("prettierd", &[".css"])),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "css",
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::CSS),
        }),
        block_comment_affixes: Some(("/*", "*/")),
        ..Language::new()
    }
}

const fn diff() -> Language {
    Language {
        extensions: &["diff"],
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "diff",
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Diff),
        }),
        ..Language::new()
    }
}

const fn dockerfile() -> Language {
    Language {
        file_names: &["Dockerfile"],
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "dockerfile",
            kind: GrammarConfigKind::FromSource {
                url: "https://github.com/camdencheek/tree-sitter-dockerfile",
                commit: "main",
                subpath: None,
            },
        }),
        line_comment_prefix: Some("#"),
        ..Language::new()
    }
}

const fn elixir() -> Language {
    Language {
        extensions: &["ex", "exs"],
        formatter_command: Some(Command("mix", &["format", "-"])),
        lsp_command: Some(LspCommand {
            command: Command("elixir-ls", &[]),
            initialization_options: None,
        }),
        lsp_language_id: Some(LanguageId::new("elixir")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "elixir",
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Elixir),
        }),
        line_comment_prefix: Some("#"),
        ..Language::new()
    }
}

const fn f_sharp() -> Language {
    Language {
        extensions: &["fs", "fsi", "fsx", "fsscript"],
        formatter_command: None,
        lsp_command: Some(LspCommand {
            // Use --log-file and --log-level arguments to debug fsautocomplete issues.
            // Example: --log-file /path/to/fsac.log --log-level debug
            command: Command("fsautocomplete", &[]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("fsharp")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "fsharp",
            kind: GrammarConfigKind::FromSource {
                url: "https://github.com/ionide/tree-sitter-fsharp.git",
                commit: "main",
                subpath: Some("fsharp"),
            },
        }),
        line_comment_prefix: Some("//"),
        block_comment_affixes: Some(("(*", "*)")),
        ..Language::new()
    }
}

const fn gitattributes() -> Language {
    Language {
        file_names: &[".gitattributes"],
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "gitattributes",
            kind: GrammarConfigKind::FromSource {
                url: "https://github.com/tree-sitter-grammars/tree-sitter-gitattributes",
                commit: "master",
                subpath: None,
            },
        }),
        line_comment_prefix: Some("#"),
        ..Language::new()
    }
}

const fn gitcommit() -> Language {
    Language {
        file_names: &["COMMIT_EDITMSG"],
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "gitcommit",
            kind: GrammarConfigKind::FromSource {
                url: "https://github.com/gbprod/tree-sitter-gitcommit",
                commit: "main",
                subpath: None,
            },
        }),
        line_comment_prefix: Some("#"),
        ..Language::new()
    }
}

const fn gitconfig() -> Language {
    Language {
        file_names: &[".gitconfig"],
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "git_config",
            kind: GrammarConfigKind::FromSource {
                url: "https://github.com/the-mikedavis/tree-sitter-git-config",
                commit: "main",
                subpath: None,
            },
        }),
        line_comment_prefix: Some("#"),
        ..Language::new()
    }
}

const fn gitignore() -> Language {
    Language {
        file_names: &[".gitignore"],
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "gitignore",
            kind: GrammarConfigKind::FromSource {
                url: "https://github.com/shunsambongi/tree-sitter-gitignore",
                commit: "main",
                subpath: None,
            },
        }),
        line_comment_prefix: Some("#"),
        ..Language::new()
    }
}

const fn gitrebase() -> Language {
    Language {
        file_names: &["git-rebase-todo"],
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "git_rebase",
            kind: GrammarConfigKind::FromSource {
                url: "https://github.com/the-mikedavis/tree-sitter-git-rebase",
                commit: "main",
                subpath: None,
            },
        }),
        line_comment_prefix: Some("#"),
        ..Language::new()
    }
}

const fn gleam() -> Language {
    Language {
        extensions: &["gleam"],
        formatter_command: Some(Command("gleam", &["format", "--stdin"])),
        lsp_command: Some(LspCommand {
            command: Command("gleam", &["lsp"]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("gleam")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "gleam",
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Gleam),
        }),
        line_comment_prefix: Some("//"),
        ..Language::new()
    }
}

const fn golang() -> Language {
    Language {
        extensions: &["go"],
        formatter_command: Some(Command("gofmt", &[])),
        lsp_command: Some(LspCommand {
            command: Command("gopls", &[]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("go")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "go",
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Go),
        }),
        line_comment_prefix: Some("//"),
        block_comment_affixes: Some(("/*", "*/")),
        ..Language::new()
    }
}

const fn graphql() -> Language {
    Language {
        extensions: &["graphql", "gql"],
        formatter_command: Some(Command("prettierd", &[".graphql"])),
        lsp_command: Some(LspCommand {
            command: Command("graphql-lsp", &["server", "-m", "stream"]),
            initialization_options: Some(r#"{ "graphql-config.load.legacy": true }"#),
        }),
        lsp_language_id: Some(LanguageId::new("graphql")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "graphql",
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Graphql),
        }),
        line_comment_prefix: Some("#"),
        ..Language::new()
    }
}

const fn hare() -> Language {
    Language {
        extensions: &["ha"],
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "hare",
            kind: GrammarConfigKind::FromSource {
                url: "https://github.com/tree-sitter-grammars/tree-sitter-hare",
                commit: "master",
                subpath: None,
            },
        }),
        ..Language::new()
    }
}

const fn heex() -> Language {
    Language {
        extensions: &["heex"],
        formatter_command: Some(Command(
            "mix",
            &["format", "--stdin-filename", "file.heex", "-"],
        )),
        lsp_command: Some(LspCommand {
            command: Command("elixir-ls", &[]),
            initialization_options: None,
        }),
        lsp_language_id: Some(LanguageId::new("heex")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "heex",
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Heex),
        }),
        block_comment_affixes: Some(("<!--", "-->")),
        ..Language::new()
    }
}

const fn html() -> Language {
    Language {
        extensions: &["htm", "html", "svg"],
        formatter_command: Some(Command("prettierd", &[".html"])),
        lsp_command: Some(LspCommand {
            command: Command("emmet-language-server", &["--stdio"]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("html")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "html",
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::HTML),
        }),
        block_comment_affixes: Some(("<!--", "-->")),
        ..Language::new()
    }
}

const fn idris() -> Language {
    Language {
        extensions: &["idr", "lidr", "ipkg"],
        lsp_command: Some(LspCommand {
            command: Command("idris2-lsp", &[]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("idris")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "idris",
            kind: GrammarConfigKind::FromSource {
                url: "https://github.com/kayhide/tree-sitter-idris",
                commit: "main",
                subpath: None,
            },
        }),
        ..Language::new()
    }
}

const fn haskell() -> Language {
    Language {
        extensions: &["hs"],
        lsp_command: Some(LspCommand {
            command: Command("haskell-language-server-wrapper", &["--lsp"]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("haskell")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "haskell",
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Haskell),
        }),
        line_comment_prefix: Some("--"),
        block_comment_affixes: Some(("{-", "-}")),
        ..Language::new()
    }
}

const fn javascript() -> Language {
    Language {
        extensions: &["js", "mjs", "cjs"],
        formatter_command: Some(Command("prettierd", &[".js"])),
        lsp_command: Some(LspCommand {
            command: Command("typescript-language-server", &["--stdio"]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("javascript")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "javascript",
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Javascript),
        }),
        line_comment_prefix: Some("//"),
        block_comment_affixes: Some(("/*", "*/")),
        ..Language::new()
    }
}

const fn javascript_react() -> Language {
    Language {
        extensions: &["jsx"],
        formatter_command: Some(Command("prettierd", &[".jsx"])),
        lsp_command: Some(LspCommand {
            command: Command("typescript-language-server", &["--stdio"]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("javascriptreact")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "jsx",
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::JSX),
        }),
        line_comment_prefix: Some("//"),
        block_comment_affixes: Some(("/*", "*/")),
        ..Language::new()
    }
}

const fn svelte() -> Language {
    Language {
        extensions: &["svelte"],
        lsp_command: Some(LspCommand {
            command: Command("svelteserver", &["--stdio"]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("svelte")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "svelte",
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Svelte),
        }),
        line_comment_prefix: Some("//"),
        block_comment_affixes: Some(("/*", "*/")),
        ..Language::new()
    }
}

const fn json() -> Language {
    Language {
        extensions: &["json"],
        formatter_command: Some(Command("prettierd", &[".json"])),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "json",
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::JSON),
        }),
        ..Language::new()
    }
}

const fn julia() -> Language {
    Language {
        extensions: &["jl"],
        /* lsp_command: Some(LspCommand {
            command: Command(
                "julia",
                &[
                    "--startup-file=no",
                    "--history-file=no",
                    "--quiet",
                    "-e",
                    "'using LanguageServer; runserver()'",
                ],
            ),
            ..LspCommand::default()
        }), */
        lsp_language_id: Some(LanguageId::new("julia")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "julia",
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Julia),
        }),
        line_comment_prefix: Some("#"),
        block_comment_affixes: Some(("#=", "=#")),
        ..Language::new()
    }
}

const fn just() -> Language {
    Language {
        file_names: &["justfile"],
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "just",
            kind: GrammarConfigKind::FromSource {
                url: "https://github.com/IndianBoy42/tree-sitter-just",
                commit: "main",
                subpath: None,
            },
        }),
        line_comment_prefix: Some("#"),
        ..Language::new()
    }
}

const fn ki_quickfix() -> Language {
    Language {
        extensions: &["ki_quickfix"],
        lsp_language_id: Some(LanguageId::new("ki_quickfix")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "ki_quickfix",
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::KiQuickfix),
        }),
        ..Language::new()
    }
}

const fn lua() -> Language {
    Language {
        extensions: &["lua"],
        formatter_command: Some(Command("stylua", &["-"])),
        lsp_command: Some(LspCommand {
            command: Command("lua-language-server", &[]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("lua")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "lua",
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Lua),
        }),
        line_comment_prefix: Some("--"),
        block_comment_affixes: Some(("--[[", "]]")),
        ..Language::new()
    }
}

const fn markdown() -> Language {
    Language {
        extensions: &["md", "mdx"],
        formatter_command: Some(Command("prettierd", &[".md"])),
        lsp_command: Some(LspCommand {
            command: Command("marksman", &["server"]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("markdown")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "markdown",
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Markdown),
        }),
        block_comment_affixes: Some(("<!--", "-->")),
        ..Language::new()
    }
}

const fn nix() -> Language {
    Language {
        formatter_command: Some(Command("nixfmt", &[])),
        extensions: &["nix"],
        lsp_command: Some(LspCommand {
            command: Command("nil", &[]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("nix")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "nix",
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Nix),
        }),
        line_comment_prefix: Some("#"),
        block_comment_affixes: Some(("/*", "*/")),
        ..Language::new()
    }
}

const fn ocaml() -> Language {
    Language {
        extensions: &["ml"],
        formatter_command: Some(Command("ocamlformat", &["-", "--impl"])),
        lsp_command: Some(LspCommand {
            command: Command("ocamllsp", &["--stdio"]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("ocaml")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "ocaml",
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::OCaml),
        }),
        block_comment_affixes: Some(("(*", "*)")),
        ..Language::new()
    }
}

const fn ocaml_interface() -> Language {
    Language {
        extensions: &["mli"],
        formatter_command: Some(Command("ocamlformat", &["-", "--intf"])),
        lsp_command: Some(LspCommand {
            command: Command("ocamllsp", &["--stdio"]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("ocaml")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "ocaml_interface",
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::OCamlInterface),
        }),
        block_comment_affixes: Some(("(*", "*)")),
        ..Language::new()
    }
}

const fn odin() -> Language {
    Language {
        extensions: &["odin"],
        formatter_command: Some(Command("odinfmt", &["-stdin"])),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "odin",
            kind: GrammarConfigKind::FromSource {
                url: "https://github.com/tree-sitter-grammars/tree-sitter-odin",
                commit: "master",
                subpath: None,
            },
        }),
        line_comment_prefix: Some("//"),
        block_comment_affixes: Some(("/*", "*/")),
        ..Language::new()
    }
}

const fn dune() -> Language {
    Language {
        extensions: &["dune-project", "dune"],
        formatter_command: Some(Command("dune", &["format-dune-file"])),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "dune",
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Scheme),
        }),
        line_comment_prefix: Some(";"),
        ..Language::new()
    }
}

const fn python() -> Language {
    Language {
        extensions: &["py"],
        formatter_command: Some(Command("ruff", &["format", "--stdin-filename", ".py"])),
        lsp_command: Some(LspCommand {
            command: Command("pyright-langserver", &["--stdio"]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("python")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "python",
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Python),
        }),
        line_comment_prefix: Some("#"),
        ..Language::new()
    }
}

const fn rescript() -> Language {
    Language {
        extensions: &["res"],
        formatter_command: Some(Command(
            "./node_modules/.bin/rescript",
            &["format", "-stdin", ".res"],
        )),
        lsp_command: Some(LspCommand {
            command: Command("./node_modules/.bin/rescript-language-server", &["--stdio"]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("rescript")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "rescript",
            kind: GrammarConfigKind::FromSource {
                url: "https://github.com/rescript-lang/tree-sitter-rescript",
                commit: "main",
                subpath: None,
            },
        }),
        line_comment_prefix: Some("//"),
        block_comment_affixes: Some(("/*", "*/")),
        ..Language::new()
    }
}

const fn ruby() -> Language {
    Language {
        extensions: &["rb", "rbs", "gemspec", "rake", "podspec"],
        file_names: &["Gemfile", "Rakefile", "Podfile", "Fastfile", "config.ru"],
        formatter_command: Some(Command(
            "rubocop",
            &["--fix-layout", "--stdin", "/dev/null", "--stderr"],
        )),
        lsp_command: Some(LspCommand {
            command: Command("ruby-lsp", &[]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("ruby")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "ruby",
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Ruby),
        }),
        line_comment_prefix: Some("#"),
        ..Language::new()
    }
}

const fn roc() -> Language {
    Language {
        extensions: &["roc"],
        formatter_command: Some(Command("roc", &["format", "--stdin", "--stdout"])),
        lsp_command: None,
        lsp_language_id: Some(LanguageId::new("roc")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "roc",
            kind: GrammarConfigKind::FromSource {
                url: "https://github.com/faldor20/tree-sitter-roc",
                commit: "master",
                subpath: None,
            },
        }),
        line_comment_prefix: Some("#"),
        ..Language::new()
    }
}

const fn rust() -> Language {
    Language {
        extensions: &["rs"],
        formatter_command: Some(Command("rustfmt", &["--edition=2021"])),
        lsp_command: Some(LspCommand {
            command: Command("rust-analyzer", &[]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("rust")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "rust",
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Rust),
        }),
        line_comment_prefix: Some("//"),
        block_comment_affixes: Some(("/*", "*/")),
        ..Language::new()
    }
}

const fn sql() -> Language {
    Language {
        extensions: &["sql", "pgsql", "mssql", "mysql"],
        formatter_command: Some(Command("sql-formatter", &["--language", "postgresql"])),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "sql",
            kind: GrammarConfigKind::FromSource {
                url: "https://github.com/DerekStride/tree-sitter-sql",
                commit: "25f94f998de79bae9df28add9782f9ea6ea0e2b8",
                subpath: None,
            },
        }),
        line_comment_prefix: Some("--"),
        block_comment_affixes: Some(("/*", "*/")),
        ..Language::new()
    }
}

const fn swift() -> Language {
    Language {
        extensions: &["swift"],
        formatter_command: Some(Command("swiftformat", &[])),
        lsp_command: Some(LspCommand {
            command: Command("sourcekit-lsp", &[]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("swift")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "swift",
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Swift),
        }),
        line_comment_prefix: Some("//"),
        block_comment_affixes: Some(("/*", "*/")),
        ..Language::new()
    }
}

const fn typst() -> Language {
    Language {
        extensions: &["typ"],
        formatter_command: Some(Command("typstyle", &["-i"])),
        lsp_command: Some(LspCommand {
            command: Command("tinymist", &["lsp"]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("typst")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "typst",
            kind: GrammarConfigKind::FromSource {
                url: "https://github.com/uben0/tree-sitter-typst",
                commit: "master",
                subpath: None,
            },
        }),
        block_comment_affixes: Some(("/*", "*/")),
        ..Language::new()
    }
}

const fn toml() -> Language {
    Language {
        extensions: &["toml"],
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "toml",
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Toml),
        }),
        line_comment_prefix: Some("#"),
        ..Language::new()
    }
}

const fn tree_sitter_query() -> Language {
    Language {
        extensions: &["scm"],
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "tsq",
            kind: GrammarConfigKind::FromSource {
                url: "https://github.com/tree-sitter/tree-sitter-tsq",
                commit: "main",
                subpath: None,
            },
        }),
        line_comment_prefix: Some(";"),
        ..Language::new()
    }
}

const fn typescript() -> Language {
    Language {
        extensions: &["ts", "mts", "cts"],
        formatter_command: Some(Command("prettierd", &[".ts"])),
        lsp_command: Some(LspCommand {
            command: Command("typescript-language-server", &["--stdio"]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("typescript")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "typescript",
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Typescript),
        }),
        line_comment_prefix: Some("//"),
        block_comment_affixes: Some(("/*", "*/")),
        ..Language::new()
    }
}

const fn typescript_react() -> Language {
    Language {
        extensions: &["tsx"],
        formatter_command: Some(Command("prettierd", &[".tsx"])),
        lsp_command: Some(LspCommand {
            command: Command("typescript-language-server", &["--stdio"]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("typescriptreact")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "tsx",
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::TSX),
        }),
        line_comment_prefix: Some("//"),
        block_comment_affixes: Some(("/*", "*/")),
        ..Language::new()
    }
}

const fn unison() -> Language {
    Language {
        extensions: &["u"],
        lsp_command: Some(LspCommand {
            command: Command("nc", &["localhost", "5757"]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("unison")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "unison",
            kind: GrammarConfigKind::FromSource {
                url: "https://github.com/kylegoetz/tree-sitter-unison",
                commit: "master",
                subpath: None,
            },
        }),
        ..Language::new()
    }
}

const fn xml() -> Language {
    Language {
        extensions: &["xml", "xaml", "axaml"],
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "xml",
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::XML),
        }),
        block_comment_affixes: Some(("<!--", "-->")),
        ..Language::new()
    }
}

const fn yaml() -> Language {
    Language {
        extensions: &["yaml", "yml"],
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "yaml",
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::YAML),
        }),
        line_comment_prefix: Some("#"),
        ..Language::new()
    }
}

const fn zig() -> Language {
    Language {
        extensions: &["zig"],
        formatter_command: Some(Command("zig", &["fmt", "--stdin"])),
        lsp_command: Some(LspCommand {
            command: Command("zls", &[]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("zig")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "zig",
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Zig),
        }),
        line_comment_prefix: Some("//"),
        ..Language::new()
    }
}
