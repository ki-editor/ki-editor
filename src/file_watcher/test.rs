use std::time::Duration;

use lazy_regex::regex;

use crate::{
    app::Dispatch::*,
    buffer::BufferOwner,
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
            WaitForAppMessage(regex!("FileWatcherEvent")),
            Expect(CurrentComponentContentMatches(regex!("external changes"))),
        ])
    })
}
