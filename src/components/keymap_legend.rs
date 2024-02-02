use event::{parse_key_event, KeyEvent};
use itertools::Itertools;
use my_proc_macros::key;

use crate::app::Dispatch;

use super::{
    component::{Component, ComponentId},
    editor::{Direction, Editor, Mode},
};

pub struct KeymapLegend {
    editor: Editor,
    config: KeymapLegendConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeymapLegendConfig {
    pub title: String,
    pub body: KeymapLegendBody,
    pub owner_id: ComponentId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeymapLegendBody {
    SingleSection { keymaps: Keymaps },
    MultipleSections { sections: Vec<KeymapLegendSection> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Keymaps(Vec<Keymap>);
impl Keymaps {
    fn display(&self, indent: usize) -> String {
        let width = self
            .0
            .iter()
            .map(|keymap| keymap.key.len())
            .max()
            .unwrap_or(0);

        let margin = 2;

        // Align the keys columns and the dispatch columns
        self.0
            .iter()
            .sorted_by_key(|keymap| keymap.key.to_lowercase())
            .map(|keymap| {
                format!(
                    "{}{:<width$} {}",
                    " ".repeat(indent),
                    keymap.key,
                    keymap.description,
                    width = width + margin
                )
            })
            .join("\n")
    }

    pub fn new(keymaps: &[Keymap]) -> Self {
        Self(keymaps.to_vec())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeymapLegendSection {
    pub title: String,
    pub keymaps: Keymaps,
}

impl KeymapLegendSection {
    fn display(&self) -> String {
        format!("{}\n{}", self.title, self.keymaps.display(2))
    }
}

impl KeymapLegendBody {
    fn display(&self) -> String {
        match self {
            KeymapLegendBody::SingleSection { keymaps } => keymaps.display(0),
            KeymapLegendBody::MultipleSections { sections } => sections
                .iter()
                .map(|section| section.display())
                .join("\n\n"),
        }
    }

    fn keymaps(&self) -> Vec<&Keymap> {
        match self {
            KeymapLegendBody::SingleSection { keymaps } => keymaps.0.iter().collect_vec(),
            KeymapLegendBody::MultipleSections { sections } => sections
                .iter()
                .flat_map(|section| section.keymaps.0.iter())
                .collect_vec(),
        }
    }
}

impl KeymapLegendConfig {
    fn display(&self) -> String {
        self.body.display()
    }

    fn keymaps(&self) -> Vec<&Keymap> {
        self.body.keymaps()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Keymap {
    key: &'static str,
    description: String,
    event: KeyEvent,
    dispatch: Dispatch,
}

impl Keymap {
    pub fn new(key: &'static str, description: String, dispatch: Dispatch) -> Keymap {
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
            .keymaps()
            .into_iter()
            .duplicates_by(|keymap| keymap.key)
            .collect_vec();

        if !duplicates.is_empty() {
            let message = format!(
                "Duplicate keymap keys for {}: {:#?}",
                config.title,
                duplicates
                    .into_iter()
                    .map(|duplicate| format!("{}: {}", duplicate.key, duplicate.description))
                    .collect_vec()
            );
            log::info!("{}", message);
            // panic!("{}", message);
        }

        let content = config.display();
        let mut editor = Editor::from_text(tree_sitter_md::language(), &content);
        editor.set_title(config.title.clone());
        editor.enter_insert_mode(Direction::End);
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
        context: &crate::context::Context,
        event: event::KeyEvent,
    ) -> anyhow::Result<Vec<crate::app::Dispatch>> {
        let close_current_window = Dispatch::CloseCurrentWindow {
            change_focused_to: Some(self.config.owner_id),
        };
        if self.editor.mode == Mode::Insert {
            match &event {
                key!("esc") => {
                    self.editor.enter_normal_mode()?;
                    Ok(vec![])
                }
                key_event => {
                    if let Some(keymap) = self
                        .config
                        .keymaps()
                        .iter()
                        .find(|keymap| &keymap.event == key_event)
                    {
                        Ok([close_current_window]
                            .into_iter()
                            .chain(vec![keymap.dispatch.clone()])
                            .collect())
                    } else {
                        Ok(vec![])
                    }
                }
            }
        } else if self.editor.mode == Mode::Normal && event == key!("esc") {
            Ok([close_current_window].to_vec())
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
            title: "Test".to_string(),
            body: KeymapLegendBody::SingleSection {
                keymaps: Keymaps::new(&[Keymap::new(
                    "s",
                    "test".to_string(),
                    Dispatch::Custom("Spongebob".to_string()),
                )]),
            },
            owner_id,
        });

        let dispatches = keymap_legend.handle_events(keys!("s")).unwrap();

        assert_eq!(
            dispatches,
            vec![
                Dispatch::CloseCurrentWindow {
                    change_focused_to: Some(owner_id)
                },
                Dispatch::Custom("Spongebob".to_string())
            ]
        )
    }
}
