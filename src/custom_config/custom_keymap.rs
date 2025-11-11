//! This file is for you to define custom keymaps.
//! The keymap starts with the leader key `\`.
//! The keymap help starts with the leader key `|`.

#![allow(dead_code)]
#![allow(unused_variables)]

use crate::components::editor_keymap::{
    KeyboardMeaningLayout,
    Meaning::{self, *},
};
use event::KeyEvent;
use my_proc_macros::keys;
use shared::canonicalized_path::CanonicalizedPath;
use std::{cmp::Ordering, fmt};
use LeaderAction::*;
use Placeholder::*;

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
    // Search selected content using Google
    RunCommand(
        "chromium",
        vec![
            Str("https://www.google.com/search?q="),
            NoSpace,
            SelectionPrimary::content(),
        ],
    )
}

fn sample_toggle_process(ctx: &LeaderContext) -> LeaderAction {
    // Open the currrent file in a new window of Chromium,
    if FileCurrent::extension().resolve(ctx) == "html" {
        ToggleProcess("chromium", vec![FileCurrent::path(), Str("--new-window")])
    } else {
        DoNothing
    }
}

fn sample_macro(_ctx: &LeaderContext) -> LeaderAction {
    Macro(keys!("a c d q F e r t i g enter a ; backspace backspace a"))
}

fn sample_to_clipboard(_ctx: &LeaderContext) -> LeaderAction {
    ToClipboard(vec![Str("Hi"), FileCurrent::path()])
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
pub(crate) fn custom_keymaps() -> Vec<(
    Meaning,
    &'static str,
    Option<fn(&LeaderContext) -> LeaderAction>,
)> {
    let custom_keymaps: [(Meaning, &str, Option<fn(&LeaderContext) -> LeaderAction>); 30] = [
        (__Q__, "Sample Run Command", Some(sample_run_command)),
        (__W__, "Sample macro", Some(sample_macro)),
        (__E__, "Sample toggle process", Some(sample_toggle_process)),
        (__R__, "Sample to clipboard", Some(sample_to_clipboard)),
        (__T__, "", None),
        (__Y__, "", None),
        (__U__, "", None),
        (__I__, "", None),
        (__O__, "", None),
        (__P__, "", None),
        // Second row
        (__A__, "", None),
        (__S__, "", None),
        (__D__, "", None),
        (__F__, "", None),
        (__G__, "", None),
        (__H__, "", None),
        (__J__, "", None),
        (__K__, "", None),
        (__L__, "", None),
        (_SEMI, "", None),
        // Third row
        (__Z__, "", None),
        (__X__, "", None),
        (__C__, "", None),
        (__V__, "", None),
        (__B__, "", None),
        (__N__, "", None),
        (__M__, "", None),
        (_COMA, "", None),
        (_DOT_, "", None),
        (_SLSH, "", None),
    ];
    custom_keymaps.into_iter().collect()
}
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum LeaderAction {
    RunCommand(&'static str, Vec<Placeholder>),
    ToClipboard(Vec<Placeholder>),
    ToggleProcess(&'static str, Vec<Placeholder>),
    Macro(&'static [KeyEvent]),
    DoNothing,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Placeholder {
    Str(&'static str),
    NoSpace,
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
    FileExistsDynamic(Box<Placeholder>),
}

pub(crate) enum ResolvedValue {
    Str(String),
    Int(i64),
    Empty,
    Bool(bool),
}

pub(crate) struct Condition(Placeholder);

pub(crate) struct LeaderContext {
    pub(crate) path: Option<CanonicalizedPath>,
    /// 0-based index
    pub(crate) primary_selection_line_index: usize,
    pub(crate) primary_selection_content: String,
    pub(crate) current_working_directory: CanonicalizedPath,
}

pub(crate) struct FileCurrent;
impl FileCurrent {
    pub fn path() -> Placeholder {
        Placeholder::FileCurrent(FileCurrentKind::Path)
    }
    pub fn extension() -> Placeholder {
        Placeholder::FileCurrent(FileCurrentKind::Extension)
    }
}

pub(crate) struct SelectionPrimary;
impl SelectionPrimary {
    pub fn content() -> Placeholder {
        Placeholder::SelectionPrimary(SelectionPrimaryKind::Content)
    }
    pub fn row_index() -> Placeholder {
        Placeholder::SelectionPrimary(SelectionPrimaryKind::RowIndex)
    }
}

pub(crate) struct DirWorking;
impl DirWorking {
    pub fn path() -> Placeholder {
        Placeholder::DirCurrent(DirWorkingKind::Path)
    }
    pub fn file_exists(filename: &'static str) -> Condition {
        Condition(Placeholder::DirCurrent(DirWorkingKind::FileExists(
            filename,
        )))
    }
    pub fn file_exists_dynamic(filename: Placeholder) -> Condition {
        Condition(Placeholder::DirCurrent(DirWorkingKind::FileExistsDynamic(
            Box::new(filename),
        )))
    }
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

impl Condition {
    pub(crate) fn resolve(&self, ctx: &LeaderContext) -> bool {
        match self.0.resolve(ctx) {
            ResolvedValue::Bool(b) => b,
            _ => false,
        }
    }
}
impl From<Condition> for Placeholder {
    fn from(condition: Condition) -> Self {
        condition.0
    }
}

impl Placeholder {
    pub(crate) fn resolve(&self, ctx: &LeaderContext) -> ResolvedValue {
        match self {
            Str(str) => ResolvedValue::Str(str.to_string()),
            Placeholder::NoSpace => ResolvedValue::Empty,
            Placeholder::FileCurrent(kind) => match kind {
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
            Placeholder::SelectionPrimary(kind) => match kind {
                SelectionPrimaryKind::Content => {
                    ResolvedValue::Str(ctx.primary_selection_content.clone())
                }
                SelectionPrimaryKind::RowIndex => {
                    ResolvedValue::Int((ctx.primary_selection_line_index + 1) as i64)
                }
            },
            Placeholder::DirCurrent(kind) => match kind {
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

impl fmt::Display for Placeholder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Placeholder::Str(s) => write!(f, "Str(\"{}\")", s),
            Placeholder::NoSpace => write!(f, "NoSpace"),

            Placeholder::FileCurrent(kind) => match kind {
                FileCurrentKind::Path => write!(f, "FileCurrent::path()"),
                FileCurrentKind::Extension => write!(f, "FileCurrent::extension()"),
            },

            Placeholder::SelectionPrimary(kind) => match kind {
                SelectionPrimaryKind::Content => write!(f, "SelectionPrimary::content()"),
                SelectionPrimaryKind::RowIndex => write!(f, "SelectionPrimary::row_index()"),
            },

            Placeholder::DirCurrent(kind) => match kind {
                DirWorkingKind::Path => write!(f, "DirCurrent::path()"),
                DirWorkingKind::FileExists(filename) => {
                    write!(f, "DirCurrent::file_exists(\"{}\")", filename)
                }
                DirWorkingKind::FileExistsDynamic(placeholder) => {
                    write!(f, "DirCurrent::FileExistsDynamic({})", placeholder)
                }
            },
        }
    }
}
