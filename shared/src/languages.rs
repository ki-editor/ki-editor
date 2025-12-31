use std::collections::HashMap;

use serde_json::json;

use crate::language::{CargoLinkedTreesitterLanguage, GrammarConfigKind};

use super::language::{Command, GrammarConfig, Language, LanguageId, LspCommand};

fn to_vec(slice: &[&'static str]) -> Vec<String> {
    slice.iter().map(|s| s.to_string()).collect()
}

pub fn languages() -> HashMap<String, Language> {
    [
        ("bash", bash()),
        ("fish", fish()),
        ("unison", unison()),
        ("c", c()),
        ("racket", racket()),
        ("scheme", scheme()),
        ("commonlisp", commonlisp()),
        ("cpp", cpp()),
        ("c_sharp", c_sharp()),
        ("css", css()),
        ("csv", csv()),
        ("diff", diff()),
        ("dockerfile", dockerfile()),
        ("elixir", elixir()),
        ("fsharp", fsharp()),
        ("gitattributes", gitattributes()),
        ("gitcommit", gitcommit()),
        ("gitconfig", gitconfig()),
        ("gitignore", gitignore()),
        ("gitrebase", gitrebase()),
        ("gleam", gleam()),
        ("go", go()),
        ("graphql", graphql()),
        ("hare", hare()),
        ("heex", heex()),
        ("html", html()),
        ("idris", idris()),
        ("haskell", haskell()),
        ("javascript", javascript()),
        ("javascriptreact", javascriptreact()),
        ("svelte", svelte()),
        ("json", json()),
        ("julia", julia()),
        ("just", just()),
        ("kiquickfix", kiquickfix()),
        ("lua", lua()),
        ("markdown", markdown()),
        ("nix", nix()),
        ("ocaml", ocaml()),
        ("ocaml_interface", ocaml_interface()),
        ("odin", odin()),
        ("dune", dune()),
        ("python", python()),
        ("rescript", rescript()),
        ("roc", roc()),
        ("ruby", ruby()),
        ("rust", rust()),
        ("sql", sql()),
        ("swift", swift()),
        ("typst", typst()),
        ("toml", toml()),
        ("tree_sitter_query", tree_sitter_query()),
        ("typescript", typescript()),
        ("typescriptreact", typescriptreact()),
        ("xml", xml()),
        ("yaml", yaml()),
        ("zig", zig()),
    ]
    .into_iter()
    .map(|(str, language)| (str.to_string(), language))
    .collect()
}

fn bash() -> Language {
    Language {
        extensions: to_vec(&["sh", "bash"]),
        file_names: to_vec(&[".bashrc", ".bash_profile", "bashrc", "bash_profile"]),
        formatter: Some(Command::new("shfmt", &[".sh", ".bash"])),
        lsp_command: Some(LspCommand {
            command: Command::new("bash-language-server", &["start"]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("bash")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "bash".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Bash),
        }),
        line_comment_prefix: Some("#".to_string()),
        ..Language::new()
    }
}

fn fish() -> Language {
    Language {
        extensions: to_vec(&["fish"]),
        formatter: Some(Command::new("fish --no-execute ", &[".fish"])),
        lsp_command: Some(LspCommand {
            command: Command::new("fish-lsp", &["start"]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("fish")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "fish".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Fish),
        }),
        line_comment_prefix: Some("#".to_string()),
        ..Language::new()
    }
}

fn c() -> Language {
    Language {
        extensions: to_vec(&["c", "h"]),
        formatter: Some(Command::new("clang-format", &[])),
        lsp_command: Some(LspCommand {
            command: Command::new("clangd", &[]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("c")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "c".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::C),
        }),
        line_comment_prefix: Some("//".to_string()),
        block_comment_affixes: Some(("/*".to_string(), "*/".to_string())),
        ..Language::new()
    }
}

fn racket() -> Language {
    Language {
        extensions: to_vec(&["rkt", "rktd", "rktl", "scrbl", "zuo"]),
        lsp_command: Some(LspCommand {
            command: Command::new("racket", &[]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("racket")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "racket".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Scheme),
        }),
        line_comment_prefix: Some(";".to_string()),
        block_comment_affixes: Some(("#|".to_string(), "|#".to_string())),
        ..Language::new()
    }
}

fn scheme() -> Language {
    Language {
        extensions: to_vec(&["ss", "scm", "sld"]),
        // lsp_language_id: Some(LanguageId::new("scheme")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "scheme".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Scheme),
        }),
        line_comment_prefix: Some(";".to_string()),
        block_comment_affixes: Some(("#|".to_string(), "|#".to_string())),
        ..Language::new()
    }
}

fn commonlisp() -> Language {
    Language {
        extensions: to_vec(&[
            "lisp", "lsp", "l", "cl", "fasl", "sbcl", "el", "asd", "ny", "podsl", "sexp",
        ]),
        lsp_command: Some(LspCommand {
            command: Command::new("cl-lsp", &[]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("commonlisp")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "commonlisp".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Scheme),
        }),
        line_comment_prefix: Some(";".to_string()),
        ..Language::new()
    }
}

fn cpp() -> Language {
    Language {
        extensions: to_vec(&[
            "cc", "hh", "c++", "cpp", "hpp", "h", "ipp", "tpp", "cxx", "hxx", "ixx", "txx", "ino",
            "cu", "cuh", "cppm", "h++", "ii", "inl",
        ]),
        formatter: Some(Command::new("clang-format", &[])),
        lsp_command: Some(LspCommand {
            command: Command::new("clangd", &[]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("cpp")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "cpp".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::CPP),
        }),
        line_comment_prefix: Some("//".to_string()),
        block_comment_affixes: Some(("/*".to_string(), "*/".to_string())),
        ..Language::new()
    }
}

fn csv() -> Language {
    Language {
        extensions: to_vec(&["csv"]),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "csv".to_string(),
            kind: GrammarConfigKind::FromSource {
                url: "https://github.com/arnau/tree-sitter-csv".to_string(),
                commit: "main".to_string(),
                subpath: None,
            },
        }),
        ..Language::new()
    }
}

fn c_sharp() -> Language {
    Language {
        extensions: to_vec(&["cs", "csx", "cake"]),
        formatter: Some(Command::new("csharpier", &["format", "--write-stdout"])),
        lsp_command: Some(LspCommand {
            command: Command::new("omnisharp", &["--languageserver"]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("c_sharp")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "c_sharp".to_string(),
            kind: GrammarConfigKind::FromSource {
                url: "https://github.com/tree-sitter/tree-sitter-c-sharp".to_string(),
                subpath: None,
                commit: "master".to_string(),
            },
        }),
        line_comment_prefix: Some("//".to_string()),
        block_comment_affixes: Some(("/*".to_string(), "*/".to_string())),
        ..Language::new()
    }
}

fn css() -> Language {
    Language {
        extensions: to_vec(&["css"]),
        formatter: Some(Command::new("prettierd", &[".css"])),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "css".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::CSS),
        }),
        block_comment_affixes: Some(("/*".to_string(), "*/".to_string())),
        ..Language::new()
    }
}

fn diff() -> Language {
    Language {
        extensions: to_vec(&["diff"]),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "diff".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Diff),
        }),
        ..Language::new()
    }
}

fn dockerfile() -> Language {
    Language {
        file_names: to_vec(&["Dockerfile"]),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "dockerfile".to_string(),
            kind: GrammarConfigKind::FromSource {
                url: "https://github.com/camdencheek/tree-sitter-dockerfile".to_string(),
                commit: "main".to_string(),
                subpath: None,
            },
        }),
        line_comment_prefix: Some("#".to_string()),
        ..Language::new()
    }
}

fn elixir() -> Language {
    Language {
        extensions: to_vec(&["ex", "exs"]),
        formatter: Some(Command::new("mix", &["format", "-"])),
        lsp_command: Some(LspCommand {
            command: Command::new("elixir-ls", &[]),
            initialization_options: None,
        }),
        lsp_language_id: Some(LanguageId::new("elixir")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "elixir".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Elixir),
        }),
        line_comment_prefix: Some("#".to_string()),
        ..Language::new()
    }
}

fn fsharp() -> Language {
    Language {
        extensions: to_vec(&["fs", "fsi", "fsx", "fsscript"]),
        formatter: None,
        lsp_command: Some(LspCommand {
            // Use --log-file and --log-level arguments to debug fsautocomplete issues.
            // Example: --log-file /path/to/fsac.log --log-level debug
            command: Command::new("fsautocomplete", &[]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("fsharp")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "fsharp".to_string(),
            kind: GrammarConfigKind::FromSource {
                url: "https://github.com/ionide/tree-sitter-fsharp.git".to_string(),
                commit: "main".to_string(),
                subpath: Some("fsharp".to_string()),
            },
        }),
        line_comment_prefix: Some("//".to_string()),
        block_comment_affixes: Some(("(*".to_string(), "*)".to_string())),
        ..Language::new()
    }
}

fn gitattributes() -> Language {
    Language {
        file_names: to_vec(&[".gitattributes"]),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "gitattributes".to_string(),
            kind: GrammarConfigKind::FromSource {
                url: "https://github.com/tree-sitter-grammars/tree-sitter-gitattributes"
                    .to_string(),
                commit: "master".to_string(),
                subpath: None,
            },
        }),
        line_comment_prefix: Some("#".to_string()),
        ..Language::new()
    }
}

fn gitcommit() -> Language {
    Language {
        file_names: to_vec(&["COMMIT_EDITMSG"]),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "gitcommit".to_string(),
            kind: GrammarConfigKind::FromSource {
                url: "https://github.com/gbprod/tree-sitter-gitcommit".to_string(),
                commit: "main".to_string(),
                subpath: None,
            },
        }),
        line_comment_prefix: Some("#".to_string()),
        ..Language::new()
    }
}

fn gitconfig() -> Language {
    Language {
        file_names: to_vec(&[".gitconfig"]),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "git_config".to_string(),
            kind: GrammarConfigKind::FromSource {
                url: "https://github.com/the-mikedavis/tree-sitter-git-config".to_string(),
                commit: "main".to_string(),
                subpath: None,
            },
        }),
        line_comment_prefix: Some("#".to_string()),
        ..Language::new()
    }
}

fn gitignore() -> Language {
    Language {
        file_names: to_vec(&[".gitignore"]),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "gitignore".to_string(),
            kind: GrammarConfigKind::FromSource {
                url: "https://github.com/shunsambongi/tree-sitter-gitignore".to_string(),
                commit: "main".to_string(),
                subpath: None,
            },
        }),
        line_comment_prefix: Some("#".to_string()),
        ..Language::new()
    }
}

fn gitrebase() -> Language {
    Language {
        file_names: to_vec(&["git-rebase-todo"]),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "git_rebase".to_string(),
            kind: GrammarConfigKind::FromSource {
                url: "https://github.com/the-mikedavis/tree-sitter-git-rebase".to_string(),
                commit: "main".to_string(),
                subpath: None,
            },
        }),
        line_comment_prefix: Some("#".to_string()),
        ..Language::new()
    }
}

fn gleam() -> Language {
    Language {
        extensions: to_vec(&["gleam"]),
        formatter: Some(Command::new("gleam", &["format", "--stdin"])),
        lsp_command: Some(LspCommand {
            command: Command::new("gleam", &["lsp"]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("gleam")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "gleam".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Gleam),
        }),
        line_comment_prefix: Some("//".to_string()),
        ..Language::new()
    }
}

fn go() -> Language {
    Language {
        extensions: to_vec(&["go"]),
        formatter: Some(Command::new("gofmt", &[])),
        lsp_command: Some(LspCommand {
            command: Command::new("gopls", &[]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("go")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "go".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Go),
        }),
        line_comment_prefix: Some("//".to_string()),
        block_comment_affixes: Some(("/*".to_string(), "*/".to_string())),
        ..Language::new()
    }
}

fn graphql() -> Language {
    Language {
        extensions: to_vec(&["graphql", "gql"]),
        formatter: Some(Command::new("prettierd", &[".graphql"])),
        lsp_command: Some(LspCommand {
            command: Command::new("graphql-lsp", &["server", "-m", "stream"]),
            initialization_options: Some(
                json! {r#"{ "graphql-config.load.legacy": true }"#.to_string()},
            ),
        }),
        lsp_language_id: Some(LanguageId::new("graphql")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "graphql".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Graphql),
        }),
        line_comment_prefix: Some("#".to_string()),
        ..Language::new()
    }
}

fn hare() -> Language {
    Language {
        extensions: to_vec(&["ha"]),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "hare".to_string(),
            kind: GrammarConfigKind::FromSource {
                url: "https://github.com/tree-sitter-grammars/tree-sitter-hare".to_string(),
                commit: "master".to_string(),
                subpath: None,
            },
        }),
        ..Language::new()
    }
}

fn heex() -> Language {
    Language {
        extensions: to_vec(&["heex"]),
        formatter: Some(Command::new(
            "mix",
            &["format", "--stdin-filename", "file.heex", "-"],
        )),
        lsp_command: Some(LspCommand {
            command: Command::new("elixir-ls", &[]),
            initialization_options: None,
        }),
        lsp_language_id: Some(LanguageId::new("heex")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "heex".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Heex),
        }),
        block_comment_affixes: Some(("<!--".to_string(), "-->".to_string())),
        ..Language::new()
    }
}

fn html() -> Language {
    Language {
        extensions: to_vec(&["htm", "html", "svg"]),
        formatter: Some(Command::new("prettierd", &[".html"])),
        lsp_command: Some(LspCommand {
            command: Command::new("emmet-language-server", &["--stdio"]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("html")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "html".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::HTML),
        }),
        block_comment_affixes: Some(("<!--".to_string(), "-->".to_string())),
        ..Language::new()
    }
}

fn idris() -> Language {
    Language {
        extensions: to_vec(&["idr", "lidr", "ipkg"]),
        lsp_command: Some(LspCommand {
            command: Command::new("idris2-lsp", &[]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("idris")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "idris".to_string(),
            kind: GrammarConfigKind::FromSource {
                url: "https://github.com/kayhide/tree-sitter-idris".to_string(),
                commit: "main".to_string(),
                subpath: None,
            },
        }),
        ..Language::new()
    }
}

fn haskell() -> Language {
    Language {
        extensions: to_vec(&["hs"]),
        lsp_command: Some(LspCommand {
            command: Command::new("haskell-language-server-wrapper", &["--lsp"]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("haskell")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "haskell".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Haskell),
        }),
        line_comment_prefix: Some("--".to_string()),
        block_comment_affixes: Some(("{-".to_string(), "-}".to_string())),
        ..Language::new()
    }
}

fn javascript() -> Language {
    Language {
        extensions: to_vec(&["js", "mjs", "cjs"]),
        formatter: Some(Command::new("prettierd", &[".js"])),
        lsp_command: Some(LspCommand {
            command: Command::new("typescript-language-server", &["--stdio"]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("javascript")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "javascript".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Javascript),
        }),
        line_comment_prefix: Some("//".to_string()),
        block_comment_affixes: Some(("/*".to_string(), "*/".to_string())),
        ..Language::new()
    }
}

fn javascriptreact() -> Language {
    Language {
        extensions: to_vec(&["jsx"]),
        formatter: Some(Command::new("prettierd", &[".jsx"])),
        lsp_command: Some(LspCommand {
            command: Command::new("typescript-language-server", &["--stdio"]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("javascriptreact")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "jsx".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::JSX),
        }),
        line_comment_prefix: Some("//".to_string()),
        block_comment_affixes: Some(("/*".to_string(), "*/".to_string())),
        ..Language::new()
    }
}

fn svelte() -> Language {
    Language {
        extensions: to_vec(&["svelte"]),
        lsp_command: Some(LspCommand {
            command: Command::new("svelteserver", &["--stdio"]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("svelte")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "svelte".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Svelte),
        }),
        line_comment_prefix: Some("//".to_string()),
        block_comment_affixes: Some(("/*".to_string(), "*/".to_string())),
        ..Language::new()
    }
}

fn json() -> Language {
    Language {
        extensions: to_vec(&["json"]),
        formatter: Some(Command::new("prettierd", &[".json"])),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "json".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::JSON),
        }),
        ..Language::new()
    }
}

fn julia() -> Language {
    Language {
        extensions: to_vec(&["jl"]),
        /* lsp_command: Some(LspCommand {
            command: Command::new(
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
            id: "julia".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Julia),
        }),
        line_comment_prefix: Some("#".to_string()),
        block_comment_affixes: Some(("#=".to_string(), "=#".to_string())),
        ..Language::new()
    }
}

fn just() -> Language {
    Language {
        file_names: to_vec(&["justfile"]),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "just".to_string(),
            kind: GrammarConfigKind::FromSource {
                url: "https://github.com/IndianBoy42/tree-sitter-just".to_string(),
                commit: "main".to_string(),
                subpath: None,
            },
        }),
        line_comment_prefix: Some("#".to_string()),
        ..Language::new()
    }
}

fn kiquickfix() -> Language {
    Language {
        extensions: to_vec(&["ki_quickfix"]),
        lsp_language_id: Some(LanguageId::new("ki_quickfix")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "ki_quickfix".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::KiQuickfix),
        }),
        ..Language::new()
    }
}

fn lua() -> Language {
    Language {
        extensions: to_vec(&["lua"]),
        formatter: Some(Command::new("stylua", &["-"])),
        lsp_command: Some(LspCommand {
            command: Command::new("lua-language-server", &[]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("lua")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "lua".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Lua),
        }),
        line_comment_prefix: Some("--".to_string()),
        block_comment_affixes: Some(("--[[".to_string(), "]]".to_string())),
        ..Language::new()
    }
}

fn markdown() -> Language {
    Language {
        extensions: to_vec(&["md", "mdx"]),
        formatter: Some(Command::new("prettierd", &[".md"])),
        lsp_command: Some(LspCommand {
            command: Command::new("marksman", &["server"]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("markdown")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "markdown".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Markdown),
        }),
        block_comment_affixes: Some(("<!--".to_string(), "-->".to_string())),
        ..Language::new()
    }
}

fn nix() -> Language {
    Language {
        formatter: Some(Command::new("nixfmt", &[])),
        extensions: to_vec(&["nix"]),
        lsp_command: Some(LspCommand {
            command: Command::new("nil", &[]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("nix")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "nix".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Nix),
        }),
        line_comment_prefix: Some("#".to_string()),
        block_comment_affixes: Some(("/*".to_string(), "*/".to_string())),
        ..Language::new()
    }
}

fn ocaml() -> Language {
    Language {
        extensions: to_vec(&["ml"]),
        formatter: Some(Command::new("ocamlformat", &["-", "--impl"])),
        lsp_command: Some(LspCommand {
            command: Command::new("ocamllsp", &["--stdio"]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("ocaml")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "ocaml".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::OCaml),
        }),
        block_comment_affixes: Some(("(*".to_string(), "*)".to_string())),
        ..Language::new()
    }
}

fn ocaml_interface() -> Language {
    Language {
        extensions: to_vec(&["mli"]),
        formatter: Some(Command::new("ocamlformat", &["-", "--intf"])),
        lsp_command: Some(LspCommand {
            command: Command::new("ocamllsp", &["--stdio"]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("ocaml")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "ocaml_interface".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::OCamlInterface),
        }),
        block_comment_affixes: Some(("(*".to_string(), "*)".to_string())),
        ..Language::new()
    }
}

fn odin() -> Language {
    Language {
        extensions: to_vec(&["odin"]),
        formatter: Some(Command::new("odinfmt", &["-stdin"])),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "odin".to_string(),
            kind: GrammarConfigKind::FromSource {
                url: "https://github.com/tree-sitter-grammars/tree-sitter-odin".to_string(),
                commit: "master".to_string(),
                subpath: None,
            },
        }),
        line_comment_prefix: Some("//".to_string()),
        block_comment_affixes: Some(("/*".to_string(), "*/".to_string())),
        ..Language::new()
    }
}

fn dune() -> Language {
    Language {
        extensions: to_vec(&["dune-project", "dune"]),
        formatter: Some(Command::new("dune", &["format-dune-file"])),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "dune".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Scheme),
        }),
        line_comment_prefix: Some(";".to_string()),
        ..Language::new()
    }
}

fn python() -> Language {
    Language {
        extensions: to_vec(&["py"]),
        formatter: Some(Command::new("ruff", &["format", "--stdin-filename", ".py"])),
        lsp_command: Some(LspCommand {
            command: Command::new("pyright-langserver", &["--stdio"]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("python")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "python".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Python),
        }),
        line_comment_prefix: Some("#".to_string()),
        ..Language::new()
    }
}

fn rescript() -> Language {
    Language {
        extensions: to_vec(&["res"]),
        formatter: Some(Command::new(
            "./node_modules/.bin/rescript",
            &["format", "-stdin", ".res"],
        )),
        lsp_command: Some(LspCommand {
            command: Command::new("./node_modules/.bin/rescript-language-server", &["--stdio"]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("rescript")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "rescript".to_string(),
            kind: GrammarConfigKind::FromSource {
                url: "https://github.com/rescript-lang/tree-sitter-rescript".to_string(),
                commit: "main".to_string(),
                subpath: None,
            },
        }),
        line_comment_prefix: Some("//".to_string()),
        block_comment_affixes: Some(("/*".to_string(), "*/".to_string())),
        ..Language::new()
    }
}

fn ruby() -> Language {
    Language {
        extensions: to_vec(&["rb", "rbs", "gemspec", "rake", "podspec"]),
        file_names: to_vec(&["Gemfile", "Rakefile", "Podfile", "Fastfile", "config.ru"]),
        formatter: Some(Command::new(
            "rubocop",
            &["--fix-layout", "--stdin", "/dev/null", "--stderr"],
        )),
        lsp_command: Some(LspCommand {
            command: Command::new("ruby-lsp", &[]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("ruby")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "ruby".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Ruby),
        }),
        line_comment_prefix: Some("#".to_string()),
        ..Language::new()
    }
}

fn roc() -> Language {
    Language {
        extensions: to_vec(&["roc"]),
        formatter: Some(Command::new("roc", &["format", "--stdin", "--stdout"])),
        lsp_command: None,
        lsp_language_id: Some(LanguageId::new("roc")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "roc".to_string(),
            kind: GrammarConfigKind::FromSource {
                url: "https://github.com/faldor20/tree-sitter-roc".to_string(),
                commit: "master".to_string(),
                subpath: None,
            },
        }),
        line_comment_prefix: Some("#".to_string()),
        ..Language::new()
    }
}

fn rust() -> Language {
    Language {
        extensions: to_vec(&["rs"]),
        formatter: Some(Command::new("rustfmt", &["--edition=2021"])),
        lsp_command: Some(LspCommand {
            command: Command::new("rust-analyzer", &[]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("rust")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "rust".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Rust),
        }),
        line_comment_prefix: Some("//".to_string()),
        block_comment_affixes: Some(("/*".to_string(), "*/".to_string())),
        ..Language::new()
    }
}

fn sql() -> Language {
    Language {
        extensions: to_vec(&["sql", "pgsql", "mssql", "mysql"]),
        formatter: Some(Command::new("sql-formatter", &["--language", "postgresql"])),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "sql".to_string(),
            kind: GrammarConfigKind::FromSource {
                url: "https://github.com/DerekStride/tree-sitter-sql".to_string(),
                commit: "25f94f998de79bae9df28add9782f9ea6ea0e2b8".to_string(),
                subpath: None,
            },
        }),
        line_comment_prefix: Some("--".to_string()),
        block_comment_affixes: Some(("/*".to_string(), "*/".to_string())),
        ..Language::new()
    }
}

fn swift() -> Language {
    Language {
        extensions: to_vec(&["swift"]),
        formatter: Some(Command::new("swiftformat", &[])),
        lsp_command: Some(LspCommand {
            command: Command::new("sourcekit-lsp", &[]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("swift")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "swift".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Swift),
        }),
        line_comment_prefix: Some("//".to_string()),
        block_comment_affixes: Some(("/*".to_string(), "*/".to_string())),
        ..Language::new()
    }
}

fn typst() -> Language {
    Language {
        extensions: to_vec(&["typ"]),
        formatter: Some(Command::new("typstyle", &["-i"])),
        lsp_command: Some(LspCommand {
            command: Command::new("tinymist", &["lsp"]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("typst")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "typst".to_string(),
            kind: GrammarConfigKind::FromSource {
                url: "https://github.com/uben0/tree-sitter-typst".to_string(),
                commit: "master".to_string(),
                subpath: None,
            },
        }),
        block_comment_affixes: Some(("/*".to_string(), "*/".to_string())),
        ..Language::new()
    }
}

fn toml() -> Language {
    Language {
        extensions: to_vec(&["toml"]),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "toml".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Toml),
        }),
        line_comment_prefix: Some("#".to_string()),
        ..Language::new()
    }
}

fn tree_sitter_query() -> Language {
    Language {
        extensions: to_vec(&["scm"]),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "tsq".to_string(),
            kind: GrammarConfigKind::FromSource {
                url: "https://github.com/tree-sitter/tree-sitter-tsq".to_string(),
                commit: "main".to_string(),
                subpath: None,
            },
        }),
        line_comment_prefix: Some(";".to_string()),
        ..Language::new()
    }
}

fn typescript() -> Language {
    Language {
        extensions: to_vec(&["ts", "mts", "cts"]),
        formatter: Some(Command::new("prettierd", &[".ts"])),
        lsp_command: Some(LspCommand {
            command: Command::new("typescript-language-server", &["--stdio"]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("typescript")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "typescript".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Typescript),
        }),
        line_comment_prefix: Some("//".to_string()),
        block_comment_affixes: Some(("/*".to_string(), "*/".to_string())),
        ..Language::new()
    }
}

fn typescriptreact() -> Language {
    Language {
        extensions: to_vec(&["tsx"]),
        formatter: Some(Command::new("prettierd", &[".tsx"])),
        lsp_command: Some(LspCommand {
            command: Command::new("typescript-language-server", &["--stdio"]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("typescriptreact")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "tsx".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::TSX),
        }),
        line_comment_prefix: Some("//".to_string()),
        block_comment_affixes: Some(("/*".to_string(), "*/".to_string())),
        ..Language::new()
    }
}

fn unison() -> Language {
    Language {
        extensions: to_vec(&["u"]),
        lsp_command: Some(LspCommand {
            command: Command::new("nc", &["localhost", "5757"]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("unison")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "unison".to_string(),
            kind: GrammarConfigKind::FromSource {
                url: "https://github.com/kylegoetz/tree-sitter-unison".to_string(),
                commit: "master".to_string(),
                subpath: None,
            },
        }),
        ..Language::new()
    }
}

fn xml() -> Language {
    Language {
        extensions: to_vec(&["xml", "xaml", "axaml"]),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "xml".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::XML),
        }),
        block_comment_affixes: Some(("<!--".to_string(), "-->".to_string())),
        ..Language::new()
    }
}

fn yaml() -> Language {
    Language {
        extensions: to_vec(&["yaml", "yml"]),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "yaml".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::YAML),
        }),
        line_comment_prefix: Some("#".to_string()),
        ..Language::new()
    }
}

fn zig() -> Language {
    Language {
        extensions: to_vec(&["zig"]),
        formatter: Some(Command::new("zig", &["fmt", "--stdin"])),
        lsp_command: Some(LspCommand {
            command: Command::new("zls", &[]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("zig")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "zig".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Zig),
        }),
        line_comment_prefix: Some("//".to_string()),
        ..Language::new()
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_languages_match_nvim_treesitter_languages() {
        const MISSING_NVIM_HIGHLIGHTS: &[&str] = &["dune", "ki_quickfix", "tsq"];

        // This test is a major consistency check.
        // First, we check that all builtin languages were searched for in nvim-treesitter.
        // Second, we check that all languages except those listed above produced queries in nvim-treesitter.
        // Third, we check that the languages listed above did not produce queries in nvim-treesitter.
        // Fourth, we check that all languages except those above can process properly, meaning their parents exist too.
        let ts_languages = nvim_treesitter_highlight_queries::all();
        let ts_ids: Vec<_> = super::languages()
            .into_values()
            .filter_map(|lang| lang.tree_sitter_grammar_config)
            .map(|ts_config| ts_config.id)
            .collect();

        for lang in &ts_ids {
            assert!(ts_languages.get(lang).is_some(), "{lang} was not searched for in nvim-treesitter! Fix nvim-treesitter-highlight-queries build.rs");
        }
        for lang in ts_ids
            .iter()
            .filter(|lang| !MISSING_NVIM_HIGHLIGHTS.contains(&lang.as_str()))
        {
            assert!(ts_languages.get(lang).unwrap().is_some(), "{lang} was searched for in nvim-treesitter but not found, and it is not in the list of exclusions!");
        }
        for lang in MISSING_NVIM_HIGHLIGHTS.iter() {
            assert!(
                ts_languages.get(&**lang).and_then(Option::as_ref).is_none(),
                "{lang} is in the exclusion list but was found in nvim-treesitter!"
            );
        }
        for lang in ts_ids
            .iter()
            .filter(|lang| !MISSING_NVIM_HIGHLIGHTS.contains(&lang.as_str()))
        {
            assert!(crate::ts_highlight_query::get_highlight_query(lang).is_some(), "{lang} is not in the exclusion list but its highlight query didn't process! Is its parent missing from nvim-treesitter-highlight-queries build.rs?");
        }
    }
}
