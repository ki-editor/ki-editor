use crossterm::event::{Event, KeyCode};

use crate::{
    component::Component,
    engine::{Editor, Mode},
    screen::{ComponentId, Dispatch, State},
};

#[derive(Clone)]
pub struct Prompt {
    editor: Editor,
    owner_id: ComponentId,
}

pub struct PromptConfig {
    pub owner_id: ComponentId,
    pub history: Vec<String>,
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
        log::info!("Prompt text: {}", text);
        let mut editor = Editor::from_text(tree_sitter_md::language(), text);
        editor.enter_insert_mode();
        Prompt {
            editor,
            owner_id: config.owner_id,
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
                            search: self.editor.get_line().trim().to_string(),
                        },
                        Dispatch::CloseCurrentWindow {
                            change_focused_to: self.owner_id,
                        },
                    ]
                }
                _ => {}
            },
            _ => {}
        };
        self.editor
            .handle_event(state, event)
            .into_iter()
            .chain(vec![Dispatch::Search {
                component_id: self.owner_id,
                search: self.editor.get_line().trim().to_string(),
            }])
            .collect()
    }
}
