use crate::components::{
    editor::Editor,
    editor_keymap::{shifted, KeyboardLayout, KEYBOARD_LAYOUT},
};
use crate::context::Context;
use crate::themes::vscode_light;
use comfy_table::{Cell, CellAlignment, ColumnConstraint::Absolute, Table, Width::Fixed};
use event::{parse_key_event, KeyModifiers};

use super::editor::Mode;

#[derive(Debug, Clone)]
struct KeymapPrintSection {
    name: String,
    key_meanings: Vec<Vec<Option<String>>>,
}

impl KeymapPrintSection {
    pub fn new(keyboard_layout: &KeyboardLayout, mode: Mode, modifiers: KeyModifiers) -> Self {
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
            ),
        }
    }

    pub fn has_content(&self) -> bool {
        self.key_meanings
            .iter()
            .any(|meanings| meanings.iter().any(Option::is_some))
    }

    fn keyboard_layout_to_keymaps(
        keyboard_layout: &KeyboardLayout,
        mode: Mode,
        modifiers: KeyModifiers,
    ) -> Vec<Vec<Option<String>>> {
        keyboard_layout
            .iter()
            .map(|row| {
                row.iter()
                    .map(|key| {
                        let mut editor = Editor::from_text(None, "");
                        editor.mode = mode.clone();
                        let context = Context::default();
                        let (modifier, key) = if modifiers == KeyModifiers::Shift {
                            (Self::shifted_modifier(key), shifted(key))
                        } else {
                            (modifiers.clone(), *key)
                        };

                        let modifier_str = match modifier {
                            KeyModifiers::Shift => "shift+",
                            KeyModifiers::Alt => "alt+",
                            KeyModifiers::Ctrl => "ctrl+",
                            _ => "",
                        };

                        let key_str = format!("{}{}", modifier_str, key);
                        let key_event = parse_key_event(&key_str).ok()?;
                        let dispatches = editor.handle_key_event(&context, key_event).ok()?;

                        let dispatches = dispatches.into_vec();
                        dispatches.into_iter().find_map(|dispatch| match dispatch {
                            crate::app::Dispatch::SetLastActionDescription {
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
    fn shifted_modifier(key: &&'static str) -> KeyModifiers {
        match *key {
            "," => KeyModifiers::None,
            "." => KeyModifiers::None,
            "/" => KeyModifiers::None,
            ";" => KeyModifiers::None,
            "'" => KeyModifiers::None,
            _ => KeyModifiers::Shift,
        }
    }
}

type KeymapPrintSections = Vec<KeymapPrintSection>;

fn collect_keymap_print_sections(layout: &KeyboardLayout) -> KeymapPrintSections {
    use KeyModifiers::*;
    use Mode::*;
    let sections: Vec<KeymapPrintSection> = [
        KeymapPrintSection::new(&layout, Normal, None),
        KeymapPrintSection::new(&layout, Normal, Shift),
        KeymapPrintSection::new(&layout, Normal, Ctrl),
        KeymapPrintSection::new(&layout, Normal, Alt),
        KeymapPrintSection::new(&layout, MultiCursor, None),
        KeymapPrintSection::new(&layout, MultiCursor, Shift),
        KeymapPrintSection::new(&layout, V, None),
        KeymapPrintSection::new(&layout, Insert, Alt),
    ]
    .to_vec();

    sections
        .into_iter()
        .filter(|section| section.has_content())
        .collect()
}

/// Print an ASCII representation of the keymap.
pub fn print_keymap_table() -> anyhow::Result<()> {
    collect_keymap_print_sections(KEYBOARD_LAYOUT.as_keyboard_layout())
        .iter()
        .for_each(print_single_keymap_table);

    Ok(())
}

fn print_single_keymap_table(keymap: &KeymapPrintSection) {
    println!("{}:", keymap.name);

    let mut table = Table::new();
    let table_rows = keymap.key_meanings.iter().map(|row| {
        let mut cols: Vec<Cell> = row
            .into_iter()
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
            Absolute(Fixed(8)),
            Absolute(Fixed(8)),
            Absolute(Fixed(8)),
            Absolute(Fixed(8)),
            Absolute(Fixed(8)),
            Absolute(Fixed(1)),
            Absolute(Fixed(8)),
            Absolute(Fixed(8)),
            Absolute(Fixed(8)),
            Absolute(Fixed(8)),
            Absolute(Fixed(8)),
        ])
        .load_preset(comfy_table::presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS);

    println!("{}", table);
    println!("");
}

/// Print a YAML representation of the keymap suitable for use with keymap drawer,
/// https://keymap-drawer.streamlit.app
pub fn print_keymap_drawer_yaml() -> anyhow::Result<()> {
    println!("layout:");
    println!("  qmk_keyboard: corne_rotated");
    println!("  layout_name: LAYOUT_split_3x5_3");
    println!("layers:");

    collect_keymap_print_sections(KEYBOARD_LAYOUT.as_keyboard_layout())
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
