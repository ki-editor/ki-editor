use event::{parse_key_event, KeyEvent};

use itertools::Itertools;
use my_proc_macros::key;

use crate::{
    app::{Dispatch, Dispatches},
    components::{editor_keymap::Meaning, editor_keymap_printer::KeymapDisplayOption},
    context::Context,
    rectangle::Rectangle,
};

use super::{
    component::Component,
    editor::{Direction, Editor, Mode},
    editor_keymap::KeyboardLayoutKind,
    editor_keymap_printer::KeymapPrintSection,
};

pub struct KeymapLegend {
    editor: Editor,
    config: KeymapLegendConfig,
    option: KeymapDisplayOption,
    keymap_layout_kind: super::editor_keymap::KeyboardLayoutKind,
    release_key: Option<ParsedReleaseKey>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KeymapLegendConfig {
    pub title: String,
    pub keymaps: Keymaps,
}

pub struct MomentaryLayer {
    pub meaning: Meaning,
    pub description: String,
    pub config: KeymapLegendConfig,
    pub on_tap: Option<OnTap>,
}

struct ParsedReleaseKey {
    key_event: KeyEvent,
    meaning: Meaning,
    on_tap: Option<OnTap>,
    other_keys_pressed: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReleaseKey {
    meaning: Meaning,
    on_tap: Option<OnTap>,
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
    pub fn new(meaning: Meaning, on_tap: Option<OnTap>) -> Self {
        Self {
            meaning,
            on_tap,
            other_keys_pressed: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Keymaps(Vec<Keymap>);
impl Keymaps {
    fn display(
        &self,
        keyboard_layout_kind: &KeyboardLayoutKind,
        terminal_width: usize,
        option: &KeymapDisplayOption,
    ) -> String {
        KeymapPrintSection::from_keymaps(
            "".to_string(),
            self,
            keyboard_layout_kind.get_keyboard_layout(),
        )
        .display(terminal_width, option)
    }
    pub fn new(keymaps: &[Keymap]) -> Self {
        Self(keymaps.to_vec())
    }

    pub fn get(&self, event: &KeyEvent) -> std::option::Option<&Keymap> {
        self.0.iter().find(|key| &key.event == event)
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Keymap> {
        self.0.iter()
    }

    pub fn into_vec(self) -> Vec<Keymap> {
        self.0
    }
}

impl KeymapLegendConfig {
    pub fn display(
        &self,
        keyboard_layout_kind: &KeyboardLayoutKind,
        width: usize,
        option: &KeymapDisplayOption,
    ) -> String {
        self.keymaps.display(keyboard_layout_kind, width, option)
    }

    pub fn keymaps(&self) -> Keymaps {
        let keymaps = &self.keymaps;
        #[cfg(test)]
        {
            let conflicting_keymaps = keymaps
                .iter()
                .chunk_by(|keymap| keymap.key)
                .into_iter()
                .map(|(key, keymaps)| (key, keymaps.collect_vec()))
                .filter(|(_, keymaps)| keymaps.len() > 1)
                .collect_vec();

            if !conflicting_keymaps.is_empty() {
                panic!("Conflicting keymaps detected:\n\n{conflicting_keymaps:#?}");
            }
        }
        keymaps.clone()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Keymap {
    key: &'static str,
    pub short_description: Option<String>,
    pub description: String,
    event: KeyEvent,
    dispatch: Dispatch,
}

impl Keymap {
    pub fn new(key: &'static str, description: String, dispatch: Dispatch) -> Keymap {
        Keymap {
            key,
            short_description: None,
            description,
            dispatch,
            event: parse_key_event(key).unwrap(),
        }
    }

    pub fn momentary_layer(
        context: &Context,
        MomentaryLayer {
            meaning,
            description,
            config,
            on_tap,
        }: MomentaryLayer,
    ) -> Keymap {
        let key = context.keyboard_layout_kind().get_key(&meaning);
        Keymap {
            key,
            short_description: None,
            description,
            dispatch: Dispatch::ShowKeymapLegendWithReleaseKey(
                config,
                ReleaseKey::new(meaning, on_tap),
            ),
            event: parse_key_event(key).unwrap(),
        }
    }

    pub fn new_extended(
        key: &'static str,
        short_description: String,
        description: String,
        dispatch: Dispatch,
    ) -> Keymap {
        Keymap {
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
    ) -> Option<Keymap> {
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
            .keymaps()
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
            key_event: KeyEvent {
                kind: crossterm::event::KeyEventKind::Release,
                ..parse_key_event(context.keyboard_layout_kind().get_key(&release_key.meaning))
                    .unwrap()
            },
            meaning: release_key.meaning,
            on_tap: release_key.on_tap,
            other_keys_pressed: release_key.other_keys_pressed,
        });

        KeymapLegend {
            editor,
            config,
            option: KeymapDisplayOption {
                show_alt: true,
                show_shift: true,
            },
            keymap_layout_kind: *context.keyboard_layout_kind(),
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
            &self.keymap_layout_kind,
            self.editor.rectangle().width,
            &self.option,
        );
        if let Some((false, on_tap)) = self.release_key.as_ref().and_then(|release_key| {
            Some((release_key.other_keys_pressed, release_key.on_tap.clone()?))
        }) {
            format!("{content}\nRelease hold: {}", on_tap.description)
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
            match &event {
                key!("esc") => {
                    self.editor.enter_normal_mode(context)?;
                    Ok(Default::default())
                }
                key_event => {
                    if let Some(keymap) = self
                        .config
                        .keymaps()
                        .iter()
                        .find(|keymap| &keymap.event == key_event)
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
                                            meaning: release_key.meaning.clone(),
                                            on_tap: release_key.on_tap.clone(),
                                            other_keys_pressed: true,
                                        },
                                    )
                                },
                            )))
                    } else if let Some(release_key) = &self.release_key {
                        if &release_key.key_event == key_event {
                            let on_tap_dispatches =
                                match (&release_key.on_tap, release_key.other_keys_pressed) {
                                    (Some(on_tap), false) => on_tap.dispatches.clone(),
                                    _ => Default::default(),
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
    use crate::{buffer::BufferOwner, components::editor::DispatchEditor, test_app::*};
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
                    keymaps: Keymaps::new(&[]),
                })),
                App(HandleKeyEvent(key!("esc"))),
                App(HandleKeyEvent(key!("esc"))),
                Expect(CurrentComponentPath(Some(s.main_rs()))),
            ])
        })
    }

    #[test]
    fn test_display_positional_full() {
        let keymaps = Keymaps(
            [
                Keymap::new("a", "Aloha".to_string(), Dispatch::Null),
                Keymap::new("b", "Bomb".to_string(), Dispatch::Null),
                Keymap::new("F", "Foo".to_string(), Dispatch::Null),
                Keymap::new("c", "Caterpillar".to_string(), Dispatch::Null),
                Keymap::new("alt+g", "Gogagg".to_string(), Dispatch::Null),
            ]
            .to_vec(),
        );
        let context = Context::default();
        let actual = keymaps
            .display(
                context.keyboard_layout_kind(),
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

        let actual = keymaps
            .display(
                context.keyboard_layout_kind(),
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
        let keymaps = Keymaps(
            [
                Keymap::new("a", "Aloha".to_string(), Dispatch::Null),
                Keymap::new("b", "Bomb".to_string(), Dispatch::Null),
                Keymap::new("F", "Foo".to_string(), Dispatch::Null),
                Keymap::new("c", "Caterpillar".to_string(), Dispatch::Null),
                Keymap::new("alt+g", "Gogagg".to_string(), Dispatch::Null),
                Keymap::new("alt+l", "Lamp".to_string(), Dispatch::Null),
            ]
            .to_vec(),
        );
        let context = Context::default();
        let actual = keymaps
            .display(
                context.keyboard_layout_kind(),
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
        let keymaps = Keymaps(
            [
                Keymap::new("a", "Aloha".to_string(), Dispatch::Null),
                Keymap::new("b", "Bomb".to_string(), Dispatch::Null),
                Keymap::new("F", "Foo".to_string(), Dispatch::Null),
                Keymap::new("c", "Caterpillar".to_string(), Dispatch::Null),
                Keymap::new("alt+g", "Gogagg".to_string(), Dispatch::Null),
                Keymap::new("alt+l", "Lamp".to_string(), Dispatch::Null),
            ]
            .to_vec(),
        );
        let context = Context::default();
        let actual = keymaps
            .display(
                context.keyboard_layout_kind(),
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
                keymaps: Keymaps::new(&[Keymap::new(
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
        )
    }

    #[test]
    /// When release key is defined and on tap is defined, display should show the on tap action.
    fn on_tap_display() {
        let mut keymap_legend = KeymapLegend::new(
            KeymapLegendConfig {
                title: "".to_string(),
                keymaps: Keymaps::new(&[]),
            },
            &Context::default(),
            Some(ReleaseKey::new(
                Meaning::AgSlL,
                Some(OnTap::new("Conichihuahua", Dispatches::default())),
            )),
        );

        let _ = keymap_legend
            .handle_dispatch_editor(
                &mut Context::default(),
                DispatchEditor::SetRectangle(Rectangle {
                    origin: Default::default(),
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
        )
    }

    #[test]
    /// When release key is defined and the release key is immediately received
    /// before any actions in the keymap is executed, the on tap dispatches should be fired.
    fn release_key_on_tap() -> anyhow::Result<()> {
        let context = Context::default();
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
                        keymaps: Keymaps::new(&[]),
                    },
                    ReleaseKey::new(
                        Meaning::Paste,
                        Some(OnTap::new(
                            "",
                            Dispatches::one(Dispatch::ToEditor(SetContent(
                                "on tapped!".to_string(),
                            ))),
                        )),
                    ),
                )),
                // Expect the keymap legend is opened
                Expect(AppGridContains("LEGEND_TITLE")),
                // Simulate key release
                App(HandleKeyEvent(
                    parse_key_event(context.keyboard_layout_kind().get_key(&Meaning::Paste))
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
        let context = Context::default();
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
                        keymaps: Keymaps::new(&[Keymap::new(
                            "x",
                            "".to_string(),
                            Dispatch::ToEditor(Insert("hello".to_string())),
                        )]),
                    },
                    ReleaseKey::new(Meaning::Paste, None),
                )),
                // Expect the keymap legend is opened
                Expect(AppGridContains("LEGEND_TITLE")),
                // Execute an action defined in the keymap
                App(HandleKeyEvent(key!("x"))),
                // Expect the keymap legend is still opened
                Expect(AppGridContains("LEGEND_TITLE")),
                // Simulate key release
                App(HandleKeyEvent(
                    parse_key_event(context.keyboard_layout_kind().get_key(&Meaning::Paste))
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
}
