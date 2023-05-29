use crossterm::event::Event;

use crate::components::component::Component;
use crate::edit::Edit;
use crate::screen::{Dispatch, State};
use crate::selection::SelectionMode;

use super::component::ComponentId;
use super::editor::{Direction, Editor};

pub struct Dropdown {
    editor: Editor,
    filter: String,
    items: Vec<DropdownItem>,
}

pub struct DropdownConfig {
    pub title: String,
}

pub struct DropdownItem {
    pub label: String,

    pub edit: Option<Edit>,
}

impl Dropdown {
    pub fn new(config: DropdownConfig) -> Self {
        let mut editor = Editor::from_text(tree_sitter_md::language(), "");
        editor.select(SelectionMode::Line, Direction::Current);
        editor.set_title(config.title);
        Self {
            editor,
            filter: String::new(),
            items: vec![],
        }
    }

    pub fn next_item(&mut self) -> String {
        self.editor.select(SelectionMode::Line, Direction::Forward);
        self.editor.get_current_line().trim().to_string()
    }

    pub fn previous_item(&mut self) -> String {
        self.editor.select(SelectionMode::Line, Direction::Backward);
        self.editor.get_current_line().trim().to_string()
    }

    pub fn set_items(&mut self, items: Vec<DropdownItem>) {
        self.items = items;
        self.update_editor();
    }

    pub fn set_filter(&mut self, filter: &str) {
        self.filter = filter.to_string();
        self.update_editor()
    }

    fn update_editor(&mut self) {
        self.editor.reset_selection();
        self.editor.update(
            &self
                .items
                .iter()
                .filter(|item| {
                    item.label
                        .to_lowercase()
                        .contains(&self.filter.to_lowercase())
                })
                .map(|item| item.label.clone())
                .collect::<Vec<String>>()
                .join("\n"),
        )
    }
}

impl Component for Dropdown {
    fn editor(&self) -> &Editor {
        &self.editor
    }

    fn editor_mut(&mut self) -> &mut Editor {
        &mut self.editor
    }

    fn handle_event(&mut self, state: &State, event: Event) -> anyhow::Result<Vec<Dispatch>> {
        self.editor.handle_event(state, event)
    }

    fn slave_ids(&self) -> Vec<ComponentId> {
        vec![]
    }
}
