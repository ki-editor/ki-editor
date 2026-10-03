//! Data-driven tests for every language injection supported out of the box.
//!
//! To cover a new injection, add an [`InjectionCase`] to [`cases`]: the host
//! language, the embedded language, and a source template around a snippet.
//! Every case is checked for:
//! 1. Highlighting parity: each non-whitespace byte of the snippet that is styled when the
//!    snippet is highlighted as a standalone file of the embedded language is styled the
//!    same when embedded. Bytes that are unstyled standalone may take the host's style
//!    (e.g. the markup style of a markdown code block).
//! 2. Structural layer: a selection inside the snippet resolves to the injected syntax
//!    tree layer, while a selection outside of it resolves to the host layer.

use itertools::Itertools;

use crate::{
    buffer::Buffer,
    char_index_range::CharIndexRange,
    selection::{CharIndex, Selection},
};

use shared::language::Language;

use super::{HighlightConfigs, HighlightedSpans};

struct InjectionCase {
    host_extension: &'static str,
    embedded_extension: &'static str,
    /// Source text before the snippet.
    prefix: &'static str,
    snippet: &'static str,
    /// Source text after the snippet.
    suffix: &'static str,
    /// Parts of the snippet that are host code (e.g. `${x}` in a template literal), which are
    /// left out of the highlighting parity check.
    host_spans: &'static [&'static str],
}

impl InjectionCase {
    fn name(&self) -> String {
        format!("{} -> {}", self.host_extension, self.embedded_extension)
    }

    fn source(&self) -> String {
        format!("{}{}{}", self.prefix, self.snippet, self.suffix)
    }
}

fn markdown_fence(
    embedded_extension: &'static str,
    info_string: &'static str,
    snippet: &'static str,
) -> InjectionCase {
    // The leading text is leaked once per case so that cases can stay `&'static str`
    // without a separate constant per language.
    InjectionCase {
        host_extension: "md",
        embedded_extension,
        prefix: Box::leak(format!("# Title\n\n```{info_string}\n").into_boxed_str()),
        snippet,
        suffix: "```\n",
        host_spans: &[],
    }
}

fn embed(
    host_extension: &'static str,
    embedded_extension: &'static str,
    prefix: &'static str,
    snippet: &'static str,
    suffix: &'static str,
) -> InjectionCase {
    InjectionCase {
        host_extension,
        embedded_extension,
        prefix,
        snippet,
        suffix,
        host_spans: &[],
    }
}

fn cases() -> Vec<InjectionCase> {
    [
        markdown_fence(
            "sh",
            "bash",
            "echo \"hello\"\nfor f in *.md; do ls \"$f\"; done\n",
        ),
        markdown_fence("css", "css", "a {\n  color: red;\n}\n"),
        markdown_fence("html", "html", "<div class=\"x\">hello</div>\n"),
        markdown_fence(
            "js",
            "javascript",
            "const x = 1;\nfunction f(a) { return a + x; }\n",
        ),
        markdown_fence("json", "json", "{\"answer\": 42, \"ok\": true}\n"),
        markdown_fence("py", "python", "def f(x):\n    return x + 1\n"),
        markdown_fence(
            "rs",
            "rust",
            "fn main() {\n    let x: u32 = 42;\n    println!(\"hello {x}\");\n}\n",
        ),
        markdown_fence("toml", "toml", "[package]\nname = \"ki\"\n"),
        markdown_fence(
            "ts",
            "typescript",
            "let x: number = 1;\nconst f = (a: string) => a;\n",
        ),
        markdown_fence("yaml", "yaml", "a: 1\nb:\n  - c\n"),
        InjectionCase {
            host_extension: "md",
            embedded_extension: "html",
            prefix: "# Title\n\n",
            snippet: "<div class=\"x\">hello</div>\n",
            suffix: "\n# After\n",
            host_spans: &[],
        },
        InjectionCase {
            host_extension: "md",
            embedded_extension: "yaml",
            prefix: "---\n",
            snippet: "title: Hello\ntags:\n  - a\n",
            suffix: "---\n\n# Body\n",
            host_spans: &[],
        },
    ]
    .into_iter()
    .chain(web_cases())
    .chain(beam_and_ml_cases())
    .chain(shell_cases())
    .chain(misc_cases())
    .collect()
}

const JS: &str = "const x = 1;\nfunction f(a) { return a + x; }\n";
const CSS: &str = "a {\n  color: red;\n}\n";
const HTML: &str = "<div class=\"x\">hello</div>";

fn web_cases() -> Vec<InjectionCase> {
    vec![
        // html
        embed(
            "html",
            "js",
            "<p>x</p>\n<script defer>\n",
            JS,
            "</script>\n",
        ),
        embed(
            "html",
            "js",
            "<script type=\"module\">\n",
            JS,
            "</script>\n",
        ),
        embed(
            "html",
            "js",
            "<script async type=\"text/javascript\">\n",
            JS,
            "</script>\n",
        ),
        embed(
            "html",
            "ts",
            "<script type=\"text/typescript\">\n",
            "let x: number = 1;\n",
            "</script>\n",
        ),
        embed(
            "html",
            "json",
            "<script type=\"importmap\">\n",
            "{\"imports\": {\"a\": \"./a.js\"}}\n",
            "</script>\n",
        ),
        embed(
            "html",
            "json",
            "<script type=\"application/json\">\n",
            "{\"a\": [1, 2, true]}\n",
            "</script>\n",
        ),
        embed("html", "css", "<style>\n", CSS, "</style>\n"),
        embed(
            "html",
            "css",
            "<style type=\"text/css\">\n",
            CSS,
            "</style>\n",
        ),
        embed(
            "html",
            "py",
            "<py-script>\n",
            "def f(x):\n    return x + 1\n",
            "</py-script>\n",
        ),
        embed(
            "html",
            "py",
            "<script type=\"pyscript\">\n",
            "def f(x):\n    return x + 1\n",
            "</script>\n",
        ),
        embed(
            "html",
            "toml",
            "<py-config>\n",
            "[package]\nname = \"ki\"\n",
            "</py-config>\n",
        ),
        // svelte
        embed(
            "svelte",
            "js",
            "<div>x</div>\n<script>\n",
            JS,
            "</script>\n",
        ),
        embed("svelte", "js", "<script lang=\"js\">\n", JS, "</script>\n"),
        embed(
            "svelte",
            "ts",
            "<script lang=\"ts\">\n",
            "let x: number = 1;\nconst f = (a: string) => a;\n",
            "</script>\n",
        ),
        embed("svelte", "css", "<style>\n", CSS, "</style>\n"),
        embed(
            "svelte",
            "scss",
            "<style lang=\"scss\">\n",
            "a {\n  b { color: red; }\n}\n",
            "</style>\n",
        ),
        embed("svelte", "js", "<div>{", "count + 1", "}</div>\n"),
        // php
        embed(
            "php",
            "html",
            "<?php\n$a = 1;\n?>\n",
            "<div class=\"x\">hello</div>\n",
            "<?php echo 1; ?>",
        ),
        InjectionCase {
            host_spans: &["<?php echo $cls; ?>"],
            ..embed(
                "php",
                "html",
                "<?php\n$a = 1;\n?>\n",
                "<div class=\"<?php echo $cls; ?>\">hello</div>\n",
                "<?php echo 1; ?>",
            )
        },
        embed("php", "sh", "<?php\n$x = `", "ls -la | wc -l", "`;\n"),
        // ecmascript
        embed("js", "html", "const a = html`", HTML, "`;\n"),
        embed("js", "html", "const a = html(`", HTML, "`);\n"),
        InjectionCase {
            host_spans: &["${cls}"],
            ..embed(
                "js",
                "html",
                "const a = html`",
                "<div class=\"${cls}\">hello</div>",
                "`;\n",
            )
        },
        InjectionCase {
            host_spans: &["${c}"],
            ..embed(
                "js",
                "css",
                "const a = styled.div`",
                "a {\n  color: red;\n  ${c}\n  background: blue;\n}\n",
                "`;\n",
            )
        },
        embed(
            "js",
            "html",
            "const a = svg`",
            "<svg><circle r=\"1\"></circle></svg>",
            "`;\n",
        ),
        embed("js", "html", "el.innerHTML = `", HTML, "`;\n"),
        embed("js", "html", "el.innerHTML = '", HTML, "';\n"),
        embed(
            "js",
            "sql",
            "const a = sql`",
            "SELECT id FROM users WHERE id = 1",
            "`;\n",
        ),
        embed(
            "js",
            "sql",
            "const a = db.sql`",
            "SELECT id FROM users WHERE id = 1",
            "`;\n",
        ),
        embed(
            "js",
            "graphql",
            "const a = gql`",
            "query { user(id: 1) { name } }",
            "`;\n",
        ),
        embed(
            "js",
            "graphql",
            "const a = graphql`",
            "query { user(id: 1) { name } }",
            "`;\n",
        ),
        embed(
            "js",
            "graphql",
            "const a = `",
            "#graphql\nquery { user(id: 1) { name } }\n",
            "`;\n",
        ),
        embed("js", "css", "const a = css`", CSS, "`;\n"),
        embed("js", "css", "const a = keyframes`", CSS, "`;\n"),
        embed("js", "css", "const a = styled.div`", CSS, "`;\n"),
        embed("js", "css", "const a = styled(Button)`", CSS, "`;\n"),
        embed(
            "js",
            "css",
            "const a = styled.div.attrs({ a: 1 })`",
            CSS,
            "`;\n",
        ),
        embed(
            "js",
            "css",
            "const a = styled(Button).attrs({ a: 1 })`",
            CSS,
            "`;\n",
        ),
        embed("js", "css", "const a = <style jsx>{`", CSS, "`}</style>;\n"),
        embed("jsx", "html", "const a = html`", HTML, "`;\n"),
        embed(
            "jsx",
            "css",
            "const a = <style jsx>{`",
            CSS,
            "`}</style>;\n",
        ),
        embed("ts", "html", "const a = html`", HTML, "`;\n"),
        embed(
            "ts",
            "css",
            "@Component({\n  styles: [`",
            CSS,
            "`],\n})\nclass A {}\n",
        ),
        embed(
            "ts",
            "css",
            "@Component({\n  styles: `",
            CSS,
            "`,\n})\nclass A {}\n",
        ),
        embed(
            "ts",
            "css",
            "const a = styled.div<{ a: number }>`",
            CSS,
            "`;\n",
        ),
        embed("ts", "css", "const a = styled.div<Props>`", CSS, "`;\n"),
        embed("tsx", "html", "const a = html`", HTML, "`;\n"),
        embed("tsx", "css", "const a = styled.div<Props>`", CSS, "`;\n"),
        embed(
            "tsx",
            "css",
            "const a = <style jsx>{`",
            CSS,
            "`}</style>;\n",
        ),
        // xml
        embed(
            "xml",
            "css",
            "<svg>\n<style>",
            "a { color: red; }",
            "</style>\n</svg>\n",
        ),
        embed(
            "xml",
            "js",
            "<svg>\n<script>",
            "const x = 1;",
            "</script>\n</svg>\n",
        ),
        embed(
            "xml",
            "sql",
            "<db>\n<pma:table>",
            "SELECT id FROM users;",
            "</pma:table>\n</db>\n",
        ),
    ]
}

fn beam_and_ml_cases() -> Vec<InjectionCase> {
    vec![
        // elixir
        embed(
            "ex",
            "md",
            "defmodule A do\n  @moduledoc \"\"\"\n",
            "# Title\n\n  Some *text* and `code`.\n",
            "  \"\"\"\nend\n",
        ),
        embed(
            "ex",
            "md",
            "defmodule A do\n  @doc \"",
            "# Title\n",
            "\"\n  def f, do: 1\nend\n",
        ),
        embed(
            "ex",
            "heex",
            "defmodule A do\n  def f(assigns), do: ~H\"\"\"\n",
            "<div class=\"x\"><%= @x %></div>\n",
            "  \"\"\"\nend\n",
        ),
        embed(
            "ex",
            "json",
            "x = ~j(",
            "{\"answer\": 42, \"ok\": true}",
            ")\n",
        ),
        embed(
            "ex",
            "zig",
            "x = ~z\"\"\"\n",
            "const x: u32 = 1;\npub fn f() void {}\n",
            "\"\"\"\n",
        ),
        // heex
        embed("heex", "ex", "<div class={", "@class", "}>x</div>\n"),
        embed("heex", "ex", "<div><%= ", "foo(@x, 1)", " %></div>\n"),
        embed(
            "heex",
            "ex",
            "<%= ",
            "if @a do",
            " %>\n  <p>x</p>\n<% end %>\n",
        ),
        // rescript
        embed("res", "js", "let a = %raw(\"", "let x = 1; x + 1", "\")\n"),
        embed("res", "js", "let a = %raw(`", "let x = 1; x + 1", "`)\n"),
        embed(
            "res",
            "graphql",
            "let a = %graphql(`",
            "query { user(id: 1) { name } }",
            "`)\n",
        ),
        embed(
            "res",
            "graphql",
            "let a = %relay(`",
            "query { user(id: 1) { name } }",
            "`)\n",
        ),
    ]
}

fn shell_cases() -> Vec<InjectionCase> {
    vec![
        // dockerfile
        embed(
            "dockerfile",
            "sh",
            "FROM alpine\nRUN ",
            "echo hello && ls | wc -l",
            "\n",
        ),
        embed(
            "dockerfile",
            "sh",
            "FROM alpine\nRUN ",
            "if true; then \\\n  echo hello; \\\n  fi",
            "\n",
        ),
        // make
        embed("make", "sh", "all:\n\t", "echo hello && ls | wc -l", "\n"),
        embed("make", "sh", "FILES := $(shell ", "ls | wc -l", ")\n"),
        // just
        embed("just", "sh", "x := `", "ls | wc -l", "`\n"),
        embed(
            "just",
            "py",
            "py:\n    #!/usr/bin/env python\n    ",
            "def f(x):\n        return x + 1",
            "\n\nx := 1\n",
        ),
        embed(
            "just",
            "py",
            "py:\n    #!/usr/bin/env python3\n    ",
            "def f(x):\n        return x + 1",
            "\n\nx := 1\n",
        ),
        embed(
            "just",
            "sh",
            "sh:\n    #!/usr/bin/env sh\n    ",
            "echo hello && ls | wc -l",
            "\n\nx := 1\n",
        ),
        embed(
            "just",
            "js",
            "js:\n    #!/usr/bin/env node\n    ",
            "const x = 1;\n    function f(a) { return a + x; }",
            "\n\nx := 1\n",
        ),
        // yaml: the block scalar indicator is part of the injected range
        embed(
            "yaml",
            "sh",
            "steps:\n  - run: |",
            "\n      echo hello\n      ls | wc -l\n",
            "other: 1\n",
        ),
        embed(
            "yaml",
            "sh",
            "build:\n  script:\n    - |",
            "\n      echo hello\n      ls | wc -l\n",
            "other: 1\n",
        ),
    ]
}

/// Looks up a language by file extension, or by language key for languages that are only
/// recognized by file name (such as `dockerfile`).
const NIX_SH: &str = "echo hello\n    ls | wc -l\n  ";

fn misc_cases() -> Vec<InjectionCase> {
    vec![
        // nix
        embed(
            "nix",
            "sh",
            "{\n  buildPhase = ''\n    ",
            NIX_SH,
            "'';\n}\n",
        ),
        embed(
            "nix",
            "sh",
            "{\n  postInstall = ''\n    ",
            NIX_SH,
            "'';\n}\n",
        ),
        embed("nix", "sh", "{\n  script = ''\n    ", NIX_SH, "'';\n}\n"),
        embed(
            "nix",
            "sh",
            "pkgs.writeShellApplication {\n  text = ''\n    ",
            NIX_SH,
            "'';\n}\n",
        ),
        embed(
            "nix",
            "sh",
            "pkgs.runCommand \"name\" { } ''\n  ",
            "echo hello\n  ls | wc -l\n",
            "''\n",
        ),
        embed(
            "nix",
            "sh",
            "pkgs.writeShellScript \"name\" ''\n  ",
            "echo hello\n  ls | wc -l\n",
            "''\n",
        ),
        embed(
            "nix",
            "fish",
            "pkgs.writeFish \"name\" ''\n  ",
            "echo hello\n  set x 1\n",
            "''\n",
        ),
        embed(
            "nix",
            "haskell",
            "pkgs.writeHaskell \"name\" { } ''\n",
            "main :: IO ()\nmain = putStrLn \"hi\"\n",
            "''\n",
        ),
        embed(
            "nix",
            "js",
            "pkgs.writeJS \"name\" { } ''\n  ",
            "const x = 1;\n  function f(a) { return a + x; }\n",
            "''\n",
        ),
        embed(
            "nix",
            "pl",
            "pkgs.writePerl \"name\" { } ''\n  ",
            "my $x = 1;\n  print $x;\n",
            "''\n",
        ),
        embed(
            "nix",
            "py",
            "pkgs.writePython3 \"name\" { } ''\n  ",
            "def f(x):\n      return x + 1\n",
            "''\n",
        ),
        embed(
            "nix",
            "rs",
            "pkgs.writeRust \"name\" { } ''\n  ",
            "fn main() {\n      let x: u32 = 42;\n  }\n",
            "''\n",
        ),
        embed(
            "nix",
            "py",
            "pkgs.testers.runNixOSTest {\n  testScript = ''\n    ",
            "def f(x):\n        return x + 1\n  ",
            "'';\n}\n",
        ),
        embed(
            "nix",
            "lua",
            "{\n  type = \"lua\";\n  config = ''\n    ",
            "local x = 1\n    print(x)\n  ",
            "'';\n}\n",
        ),
        // gitcommit
        embed(
            "gitcommit",
            "diff",
            "subject\n\nbody\n\n# ------------------------ >8 ------------------------\n",
            "diff --git a/a b/a\nindex 1..2 100644\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-a\n+b\n",
            "",
        ),
        // julia
        embed(
            "jl",
            "md",
            "\"\"\"",
            "\n# Title\n",
            "\"\"\"\nfunction f(x)\n    x\nend\n",
        ),
        embed("jl", "md", "x = md\"\"\"", "\n# Title\n", "\"\"\"\n"),
        embed("jl", "sh", "run(`", "ls -la | wc -l", "`)\n"),
        // lua
        embed(
            "lua",
            "c",
            "ffi.cdef([[\n",
            "int f(int x);\nstruct a { int b; };\n",
            "]])\n",
        ),
        embed("lua", "c", "ffi.cdef\"", "int g(void);", "\"\n"),
        // rust
        embed(
            "rs",
            "json",
            "fn main() {\n    let j = json!(",
            "{\"a\": 1, \"b\": [1, 2]}",
            ");\n}\n",
        ),
        embed(
            "rs",
            "rs",
            "macro_rules! m {\n    ($a:expr) => ",
            "{ $a.len() + Foo::new(1) }",
            ";\n}\n",
        ),
    ]
}

fn language_of(extension: &str) -> anyhow::Result<Language> {
    crate::config::from_extension(extension)
        .or_else(|| {
            crate::config::AppConfig::singleton()
                .languages()
                .get(extension)
                .cloned()
        })
        .ok_or_else(|| anyhow::anyhow!("no language for extension {extension:?}"))
}

fn highlight(
    configs: &mut HighlightConfigs,
    language: Language,
    source: &str,
) -> anyhow::Result<HighlightedSpans> {
    configs.highlight(language, source, &std::sync::atomic::AtomicUsize::new(0))
}

/// The style of the innermost span covering `byte`.
fn style_at(spans: &HighlightedSpans, byte: usize) -> Option<crate::grid::StyleKey> {
    spans
        .0
        .iter()
        .rfind(|span| span.byte_range.contains(&byte))
        .map(|span| span.style_key.clone())
}

fn is_host_span(case: &InjectionCase, index: usize) -> bool {
    case.host_spans.iter().any(|span| {
        case.snippet
            .match_indices(span)
            .any(|(start, _)| (start..start + span.len()).contains(&index))
    })
}

fn check_highlight_parity(
    configs: &mut HighlightConfigs,
    case: &InjectionCase,
    host: Language,
) -> anyhow::Result<Vec<String>> {
    let source = case.source();
    let offset = case.prefix.len();
    let standalone = highlight(configs, language_of(case.embedded_extension)?, case.snippet)?;
    let embedded = highlight(configs, host, &source)?;

    let vacuous = standalone
        .0
        .is_empty()
        .then(|| "the snippet has no highlights even as a standalone file".to_string());
    let mismatches = case
        .snippet
        .char_indices()
        .filter(|(_, char)| !char.is_whitespace())
        .filter(|(index, _)| !is_host_span(case, *index))
        .filter(|(index, _)| {
            style_at(&standalone, *index)
                .is_some_and(|style| style_at(&embedded, offset + index).as_ref() != Some(&style))
        })
        .map(|(index, char)| {
            format!(
                "{char:?} at snippet byte {index}: embedded {:?}, standalone {:?}",
                style_at(&embedded, offset + index),
                style_at(&standalone, index)
            )
        });
    Ok(vacuous.into_iter().chain(mismatches).collect())
}

fn check_layers(case: &InjectionCase, host: Language) -> anyhow::Result<Vec<String>> {
    let source = case.source();
    let mut buffer = Buffer::new(host.tree_sitter_language(), &source);
    buffer.set_language(host)?;

    let is_injected = |byte: usize| -> anyhow::Result<bool> {
        let start = source[..byte].chars().count();
        let selection = Selection::default()
            .set_range(CharIndexRange::from(CharIndex(start)..CharIndex(start + 1)));
        Ok(buffer
            .syntax_tree_layer_for_selection(&selection)?
            .is_some_and(|layer| layer.is_injected))
    };
    // The last byte of the source is outside of the snippet, unless the snippet runs until the end.
    let host_probe = if case.suffix.is_empty() {
        0
    } else {
        source.len() - 1
    };
    let first_snippet_byte = case.prefix.len()
        + case
            .snippet
            .find(|char: char| !char.is_whitespace())
            .unwrap_or(0);
    Ok([
        (!is_injected(first_snippet_byte)?)
            .then(|| "a selection inside the snippet should use the injected layer".to_string()),
        is_injected(host_probe)?
            .then(|| "a selection outside the snippet should use the host layer".to_string()),
    ]
    .into_iter()
    .flatten()
    .collect())
}

fn check_case(
    configs: &mut HighlightConfigs,
    case: &InjectionCase,
    host: Language,
) -> anyhow::Result<Vec<String>> {
    let parity = check_highlight_parity(configs, case, host.clone())?;
    Ok(parity
        .into_iter()
        .chain(check_layers(case, host)?)
        .collect_vec())
}

/// Every case must fail when the host language has no injections. Otherwise the case would
/// also pass without the injection it is meant to cover.
#[test]
fn cases_are_not_vacuous() -> anyhow::Result<()> {
    let mut configs = HighlightConfigs::new();
    let vacuous = cases()
        .iter()
        .map(|case| -> anyhow::Result<_> {
            let host = language_of(case.host_extension)?.without_injections();
            Ok((case.name(), check_case(&mut configs, case, host)?))
        })
        .collect::<anyhow::Result<Vec<_>>>()?
        .into_iter()
        .filter(|(_, problems)| problems.is_empty())
        .map(|(name, _)| name)
        .collect_vec();
    assert!(
        vacuous.is_empty(),
        "these cases pass without any injection query: {vacuous:?}"
    );
    Ok(())
}

#[test]
fn supported_injections() -> anyhow::Result<()> {
    let mut configs = HighlightConfigs::new();
    let failures = cases()
        .iter()
        .map(|case| -> anyhow::Result<_> {
            let problems = check_case(&mut configs, case, language_of(case.host_extension)?)?;
            Ok((case.name(), problems))
        })
        .collect::<anyhow::Result<Vec<_>>>()?
        .into_iter()
        .filter(|(_, problems)| !problems.is_empty())
        .map(|(name, problems)| format!("{name}:\n  {}", problems.join("\n  ")))
        .collect_vec();
    assert!(
        failures.is_empty(),
        "injection cases failed:\n{}",
        failures.join("\n")
    );
    Ok(())
}

/// Every built-in injection query must compile against its grammar. Otherwise highlighting
/// silently falls back to the host language alone.
#[test]
fn injection_queries_are_valid() -> anyhow::Result<()> {
    let problems = crate::config::AppConfig::singleton()
        .languages()
        .iter()
        .filter_map(|(key, language)| {
            let query = language.injection_query()?;
            let grammar = language.tree_sitter_language()?;
            let compile_error = tree_sitter::Query::new(&grammar, query)
                .err()
                .map(|error| format!("{key}: invalid injection query: {error}"));
            let unlisted = query
                .split("injection.language \"")
                .skip(1)
                .filter_map(|rest| rest.split('"').next())
                .filter(|name| {
                    super::language_from_injection_name(name).is_some()
                        && !language.injected_language_ids().contains(name)
                })
                .unique()
                .map(|name| {
                    format!("{key}: injects {name:?} but does not list it in injected_languages")
                });
            let unknown = language
                .injected_language_ids()
                .filter(|name| super::language_from_injection_name(name).is_none())
                .map(|name| format!("{key}: injected_languages has unknown language {name:?}"));
            Some(
                compile_error
                    .into_iter()
                    .chain(unlisted)
                    .chain(unknown)
                    .collect_vec(),
            )
        })
        .flatten()
        .sorted()
        .collect_vec();
    assert!(problems.is_empty(), "{}", problems.join("\n"));
    Ok(())
}
