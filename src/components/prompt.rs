use std::{cell::RefCell, rc::Rc};

use itertools::Itertools;
use my_proc_macros::key;

use crate::{
    buffer::Buffer,
    context::Context,
    lsp::completion::{Completion, CompletionItem},
    screen::Dispatch,
};

use super::{
    component::{Component, ComponentId},
    editor::{Editor, Mode},
    suggestive_editor::{SuggestiveEditor, SuggestiveEditorFilter},
};

pub struct Prompt {
    editor: SuggestiveEditor,
    text: String,
    owner: Option<Rc<RefCell<dyn Component>>>,
    on_enter: OnEnter,
    on_text_change: OnTextChange,
}

type OnEnter = Box<
    dyn Fn(
        /* current_suggestion */ &str,
        /*owner*/ Option<Rc<RefCell<dyn Component>>>,
    ) -> anyhow::Result<Vec<Dispatch>>,
>;

type OnTextChange = Box<
    dyn Fn(
        /* text */ &str,
        /*owner*/ Rc<RefCell<dyn Component>>,
    ) -> anyhow::Result<Vec<Dispatch>>,
>;

pub struct PromptConfig {
    pub initial_text: Option<String>,
    pub history: Vec<String>,
    pub owner: Option<Rc<RefCell<dyn Component>>>,
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
        let mut editor = SuggestiveEditor::from_buffer(
            Rc::new(RefCell::new(Buffer::new(tree_sitter_md::language(), text))),
            SuggestiveEditorFilter::CurrentLine,
        );
        editor.set_content(&config.initial_text.unwrap_or_default());
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
}

impl Component for Prompt {
    fn editor(&self) -> &Editor {
        self.editor.editor()
    }
    fn editor_mut(&mut self) -> &mut Editor {
        self.editor.editor_mut()
    }
    fn handle_key_event(
        &mut self,
        context: &mut Context,
        event: event::KeyEvent,
    ) -> anyhow::Result<Vec<Dispatch>> {
        match event {
            key!("esc") if self.editor().mode == Mode::Normal => {
                return Ok(vec![Dispatch::CloseCurrentWindow {
                    change_focused_to: self.owner.clone().map(|owner| owner.borrow().id()),
                }])
            }
            key!("enter") => {
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
                    change_focused_to: self.owner.clone().map(|owner| owner.borrow().id()),
                }]
                .into_iter()
                .chain(dispatches)
                .collect_vec());
            }
            _ => {}
        };

        let dispatches = self.editor.handle_key_event(context, event)?;

        let current_text = self.editor().current_line()?;

        let result = if current_text == self.text {
            dispatches
        } else {
            self.text = current_text.clone();

            if let Some(owner) = self.owner.clone() {
                (self.on_text_change)(&current_text, owner.clone())?;
            }

            dispatches.into_iter().chain(vec![]).collect_vec()
        };
        Ok(result)
    }

    fn children(&self) -> Vec<Option<Rc<RefCell<dyn Component>>>> {
        self.editor.children()
    }

    fn remove_child(&mut self, id: ComponentId) {
        self.editor.remove_child(id)
    }
}

#[cfg(test)]
mod test_prompt {
    use my_proc_macros::keys;
    use std::{cell::RefCell, rc::Rc};

    use crate::{
        components::{component::Component, prompt::Prompt},
        screen::Dispatch,
    };

    use super::*;

    #[test]
    fn should_return_custom_dispatches_regardless_of_owner_id() {
        fn run_test(owner: Option<Rc<RefCell<dyn Component>>>) {
            let mut prompt = Prompt::new(super::PromptConfig {
                history: vec![],
                initial_text: None,
                owner,
                on_enter: Box::new(|_, _| Ok(vec![Dispatch::Custom("haha")])),
                on_text_change: Box::new(|_, _| Ok(vec![])),
                items: vec![],
                title: "".to_string(),
            });

            let dispatches = prompt.handle_events(keys!("enter")).unwrap();

            assert!(dispatches
                .iter()
                .any(|dispatch| matches!(dispatch, Dispatch::Custom("haha"))));
        }

        run_test(None);

        run_test(Some(Rc::new(RefCell::new(Editor::from_text(
            tree_sitter_md::language(),
            "",
        )))))
    }
}
