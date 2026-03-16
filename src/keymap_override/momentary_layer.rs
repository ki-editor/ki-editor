use event::KeyEvent;
use my_proc_macros::key;

use crate::{
    app::{Dispatch, Dispatches},
    components::keymap_legend::{Keybinding, KeymapLegendConfig, ReleaseKey},
    context::Context,
    keymap_override::{KeymapOverrideScope, KeymapOverrideTrait},
};

#[derive(Debug, Clone, PartialEq)]
pub struct MomentaryLayerKeymapOverride {
    override_scope: KeymapOverrideScope,
    config: KeymapLegendConfig,
    release_key: ReleaseKey,
    other_keys_pressed: bool,
}

impl KeymapOverrideTrait for MomentaryLayerKeymapOverride {
    fn handle_press(
        &mut self,
        context: &Context,
        key_event: KeyEvent,
    ) -> anyhow::Result<Dispatches> {
        if key_event == key!("esc") {
            return Ok(self.close_dispatches());
        }
        let key_event = context
            .keyboard_layout()
            .translate_key_event_to_qwerty(key_event);
        Ok(
            match self
                .config
                .keymap
                .get(&key_event)
                .map(Keybinding::get_dispatches)
            {
                Some(dispatches) => {
                    self.other_keys_pressed = true;
                    dispatches
                }
                None => Dispatches::default(),
            },
        )
    }

    fn handle_release(
        &mut self,
        context: &Context,
        key_event: KeyEvent,
    ) -> anyhow::Result<Dispatches> {
        let key_event = context
            .keyboard_layout()
            .translate_key_event_to_qwerty(key_event);

        Ok(if self.release_key.key_event() == &key_event {
            let dispatches = self.close_dispatches();

            match self.release_key.on_tap() {
                Some(on_tap) if !self.other_keys_pressed => {
                    dispatches.append(on_tap.dispatch().clone())
                }
                _ => dispatches,
            }
        } else {
            Dispatches::default()
        })
    }
}

impl MomentaryLayerKeymapOverride {
    pub fn new(
        override_scope: KeymapOverrideScope,
        config: KeymapLegendConfig,
        release_key: ReleaseKey,
    ) -> Self {
        Self {
            override_scope,
            config,
            release_key,
            other_keys_pressed: false,
        }
    }

    fn close_dispatches(&self) -> Dispatches {
        Dispatches::one(match self.override_scope {
            KeymapOverrideScope::App => Dispatch::CloseAppKeymapLegend,
            KeymapOverrideScope::Editor => Dispatch::CloseKeymapLegend,
        })
    }

    pub fn config(&self) -> &KeymapLegendConfig {
        &self.config
    }
}
