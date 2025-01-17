use std::{cell::RefCell, rc::Rc};

use crate::{
    app::Dispatch::SetLastActionDescription,
    buffer::Buffer,
    components::{
        dropdown::DropdownItem,
        editor::Mode,
        editor_keymap::{shifted, KeyboardLayout, Meaning, KEYBOARD_LAYOUT},
    },
    context::Context,
    lsp::completion::Completion,
};
use comfy_table::{Cell, CellAlignment, ColumnConstraint::Absolute, Table, Width::Fixed};
use event::{parse_key_event, KeyEvent, KeyModifiers};
use itertools::Itertools;

use super::{
    component::Component,
    editor_keymap::{alted, KeyboardMeaningLayout},
    keymap_legend::Keymap,
    suggestive_editor::{SuggestiveEditor, SuggestiveEditorFilter},
};

#[derive(Debug, Clone)]
pub(crate) struct KeymapPrintSection {
    name: String,
    key_meanings: Vec<Vec<Option<String>>>,
}

impl KeymapPrintSection {
    pub(crate) fn new(
        keyboard_layout: &KeyboardLayout,
        mode: Mode,
        modifiers: KeyModifiers,
        initial_key_events: Option<Vec<String>>,
    ) -> Self {
        KeymapPrintSection {
            name: if modifiers == KeyModifiers::None {
                format!("{mode:?}",)
            } else {
                format!("{mode:?} {:?}", modifiers)
            },
            key_meanings: KeymapPrintSection::keyboard_layout_to_keymaps(
                keyboard_layout,
                mode,
                modifiers,
                initial_key_events.map(|key_events| {
                    key_events
                        .into_iter()
                        .map(|key_event| parse_key_event(&key_event).unwrap())
                        .collect_vec()
                }),
            ),
        }
    }

    pub(crate) fn from_keymaps(
        name: String,
        keymaps: &[Keymap],
        keyboard_layout: &KeyboardLayout,
    ) -> Self {
        KeymapPrintSection {
            name,
            key_meanings: keyboard_layout
                .into_iter()
                .map(|row| {
                    row.iter()
                        .map(|cell| {
                            Some(
                                keymaps
                                    .iter()
                                    .filter_map(|keymap| {
                                        let key_event = keymap.event().display();
                                        let description = keymap
                                            .short_description
                                            .clone()
                                            .unwrap_or_else(|| keymap.description.clone());
                                        let compare_key_event = |modifier: &str| {
                                            key_event.contains(modifier)
                                                && key_event
                                                    .replace(&format!("{modifier}+"), "")
                                                    .to_lowercase()
                                                    == cell.to_lowercase()
                                        };
                                        if key_event == **cell {
                                            Some(description.clone())
                                        } else if key_event == shifted(*cell) {
                                            Some(format!("⇧ {description}"))
                                        } else if key_event == alted(*cell) {
                                            Some(format!("⌥ {description}"))
                                        } else {
                                            None
                                        }
                                    })
                                    .collect_vec()
                                    .join("\n"),
                            )
                        })
                        .collect()
                })
                .collect(),
        }
    }

    pub(crate) fn has_content(&self) -> bool {
        self.key_meanings
            .iter()
            .any(|meanings| meanings.iter().any(Option::is_some))
    }

    fn keyboard_layout_to_keymaps(
        keyboard_layout: &KeyboardLayout,
        mode: Mode,
        modifiers: KeyModifiers,
        initial_key_events: Option<Vec<KeyEvent>>,
    ) -> Vec<Vec<Option<String>>> {
        keyboard_layout
            .iter()
            .map(|row| {
                row.iter()
                    .map(|key| {
                        let mut editor = SuggestiveEditor::from_buffer(
                            Rc::new(RefCell::new(Buffer::new(None, ""))),
                            SuggestiveEditorFilter::CurrentLine,
                        );
                        // This is necessary for extracting Next/Prev Completion Item keybindings
                        editor.set_completion(Completion {
                            items: [
                                DropdownItem::new("foo".to_string()),
                                DropdownItem::new("bar".to_string()),
                            ]
                            .to_vec(),
                            trigger_characters: vec![],
                        });
                        editor.editor_mut().mode = mode.clone();
                        let context = Context::default();
                        for key_event in initial_key_events.clone().unwrap_or_default() {
                            let _ = editor
                                .editor_mut()
                                .handle_key_event(&context, key_event)
                                .unwrap();
                        }
                        let key_event =
                            parse_key_event(&Self::generate_key_string(modifiers.clone(), key))
                                .ok()?;
                        let dispatches = editor.handle_key_event(&context, key_event).ok()?;
                        let dispatches = dispatches.into_vec();
                        dispatches.into_iter().find_map(|dispatch| match dispatch {
                            SetLastActionDescription {
                                long_description,
                                short_description,
                            } => Some(short_description.unwrap_or(long_description)),
                            _ => None,
                        })
                    })
                    .collect()
            })
            .collect()
    }

    fn generate_key_string(modifiers: KeyModifiers, key: &'static str) -> String {
        // The modifiers act differently with shifted ,./;' keys. They actually should not be Shift+,
        // or Shift+., etc... but <, >, etc... This function will map the Shift modifier
        // and characters correctly.

        let (modifier, key) = if modifiers == KeyModifiers::Shift {
            (Self::shifted_modifier(key), shifted(key))
        } else {
            (modifiers.clone(), key)
        };

        let modifier_joiner = match modifier {
            KeyModifiers::None => "",
            _ => "+",
        };

        format!("{}{}{}", modifier.to_string(), modifier_joiner, key)
    }

    fn shifted_modifier(key: &'static str) -> KeyModifiers {
        match key {
            "," => KeyModifiers::None,
            "." => KeyModifiers::None,
            "/" => KeyModifiers::None,
            ";" => KeyModifiers::None,
            "'" => KeyModifiers::None,
            _ => KeyModifiers::Shift,
        }
    }

    pub(crate) fn display(&self, terminal_width: u16) -> String {
        let column_width = terminal_width / 11;
        let mut table = Table::new();
        let table_rows = self.key_meanings.iter().map(|row| {
            let mut cols: Vec<Cell> = row
                .iter()
                .map(|value| {
                    let display = match value {
                        Some(value) => value.to_string(),
                        None => "".to_string(),
                    };

                    Cell::new(display).set_alignment(CellAlignment::Center)
                })
                .collect();

            cols.insert(5, Cell::new(""));

            cols
        });

        table
            .add_rows(table_rows)
            .set_constraints(vec![
                Absolute(Fixed(column_width)),
                Absolute(Fixed(column_width)),
                Absolute(Fixed(column_width)),
                Absolute(Fixed(column_width)),
                Absolute(Fixed(column_width)),
                Absolute(Fixed(1)),
                Absolute(Fixed(column_width)),
                Absolute(Fixed(column_width)),
                Absolute(Fixed(column_width)),
                Absolute(Fixed(column_width)),
                Absolute(Fixed(column_width)),
            ])
            .load_preset(comfy_table::presets::UTF8_FULL)
            .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS);

        format!("{}", table)
    }
}

type KeymapPrintSections = Vec<KeymapPrintSection>;

fn collect_keymap_print_sections(layout: &KeyboardLayout) -> KeymapPrintSections {
    use KeyModifiers::*;
    use Mode::*;
    let sections: Vec<KeymapPrintSection> = [
        KeymapPrintSection::new(layout, Normal, None, Option::None),
        KeymapPrintSection::new(layout, Normal, Shift, Option::None),
        KeymapPrintSection::new(layout, Normal, Ctrl, Option::None),
        KeymapPrintSection::new(layout, Normal, Alt, Option::None),
        KeymapPrintSection::new(layout, MultiCursor, None, Option::None),
        KeymapPrintSection::new(layout, MultiCursor, Shift, Option::None),
        KeymapPrintSection::new(layout, V, None, Option::None),
        KeymapPrintSection::new(layout, Insert, Alt, Option::None),
        // Global keymap
        KeymapPrintSection::new(
            layout,
            Normal,
            None,
            Option::Some([KEYBOARD_LAYOUT.get_key(&Meaning::Globl).to_string()].to_vec()),
        ),
    ]
    .to_vec();

    sections
        .into_iter()
        .filter(|section| section.has_content())
        .collect()
}

/// Print an ASCII representation of the keymap.
pub(crate) fn print_keymap_table() -> anyhow::Result<()> {
    collect_keymap_print_sections(KEYBOARD_LAYOUT.get_keyboard_layout())
        .iter()
        .for_each(print_single_keymap_table);

    Ok(())
}

fn print_single_keymap_table(keymap: &KeymapPrintSection) {
    println!("{}:", keymap.name);

    let table = keymap.display(
        crossterm::terminal::size()
            .map(|(terminal_width, _)| terminal_width / 11)
            .unwrap_or(8),
    );

    println!("{}", table);
    println!();
}

/// Print a YAML representation of the keymap suitable for use with keymap drawer,
/// https://keymap-drawer.streamlit.app
pub(crate) fn print_keymap_drawer_yaml() -> anyhow::Result<()> {
    println!("layout:");
    println!("  qmk_keyboard: corne_rotated");
    println!("  layout_name: LAYOUT_split_3x5_3");
    println!("layers:");

    collect_keymap_print_sections(KEYBOARD_LAYOUT.get_keyboard_layout())
        .iter()
        .for_each(print_keymap_drawer);

    println!("draw_config:");
    println!("  key_w: 82");
    println!("  key_h: 72");
    println!("  footer_text: Keymap for the <a href=\"https://ki-editor.github.io/ki-editor/\">Ki editor</a>");

    Ok(())
}

fn print_keymap_drawer(section: &KeymapPrintSection) {
    let safe_name = section
        .name
        .replace(" ", "_")
        .replace("-", "_")
        .replace("+", "plus");

    println!("  {}:", safe_name);
    for row in section.key_meanings.iter() {
        let row_strings: Vec<&str> = row
            .iter()
            .map(|meaning| match meaning {
                Some(display) => display,
                None => "",
            })
            .collect();

        println!("    - [\"{}\"]", row_strings.join("\", \""));
    }

    println!("    - [\"\", \"\", \"\", \"\", \"\", \"\"]");
}
