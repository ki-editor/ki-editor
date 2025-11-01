//! This file is for user to define custom keymap.
//! The keymap starts with the leader key `\`.

use event::KeyEvent;
use my_proc_macros::keys;
use shared::canonicalized_path::CanonicalizedPath;

use crate::components::editor_keymap::{
    KeyboardMeaningLayout,
    Meaning::{self, *},
};

use LeaderAction::*;
use RunCommandPart::*;

pub(crate) const KEYMAP_LEADER: KeyboardMeaningLayout = [
    [
        __Q__, __W__, __E__, __R__, __T__, /****/ __Y__, __U__, __I__, __O__, __P__,
    ],
    [
        __A__, __S__, __D__, __F__, __G__, /****/ __H__, __J__, __K__, __L__, _SEMI,
    ],
    [
        __Z__, __X__, __C__, __V__, __B__, /****/ __N__, __M__, _COMA, _DOT_, _SLSH,
    ],
];

fn sample_run_command() -> LeaderAction {
    RunCommand(
        "echo",
        &[
            Str("The current file is"),
            CurrentFilePath,
            Str("The current line is"),
            PrimarySelectionLineNumber,
        ],
    )
}

fn sample_macro() -> LeaderAction {
    // This macro adds new cursor to the next selection that matches the current selection
    Macro(keys!("e r l esc").to_vec())
}

fn test() -> LeaderAction {
    RunCommand(
        "kitty",
        &[
            Str("@"),
            Str("launch"),
            Str("--hold"),
            Str("--no-response"),
            Str("--cwd"),
            CurrentWorkingDirectory,
            Str("just"),
            Str("test"),
            PrimarySelectionContent,
        ],
    )
}

pub(crate) fn leader_keymap() -> Vec<(Meaning, &'static str, LeaderAction)> {
    [
        (__Q__, "Sample run command", sample_run_command()),
        (__W__, "Sample macro", sample_macro()),
        (__E__, "", DoNothing),
        (__R__, "", DoNothing),
        (__T__, "Test", test()),
        (__Y__, "", DoNothing),
        (__U__, "", DoNothing),
        (__I__, "", DoNothing),
        (__O__, "", DoNothing),
        (__P__, "", DoNothing),
        // Second row
        (__A__, "", DoNothing),
        (__S__, "", DoNothing),
        (__D__, "", DoNothing),
        (__F__, "", DoNothing),
        (__G__, "", DoNothing),
        (__H__, "", DoNothing),
        (__J__, "", DoNothing),
        (__K__, "", DoNothing),
        (__L__, "", DoNothing),
        (_SEMI, "", DoNothing),
        // Third row
        (__Z__, "", DoNothing),
        (__X__, "", DoNothing),
        (__C__, "", DoNothing),
        (__V__, "", DoNothing),
        (__B__, "", DoNothing),
        (__N__, "", DoNothing),
        (__M__, "", DoNothing),
        (_COMA, "", DoNothing),
        (_DOT_, "", DoNothing),
        (_SLSH, "", DoNothing),
    ]
    .into_iter()
    .collect()
}

pub(crate) struct LeaderContext {
    pub(crate) path: Option<CanonicalizedPath>,
    /// 0-based index
    pub(crate) primary_selection_line_index: usize,
    pub(crate) primary_selection_content: String,
    pub(crate) current_working_directory: CanonicalizedPath,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum LeaderAction {
    RunCommand(&'static str, &'static [RunCommandPart]),
    DoNothing,
    Macro(Vec<KeyEvent>),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum RunCommandPart {
    Str(&'static str),
    CurrentFilePath,
    /// 1-based
    PrimarySelectionLineNumber,
    PrimarySelectionContent,
    CurrentWorkingDirectory,
}
impl RunCommandPart {
    pub(crate) fn to_string(&self, leader_context: &LeaderContext) -> String {
        match self {
            Str(str) => str.to_string(),
            CurrentFilePath => leader_context
                .path
                .as_ref()
                .map(|path| path.display_absolute())
                .unwrap_or_default(),
            PrimarySelectionLineNumber => {
                (leader_context.primary_selection_line_index + 1).to_string()
            }
            PrimarySelectionContent => leader_context.primary_selection_content.to_string(),
            CurrentWorkingDirectory => leader_context.current_working_directory.display_absolute(),
        }
    }
}
