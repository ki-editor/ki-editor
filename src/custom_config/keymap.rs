//! This file is for user to define custom keymap.
//! The keymap starts with the leader key `\`.

use event::KeyEvent;
use my_proc_macros::keys;
use shared::canonicalized_path::CanonicalizedPath;

use crate::components::editor_keymap::{
    KeyboardMeaningLayout,
    Meaning::{self, *},
};
use std::sync::Arc;

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

fn sample_run_command(ctx: &LeaderContext) -> LeaderAction {
    if resolve(ctx, PrimarySelectionContent) == "fn" {
        RunCommand(
            "wl-copy",
            &[
                Str("The current file is"),
                CurrentFilePath,
                Str("The current line is"),
                PrimarySelectionLineNumber,
            ],
        )
    } else if resolve(ctx, PrimarySelectionContent) == "else" {
        // This macro adds new cursor to the next selection that matches the current selection
        Macro(keys!("e r l esc").to_vec())
    } else {
        DoNothing
    }
}

fn sample_macro(_ctx: &LeaderContext) -> LeaderAction {
    // This macro adds new cursor to the next selection that matches the current selection
    Macro(keys!("e r l esc").to_vec())
}

fn test(_ctx: &LeaderContext) -> LeaderAction {
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

pub(crate) fn leader_keymap() -> Vec<(
    Meaning,
    &'static str,
    Arc<dyn Fn(&LeaderContext) -> LeaderAction + Send + Sync>,
)> {
    [
        (
            __Q__,
            "Sample run command",
            Arc::new(sample_run_command) as _,
        ),
        (__W__, "Sample macro", Arc::new(sample_macro) as _),
        (__E__, "Other", Arc::new(|_: &LeaderContext| DoNothing) as _),
        (__R__, "", Arc::new(|_: &LeaderContext| DoNothing) as _),
        (__T__, "Test", Arc::new(test) as _),
        (__Y__, "", Arc::new(|_: &LeaderContext| DoNothing) as _),
        (__U__, "", Arc::new(|_: &LeaderContext| DoNothing) as _),
        (__I__, "", Arc::new(|_: &LeaderContext| DoNothing) as _),
        (__O__, "", Arc::new(|_: &LeaderContext| DoNothing) as _),
        (__P__, "", Arc::new(|_: &LeaderContext| DoNothing) as _),
        // Second row
        (__A__, "", Arc::new(|_: &LeaderContext| DoNothing) as _),
        (__S__, "", Arc::new(|_: &LeaderContext| DoNothing) as _),
        (__D__, "", Arc::new(|_: &LeaderContext| DoNothing) as _),
        (__F__, "", Arc::new(|_: &LeaderContext| DoNothing) as _),
        (__G__, "", Arc::new(|_: &LeaderContext| DoNothing) as _),
        (__H__, "", Arc::new(|_: &LeaderContext| DoNothing) as _),
        (__J__, "", Arc::new(|_: &LeaderContext| DoNothing) as _),
        (__K__, "", Arc::new(|_: &LeaderContext| DoNothing) as _),
        (__L__, "", Arc::new(|_: &LeaderContext| DoNothing) as _),
        (_SEMI, "", Arc::new(|_: &LeaderContext| DoNothing) as _),
        // Third row
        (__Z__, "", Arc::new(|_: &LeaderContext| DoNothing) as _),
        (__X__, "", Arc::new(|_: &LeaderContext| DoNothing) as _),
        (__C__, "", Arc::new(|_: &LeaderContext| DoNothing) as _),
        (__V__, "", Arc::new(|_: &LeaderContext| DoNothing) as _),
        (__B__, "", Arc::new(|_: &LeaderContext| DoNothing) as _),
        (__N__, "", Arc::new(|_: &LeaderContext| DoNothing) as _),
        (__M__, "", Arc::new(|_: &LeaderContext| DoNothing) as _),
        (_COMA, "", Arc::new(|_: &LeaderContext| DoNothing) as _),
        (_DOT_, "", Arc::new(|_: &LeaderContext| DoNothing) as _),
        (_SLSH, "", Arc::new(|_: &LeaderContext| DoNothing) as _),
    ]
    .into_iter()
    .collect()
}

pub(crate) fn resolve(ctx: &LeaderContext, part: RunCommandPart) -> String {
    match part {
        Str(str) => str.to_string(),
        CurrentFilePath => ctx
            .path
            .as_ref()
            .map(|path| path.display_absolute())
            .unwrap_or_default(),
        PrimarySelectionLineNumber => (ctx.primary_selection_line_index + 1).to_string(),
        PrimarySelectionContent => ctx.primary_selection_content.clone(),
        CurrentWorkingDirectory => ctx.current_working_directory.display_absolute(),
    }
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
