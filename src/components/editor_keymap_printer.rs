use super::{
    editor::IfCurrentNotFound,
    editor_keymap::alted,
    editor_keymap_legend::{
        extend_mode_normal_mode_override, multicursor_mode_normal_mode_override,
    },
    file_explorer::file_explorer_normal_mode_override,
    keymap_legend::{Keymap, Keymaps},
    suggestive_editor::completion_item_keymaps,
};
use crate::{
    app::Scope,
    components::{
        editor::Editor,
        editor_keymap::{shifted, KeyboardLayout, KEYBOARD_LAYOUT},
    },
    context::Context,
};
use comfy_table::{
    Cell, CellAlignment,
    ColumnConstraint::{self, Absolute},
    Table,
    Width::{self, Fixed},
};

#[derive(Debug, Clone)]
pub(crate) struct KeymapPrintSection {
    name: String,
    keys: Vec<Vec<Key>>,
}

#[derive(Debug, Clone)]
struct Key {
    normal: Option<Keymap>,
    shifted: Option<Keymap>,
    alted: Option<Keymap>,
}
impl Key {
    fn has_content(&self) -> bool {
        self.normal.is_some() || self.shifted.is_some() || self.alted.is_some()
    }

    fn display(&self, show_shift_alt_keys: bool) -> String {
        if show_shift_alt_keys {
            [
                self.alted
                    .as_ref()
                    .map(|key| key.display())
                    .unwrap_or_default(),
                self.shifted
                    .as_ref()
                    .map(|key| key.display())
                    .unwrap_or_default(),
                self.normal
                    .as_ref()
                    .map(|key| key.display())
                    .unwrap_or_default(),
            ]
            .join("\n")
        } else {
            self.normal
                .as_ref()
                .map(|key| key.display())
                .unwrap_or_default()
        }
    }
}

impl KeymapPrintSection {
    pub(crate) fn from_keymaps(
        name: String,
        keymaps: &Keymaps,
        keyboard_layout: &KeyboardLayout,
    ) -> Self {
        KeymapPrintSection {
            name,
            keys: keyboard_layout
                .iter()
                .map(|row| {
                    row.iter()
                        .map(|cell| Key {
                            normal: keymaps
                                .iter()
                                .find(|keymap| keymap.event().display() == *cell)
                                .cloned(),
                            shifted: keymaps
                                .iter()
                                .find(|keymap| {
                                    keymap.event().display().replace("shift+", "") == shifted(cell)
                                })
                                .cloned(),
                            alted: keymaps
                                .iter()
                                .find(|keymap| keymap.event().display() == alted(cell))
                                .cloned(),
                        })
                        .collect()
                })
                .collect(),
        }
    }

    pub(crate) fn has_content(&self) -> bool {
        self.keys
            .iter()
            .any(|keys| keys.iter().any(|key| key.has_content()))
    }

    pub(crate) fn display(&self, terminal_width: u16, show_shift_alt_keys: bool) -> String {
        let max_column_width = terminal_width / 11;
        let mut table = Table::new();
        let table_rows = self.keys.iter().map(|row| {
            let mut cols: Vec<Cell> = row
                .iter()
                .map(|key| {
                    let display = key.display(show_shift_alt_keys);
                    Cell::new(display).set_alignment(CellAlignment::Center)
                })
                .collect();

            cols.insert(
                5,
                Cell::new(if show_shift_alt_keys {
                    "⌥\n⇧\n∅"
                } else {
                    "∅"
                }),
            );

            cols
        });

        let get_column_constraint = |column_index: usize| {
            let min_width = self
                .keys
                .iter()
                .filter_map(|row| row.get(column_index))
                .map(|key| key.display(show_shift_alt_keys).len())
                .max()
                .unwrap_or_default() as u16;
            ColumnConstraint::LowerBoundary(Width::Fixed(min_width.min(max_column_width)))
        };

        table
            .add_rows(table_rows)
            .set_constraints(vec![
                get_column_constraint(0),
                get_column_constraint(1),
                get_column_constraint(2),
                get_column_constraint(3),
                get_column_constraint(4),
                Absolute(Fixed(1)),
                get_column_constraint(5),
                get_column_constraint(6),
                get_column_constraint(7),
                get_column_constraint(8),
                get_column_constraint(9),
            ])
            .set_width(terminal_width)
            .load_preset(comfy_table::presets::UTF8_FULL)
            .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS);

        format!("{}", table)
    }
}

type KeymapPrintSections = Vec<KeymapPrintSection>;

fn collect_keymap_print_sections(layout: &KeyboardLayout) -> KeymapPrintSections {
    let editor = Editor::from_text(Option::None, "");
    let sections: Vec<KeymapPrintSection> = [
        KeymapPrintSection::from_keymaps(
            "Insert".to_string(),
            &editor.insert_mode_keymaps(false),
            layout,
        ),
        KeymapPrintSection::from_keymaps(
            "Normal".to_string(),
            &editor.normal_mode_keymaps(&Context::default(), Default::default()),
            layout,
        ),
        KeymapPrintSection::from_keymaps(
            "Movements".to_string(),
            &Keymaps::new(&editor.keymap_core_movements()),
            layout,
        ),
        KeymapPrintSection::from_keymaps(
            "Primary Selection Modes".to_string(),
            &Keymaps::new(&editor.keymap_primary_selection_modes()),
            layout,
        ),
        KeymapPrintSection::from_keymaps(
            "Secondary Selection Modes Init".to_string(),
            &Keymaps::new(&editor.keymap_secondary_selection_modes_init(&Context::default())),
            layout,
        ),
        KeymapPrintSection::from_keymaps(
            "Secondary Selection Modes (Local Forward)".to_string(),
            &editor
                .secondary_selection_modes_keymap_legend_config(
                    &Context::default(),
                    Scope::Local,
                    IfCurrentNotFound::LookForward,
                )
                .keymaps(),
            layout,
        ),
        KeymapPrintSection::from_keymaps(
            "Secondary Selection Modes (Local Backward)".to_string(),
            &editor
                .secondary_selection_modes_keymap_legend_config(
                    &Context::default(),
                    Scope::Local,
                    IfCurrentNotFound::LookBackward,
                )
                .keymaps(),
            layout,
        ),
        KeymapPrintSection::from_keymaps(
            "Secondary Selection Modes (Global)".to_string(),
            &editor
                .secondary_selection_modes_keymap_legend_config(
                    &Context::default(),
                    Scope::Global,
                    IfCurrentNotFound::LookForward,
                )
                .keymaps(),
            layout,
        ),
        KeymapPrintSection::from_keymaps(
            "Actions".to_string(),
            &Keymaps::new(&editor.keymap_actions(&Default::default(), false)),
            layout,
        ),
        KeymapPrintSection::from_keymaps(
            "Other movements".to_string(),
            &Keymaps::new(&editor.keymap_other_movements()),
            layout,
        ),
        KeymapPrintSection::from_keymaps(
            "Space".to_string(),
            &editor
                .space_keymap_legend_config(&Default::default())
                .keymaps(),
            layout,
        ),
        KeymapPrintSection::from_keymaps(
            "File Explorer Actions".to_string(),
            &Keymaps::new(&editor.keymap_overridable(&file_explorer_normal_mode_override(), true)),
            layout,
        ),
        KeymapPrintSection::from_keymaps(
            "Extend".to_string(),
            &Keymaps::new(&editor.keymap_overridable(&extend_mode_normal_mode_override(), true)),
            layout,
        ),
        KeymapPrintSection::from_keymaps(
            "Sub Modes".to_string(),
            &Keymaps::new(&editor.keymap_sub_modes(&Default::default())),
            layout,
        ),
        KeymapPrintSection::from_keymaps(
            "Multi-cursor".to_string(),
            &Keymaps::new(
                &editor.keymap_overridable(&multicursor_mode_normal_mode_override(), true),
            ),
            layout,
        ),
        KeymapPrintSection::from_keymaps(
            "Completion Items".to_string(),
            &completion_item_keymaps(),
            layout,
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
        true,
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
    for row in section.keys.iter() {
        let row_strings: Vec<String> = row.iter().map(|key| key.display(true)).collect();

        println!("    - [\"{}\"]", row_strings.join("\", \""));
    }

    println!("    - [\"\", \"\", \"\", \"\", \"\", \"\"]");
}
