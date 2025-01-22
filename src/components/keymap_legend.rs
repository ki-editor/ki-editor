use event::{parse_key_event, KeyEvent};
use regex::Regex;
use unicode_width::UnicodeWidthStr;

use itertools::Itertools;
use my_proc_macros::key;

use crate::{
    app::{Dispatch, Dispatches},
    components::editor::RegexHighlightRuleCaptureStyle,
    grid::StyleKey,
    rectangle::Rectangle,
};

use super::{
    component::Component,
    editor::{Direction, Editor, Mode, RegexHighlightRule},
    editor_keymap::KEYBOARD_LAYOUT,
    editor_keymap_printer::KeymapPrintSection,
    render_editor::Source,
};

pub(crate) struct KeymapLegend {
    editor: Editor,
    config: KeymapLegendConfig,
    show_shift_alt_keys: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct KeymapLegendConfig {
    pub(crate) title: String,
    pub(crate) body: KeymapLegendBody,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum KeymapLegendBody {
    Positional(Keymaps),
    Mnemonic(Keymaps),
}
const BETWEEN_KEY_AND_DESCRIPTION: &str = " → ";

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Keymaps(Vec<Keymap>);
impl Keymaps {
    fn display_positional(&self, terminal_width: u16, show_shift_alt_keys: bool) -> String {
        let table = KeymapPrintSection::from_keymaps(
            "".to_string(),
            self,
            KEYBOARD_LAYOUT.get_keyboard_layout(),
        )
        .display(terminal_width, show_shift_alt_keys);

        format!("Press space to toggle alt/shift keys.\n{table}")
    }
    fn display_mnemonic(&self, indent: usize, width: usize) -> String {
        let width = width.saturating_sub(indent);
        let max_key_width = self
            .0
            .iter()
            .map(|keymap| keymap.key.len())
            .max()
            .unwrap_or(0);
        let max_description_width = self
            .0
            .iter()
            .map(|keymap| keymap.description.len())
            .max()
            .unwrap_or(0);
        let key_description_gap = UnicodeWidthStr::width(BETWEEN_KEY_AND_DESCRIPTION);
        let column_gap = key_description_gap * 2;
        let column_width = max_key_width + key_description_gap + max_description_width + column_gap;
        let column_count = width / column_width;

        // Align the keys columns and the dispatch columns
        let result = self
            .0
            .iter()
            // .sorted_by_key(|keymap| keymap.key.to_lowercase())
            .map(|keymap| {
                let formatted = format!(
                    "{: >width$}{}{}",
                    keymap.key,
                    BETWEEN_KEY_AND_DESCRIPTION,
                    keymap.description,
                    width = max_key_width
                );
                formatted
            })
            .chunks(column_count.max(1)) // At least 1, otherwise `chunks` will panic
            .into_iter()
            .map(|chunks| {
                chunks
                    .map(|chunk| {
                        let second_formatted = format!("{: <width$}", chunk, width = column_width);
                        second_formatted
                    })
                    .join("")
            })
            .join("\n");
        let result = dedent(&result);
        result
            .lines()
            .map(|line| format!("{}{}", " ".repeat(indent), line.trim_end()))
            .join("\n")
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
}

fn dedent(s: &str) -> String {
    // Split the input string into lines
    let lines: Vec<&str> = s.lines().collect();

    // Find the minimum indentation (number of leading spaces)
    let min_indent = lines
        .iter()
        .filter(|&&line| !line.trim().is_empty())
        .map(|line| line.chars().take_while(|&c| c == ' ').count())
        .min()
        .unwrap_or(0);

    // Remove the common indentation from each line
    let dedented_lines: Vec<String> = lines
        .iter()
        .map(|&line| {
            if line.len() >= min_indent {
                line[min_indent..].to_string()
            } else {
                line.to_string()
            }
        })
        .collect();

    // Join the dedented lines back into a single string
    dedented_lines.join("\n")
}

impl KeymapLegendBody {
    fn display(&self, width: u16, show_shift_alt_keys: bool) -> String {
        match self {
            KeymapLegendBody::Positional(keymaps) => {
                keymaps.display_positional(width, show_shift_alt_keys)
            }
            KeymapLegendBody::Mnemonic(keymaps) => keymaps.display_mnemonic(0, width as usize),
        }
    }

    fn keymaps(&self) -> Vec<&Keymap> {
        match self {
            KeymapLegendBody::Positional(keymaps) => keymaps.0.iter().collect_vec(),
            KeymapLegendBody::Mnemonic(keymaps) => keymaps.0.iter().collect_vec(),
        }
    }
}

impl KeymapLegendConfig {
    fn display(&self, width: u16, show_shift_alt_keys: bool) -> String {
        self.body.display(width, show_shift_alt_keys)
    }

    pub(crate) fn keymaps(&self) -> Keymaps {
        let keymaps = self.body.keymaps();
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
        Keymaps::new(&keymaps.into_iter().cloned().collect_vec())
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

impl From<KeymapLegendConfig> for Vec<Keymaps> {
    fn from(keymap_legend_config: KeymapLegendConfig) -> Self {
        match &keymap_legend_config.body {
            KeymapLegendBody::Positional(keymaps) => vec![keymaps.clone()],
            KeymapLegendBody::Mnemonic(keymaps) => vec![keymaps.clone()],
        }
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
    ) -> Keymap {
        match keymap_override {
            Some(keymap_override) => Self {
                short_description: Some(keymap_override.description.to_string()),
                description: keymap_override.description.to_string(),
                dispatch: keymap_override.dispatch.clone(),
                ..self
            },
            None => self,
        }
    }

    pub(crate) fn display(&self) -> String {
        let key_event = self.event().display();
        let description = self
            .short_description
            .clone()
            .unwrap_or_else(|| self.description.clone());
        if key_event.contains("shift+") {
            format!("⇧ {description}")
        } else if key_event.contains("alt+") {
            format!("⌥ {description}")
        } else {
            description.clone()
        }
    }
}

impl KeymapLegend {
    pub(crate) fn new(config: KeymapLegendConfig) -> KeymapLegend {
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
            log::info!("{}", message);
            // panic!("{}", message);
        }

        let mut editor = Editor::from_text(None, "");
        editor.set_title(config.title.clone());
        let _ = editor.enter_insert_mode(Direction::End).unwrap_or_default();
        editor.set_regex_highlight_rules(config.get_regex_highlight_rules());
        KeymapLegend {
            editor,
            config,
            show_shift_alt_keys: false,
        }
    }

    fn refresh(&mut self) {
        let content = self
            .config
            .display(self.editor.rectangle().width, self.show_shift_alt_keys);
        self.editor_mut().set_content(&content).unwrap_or_default();
    }
}

impl Component for KeymapLegend {
    fn editor(&self) -> &Editor {
        &self.editor
    }

    fn set_rectangle(&mut self, rectangle: Rectangle) {
        self.refresh(); // TODO: pass theme from App.rs
        self.editor_mut().set_rectangle(rectangle);
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
                    self.editor.enter_normal_mode()?;
                    Ok(Default::default())
                }
                key!("space") => {
                    self.show_shift_alt_keys = !self.show_shift_alt_keys;
                    Ok(Default::default())
                }
                key!("ctrl+c") => Ok(Dispatches::one(Dispatch::CloseCurrentWindow)),
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
                    body: KeymapLegendBody::Positional(Keymaps::new(&[])),
                })),
                App(HandleKeyEvent(key!("esc"))),
                App(HandleKeyEvent(key!("esc"))),
                Expect(CurrentComponentPath(Some(s.main_rs()))),
            ])
        })
    }

    #[test]
    fn test_display_mnemonic() {
        let keymaps = Keymaps(
            [
                Keymap::new("a", "Aloha".to_string(), Dispatch::Null),
                Keymap::new("b", "Bomb".to_string(), Dispatch::Null),
                Keymap::new("c", "Caterpillar".to_string(), Dispatch::Null),
                Keymap::new("d", "D".to_string(), Dispatch::Null),
                Keymap::new("e", "Elephant".to_string(), Dispatch::Null),
                Keymap::new("space", "Gogagg".to_string(), Dispatch::Null),
            ]
            .to_vec(),
        );
        let width = 53;
        let actual = keymaps.display_mnemonic(2, width).to_string();
        let expected = "
  a → Aloha                b → Bomb
  c → Caterpillar          d → D
  e → Elephant         space → Gogagg"
            .trim_matches('\n');
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_display_positional() {
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
        let actual = keymaps.display_positional(19, false).to_string();
        let expected = "
Press space to toggle alt/shift keys.
╭───────┬───┬─────────────┬───┬──────┬───┬───┬───┬───┬───┬───╮
│       ┆   ┆             ┆   ┆      ┆ - ┆   ┆   ┆   ┆   ┆   │
├╌╌╌╌╌╌╌┼╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌┼╌╌╌╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┤
│ Aloha ┆   ┆             ┆   ┆      ┆ - ┆   ┆   ┆   ┆   ┆   │
├╌╌╌╌╌╌╌┼╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌┼╌╌╌╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┤
│       ┆   ┆ Caterpillar ┆   ┆ Bomb ┆ - ┆   ┆   ┆   ┆   ┆   │
╰───────┴───┴─────────────┴───┴──────┴───┴───┴───┴───┴───┴───╯"
            .trim_matches('\n');
        assert_eq!(actual, expected);

        let actual = keymaps.display_positional(19, true).to_string();
        let expected = "
Press space to toggle alt/shift keys.
╭───────┬───┬─────────────┬───────┬──────────┬───┬───┬───┬───┬───┬───╮
│       ┆   ┆             ┆       ┆          ┆ - ┆   ┆   ┆   ┆   ┆   │
│       ┆   ┆             ┆       ┆          ┆   ┆   ┆   ┆   ┆   ┆   │
│       ┆   ┆             ┆       ┆          ┆   ┆   ┆   ┆   ┆   ┆   │
├╌╌╌╌╌╌╌┼╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┤
│       ┆   ┆             ┆       ┆ ⌥ Gogagg ┆ - ┆   ┆   ┆   ┆   ┆   │
│       ┆   ┆             ┆ ⇧ Foo ┆          ┆   ┆   ┆   ┆   ┆   ┆   │
│ Aloha ┆   ┆             ┆       ┆          ┆   ┆   ┆   ┆   ┆   ┆   │
├╌╌╌╌╌╌╌┼╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┤
│       ┆   ┆             ┆       ┆          ┆ - ┆   ┆   ┆   ┆   ┆   │
│       ┆   ┆             ┆       ┆          ┆   ┆   ┆   ┆   ┆   ┆   │
│       ┆   ┆ Caterpillar ┆       ┆   Bomb   ┆   ┆   ┆   ┆   ┆   ┆   │
╰───────┴───┴─────────────┴───────┴──────────┴───┴───┴───┴───┴───┴───╯"
            .trim_matches('\n');
        assert_eq!(actual, expected);
    }

    #[test]
    fn should_intercept_key_event_defined_in_config() {
        let mut keymap_legend = KeymapLegend::new(KeymapLegendConfig {
            title: "Test".to_string(),
            body: KeymapLegendBody::Positional(Keymaps::new(&[Keymap::new(
                "s",
                "fifafofum".to_string(),
                Dispatch::Custom("Spongebob".to_string()),
            )])),
        });

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
            body: KeymapLegendBody::Positional(keymaps),
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
