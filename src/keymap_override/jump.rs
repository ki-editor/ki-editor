use crossterm::event::KeyCode;
use itertools::Itertools as _;

use crate::{
    app::{Dispatch, Dispatches},
    components::{
        editor::{DispatchEditor, Editor, Jump, Movement},
        editor_keymap::CombinedKeyEvent,
    },
    context::Context,
    keymap_override::KeymapOverrideTrait,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JumpLandingAction {
    MoveSelection,
    DeleteWithMovement,
    CutWithMovement,
    Eat,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JumpKeymapOverride {
    pub jumps: Vec<Jump>,
    pub landing_action: JumpLandingAction,
}

impl KeymapOverrideTrait for JumpKeymapOverride {
    fn handle_press(
        &mut self,
        context: &Context,
        key_event: CombinedKeyEvent,
    ) -> anyhow::Result<Dispatches> {
        let c = match key_event.original.code {
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
            Some((jump, [])) => {
                let movement = Movement::Jump(jump.selection.range());
                let landing_dispatch = match self.landing_action {
                    JumpLandingAction::MoveSelection => {
                        Dispatch::ToEditor(DispatchEditor::MoveSelection(movement))
                    }
                    JumpLandingAction::DeleteWithMovement => {
                        Dispatch::ToEditor(DispatchEditor::DeleteWithMovement(movement))
                    }
                    JumpLandingAction::CutWithMovement => {
                        Dispatch::ToEditor(DispatchEditor::CutWithMovement(movement))
                    }
                    JumpLandingAction::Eat => Dispatch::ToEditor(DispatchEditor::Eat(movement)),
                };
                Dispatches::from(vec![
                    Dispatch::ToEditor(DispatchEditor::SetKeymapOverride(None)),
                    landing_dispatch,
                ])
            }
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
