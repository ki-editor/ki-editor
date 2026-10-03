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
        ("zsh", zsh()),
        ("fish", fish()),
        ("unison", unison()),
        ("c", c()),
        ("make", make()),
        ("racket", racket()),
        ("scheme", scheme()),
        ("commonlisp", commonlisp()),
        ("cpp", cpp()),
        ("c_sharp", c_sharp()),
        ("css", css()),
        ("scss", scss()),
        ("csv", csv()),
        ("devicetree", devicetree()),
        ("diff", diff()),
        ("dockerfile", dockerfile()),
        ("elixir", elixir()),
        ("fsharp", fsharp()),
        ("gherkin", gherkin()),
        ("gitattributes", gitattributes()),
        ("gitcommit", gitcommit()),
        ("gitconfig", gitconfig()),
        ("gitignore", gitignore()),
        ("gitrebase", gitrebase()),
        ("jjdescription", jjdescription()),
        ("gleam", gleam()),
        ("go", go()),
        ("graphql", graphql()),
        ("gnuplot", gnuplot()),
        ("hare", hare()),
        ("hcl", hcl()),
        ("heex", heex()),
        ("latex", latex()),
        ("html", html()),
        ("idris", idris()),
        ("haskell", haskell()),
        ("java", java()),
        ("javascript", javascript()),
        ("qml", qml()),
        ("qmldir", qmldir()),
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
        ("php", php()),
        ("python", python()),
        ("perl", perl()),
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
        ("kdl", kdl()),
        ("zig", zig()),
        ("clojure", clojure()),
        ("scala", scala()),
        ("glsl", glsl()),
        ("wit", wit()),
    ]
    .into_iter()
    .map(|(str, language)| (str.to_string(), language))
    .collect()
}

fn bash() -> Language {
    Language {
        extensions: to_vec(&["sh", "bash"]),
        file_names: to_vec(&[".bashrc", ".bash_profile", "bashrc", "bash_profile"]),
        formatter: Some(Command::new("shfmt", &[])),
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

fn zsh() -> Language {
    Language {
        extensions: to_vec(&["zsh"]),
        file_names: to_vec(&[".zshrc", ".zprofile", ".zshenv", ".zlogout"]),
        // There is no formatter and lsp for zsh but since zsh is a superset pretty close to bash
        // we can mostly use the bash one as-is.
        // For example, helix just consider all zsh files to just be bash.
        formatter: Some(Command::new("shfmt", &[])),
        lsp_command: Some(LspCommand {
            command: Command::new("bash-language-server", &["start"]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("zsh")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "zsh".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Zsh),
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

fn make() -> Language {
    Language {
        file_names: to_vec(&["Makefile", "makefile", "GNUmakefile"]),
        formatter: Some(Command::new("mbake", &["format", "--stdin"])),
        lsp_language_id: Some(LanguageId::new("make")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "make".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Make),
        }),
        line_comment_prefix: Some("#".to_string()),
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
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::CSharp),
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

fn scss() -> Language {
    Language {
        extensions: to_vec(&["scss"]),
        formatter: Some(Command::new("prettierd", &[".scss"])),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "scss".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Scss),
        }),
        line_comment_prefix: Some("//".to_string()),
        block_comment_affixes: Some(("/*".to_string(), "*/".to_string())),
        ..Language::new()
    }
}

fn devicetree() -> Language {
    Language {
        extensions: to_vec(&["dts", "dtsi", "keymap"]),
        formatter: None,
        lsp_command: None,
        lsp_language_id: None,
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "devicetree".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Devicetree),
        }),
        line_comment_prefix: Some("//".to_string()),
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
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Dockerfile),
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
            environment: HashMap::new(),
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
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::FSharp),
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
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Gitattributes),
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
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Gitcommit),
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

fn jjdescription() -> Language {
    Language {
        extensions: to_vec(&["jjdescription"]),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "jj description".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::JjDescription),
        }),
        line_comment_prefix: Some("JJ".to_string()),
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
            environment: HashMap::new(),
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

fn gnuplot() -> Language {
    Language {
        extensions: to_vec(&["gnu", "gnuplot", "gp", "plot", "plt"]),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "gnuplot".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Gnuplot),
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

fn hcl() -> Language {
    Language {
        extensions: to_vec(&["tf"]),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "hcl".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Hcl),
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
            environment: HashMap::new(),
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

fn latex() -> Language {
    Language {
        extensions: to_vec(&["tex"]),
        formatter: Some(Command::new("tex-fmt", &["-s"])),
        lsp_command: Some(LspCommand {
            command: Command::new("texlab", &[]),
            initialization_options: None,
            environment: HashMap::new(),
        }),
        lsp_language_id: Some(LanguageId::new("latex")),
        line_comment_prefix: Some("%".to_string()),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "latex".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Latex),
        }),
        ..Language::new()
    }
}

/// Based on nvim-treesitter's `html_tags` injections, shared by languages that embed HTML
/// elements. `#lua-match?` and `#gsub!` are rewritten as `#match?` and explicit `type` values.
/// Not ported: `style="..."` (a declaration list is not a stylesheet), `on*="..."` handlers
/// (the attribute value is highlighted as a string by the host, which tree-sitter-highlight
/// lets win over the injected highlights at the start of the range), lit-html `${}`
/// attributes (needs `#offset!`), `pattern="..."` (no regex language), and comments.
const HTML_TAGS_INJECTION_QUERY: &str = r#"
; <style>...</style>; `lang`/`type` attributes are handled by the rules below
((style_element
  (start_tag) @_start_tag
  (raw_text) @injection.content)
  (#not-match? @_start_tag "\\s(lang|type)\\s*=")
  (#set! injection.language "css"))

((style_element
  (start_tag
    (attribute
      (attribute_name) @_attr
      (quoted_attribute_value
        (attribute_value) @_type)))
  (raw_text) @injection.content)
  (#eq? @_attr "type")
  (#eq? @_type "text/css")
  (#set! injection.language "css"))

; <script>...</script>
((script_element
  (start_tag) @_start_tag
  (raw_text) @injection.content)
  (#not-match? @_start_tag "\\s(lang|type)\\s*=")
  (#set! injection.language "javascript"))

; <script type="module">, <script type="text/javascript">
((script_element
  (start_tag
    (attribute
      (attribute_name) @_attr
      (quoted_attribute_value
        (attribute_value) @_type)))
  (raw_text) @injection.content)
  (#eq? @_attr "type")
  (#any-of? @_type "module" "text/javascript" "application/javascript" "text/ecmascript" "application/ecmascript")
  (#set! injection.language "javascript"))

((script_element
  (start_tag
    (attribute
      (attribute_name) @_attr
      (quoted_attribute_value
        (attribute_value) @_type)))
  (raw_text) @injection.content)
  (#eq? @_attr "type")
  (#any-of? @_type "text/typescript" "application/typescript")
  (#set! injection.language "typescript"))

; <script type="importmap">, <script type="application/json">
((script_element
  (start_tag
    (attribute
      (attribute_name) @_attr
      (quoted_attribute_value
        (attribute_value) @_type)))
  (raw_text) @injection.content)
  (#eq? @_attr "type")
  (#any-of? @_type "importmap" "application/json")
  (#set! injection.language "json"))
"#;

/// PyScript injections, which nvim-treesitter adds on top of `html_tags`.
const HTML_INJECTION_QUERY: &str = r#"
; PyScript: <py-script>, <py-repl>, <script type="pyscript">
((element
  (start_tag
    (tag_name) @_py_script)
  (text) @injection.content)
  (#any-of? @_py_script "py-script" "py-repl")
  (#set! injection.language "python"))

((script_element
  (start_tag
    (attribute
      (attribute_name) @_attr
      (quoted_attribute_value
        (attribute_value) @_type)))
  (raw_text) @injection.content)
  (#eq? @_attr "type")
  (#any-of? @_type "pyscript" "py-script")
  (#set! injection.language "python"))

((element
  (start_tag
    (tag_name) @_py_config)
  (text) @injection.content)
  (#eq? @_py_config "py-config")
  (#set! injection.language "toml"))
"#;

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
        injection_query: Some(format!("{HTML_TAGS_INJECTION_QUERY}{HTML_INJECTION_QUERY}")),
        injected_languages: to_vec(&["css", "javascript", "json", "python", "toml", "typescript"]),
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
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Idris),
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

fn java() -> Language {
    Language {
        extensions: to_vec(&["java"]),
        lsp_command: Some(LspCommand {
            command: Command::new("jdtls", &[]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("java")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "java".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Java),
        }),
        line_comment_prefix: Some("//".to_string()),
        block_comment_affixes: Some(("/*".to_string(), "*/".to_string())),
        ..Language::new()
    }
}

/// Based on nvim-treesitter's `ecma` injections. Template literals are captured through their
/// `string_fragment` children, which sidesteps `#offset!` (unsupported) for the backticks.
/// CSS-in-JS is parsed as plain CSS rather than nvim-treesitter's `styled`. Not ported: jsdoc,
/// regex, groq, glimmer and angular.
const ECMA_INJECTION_QUERY: &str = r#"
; html`...`, html(`...`), sql`...`, graphql`...`; template substitutions are
; skipped and the remaining fragments are parsed as one document
(call_expression
  function: (identifier) @injection.language
  arguments: [
    (arguments
      (template_string
        (string_fragment) @injection.content))
    (template_string
      (string_fragment) @injection.content)
  ]
  (#any-of? @injection.language "html" "sql" "graphql")
  (#set! injection.combined))

; svg`...` or svg(`...`)
(call_expression
  function: (identifier) @_name
  arguments: [
    (arguments
      (template_string
        (string_fragment) @injection.content))
    (template_string
      (string_fragment) @injection.content)
  ]
  (#eq? @_name "svg")
  (#set! injection.language "html")
  (#set! injection.combined))

; gql`...`
(call_expression
  function: (identifier) @_name
  arguments: (template_string
    (string_fragment) @injection.content)
  (#eq? @_name "gql")
  (#set! injection.language "graphql")
  (#set! injection.combined))

; foo.sql`...` or foo.sql(`...`)
(call_expression
  function: (member_expression
    property: (property_identifier) @_name)
  arguments: [
    (arguments
      (template_string
        (string_fragment) @injection.content))
    (template_string
      (string_fragment) @injection.content)
  ]
  (#eq? @_name "sql")
  (#set! injection.language "sql")
  (#set! injection.combined))

; /* tagged by a leading #graphql comment */
((template_string
  (string_fragment) @injection.content)
  (#match? @injection.content "^#graphql")
  (#set! injection.language "graphql"))

; css`...`, keyframes`...`
(call_expression
  function: (identifier) @_name
  arguments: (template_string
    (string_fragment) @injection.content)
  (#any-of? @_name "css" "keyframes")
  (#set! injection.language "css")
  (#set! injection.combined))

; styled.div`...`
(call_expression
  function: (member_expression
    object: (identifier) @_name)
  arguments: (template_string
    (string_fragment) @injection.content)
  (#eq? @_name "styled")
  (#set! injection.language "css")
  (#set! injection.combined))

; styled(Component)`...`
(call_expression
  function: (call_expression
    function: (identifier) @_name)
  arguments: (template_string
    (string_fragment) @injection.content)
  (#eq? @_name "styled")
  (#set! injection.language "css")
  (#set! injection.combined))

; styled.div.attrs({ prop: "foo" })`...`
(call_expression
  function: (call_expression
    function: (member_expression
      object: (member_expression
        object: (identifier) @_name)))
  arguments: (template_string
    (string_fragment) @injection.content)
  (#eq? @_name "styled")
  (#set! injection.language "css")
  (#set! injection.combined))

; styled(Component).attrs({ prop: "foo" })`...`
(call_expression
  function: (call_expression
    function: (member_expression
      object: (call_expression
        function: (identifier) @_name)))
  arguments: (template_string
    (string_fragment) @injection.content)
  (#eq? @_name "styled")
  (#set! injection.language "css")
  (#set! injection.combined))

; el.innerHTML = `<b>x</b>` or el.innerHTML = '<b>x</b>'
(assignment_expression
  left: (member_expression
    property: (property_identifier) @_prop)
  right: [
    (template_string
      (string_fragment) @injection.content)
    (string
      (string_fragment) @injection.content)
  ]
  (#any-of? @_prop "outerHTML" "innerHTML")
  (#set! injection.language "html")
  (#set! injection.combined))

; @Component({ styles: [`...`] }) and @Component({ styles: `...` })
(decorator
  (call_expression
    function: (identifier) @_name
    arguments: (arguments
      (object
        (pair
          key: (property_identifier) @_prop
          value: [
            (array
              (template_string
                (string_fragment) @injection.content))
            (template_string
              (string_fragment) @injection.content)
          ]))))
  (#eq? @_name "Component")
  (#eq? @_prop "styles")
  (#set! injection.language "css")
  (#set! injection.combined))
"#;

/// Based on nvim-treesitter's `jsx` injections.
const JSX_INJECTION_QUERY: &str = r#"
; <style jsx>{`...`}</style>
(jsx_element
  (jsx_opening_element
    (identifier) @_name
    (jsx_attribute) @_attr)
  (jsx_expression
    (template_string
      (string_fragment) @injection.content))
  (#eq? @_name "style")
  (#eq? @_attr "jsx")
  (#set! injection.language "css")
  (#set! injection.combined))
"#;

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
        injection_query: Some(format!("{ECMA_INJECTION_QUERY}{JSX_INJECTION_QUERY}")),
        injected_languages: to_vec(&["css", "graphql", "html", "sql"]),
        ..Language::new()
    }
}

fn qml() -> Language {
    Language {
        extensions: to_vec(&["qml"]),
        formatter: None,
        lsp_command: Some(LspCommand {
            command: Command::new("qmlls6", &[]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("qmljs")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "qmljs".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::QmlJs),
        }),
        line_comment_prefix: Some("//".to_string()),
        block_comment_affixes: Some(("/*".to_string(), "*/".to_string())),
        ..Language::new()
    }
}

fn qmldir() -> Language {
    Language {
        file_names: to_vec(&["qmldir"]),
        formatter: None,
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "qmldir".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::QmlDir),
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
        injection_query: Some(format!("{ECMA_INJECTION_QUERY}{JSX_INJECTION_QUERY}")),
        injected_languages: to_vec(&["css", "graphql", "html", "sql"]),
        ..Language::new()
    }
}

/// Svelte-specific injections on top of [`HTML_TAGS_INJECTION_QUERY`]. `pug` is not ported.
const SVELTE_INJECTION_QUERY: &str = r#"
((svelte_raw_text) @injection.content
  (#set! injection.language "javascript"))

((style_element
  (start_tag
    (attribute
      (attribute_name) @_attr
      (quoted_attribute_value
        (attribute_value) @_lang)))
  (raw_text) @injection.content)
  (#eq? @_attr "lang")
  (#any-of? @_lang "scss" "postcss" "less")
  (#set! injection.language "scss"))

((script_element
  (start_tag
    (attribute
      (attribute_name) @_attr
      (quoted_attribute_value
        (attribute_value) @_lang)))
  (raw_text) @injection.content)
  (#eq? @_attr "lang")
  (#any-of? @_lang "ts" "typescript")
  (#set! injection.language "typescript"))

((script_element
  (start_tag
    (attribute
      (attribute_name) @_attr
      (quoted_attribute_value
        (attribute_value) @_lang)))
  (raw_text) @injection.content)
  (#eq? @_attr "lang")
  (#any-of? @_lang "js" "javascript")
  (#set! injection.language "javascript"))
"#;

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
        injection_query: Some(format!(
            "{HTML_TAGS_INJECTION_QUERY}{SVELTE_INJECTION_QUERY}"
        )),
        injected_languages: to_vec(&["css", "javascript", "json", "scss", "typescript"]),
        ..Language::new()
    }
}

fn json() -> Language {
    Language {
        extensions: to_vec(&["json", "gyp"]),
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
        file_names: to_vec(&["justfile", "Justfile"]),
        extensions: to_vec(&["just"]),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "just".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Just),
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

/// Based on `tree_sitter_md::INJECTION_QUERY_BLOCK`, except that
/// `injection.include-children` is set on every rule: block nodes have one
/// `block_continuation` child per line, which would otherwise be excluded from the
/// injected ranges and split the embedded code into unparsable fragments.
const MARKDOWN_INJECTION_QUERY: &str = r#"
(fenced_code_block
  (info_string
    (language) @injection.language)
  (code_fence_content) @injection.content
  (#set! injection.include-children))

((html_block) @injection.content
  (#set! injection.language "html")
  (#set! injection.include-children))

(document . (section . (thematic_break) (_) @injection.content (thematic_break))
  (#set! injection.language "yaml")
  (#set! injection.include-children))

([(minus_metadata) (plus_metadata)] @injection.content
  (#set! injection.language "yaml")
  (#set! injection.include-children))
"#;

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
        injection_query: Some(MARKDOWN_INJECTION_QUERY.to_string()),
        injected_languages: to_vec(&[
            "bash",
            "css",
            "html",
            "javascript",
            "json",
            "python",
            "rust",
            "toml",
            "typescript",
            "yaml",
        ]),
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
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Odin),
        }),
        line_comment_prefix: Some("//".to_string()),
        block_comment_affixes: Some(("/*".to_string(), "*/".to_string())),
        lsp_language_id: Some(LanguageId::new("odin")),
        lsp_command: Some(LspCommand {
            command: Command::new("ols", &[]),
            ..LspCommand::default()
        }),
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

/// Based on nvim-treesitter's `php_only` and `php` injections. Not ported: phpdoc, regex
/// (`preg_*`), heredoc/nowdoc (the language is the case-sensitive label) and
/// `shell_exec("...")` and friends (the host highlights the string content, which
/// tree-sitter-highlight lets win over the injected highlights).
const PHP_INJECTION_QUERY: &str = r#"
; Inline HTML outside of <?php ... ?>
((text) @injection.content
  (#set! injection.language "html")
  (#set! injection.combined))

; `ls -la`
((shell_command_expression
  (string_content) @injection.content)
  (#set! injection.language "bash"))
"#;

fn php() -> Language {
    Language {
        extensions: to_vec(&["php", "php3", "php4", "php5", "php7", "phtml"]),
        lsp_language_id: Some(LanguageId::new("php")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "php".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Php),
        }),
        line_comment_prefix: Some("//".to_string()),
        block_comment_affixes: Some(("/*".to_string(), "*/".to_string())),
        injection_query: Some(PHP_INJECTION_QUERY.to_string()),
        injected_languages: to_vec(&[
            "bash",
            "css",
            "html",
            "javascript",
            "json",
            "python",
            "toml",
            "typescript",
        ]),
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

fn perl() -> Language {
    Language {
        extensions: to_vec(&[
            "pl", "pm", "t", "psgi", "raku", "rakumod", "rakutest", "rakudoc", "nqp", "p6", "pl6",
            "pm6",
        ]),
        //formatter: Some(Command::new("pertidy"])),
        lsp_command: Some(LspCommand {
            command: Command::new("perlnavigator", &[]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("perl")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "perl".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Perl),
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
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Rescript),
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
            &["--fix-layout", "--stdin", "file.rb", "--stderr"],
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

fn gherkin() -> Language {
    Language {
        extensions: to_vec(&["feature"]),
        formatter: None,
        lsp_command: None,
        lsp_language_id: Some(LanguageId::new("feature")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "gherkin".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Gherkin),
        }),
        line_comment_prefix: Some("#".to_string()),
        ..Language::new()
    }
}

fn roc() -> Language {
    Language {
        extensions: to_vec(&["roc"]),
        formatter: Some(Command::new("roc", &["fmt", "--stdin"])),
        lsp_command: None,
        lsp_language_id: Some(LanguageId::new("roc")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "roc".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Roc),
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
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Sql),
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
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Typst),
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

/// Based on nvim-treesitter's `typescript` injections, on top of [`ECMA_INJECTION_QUERY`].
const TYPESCRIPT_INJECTION_QUERY: &str = r#"
; styled.div<{}>`...`
(call_expression
  function: (non_null_expression
    (instantiation_expression
      (member_expression
        object: (identifier) @_name
        property: (property_identifier))
      type_arguments: (type_arguments)))
  arguments: (template_string
    (string_fragment) @injection.content)
  (#eq? @_name "styled")
  (#set! injection.language "css")
  (#set! injection.combined))

; styled.div<T>`...`
(binary_expression
  left: (binary_expression
    left: (member_expression
      object: (identifier) @_name
      property: (property_identifier))
    right: (identifier))
  right: (template_string
    (string_fragment) @injection.content)
  (#eq? @_name "styled")
  (#set! injection.language "css")
  (#set! injection.combined))
"#;

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
        injection_query: Some(format!(
            "{ECMA_INJECTION_QUERY}{TYPESCRIPT_INJECTION_QUERY}"
        )),
        injected_languages: to_vec(&["css", "graphql", "html", "sql"]),
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
        injection_query: Some(format!(
            "{ECMA_INJECTION_QUERY}{TYPESCRIPT_INJECTION_QUERY}{JSX_INJECTION_QUERY}"
        )),
        injected_languages: to_vec(&["css", "graphql", "html", "sql"]),
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

/// Based on nvim-treesitter's `xml` injections, except that `injection.combined` is not set so
/// that every element is parsed on its own.
const XML_INJECTION_QUERY: &str = r#"
; <style> and <script> (e.g. in SVG). Children are included because the character
; data of an element is a child of its `content` node.
((element
  (STag
    (Name) @_name)
  (content) @injection.content)
  (#eq? @_name "style")
  (#set! injection.include-children)
  (#set! injection.language "css"))

((element
  (STag
    (Name) @_name)
  (content) @injection.content)
  (#eq? @_name "script")
  (#set! injection.include-children)
  (#set! injection.language "javascript"))

; phpMyAdmin dump
((element
  (STag
    (Name) @_name)
  (content) @injection.content)
  (#eq? @_name "pma:table")
  (#set! injection.include-children)
  (#set! injection.language "sql"))
"#;

fn xml() -> Language {
    Language {
        extensions: to_vec(&["xml", "xaml", "axaml"]),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "xml".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::XML),
        }),
        block_comment_affixes: Some(("<!--".to_string(), "-->".to_string())),
        injection_query: Some(XML_INJECTION_QUERY.to_string()),
        injected_languages: to_vec(&["css", "javascript", "sql"]),
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

fn kdl() -> Language {
    Language {
        extensions: to_vec(&["kdl"]),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "kdl".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Kdl),
        }),
        line_comment_prefix: Some("//".to_string()),
        block_comment_affixes: Some(("/-".to_string(), "}".to_string())),
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

fn clojure() -> Language {
    Language {
        extensions: to_vec(&["clj", "cljs"]),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "clojure".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Clojure),
        }),
        ..Language::new()
    }
}

fn scala() -> Language {
    Language {
        extensions: to_vec(&["scala"]),
        formatter: Some(Command::new("scalafmt", &["--stdin"])),
        lsp_command: Some(LspCommand {
            command: Command::new("metals", &[]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("scala")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "scala".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Scala),
        }),
        line_comment_prefix: Some("//".to_string()),
        block_comment_affixes: Some(("/*".to_string(), "*/".to_string())),
        ..Language::new()
    }
}

fn glsl() -> Language {
    Language {
        extensions: to_vec(&["glsl"]),
        formatter: Some(Command::new("clang-format", &[])),
        lsp_command: Some(LspCommand {
            command: Command::new("glsl_analyzer", &[]),
            ..LspCommand::default()
        }),
        lsp_language_id: Some(LanguageId::new("glsl")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "glsl".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Glsl),
        }),
        line_comment_prefix: Some("//".to_string()),
        block_comment_affixes: Some(("/*".to_string(), "*/".to_string())),
        ..Language::new()
    }
}

fn wit() -> Language {
    Language {
        extensions: to_vec(&["wit"]),
        formatter: None,
        lsp_command: None,
        lsp_language_id: Some(LanguageId::new("wit")),
        tree_sitter_grammar_config: Some(GrammarConfig {
            id: "wit".to_string(),
            kind: GrammarConfigKind::CargoLinked(CargoLinkedTreesitterLanguage::Wit),
        }),
        line_comment_prefix: Some("//".to_string()),
        block_comment_affixes: Some(("/*".to_string(), "*/".to_string())),
        ..Language::new()
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_languages_match_nvim_treesitter_languages() {
        const MISSING_NVIM_HIGHLIGHTS: &[&str] = &[
            "dune",
            "ki_quickfix",
            "tsq",
            "jj description",
            "qml",
            "gherkin",
        ];

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
            assert!(ts_languages.contains_key(lang), "{lang} was not searched for in nvim-treesitter! Add '{lang}' to INCLUDED_NVIM_TREESITTER_LANGUAGES");
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
