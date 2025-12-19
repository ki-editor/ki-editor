use std::time::Duration;

use lazy_regex::regex;
use my_proc_macros::{key, keys};

use crate::{
    app::{Dimension, Dispatch::*, Scope},
    buffer::BufferOwner,
    components::editor::{Direction, DispatchEditor::*, IfCurrentNotFound},
    grid::StyleKey,
    selection::SelectionMode,
    test_app::{execute_test_custom, ExpectKind::*, RunTestOptions, Step::*},
};

#[test]
fn rust_lsp_auto_import_from_completion_item() -> Result<(), anyhow::Error> {
    let options = RunTestOptions {
        enable_lsp: true,
        enable_syntax_highlighting: false,
        enable_file_watcher: false,
    };
    execute_test_custom(options, |s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            // Initially, expect there's no `std::path::Path` import.
            Expect(CurrentComponentContent(
                r#"mod foo;

fn main() {
    foo::foo();
    println!("Hello, world!");
}
"#,
            )),
            WaitForAppMessage(lazy_regex::regex!("LspNotification.*Initialized")),
            WaitForAppMessage(lazy_regex::regex!("LspNotification.*PublishDiagnostics")),
            WaitForAppMessage(lazy_regex::regex!("LspNotification.*PublishDiagnostics")),
            Editor(MatchLiteral("println".to_string())),
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                SelectionMode::Line,
            )),
            App(TerminalDimensionChanged(Dimension {
                height: 20,
                width: 100,
            })),
            // Attempt to import `std::path::Path` from the auto-completion of "path"
            Editor(Open),
            App(HandleKeyEvents(keys!("P a t h").to_vec())),
            WaitForAppMessage(regex!("LspNotification.*Completion")),
            Expect(AppGridContains("Path(use std::path::Path)")),
            WaitForAppMessage(regex!("LspNotification.*CompletionItemResolve")),
            App(HandleKeyEvent(key!("tab"))),
            // Use the completion item `Path`, expect the import `std::path::Path` is added automatically.
            Expect(CurrentComponentContent(
                r#"use std::path::Path;

mod foo;

fn main() {
    foo::foo();
    println!("Hello, world!");
    Path
}
"#,
            )),
        ])
    })
}

#[test]
fn typescript_lsp_workspace_symbols() -> Result<(), anyhow::Error> {
    let options = RunTestOptions {
        enable_lsp: true,
        enable_syntax_highlighting: false,
        enable_file_watcher: false,
    };
    execute_test_custom(options, |s| {
        Box::new([
            App(AddPath(s.new_path("hello.ts").display().to_string())),
            App(HandleKeyEvent(key!("enter"))),
            Editor(SetContent("export function hello() {}".to_string())),
            WaitForAppMessage(lazy_regex::regex!("LspNotification.*Initialized")),
            App(OpenWorkspaceSymbolsPrompt),
            App(HandleKeyEvents(keys!("h e").to_vec())),
            Expect(AppMessageIsReceived {
                matches: regex!("LspNotification.*WorkspaceSymbols"),
                timeout: Duration::from_secs(5),
            }),
        ])
    })
}

#[test]
fn typescript_lsp_references() -> Result<(), anyhow::Error> {
    let options = RunTestOptions {
        enable_lsp: true,
        enable_syntax_highlighting: false,
        enable_file_watcher: false,
    };
    execute_test_custom(options, |s| {
        Box::new([
            App(AddPath(s.new_path("foo.ts").display().to_string())),
            App(HandleKeyEvent(key!("enter"))),
            Editor(SetContent(
                // The `hello` method is purposefully indented
                // so that we can test that the quickfix list lines are not trimmed
                // because trimming will cause match highlighting issues
                "
class Hi {
    hello() {}
}
"
                .trim()
                .to_string(),
            )),
            WaitForAppMessage(lazy_regex::regex!("LspNotification.*Initialized")),
            Editor(Save),
            Editor(MatchLiteral("hello".to_string())),
            Expect(CurrentSelectedTexts(&["hello"])),
            Editor(EnterInsertMode(Direction::End)),
            App(RequestReferences {
                include_declaration: true,
                scope: Scope::Global,
            }),
            Expect(AppMessageIsReceived {
                matches: regex!("LspNotification.*References"),
                timeout: Duration::from_secs(5),
            }),
            App(TerminalDimensionChanged(Dimension {
                height: 100,
                width: 100,
            })),
            // Move to the quickfix list window
            App(OtherWindow),
            Expect(CurrentComponentContent(
                "
foo.ts
    2:5      hello() {}"
                    .trim(),
            )),
            // Expect `hello` is styled as search matches
            Expect(RangeStyleKey(
                "hello",
                Some(StyleKey::UiIncrementalSearchMatch),
            )),
        ])
    })
}
