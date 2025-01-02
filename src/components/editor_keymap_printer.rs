use crate::components::{
    editor::{Editor, Mode},
    editor_keymap::{shifted, KeyboardLayout, KEYBOARD_LAYOUT},
    keymap_legend::Keymaps,
};
use crate::context::Context;
use comfy_table::{Cell, CellAlignment, ColumnConstraint::Absolute, Table, Width::Fixed};
use crossterm::event::KeyCode;
use event::{KeyEvent, KeyModifiers};

#[derive(Debug, Clone)]
struct KeymapPrintSection {
    name: String,
    key_meanings: Vec<Vec<Option<String>>>,
}

impl KeymapPrintSection {
    pub fn new(
        name: &str,
        keyboard_layout: &KeyboardLayout,
        modifiers: KeyModifiers,
        keymaps: &Vec<Keymaps>,
    ) -> Self {
        KeymapPrintSection {
            name: name.to_string(),
            key_meanings: KeymapPrintSection::keyboard_layout_to_keymaps(
                keyboard_layout,
                modifiers,
                keymaps,
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
        modifiers: KeyModifiers,
        keymaps: &Vec<Keymaps>,
    ) -> Vec<Vec<Option<String>>> {
        keyboard_layout
            .iter()
            .map(|row| {
                row.iter()
                    .map(|key| {
                        let key_event = match modifiers {
                            KeyModifiers::Shift => KeyEvent {
                                code: KeyCode::Char(shifted(key).chars().next().unwrap()),
                                modifiers: KeymapPrintSection::shifted_modifier(key),
                            },
                            _ => KeyEvent {
                                code: KeyCode::Char(key.chars().next().unwrap()),
                                modifiers: modifiers.clone(),
                            },
                        };

                        match keymaps.iter().find_map(|keymap| keymap.get(&key_event)) {
                            // One may wonder why not simply return the short_description as it is
                            // an Option<String> but by detecting None here and replacing it with
                            // ??? we are able to see where we may not be providing a short
                            // description in code.
                            Some(keymap) => match keymap.short_description.clone() {
                                Some(short) => Some(short),
                                None => Some("???".to_string()),
                            },
                            // There truly is no key mapping for this key event.
                            None => None,
                        }
                    })
                    .collect()
            })
            .collect()
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
}

type KeymapPrintSections = Vec<KeymapPrintSection>;

fn collect_keymap_print_sections(layout: &KeyboardLayout) -> KeymapPrintSections {
    let context = Context::default();
    let mut editor = Editor::from_text(None, "");
    let normal_keymaps: Vec<Keymaps> = editor.normal_mode_keymap_legend_config(&context).into();

    editor.mode = Mode::MultiCursor;
    let multicursor_keymaps: Vec<Keymaps> =
        editor.normal_mode_keymap_legend_config(&context).into();

    let mut vmode_keymaps = normal_keymaps.clone();
    vmode_keymaps.insert(0, editor.visual_mode_initialized_keymaps());

    let sections: Vec<KeymapPrintSection> = [
        KeymapPrintSection::new("Normal", &layout, KeyModifiers::None, &normal_keymaps),
        KeymapPrintSection::new(
            "Normal Shift",
            &layout,
            KeyModifiers::Shift,
            &normal_keymaps,
        ),
        KeymapPrintSection::new(
            "Normal Control",
            &layout,
            KeyModifiers::Ctrl,
            &normal_keymaps,
        ),
        KeymapPrintSection::new(
            "Normal Alternate",
            &layout,
            KeyModifiers::Alt,
            &normal_keymaps,
        ),
        KeymapPrintSection::new(
            "Multi-Cursor",
            &layout,
            KeyModifiers::None,
            &multicursor_keymaps,
        ),
        KeymapPrintSection::new(
            "Multi-Cursor Shift",
            &layout,
            KeyModifiers::Shift,
            &multicursor_keymaps,
        ),
        KeymapPrintSection::new("V-mode", &layout, KeyModifiers::None, &vmode_keymaps),
        KeymapPrintSection::new(
            "Insert",
            &layout,
            KeyModifiers::Alt,
            &editor.insert_mode_keymap_legend_config().into(),
        ),
    ]
    .to_vec();

    sections.into_iter().filter(|km| km.has_content()).collect()
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
    println!("  footer_text: Keymap for the <a href=\"https://ki-editor.github.io/ki-editor/\">Ki editor</a>");

    Ok(())
}

fn print_keymap_drawer(section: &KeymapPrintSection) {
    let safe_name = section.name.replace(" ", "_").replace("-", "_");

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
