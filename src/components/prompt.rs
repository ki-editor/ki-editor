use std::{cell::RefCell, rc::Rc};

use itertools::Itertools;
use my_proc_macros::key;

use crate::{
    app::Dispatch,
    buffer::Buffer,
    context::Context,
    lsp::completion::{Completion, CompletionItem},
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
    enter_selects_first_matching_item: bool,
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
    pub history: Vec<String>,
    pub owner: Option<Rc<RefCell<dyn Component>>>,
    pub on_enter: OnEnter,
    pub on_text_change: OnTextChange,
    pub items: Vec<CompletionItem>,
    pub title: String,
    pub enter_selects_first_matching_item: bool,
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
        log::info!("Prompt.text = {text}");
        let mut editor = SuggestiveEditor::from_buffer(
            Rc::new(RefCell::new(Buffer::new(tree_sitter_md::language(), text))),
            SuggestiveEditorFilter::CurrentLine,
        );
        editor.enter_insert_mode().unwrap_or_default();
        // TODO: set cursor to last line
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
            enter_selects_first_matching_item: config.enter_selects_first_matching_item,
        }
    }
    fn text(&self) -> &str {
        &self.text
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
        context: &Context,
        event: event::KeyEvent,
    ) -> anyhow::Result<Vec<Dispatch>> {
        match event {
            key!("esc") if self.editor().mode == Mode::Normal => {
                Ok(vec![Dispatch::CloseCurrentWindow {
                    change_focused_to: self.owner.clone().map(|owner| owner.borrow().id()),
                }])
            }
            key!("tab") => {
                if self.editor.dropdown_opened() {
                    if let Some(item) = self.editor.current_item() {
                        self.text = item.label();
                        self.editor.set_content(&self.text)?;
                        self.editor_mut().move_to_line_end(context)?;
                    }
                    Ok(Vec::new())
                } else {
                    return self.editor_mut().handle_key_event(context, event);
                }
            }
            key!("enter") => {
                let current_item =
                    if self.enter_selects_first_matching_item && self.editor.dropdown_opened() {
                        self.editor
                            .current_item()
                            .map(|item| item.label())
                            .unwrap_or_default()
                    } else {
                        self.text.clone()
                    };

                let dispatches = (self.on_enter)(&current_item, self.owner.clone())?;

                Ok(vec![Dispatch::CloseCurrentWindow {
                    change_focused_to: self.owner.clone().map(|owner| owner.borrow().id()),
                }]
                .into_iter()
                .chain(dispatches)
                .collect_vec())
            }
            _ => {
                let dispatches = self.editor.handle_key_event(context, event)?;

                let current_text = self.editor().current_line()?;

                let result = if current_text == self.text {
                    dispatches
                } else {
                    self.text = current_text.clone();

                    let text_change_dispatches = if let Some(owner) = self.owner.clone() {
                        (self.on_text_change)(&current_text, owner.clone())?
                    } else {
                        Default::default()
                    };

                    dispatches
                        .into_iter()
                        .chain(text_change_dispatches)
                        .collect_vec()
                };
                Ok(result)
            }
        }
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
        app::Dispatch,
        components::{component::Component, prompt::Prompt},
        position::Position,
    };

    use super::*;

    #[test]
    fn enter_selects_first_matching_item() {
        fn run_test(
            enter_selects_first_matching_item: bool,
            input_text: &str,
            expected_invoked_text: &str,
        ) {
            let mut prompt = Prompt::new(super::PromptConfig {
                history: vec![],
                owner: None,
                on_enter: Box::new(|text, _| Ok(vec![Dispatch::Custom(text.to_string())])),
                on_text_change: Box::new(|_, _| Ok(vec![])),
                items: [
                    CompletionItem::from_label("foo".to_string()),
                    CompletionItem::from_label("bar".to_string()),
                ]
                .to_vec(),

                title: "".to_string(),
                enter_selects_first_matching_item,
            });
            prompt
                .handle_events(&event::parse_key_events(input_text).unwrap())
                .unwrap();

            let dispatches = prompt.handle_events(keys!("enter")).unwrap();

            assert!(dispatches
                .iter()
                .any(|dispatch| matches!(dispatch, Dispatch::Custom(text) if text == expected_invoked_text)));
        }

        run_test(true, "f", "foo");
        run_test(false, "f", "f");
    }

    #[test]
    fn tab_replace_current_content_with_first_highlighted_suggestion() -> anyhow::Result<()> {
        let mut prompt = Prompt::new(super::PromptConfig {
            history: vec![],
            owner: None,
            on_enter: Box::new(|text, _| Ok(vec![Dispatch::Custom(text.to_string())])),
            on_text_change: Box::new(|_, _| Ok(vec![])),
            items: [CompletionItem::from_label("foo_bar".to_string())].to_vec(),

            title: "".to_string(),
            enter_selects_first_matching_item: true,
        });
        prompt.handle_events(&event::parse_key_events("f o o _ b")?)?;

        prompt.handle_events(keys!("tab"))?;
        assert_eq!(prompt.text(), "foo_bar");
        assert_eq!(prompt.content(), prompt.text());
        assert_eq!(
            prompt.editor().get_cursor_position()?,
            Position { line: 0, column: 7 }
        );
        Ok(())
    }

    #[test]
    fn should_return_custom_dispatches_regardless_of_owner_id() {
        fn run_test(owner: Option<Rc<RefCell<dyn Component>>>) {
            let mut prompt = Prompt::new(super::PromptConfig {
                history: vec![],
                owner,
                on_enter: Box::new(|_, _| Ok(vec![Dispatch::Custom("haha".to_string())])),
                on_text_change: Box::new(|_, _| Ok(vec![])),
                items: vec![],
                title: "".to_string(),
                enter_selects_first_matching_item: false,
            });

            let dispatches = prompt.handle_events(keys!("enter")).unwrap();

            assert!(dispatches
                .iter()
                .any(|dispatch| matches!(dispatch, Dispatch::Custom(text) if text == "haha")));
        }

        run_test(None);

        run_test(Some(Rc::new(RefCell::new(Editor::from_text(
            tree_sitter_md::language(),
            "",
        )))))
    }
}
