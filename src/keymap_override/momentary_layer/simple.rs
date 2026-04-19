use event::KeyEvent;

use crate::{
    app::Dispatches,
    components::keymap_legend::{Keybinding, KeymapLegendConfig, OnTap},
    keymap_override::momentary_layer::MomentaryLayerBaseTrait,
};

#[derive(Debug, Clone, PartialEq)]
pub(super) struct SimpleMomentaryLayer {
    pub(super) config: KeymapLegendConfig,
    pub(super) tap: Option<OnTap>,
}

impl MomentaryLayerBaseTrait for SimpleMomentaryLayer {
    fn handle_press(&mut self, key_event: KeyEvent) -> anyhow::Result<(bool, Dispatches)> {
        Ok(
            match self
                .config
                .keymap
                .get(&key_event)
                .map(Keybinding::get_dispatches)
            {
                Some(dispatches) => (true, dispatches),
                None => (false, Dispatches::default()),
            },
        )
    }

    fn tap(&mut self) -> anyhow::Result<Dispatches> {
        Ok(self
            .tap
            .as_ref()
            .map(OnTap::dispatch)
            .cloned()
            .map(Dispatches::one)
            .unwrap_or_default())
    }
}
