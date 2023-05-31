use std::{cell::RefCell, rc::Rc};

use crossterm::event::{Event, KeyCode, KeyModifiers};
use itertools::Itertools;

use crate::screen::{Dispatch, State};

use super::{
    component::{Component, ComponentId},
    dropdown::{Dropdown, DropdownConfig},
    editor::{Direction, Editor, Mode},
};

pub struct Prompt {
    editor: Editor,
    text: String,
    dropdown: Rc<RefCell<Dropdown<String>>>,
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
    pub history: Vec<String>,
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
            text: "".to_string(),
            dropdown: Rc::new(RefCell::new(Dropdown::new(DropdownConfig {
                title: "Suggestions".to_string(),
                items: vec![],
            }))),
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

    fn select_previous_suggestion(&mut self) {
        let text = self.dropdown.borrow_mut().previous_item();
        text.map(|text| self.set_text(text));
    }

    fn select_next_suggestion(&mut self) {
        let text = self.dropdown.borrow_mut().next_item();
        text.map(|text| self.set_text(text));
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
                        change_focused_to: self.owner.borrow().id(),
                    }]);
                }
                KeyCode::Enter => {
                    let dispatches = (self.on_enter)(
                        &self.text,
                        &self
                            .dropdown
                            .borrow()
                            .current_item()
                            .unwrap_or(String::new()),
                        self.owner.clone(),
                    );
                    return Ok(vec![Dispatch::CloseCurrentWindow {
                        change_focused_to: self.owner.borrow().id(),
                    }]
                    .into_iter()
                    .chain(dispatches)
                    .collect());
                }
                KeyCode::Down => {
                    self.select_next_suggestion();
                    return Ok(vec![]);
                }
                KeyCode::Up => {
                    self.select_previous_suggestion();
                    return Ok(vec![]);
                }
                KeyCode::Char('n') if key_event.modifiers == KeyModifiers::CONTROL => {
                    self.select_next_suggestion();
                    return Ok(vec![]);
                }
                KeyCode::Char('p') if key_event.modifiers == KeyModifiers::CONTROL => {
                    self.select_previous_suggestion();
                    return Ok(vec![]);
                }
                _ => {}
            },
            _ => {}
        };

        let dispatches = self.editor.handle_event(state, event.clone())?;

        let suggestions = (self.get_suggestions)(&self.text, self.owner.clone())?;

        // TODO: don't use dropdown.update, use dropdown.set_items instead
        self.dropdown.borrow_mut().set_items(suggestions);

        let current_text = self
            .editor
            .get_current_line()
            .to_string()
            .trim()
            .to_string();

        let result = if current_text == self.text {
            dispatches
        } else {
            self.text = current_text.clone();

            (self.on_text_change)(&current_text, self.owner.clone())?;

            dispatches.into_iter().chain(vec![]).collect_vec()
        };
        Ok(result)
    }

    fn children(&self) -> Vec<Rc<RefCell<dyn Component>>> {
        vec![self.dropdown.clone()]
    }
}
