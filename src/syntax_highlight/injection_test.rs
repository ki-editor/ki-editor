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

use super::{HighlightConfigs, HighlightedSpans};

struct InjectionCase {
    host_extension: &'static str,
    embedded_extension: &'static str,
    /// Source text before the snippet.
    prefix: &'static str,
    snippet: &'static str,
    /// Source text after the snippet.
    suffix: &'static str,
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
        },
        InjectionCase {
            host_extension: "md",
            embedded_extension: "yaml",
            prefix: "---\n",
            snippet: "title: Hello\ntags:\n  - a\n",
            suffix: "---\n\n# Body\n",
        },
    ]
    .into()
}

fn highlight(extension: &str, source: &str) -> anyhow::Result<HighlightedSpans> {
    HighlightConfigs::new().highlight(
        crate::config::from_extension(extension)
            .ok_or_else(|| anyhow::anyhow!("no language for extension {extension:?}"))?,
        source,
        &std::sync::atomic::AtomicUsize::new(0),
    )
}

/// The style of the innermost span covering `byte`.
fn style_at(spans: &HighlightedSpans, byte: usize) -> Option<crate::grid::StyleKey> {
    spans
        .0
        .iter()
        .rfind(|span| span.byte_range.contains(&byte))
        .map(|span| span.style_key.clone())
}

fn check_highlight_parity(case: &InjectionCase) -> anyhow::Result<Vec<String>> {
    let source = case.source();
    let offset = case.prefix.len();
    let standalone = highlight(case.embedded_extension, case.snippet)?;
    let embedded = highlight(case.host_extension, &source)?;

    let vacuous = standalone
        .0
        .is_empty()
        .then(|| "the snippet has no highlights even as a standalone file".to_string());
    let mismatches = case
        .snippet
        .char_indices()
        .filter(|(_, char)| !char.is_whitespace())
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

fn check_layers(case: &InjectionCase) -> anyhow::Result<Vec<String>> {
    let source = case.source();
    let language = crate::config::from_extension(case.host_extension)
        .ok_or_else(|| anyhow::anyhow!("no host language for {:?}", case.host_extension))?;
    let mut buffer = Buffer::new(language.tree_sitter_language(), &source);
    buffer.set_language(language)?;

    let is_injected = |byte: usize| -> anyhow::Result<bool> {
        let start = source[..byte].chars().count();
        let selection = Selection::default()
            .set_range(CharIndexRange::from(CharIndex(start)..CharIndex(start + 1)));
        Ok(buffer
            .syntax_tree_layer_for_selection(&selection)?
            .is_some_and(|layer| layer.is_injected))
    };
    let first_snippet_byte = case.prefix.len()
        + case
            .snippet
            .find(|char: char| !char.is_whitespace())
            .unwrap_or(0);
    Ok([
        (!is_injected(first_snippet_byte)?)
            .then(|| "a selection inside the snippet should use the injected layer".to_string()),
        is_injected(source.len() - 1)?
            .then(|| "a selection outside the snippet should use the host layer".to_string()),
    ]
    .into_iter()
    .flatten()
    .collect())
}

#[test]
fn supported_injections() -> anyhow::Result<()> {
    let failures = cases()
        .iter()
        .map(|case| -> anyhow::Result<_> {
            let problems = check_highlight_parity(case)?
                .into_iter()
                .chain(check_layers(case)?)
                .collect_vec();
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
