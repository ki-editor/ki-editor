use crossterm::event::Event;

use crate::components::component::Component;
use crate::screen::{Dispatch, State};

use super::editor::Editor;

pub trait DropdownItem: Clone + std::fmt::Debug {
    fn label(&self) -> String;
}

impl DropdownItem for String {
    fn label(&self) -> String {
        self.clone()
    }
}

pub struct Dropdown<T: DropdownItem> {
    editor: Editor,
    filter: String,
    items: Vec<T>,
    filtered_items: Vec<T>,
    current_item_index: usize,
}

pub struct DropdownConfig {
    pub title: String,
}

impl<T: DropdownItem> Dropdown<T> {
    pub fn new(config: DropdownConfig) -> Self {
        let mut editor = Editor::from_text(tree_sitter_md::language(), "");
        editor.set_title(config.title);
        let mut dropdown = Self {
            editor,
            filter: String::new(),
            items: vec![],
            filtered_items: vec![],
            current_item_index: 0,
        };
        dropdown.update_editor();
        dropdown
    }

    pub fn next_item(&mut self) -> Option<T> {
        if self.current_item_index == self.filtered_items.len() - 1 {
            return self.current_item();
        }
        self.current_item_index += 1;
        self.editor.select_line_at(self.current_item_index);
        self.current_item()
    }

    pub fn previous_item(&mut self) -> Option<T> {
        if self.current_item_index == 0 {
            return self.current_item();
        }
        self.current_item_index -= 1;
        self.editor.select_line_at(self.current_item_index);
        self.current_item()
    }

    pub fn current_item(&self) -> Option<T> {
        self.filtered_items.get(self.current_item_index).cloned()
    }

    pub fn set_items(&mut self, items: Vec<T>) {
        self.items = items;
        self.current_item_index = 0;
        self.compute_filtered_items();
        self.update_editor();
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
            .collect();
    }

    pub fn set_filter(&mut self, filter: &str) {
        self.filter = filter.to_string();
        self.current_item_index = 0;
        self.compute_filtered_items();
        self.update_editor()
    }

    fn update_editor(&mut self) {
        self.editor.update(
            &self
                .filtered_items
                .iter()
                .map(|item| item.label())
                .collect::<Vec<String>>()
                .join("\n"),
        );

        self.editor.select_line_at(0);
    }
}

impl<T: DropdownItem + 'static> Component for Dropdown<T> {
    fn editor(&self) -> &Editor {
        &self.editor
    }

    fn editor_mut(&mut self) -> &mut Editor {
        &mut self.editor
    }

    fn handle_event(&mut self, state: &State, event: Event) -> anyhow::Result<Vec<Dispatch>> {
        self.editor.handle_event(state, event)
    }

    fn children(&self) -> Vec<std::rc::Rc<std::cell::RefCell<dyn Component>>> {
        vec![]
    }
}

mod test_dropdown {
    use crate::{
        components::dropdown::{Dropdown, DropdownConfig, DropdownItem},
        selection::CharIndex,
    };

    #[test]
    fn test_dropdown() {
        let mut dropdown = Dropdown::new(DropdownConfig {
            title: "test".to_string(),
        });
        dropdown.set_items(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
        assert_eq!(dropdown.current_item().unwrap().label(), "a");
        assert_eq!(
            dropdown.editor.selection_set.primary.range,
            CharIndex(0)..CharIndex(2)
        );
        dropdown.next_item();
        assert_eq!(dropdown.current_item().unwrap().label(), "b");
        dropdown.next_item();
        assert_eq!(dropdown.current_item().unwrap().label(), "c");
        dropdown.next_item();
        assert_eq!(dropdown.current_item().unwrap().label(), "c");

        dropdown.previous_item();
        assert_eq!(dropdown.current_item().unwrap().label(), "b");
        dropdown.previous_item();
        assert_eq!(dropdown.current_item().unwrap().label(), "a");
        dropdown.previous_item();
        assert_eq!(dropdown.current_item().unwrap().label(), "a");

        dropdown.set_filter("b");
        assert_eq!(dropdown.current_item().unwrap().label(), "b");
        dropdown.set_filter("c");
        assert_eq!(dropdown.current_item().unwrap().label(), "c");
        dropdown.set_filter("d");
        assert_eq!(dropdown.current_item(), None);

        dropdown.set_filter("");
        assert_eq!(dropdown.current_item().unwrap().label(), "a");
        dropdown.next_item();
        assert_eq!(dropdown.current_item().unwrap().label(), "b");

        dropdown.set_items(vec![
            "lorem".to_string(),
            "ipsum".to_string(),
            "dolor".to_string(),
        ]);
        assert_eq!(dropdown.current_item().unwrap().label(), "lorem");
        assert_eq!(dropdown.editor.get_current_line(), "lorem\n");
        dropdown.next_item();
        assert_eq!(dropdown.current_item().unwrap().label(), "ipsum");
        assert_eq!(dropdown.editor.get_current_line(), "ipsum\n");
    }

    #[test]
    fn filter_should_work_regardless_of_case() {
        let mut dropdown = Dropdown::new(DropdownConfig {
            title: "test".to_string(),
        });
        dropdown.set_items(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
        dropdown.set_filter("A");
        assert_eq!(dropdown.current_item().unwrap().label(), "a");
    }
}
