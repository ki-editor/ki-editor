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
    let (language, errors) = shared::language::Language::extract_lenient(
        &serde_json::json!({
            "injection_query": tree_sitter_md::INJECTION_QUERY_BLOCK,
            "injected_languages": ["json"]
        }),
        &crate::config::from_extension("md").unwrap(),
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
