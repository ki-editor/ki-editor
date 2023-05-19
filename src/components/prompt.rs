use std::{cell::RefCell, rc::Rc};

use crossterm::event::{Event, KeyCode};
use itertools::Itertools;

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
    on_text_change: OnTextChange,
    get_suggestions: GetSuggestions,
}

type OnEnter = Box<
    dyn Fn(
        /* text */ &str,
        /* current_suggestion */ &str,
        /*owner*/ Rc<RefCell<dyn Component>>,
    ) -> Vec<Dispatch>,
>;
type GetSuggestions = Box<
    dyn Fn(
        /* text */ &str,
        /*owner*/ Rc<RefCell<dyn Component>>,
    ) -> anyhow::Result<Vec<String>>,
>;

type OnTextChange = Box<
    dyn Fn(
        /* text */ &str,
        /*owner*/ Rc<RefCell<dyn Component>>,
    ) -> anyhow::Result<Vec<Dispatch>>,
>;

pub struct PromptConfig {
    pub owner_id: ComponentId,
    pub history: Vec<String>,
    pub dropdown_id: ComponentId,
    pub dropdown: Rc<RefCell<Dropdown>>,
    pub owner: Rc<RefCell<dyn Component>>,
    pub on_enter: OnEnter,
    pub on_text_change: OnTextChange,
    pub get_suggestions: GetSuggestions,
    pub title: String,
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
        editor.set_title(config.title);
        Prompt {
            editor,
            owner_id: config.owner_id,
            dropdown_id: config.dropdown_id,
            text: "".to_string(),
            dropdown: config.dropdown,
            owner: config.owner,
            on_enter: config.on_enter,
            on_text_change: config.on_text_change,
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
    fn handle_event(&mut self, state: &State, event: Event) -> anyhow::Result<Vec<Dispatch>> {
        match event {
            Event::Key(key_event) => match key_event.code {
                KeyCode::Esc if self.editor.mode == Mode::Normal => {
                    return Ok(vec![Dispatch::CloseCurrentWindow {
                        change_focused_to: self.owner_id,
                    }]);
                }
                KeyCode::Enter => {
                    let dispatches = (self.on_enter)(
                        &self.text,
                        &self.dropdown.borrow().editor().get_current_line().trim(),
                        self.owner.clone(),
                    );
                    return Ok(vec![Dispatch::CloseCurrentWindow {
                        change_focused_to: self.owner_id,
                    }]
                    .into_iter()
                    .chain(dispatches)
                    .collect());
                }
                KeyCode::Down => {
                    let text = self.dropdown.borrow_mut().next_item();
                    self.set_text(text);
                    return Ok(vec![]);
                }
                KeyCode::Up => {
                    let text = self.dropdown.borrow_mut().previous_item();
                    self.set_text(text);

                    return Ok(vec![]);
                }
                _ => {}
            },
            _ => {}
        };

        let dispatches = self.editor.handle_event(state, event.clone())?;

        let suggestions = (self.get_suggestions)(&self.text, self.owner.clone());
        self.dropdown.borrow_mut().update(&suggestions?.join("\n"));
        self.dropdown
            .borrow_mut()
            .editor_mut()
            .select_line(Direction::Current);

        let current_text = self.editor.get_current_line().trim().to_string();

        let result = if current_text == self.text {
            dispatches
        } else {
            self.text = current_text.clone();

            (self.on_text_change)(&current_text, self.owner.clone())?;

            dispatches.into_iter().chain(vec![]).collect_vec()
        };
        Ok(result)
    }

    fn slave_ids(&self) -> Vec<ComponentId> {
        vec![self.dropdown_id]
    }
}
