use crate::{
    app::{Dispatch, Dispatches},
    components::{
        editor::{DispatchEditor, IfCurrentNotFound},
        editor_keymap::CombinedKeyEvent,
    },
    context::{Context, LocalSearchConfigMode, Search},
    keymap_override::KeymapOverrideTrait,
    list::grep::RegexConfig,
    selection::SelectionMode,
};
use crossterm::event::KeyCode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindOneCharKeymapOverride {
    pub if_current_not_found: IfCurrentNotFound,
}

impl KeymapOverrideTrait for FindOneCharKeymapOverride {
    fn handle_press(
        &mut self,
        _context: &Context,
        key_event: CombinedKeyEvent,
    ) -> anyhow::Result<Dispatches> {
        match key_event.original.code {
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
