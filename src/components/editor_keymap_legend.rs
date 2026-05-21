use crossterm::event::KeyCode;

use event::KeyEventKind;

use crate::{
    app::{Dispatch, Dispatches},
    components::editor_keymap::CombinedKeyEvent,
    keymap::keymap_universal,
};

use super::{editor::Editor, keymap_legend::Keymap};

impl Editor {
    pub fn handle_insert_mode(&self, event: CombinedKeyEvent) -> anyhow::Result<Dispatches> {
        if let Some(dispatches) = self
            .insert_mode_keymap(true)
            .iter()
            .find(|keymap| keymap.event() == &event.translated)
            .map(|keymap| keymap.get_dispatches())
        {
            Ok(dispatches)
        } else if let (KeyCode::Char(c), KeyEventKind::Press) =
            (event.original.code, event.original.kind)
        {
            Ok(Dispatches::one(Dispatch::ToEditor(
                super::editor::DispatchEditor::InsertChar(c),
            )))
        } else {
            Ok(Dispatches::default())
        }
    }

    pub fn handle_universal_key(
        &self,
        event: &CombinedKeyEvent,
    ) -> anyhow::Result<Option<Dispatches>> {
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
