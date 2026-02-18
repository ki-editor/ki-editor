use event::{parse_key_event, KeyEvent};

use itertools::Itertools;
use my_proc_macros::key;

use crate::{
    app::{Dispatch, Dispatches},
    components::editor_keymap_printer::KeymapDisplayOption,
    context::Context,
    rectangle::Rectangle,
};

use super::{
    component::Component,
    editor::{Direction, Editor, Mode},
    editor_keymap_printer::KeymapPrintSection,
};

pub struct KeymapLegend {
    editor: Editor,
    config: KeymapLegendConfig,
    release_key: Option<ParsedReleaseKey>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KeymapLegendConfig {
    pub title: String,
    pub keymap: Keymap,
}

pub struct MomentaryLayer {
    pub key: &'static str,
    pub description: String,
    pub config: KeymapLegendConfig,
    pub on_tap: Option<OnTap>,
    pub on_spacebar_tapped: Option<OnSpacebarTapped>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum OnSpacebarTapped {
    DeactivatesMomentaryLaterAndOpenMenu(KeymapLegendConfig),
}
impl OnSpacebarTapped {
    fn description(&self) -> String {
        match self {
            OnSpacebarTapped::DeactivatesMomentaryLaterAndOpenMenu(keymap_legend_config) => {
                keymap_legend_config.title.clone()
            }
        }
    }
}

struct ParsedReleaseKey {
    key: &'static str,
    key_event: KeyEvent,
    on_tap: Option<OnTap>,
    on_spacebar_tapped: Option<OnSpacebarTapped>,
    other_keys_pressed: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReleaseKey {
    key: &'static str,
    on_tap: Option<OnTap>,
    on_spacebar_tapped: Option<OnSpacebarTapped>,
    /// Initial value should be false.
    /// This field is necessary for implementing `on_tap`.
    other_keys_pressed: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OnTap {
    description: &'static str,
    dispatches: Dispatches,
}

impl OnTap {
    pub fn new(description: &'static str, dispatches: Dispatches) -> Self {
        Self {
            description,
            dispatches,
        }
    }
}

impl ReleaseKey {
    pub fn new(
        key: &'static str,
        on_tap: Option<OnTap>,
        on_spacebar_tapped: Option<OnSpacebarTapped>,
    ) -> Self {
        Self {
            key,
            on_tap,
            other_keys_pressed: false,
            on_spacebar_tapped,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Keymap(Vec<Keybinding>);
impl Keymap {
    fn display(&self, terminal_width: usize, option: &KeymapDisplayOption) -> String {
        KeymapPrintSection::from_keymap("".to_string(), self).display(terminal_width, option)
    }
    pub fn new(keybindings: &[Keybinding]) -> Self {
        Self(keybindings.to_vec())
    }

    pub fn get(&self, event: &KeyEvent) -> std::option::Option<&Keybinding> {
        self.0.iter().find(|key| &key.event == event)
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Keybinding> {
        self.0.iter()
    }

    pub fn into_vec(self) -> Vec<Keybinding> {
        self.0
    }
}

impl KeymapLegendConfig {
    pub fn display(&self, width: usize, option: &KeymapDisplayOption) -> String {
        self.keymap.display(width, option)
    }

    pub fn keymap(&self) -> Keymap {
        let keymap = &self.keymap;
        #[cfg(test)]
        {
            let conflicting_keybindings = keymap
                .iter()
                .chunk_by(|keymap| keymap.key)
                .into_iter()
                .map(|(key, keybindings)| (key, keybindings.collect_vec()))
                .filter(|(_, keybindings)| keybindings.len() > 1)
                .collect_vec();

            if !conflicting_keybindings.is_empty() {
                panic!("Conflicting keybindings detected:\n\n{conflicting_keybindings:#?}");
            }
        }
        keymap.clone()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Keybinding {
    key: &'static str,
    pub short_description: Option<String>,
    pub description: String,
    event: KeyEvent,
    dispatch: Dispatch,
}

impl Keybinding {
    pub fn new(key: &'static str, description: String, dispatch: Dispatch) -> Keybinding {
        Keybinding {
            key,
            short_description: None,
            description,
            dispatch,
            event: parse_key_event(key).unwrap(),
        }
    }

    pub fn momentary_layer(
        MomentaryLayer {
            key,
            description,
            config,
            on_tap,
            on_spacebar_tapped,
        }: MomentaryLayer,
    ) -> Keybinding {
        Keybinding {
            key,
            short_description: None,
            description,
            dispatch: Dispatch::ShowKeymapLegendWithReleaseKey(
                config,
                ReleaseKey::new(key, on_tap, on_spacebar_tapped),
            ),
            event: parse_key_event(key).unwrap(),
        }
    }

    pub fn new_extended(
        key: &'static str,
        short_description: String,
        description: String,
        dispatch: Dispatch,
    ) -> Keybinding {
        Keybinding {
            key,
            short_description: Some(short_description),
            description,
            dispatch,
            event: parse_key_event(key).unwrap(),
        }
    }

    pub fn get_dispatches(&self) -> Dispatches {
        Dispatches::one(self.dispatch.clone()).append(Dispatch::SetLastActionDescription {
            long_description: self.description.clone(),
            short_description: self.short_description.clone(),
        })
    }

    pub fn event(&self) -> &KeyEvent {
        &self.event
    }

    pub fn override_keymap(
        self,
        keymap_override: Option<&super::editor_keymap_legend::KeymapOverride>,
        none_if_no_override: bool,
    ) -> Option<Keybinding> {
        match keymap_override {
            Some(keymap_override) => Some(Self {
                short_description: Some(keymap_override.description.to_string()),
                description: keymap_override.description.to_string(),
                dispatch: keymap_override.dispatch.clone(),
                ..self
            }),
            None => {
                if none_if_no_override {
                    None
                } else {
                    Some(self)
                }
            }
        }
    }

    pub fn display(&self) -> String {
        self.short_description
            .clone()
            .unwrap_or_else(|| self.description.clone())
    }
}

impl KeymapLegend {
    pub fn new(
        config: KeymapLegendConfig,
        context: &Context,
        release_key: Option<ReleaseKey>,
    ) -> KeymapLegend {
        // Check for duplicate keys
        let duplicates = config
            .keymap()
            .0
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
            log::info!("{message}");
            // panic!("{}", message);
        }

        let mut editor = Editor::from_text(None, "");
        editor.set_title(config.title.clone());
        let _ = editor
            .enter_insert_mode(Direction::End, context)
            .unwrap_or_default();

        let release_key = release_key.map(|release_key| ParsedReleaseKey {
            key: release_key.key,
            key_event: KeyEvent {
                kind: crossterm::event::KeyEventKind::Release,
                ..parse_key_event(release_key.key).unwrap()
            },
            on_tap: release_key.on_tap,
            other_keys_pressed: release_key.other_keys_pressed,
            on_spacebar_tapped: release_key.on_spacebar_tapped,
        });

        KeymapLegend {
            editor,
            config,
            release_key,
        }
    }

    fn refresh(&mut self, context: &Context) {
        let content = self.display();
        self.editor_mut()
            .set_content(&content, context)
            .unwrap_or_default();
    }

    fn display(&self) -> String {
        let content = self.config.display(
            self.editor.rectangle().width,
            &KeymapDisplayOption {
                show_alt: true,
                show_shift: true,
            },
        );
        let content = if let Some((false, on_tap)) =
            self.release_key.as_ref().and_then(|release_key| {
                Some((release_key.other_keys_pressed, release_key.on_tap.clone()?))
            }) {
            format!("{content}\nRelease hold: {}", on_tap.description)
        } else {
            content
        };

        if let Some((false, on_spacebar_tapped)) =
            self.release_key.as_ref().and_then(|release_key| {
                Some((
                    release_key.other_keys_pressed,
                    release_key.on_spacebar_tapped.clone()?,
                ))
            })
        {
            format!("{content}\nSpace: {}", on_spacebar_tapped.description())
        } else {
            content
        }
    }
}

impl Component for KeymapLegend {
    fn editor(&self) -> &Editor {
        &self.editor
    }

    fn set_rectangle(&mut self, rectangle: Rectangle, context: &Context) {
        self.refresh(context); // TODO: pass theme from App.rs
        self.editor_mut().set_rectangle(rectangle, context);
    }

    fn editor_mut(&mut self) -> &mut Editor {
        &mut self.editor
    }

    fn handle_key_event(
        &mut self,
        context: &crate::context::Context,
        event: event::KeyEvent,
    ) -> Result<Dispatches, anyhow::Error> {
        let close_current_window = Dispatch::CloseCurrentWindowAndFocusParent;
        if self.editor.mode == Mode::Insert {
            match event {
                key!("esc") => {
                    self.editor.enter_normal_mode(context)?;
                    Ok(Dispatches::default())
                }
                key_event => {
                    let key_event = context
                        .keyboard_layout_kind()
                        .translate_key_event_to_qwerty(key_event.clone());
                    if let Some(keymap) = self
                        .config
                        .keymap()
                        .iter()
                        .find(|keymap| keymap.event == key_event)
                    {
                        Ok(Dispatches::one(close_current_window)
                            .chain(keymap.get_dispatches())
                            // If release key is enabled, it means that this is a momentary layer,
                            // so we should keep showing the menu even after some keys are executed.
                            .append_some(self.release_key.as_ref().map(
                                |release_key| -> Dispatch {
                                    Dispatch::ShowKeymapLegendWithReleaseKey(
                                        self.config.clone(),
                                        ReleaseKey {
                                            key: release_key.key,
                                            on_tap: release_key.on_tap.clone(),
                                            other_keys_pressed: true,
                                            on_spacebar_tapped: release_key
                                                .on_spacebar_tapped
                                                .clone(),
                                        },
                                    )
                                },
                            )))
                    } else if let Some(release_key) = &self.release_key {
                        if let (Some(on_spacebar_tapped), crossterm::event::KeyCode::Char(' ')) =
                            (&release_key.on_spacebar_tapped, key_event.code)
                        {
                            match on_spacebar_tapped {
                                OnSpacebarTapped::DeactivatesMomentaryLaterAndOpenMenu(
                                    keymap_legend_config,
                                ) => Ok(Dispatches::one(close_current_window).append(
                                    Dispatch::ShowKeymapLegend(keymap_legend_config.clone()),
                                )),
                            }
                        } else if release_key.key_event == key_event {
                            let on_tap_dispatches =
                                match (&release_key.on_tap, release_key.other_keys_pressed) {
                                    (Some(on_tap), false) => on_tap.dispatches.clone(),
                                    _ => Dispatches::default(),
                                };
                            Ok(Dispatches::one(close_current_window).chain(on_tap_dispatches))
                        } else {
                            Ok(vec![].into())
                        }
                    } else {
                        Ok(vec![].into())
                    }
                }
            }
        } else if self.editor.mode == Mode::Normal && event == key!("esc") {
            Ok([close_current_window].to_vec().into())
        } else {
            self.editor.handle_key_event(context, event)
        }
    }
}

#[cfg(test)]
mod test_keymap_legend {
    use super::*;
    use crate::{
        app::Dimension, buffer::BufferOwner, components::editor::DispatchEditor,
        position::Position, test_app::*,
    };
    use crossterm::event::KeyEventKind;
    use my_proc_macros::keys;

    #[test]
    fn test_esc() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                App(ShowKeymapLegend(KeymapLegendConfig {
                    title: "".to_string(),
                    keymap: Keymap::new(&[]),
                })),
                App(HandleKeyEvent(key!("esc"))),
                App(HandleKeyEvent(key!("esc"))),
                Expect(CurrentComponentPath(Some(s.main_rs()))),
            ])
        })
    }

    #[test]
    fn test_display_positional_full() {
        let keymap = Keymap(
            [
                Keybinding::new("a", "Aloha".to_string(), Dispatch::Null),
                Keybinding::new("b", "Bomb".to_string(), Dispatch::Null),
                Keybinding::new("F", "Foo".to_string(), Dispatch::Null),
                Keybinding::new("c", "Caterpillar".to_string(), Dispatch::Null),
                Keybinding::new("alt+g", "Gogagg".to_string(), Dispatch::Null),
            ]
            .to_vec(),
        );
        let actual = keymap
            .display(
                100,
                &KeymapDisplayOption {
                    show_alt: false,
                    show_shift: false,
                },
            )
            .to_string();
        let expected = r#"
╭───────┬───┬─────────────┬───┬──────┬───┬───┬───┬───┬───┬───╮
│       ┆   ┆             ┆   ┆      ┆ ∅ ┆   ┆   ┆   ┆   ┆   │
├╌╌╌╌╌╌╌┼╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌┼╌╌╌╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┤
│ Aloha ┆   ┆             ┆   ┆      ┆ ∅ ┆   ┆   ┆   ┆   ┆   │
├╌╌╌╌╌╌╌┼╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌┼╌╌╌╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┤
│       ┆   ┆ Caterpillar ┆   ┆ Bomb ┆ ∅ ┆   ┆   ┆   ┆   ┆   │
╰───────┴───┴─────────────┴───┴──────┴───┴───┴───┴───┴───┴───╯
* Pick Keyboard    \ Leader
"#
        .trim_matches('\n');
        assert_eq!(actual, expected);

        let actual = keymap
            .display(
                100,
                &KeymapDisplayOption {
                    show_alt: true,
                    show_shift: true,
                },
            )
            .to_string()
            .trim_matches('\n')
            .to_string();
        let expected = r#"
╭───────┬───┬─────────────┬─────┬────────┬───┬───┬───┬───┬───┬───╮
│       ┆   ┆             ┆     ┆        ┆ ∅ ┆   ┆   ┆   ┆   ┆   │
├╌╌╌╌╌╌╌┼╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌┼╌╌╌╌╌╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┤
│       ┆   ┆             ┆     ┆ Gogagg ┆ ⌥ ┆   ┆   ┆   ┆   ┆   │
│       ┆   ┆             ┆ Foo ┆        ┆ ⇧ ┆   ┆   ┆   ┆   ┆   │
│ Aloha ┆   ┆             ┆     ┆        ┆ ∅ ┆   ┆   ┆   ┆   ┆   │
├╌╌╌╌╌╌╌┼╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌┼╌╌╌╌╌╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┤
│       ┆   ┆ Caterpillar ┆     ┆  Bomb  ┆ ∅ ┆   ┆   ┆   ┆   ┆   │
╰───────┴───┴─────────────┴─────┴────────┴───┴───┴───┴───┴───┴───╯
* Pick Keyboard    \ Leader"#
            .trim_matches('\n');
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_display_positional_stacked() {
        let keymap = Keymap(
            [
                Keybinding::new("a", "Aloha".to_string(), Dispatch::Null),
                Keybinding::new("b", "Bomb".to_string(), Dispatch::Null),
                Keybinding::new("F", "Foo".to_string(), Dispatch::Null),
                Keybinding::new("c", "Caterpillar".to_string(), Dispatch::Null),
                Keybinding::new("alt+g", "Gogagg".to_string(), Dispatch::Null),
                Keybinding::new("alt+l", "Lamp".to_string(), Dispatch::Null),
            ]
            .to_vec(),
        );
        let actual = keymap
            .display(
                50,
                &KeymapDisplayOption {
                    show_alt: true,
                    show_shift: true,
                },
            )
            .to_string();
        let expected = r#"
╭───────┬───┬─────────────┬─────┬────────┬───╮
│       ┆   ┆             ┆     ┆        ┆ ∅ │
├╌╌╌╌╌╌╌┼╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌┼╌╌╌╌╌╌╌╌┼╌╌╌┤
│       ┆   ┆             ┆     ┆ Gogagg ┆ ⌥ │
│       ┆   ┆             ┆ Foo ┆        ┆ ⇧ │
│ Aloha ┆   ┆             ┆     ┆        ┆ ∅ │
├╌╌╌╌╌╌╌┼╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌┼╌╌╌╌╌╌╌╌┼╌╌╌┤
│       ┆   ┆ Caterpillar ┆     ┆  Bomb  ┆ ∅ │
╰───────┴───┴─────────────┴─────┴────────┴───╯
╭───┬───┬───┬───┬──────┬───╮
│ ∅ ┆   ┆   ┆   ┆      ┆   │
├╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌╌╌╌┼╌╌╌┤
│ ⌥ ┆   ┆   ┆   ┆ Lamp ┆   │
│ ∅ ┆   ┆   ┆   ┆      ┆   │
├╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌╌╌╌┼╌╌╌┤
│ ∅ ┆   ┆   ┆   ┆      ┆   │
╰───┴───┴───┴───┴──────┴───╯
* Pick Keyboard    \ Leader"#
            .trim();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_display_positional_too_small() {
        let keymap = Keymap(
            [
                Keybinding::new("a", "Aloha".to_string(), Dispatch::Null),
                Keybinding::new("b", "Bomb".to_string(), Dispatch::Null),
                Keybinding::new("F", "Foo".to_string(), Dispatch::Null),
                Keybinding::new("c", "Caterpillar".to_string(), Dispatch::Null),
                Keybinding::new("alt+g", "Gogagg".to_string(), Dispatch::Null),
                Keybinding::new("alt+l", "Lamp".to_string(), Dispatch::Null),
            ]
            .to_vec(),
        );
        let actual = keymap
            .display(
                10,
                &KeymapDisplayOption {
                    show_alt: true,
                    show_shift: true,
                },
            )
            .to_string();
        let expected = "Window is too small to display keymap legend :(";
        assert_eq!(actual, expected);
    }

    #[test]
    fn should_intercept_key_event_defined_in_config() {
        let mut keymap_legend = KeymapLegend::new(
            KeymapLegendConfig {
                title: "Test".to_string(),
                keymap: Keymap::new(&[Keybinding::new(
                    "s",
                    "fifafofum".to_string(),
                    Dispatch::Custom("Spongebob".to_string()),
                )]),
            },
            &Context::default(),
            None,
        );

        let dispatches = keymap_legend.handle_events(keys!("s")).unwrap();

        assert_eq!(
            dispatches,
            Dispatches::new(vec![
                Dispatch::CloseCurrentWindowAndFocusParent,
                Dispatch::Custom("Spongebob".to_string()),
                SetLastActionDescription {
                    long_description: "fifafofum".to_string(),
                    short_description: None
                }
            ])
        );
    }

    #[test]
    /// When release key is defined and on tap is defined, display should show the on tap action.
    fn on_tap_display() {
        let mut keymap_legend = KeymapLegend::new(
            KeymapLegendConfig {
                title: "".to_string(),
                keymap: Keymap::new(&[]),
            },
            &Context::default(),
            Some(ReleaseKey::new(
                "Y",
                Some(OnTap::new("Conichihuahua", Dispatches::default())),
                None,
            )),
        );

        let _ = keymap_legend
            .handle_dispatch_editor(
                &mut Context::default(),
                DispatchEditor::SetRectangle(Rectangle {
                    origin: Position::default(),
                    width: 100,
                    height: 100,
                }),
            )
            .unwrap();

        assert_eq!(
            keymap_legend.display(),
            "
╭───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───╮
│   ┆   ┆   ┆   ┆   ┆ ∅ ┆   ┆   ┆   ┆   ┆   │
├╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┤
│   ┆   ┆   ┆   ┆   ┆ ∅ ┆   ┆   ┆   ┆   ┆   │
├╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┤
│   ┆   ┆   ┆   ┆   ┆ ∅ ┆   ┆   ┆   ┆   ┆   │
╰───┴───┴───┴───┴───┴───┴───┴───┴───┴───┴───╯
* Pick Keyboard    \\ Leader
Release hold: Conichihuahua
"
            .trim()
        );
    }

    #[test]
    /// When release key is defined and the release key is immediately received
    /// before any actions in the keymap is executed, the on tap dispatches should be fired.
    fn release_key_on_tap() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("".to_string())),
                App(ShowKeymapLegendWithReleaseKey(
                    KeymapLegendConfig {
                        title: "LEGEND_TITLE".to_string(),
                        keymap: Keymap::new(&[]),
                    },
                    ReleaseKey::new(
                        "b",
                        Some(OnTap::new(
                            "",
                            Dispatches::one(Dispatch::ToEditor(SetContent(
                                "on tapped!".to_string(),
                            ))),
                        )),
                        None,
                    ),
                )),
                // Expect the keymap legend is opened
                Expect(AppGridContains("LEGEND_TITLE")),
                // Simulate key release
                App(HandleKeyEvent(
                    parse_key_event("b")
                        .unwrap()
                        .set_event_kind(KeyEventKind::Release),
                )),
                Expect(CurrentComponentContent("on tapped!")),
            ])
        })
    }

    #[test]
    fn when_release_key_is_defined_legend_should_show_until_release_key_is_received(
    ) -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("".to_string())),
                App(ShowKeymapLegendWithReleaseKey(
                    KeymapLegendConfig {
                        title: "LEGEND_TITLE".to_string(),
                        keymap: Keymap::new(&[Keybinding::new(
                            "x",
                            "".to_string(),
                            Dispatch::ToEditor(Insert("hello".to_string())),
                        )]),
                    },
                    ReleaseKey::new("b", None, None),
                )),
                // Expect the keymap legend is opened
                Expect(AppGridContains("LEGEND_TITLE")),
                // Execute an action defined in the keymap
                App(HandleKeyEvent(key!("x"))),
                // Expect the keymap legend is still opened
                Expect(AppGridContains("LEGEND_TITLE")),
                // Simulate key release
                App(HandleKeyEvent(
                    parse_key_event("b")
                        .unwrap()
                        .set_event_kind(KeyEventKind::Release),
                )),
                // Expect the legend is closed
                Expect(Not(Box::new(AppGridContains("LEGEND_TITLE")))),
                // Expect the action defined in the keymap is actually executed
                Expect(CurrentComponentContent("hello")),
            ])
        })
    }

    #[test]
    fn on_spacebar_tapped() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("".to_string())),
                App(ShowKeymapLegendWithReleaseKey(
                    KeymapLegendConfig {
                        title: "MOL_INIT".to_string(),
                        keymap: Keymap::new(&[]),
                    },
                    ReleaseKey::new(
                        "b",
                        None,
                        Some(OnSpacebarTapped::DeactivatesMomentaryLaterAndOpenMenu(
                            KeymapLegendConfig {
                                title: "MOL_SPACE_MENU".to_string(),
                                keymap: Keymap::new(&[]),
                            },
                        )),
                    ),
                )),
                // Expect the keymap legend is opened
                App(Dispatch::TerminalDimensionChanged(Dimension {
                    height: 100,
                    width: 100,
                })),
                Expect(AppGridContains("MOL_INIT")),
                // Expect a legend indicating pressing space will open the MOL_SPACE_MENU,
                Expect(AppGridContains("MOL_SPACE_MENU")),
                // Press "space"
                App(HandleKeyEvent(key!("space"))),
                // Expect the momentary layer is closed
                Expect(Not(Box::new(AppGridContains("LEGEND_TITLE")))),
                // Expect the momentary layer space menu
                Expect(AppGridContains("MOL_SPACE_MENU")),
            ])
        })
    }
}
