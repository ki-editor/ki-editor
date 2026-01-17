use super::{
    editor::IfCurrentNotFound,
    editor_keymap::alted,
    editor_keymap_legend::extend_mode_normal_mode_override,
    file_explorer::file_explorer_normal_mode_override,
    keymap_legend::{Keybinding, Keymap},
    suggestive_editor::completion_item_keymap,
};
use crate::{
    app::Scope,
    components::{
        editor::Editor,
        editor_keymap::{shifted, KeyboardLayout},
        editor_keymap_legend::{
            cut_keymap, delete_keymap, multicursor_keymap, paste_keymap, swap_keymap,
        },
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
use shared::canonicalized_path::CanonicalizedPath;

#[derive(Debug, Clone)]
pub struct KeymapPrintSection {
    name: String,
    keys: Vec<Vec<Key>>,
}

#[derive(Debug, Clone)]
pub struct Key {
    pub normal: Option<Keybinding>,
    pub shifted: Option<Keybinding>,
    pub alted: Option<Keybinding>,
}

pub struct KeymapDisplayOption {
    pub show_alt: bool,
    pub show_shift: bool,
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
    pub fn from_keymap(name: String, keymap: &Keymap, keyboard_layout: &KeyboardLayout) -> Self {
        KeymapPrintSection {
            name,
            keys: keyboard_layout
                .iter()
                .map(|row| {
                    row.iter()
                        .map(|cell| Key {
                            normal: keymap
                                .iter()
                                .find(|keymap| keymap.event().display() == *cell)
                                .cloned(),
                            shifted: keymap
                                .iter()
                                .find(|keymap| {
                                    keymap.event().display().replace("shift+", "") == shifted(cell)
                                })
                                .cloned(),
                            alted: keymap
                                .iter()
                                .find(|keymap| keymap.event().display() == alted(cell))
                                .cloned(),
                        })
                        .collect()
                })
                .collect(),
        }
    }

    pub fn has_content(&self) -> bool {
        self.keys
            .iter()
            .any(|keys| keys.iter().any(|key| key.has_content()))
    }

    /// Returns None if the terminal width is too small
    pub fn display(&self, terminal_width: usize, option: &KeymapDisplayOption) -> String {
        let table = self.display_full(terminal_width, option);

        fn get_content_width(table: &Table) -> usize {
            let content_width: u16 = table.column_max_content_widths().iter().sum();
            // column content, separators, padding & editor margins
            content_width as usize + 12 + 22 + 2
        }

        let exmatrix_keybindings = ["* Pick Keyboard", "\\ Leader"].join(&" ".repeat(4));
        if get_content_width(&table) < terminal_width {
            format!("{table}\n{exmatrix_keybindings}")
        } else {
            let (left, right) = self.display_stacked(terminal_width, option);
            let content_width = get_content_width(&left).min(get_content_width(&right));
            if content_width < terminal_width {
                format!("{left}\n{right}\n{exmatrix_keybindings}")
            } else {
                "Window is too small to display keymap legend :(".to_string()
            }
        }
    }

    #[cfg(test)]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[cfg(test)]
    pub fn keys(&self) -> &Vec<Vec<Key>> {
        &self.keys
    }

    fn display_full(&self, terminal_width: usize, option: &KeymapDisplayOption) -> Table {
        self.display_one_side(terminal_width, option, 0, 10, 5)
    }

    fn display_stacked(
        &self,
        terminal_width: usize,
        option: &KeymapDisplayOption,
    ) -> (Table, Table) {
        let left = self.display_one_side(terminal_width, option, 0, 5, 5);
        let right = self.display_one_side(terminal_width, option, 5, 5, 0);
        (left, right)
    }

    fn display_one_side(
        &self,
        terminal_width: usize,
        option: &KeymapDisplayOption,
        skip: usize,
        take: usize,
        modifiers_column_index: usize,
    ) -> Table {
        let columns_count = take;
        let max_column_width = terminal_width / columns_count;
        let mut table = Table::new();
        let table_rows = self.keys.iter().map(|row| {
            let cells = row.iter().skip(skip).take(take);
            // Only show alt/shift row if the row contains any alt/shift keybinding
            let option = KeymapDisplayOption {
                show_alt: option.show_alt && cells.clone().any(|cell| cell.alted.is_some()),
                show_shift: option.show_shift && cells.clone().any(|cell| cell.shifted.is_some()),
            };
            let mut cols: Vec<Cell> = cells
                .map(|key| {
                    let display = key.display(&option);
                    Cell::new(display).set_alignment(CellAlignment::Center)
                })
                .collect();

            cols.insert(
                modifiers_column_index,
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
                .skip(skip)
                .take(take)
                .filter_map(|row| row.get(column_index))
                .map(|key| {
                    key.display(option)
                        .lines()
                        .map(|line| line.chars().count())
                        .max()
                        .unwrap_or_default()
                })
                .max()
                .unwrap_or_default();
            ColumnConstraint::LowerBoundary(Width::Fixed(min_width.min(max_column_width) as u16))
        };

        table
            .add_rows(table_rows)
            .set_constraints({
                let mut columns = (0..columns_count).map(get_column_constraint).collect_vec();
                columns.insert(modifiers_column_index, Absolute(Fixed(1)));
                columns
            })
            .set_width(terminal_width as u16)
            .load_preset(comfy_table::presets::UTF8_FULL)
            .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS);
        table
    }
}

pub struct KeymapPrintSections {
    #[allow(unused)]
    context: Context,
    sections: Vec<KeymapPrintSection>,
}

impl KeymapPrintSections {
    pub fn new() -> Self {
        let context = Context::new(CanonicalizedPath::try_from(".").unwrap(), false, None);
        let layout = context.keyboard_layout_kind().get_keyboard_layout();
        let editor = Editor::from_text(Option::None, "");
        let sections: Vec<KeymapPrintSection> = [
            KeymapPrintSection::from_keymap(
                "Insert".to_string(),
                &editor.insert_mode_keymap(false),
                layout,
            ),
            KeymapPrintSection::from_keymap(
                "Normal".to_string(),
                &Keymap::new(&editor.normal_mode_keymap(Default::default(), None)),
                layout,
            ),
            KeymapPrintSection::from_keymap(
                "Movements".to_string(),
                &Keymap::new(&editor.keymap_core_movements(None)),
                layout,
            ),
            KeymapPrintSection::from_keymap(
                "Primary Selection Modes".to_string(),
                &Keymap::new(&editor.keymap_primary_selection_modes(None)),
                layout,
            ),
            KeymapPrintSection::from_keymap(
                "Secondary Selection Modes Init".to_string(),
                &Keymap::new(&editor.keymap_secondary_selection_modes_init(None)),
                layout,
            ),
            KeymapPrintSection::from_keymap(
                "Secondary Selection Modes (Local)".to_string(),
                &editor
                    .secondary_selection_modes_keymap_legend_config(
                        Scope::Local,
                        IfCurrentNotFound::LookForward,
                        None,
                    )
                    .keymap(),
                layout,
            ),
            KeymapPrintSection::from_keymap(
                "Secondary Selection Modes (Global)".to_string(),
                &editor
                    .secondary_selection_modes_keymap_legend_config(
                        Scope::Global,
                        IfCurrentNotFound::LookForward,
                        None,
                    )
                    .keymap(),
                layout,
            ),
            KeymapPrintSection::from_keymap(
                "Actions".to_string(),
                &Keymap::new(&editor.keymap_actions(&Default::default(), false, None)),
                layout,
            ),
            KeymapPrintSection::from_keymap(
                "Other Movements".to_string(),
                &Keymap::new(&editor.keymap_other_movements()),
                layout,
            ),
            KeymapPrintSection::from_keymap(
                "Space".to_string(),
                &editor.space_keymap_legend_config(&context).keymap(),
                layout,
            ),
            KeymapPrintSection::from_keymap(
                "Space Context".to_string(),
                &editor.space_context_keymap_legend_config().keymap(),
                layout,
            ),
            KeymapPrintSection::from_keymap(
                "Space Editor".to_string(),
                &editor.space_editor_keymap_legend_config().keymap(),
                layout,
            ),
            KeymapPrintSection::from_keymap(
                "Space Pick".to_string(),
                &editor.space_pick_keymap_legend_config().keymap(),
                layout,
            ),
            KeymapPrintSection::from_keymap(
                "File Explorer Actions".to_string(),
                &Keymap::new(
                    &editor.keymap_overridable(&file_explorer_normal_mode_override(), true),
                ),
                layout,
            ),
            KeymapPrintSection::from_keymap(
                "Extend".to_string(),
                &Keymap::new(&editor.keymap_overridable(&extend_mode_normal_mode_override(), true)),
                layout,
            ),
            KeymapPrintSection::from_keymap(
                "Completion Items".to_string(),
                &completion_item_keymap(),
                layout,
            ),
            KeymapPrintSection::from_keymap(
                "Universal Keymap".to_string(),
                &Keymap::new(&editor.keymap_universal()),
                layout,
            ),
            KeymapPrintSection::from_keymap(
                "Transform".to_string(),
                &Keymap::new(&editor.keymap_transform()),
                layout,
            ),
            KeymapPrintSection::from_keymap("Paste".to_string(), &paste_keymap(), layout),
            KeymapPrintSection::from_keymap(
                "Multi-cursor".to_string(),
                &multicursor_keymap(),
                layout,
            ),
            KeymapPrintSection::from_keymap("Cut".to_string(), &cut_keymap(), layout),
            KeymapPrintSection::from_keymap("Swap".to_string(), &swap_keymap(), layout),
            KeymapPrintSection::from_keymap("Delete".to_string(), &delete_keymap(), layout),
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

    pub fn sections(&self) -> &Vec<KeymapPrintSection> {
        &self.sections
    }
}

/// Print an ASCII representation of the keymap.
pub fn print_keymap_table() -> anyhow::Result<()> {
    KeymapPrintSections::new()
        .sections()
        .iter()
        .for_each(print_single_keymap_table);

    Ok(())
}

fn print_single_keymap_table(keymap: &KeymapPrintSection) {
    println!("{}:", keymap.name);

    let table = keymap.display(
        usize::MAX,
        &KeymapDisplayOption {
            show_alt: true,
            show_shift: true,
        },
    );

    println!("{table}");
    println!();
}

/// Print a YAML representation of the keymap suitable for use with keymap drawer,
/// https://keymap-drawer.streamlit.app
pub fn print_keymap_drawer_yaml() -> anyhow::Result<()> {
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

    println!("  {safe_name}:");
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
