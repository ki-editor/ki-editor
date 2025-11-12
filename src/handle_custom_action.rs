#![allow(dead_code)]
#![allow(unused_variables)]

use event::KeyEvent;
use shared::canonicalized_path::CanonicalizedPath;
use std::{cmp::Ordering, fmt};
use Placeholder::*;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum CustomAction {
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
    Extension,
    PathRoot,
    PathLocal,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum SelectionPrimaryKind {
    Content,
    /// 1-based
    RowIndex,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum DirWorkingKind {
    PathRoot,
    FileExists(&'static str),
    FileExistsDynamic(Box<Placeholder>),
}

pub(crate) enum ResolvedValue {
    Str(String),
    Int(i64),
    Empty,
    Bool(bool),
}

pub(crate) struct Condition(pub(crate) Placeholder);

pub(crate) struct CustomContext {
    pub(crate) path: Option<CanonicalizedPath>,
    /// 0-based index
    pub(crate) primary_selection_line_index: usize,
    pub(crate) primary_selection_content: String,
    pub(crate) current_working_directory: CanonicalizedPath,
}

pub(crate) struct FileCurrent;
impl FileCurrent {
    pub fn extension() -> Placeholder {
        Placeholder::FileCurrent(FileCurrentKind::Extension)
    }
    pub fn path_root() -> Placeholder {
        Placeholder::FileCurrent(FileCurrentKind::PathRoot)
    }
    pub fn path_local() -> Placeholder {
        Placeholder::FileCurrent(FileCurrentKind::PathLocal)
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
    pub fn path_root() -> Placeholder {
        Placeholder::DirCurrent(DirWorkingKind::PathRoot)
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
    pub(crate) fn resolve(&self, ctx: &CustomContext) -> bool {
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
    pub(crate) fn resolve(&self, ctx: &CustomContext) -> ResolvedValue {
        match self {
            Str(str) => ResolvedValue::Str(str.to_string()),
            Placeholder::NoSpace => ResolvedValue::Empty,
            Placeholder::FileCurrent(kind) => match kind {
                FileCurrentKind::Extension => match &ctx.path {
                    Some(path) => {
                        ResolvedValue::Str(path.extension().unwrap_or_default().to_string())
                    }
                    None => ResolvedValue::Empty,
                },
                FileCurrentKind::PathRoot => match &ctx.path {
                    Some(path) => ResolvedValue::Str(path.display_absolute()),
                    None => ResolvedValue::Empty,
                },
                FileCurrentKind::PathLocal => match &ctx.path {
                    Some(path) => {
                        let relative_path = path
                            .as_ref()
                            .strip_prefix(&ctx.current_working_directory)
                            .unwrap_or_else(|_| path.as_ref())
                            .display()
                            .to_string();
                        ResolvedValue::Str(relative_path)
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
                DirWorkingKind::PathRoot => {
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
                FileCurrentKind::PathRoot => write!(f, "FileCurrent::path()"),
                FileCurrentKind::Extension => write!(f, "FileCurrent::extension()"),
                FileCurrentKind::PathLocal => write!(f, "FileCurrent::path_local()"),
            },

            Placeholder::SelectionPrimary(kind) => match kind {
                SelectionPrimaryKind::Content => write!(f, "SelectionPrimary::content()"),
                SelectionPrimaryKind::RowIndex => write!(f, "SelectionPrimary::row_index()"),
            },

            Placeholder::DirCurrent(kind) => match kind {
                DirWorkingKind::PathRoot => write!(f, "DirCurrent::path()"),
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
