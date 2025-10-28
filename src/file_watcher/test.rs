use std::time::Duration;

use lazy_regex::regex;

use DispatchEditor::*;

use crate::{
    app::Dispatch::*,
    buffer::BufferOwner,
    components::editor::DispatchEditor::{self},
    test_app::{execute_test_custom, ExpectKind::*, RunTestOptions, Step::*},
};

#[test]
fn file_modified_externally() -> Result<(), anyhow::Error> {
    let options = RunTestOptions {
        enable_lsp: false,
        enable_syntax_highlighting: false,
        enable_file_watcher: true,
    };
    execute_test_custom(options, |s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            // Modify file externally
            WaitForDuration(Duration::from_secs(1)),
            Shell(
                "bash",
                [
                    "-c".to_string(),
                    format!(
                        "echo external changes >> {}",
                        s.main_rs().display_absolute(),
                    ),
                ]
                .to_vec(),
            ),
            WaitForDuration(Duration::from_secs(1)),
            WaitForAppMessage(regex!("FileWatcherEvent.*ContentModified")),
            Expect(CurrentComponentContentMatches(regex!("external changes"))),
        ])
    })
}

#[test]
fn expect_no_file_notifications_for_unopened_files() -> Result<(), anyhow::Error> {
    let options = RunTestOptions {
        enable_lsp: false,
        enable_syntax_highlighting: false,
        enable_file_watcher: true,
    };
    execute_test_custom(options, |s| {
        Box::new([
            // Open main.rs
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            // Modify foo.rs externally
            WaitForDuration(Duration::from_secs(1)),
            Shell(
                "bash",
                [
                    "-c".to_string(),
                    format!("echo external changes >> {}", s.foo_rs().display_absolute(),),
                ]
                .to_vec(),
            ),
            WaitForDuration(Duration::from_secs(1)),
            Expect(AppMessageNotReceived {
                matches: regex!("FileWatcherEvent.*ContentModified"),
                timeout: Duration::from_secs(5),
            }),
        ])
    })
}

#[test]
fn expect_no_file_notifications_for_closed_files() -> Result<(), anyhow::Error> {
    let options = RunTestOptions {
        enable_lsp: false,
        enable_syntax_highlighting: false,
        enable_file_watcher: true,
    };
    execute_test_custom(options, |s| {
        Box::new([
            // Open main.rs
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            // Close main.rs
            App(CloseCurrentWindow),
            // Modify main.rs externally
            WaitForDuration(Duration::from_secs(1)),
            Shell(
                "bash",
                [
                    "-c".to_string(),
                    format!(
                        "echo external changes >> {}",
                        s.main_rs().display_absolute(),
                    ),
                ]
                .to_vec(),
            ),
            WaitForDuration(Duration::from_secs(1)),
            Expect(AppMessageNotReceived {
                matches: regex!("FileWatcherEvent.*ContentModified"),
                timeout: Duration::from_secs(5),
            }),
        ])
    })
}

#[test]
fn path_removal_should_refresh_explorer() -> Result<(), anyhow::Error> {
    let options = RunTestOptions {
        enable_lsp: false,
        enable_syntax_highlighting: false,
        enable_file_watcher: true,
    };
    execute_test_custom(options, |s| {
        Box::new([
            App(RevealInExplorer(s.main_rs())),
            Expect(CurrentComponentContentMatches(regex!("main.rs"))),
            WaitForDuration(Duration::from_secs(1)),
            // Add a new file named new_file
            Shell("rm", [s.main_rs().display_absolute()].to_vec()),
            WaitForAppMessage(regex!("FileWatcherEvent.*PathRemoved")),
            Expect(Not(Box::new(CurrentComponentContentMatches(regex!(
                "main.rs"
            ))))),
        ])
    })
}

#[test]
fn path_rename_should_refresh_explorer() -> Result<(), anyhow::Error> {
    let options = RunTestOptions {
        enable_lsp: false,
        enable_syntax_highlighting: false,
        enable_file_watcher: true,
    };
    execute_test_custom(options, |s| {
        Box::new([
            App(RevealInExplorer(s.main_rs())),
            Expect(Not(Box::new(CurrentComponentContentMatches(regex!(
                "renamed.rs"
            ))))),
            WaitForDuration(Duration::from_secs(2)),
            // Add a new file named new_file
            Shell(
                "mv",
                [
                    s.main_rs().display_absolute(),
                    s.new_path("renamed.rs").display().to_string(),
                ]
                .to_vec(),
            ),
            WaitForDuration(Duration::from_secs(2)),
            WaitForAppMessage(regex!("FileWatcherEvent.*PathRenamed")),
            Expect(CurrentComponentContentMatches(regex!("renamed.rs"))),
        ])
    })
}

#[test]
fn path_modified_under_a_non_expanded_folder_should_not_refresh_explorer(
) -> Result<(), anyhow::Error> {
    let options = RunTestOptions {
        enable_lsp: false,
        enable_syntax_highlighting: false,
        enable_file_watcher: true,
    };
    execute_test_custom(options, |s| {
        Box::new([
            App(RevealInExplorer(s.gitignore())),
            // Expect "src" folder is not expanded,
            Expect(CurrentComponentContent(
                " - 📁  .git/ :
 - 🙈  .gitignore
 - 🔒  Cargo.lock
 - 📄  Cargo.toml
 - 📁  src/ :",
            )),
            // Rename "src/main.rs" to "src/renamed.rs"
            Shell(
                "mv",
                [
                    s.main_rs().display_absolute(),
                    s.new_path("renamed.rs").display().to_string(),
                ]
                .to_vec(),
            ),
            Expect(AppMessageNotReceived {
                matches: regex!("FileWatcherEvent.*PathRenamed"),
                timeout: Duration::from_secs(5),
            }),
        ])
    })
}

#[test]
fn path_modified_under_current_working_directory_should_refresh_explorer(
) -> Result<(), anyhow::Error> {
    let options = RunTestOptions {
        enable_lsp: false,
        enable_syntax_highlighting: false,
        enable_file_watcher: true,
    };
    execute_test_custom(options, |s| {
        Box::new([
            App(RevealInExplorer(s.main_rs())),
            Expect(Not(Box::new(CurrentComponentContentMatches(regex!(
                "renamed"
            ))))),
            WaitForDuration(Duration::from_secs(1)),
            Shell(
                "mv",
                [
                    s.gitignore().display_absolute(),
                    s.new_path("renamed").display().to_string(),
                ]
                .to_vec(),
            ),
            WaitForAppMessage(regex!("FileWatcherEvent.*PathRenamed")),
            Expect(CurrentComponentContentMatches(regex!("renamed"))),
        ])
    })
}

#[test]
fn saving_a_file_should_not_refreshes_the_buffer_due_to_incoming_file_modified_notification(
) -> Result<(), anyhow::Error> {
    let options = RunTestOptions {
        enable_lsp: false,
        enable_syntax_highlighting: false,
        enable_file_watcher: true,
    };
    execute_test_custom(options, |s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(MatchLiteral("mod".to_string())),
            Editor(Delete),
            Editor(Save),
            WaitForDuration(Duration::from_secs(2)),
            WaitForAppMessage(regex!("FileWatcherEvent.*ContentModified")),
            Editor(Undo),
            Expect(CurrentSelectedTexts(&["mod"])),
        ])
    })
}
