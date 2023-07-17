use event::{parse_key_event, KeyEvent};
use itertools::Itertools;
use my_proc_macros::key;

use crate::screen::Dispatch;

use super::{
    component::{Component, ComponentId},
    editor::{CursorDirection, Editor, Mode},
};

pub struct KeymapLegend {
    editor: Editor,
    config: KeymapLegendConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeymapLegendConfig {
    pub title: &'static str,
    pub keymaps: Vec<Keymap>,
    pub owner_id: ComponentId,
}
impl KeymapLegendConfig {
    fn display(&self) -> String {
        let width = self
            .keymaps
            .iter()
            .map(|keymap| keymap.key.len())
            .max()
            .unwrap_or(0);

        let margin = 2;

        // Align the keys columns and the dispatch columns
        self.keymaps
            .iter()
            .map(|keymap| {
                format!(
                    "{:<width$} {}",
                    keymap.key,
                    keymap.description,
                    width = width + margin
                )
            })
            .join("\n")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Keymap {
    key: &'static str,
    description: &'static str,
    event: KeyEvent,
    dispatch: Dispatch,
}

impl Keymap {
    pub fn new(key: &'static str, description: &'static str, dispatch: Dispatch) -> Keymap {
        Keymap {
            key,
            description,
            dispatch,
            event: parse_key_event(key).unwrap(),
        }
    }
}

impl KeymapLegend {
    pub fn new(config: KeymapLegendConfig) -> KeymapLegend {
        // Check for duplicate keys
        let duplicates = config
            .keymaps
            .iter()
            .duplicates_by(|keymap| keymap.key)
            .collect_vec();

        if !duplicates.is_empty() {
            panic!(
                "Duplicate keymap keys for {}: {:#?}",
                config.title,
                duplicates
                    .into_iter()
                    .map(|duplicate| format!("{}: {}", duplicate.key, duplicate.description))
                    .collect_vec()
            );
        }

        let content = config.display();
        let mut editor = Editor::from_text(tree_sitter_md::language(), &content);
        editor.set_title(config.title.into());
        editor.enter_insert_mode(CursorDirection::End);
        KeymapLegend { editor, config }
    }
}

impl Component for KeymapLegend {
    fn editor(&self) -> &Editor {
        &self.editor
    }

    fn editor_mut(&mut self) -> &mut Editor {
        &mut self.editor
    }

    fn handle_key_event(
        &mut self,
        context: &mut crate::context::Context,
        event: event::KeyEvent,
    ) -> anyhow::Result<Vec<crate::screen::Dispatch>> {
        if self.editor.mode == Mode::Insert {
            match &event {
                key!("esc") => {
                    self.editor.enter_normal_mode()?;
                    Ok(vec![])
                }
                key_event => {
                    if let Some(keymap) = self
                        .config
                        .keymaps
                        .iter()
                        .find(|keymap| &keymap.event == key_event)
                    {
                        Ok(vec![Dispatch::CloseCurrentWindow {
                            change_focused_to: Some(self.config.owner_id),
                        }]
                        .into_iter()
                        .chain(vec![keymap.dispatch.clone()])
                        .collect())
                    } else {
                        Ok(vec![])
                    }
                }
            }
        } else {
            self.editor.handle_key_event(context, event)
        }
    }

    fn children(&self) -> Vec<Option<std::rc::Rc<std::cell::RefCell<dyn Component>>>> {
        self.editor.children()
    }

    fn remove_child(&mut self, component_id: super::component::ComponentId) {
        self.editor.remove_child(component_id)
    }
}

#[cfg(test)]
mod test_keymap_legend {
    use my_proc_macros::keys;

    use super::*;
    #[test]
    fn should_intercept_key_event_defined_in_config() {
        let owner_id = ComponentId::new();
        let mut keymap_legend = KeymapLegend::new(KeymapLegendConfig {
            title: "Test",
            keymaps: vec![Keymap::new("s", "test", Dispatch::Custom("Spongebob"))],
            owner_id,
        });

        let dispatches = keymap_legend.handle_events(keys!("s")).unwrap();

        assert_eq!(
            dispatches,
            vec![
                Dispatch::CloseCurrentWindow {
                    change_focused_to: Some(owner_id)
                },
                Dispatch::Custom("Spongebob")
            ]
        )
    }

    #[test]
    fn esc_should_close_the_window() {
        let owner_id = ComponentId::new();
        let mut keymap_legend = KeymapLegend::new(KeymapLegendConfig {
            title: "Test",
            keymaps: vec![],
            owner_id,
        });

        let dispatches = keymap_legend.handle_events(keys!("esc")).unwrap();

        assert_eq!(
            dispatches,
            vec![Dispatch::CloseCurrentWindow {
                change_focused_to: Some(owner_id)
            }]
        )
    }
}
