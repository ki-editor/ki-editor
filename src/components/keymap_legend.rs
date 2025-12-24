use event::{parse_key_event, KeyEvent};
use regex::Regex;

use itertools::Itertools;
use my_proc_macros::key;

use crate::{
    app::{Dispatch, Dispatches},
    components::{
        editor::RegexHighlightRuleCaptureStyle, editor_keymap_printer::KeymapDisplayOption,
    },
    context::Context,
    grid::StyleKey,
    rectangle::Rectangle,
};

use super::{
    component::Component,
    editor::{Direction, Editor, Mode, RegexHighlightRule},
    editor_keymap::KeyboardLayoutKind,
    editor_keymap_printer::KeymapPrintSection,
    render_editor::Source,
};

pub(crate) struct KeymapLegend {
    editor: Editor,
    config: KeymapLegendConfig,
    option: KeymapDisplayOption,
    keymap_layout_kind: super::editor_keymap::KeyboardLayoutKind,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct KeymapLegendConfig {
    pub(crate) title: String,
    pub(crate) keymaps: Keymaps,
}

const BETWEEN_KEY_AND_DESCRIPTION: &str = " → ";

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Keymaps(Vec<Keymap>);
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
    pub(crate) fn new(keymaps: &[Keymap]) -> Self {
        Self(keymaps.to_vec())
    }

    pub(crate) fn get(&self, event: &KeyEvent) -> std::option::Option<&Keymap> {
        self.0.iter().find(|key| &key.event == event)
    }

    pub(crate) fn iter(&self) -> std::slice::Iter<'_, Keymap> {
        self.0.iter()
    }

    pub(crate) fn into_vec(self) -> Vec<Keymap> {
        self.0
    }
}

impl KeymapLegendConfig {
    pub(crate) fn display(
        &self,
        keyboard_layout_kind: &KeyboardLayoutKind,
        width: usize,
        option: &KeymapDisplayOption,
    ) -> String {
        self.keymaps.display(keyboard_layout_kind, width, option)
    }

    pub(crate) fn keymaps(&self) -> Keymaps {
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

    fn get_regex_highlight_rules(&self) -> Vec<RegexHighlightRule> {
        self.keymaps()
            .0
            .into_iter()
            .flat_map(|keymap| {
                let keymap_key = RegexHighlightRule {
                    regex: Regex::new(&format!(
                        "(?<key>{})(?<arrow>{})({})",
                        regex::escape(keymap.key),
                        BETWEEN_KEY_AND_DESCRIPTION,
                        regex::escape(&keymap.description),
                    ))
                    .unwrap(),
                    capture_styles: vec![
                        RegexHighlightRuleCaptureStyle::new(
                            "key",
                            Source::StyleKey(StyleKey::KeymapKey),
                        ),
                        RegexHighlightRuleCaptureStyle::new(
                            "arrow",
                            Source::StyleKey(StyleKey::KeymapArrow),
                        ),
                    ],
                };
                let keymap_hint = (|| {
                    let index = keymap
                        .description
                        .to_lowercase()
                        .find(&keymap.key.to_lowercase())?;
                    let range = index..index + 1;
                    let marked_description = {
                        let mut description = keymap.description.clone();
                        description.replace_range(
                            range.clone(),
                            &format!("___OPEN___{}___CLOSE___", keymap.description.get(range)?),
                        );
                        regex::escape(&description)
                            .replace("___OPEN___", "(?<hint>")
                            .replace("___CLOSE___", ")")
                    };
                    Some(RegexHighlightRule {
                        regex: regex::Regex::new(
                            &(format!(
                                "{}{}{}",
                                regex::escape(keymap.key),
                                BETWEEN_KEY_AND_DESCRIPTION,
                                marked_description
                            )),
                        )
                        .unwrap(),
                        capture_styles: vec![RegexHighlightRuleCaptureStyle::new(
                            "hint",
                            Source::StyleKey(StyleKey::KeymapHint),
                        )],
                    })
                })();
                vec![Some(keymap_key), keymap_hint]
            })
            .flatten()
            .collect_vec()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Keymap {
    key: &'static str,
    pub short_description: Option<String>,
    pub description: String,
    event: KeyEvent,
    dispatch: Dispatch,
}

impl Keymap {
    pub(crate) fn new(key: &'static str, description: String, dispatch: Dispatch) -> Keymap {
        Keymap {
            key,
            short_description: None,
            description,
            dispatch,
            event: parse_key_event(key).unwrap(),
        }
    }

    pub(crate) fn new_extended(
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

    pub(crate) fn get_dispatches(&self) -> Dispatches {
        Dispatches::one(self.dispatch.clone()).append(Dispatch::SetLastActionDescription {
            long_description: self.description.clone(),
            short_description: self.short_description.clone(),
        })
    }

    pub(crate) fn event(&self) -> &KeyEvent {
        &self.event
    }

    pub(crate) fn override_keymap(
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

    pub(crate) fn display(&self) -> String {
        self.short_description
            .clone()
            .unwrap_or_else(|| self.description.clone())
    }
}

impl KeymapLegend {
    pub(crate) fn new(config: KeymapLegendConfig, context: &Context) -> KeymapLegend {
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
        editor.set_regex_highlight_rules(config.get_regex_highlight_rules());
        KeymapLegend {
            editor,
            config,
            option: KeymapDisplayOption {
                show_alt: true,
                show_shift: true,
            },
            keymap_layout_kind: *context.keyboard_layout_kind(),
        }
    }

    fn refresh(&mut self, context: &Context) {
        let content = self.config.display(
            &self.keymap_layout_kind,
            self.editor.rectangle().width,
            &self.option,
        );
        self.editor_mut()
            .set_content(&content, context)
            .unwrap_or_default();
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
                        Ok(Dispatches::one(close_current_window).chain(keymap.get_dispatches()))
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
    use crate::{buffer::BufferOwner, test_app::*};
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
        let expected = "
╭───────┬───┬─────────────┬───┬──────┬───┬───┬───┬───┬───┬───╮
│       ┆   ┆             ┆   ┆      ┆ ∅ ┆   ┆   ┆   ┆   ┆   │
├╌╌╌╌╌╌╌┼╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌┼╌╌╌╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┤
│ Aloha ┆   ┆             ┆   ┆      ┆ ∅ ┆   ┆   ┆   ┆   ┆   │
├╌╌╌╌╌╌╌┼╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌┼╌╌╌╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┤
│       ┆   ┆ Caterpillar ┆   ┆ Bomb ┆ ∅ ┆   ┆   ┆   ┆   ┆   │
╰───────┴───┴─────────────┴───┴──────┴───┴───┴───┴───┴───┴───╯
* Pick Keyboard    ] Expand Forward    [ Expand Backward
"
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
        let expected = "
╭───────┬───┬─────────────┬─────┬────────┬───┬───┬───┬───┬───┬───╮
│       ┆   ┆             ┆     ┆        ┆ ∅ ┆   ┆   ┆   ┆   ┆   │
├╌╌╌╌╌╌╌┼╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌┼╌╌╌╌╌╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┤
│       ┆   ┆             ┆     ┆ Gogagg ┆ ⌥ ┆   ┆   ┆   ┆   ┆   │
│       ┆   ┆             ┆ Foo ┆        ┆ ⇧ ┆   ┆   ┆   ┆   ┆   │
│ Aloha ┆   ┆             ┆     ┆        ┆ ∅ ┆   ┆   ┆   ┆   ┆   │
├╌╌╌╌╌╌╌┼╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌┼╌╌╌╌╌╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┤
│       ┆   ┆ Caterpillar ┆     ┆  Bomb  ┆ ∅ ┆   ┆   ┆   ┆   ┆   │
╰───────┴───┴─────────────┴─────┴────────┴───┴───┴───┴───┴───┴───╯
* Pick Keyboard    ] Expand Forward    [ Expand Backward"
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
        let expected = "
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
* Pick Keyboard    ] Expand Forward    [ Expand Backward"
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
    fn test_regex_keymap_hint() {
        let keymaps = Keymaps(
            [
                Keymap::new("a", "Aloha".to_string(), Dispatch::Null),
                Keymap::new("b", "Cob".to_string(), Dispatch::Null),
                Keymap::new("f", "Find (Local)".to_string(), Dispatch::Null),
                Keymap::new("g", "Find (Global)".to_string(), Dispatch::Null),
            ]
            .to_vec(),
        );
        let regexes = KeymapLegendConfig {
            title: "".to_string(),
            keymaps,
        }
        .get_regex_highlight_rules()
        .into_iter()
        .map(|rule| rule.regex.as_str().to_string())
        .collect_vec();

        let expected = [
            "(?<key>a)(?<arrow> → )(Aloha)",
            "a → (?<hint>A)loha",
            "(?<key>b)(?<arrow> → )(Cob)",
            "b → Co(?<hint>b)",
            "(?<key>f)(?<arrow> → )(Find \\(Local\\))",
            "f → (?<hint>F)ind \\(Local\\)",
            "(?<key>g)(?<arrow> → )(Find \\(Global\\))",
            "g → Find \\((?<hint>G)lobal\\)",
        ];
        assert_eq!(regexes, expected)
    }
}
