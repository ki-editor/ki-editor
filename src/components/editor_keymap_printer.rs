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
        editor_keymap::{shifted, KeyboardLayout},
    },
    context::Context,
};
use comfy_table::{
    Cell, CellAlignment,
    ColumnConstraint::{self, Absolute},
    Table,
    Width::{self, Fixed},
};
use itertools::Itertools;

#[derive(Debug, Clone)]
pub(crate) struct KeymapPrintSection {
    name: String,
    keys: Vec<Vec<Key>>,
}

#[derive(Debug, Clone)]
pub(crate) struct Key {
    pub(crate) normal: Option<Keymap>,
    pub(crate) shifted: Option<Keymap>,
    pub(crate) alted: Option<Keymap>,
}

pub(crate) struct KeymapDisplayOption {
    pub(crate) show_alt: bool,
    pub(crate) show_shift: bool,
}

impl Key {
    fn has_content(&self) -> bool {
        self.normal.is_some() || self.shifted.is_some() || self.alted.is_some()
    }

    fn display(&self, option: &KeymapDisplayOption) -> String {
        [].into_iter()
            .chain(option.show_alt.then(|| {
                format!(
                    "{}\n",
                    self.alted
                        .as_ref()
                        .map(|key| key.display())
                        .unwrap_or_default()
                )
            }))
            .chain(option.show_shift.then(|| {
                format!(
                    "{}\n",
                    self.shifted
                        .as_ref()
                        .map(|key| key.display())
                        .unwrap_or_default()
                )
            }))
            .chain(Some(
                self.normal
                    .as_ref()
                    .map(|key| key.display())
                    .unwrap_or_default(),
            ))
            .collect_vec()
            .join("")
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

    pub(crate) fn display(&self, terminal_width: u16, option: &KeymapDisplayOption) -> String {
        let max_column_width = terminal_width / 11;
        let mut table = Table::new();
        let table_rows = self.keys.iter().map(|row| {
            let mut cols: Vec<Cell> = row
                .iter()
                .map(|key| {
                    let display = key.display(option);
                    Cell::new(display).set_alignment(CellAlignment::Center)
                })
                .collect();

            cols.insert(
                5,
                Cell::new(
                    [
                        if option.show_alt { "⌥\n" } else { "" },
                        if option.show_shift { "⇧\n" } else { "" },
                        "∅",
                    ]
                    .join(""),
                ),
            );

            cols
        });

        let get_column_constraint = |column_index: usize| {
            let min_width = self
                .keys
                .iter()
                .filter_map(|row| row.get(column_index))
                .map(|key| {
                    key.display(option)
                        .lines()
                        .map(|line| line.chars().count())
                        .max()
                        .unwrap_or_default() as u16
                })
                .max()
                .unwrap_or_default();
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

    #[cfg(test)]
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    #[cfg(test)]
    pub(crate) fn keys(&self) -> &Vec<Vec<Key>> {
        &self.keys
    }
}

pub(crate) struct KeymapPrintSections {
    #[allow(unused)]
    context: Context,
    sections: Vec<KeymapPrintSection>,
}

impl KeymapPrintSections {
    pub(crate) fn new() -> Self {
        let context = Context::default();
        let layout = context.keyboard_layout_kind().get_keyboard_layout();
        let editor = Editor::from_text(Option::None, "");
        let context = Context::default();
        let sections: Vec<KeymapPrintSection> = [
            KeymapPrintSection::from_keymaps(
                "Insert".to_string(),
                &editor.insert_mode_keymaps(false, &context),
                layout,
            ),
            KeymapPrintSection::from_keymaps(
                "Normal".to_string(),
                &editor.normal_mode_keymaps(&context, Default::default()),
                layout,
            ),
            KeymapPrintSection::from_keymaps(
                "Movements".to_string(),
                &Keymaps::new(&editor.keymap_core_movements(&context)),
                layout,
            ),
            KeymapPrintSection::from_keymaps(
                "Primary Selection Modes".to_string(),
                &Keymaps::new(&editor.keymap_primary_selection_modes(&context)),
                layout,
            ),
            KeymapPrintSection::from_keymaps(
                "Secondary Selection Modes Init".to_string(),
                &Keymaps::new(&editor.keymap_secondary_selection_modes_init(&context)),
                layout,
            ),
            KeymapPrintSection::from_keymaps(
                "Secondary Selection Modes (Local Forward)".to_string(),
                &editor
                    .secondary_selection_modes_keymap_legend_config(
                        &context,
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
                        &context,
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
                        &context,
                        Scope::Global,
                        IfCurrentNotFound::LookForward,
                    )
                    .keymaps(),
                layout,
            ),
            KeymapPrintSection::from_keymaps(
                "Actions".to_string(),
                &Keymaps::new(&editor.keymap_actions(&Default::default(), false, &context)),
                layout,
            ),
            KeymapPrintSection::from_keymaps(
                "Other movements".to_string(),
                &Keymaps::new(&editor.keymap_other_movements(&context)),
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
                &Keymaps::new(&editor.keymap_overridable(
                    &file_explorer_normal_mode_override(),
                    true,
                    &context,
                )),
                layout,
            ),
            KeymapPrintSection::from_keymaps(
                "Extend".to_string(),
                &Keymaps::new(&editor.keymap_overridable(
                    &extend_mode_normal_mode_override(&context),
                    true,
                    &context,
                )),
                layout,
            ),
            KeymapPrintSection::from_keymaps(
                "Sub Modes".to_string(),
                &Keymaps::new(&editor.keymap_sub_modes(&Default::default(), &context)),
                layout,
            ),
            KeymapPrintSection::from_keymaps(
                "Multi-cursor".to_string(),
                &Keymaps::new(&editor.keymap_overridable(
                    &multicursor_mode_normal_mode_override(),
                    true,
                    &context,
                )),
                layout,
            ),
            KeymapPrintSection::from_keymaps(
                "Completion Items".to_string(),
                &completion_item_keymaps(&context),
                layout,
            ),
            KeymapPrintSection::from_keymaps(
                "Universal Keymap".to_string(),
                &Keymaps::new(&editor.keymap_universal(&context)),
                layout,
            ),
            KeymapPrintSection::from_keymaps(
                "Transform".to_string(),
                &Keymaps::new(&editor.keymap_transform(&context)),
                layout,
            ),
        ]
        .to_vec();

        Self {
            context,
            sections: sections
                .into_iter()
                .filter(|section| section.has_content())
                .collect(),
        }
    }

    pub(crate) fn sections(&self) -> &Vec<KeymapPrintSection> {
        &self.sections
    }
}

/// Print an ASCII representation of the keymap.
pub(crate) fn print_keymap_table() -> anyhow::Result<()> {
    KeymapPrintSections::new()
        .sections()
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
        &KeymapDisplayOption {
            show_alt: true,
            show_shift: true,
        },
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

    KeymapPrintSections::new()
        .sections()
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
        let row_strings: Vec<String> = row
            .iter()
            .map(|key| {
                key.display(&KeymapDisplayOption {
                    show_alt: true,
                    show_shift: true,
                })
            })
            .collect();

        println!("    - [\"{}\"]", row_strings.join("\", \""));
    }

    println!("    - [\"\", \"\", \"\", \"\", \"\", \"\"]");
}
