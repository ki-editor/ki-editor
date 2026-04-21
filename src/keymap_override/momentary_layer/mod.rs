use event::KeyEvent;
use my_proc_macros::key;

use crate::{
    app::{Dispatch, Dispatches},
    components::keymap_legend::{KeymapLegendConfig, OnTap, ReleaseKey},
    context::Context,
    keymap_override::{
        momentary_layer::{joint::JointMomentaryLayer, simple::SimpleMomentaryLayer},
        KeymapOverrideScope, KeymapOverrideTrait,
    },
};

mod joint;
mod simple;

trait MomentaryLayerBaseTrait {
    fn handle_press(&mut self, key_event: KeyEvent) -> anyhow::Result<(bool, Dispatches)>;
    fn tap(&mut self) -> anyhow::Result<Dispatches>;
}

#[derive(Debug, Clone, PartialEq)]
enum MomentaryLayerBase {
    Simple(SimpleMomentaryLayer),
    Joint(JointMomentaryLayer),
}

impl MomentaryLayerBase {
    fn inner(&mut self) -> &mut dyn MomentaryLayerBaseTrait {
        match self {
            MomentaryLayerBase::Simple(simple_momentary_layer) => simple_momentary_layer,
            MomentaryLayerBase::Joint(joint_momentary_layer) => joint_momentary_layer,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MomentaryLayerKeymapOverride {
    override_scope: KeymapOverrideScope,
    base: MomentaryLayerBase,
    release_key: KeyEvent,
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
        let (key_press, dispatches) = self.base.inner().handle_press(key_event)?;
        self.other_keys_pressed |= key_press;
        Ok(dispatches)
    }

    fn handle_release(
        &mut self,
        context: &Context,
        key_event: KeyEvent,
    ) -> anyhow::Result<Dispatches> {
        let key_event = context
            .keyboard_layout()
            .translate_key_event_to_qwerty(key_event);

        Ok(if self.release_key == key_event {
            let dispatches = self.close_dispatches();

            if !self.other_keys_pressed {
                dispatches.chain(self.base.inner().tap()?)
            } else {
                dispatches
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
            base: MomentaryLayerBase::Simple(SimpleMomentaryLayer {
                config,
                tap: release_key.on_tap().cloned(),
            }),
            release_key: release_key.key_event().clone(),
            other_keys_pressed: false,
        }
    }

    pub fn new_joint(
        override_scope: KeymapOverrideScope,
        release_key: ReleaseKey,
        swap_key: KeyEvent,
        active_config: KeymapLegendConfig,
        inactive_config: KeymapLegendConfig,
        inactive_tap: Option<OnTap>,
    ) -> Self {
        Self {
            override_scope,
            base: MomentaryLayerBase::Joint(JointMomentaryLayer {
                tap_key: release_key.key(),
                active: SimpleMomentaryLayer {
                    config: active_config,
                    tap: release_key.on_tap().cloned(),
                },
                inactive: SimpleMomentaryLayer {
                    config: inactive_config,
                    tap: inactive_tap.clone(),
                },
                swap_key,
            }),
            release_key: release_key.key_event().clone(),
            other_keys_pressed: false,
        }
    }

    fn close_dispatches(&self) -> Dispatches {
        Dispatches::one(match self.override_scope {
            KeymapOverrideScope::App => Dispatch::CloseAppKeymapLegend,
            KeymapOverrideScope::Editor => Dispatch::CloseKeymapLegend,
        })
    }
}
