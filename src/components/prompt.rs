use std::{cell::RefCell, rc::Rc};

use itertools::Itertools;
use my_proc_macros::key;

use crate::{
    app::{Dispatch, DispatchPrompt},
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
    owner_id: Option<ComponentId>,
    on_enter: DispatchPrompt,
    enter_selects_first_matching_item: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromptConfig {
    pub history: Vec<String>,
    pub on_enter: DispatchPrompt,
    pub items: Vec<CompletionItem>,
    pub title: String,
    pub enter_selects_first_matching_item: bool,
}

impl Prompt {
    pub fn new(config: PromptConfig, owner_id: Option<ComponentId>) -> (Self, Vec<Dispatch>) {
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
        let dispatches = editor.render();
        (
            Prompt {
                editor,
                on_enter: config.on_enter,
                enter_selects_first_matching_item: config.enter_selects_first_matching_item,
                owner_id,
            },
            dispatches,
        )
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
                    change_focused_to: self.owner_id,
                }])
            }
            key!("tab") => {
                if self.editor.completion_dropdown_opened() {
                    if let Some(item) = self.editor.completion_dropdown_current_item() {
                        self.editor.set_content(&item.label())?;
                        return self.editor_mut().move_to_line_end();
                    }
                    Ok(Vec::new())
                } else {
                    return self.editor_mut().handle_key_event(context, event);
                }
            }
            key!("enter") => {
                let current_item = if self.enter_selects_first_matching_item
                    && self.editor.completion_dropdown_opened()
                {
                    self.editor
                        .completion_dropdown_current_item()
                        .map(|item| item.label())
                        .unwrap_or_default()
                } else {
                    self.editor().current_line()?
                };

                let dispatches = self.on_enter.to_dispatches(&current_item)?;

                Ok(vec![Dispatch::CloseCurrentWindow {
                    change_focused_to: self.owner_id,
                }]
                .into_iter()
                .chain(dispatches)
                .collect_vec())
            }
            _ => self.editor.handle_key_event(context, event),
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
    use crate::test_app::test_app::*;
    use my_proc_macros::keys;

    use crate::{app::Dispatch, position::Position};

    use super::*;

    #[test]
    fn enter_selects_first_matching_item() {
        fn run_test(
            enter_selects_first_matching_item: bool,
            input_text: &str,
            expected_invoked_text: &'static str,
        ) {
            execute_test(|s| {
                Box::new([
                    App(OpenFile(s.main_rs())),
                    App(OpenPrompt(super::PromptConfig {
                        history: vec![],
                        on_enter: DispatchPrompt::SetContent,
                        items: [
                            CompletionItem::from_label("foo".to_string()),
                            CompletionItem::from_label("bar".to_string()),
                        ]
                        .to_vec(),

                        title: "".to_string(),
                        enter_selects_first_matching_item,
                    })),
                    Expect(CompletionDropdownIsOpen(true)),
                    App(HandleKeyEvents(
                        event::parse_key_events(input_text).unwrap(),
                    )),
                    App(HandleKeyEvent(key!("enter"))),
                    Expect(CurrentComponentContent(expected_invoked_text)),
                ])
            })
            .unwrap()
        }

        run_test(true, "f", "foo");
        run_test(false, "f", "f");
    }

    #[test]
    fn tab_replace_current_content_with_first_highlighted_suggestion() -> anyhow::Result<()> {
        execute_test(|_| {
            Box::new([
                App(Dispatch::OpenPrompt(super::PromptConfig {
                    history: vec![],
                    on_enter: DispatchPrompt::SetContent,
                    items: [CompletionItem::from_label("foo_bar".to_string())].to_vec(),

                    title: "".to_string(),
                    enter_selects_first_matching_item: true,
                })),
                App(HandleKeyEvents(keys!("f o o _ b tab").to_vec())),
                Expect(CurrentComponentContent("foo_bar")),
                Expect(EditorCursorPosition(Position { line: 0, column: 7 })),
            ])
        })
    }
}
