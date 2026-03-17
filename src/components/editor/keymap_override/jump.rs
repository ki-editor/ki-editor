use crossterm::event::KeyCode;
use itertools::Itertools as _;

use crate::{
    app::{Dispatch, Dispatches},
    components::editor::{
        keymap_override::KeymapOverrideTrait, DispatchEditor, Editor, Jump, Movement,
    },
    context::Context,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JumpKeymapOverride {
    pub jumps: Vec<Jump>,
}

impl KeymapOverrideTrait for JumpKeymapOverride {
    fn handle_press(
        &mut self,
        context: &Context,
        key_event: event::KeyEvent,
    ) -> anyhow::Result<Dispatches> {
        let c = match key_event.code {
            KeyCode::Char(c) => c,
            KeyCode::Esc => {
                return Ok(Dispatches::one(Dispatch::ToEditor(
                    DispatchEditor::SetKeymapOverride(None),
                )))
            }
            _ => return Ok(Dispatches::default()),
        };

        let matching_jumps = self
            .jumps
            .iter()
            .filter(|jump| c == jump.character)
            .collect_vec();
        Ok(match matching_jumps.split_first() {
            None => Dispatches::default(),
            Some((jump, [])) => Dispatches::from(vec![
                Dispatch::ToEditor(DispatchEditor::SetKeymapOverride(None)),
                Dispatch::ToEditor(DispatchEditor::MoveSelection(Movement::Jump(
                    jump.selection.range(),
                ))),
            ]),
            Some(_) => {
                self.jumps = matching_jumps
                    .into_iter()
                    .zip(Editor::jump_characters(context).into_iter().cycle())
                    .map(|(jump, character)| Jump {
                        character,
                        ..jump.clone()
                    })
                    .collect_vec();
                Dispatches::default()
            }
        })
    }
}
