use std::{ops::Range, path::PathBuf};

use itertools::Itertools;

use crate::position::Position;

pub struct QuickfixLists {
    lists: Vec<QuickfixList>,
}

impl QuickfixLists {
    pub fn new() -> QuickfixLists {
        QuickfixLists { lists: vec![] }
    }

    pub fn current_mut(&mut self) -> Option<&mut QuickfixList> {
        self.lists.last_mut()
    }

    pub fn push(&mut self, quickfix_list: QuickfixList) {
        self.lists.push(quickfix_list);
    }
}

#[derive(Clone)]
pub struct QuickfixList {
    current_index: usize,
    items: Vec<QuickfixListItem>,
}

impl QuickfixList {
    pub fn new(items: Vec<QuickfixListItem>) -> QuickfixList {
        QuickfixList {
            current_index: 0,
            items: {
                let mut items = items;

                // Sort the items by location
                items.sort_by(|a, b| a.location.cmp(&b.location));

                // Merge items of same locations
                items
                    .into_iter()
                    .group_by(|item| item.location.clone())
                    .into_iter()
                    .map(|(location, items)| QuickfixListItem {
                        location,
                        infos: items.into_iter().flat_map(|item| item.infos).collect_vec(),
                    })
                    .collect_vec()
            },
        }
    }

    pub fn current_item(&self) -> Option<&QuickfixListItem> {
        self.items.get(self.current_index)
    }

    pub fn next_item(&mut self) -> Option<&QuickfixListItem> {
        if self.current_index == self.items.len() - 1 {
            return self.current_item();
        }
        self.current_index += 1;
        self.current_item()
    }

    pub fn previous_item(&mut self) -> Option<&QuickfixListItem> {
        if self.current_index == 0 {
            return self.current_item();
        }
        self.current_index -= 1;
        self.current_item()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickfixListItem {
    location: Location,
    infos: Vec<String>,
}

impl From<Location> for QuickfixListItem {
    fn from(value: Location) -> Self {
        QuickfixListItem {
            location: value,
            infos: vec![],
        }
    }
}

impl QuickfixListItem {
    pub fn new(location: Location, infos: Vec<String>) -> QuickfixListItem {
        QuickfixListItem { location, infos }
    }

    pub fn location(&self) -> &Location {
        &self.location
    }

    pub fn infos(&self) -> Vec<String> {
        self.infos.clone()
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Location {
    pub path: PathBuf,
    pub range: Range<Position>,
}

impl TryFrom<lsp_types::Location> for Location {
    type Error = anyhow::Error;

    fn try_from(value: lsp_types::Location) -> Result<Self, Self::Error> {
        Ok(Location {
            path: value
                .uri
                .to_file_path()
                .map_err(|_| anyhow::anyhow!("Failed to convert uri to file path"))?,
            range: value.range.start.into()..value.range.end.into(),
        })
    }
}

impl PartialOrd for Location {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.path.partial_cmp(&other.path).map(|ord| match ord {
            std::cmp::Ordering::Equal => self.range.start.cmp(&other.range.start),
            _ => ord,
        })
    }
}

impl Ord for Location {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other)
            .unwrap_or_else(|| std::cmp::Ordering::Equal)
    }
}

#[derive(Debug, Clone)]
pub enum QuickfixListType {
    LspDiagnostic,
}

#[cfg(test)]
mod test_quickfix_list {
    use crate::position::Position;

    use super::{Location, QuickfixList, QuickfixListItem};
    use pretty_assertions::assert_eq;

    #[test]
    fn should_sort_items() {
        let foo = QuickfixListItem {
            location: Location {
                path: "a".into(),
                range: Position { line: 1, column: 2 }..Position { line: 1, column: 3 },
            },
            infos: vec![],
        };
        let bar = QuickfixListItem {
            location: Location {
                path: "b".into(),
                range: Position { line: 1, column: 1 }..Position { line: 1, column: 2 },
            },
            infos: vec![],
        };
        let spam = QuickfixListItem {
            location: Location {
                path: "a".into(),
                range: Position { line: 1, column: 1 }..Position { line: 1, column: 2 },
            },
            infos: vec![],
        };
        let quickfix_list = QuickfixList::new(vec![foo.clone(), bar.clone(), spam.clone()]);

        assert_eq!(quickfix_list.items, vec![spam, foo, bar])
    }

    #[test]
    fn should_merge_items_of_same_location() {
        let items = vec![
            QuickfixListItem {
                location: Location {
                    path: "a".into(),
                    range: Position { line: 1, column: 1 }..Position { line: 1, column: 2 },
                },
                infos: vec!["spongebob".to_string()],
            },
            QuickfixListItem {
                location: Location {
                    path: "a".into(),
                    range: Position { line: 1, column: 1 }..Position { line: 1, column: 2 },
                },
                infos: vec!["squarepants".to_string()],
            },
        ];

        let quickfix_list = QuickfixList::new(items);

        assert_eq!(
            quickfix_list.items,
            vec![QuickfixListItem {
                location: Location {
                    path: "a".into(),
                    range: Position { line: 1, column: 1 }..Position { line: 1, column: 2 },
                },
                infos: vec!["spongebob".to_string(), "squarepants".to_string()],
            }]
        )
    }
}
