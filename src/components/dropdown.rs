use crate::app::Dispatch;
use crate::components::component::Component;
use crate::components::editor::Movement;
use crate::context::Context;
use crate::lsp::completion::CompletionItem;

use itertools::Itertools;
use std::cell::RefCell;
use std::rc::Rc;

use super::component::ComponentId;
use super::editor::Editor;
use super::suggestive_editor::Info;

pub trait DropdownItem: Clone + std::fmt::Debug + Ord {
    fn emoji(&self) -> String {
        String::new()
    }
    fn label(&self) -> String;
    fn display(&self) -> String {
        if self.emoji().is_empty() {
            self.label()
        } else {
            format!("{} {}", self.emoji(), self.label())
        }
    }
    fn group() -> Option<Box<dyn Fn(&Self) -> String>>;
    fn info(&self) -> Option<Info>;
}

impl DropdownItem for String {
    fn label(&self) -> String {
        self.clone()
    }

    fn info(&self) -> Option<Info> {
        None
    }

    fn group() -> Option<Box<dyn Fn(&Self) -> String>> {
        None
    }
}

pub struct Dropdown<T: DropdownItem> {
    open: bool,
    editor: Editor,
    filter: String,
    items: Vec<T>,
    filtered_items: Vec<T>,
    current_item_index: usize,
    info_panel: Option<Rc<RefCell<Editor>>>,
    owner_id: Option<ComponentId>,
}

pub struct DropdownConfig {
    pub title: String,
    pub owner_id: Option<ComponentId>,
}

impl<T: DropdownItem> Dropdown<T> {
    pub fn new(config: DropdownConfig) -> Self {
        let mut editor = Editor::from_text(tree_sitter_quickfix::language(), "");
        editor.set_title(config.title);
        let mut dropdown = Self {
            open: false,
            editor,
            filter: String::new(),
            items: vec![],
            filtered_items: vec![],
            current_item_index: 0,
            info_panel: None,
            owner_id: config.owner_id,
        };
        dropdown.update_editor();
        dropdown
    }

    pub fn owner_id(&self) -> Option<ComponentId> {
        self.owner_id.clone()
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn handle_dispatch(
        &mut self,
        dispatch: DispatchDropdown<T>,
    ) -> anyhow::Result<Vec<Dispatch>> {
        match dispatch {
            DispatchDropdown::SetOpen(open) => {
                self.open = open;
                Ok(Vec::new())
            }
            DispatchDropdown::SetItems(items) => self.set_items(items),
            DispatchDropdown::SetFilter(filter) => self.set_filter(&filter),
            DispatchDropdown::PreviousItem => Ok(self
                .previous_item()
                .map(|(_, dispatches)| dispatches)
                .unwrap_or_default()),
            DispatchDropdown::NextItem => Ok(self
                .next_item()
                .map(|(_, dispatches)| dispatches)
                .unwrap_or_default()),
        }
    }

    pub fn change_index(&mut self, index: usize) -> Option<(T, Vec<Dispatch>)> {
        if !(0..self.items.len()).contains(&index) {
            return self.current_item().map(|item| (item, Vec::new()));
        }
        self.current_item_index = index;
        let group_title_size = T::group().map(|_| 1).unwrap_or(0);

        let result = self.current_item_index
            + self.get_current_item_group_index().unwrap_or(0) * group_title_size
            + group_title_size;
        let dispatches = self.editor.select_line_at(result).ok().unwrap_or_default();
        let item = self.show_current_item();
        item.map(|item| (item, dispatches))
    }

    pub fn next_item(&mut self) -> Option<(T, Vec<Dispatch>)> {
        self.change_index(self.current_item_index + 1)
    }

    pub fn previous_item(&mut self) -> Option<(T, Vec<Dispatch>)> {
        self.change_index(self.current_item_index.saturating_sub(1))
    }

    fn last_item(&mut self) -> Option<(T, Vec<Dispatch>)> {
        self.change_index(self.items.len().saturating_sub(1))
    }

    fn first_item(&mut self) -> Option<(T, Vec<Dispatch>)> {
        self.change_index(0)
    }

    fn groups(&self) -> Option<Vec<String>> {
        T::group().map(|f| {
            self.filtered_items
                .iter()
                .map(f)
                .unique()
                .sorted()
                .collect_vec()
        })
    }

    fn get_current_item_group_index(&self) -> Option<usize> {
        let current_group = T::group()?(&self.current_item()?);
        let groups = self.groups()?;
        let (current_group_index, _) = groups
            .iter()
            .find_position(|group| group == &&current_group)?;
        Some(current_group_index)
    }

    fn change_group_index(&mut self, increment: bool) -> Option<(T, Vec<Dispatch>)> {
        let groups = self.groups()?;
        let get_group = T::group()?;
        let current_group_index = self.get_current_item_group_index()?;
        let new_group_index = if increment {
            current_group_index.saturating_add(1)
        } else {
            current_group_index.saturating_sub(1)
        };
        let new_group = groups.get(new_group_index)?;
        let (new_item_index, _) = self
            .filtered_items
            .iter()
            .find_position(|item| &get_group(item) == new_group)?;
        self.change_index(new_item_index)
    }

    fn next_group(&mut self) -> Option<(T, Vec<Dispatch>)> {
        self.change_group_index(true)
    }

    fn previous_group(&mut self) -> Option<(T, Vec<Dispatch>)> {
        self.change_group_index(false)
    }

    pub fn show_current_item(&mut self) -> Option<T> {
        self.filtered_items
            .get(self.current_item_index)
            .cloned()
            .map(|item| {
                let info = item.info();
                self.show_info(info);
                item
            })
    }

    pub fn current_item(&self) -> Option<T> {
        self.filtered_items.get(self.current_item_index).cloned()
    }

    pub fn set_items(&mut self, items: Vec<T>) -> Result<Vec<Dispatch>, anyhow::Error> {
        self.items = items;
        self.current_item_index = 0;
        self.compute_filtered_items();
        self.update_editor()
    }

    fn compute_filtered_items(&mut self) {
        self.filtered_items = self
            .items
            .iter()
            .filter(|item| {
                item.label()
                    .to_lowercase()
                    .contains(&self.filter.to_lowercase())
            })
            .cloned()
            .sorted_by(|a, b| match T::group() {
                Some(f) => match f(a).cmp(&f(b)) {
                    std::cmp::Ordering::Equal => a.label().cmp(&b.label()),
                    ord => ord,
                },
                None => a.label().cmp(&b.label()),
            })
            .collect();

        self.show_current_item();
    }

    pub fn set_filter(&mut self, filter: &str) -> anyhow::Result<Vec<Dispatch>> {
        self.filter = filter.to_string();
        self.current_item_index = 0;
        self.compute_filtered_items();
        self.update_editor()
    }

    fn update_editor(&mut self) -> anyhow::Result<Vec<Dispatch>> {
        self.editor.set_content(
            &self
                .filtered_items
                .iter()
                .group_by(|item| T::group().map(|f| f(item)))
                .into_iter()
                .map(|(group_key, items)| {
                    if let Some(group_key) = group_key {
                        let items = items.collect_vec();
                        let items_len = items.len();
                        let items = items
                            .into_iter()
                            .sorted()
                            .enumerate()
                            .map(|(index, item)| {
                                let content = item.display();
                                let indicator = if index == items_len.saturating_sub(1) {
                                    "└"
                                } else {
                                    "├"
                                };
                                format!(" {} {}", indicator, content)
                            })
                            .join("\n");
                        format!("■┬ {}\n{}", group_key, items)
                    } else {
                        items.into_iter().map(|item| item.display()).join("\n")
                    }
                })
                .collect::<Vec<String>>()
                .join("\n"),
        )?;

        self.editor.select_line_at(match T::group() {
            Some(_) => 1,
            None => 0,
        })
    }

    fn show_info(&mut self, info: Option<Info>) -> anyhow::Result<()> {
        match info {
            Some(info) => {
                let info_panel = match self.info_panel.take() {
                    Some(info_panel) => info_panel,
                    None => Rc::new(RefCell::new(Editor::from_text(
                        tree_sitter_md::language(),
                        "INFO",
                    ))),
                };

                info_panel.borrow_mut().show_info(info);
                self.info_panel = Some(info_panel);
            }
            _ => self.info_panel = None,
        }
        Ok(())
    }

    pub fn get_item(&mut self, movement: Movement) -> Option<T> {
        match movement {
            Movement::Next => Some(self.next_item()?.0),
            Movement::Current => self.current_item(),
            Movement::Previous => Some(self.previous_item()?.0),
            Movement::Last => Some(self.last_item()?.0),
            Movement::First => Some(self.first_item()?.0),
            Movement::Up => Some(self.previous_group()?.0),
            Movement::Down => Some(self.next_group()?.0),
            _ => None,
        }
    }

    pub fn filtered_items(&self) -> &Vec<T> {
        &self.filtered_items
    }

    #[cfg(test)]
    fn assert_current_label(&self, label: &str, current_selected_text: &str) {
        assert_eq!(self.current_item().unwrap().label(), label);
        assert_eq!(self.editor.get_selected_texts(), &[current_selected_text]);
    }

    pub(crate) fn open(&mut self, open: bool) {
        self.open = open;
    }
}

impl<T: DropdownItem + 'static> Component for Dropdown<T> {
    fn editor(&self) -> &Editor {
        &self.editor
    }

    fn editor_mut(&mut self) -> &mut Editor {
        &mut self.editor
    }

    fn handle_key_event(
        &mut self,
        context: &Context,
        event: event::KeyEvent,
    ) -> anyhow::Result<Vec<Dispatch>> {
        self.editor.handle_key_event(context, event)
    }

    fn children(&self) -> Vec<Option<Rc<RefCell<dyn Component>>>> {
        vec![self
            .info_panel
            .clone()
            .map(|info_panel| Some(info_panel as Rc<RefCell<dyn Component>>))
            .unwrap_or_default()]
    }

    fn remove_child(&mut self, component_id: ComponentId) {
        if matches!(self.info_panel, Some(ref info_panel) if info_panel.borrow().id() == component_id)
        {
            self.info_panel = None;
        }
    }
}

#[cfg(test)]
mod test_dropdown {
    use crate::{
        components::{
            dropdown::{Dropdown, DropdownConfig, DropdownItem},
            suggestive_editor::Info,
        },
        selection::CharIndex,
    };
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct Item {
        label: String,
        info: Info,
        group: String,
    }
    impl PartialOrd for Item {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            self.label.partial_cmp(&other.label)
        }
    }

    impl Ord for Item {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            self.label.cmp(&other.label)
        }
    }

    impl Item {
        fn new(label: &str, info: &str, group: &str) -> Self {
            Self {
                label: label.to_string(),
                info: Info::new(info.to_string()),
                group: group.to_string(),
            }
        }
    }

    impl DropdownItem for Item {
        fn label(&self) -> String {
            self.label.to_string()
        }

        fn info(&self) -> Option<Info> {
            Some(self.info.clone())
        }

        fn group() -> Option<Box<dyn Fn(&Self) -> String>> {
            Some(Box::new(|item| item.group.clone()))
        }
    }

    #[test]
    fn test_next_prev_group() {
        let mut dropdown = Dropdown::new(DropdownConfig {
            title: "test".to_string(),
            owner_id: None,
        });
        dropdown.set_items(
            [
                Item::new("a", "", "1"),
                Item::new("d", "", "2"),
                Item::new("c", "", "2"),
                Item::new("b", "", "3"),
            ]
            .to_vec(),
        );

        // Expect the items are sorted by group first, then by label
        assert_eq!(
            dropdown.editor.buffer().rope().to_string(),
            "
■┬ 1
 └ a
■┬ 2
 ├ c
 └ d
■┬ 3
 └ b
"
            .trim()
        );
        dropdown.assert_current_label("a", " └ a\n");

        dropdown.next_group();
        dropdown.assert_current_label("c", " ├ c\n");
        dropdown.next_group();
        dropdown.assert_current_label("b", " └ b");

        dropdown.previous_group();
        dropdown.assert_current_label("c", " ├ c\n");
        dropdown.previous_group();
        dropdown.assert_current_label("a", " └ a\n");
    }

    #[test]
    fn test_dropdown_without_group() -> anyhow::Result<()> {
        let mut dropdown = Dropdown::new(DropdownConfig {
            title: "test".to_string(),
            owner_id: None,
        });
        dropdown.set_items(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
        assert_eq!(dropdown.editor.buffer().rope().to_string(), "a\nb\nc");
        dropdown.assert_current_label("a", "a\n");
        assert_eq!(
            dropdown.editor.selection_set.primary.extended_range(),
            (CharIndex(0)..CharIndex(2)).into()
        );
        dropdown.next_item();
        dropdown.assert_current_label("b", "b\n");
        assert_eq!(dropdown.current_item().unwrap().label(), "b");
        dropdown.next_item();
        dropdown.assert_current_label("c", "c");
        dropdown.next_item();
        dropdown.assert_current_label("c", "c");

        dropdown.previous_item();
        dropdown.assert_current_label("b", "b\n");
        dropdown.previous_item();
        dropdown.assert_current_label("a", "a\n");
        dropdown.previous_item();
        dropdown.assert_current_label("a", "a\n");

        dropdown.set_filter("b")?;
        dropdown.assert_current_label("b", "b");
        dropdown.set_filter("c")?;
        dropdown.assert_current_label("c", "c");
        dropdown.set_filter("d")?;
        assert_eq!(dropdown.current_item(), None);

        dropdown.set_filter("")?;
        dropdown.assert_current_label("a", "a\n");
        dropdown.next_item();
        dropdown.assert_current_label("b", "b\n");

        dropdown.set_items(vec![
            "lorem".to_string(),
            "ipsum".to_string(),
            "dolor".to_string(),
        ]);

        // The current item should be `dolor` because dropdown will sort the items
        dropdown.assert_current_label("dolor", "dolor\n");
        dropdown.next_item();
        dropdown.assert_current_label("ipsum", "ipsum\n");

        Ok(())
    }

    #[test]
    fn filter_should_work_regardless_of_case() -> anyhow::Result<()> {
        let mut dropdown = Dropdown::new(DropdownConfig {
            title: "test".to_string(),
            owner_id: None,
        });
        dropdown.set_items(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
        dropdown.set_filter("A")?;
        assert_eq!(dropdown.current_item().unwrap().label(), "a");
        Ok(())
    }

    #[test]
    fn setting_filter_should_show_info_of_the_new_first_item() -> anyhow::Result<()> {
        let mut dropdown = Dropdown::new(DropdownConfig {
            title: "test".to_string(),
            owner_id: None,
        });
        dropdown.set_items(vec![
            Item::new("a", "info a", ""),
            Item::new("b", "info b", ""),
            Item::new("c", "info c", ""),
        ]);

        assert_eq!(dropdown.current_item().unwrap().label(), "a");
        assert_eq!(
            dropdown.info_panel.as_ref().unwrap().borrow().text(),
            "info a"
        );

        dropdown.set_filter("b")?;

        assert_eq!(dropdown.current_item().unwrap().label(), "b");
        assert_eq!(
            dropdown.info_panel.as_ref().unwrap().borrow().text(),
            "info b"
        );
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DispatchDropdown<T> {
    SetOpen(bool),
    SetItems(Vec<T>),
    SetFilter(String),
    PreviousItem,
    NextItem,
}
