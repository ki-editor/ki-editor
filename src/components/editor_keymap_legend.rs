use crossterm::event::KeyCode;

use event::KeyEventKind;

#[cfg(test)]
use crate::keymap::insert_mode_keymap_legend_config;
use crate::{
    app::{Dispatch, Dispatches},
    components::editor_keymap::CombinedKeyEvent,
    keymap::keymap_universal,
};

#[cfg(test)]
use super::editor::Mode;
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

/// Mirrors the classification in `Editor::handle_insert_mode`: true unless `event`
/// is Insert-mode literal text input that bypasses the insert-mode keymap.
/// Keep in sync with `handle_insert_mode` if its fallback logic changes.
///
/// Only used by doc recipe generation (`generate_recipes.rs`) and its own tests,
/// both of which are `#[cfg(test)]`-only, hence this is too.
#[cfg(test)]
pub fn is_positional_key_event(mode: Mode, event: &CombinedKeyEvent) -> bool {
    mode != Mode::Insert
        || insert_mode_keymap_legend_config(true)
            .keymap()
            .iter()
            .any(|keymap| keymap.event() == &event.translated)
}

#[cfg(test)]
mod test_is_positional_key_event {
    use my_proc_macros::key;

    use super::{is_positional_key_event, CombinedKeyEvent, Mode};

    fn combined(event: event::KeyEvent) -> CombinedKeyEvent {
        CombinedKeyEvent {
            original: event,
            translated: event,
        }
    }

    #[test]
    fn normal_mode_key_is_always_positional() {
        assert!(is_positional_key_event(Mode::Normal, &combined(key!("s"))));
    }

    #[test]
    fn insert_mode_literal_text_is_not_positional() {
        assert!(!is_positional_key_event(Mode::Insert, &combined(key!("s"))));
    }

    #[test]
    fn insert_mode_keymap_binding_is_positional() {
        assert!(is_positional_key_event(
            Mode::Insert,
            &combined(key!("esc"))
        ));
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
