use crossterm::event::KeyCode;

use event::{KeyEvent, KeyEventKind};

use crate::{
    app::{Dispatch, Dispatches},
    context::Context,
    keymap::keymap_universal,
};

use super::{editor::Editor, keymap_legend::Keymap};

impl Editor {
    pub fn handle_insert_mode(
        &self,
        context: &Context,
        event: KeyEvent,
    ) -> anyhow::Result<Dispatches> {
        let translated_event = context
            .keyboard_layout()
            .translate_key_event_to_qwerty(event.clone());
        if let Some(dispatches) = self
            .insert_mode_keymap(true)
            .iter()
            .find(|keymap| keymap.event() == &translated_event)
            .map(|keymap| keymap.get_dispatches())
        {
            Ok(dispatches)
        } else if let (KeyCode::Char(c), KeyEventKind::Press) = (event.code, event.kind) {
            Ok(Dispatches::one(Dispatch::ToEditor(
                super::editor::DispatchEditor::InsertChar(c),
            )))
        } else {
            Ok(Dispatches::default())
        }
    }

    pub fn handle_universal_key(&self, event: &KeyEvent) -> anyhow::Result<Option<Dispatches>> {
        if let Some(keymap) = Keymap::new(&keymap_universal()).get(event) {
            Ok(Some(keymap.get_dispatches()))
        } else {
            Ok(None)
        }
    }
}

#[derive(Default, Clone)]
pub struct NormalModeOverride {
    pub change: Option<KeymapOverride>,
    pub delete: Option<KeymapOverride>,
    pub insert: Option<KeymapOverride>,
    pub append: Option<KeymapOverride>,
    pub open: Option<KeymapOverride>,
    pub paste: Option<KeymapOverride>,
    pub cut: Option<KeymapOverride>,
    pub multicursor: Option<KeymapOverride>,
}

#[derive(Clone)]
pub struct KeymapOverride {
    pub description: &'static str,
    pub dispatch: Dispatch,
}
