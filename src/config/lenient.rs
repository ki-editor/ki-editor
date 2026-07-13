use std::collections::HashMap;

use crate::components::editor_keymap::KeyboardLayoutKeys;
use crate::scripting::Keybinding;
use shared::language::Language;

use super::{LeaderKeymap, RawConfig};
use crate::app::StatusLine;

/// The result of leniently extracting a config value: the value itself
/// (already falling back to a default wherever something couldn't be
/// parsed) paired with a human-readable message for each thing that failed.
pub(super) struct Extracted<T> {
    pub(super) value: T,
    pub(super) errors: Vec<String>,
}

impl<T> From<(T, Vec<String>)> for Extracted<T> {
    fn from((value, errors): (T, Vec<String>)) -> Self {
        Self { value, errors }
    }
}

/// Extracts `RawConfig` from `figment` field-by-field instead of in one shot,
/// so a malformed value for one field (or one language entry) falls back to
/// its default instead of taking down the whole config. Returns the resulting
/// config plus a human-readable message for each field that couldn't be parsed.
pub(super) fn extract_lenient(figment: &figment::Figment) -> Extracted<RawConfig> {
    let default = RawConfig::default();

    // If the merged config can't even be resolved into a generic value tree,
    // the file itself is syntactically invalid (e.g. empty or malformed
    // JSON/YAML/TOML): there are no individual fields to lenient-parse, so
    // bail out with one clean message instead of a per-field extraction that
    // would just fail on every field for the same root cause.
    if let Err(error) = figment.find_value("") {
        return (
            default,
            vec![format!(
                "Ki config could not be parsed (invalid syntax), using defaults: {error}"
            )],
        )
            .into();
    }

    macro_rules! extract_field {
        ($field:ident, $key:literal) => {
            match figment.find_value($key) {
                Ok(value) => match serde_path_to_error::deserialize(&value) {
                    Ok(value) => (value, None),
                    Err(error) => (
                        default.$field,
                        Some(format!(
                            "{}, using default value",
                            describe_path_error($key, error)
                        )),
                    ),
                },
                Err(_) => (default.$field, None),
            }
        };
    }

    let Extracted {
        value: languages,
        errors: language_errors,
    } = extract_languages(figment);
    let Extracted {
        value: status_lines,
        errors: status_line_errors,
    } = extract_status_lines(figment, &default);
    let Extracted {
        value: leader_keymap,
        errors: leader_keymap_errors,
    } = extract_leader_keymap(figment, &default);
    let Extracted {
        value: custom_keyboard_layouts,
        errors: custom_layout_errors,
    } = extract_custom_keyboard_layouts(figment);
    let (keyboard_layout, keyboard_layout_error) =
        extract_field!(keyboard_layout, "keyboard_layout");
    let (theme, theme_error) = extract_field!(theme, "theme");
    let (indent_char, indent_char_error) = extract_field!(indent_char, "indent_char");
    let (indent_width, indent_width_error) = extract_field!(indent_width, "indent_width");
    let (show_key_in_keymap, show_key_in_keymap_error) =
        extract_field!(show_key_in_keymap, "show_key_in_keymap");
    let (icon_style, icon_style_error) = extract_field!(icon_style, "icon_style");

    let config = RawConfig {
        languages,
        keyboard_layout,
        theme,
        status_lines,
        leader_keymap,
        custom_keyboard_layouts,
        indent_char,
        indent_width,
        show_key_in_keymap,
        icon_style,
    };

    let errors = language_errors
        .into_iter()
        .chain(status_line_errors)
        .chain(leader_keymap_errors)
        .chain(custom_layout_errors)
        .chain(keyboard_layout_error)
        .chain(theme_error)
        .chain(indent_char_error)
        .chain(indent_width_error)
        .chain(show_key_in_keymap_error)
        .chain(icon_style_error)
        .collect();

    (config, errors).into()
}

/// Formats a `serde_path_to_error` error as a fully-qualified, dotted config
/// path (e.g. "`languages.javascript.formatter.arguments`: invalid type: ...")
/// instead of just the leaf error message, so the user knows exactly which
/// value in the config file is wrong.
fn describe_path_error(prefix: &str, error: serde_path_to_error::Error<figment::Error>) -> String {
    let path = error.path().to_string();
    let full_path = if path == "." {
        prefix.to_string()
    } else {
        format!("{prefix}.{path}")
    };
    format!("`{full_path}` {}", error.into_inner())
}

/// Parses the `languages` map entry-by-entry, and within each entry,
/// field-by-field, so a malformed value for one field of one language (e.g.
/// a bad `formatter`) only resets that field to its default instead of
/// dropping the whole language override, let alone the entire config.
fn extract_languages(figment: &figment::Figment) -> Extracted<HashMap<String, Language>> {
    let languages = shared::languages::languages();
    match figment.find_value("languages") {
        Ok(value) => {
            if let Some(dict) = value.into_dict() {
                dict.into_iter().fold(
                    (languages, Vec::new()),
                    |(mut languages, mut errors), (name, entry)| {
                        match serde_path_to_error::deserialize::<_, serde_json::Value>(&entry) {
                            Ok(entry_json) => {
                                let default_language =
                                    languages.get(&name).cloned().unwrap_or_default();
                                let (language, field_errors) =
                                    Language::extract_lenient(&entry_json, &default_language);
                                errors.extend(field_errors.into_iter().map(
                                    |(field_path, message)| {
                                        let full_path = if field_path.is_empty() {
                                            format!("languages.{name}")
                                        } else {
                                            format!("languages.{name}.{field_path}")
                                        };
                                        format!("`{full_path}` {message}, using default value")
                                    },
                                ));
                                languages.insert(name, language);
                            }
                            Err(error) => {
                                errors.push(format!(
                                    "{}, ignoring language `{name}`",
                                    describe_path_error(&format!("languages.{name}"), error)
                                ));
                            }
                        }
                        (languages, errors)
                    },
                )
            } else {
                (
                    languages,
                    vec!["`languages` config must be an object/table.".to_string()],
                )
            }
        }
        Err(error) => (
            languages,
            vec![format!("Failed to parse `languages`: {error}")],
        ),
    }
    .into()
}

/// Parses `custom_keyboard_layouts` entry-by-entry, so a malformed layout
/// only drops that one entry instead of every custom layout the user
/// defined.
fn extract_custom_keyboard_layouts(
    figment: &figment::Figment,
) -> Extracted<HashMap<String, KeyboardLayoutKeys>> {
    match figment.find_value("custom_keyboard_layouts") {
        Ok(value) => {
            if let Some(dict) = value.into_dict() {
                dict.into_iter().fold(
                    (HashMap::new(), Vec::new()),
                    |(mut layouts, mut errors), (name, entry)| {
                        match serde_path_to_error::deserialize(&entry) {
                            Ok(keys) => {
                                layouts.insert(name, keys);
                            }
                            Err(error) => {
                                errors.push(format!(
                                    "{}, ignoring custom keyboard layout `{name}`",
                                    describe_path_error(
                                        &format!("custom_keyboard_layouts.{name}"),
                                        error
                                    )
                                ));
                            }
                        }
                        (layouts, errors)
                    },
                )
            } else {
                (
                    HashMap::new(),
                    vec!["`custom_keyboard_layouts` config must be an object/table.".to_string()],
                )
            }
        }
        // Absent entirely (e.g. not specified by the user), which is fine: it defaults to empty.
        Err(_) => (HashMap::new(), Vec::new()),
    }
    .into()
}

/// Parses `status_lines` entry-by-entry, so a malformed status line only
/// drops that one line, not resetting the whole status line configuration
/// back to the default.
fn extract_status_lines(
    figment: &figment::Figment,
    default: &RawConfig,
) -> Extracted<Vec<StatusLine>> {
    match figment.find_value("status_lines") {
        Ok(value) => {
            if let Some(array) = value.into_array() {
                let (status_lines, errors): (Vec<_>, Vec<_>) = array
                    .into_iter()
                    .enumerate()
                    .map(
                        |(index, entry)| match serde_path_to_error::deserialize(&entry) {
                            Ok(status_line) => (Some(status_line), None),
                            Err(error) => (
                                None,
                                Some(format!(
                                    "{}, ignoring this status line",
                                    describe_path_error(&format!("status_lines.{index}"), error)
                                )),
                            ),
                        },
                    )
                    .unzip();
                (
                    status_lines.into_iter().flatten().collect(),
                    errors.into_iter().flatten().collect(),
                )
            } else {
                (
                    default.status_lines.clone(),
                    vec!["`status_lines` config must be an array, using default value".to_string()],
                )
            }
        }
        Err(_) => (default.status_lines.clone(), Vec::new()),
    }
    .into()
}

/// Parses `leader_keymap` cell-by-cell, so a malformed keybinding (e.g. one
/// pointing to a missing script) only clears that one cell instead of
/// wiping out every keybinding the user configured.
fn extract_leader_keymap(
    figment: &figment::Figment,
    default: &RawConfig,
) -> Extracted<LeaderKeymap> {
    match figment.find_value("leader_keymap") {
        Ok(value) => {
            let Some(rows) = value.into_array() else {
                return (
                    default.leader_keymap.clone(),
                    vec![
                        "`leader_keymap` config must be an array, using default value".to_string(),
                    ],
                )
                    .into();
            };
            let (grid, errors) = rows.into_iter().enumerate().take(3).fold(
                (<[[Option<Keybinding>; 10]; 3]>::default(), Vec::new()),
                |(mut grid, mut errors), (row_index, row)| {
                    let Some(cells) = row.into_array() else {
                        errors.push(format!(
                            "`leader_keymap.{row_index}` must be an array, using default value for this row"
                        ));
                        return (grid, errors);
                    };
                    for (col_index, cell) in cells.into_iter().enumerate().take(10) {
                        match serde_path_to_error::deserialize(&cell) {
                            Ok(keybinding) => grid[row_index][col_index] = keybinding,
                            Err(error) => {
                                errors.push(format!(
                                    "{}, clearing this keybinding",
                                    describe_path_error(
                                        &format!("leader_keymap.{row_index}.{col_index}"),
                                        error
                                    )
                                ));
                            }
                        }
                    }
                    (grid, errors)
                },
            );
            (LeaderKeymap(grid), errors)
        }
        Err(_) => (default.leader_keymap.clone(), Vec::new()),
    }
    .into()
}
