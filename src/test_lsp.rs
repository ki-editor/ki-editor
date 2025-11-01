use std::time::Duration;

use lazy_regex::regex;
use my_proc_macros::{key, keys};

use crate::{
    app::{Dimension, Dispatch::*},
    buffer::BufferOwner,
    components::editor::{DispatchEditor::*, IfCurrentNotFound},
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
