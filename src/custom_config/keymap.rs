//! This file is for user to define custom keymap.
//! The keymap starts with the leader key `\`.

use event::KeyEvent;
use my_proc_macros::keys;
use shared::canonicalized_path::CanonicalizedPath;
use std::cmp::Ordering;

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
    if DirWorking::file_exists("Cargo.toml").resolve(ctx) == true {
        RunCommand("wl-copy", vec![Str("cargo run")])
    } else if FileCurrent::extension().resolve(ctx) == "py" {
        RunCommand("wl-copy", vec![Str("python"), FileCurrent::path()])
    } else if DirWorking::file_exists_dynamic(SelectionPrimary::content()).resolve(ctx) == true {
        RunCommand(
            "wl-copy",
            vec![
                Str("selection:"),
                SelectionPrimary::content(),
                Str("at: "),
                FileCurrent::path(),
            ],
        )
    } else {
        RunCommand(
            "wl-copy",
            vec![
                Str("zig?\n"),
                DirWorking::file_exists("build.zig"),
                Str("\ngo?\n"),
                DirWorking::file_exists("go.mod"),
                Str("\nhaskell?\n"),
                DirWorking::file_exists(".cabal"),
            ],
        )
    }
}

fn sample_macro(_ctx: &LeaderContext) -> LeaderAction {
    // This macro adds new cursor to the next selection that matches the current selection
    Macro(keys!("e r l esc").to_vec())
}

fn test(_ctx: &LeaderContext) -> LeaderAction {
    RunCommand(
        "kitty",
        vec![
            Str("@"),
            Str("launch"),
            Str("--hold"),
            Str("--no-response"),
            Str("--cwd"),
            DirWorking::path(),
            Str("just"),
            Str("test"),
            SelectionPrimary::content(),
        ],
    )
}

pub(crate) fn leader_keymap() -> Vec<(
    Meaning,
    &'static str,
    Arc<dyn Fn(&LeaderContext) -> LeaderAction + Send + Sync>,
)> {
    [
        (__Q__, "Sample run command", action(sample_run_command)),
        (__W__, "Sample macro", action(sample_macro)),
        (__E__, "", do_nothing()),
        (__R__, "", do_nothing()),
        (__T__, "Test", action(test)),
        (__Y__, "", do_nothing()),
        (__U__, "", do_nothing()),
        (__I__, "", do_nothing()),
        (__O__, "", do_nothing()),
        (__P__, "", do_nothing()),
        // Second row
        (__A__, "", do_nothing()),
        (__S__, "", do_nothing()),
        (__D__, "", do_nothing()),
        (__F__, "", do_nothing()),
        (__G__, "", do_nothing()),
        (__H__, "", do_nothing()),
        (__J__, "", do_nothing()),
        (__K__, "", do_nothing()),
        (__L__, "", do_nothing()),
        (_SEMI, "", do_nothing()),
        // Third row
        (__Z__, "", do_nothing()),
        (__X__, "", do_nothing()),
        (__C__, "", do_nothing()),
        (__V__, "", do_nothing()),
        (__B__, "", do_nothing()),
        (__N__, "", do_nothing()),
        (__M__, "", do_nothing()),
        (_COMA, "", do_nothing()),
        (_DOT_, "", do_nothing()),
        (_SLSH, "", do_nothing()),
    ]
    .into_iter()
    .collect()
}

fn do_nothing() -> Arc<dyn Fn(&LeaderContext) -> LeaderAction + Send + Sync> {
    Arc::new(|_: &LeaderContext| DoNothing)
}

fn action(
    f: fn(&LeaderContext) -> LeaderAction,
) -> Arc<dyn Fn(&LeaderContext) -> LeaderAction + Send + Sync> {
    Arc::new(f)
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
    RunCommand(&'static str, Vec<RunCommandPart>),
    DoNothing,
    Macro(Vec<KeyEvent>),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum RunCommandPart {
    Str(&'static str),
    FileCurrent(FileCurrentKind),
    SelectionPrimary(SelectionPrimaryKind),
    DirCurrent(DirWorkingKind),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum FileCurrentKind {
    Path,
    Extension,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum SelectionPrimaryKind {
    Content,
    /// 1-based
    RowIndex,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum DirWorkingKind {
    Path,
    FileExists(&'static str),
    FileExistsDynamic(Box<RunCommandPart>),
}

pub(crate) struct FileCurrent;
impl FileCurrent {
    pub fn path() -> RunCommandPart {
        RunCommandPart::FileCurrent(FileCurrentKind::Path)
    }
    pub fn extension() -> RunCommandPart {
        RunCommandPart::FileCurrent(FileCurrentKind::Extension)
    }
}

pub(crate) struct SelectionPrimary;
impl SelectionPrimary {
    pub fn content() -> RunCommandPart {
        RunCommandPart::SelectionPrimary(SelectionPrimaryKind::Content)
    }
    pub fn row_index() -> RunCommandPart {
        RunCommandPart::SelectionPrimary(SelectionPrimaryKind::RowIndex)
    }
}

pub(crate) struct DirWorking;
impl DirWorking {
    pub fn path() -> RunCommandPart {
        RunCommandPart::DirCurrent(DirWorkingKind::Path)
    }
    pub fn file_exists(filename: &'static str) -> RunCommandPart {
        RunCommandPart::DirCurrent(DirWorkingKind::FileExists(filename))
    }
    pub fn file_exists_dynamic(filename: RunCommandPart) -> RunCommandPart {
        RunCommandPart::DirCurrent(DirWorkingKind::FileExistsDynamic(Box::new(filename)))
    }
}

pub(crate) enum ResolvedValue {
    Str(String),
    Int(i64),
    Empty,
    Bool(bool),
}

impl ResolvedValue {
    pub(crate) fn to_string(&self) -> String {
        match self {
            ResolvedValue::Str(string) => string.clone(),
            ResolvedValue::Int(integer) => integer.to_string(),
            ResolvedValue::Bool(bool) => bool.to_string(),
            ResolvedValue::Empty => String::new(),
        }
    }
}

impl RunCommandPart {
    pub(crate) fn resolve(&self, ctx: &LeaderContext) -> ResolvedValue {
        match self {
            Str(str) => ResolvedValue::Str(str.to_string()),
            RunCommandPart::FileCurrent(kind) => match kind {
                FileCurrentKind::Path => match &ctx.path {
                    Some(path) => ResolvedValue::Str(path.display_absolute()),
                    None => ResolvedValue::Empty,
                },
                FileCurrentKind::Extension => match &ctx.path {
                    Some(path) => {
                        ResolvedValue::Str(path.extension().unwrap_or_default().to_string())
                    }
                    None => ResolvedValue::Empty,
                },
            },
            RunCommandPart::SelectionPrimary(kind) => match kind {
                SelectionPrimaryKind::Content => {
                    ResolvedValue::Str(ctx.primary_selection_content.clone())
                }
                SelectionPrimaryKind::RowIndex => {
                    ResolvedValue::Int((ctx.primary_selection_line_index + 1) as i64)
                }
            },
            RunCommandPart::DirCurrent(kind) => match kind {
                DirWorkingKind::Path => {
                    ResolvedValue::Str(ctx.current_working_directory.display_absolute())
                }
                DirWorkingKind::FileExists(file_name) => {
                    let exists = ctx
                        .current_working_directory
                        .join(*file_name)
                        .map(|path| path.exists())
                        .unwrap_or(false);
                    ResolvedValue::Bool(exists)
                }
                DirWorkingKind::FileExistsDynamic(file_name_part) => {
                    let file_name_resolved = file_name_part.resolve(ctx);
                    let file_name_str = match file_name_resolved {
                        ResolvedValue::Str(s) => s,
                        ResolvedValue::Int(i) => i.to_string(),
                        ResolvedValue::Bool(b) => b.to_string(),
                        ResolvedValue::Empty => String::new(),
                    };

                    if file_name_str.is_empty() {
                        return ResolvedValue::Bool(false);
                    }

                    let exists = ctx
                        .current_working_directory
                        .join(file_name_str.as_ref())
                        .map(|path| path.exists())
                        .unwrap_or(false);
                    ResolvedValue::Bool(exists)
                }
            },
        }
    }
}

impl PartialEq<&str> for ResolvedValue {
    fn eq(&self, other: &&str) -> bool {
        match self {
            ResolvedValue::Str(val) => val == *other,
            _ => false,
        }
    }
}

impl PartialEq<bool> for ResolvedValue {
    fn eq(&self, other: &bool) -> bool {
        match self {
            ResolvedValue::Bool(val) => val == other,
            _ => false,
        }
    }
}

impl PartialEq<i64> for ResolvedValue {
    fn eq(&self, other: &i64) -> bool {
        match self {
            ResolvedValue::Int(val) => val == other,
            _ => false,
        }
    }
}

impl PartialOrd<i64> for ResolvedValue {
    fn partial_cmp(&self, other: &i64) -> Option<Ordering> {
        match self {
            ResolvedValue::Int(val) => val.partial_cmp(other),
            _ => None,
        }
    }
}
