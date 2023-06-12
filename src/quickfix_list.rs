use std::{ops::Range, path::PathBuf};

use crate::position::Position;

pub struct QuickfixLists {
    lists: Vec<QuickfixList>,
}

impl QuickfixLists {
    pub fn new() -> QuickfixLists {
        QuickfixLists { lists: vec![] }
    }
}

#[derive(Clone)]
pub struct QuickfixList {
    items: Vec<QuickfixListItem>,
}

#[derive(Clone)]
pub struct QuickfixListItem {
    pub path: PathBuf,
    pub range: Range<Position>,
    pub info: Option<String>,
}

#[derive(Debug, Clone)]
pub enum QuickfixListType {
    LspDiagnosticError,
}
