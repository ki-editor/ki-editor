use crate::language::CargoLinkedTreesitterLanguage;

use super::language::{Command, GrammarConfig, Language, LanguageId, LspCommand};

pub const LANGUAGES: &[&Language] = &[
    &bash(),
    &fish(),
    &unison(),
    &c(),
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
    &javascript(),
    &javascript_react(),
    &svelte(),
    &json(),
    &just(),
    &lua(),
    &markdown(),
    &nix(),
    &ocaml(),
    &ocaml_interface(),
    &dune(),
    &python(),
    &rescript(),
    &roc(),
    &ruby(),
    &rust(),
    &sql(),
    &swift(),
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
            url: "https://github.com/tree-sitter/tree-sitter-bash",
            commit: "master",
            subpath: None,
        }),
        language_fallback: Some(CargoLinkedTreesitterLanguage::Bash),
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
            url: "https://github.com/ram02z/tree-sitter-fish",
            commit: "master",
            subpath: None,
        }),
        language_fallback: Some(CargoLinkedTreesitterLanguage::Fish),
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
            url: "https://github.com/tree-sitter/tree-sitter-c",
            commit: "master",
            subpath: None,
        }),
        language_fallback: Some(CargoLinkedTreesitterLanguage::C),
        line_comment_prefix: Some("//"),
        block_comment_affixes: Some(("/*", "*/")),
        ..Language::new()
    }
}

const fn common_lisp() -> Language {
    Language {
        extensions: &["lisp", "lsp", "l", "cl", "fasl", "sbcl", "el"],
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "commonlisp",
            url: "https://github.com/theHamsta/tree-sitter-commonlisp",
            commit: "master",
            subpath: None,
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
            url: "https://github.com/tree-sitter/tree-sitter-cpp",
            commit: "master",
            subpath: None,
        }),
        language_fallback: Some(CargoLinkedTreesitterLanguage::CPP),
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
            url: "https://github.com/arnau/tree-sitter-csv",
            commit: "main",
            subpath: None,
        }),
        language_fallback: None,
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
            url: "https://github.com/tree-sitter/tree-sitter-c-sharp",
            subpath: None,
            commit: "master",
        }),
        language_fallback: None,
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
            url: "https://github.com/tree-sitter/tree-sitter-css",
            commit: "master",
            subpath: None,
        }),
        language_fallback: Some(CargoLinkedTreesitterLanguage::CSS),
        block_comment_affixes: Some(("/*", "*/")),
        ..Language::new()
    }
}

const fn diff() -> Language {
    Language {
        extensions: &["diff"],
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "diff",
            url: "https://github.com/the-mikedavis/tree-sitter-diff",
            commit: "main",
            subpath: None,
        }),
        language_fallback: Some(CargoLinkedTreesitterLanguage::Diff),
        ..Language::new()
    }
}

const fn dockerfile() -> Language {
    Language {
        file_names: &["Dockerfile"],
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "dockerfile",
            url: "https://github.com/camdencheek/tree-sitter-dockerfile",
            commit: "main",
            subpath: None,
        }),
        language_fallback: None,
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
            url: "https://github.com/elixir-lang/tree-sitter-elixir",
            commit: "main",
            subpath: None,
        }),
        language_fallback: Some(CargoLinkedTreesitterLanguage::Elixir),
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
            url: "https://github.com/ionide/tree-sitter-fsharp.git",
            commit: "main",
            subpath: Some("fsharp"),
        }),
        language_fallback: None,
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
            url: "https://github.com/tree-sitter-grammars/tree-sitter-gitattributes",
            commit: "master",
            subpath: None,
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
            url: "https://github.com/gbprod/tree-sitter-gitcommit",
            commit: "main",
            subpath: None,
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
            url: "https://github.com/the-mikedavis/tree-sitter-git-config",
            commit: "main",
            subpath: None,
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
            url: "https://github.com/shunsambongi/tree-sitter-gitignore",
            commit: "main",
            subpath: None,
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
            url: "https://github.com/the-mikedavis/tree-sitter-git-rebase",
            commit: "main",
            subpath: None,
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
            url: "https://github.com/gleam-lang/tree-sitter-gleam",
            commit: "main",
            subpath: None,
        }),
        language_fallback: Some(CargoLinkedTreesitterLanguage::Gleam),
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
            url: "https://github.com/tree-sitter/tree-sitter-go",
            subpath: None,
            commit: "master",
        }),
        language_fallback: Some(CargoLinkedTreesitterLanguage::Go),
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
            url: "https://github.com/bkegley/tree-sitter-graphql",
            commit: "master",
            subpath: None,
        }),
        language_fallback: Some(CargoLinkedTreesitterLanguage::Graphql),
        line_comment_prefix: Some("#"),
        ..Language::new()
    }
}

const fn hare() -> Language {
    Language {
        extensions: &["ha"],
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "hare",
            url: "https://github.com/tree-sitter-grammars/tree-sitter-hare",
            commit: "master",
            subpath: None,
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
            url: "https://github.com/phoenixframework/tree-sitter-heex",
            commit: "main",
            subpath: None,
        }),
        language_fallback: Some(CargoLinkedTreesitterLanguage::Heex),
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
            url: "https://github.com/tree-sitter/tree-sitter-html",
            commit: "master",
            subpath: None,
        }),
        language_fallback: Some(CargoLinkedTreesitterLanguage::HTML),
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
            url: "https://github.com/kayhide/tree-sitter-idris",
            commit: "main",
            subpath: None,
        }),
        language_fallback: None,
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
            url: "https://github.com/tree-sitter/tree-sitter-javascript",
            commit: "master",
            subpath: None,
        }),
        language_fallback: Some(CargoLinkedTreesitterLanguage::Javascript),
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
            url: "https://github.com/tree-sitter/tree-sitter-javascript",
            commit: "master",
            subpath: None,
        }),
        language_fallback: Some(CargoLinkedTreesitterLanguage::JSX),
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
            url: "https://github.com/tree-sitter-grammars/tree-sitter-svelte",
            commit: "master",
            subpath: None,
        }),
        language_fallback: Some(CargoLinkedTreesitterLanguage::Svelte),
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
            url: "https://github.com/tree-sitter/tree-sitter-json",
            commit: "master",
            subpath: None,
        }),
        language_fallback: Some(CargoLinkedTreesitterLanguage::JSON),
        ..Language::new()
    }
}

const fn just() -> Language {
    Language {
        file_names: &["justfile"],
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "just",
            url: "https://github.com/IndianBoy42/tree-sitter-just",
            commit: "main",
            subpath: None,
        }),
        line_comment_prefix: Some("#"),
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
            url: "https://github.com/tree-sitter-grammars/tree-sitter-lua",
            commit: "main",
            subpath: None,
        }),
        language_fallback: Some(CargoLinkedTreesitterLanguage::Lua),
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
            url: "https://github.com/MDeiml/tree-sitter-markdown",
            commit: "split_parser",
            subpath: Some("tree-sitter-markdown"),
        }),
        language_fallback: Some(CargoLinkedTreesitterLanguage::Markdown),
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
            url: "https://github.com/nix-community/tree-sitter-nix",
            commit: "master",
            subpath: None,
        }),
        language_fallback: Some(CargoLinkedTreesitterLanguage::Nix),
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
            url: "https://github.com/tree-sitter/tree-sitter-ocaml",
            commit: "master",
            subpath: Some("grammars/ocaml"),
        }),
        language_fallback: Some(CargoLinkedTreesitterLanguage::OCaml),
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
            url: "https://github.com/tree-sitter/tree-sitter-ocaml",
            commit: "master",
            subpath: Some("grammars/interface"),
        }),
        language_fallback: Some(CargoLinkedTreesitterLanguage::OCamlInterface),
        block_comment_affixes: Some(("(*", "*)")),
        ..Language::new()
    }
}

const fn dune() -> Language {
    Language {
        extensions: &["dune-project", "dune"],
        formatter_command: Some(Command("dune", &["format-dune-file"])),
        lsp_language_id: Some(LanguageId::new("dune")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "dune",
            url: "https://github.com/6cdh/tree-sitter-scheme",
            commit: "main",
            subpath: None,
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
            url: "https://github.com/tree-sitter/tree-sitter-python",
            commit: "master",
            subpath: None,
        }),
        language_fallback: Some(CargoLinkedTreesitterLanguage::Python),
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
            url: "https://github.com/rescript-lang/tree-sitter-rescript",
            commit: "main",
            subpath: None,
        }),
        language_fallback: None,
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
            url: "https://github.com/tree-sitter/tree-sitter-ruby",
            commit: "master",
            subpath: None,
        }),
        language_fallback: Some(CargoLinkedTreesitterLanguage::Ruby),
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
            url: "https://github.com/faldor20/tree-sitter-roc",
            commit: "master",
            subpath: None,
        }),
        language_fallback: None,
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
            url: "https://github.com/tree-sitter/tree-sitter-rust",
            commit: "e86119bdb4968b9799f6a014ca2401c178d54b5f",
            subpath: None,
        }),
        language_fallback: Some(CargoLinkedTreesitterLanguage::Rust),
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
            url: "https://github.com/DerekStride/tree-sitter-sql",
            commit: "25f94f998de79bae9df28add9782f9ea6ea0e2b8",
            subpath: None,
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
            url: "https://github.com/alex-pinkus/tree-sitter-swift",
            commit: "with-generated-files",
            subpath: None,
        }),
        language_fallback: Some(CargoLinkedTreesitterLanguage::Swift),
        line_comment_prefix: Some("//"),
        block_comment_affixes: Some(("/*", "*/")),
        ..Language::new()
    }
}

const fn toml() -> Language {
    Language {
        extensions: &["toml"],
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "toml",
            url: "https://github.com/ikatyang/tree-sitter-toml",
            commit: "master",
            subpath: None,
        }),
        language_fallback: Some(CargoLinkedTreesitterLanguage::Toml),
        line_comment_prefix: Some("#"),
        ..Language::new()
    }
}

const fn tree_sitter_query() -> Language {
    Language {
        extensions: &["scm"],
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "tsq",
            url: "https://github.com/tree-sitter/tree-sitter-tsq",
            commit: "main",
            subpath: None,
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
            url: "https://github.com/tree-sitter/tree-sitter-typescript",
            commit: "master",
            subpath: Some("typescript"),
        }),
        language_fallback: Some(CargoLinkedTreesitterLanguage::Typescript),
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
            url: "https://github.com/tree-sitter/tree-sitter-typescript",
            commit: "master",
            subpath: Some("tsx"),
        }),
        language_fallback: Some(CargoLinkedTreesitterLanguage::TSX),
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
            url: "https://github.com/kylegoetz/tree-sitter-unison",
            commit: "master",
            subpath: None,
        }),
        language_fallback: None,
        ..Language::new()
    }
}

const fn xml() -> Language {
    Language {
        extensions: &["xml", "xaml", "axaml"],
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "xml",
            url: "https://github.com/tree-sitter-grammars/tree-sitter-xml",
            subpath: Some("xml"),
            commit: "master",
        }),
        language_fallback: Some(CargoLinkedTreesitterLanguage::XML),
        block_comment_affixes: Some(("<!--", "-->")),
        ..Language::new()
    }
}

const fn yaml() -> Language {
    Language {
        extensions: &["yaml", "yml"],
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "yaml",
            url: "https://github.com/ikatyang/tree-sitter-yaml",
            subpath: None,
            commit: "master",
        }),
        language_fallback: Some(CargoLinkedTreesitterLanguage::YAML),
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
            url: "https://github.com/tree-sitter-grammars/tree-sitter-zig",
            subpath: None,
            commit: "master",
        }),
        language_fallback: Some(CargoLinkedTreesitterLanguage::Zig),
        line_comment_prefix: Some("//"),
        ..Language::new()
    }
}
