use std::{cell::RefCell, rc::Rc};

use crossterm::event::{Event, KeyCode};

use crate::screen::{Dispatch, State};

use super::{
    component::{Component, ComponentId},
    dropdown::Dropdown,
    editor::{Direction, Editor, Mode},
};

pub struct Prompt {
    editor: Editor,
    owner_id: ComponentId,
    dropdown_id: ComponentId,
    text: String,
    dropdown: Rc<RefCell<Dropdown>>,
    owner: Rc<RefCell<dyn Component>>,
    on_enter: OnEnter,
    get_suggestions: GetSuggestions,
}

type OnEnter = Box<dyn Fn(/* text */ &str, /*owner*/ Rc<RefCell<dyn Component>>) -> Vec<Dispatch>>;
type GetSuggestions =
    Box<dyn Fn(/* text */ &str, /*owner*/ Rc<RefCell<dyn Component>>) -> Vec<String>>;

pub struct PromptConfig {
    pub owner_id: ComponentId,
    pub history: Vec<String>,
    pub dropdown_id: ComponentId,
    pub dropdown: Rc<RefCell<Dropdown>>,
    pub owner: Rc<RefCell<dyn Component>>,
    pub on_enter: OnEnter,
    pub get_suggestions: GetSuggestions,
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
            text: "".to_string(),
            dropdown: config.dropdown,
            owner: config.owner,
            on_enter: config.on_enter,
            get_suggestions: config.get_suggestions,
        }
    }

    fn set_text(&mut self, text: String) {
        self.text = text;
        self.editor.update(&self.text);
    }
}

impl Component for Prompt {
    fn editor(&self) -> &Editor {
        &self.editor
    }
    fn editor_mut(&mut self) -> &mut Editor {
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
                    let dispatches = (self.on_enter)(&self.text, self.owner.clone());
                    return dispatches
                        .into_iter()
                        .chain(vec![Dispatch::CloseCurrentWindow {
                            change_focused_to: self.owner_id,
                        }])
                        .collect();
                }
                KeyCode::Down => {
                    let text = self.dropdown.borrow_mut().next_item();
                    self.set_text(text);
                    return vec![];
                }
                KeyCode::Up => {
                    let text = self.dropdown.borrow_mut().previous_item();
                    self.set_text(text);

                    return vec![];
                }
                _ => {}
            },
            _ => {}
        };

        let dispatches = self.editor.handle_event(state, event.clone());

        let suggestions = (self.get_suggestions)(&self.text, self.owner.clone());
        self.dropdown.borrow_mut().update(&suggestions.join("\n"));

        let current_text = self.editor.get_current_line().trim().to_string();

        if current_text == self.text {
            dispatches
        } else {
            self.text = current_text.clone();
            self.owner
                .borrow_mut()
                .editor_mut()
                .select_match(Direction::Forward, &Some(current_text));
            dispatches.into_iter().chain(vec![]).collect()
        }
    }

    fn slave_ids(&self) -> Vec<ComponentId> {
        vec![self.dropdown_id]
    }
}
