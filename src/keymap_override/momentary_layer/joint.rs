use event::KeyEvent;

use crate::{
    app::{Dispatch, Dispatches},
    components::keymap_legend::ReleaseKey,
};

use super::{simple::SimpleMomentaryLayer, MomentaryLayerBaseTrait};

#[derive(Debug, Clone, PartialEq)]
pub(super) struct JointMomentaryLayer {
    pub(super) tap_key: &'static str,
    pub(super) swap_key: KeyEvent,
    pub(super) active: SimpleMomentaryLayer,
    pub(super) inactive: SimpleMomentaryLayer,
}

impl MomentaryLayerBaseTrait for JointMomentaryLayer {
    fn handle_press(&mut self, key_event: KeyEvent) -> anyhow::Result<(bool, Dispatches)> {
        if key_event == self.swap_key {
            std::mem::swap(&mut self.active, &mut self.inactive);
            Ok((
                false,
                Dispatches::one(Dispatch::ShowKeymapLegend {
                    on_root: false,
                    keymap_legend_config: self.active.config.clone(),
                    release_key: Some(ReleaseKey::new(self.tap_key, self.active.tap.clone())),
                }),
            ))
        } else {
            self.active.handle_press(key_event)
        }
    }

    fn tap(&mut self) -> anyhow::Result<Dispatches> {
        self.active.tap()
    }
}
