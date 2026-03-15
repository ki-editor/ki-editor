use event::KeyEvent;

use crate::{
    app::Dispatches,
    components::editor::keymap_override::{
        find_one::FindOneCharKeymapOverride, jump::JumpKeymapOverride,
    },
    context::Context,
};

pub mod find_one;
pub mod jump;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeymapOverride {
    Jumps(JumpKeymapOverride),
    FindOneChar(FindOneCharKeymapOverride),
}

pub trait KeymapOverrideTrait {
    fn handle_press(
        &mut self,
        context: &Context,
        key_event: KeyEvent,
    ) -> anyhow::Result<Dispatches>;

    fn handle_release(
        &mut self,
        context: &Context,
        key_event: KeyEvent,
    ) -> anyhow::Result<Dispatches> {
        let _ = (context, key_event);
        Ok(Dispatches::default())
    }
}

impl KeymapOverride {
    fn inner(&mut self) -> &mut dyn KeymapOverrideTrait {
        match self {
            KeymapOverride::Jumps(jump_keymap_override) => jump_keymap_override,
            KeymapOverride::FindOneChar(find_one_char_keymap_override) => {
                find_one_char_keymap_override
            }
        }
    }
}

impl KeymapOverrideTrait for KeymapOverride {
    fn handle_press(
        &mut self,
        context: &Context,
        key_event: KeyEvent,
    ) -> anyhow::Result<Dispatches> {
        self.inner().handle_press(context, key_event)
    }

    fn handle_release(
        &mut self,
        context: &Context,
        key_event: KeyEvent,
    ) -> anyhow::Result<Dispatches> {
        self.inner().handle_release(context, key_event)
    }
}
