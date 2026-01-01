const LANGS: &[&str] = &[
    "bash",
    "c",
    "c_sharp",
    "commonlisp",
    "cpp",
    "css",
    "csv",
    "diff",
    "dockerfile",
    "dune",
    "ecma",
    "elixir",
    "fish",
    "fsharp",
    "git_config",
    "git_rebase",
    "gitattributes",
    "gitcommit",
    "gitignore",
    "gleam",
    "go",
    "graphql",
    "hare",
    "haskell",
    "heex",
    "html",
    "html_tags",
    "idris",
    "javascript",
    "json",
    "jsx",
    "julia",
    "just",
    "ki_quickfix",
    "lua",
    "markdown",
    "nix",
    "ocaml",
    "ocaml_interface",
    "odin",
    "python",
    "racket",
    "rescript",
    "roc",
    "ruby",
    "rust",
    "scheme",
    "sql",
    "svelte",
    "swift",
    "toml",
    "tsq",
    "tsv",
    "tsx",
    "typescript",
    "typst",
    "unison",
    "xml",
    "yaml",
    "zig",
];

const MISSING_NVIM_HIGHLIGHTS: &[&str] = &["dune", "ki_quickfix", "tsq"];

fn main() {
    let compiled_highlight_query_path = std::path::PathBuf::from(
        std::env::var_os("OUT_DIR").expect("Cargo didn't give us an OUT_DIR?"),
    )
    .join("compiled_highlight_queries.bin");

    // Format is lang={query}, null-separated
    let data = LANGS
        .iter()
        .map(|lang| {
            let path = format!("nvim-treesitter/runtime/queries/{lang}/highlights.scm");
            let content =
                std::fs::read_to_string(path.clone()).unwrap_or_else(|e| match e.kind() {
                    std::io::ErrorKind::NotFound => {
                        assert!(
                            MISSING_NVIM_HIGHLIGHTS.contains(lang),
                            "Non-whitelisted language {lang} has no nvim-treesitter highlight query! Is the submodule initialized?",
                        );
                        String::new()
                    }
                    _ => panic!("Got error {e:?} when opening highlight query for {lang}"),
                });
            format!("{lang}={content}")
        })
        .collect::<Vec<_>>()
        .join("\0");

    // 6 is the default compression level and a good compromise.
    let compressed_data = miniz_oxide::deflate::compress_to_vec(data.as_bytes(), 6);

    std::fs::write(compiled_highlight_query_path, compressed_data)
        .expect("Failed to write compiled theme contents");
}
