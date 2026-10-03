use lazy_regex::regex;
use my_proc_macros::key;

use crate::{
    app::{Dimension, Dispatch::*},
    components::editor::{Direction, DispatchEditor::*},
    grid::{IndexedHighlightGroup, StyleKey},
    test_app::{execute_test_custom, ExpectKind::*, RunTestOptions, Step::*},
};

#[test]
fn highlights_configured_injections() -> anyhow::Result<()> {
    let source = r##"fn main() { let data = r#"{"answer": 42}"#; }"##;
    let (language, errors) = shared::language::Language::extract_lenient(
        &serde_json::json!({
            "injection_query": "((raw_string_literal (string_content) @injection.content) (#set! injection.language \"json\"))",
            "injected_languages": ["json"]
        }),
        &crate::config::from_extension("rs").unwrap(),
    );
    assert!(errors.is_empty(), "{errors:?}");
    let spans = super::HighlightConfigs::new().highlight(
        language,
        source,
        &std::sync::atomic::AtomicUsize::new(0),
    )?;
    let start = source.find("42").unwrap();
    let number = StyleKey::Syntax(IndexedHighlightGroup::from_str("number").unwrap());
    assert!(spans
        .0
        .iter()
        .any(|span| span.style_key == number && span.byte_range == (start..start + 2)));
    Ok(())
}

#[test]
fn highlights_code_snippets_in_markdown() -> anyhow::Result<()> {
    let source = "# Title\n\n```json\n{\"answer\": 42}\n```\n";
    // Markdown injects fenced code blocks by default
    let language = crate::config::from_extension("md").unwrap();
    let spans = super::HighlightConfigs::new().highlight(
        language,
        source,
        &std::sync::atomic::AtomicUsize::new(0),
    )?;
    let start = source.find("42").unwrap();
    let number = StyleKey::Syntax(IndexedHighlightGroup::from_str("number").unwrap());
    assert!(spans
        .0
        .iter()
        .any(|span| span.style_key == number && span.byte_range == (start..start + 2)));
    Ok(())
}

#[test]
fn syntax_highlight_json() -> anyhow::Result<()> {
    let options = RunTestOptions {
        enable_lsp: false,
        enable_syntax_highlighting: true,
        enable_file_watcher: false,
    };
    execute_test_custom(options, |s| {
        Box::new([
            App(AddPath(s.new_path("hello.json").display().to_string())),
            Expect(CurrentComponentTitle("File Explorer".to_string())),
            App(HandleKeyEvent(key!("enter"))),
            ExpectLater(Box::new(move || {
                CurrentComponentPath(Some(s.new_path("hello.json").try_into().unwrap()))
            })),
            Editor(SetContent(r#"{"x": 19}"#.to_string())),
            // Insert something to trigger syntax highlight request
            Editor(EnterInsertMode(Direction::End)),
            App(HandleKeyEvent(key!("space"))),
            WaitForAppMessage(regex!("SyntaxHighlightResponse")),
            App(TerminalDimensionChanged(Dimension {
                height: 20,
                width: 50,
            })),
            // Expect "x" is highlighted as "string"
            Expect(RangeStyleKey(
                "x",
                Some(StyleKey::Syntax(
                    IndexedHighlightGroup::from_str("string").unwrap(),
                )),
            )),
            // Expect 19 is highlighted as "number"
            Expect(RangeStyleKey(
                "19",
                Some(StyleKey::Syntax(
                    IndexedHighlightGroup::from_str("number").unwrap(),
                )),
            )),
        ])
    })
}

#[test]
fn markdown_code_snippet_highlight_matches_standalone_file() -> anyhow::Result<()> {
    let code = "fn main() {\n    let x: u32 = 42;\n    println!(\"hello {x}\");\n}\n";
    let markdown = format!("# Title\n\n```rust\n{code}```\n");
    let highlight = |extension: &str, source: &str| {
        super::HighlightConfigs::new().highlight(
            crate::config::from_extension(extension).unwrap(),
            source,
            &std::sync::atomic::AtomicUsize::new(0),
        )
    };
    let offset = markdown.find(code).unwrap();
    let standalone = highlight("rs", code)?;
    let embedded = highlight("md", &markdown)?;
    // The style at each byte of the snippet should be the same as when
    // the snippet is highlighted as a standalone Rust file
    let style_at = |spans: &super::HighlightedSpans, byte: usize| {
        spans
            .0
            .iter()
            .rfind(|span| span.byte_range.contains(&byte))
            .map(|span| span.style_key.clone())
    };
    code.char_indices()
        .filter(|(_, c)| !c.is_whitespace())
        .for_each(|(index, c)| {
            assert_eq!(
                style_at(&embedded, offset + index),
                style_at(&standalone, index),
                "unexpected style for {c:?} at byte {index} of the snippet"
            )
        });
    Ok(())
}
