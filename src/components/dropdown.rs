use crossterm::event::Event;

use crate::components::component::Component;
use crate::edit::Edit;
use crate::screen::{Dispatch, State};
use crate::selection::SelectionMode;

use super::editor::{Direction, Editor};

pub trait DropdownItem: Clone {
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

pub struct DropdownConfig<T: DropdownItem> {
    pub title: String,
    pub items: Vec<T>,
}

impl<T: DropdownItem> Dropdown<T> {
    pub fn new(config: DropdownConfig<T>) -> Self {
        let mut editor = Editor::from_text(tree_sitter_md::language(), "");
        editor.select(SelectionMode::Line, Direction::Current);
        editor.set_title(config.title);
        Self {
            editor,
            filter: String::new(),
            filtered_items: config.items.clone(),
            items: config.items,
            current_item_index: 0,
        }
    }

    pub fn next_item(&mut self) -> T {
        if self.current_item_index == self.filtered_items.len() - 1 {
            return self.current_item();
        }
        self.editor.select(SelectionMode::Line, Direction::Forward);
        self.current_item_index += 1;
        self.current_item()
    }

    pub fn previous_item(&mut self) -> T {
        if self.current_item_index == 0 {
            return self.current_item();
        }
        self.editor.select(SelectionMode::Line, Direction::Backward);
        self.current_item_index -= 1;
        self.current_item()
    }

    pub fn current_item(&self) -> T {
        self.filtered_items[self.current_item_index].clone()
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
            .filter(|item| item.label().contains(&self.filter))
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
        self.editor.reset_selection();
        self.editor.update(
            &self
                .filtered_items
                .iter()
                .map(|item| item.label())
                .collect::<Vec<String>>()
                .join("\n"),
        )
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

// TODO: add tests
