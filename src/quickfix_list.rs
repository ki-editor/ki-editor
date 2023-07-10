use std::{cell::RefCell, ops::Range, rc::Rc};

use itertools::Itertools;

use crate::{
    canonicalized_path::CanonicalizedPath,
    components::{
        component::{Component, ComponentId},
        dropdown::{Dropdown, DropdownConfig, DropdownItem},
        editor::Direction,
    },
    position::Position,
};

pub struct QuickfixLists {
    lists: Vec<QuickfixList>,
    dropdown: Dropdown<QuickfixListItem>,
}

impl DropdownItem for QuickfixListItem {
    fn label(&self) -> String {
        self.location().display()
    }

    fn info(&self) -> Option<String> {
        self.infos.join("\n").into()
    }
}

impl Component for QuickfixLists {
    fn editor(&self) -> &crate::components::editor::Editor {
        self.dropdown.editor()
    }

    fn editor_mut(&mut self) -> &mut crate::components::editor::Editor {
        self.dropdown.editor_mut()
    }

    fn handle_key_event(
        &mut self,
        context: &mut crate::context::Context,
        event: event::KeyEvent,
    ) -> anyhow::Result<Vec<crate::screen::Dispatch>> {
        self.dropdown.handle_key_event(context, event)
    }

    fn children(&self) -> Vec<Option<Rc<RefCell<dyn Component>>>> {
        self.dropdown.children()
    }

    fn remove_child(&mut self, component_id: ComponentId) {
        self.dropdown.remove_child(component_id)
    }
}

impl QuickfixLists {
    pub fn new() -> QuickfixLists {
        QuickfixLists {
            lists: vec![],
            dropdown: Dropdown::new(DropdownConfig {
                title: "Quickfix".to_string(),
            }),
        }
    }

    pub fn current_mut(&mut self) -> Option<&mut QuickfixList> {
        self.lists.last_mut()
    }

    pub fn push(&mut self, quickfix_list: QuickfixList) {
        self.dropdown.set_items(quickfix_list.items.clone());
        self.lists.push(quickfix_list);
    }

    pub fn get_item(&mut self, direction: Direction) -> Option<QuickfixListItem> {
        self.dropdown.get_item(direction)
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickfixListItem {
    location: Location,
    infos: Vec<String>,
}

impl PartialOrd for QuickfixListItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.location.partial_cmp(&other.location)
    }
}

impl Ord for QuickfixListItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap_or(std::cmp::Ordering::Equal)
    }
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
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Location {
    pub path: CanonicalizedPath,
    pub range: Range<Position>,
}

impl Location {
    pub fn display(&self) -> String {
        format!(
            "{}:{}:{}-{}:{}",
            self.path
                .display_relative()
                .unwrap_or_else(|_| self.path.display()),
            self.range.start.line + 1,
            self.range.start.column + 1,
            self.range.end.line + 1,
            self.range.end.column + 1
        )
    }

    pub fn read(&self) -> anyhow::Result<String> {
        // TODO: optimize this function, should not read the whole file
        self.path
            .read()
            .map(|result| {
                // Return only the specified range
                result
                    .lines()
                    .enumerate()
                    .filter(|(line_index, _)| {
                        line_index >= &self.range.start.line && line_index <= &self.range.end.line
                    })
                    .map(|(_, line)| line)
                    .collect_vec()
                    .join("\n")
            })
            .map_err(|err| {
                anyhow::anyhow!(
                    "Failed to read file {}: {}",
                    self.path.display(),
                    err.to_string()
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
        self.path.partial_cmp(&other.path).map(|ord| match ord {
            std::cmp::Ordering::Equal => self.range.start.cmp(&other.range.start),
            _ => ord,
        })
    }
}

impl Ord for Location {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap_or(std::cmp::Ordering::Equal)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
                path: ".gitignore".try_into().unwrap(),
                range: Position { line: 1, column: 2 }..Position { line: 1, column: 3 },
            },
            infos: vec![],
        };
        let bar = QuickfixListItem {
            location: Location {
                path: "readme.md".try_into().unwrap(),
                range: Position { line: 1, column: 1 }..Position { line: 1, column: 2 },
            },
            infos: vec![],
        };
        let spam = QuickfixListItem {
            location: Location {
                path: ".gitignore".try_into().unwrap(),
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
                    path: "readme.md".try_into().unwrap(),
                    range: Position { line: 1, column: 1 }..Position { line: 1, column: 2 },
                },
                infos: vec!["spongebob".to_string()],
            },
            QuickfixListItem {
                location: Location {
                    path: "readme.md".try_into().unwrap(),
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
                    path: "readme.md".try_into().unwrap(),
                    range: Position { line: 1, column: 1 }..Position { line: 1, column: 2 },
                },
                infos: vec!["spongebob".to_string(), "squarepants".to_string()],
            }]
        )
    }
}
