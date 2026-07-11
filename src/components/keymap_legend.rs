use event::{KeyEvent, KeyEventKind};
use itertools::Itertools;
use my_proc_macros::key;
use std::borrow::Cow;

use crate::{
    app::{Dispatch, Dispatches},
    components::{
        editor_keymap::{CombinedKeyEvent, KeyboardLayout},
        editor_keymap_printer::KeymapDisplayOption,
    },
    config::AppConfig,
    context::Context,
    rectangle::Rectangle,
};

use super::{component::Component, editor::Editor, editor_keymap_printer::KeymapPrintSection};

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
    pub event: KeyEvent,
    pub name: String,
    pub config: KeymapLegendConfig,
    pub on_tap: Option<OnTap>,
}

struct ParsedReleaseKey {
    on_tap: Option<OnTap>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReleaseKey {
    key_event: KeyEvent,
    on_tap: Option<OnTap>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OnTap {
    description: &'static str,
    dispatch: Box<Dispatch>,
}

impl OnTap {
    pub fn new(description: &'static str, dispatch: Dispatch) -> Self {
        Self {
            description,
            dispatch: Box::new(dispatch),
        }
    }

    pub fn description(&self) -> &'static str {
        self.description
    }

    pub fn dispatch(&self) -> &Dispatch {
        &self.dispatch
    }
}

impl ReleaseKey {
    pub fn new(event: KeyEvent, on_tap: Option<OnTap>) -> Self {
        Self {
            key_event: event.set_event_kind(KeyEventKind::Release),
            on_tap,
        }
    }

    pub fn key_event(&self) -> KeyEvent {
        self.key_event
    }

    pub fn on_tap(&self) -> Option<&OnTap> {
        self.on_tap.as_ref()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Keymap(Vec<Keybinding>);
impl Keymap {
    fn display(
        &self,
        terminal_width: usize,
        option: &KeymapDisplayOption,
        layout: Option<&KeyboardLayout>,
    ) -> String {
        KeymapPrintSection::from_keymap("".to_string(), self).display(
            terminal_width,
            option,
            layout,
        )
    }
    pub fn new(keybindings: &[Keybinding]) -> Self {
        Self(keybindings.to_vec())
    }

    pub fn get(&self, event: &CombinedKeyEvent) -> std::option::Option<&Keybinding> {
        self.0.iter().find(|key| key.event == event.translated)
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Keybinding> {
        self.0.iter()
    }

    pub fn into_vec(self) -> Vec<Keybinding> {
        self.0
    }
}

impl KeymapLegendConfig {
    pub fn display(
        &self,
        width: usize,
        option: &KeymapDisplayOption,
        layout: Option<&KeyboardLayout>,
    ) -> String {
        self.keymap.display(width, option, layout)
    }

    pub fn keymap(&self) -> Keymap {
        let keymap = &self.keymap;
        #[cfg(test)]
        {
            use itertools::Itertools;

            let conflicting_keybindings = keymap
                .iter()
                .chunk_by(|keymap| &keymap.event)
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
    event: KeyEvent,
    action: KeybindingAction,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KeybindingAction {
    name: Cow<'static, str>,
    documentation: Option<&'static str>,
    dispatch: Dispatch,
}

impl Keybinding {
    pub fn new_undocumented(event: KeyEvent, name: &'static str, dispatch: Dispatch) -> Keybinding {
        Keybinding {
            event,
            action: KeybindingAction {
                documentation: None,
                name: name.into(),
                dispatch,
            },
        }
    }

    pub fn new(
        event: KeyEvent,
        name: &'static str,
        doc: &'static str,
        dispatch: Dispatch,
    ) -> Keybinding {
        Keybinding {
            event,
            action: KeybindingAction {
                documentation: Some(doc),
                name: name.into(),
                dispatch,
            },
        }
    }

    pub fn new_dynamic(event: KeyEvent, name: String, dispatch: Dispatch) -> Keybinding {
        Keybinding {
            event,
            action: KeybindingAction {
                documentation: None,
                name: name.into(),
                dispatch,
            },
        }
    }

    pub fn app_momentary_layer(
        MomentaryLayer {
            event,
            name,
            config,
            on_tap,
        }: MomentaryLayer,
    ) -> Keybinding {
        Keybinding {
            event,
            action: KeybindingAction {
                name: name.into(),
                documentation: None,
                dispatch: Dispatch::ShowAppMomentaryLayer(config, ReleaseKey::new(event, on_tap)),
            },
        }
    }

    pub fn momentary_layer(
        MomentaryLayer {
            event,
            name,
            config,
            on_tap,
        }: MomentaryLayer,
    ) -> Keybinding {
        Keybinding {
            event,
            action: KeybindingAction {
                name: name.into(),
                documentation: None,
                dispatch: Dispatch::ShowMomentaryLayer(config, ReleaseKey::new(event, on_tap)),
            },
        }
    }

    pub fn get_dispatches(&self) -> Dispatches {
        Dispatches::one(self.action.dispatch.clone()).append(Dispatch::SetLastActionDescription {
            long_description: self.action.name.clone().into_owned(),
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
                event: self.event,
                action: KeybindingAction {
                    name: keymap_override.description.into(),
                    dispatch: keymap_override.dispatch.clone(),
                    ..self.action
                },
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
        self.action.name.clone().into_owned()
    }
}

impl KeymapLegend {
    pub fn new(config: KeymapLegendConfig, release_key: Option<ReleaseKey>) -> KeymapLegend {
        let mut editor = Editor::from_text(None, "");
        editor.set_title(config.title.clone());

        let release_key = release_key.map(|release_key| ParsedReleaseKey {
            on_tap: release_key.on_tap,
        });

        KeymapLegend {
            editor,
            config,
            release_key,
        }
    }

    fn refresh(&mut self, width: usize, context: &Context) {
        // Check for duplicate keys
        let duplicates = self
            .config
            .keymap()
            .0
            .into_iter()
            .duplicates_by(|keymap| keymap.event)
            .collect_vec();

        if !duplicates.is_empty() {
            let message = format!(
                "Duplicate keymap keys for {}: {:#?}",
                self.config.title,
                duplicates
                    .into_iter()
                    .map(|duplicate| format!("{:?}: {}", duplicate.event, duplicate.action.name))
                    .collect_vec()
            );
            log::info!("{message}");
            // panic!("{}", message);
        }

        let content = self.display(width, self.keyboard_layout(context).as_ref());

        // dropping dispatch as this is a buffer with no path and
        // set_content dispatches are related to file dirty status
        let _ = self
            .editor_mut()
            .set_content(&content, context)
            .unwrap_or_default();
    }

    fn keyboard_layout(&self, context: &Context) -> Option<KeyboardLayout> {
        AppConfig::singleton()
            .show_key_in_keymap()
            .then(|| context.keyboard_layout().clone())
    }

    fn display(&self, width: usize, layout: Option<&KeyboardLayout>) -> String {
        // Generate the content one column narrower than the given width. The
        // editor soft-wraps at `width - 1` (one column is reserved for the
        // cursor at the last column), so content spanning the full width
        // would wrap and double up each keyboard row.
        let content = self.config.display(
            width.saturating_sub(1),
            &KeymapDisplayOption {
                show_alt: true,
                show_shift: true,
            },
            layout,
        );

        // Show space keybinding, if any
        let space_keybinding = self
            .config
            .keymap()
            .iter()
            .find(|keybinding| keybinding.event == key!("space"))
            .map(|keybinding| format!("Space: {}", keybinding.action.name));

        let on_tap = self.release_key.as_ref().and_then(|release_key| {
            let description = release_key.on_tap.as_ref()?.description;
            Some(format!("Release hold: {}", description))
        });

        Some(content)
            .into_iter()
            .chain(space_keybinding)
            .chain(on_tap)
            .collect_vec()
            .join("\n")
    }
}

impl Component for KeymapLegend {
    fn editor(&self) -> &Editor {
        &self.editor
    }

    fn set_rectangle(&mut self, rectangle: Rectangle, context: &Context) {
        self.refresh(rectangle.width, context); // TODO: pass theme from App.rs
        self.editor_mut().set_rectangle(rectangle, context);
    }

    fn editor_mut(&mut self) -> &mut Editor {
        &mut self.editor
    }

    /// The keymap legend is rendered as a bottom overlay sized to fit
    /// exactly its own content (keybindings + title), rather than being
    /// tiled like other components.
    fn desired_height(&self, width: usize, context: &Context) -> Option<usize> {
        let content = self.display(width, self.keyboard_layout(context).as_ref());
        let content_height = crate::soft_wrap::soft_wrap(&content, width)
            .wrapped_lines_count()
            .max(1);
        let title_lines = self
            .editor
            .title(
                context,
                &crate::app::Dimension { width, height: 0 },
                &crate::components::component::RenderTitleMode::Tabline,
            )
            .lines()
            .count();
        Some(content_height + title_lines)
    }

    fn handle_key_event(
        &mut self,
        context: &crate::context::Context,
        event: event::KeyEvent,
    ) -> anyhow::Result<Dispatches> {
        self.editor.handle_key_event(context, event)
    }
}

#[cfg(test)]
mod test_keymap_legend {
    use super::*;
    use crate::{
        buffer::BufferOwner, components::editor::DispatchEditor, position::Position, test_app::*,
    };
    use event::{parse_key_event, KeyEventKind};
    use my_proc_macros::key;

    #[test]
    fn test_esc() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                App(ShowMenu(KeymapLegendConfig {
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
                Keybinding::new_undocumented(key!("a"), "Aloha", Dispatch::Null),
                Keybinding::new_undocumented(key!("b"), "Bomb", Dispatch::Null),
                Keybinding::new_undocumented(key!("F"), "Foo", Dispatch::Null),
                Keybinding::new_undocumented(key!("c"), "Caterpillar", Dispatch::Null),
                Keybinding::new_undocumented(key!("alt+g"), "Gogagg", Dispatch::Null),
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
                None,
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
                None,
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
                Keybinding::new_undocumented(key!("a"), "Aloha", Dispatch::Null),
                Keybinding::new_undocumented(key!("b"), "Bomb", Dispatch::Null),
                Keybinding::new_undocumented(key!("F"), "Foo", Dispatch::Null),
                Keybinding::new_undocumented(key!("c"), "Caterpillar", Dispatch::Null),
                Keybinding::new_undocumented(key!("alt+g"), "Gogagg", Dispatch::Null),
                Keybinding::new_undocumented(key!("alt+l"), "Lamp", Dispatch::Null),
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
                None,
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
                Keybinding::new_undocumented(key!("a"), "Aloha", Dispatch::Null),
                Keybinding::new_undocumented(key!("b"), "Bomb", Dispatch::Null),
                Keybinding::new_undocumented(key!("F"), "Foo", Dispatch::Null),
                Keybinding::new_undocumented(key!("c"), "Caterpillar", Dispatch::Null),
                Keybinding::new_undocumented(key!("alt+g"), "Gogagg", Dispatch::Null),
                Keybinding::new_undocumented(key!("alt+l"), "Lamp", Dispatch::Null),
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
                None,
            )
            .to_string();
        let expected = "Window is too small to display keymap legend :(";
        assert_eq!(actual, expected);
    }

    #[test]
    fn space_keybinding_should_be_displayed() {
        let mut keymap_legend = KeymapLegend::new(
            KeymapLegendConfig {
                title: "".to_string(),
                keymap: Keymap::new(&[Keybinding::new_undocumented(
                    key!("space"),
                    "Hello world",
                    Dispatch::Null,
                )]),
            },
            None,
        );

        let _ = keymap_legend
            .handle_dispatch_editor(
                &Context::default(),
                DispatchEditor::SetRectangle(Rectangle {
                    origin: Position::default(),
                    width: 100,
                    height: 100,
                }),
            )
            .unwrap();

        assert_eq!(
            keymap_legend.display(100, None),
            "
╭───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───╮
│   ┆   ┆   ┆   ┆   ┆ ∅ ┆   ┆   ┆   ┆   ┆   │
├╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┤
│   ┆   ┆   ┆   ┆   ┆ ∅ ┆   ┆   ┆   ┆   ┆   │
├╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┤
│   ┆   ┆   ┆   ┆   ┆ ∅ ┆   ┆   ┆   ┆   ┆   │
╰───┴───┴───┴───┴───┴───┴───┴───┴───┴───┴───╯
* Pick Keyboard    \\ Leader
Space: Hello world
"
            .trim()
        );
    }

    #[test]
    fn keymap_legend_reserves_bottom_space_matching_desired_height() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                App(crate::app::Dispatch::TerminalDimensionChanged(
                    crate::app::Dimension {
                        width: 60,
                        height: 15,
                    },
                )),
                Editor(DispatchEditor::SetContent(
                    "a\nb\nc\nd\ne\nf\ng\nh\n".to_string(),
                )),
                App(ShowMenu(KeymapLegendConfig {
                    title: "T".to_string(),
                    keymap: Keymap::new(&[Keybinding::new_undocumented(
                        key!("x"),
                        "Xray",
                        Dispatch::Null,
                    )]),
                })),
                // The tiled editor above shrinks to the remaining space,
                // while the keymap legend below occupies exactly its
                // `desired_height` (title line + grid + legend footnote),
                // with no overlap or stray blank lines between them.
                Expect(AppGrid(
                    " [:] 🦀  main.rs
1│█
2│b
3│c
4│d
T
1│╭───┬──────┬───┬───┬───┬───┬───┬───┬───┬───┬───╮
2││   ┆      ┆   ┆   ┆   ┆ ∅ ┆   ┆   ┆   ┆   ┆   │
3│├╌╌╌┼╌╌╌╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┤
4││   ┆      ┆   ┆   ┆   ┆ ∅ ┆   ┆   ┆   ┆   ┆   │
5│├╌╌╌┼╌╌╌╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┤
6││   ┆ Xray ┆   ┆   ┆   ┆ ∅ ┆   ┆   ┆   ┆   ┆   │
7│╰───┴──────┴───┴───┴───┴───┴───┴───┴───┴───┴───╯
8│* Pick Keyboard    \\ Leader"
                        .to_string(),
                )),
            ])
        })
    }

    #[test]
    fn desired_height_accounts_for_content_and_title_lines() {
        let keymap_legend = KeymapLegend::new(
            KeymapLegendConfig {
                title: "T".to_string(),
                keymap: Keymap::new(&[]),
            },
            None,
        );

        let content = keymap_legend.display(100, None);
        // No title line is included in `display`, so the expected height is
        // the content's line count plus one line for the title.
        let expected_content_lines = content.lines().count();
        assert_eq!(
            keymap_legend.desired_height(100, &Context::default()),
            Some(expected_content_lines + 1)
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
            Some(ReleaseKey::new(
                key!("Y"),
                Some(OnTap::new("Conichihuahua", Dispatch::Null)),
            )),
        );

        let _ = keymap_legend
            .handle_dispatch_editor(
                &Context::default(),
                DispatchEditor::SetRectangle(Rectangle {
                    origin: Position::default(),
                    width: 100,
                    height: 100,
                }),
            )
            .unwrap();

        assert_eq!(
            keymap_legend.display(100, None),
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
                App(ShowMomentaryLayer(
                    KeymapLegendConfig {
                        title: "LEGEND_TITLE".to_string(),
                        keymap: Keymap::new(&[]),
                    },
                    ReleaseKey::new(
                        key!("b"),
                        Some(OnTap::new(
                            "",
                            Dispatch::ToEditor(SetContent("on tapped!".to_string())),
                        )),
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
                App(ShowMomentaryLayer(
                    KeymapLegendConfig {
                        title: "LEGEND_TITLE".to_string(),
                        keymap: Keymap::new(&[Keybinding::new_undocumented(
                            key!("x"),
                            "",
                            Dispatch::ToEditor(Insert("hello".to_string())),
                        )]),
                    },
                    ReleaseKey::new(key!("b"), None),
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
}
