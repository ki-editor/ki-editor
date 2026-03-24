use event::KeyEvent;

use crate::{
    app::Dispatches,
    context::Context,
    keymap_override::{
        find_one::FindOneCharKeymapOverride, jump::JumpKeymapOverride, menu::MenuKeymapOverride,
        momentary_layer::MomentaryLayerKeymapOverride,
    },
};

pub mod find_one;
pub mod jump;
pub mod menu;
pub mod momentary_layer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeymapOverrideScope {
    App,
    Editor,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppKeymapOverride {
    MomentaryLayer(MomentaryLayerKeymapOverride),
}

#[derive(Debug, Clone, PartialEq)]
pub enum EditorKeymapOverride {
    Jumps(JumpKeymapOverride),
    FindOneChar(FindOneCharKeymapOverride),
    Menu(MenuKeymapOverride),
    MomentaryLayer(MomentaryLayerKeymapOverride),
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

impl AppKeymapOverride {
    fn inner(&mut self) -> &mut dyn KeymapOverrideTrait {
        match self {
            Self::MomentaryLayer(momentary_layer_keymap_override) => {
                momentary_layer_keymap_override
            }
        }
    }
}

impl KeymapOverrideTrait for AppKeymapOverride {
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

impl EditorKeymapOverride {
    fn inner(&mut self) -> &mut dyn KeymapOverrideTrait {
        match self {
            Self::Jumps(jump_keymap_override) => jump_keymap_override,
            Self::FindOneChar(find_one_char_keymap_override) => find_one_char_keymap_override,
            Self::Menu(menu_keymap_override) => menu_keymap_override,
            Self::MomentaryLayer(momentary_layer_keymap_override) => {
                momentary_layer_keymap_override
            }
        }
    }
}

impl KeymapOverrideTrait for EditorKeymapOverride {
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
