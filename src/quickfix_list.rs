use std::{cell::RefCell, ops::Range, rc::Rc};

use itertools::Itertools;
use lsp_types::DiagnosticSeverity;

use crate::{
    app::Dispatches,
    buffer::Buffer,
    components::{
        dropdown::{Dropdown, DropdownConfig, DropdownItem},
        editor::Movement,
        suggestive_editor::Info,
    },
    position::Position,
};
use shared::canonicalized_path::CanonicalizedPath;

impl QuickfixListItem {
    fn into_dropdown_item(self: QuickfixListItem, buffers: &[Rc<RefCell<Buffer>>]) -> DropdownItem {
        let location = self.location();
        let Position { line, column } = location.range.start;
        DropdownItem::new({
            let content = location
                .read_from_buffers(buffers)
                .unwrap_or_else(|| "[Failed to read file]".to_string())
                .trim_matches(|c: char| c.is_whitespace())
                .to_string();
            format!("{}:{}  {}", line + 1, column + 1, content)
        })
        .set_info(self.info.clone())
        .set_group({
            let path = self.location().path.clone();
            Some(
                path.display_relative()
                    .unwrap_or_else(|_| path.display_absolute()),
            )
        })
        .set_dispatches(Dispatches::one(crate::app::Dispatch::GotoLocation(
            self.location().to_owned(),
        )))
        .set_rank(Some(Box::new([line, column])))
    }

    pub(crate) fn set_location_range(self, range: Range<Position>) -> QuickfixListItem {
        let QuickfixListItem {
            location: Location { path, .. },
            info,
        } = self;
        QuickfixListItem {
            info,
            location: Location { path, range },
        }
    }
}

pub struct QuickfixList {
    dropdown: Dropdown,
    #[cfg(test)]
    items: Vec<QuickfixListItem>,
}

impl QuickfixList {
    pub(crate) fn new(
        items: Vec<QuickfixListItem>,
        buffers: Vec<Rc<RefCell<Buffer>>>,
    ) -> QuickfixList {
        let mut dropdown = Dropdown::new(DropdownConfig {
            title: "Quickfix".to_string(),
        });
        // Merge items of same locations
        let items = items
            .into_iter()
            // Sort the items by location
            .sorted_by_key(|item| item.location.clone())
            .group_by(|item| item.location.clone())
            .into_iter()
            .map(|(location, items)| QuickfixListItem {
                location,
                info: items
                    .into_iter()
                    .flat_map(|item| item.info)
                    .reduce(Info::join),
            })
            .collect_vec();
        dropdown.set_items(
            items
                .iter()
                .map(|item| item.to_owned().into_dropdown_item(&buffers))
                .collect(),
        );

        QuickfixList {
            #[cfg(test)]
            items,
            dropdown,
        }
    }

    #[cfg(test)]
    pub(crate) fn items(&self) -> Vec<QuickfixListItem> {
        self.items.clone()
    }

    pub(crate) fn render(&self) -> crate::components::dropdown::DropdownRender {
        self.dropdown.render()
    }

    /// Returns the current item index after `movement` is applied
    pub(crate) fn get_item(&mut self, movement: Movement) -> Option<(usize, Dispatches)> {
        self.dropdown.apply_movement(movement);
        Some((
            self.dropdown.current_item_index(),
            self.dropdown.current_item()?.dispatches,
        ))
    }

    pub(crate) fn set_current_item_index(mut self, item_index: usize) -> Self {
        self.dropdown.set_current_item_index(item_index);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickfixListItem {
    location: Location,
    info: Option<Info>,
}

impl PartialOrd for QuickfixListItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for QuickfixListItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.location.cmp(&other.location)
    }
}

impl From<Location> for QuickfixListItem {
    fn from(value: Location) -> Self {
        QuickfixListItem {
            location: value,
            info: None,
        }
    }
}

impl QuickfixListItem {
    pub(crate) fn new(location: Location, info: Option<Info>) -> QuickfixListItem {
        QuickfixListItem { location, info }
    }

    pub(crate) fn location(&self) -> &Location {
        &self.location
    }

    pub(crate) fn info(&self) -> &Option<Info> {
        &self.info
    }

    #[cfg(test)]
    pub(crate) fn set_info(self, info: Option<Info>) -> Self {
        Self { info, ..self }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Location {
    pub path: CanonicalizedPath,
    pub range: Range<Position>,
}

impl Location {
    fn read_from_buffers(&self, buffers: &[Rc<RefCell<Buffer>>]) -> Option<String> {
        buffers
            .iter()
            .find(|buffer| {
                if let Some(path) = buffer.borrow().path() {
                    path == self.path
                } else {
                    false
                }
            })
            .and_then(|buffer| {
                Some(
                    buffer
                        .borrow()
                        .get_line_by_line_index(self.range.start.line)?
                        .to_string(),
                )
            })
    }
}

impl TryFrom<lsp_types::Location> for Location {
    type Error = anyhow::Error;

    fn try_from(value: lsp_types::Location) -> Result<Self, Self::Error> {
        Ok(Location {
            path: value
                .uri
                .to_file_path()
                .map_err(|_| anyhow::anyhow!("Failed to convert uri to file path"))?
                .try_into()?,
            range: value.range.start.into()..value.range.end.into(),
        })
    }
}

impl PartialOrd for Location {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Location {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (&self.path, self.range.start.line, self.range.start.column).cmp(&(
            &other.path,
            other.range.start.line,
            other.range.start.column,
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuickfixListType {
    Diagnostic(DiagnosticSeverityRange),
    Items(Vec<QuickfixListItem>),
    Bookmark,
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum DiagnosticSeverityRange {
    All,
    Error,
    Warning,
    Information,
    Hint,
}
impl DiagnosticSeverityRange {
    pub(crate) fn contains(&self, severity: Option<DiagnosticSeverity>) -> bool {
        matches!(
            (self, severity),
            (DiagnosticSeverityRange::All, _)
                | (
                    DiagnosticSeverityRange::Error,
                    Some(DiagnosticSeverity::ERROR)
                )
                | (
                    DiagnosticSeverityRange::Warning,
                    Some(DiagnosticSeverity::WARNING)
                )
                | (
                    DiagnosticSeverityRange::Information,
                    Some(DiagnosticSeverity::INFORMATION)
                )
                | (
                    DiagnosticSeverityRange::Hint,
                    Some(DiagnosticSeverity::HINT)
                )
        )
    }
}

#[cfg(test)]
mod test_quickfix_list {
    use crate::{components::suggestive_editor::Info, position::Position};

    use super::{Location, QuickfixList, QuickfixListItem};
    use pretty_assertions::assert_eq;

    #[test]
    fn should_sort_items() {
        let foo = QuickfixListItem {
            location: Location {
                path: ".gitignore".try_into().unwrap(),
                range: Position { line: 1, column: 2 }..Position { line: 1, column: 3 },
            },
            info: None,
        };
        let bar = QuickfixListItem {
            location: Location {
                path: "readme.md".try_into().unwrap(),
                range: Position { line: 1, column: 1 }..Position { line: 1, column: 2 },
            },
            info: None,
        };
        let spam = QuickfixListItem {
            location: Location {
                path: ".gitignore".try_into().unwrap(),
                range: Position { line: 1, column: 1 }..Position { line: 1, column: 2 },
            },
            info: None,
        };
        let quickfix_list =
            QuickfixList::new(vec![foo.clone(), bar.clone(), spam.clone()], Vec::new());
        assert_eq!(quickfix_list.items(), vec![spam, foo, bar])
    }

    #[test]
    fn should_merge_items_of_same_location() {
        let items = [
            QuickfixListItem {
                location: Location {
                    path: "readme.md".try_into().unwrap(),
                    range: Position { line: 1, column: 1 }..Position { line: 1, column: 2 },
                },
                info: Some(Info::new("Title 1".to_string(), "spongebob".to_string())),
            },
            QuickfixListItem {
                location: Location {
                    path: "readme.md".try_into().unwrap(),
                    range: Position { line: 1, column: 1 }..Position { line: 1, column: 2 },
                },
                info: Some(Info::new("Title 2".to_string(), "squarepants".to_string())),
            },
        ]
        .to_vec();

        let quickfix_list = QuickfixList::new(items, Vec::new());

        assert_eq!(
            quickfix_list.items(),
            vec![QuickfixListItem {
                location: Location {
                    path: "readme.md".try_into().unwrap(),
                    range: Position { line: 1, column: 1 }..Position { line: 1, column: 2 },
                },
                info: Some(Info::new(
                    "Title 1".to_string(),
                    ["spongebob", "squarepants"].join("\n==========\n")
                ))
            }]
        )
    }
}
