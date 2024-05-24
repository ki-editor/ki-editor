use std::{cell::RefCell, rc::Rc};

use my_proc_macros::key;

use crate::{
    app::{Dispatch, DispatchPrompt, Dispatches},
    buffer::Buffer,
    components::editor::{self, DispatchEditor},
    context::Context,
    lsp::completion::Completion,
};

use super::{
    component::Component,
    dropdown::DropdownItem,
    editor::{Editor, Mode},
    suggestive_editor::{SuggestiveEditor, SuggestiveEditorFilter},
};

pub(crate) struct Prompt {
    editor: SuggestiveEditor,
    /// This will only be run on user input if the user input matches no dropdown item.
    on_enter: DispatchPrompt,
    enter_selects_first_matching_item: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PromptConfig {
    pub(crate) history: Vec<String>,
    pub(crate) on_enter: DispatchPrompt,
    pub(crate) items: Vec<DropdownItem>,
    pub(crate) title: String,
    pub(crate) enter_selects_first_matching_item: bool,
    pub(crate) leaves_current_line_empty: bool,
}

impl Prompt {
    pub(crate) fn new(config: PromptConfig) -> (Self, Dispatches) {
        let text = {
            let mut history = config.history.clone();
            history.reverse();
            format!(
                "{}{}",
                if config.leaves_current_line_empty {
                    "\n"
                } else {
                    ""
                },
                history.join("\n")
            )
        };
        let mut editor = SuggestiveEditor::from_buffer(
            Rc::new(RefCell::new(Buffer::new(None, &text))),
            SuggestiveEditorFilter::CurrentLine,
        );
        let dispatches = Dispatches::one(Dispatch::ToEditor(if config.leaves_current_line_empty {
            DispatchEditor::EnterInsertMode(editor::Direction::Start)
        } else {
            DispatchEditor::MoveToLineEnd
        }));
        // TODO: set cursor to last line
        editor.set_title(config.title);
        editor.set_completion(Completion {
            items: config.items,
            trigger_characters: vec![" ".to_string()],
        });
        let dispatches = dispatches.append(editor.render_completion_dropdown(true));
        (
            Prompt {
                editor,
                on_enter: config.on_enter,
                enter_selects_first_matching_item: config.enter_selects_first_matching_item,
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
    fn handle_dispatch_editor(
        &mut self,
        context: &mut Context,
        dispatch: super::editor::DispatchEditor,
    ) -> anyhow::Result<Dispatches> {
        self.editor.handle_dispatch_editor(context, dispatch)
    }
    fn handle_key_event(
        &mut self,
        context: &Context,
        event: event::KeyEvent,
    ) -> anyhow::Result<Dispatches> {
        match event {
            key!("esc") if self.editor().mode == Mode::Normal => {
                Ok(vec![Dispatch::CloseCurrentWindow].into())
            }
            key!("ctrl+space") => {
                if self.editor.completion_dropdown_opened() {
                    if let Some(item) = self.editor.completion_dropdown_current_item() {
                        self.editor.set_content(&item.display())?;
                        return self.editor_mut().move_to_line_end();
                    }
                    Ok(Default::default())
                } else {
                    self.editor_mut().handle_key_event(context, event)
                }
            }
            key!("enter") => {
                let dispatches = if self.enter_selects_first_matching_item
                    && self.editor.completion_dropdown_current_item().is_some()
                {
                    self.editor
                        .completion_dropdown_current_item()
                        .map(|item| item.dispatches)
                        .unwrap_or_default()
                } else {
                    self.on_enter
                        .to_dispatches(&self.editor().current_line()?)?
                };

                Ok(Dispatches::one(Dispatch::CloseCurrentWindow).chain(dispatches))
            }
            _ => self.editor.handle_key_event(context, event),
        }
    }
}

#[cfg(test)]
mod test_prompt {
    use crate::{components::editor::Direction, lsp::completion::CompletionItem, test_app::*};
    use my_proc_macros::keys;

    use crate::{app::Dispatch, position::Position};

    use super::*;

    #[test]
    fn leaves_current_line_empty() {
        fn test(
            leaves_current_line_empty: bool,
            expected_text: &'static str,
            expected_cursor_position: Position,
        ) {
            execute_test(|s| {
                Box::new([
                    App(OpenFile(s.main_rs())),
                    App(OpenPrompt(PromptConfig {
                        history: ["hello".to_string()].to_vec(),
                        on_enter: DispatchPrompt::Null,
                        items: Default::default(),
                        title: "".to_string(),
                        enter_selects_first_matching_item: true,
                        leaves_current_line_empty,
                    })),
                    Expect(CurrentComponentContent(expected_text)),
                    Expect(EditorCursorPosition(expected_cursor_position)),
                ])
            })
            .unwrap();
        }
        test(true, "\nhello", Position::default());
        test(false, "hello", Position::new(0, 5));
    }

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
                        items: ["foo".to_string(), "bar".to_string()]
                            .into_iter()
                            .map(|str| {
                                let item: DropdownItem = str.clone().into();
                                item.set_dispatches(Dispatches::one(ToEditor(SetContent(str))))
                            })
                            .collect(),

                        title: "".to_string(),
                        enter_selects_first_matching_item,
                        leaves_current_line_empty: true,
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
    fn ctrl_space_replace_current_content_with_first_highlighted_suggestion() -> anyhow::Result<()>
    {
        execute_test(|_| {
            Box::new([
                App(Dispatch::OpenPrompt(super::PromptConfig {
                    history: vec![],
                    on_enter: DispatchPrompt::SetContent,
                    items: ["foo_bar".to_string()]
                        .into_iter()
                        .map(|item| item.into())
                        .collect(),

                    title: "".to_string(),
                    enter_selects_first_matching_item: true,
                    leaves_current_line_empty: true,
                })),
                App(HandleKeyEvents(keys!("f o o _ b ctrl+space").to_vec())),
                Expect(CurrentComponentContent("foo_bar")),
                Expect(EditorCursorPosition(Position { line: 0, column: 7 })),
            ])
        })
    }

    #[test]
    fn filter_without_matching_items_should_clear_suggestions() -> anyhow::Result<()> {
        execute_test(|_| {
            Box::new([
                App(Dispatch::OpenPrompt(super::PromptConfig {
                    history: vec![],
                    on_enter: DispatchPrompt::SetContent,
                    items: [CompletionItem::from_label(
                        "spongebob squarepants".to_string(),
                    )]
                    .into_iter()
                    .map(|item| item.into())
                    .collect(),

                    title: "".to_string(),
                    enter_selects_first_matching_item: true,
                    leaves_current_line_empty: true,
                })),
                App(TerminalDimensionChanged(crate::app::Dimension {
                    height: 10,
                    width: 50,
                })),
                Expect(AppGridContains("squarepants")),
                App(HandleKeyEvents(keys!("f o o").to_vec())),
                Expect(Not(Box::new(AppGridContains("squarepants")))),
            ])
        })
    }

    #[test]
    fn suggestion_should_update_with_ctrl_k_and_ctrl_u() -> Result<(), anyhow::Error> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("".to_string())),
                Editor(EnterInsertMode(Direction::Start)),
                App(Dispatch::OpenPrompt(super::PromptConfig {
                    history: vec![],
                    on_enter: DispatchPrompt::SetContent,
                    items: ["Patrick", "Spongebob", "Squidward"]
                        .into_iter()
                        .map(|item| item.to_string().into())
                        .collect(),

                    title: "".to_string(),
                    enter_selects_first_matching_item: true,
                    leaves_current_line_empty: true,
                })),
                // Expect the completion dropdown to be open,
                Expect(CompletionDropdownContent("Patrick\nSpongebob\nSquidward")),
                // Type in 'pa'
                App(HandleKeyEvents(keys!("p a").to_vec())),
                // Expect only 'Patrick' remains in the completion dropdown
                Expect(CompletionDropdownContent("Patrick")),
                // Clear 'pa' using ctrl+u
                App(HandleKeyEvent(key!("ctrl+u"))),
                // Expect all items are shown again
                Expect(CompletionDropdownContent("Patrick\nSpongebob\nSquidward")),
                //
                //
                // Perform the same test for ctrl+k
                App(HandleKeyEvents(keys!("p a").to_vec())),
                Expect(CompletionDropdownContent("Patrick")),
                App(HandleKeyEvents(keys!("ctrl+a ctrl+k").to_vec())),
                Expect(CompletionDropdownContent("Patrick\nSpongebob\nSquidward")),
            ])
        })
    }
}
