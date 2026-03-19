use event::KeyEvent;
use my_proc_macros::key;

use crate::{
    app::{Dispatch, Dispatches},
    components::{editor::DispatchEditor, keymap_legend::KeymapLegendConfig},
    context::Context,
    keymap_override::KeymapOverrideTrait,
};

#[derive(Debug, Clone, PartialEq)]
pub struct MenuKeymapOverride {
    config: KeymapLegendConfig,
}

impl KeymapOverrideTrait for MenuKeymapOverride {
    fn handle_press(
        &mut self,
        context: &Context,
        key_event: KeyEvent,
    ) -> anyhow::Result<Dispatches> {
        let close_dispatches = Dispatches::from(vec![
            Dispatch::CloseKeymapLegend,
            Dispatch::ToEditor(DispatchEditor::SetKeymapOverride(None)),
        ]);
        if key_event == key!("esc") {
            return Ok(close_dispatches);
        }

        let key_event = context
            .keyboard_layout()
            .translate_key_event_to_qwerty(key_event);
        Ok(match self.config.keymap.get(&key_event) {
            Some(binding) => close_dispatches.chain(binding.get_dispatches()),
            None => Dispatches::default(),
        })
    }
}

impl MenuKeymapOverride {
    pub fn new(config: KeymapLegendConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &KeymapLegendConfig {
        &self.config
    }
}
