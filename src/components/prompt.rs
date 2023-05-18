use crossterm::event::{Event, KeyCode};

use crate::screen::{Dispatch, State};

use super::{
    component::{Component, ComponentId},
    editor::{Editor, Mode},
};

#[derive(Clone)]
pub struct Prompt {
    editor: Editor,
    owner_id: ComponentId,
    dropdown_id: ComponentId,
    search: String,
}

pub struct PromptConfig {
    pub owner_id: ComponentId,
    pub history: Vec<String>,
    pub dropdown_id: ComponentId,
}

impl Prompt {
    pub fn new(config: PromptConfig) -> Self {
        let text = &if config.history.is_empty() {
            "".to_string()
        } else {
            let mut history = config.history.clone();
            history.reverse();
            format!("\n{}", history.join("\n"))
        };
        let mut editor = Editor::from_text(tree_sitter_md::language(), text);
        editor.enter_insert_mode();
        Prompt {
            editor,
            owner_id: config.owner_id,
            dropdown_id: config.dropdown_id,
            search: "".to_string(),
        }
    }
}

impl Component for Prompt {
    fn child(&self) -> &dyn Component {
        &self.editor
    }
    fn child_mut(&mut self) -> &mut dyn Component {
        &mut self.editor
    }
    fn handle_event(&mut self, state: &State, event: Event) -> Vec<Dispatch> {
        match event {
            Event::Key(key_event) => match key_event.code {
                KeyCode::Esc if self.editor.mode == Mode::Normal => {
                    return vec![Dispatch::CloseCurrentWindow {
                        change_focused_to: self.owner_id,
                    }];
                }

                KeyCode::Enter => {
                    return vec![
                        Dispatch::SetSearch {
                            search: self.editor.get_current_line().trim().to_string(),
                        },
                        Dispatch::CloseCurrentWindow {
                            change_focused_to: self.owner_id,
                        },
                    ]
                }
                KeyCode::Down => {
                    return vec![Dispatch::NextDropdownItem {
                        dropdown_id: self.dropdown_id,
                    }]
                }
                KeyCode::Up => {
                    return vec![Dispatch::PreviousDropdownItem {
                        dropdown_id: self.dropdown_id,
                    }]
                }
                _ => {}
            },
            _ => {}
        };

        let dispatches = self.editor.handle_event(state, event);

        let current_search = self.editor.get_current_line().trim().to_string();

        if current_search == self.search {
            dispatches
        } else {
            self.search = current_search.clone();
            dispatches
                .into_iter()
                .chain(vec![Dispatch::Search {
                    editor_id: self.owner_id,
                    dropdown_id: self.dropdown_id,
                    search: current_search,
                }])
                .collect()
        }
    }

    fn slave_ids(&self) -> Vec<ComponentId> {
        vec![self.dropdown_id]
    }
}
