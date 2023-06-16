use std::{cell::RefCell, rc::Rc};

use crossterm::event::{Event, KeyCode};
use itertools::Itertools;

use crate::{
    buffer::Buffer,
    lsp::completion::{Completion, CompletionItem},
    screen::{Dispatch, State},
};

use super::{
    component::Component,
    editor::{Editor, Mode},
    suggestive_editor::SuggestiveEditor,
};

pub struct Prompt {
    editor: SuggestiveEditor,
    text: String,
    owner: Rc<RefCell<dyn Component>>,
    on_enter: OnEnter,
    on_text_change: OnTextChange,
}

type OnEnter = Box<
    dyn Fn(
        /* current_suggestion */ &str,
        /*owner*/ Rc<RefCell<dyn Component>>,
    ) -> anyhow::Result<Vec<Dispatch>>,
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
    pub items: Vec<CompletionItem>,
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
        let mut editor = SuggestiveEditor::from_buffer(Rc::new(RefCell::new(Buffer::new(
            tree_sitter_md::language(),
            text,
        ))));
        editor.enter_insert_mode();
        editor.set_title(config.title);
        editor.set_completion(Completion {
            items: config.items,
            trigger_characters: vec![" ".to_string()],
        });
        Prompt {
            editor,
            text: "".to_string(),
            owner: config.owner,
            on_enter: config.on_enter,
            on_text_change: config.on_text_change,
        }
    }

    fn set_text(&mut self, text: String) {
        self.text = text;
        self.editor.set_content(&self.text);
    }
}

impl Component for Prompt {
    fn editor(&self) -> &Editor {
        self.editor.editor()
    }
    fn editor_mut(&mut self) -> &mut Editor {
        self.editor.editor_mut()
    }
    fn handle_event(&mut self, state: &State, event: Event) -> anyhow::Result<Vec<Dispatch>> {
        match event {
            Event::Key(key_event) => match key_event.code {
                KeyCode::Esc if self.editor().mode == Mode::Normal => {
                    return Ok(vec![Dispatch::CloseCurrentWindow {
                        change_focused_to: self.owner.borrow().id(),
                    }]);
                }
                KeyCode::Enter => {
                    let current_item = if self.editor.dropdown_opened() {
                        self.editor
                            .current_item()
                            .map(|item| item.label())
                            .unwrap_or(String::new())
                    } else {
                        self.text.clone()
                    };
                    let dispatches = (self.on_enter)(&current_item, self.owner.clone())?;
                    return Ok(vec![Dispatch::CloseCurrentWindow {
                        change_focused_to: self.owner.borrow().id(),
                    }]
                    .into_iter()
                    .chain(dispatches)
                    .collect());
                }
                _ => {}
            },
            _ => {}
        };

        let dispatches = self.editor.handle_event(state, event)?;

        let current_text = self
            .editor()
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
        self.editor.children()
    }
}
