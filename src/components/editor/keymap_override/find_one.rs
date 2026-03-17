use crossterm::event::KeyCode;
use event::KeyEvent;

use crate::{
    app::{Dispatch, Dispatches},
    components::editor::{keymap_override::KeymapOverrideTrait, DispatchEditor, IfCurrentNotFound},
    context::{Context, LocalSearchConfigMode, Search},
    list::grep::RegexConfig,
    selection::SelectionMode,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindOneCharKeymapOverride {
    pub if_current_not_found: IfCurrentNotFound,
}

impl KeymapOverrideTrait for FindOneCharKeymapOverride {
    fn handle_press(
        &mut self,
        _context: &Context,
        key_event: KeyEvent,
    ) -> anyhow::Result<Dispatches> {
        match key_event.code {
            KeyCode::Esc => Ok(Dispatches::one(Dispatch::ToEditor(
                DispatchEditor::SetKeymapOverride(None),
            ))),
            KeyCode::Char(c) => Ok(Dispatches::from(vec![
                Dispatch::ToEditor(DispatchEditor::SetKeymapOverride(None)),
                Dispatch::ToEditor(DispatchEditor::SetSelectionMode(
                    self.if_current_not_found,
                    SelectionMode::Find {
                        search: Search {
                            search: c.to_string(),
                            mode: LocalSearchConfigMode::Regex(RegexConfig {
                                escaped: true,
                                case_sensitive: true,
                                match_whole_word: false,
                            }),
                        },
                    },
                )),
            ])),
            _ => Ok(Dispatches::default()),
        }
    }
}
