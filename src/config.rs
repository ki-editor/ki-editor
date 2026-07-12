use std::borrow::Cow;
use std::{collections::HashMap, path::PathBuf};

use anyhow::Context;
use itertools::Itertools;
use regex::Regex;

use crate::app::StatusLine;
use crate::components::editor_keymap::{builtin_layout_map, KeyboardLayout, KeyboardLayoutKeys};
use crate::scripting::{Keybinding, Script};
use crate::themes::Theme;
use figment::providers;
use figment::providers::Format;
use once_cell::sync::OnceCell;
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Serialize};
use shared::absolute_path::AbsolutePath;
use shared::language::{self, Language};

pub struct AppConfig {
    languages: HashMap<String, Language>,
    keyboard_layout: KeyboardLayout,
    theme: ConfigTheme,
    status_lines: Vec<StatusLine>,
    leader_keymap: LeaderKeymap,
    keyboard_layouts: HashMap<String, KeyboardLayout>,
    indent_char: char,
    indent_width: usize,
    show_key_in_keymap: bool,
    icon_config: shared::icons::IconsConfig,
    load_errors: Vec<String>,
}

#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RawConfig {
    languages: HashMap<String, Language>,
    keyboard_layout: String,
    theme: ConfigThemeName,
    status_lines: Vec<StatusLine>,
    leader_keymap: LeaderKeymap,
    #[serde(default)]
    custom_keyboard_layouts: HashMap<String, KeyboardLayoutKeys>,
    indent_char: IndentChar,
    indent_width: usize,
    show_key_in_keymap: bool,
    #[serde(default)]
    icon_style: shared::icons::IconStyle,
}

#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
enum IndentChar {
    Space,
    Tab,
}

const DEFAULT_CONFIG: &str = include_str!("config_default.json");

impl Default for RawConfig {
    fn default() -> Self {
        RawConfig {
            languages: shared::languages::languages(),
            ..serde_json::from_str(DEFAULT_CONFIG)
                .expect("Default config doesn't parse, this is a bug!")
        }
    }
}

impl TryFrom<RawConfig> for AppConfig {
    type Error = anyhow::Error;
    fn try_from(value: RawConfig) -> Result<Self, Self::Error> {
        let keyboard_layouts: HashMap<_, _> = builtin_layout_map()
            .into_iter()
            .chain(
                value
                    .custom_keyboard_layouts
                    .into_iter()
                    .map(|(name, keys)| (name.clone(), KeyboardLayout::new(name, keys))),
            )
            .collect();
        let keyboard_layout = keyboard_layouts
            .get(&value.keyboard_layout)
            .context(format!(
                "Unknown keyboard layout {}, possible values are {:?}",
                value.keyboard_layout,
                keyboard_layouts.keys().collect::<Vec<_>>()
            ))?;
        Ok(Self {
            languages: value.languages,
            keyboard_layout: keyboard_layout.clone(),
            theme: ConfigTheme::try_from(value.theme)?,
            status_lines: value.status_lines,
            leader_keymap: value.leader_keymap,
            keyboard_layouts,
            indent_char: match value.indent_char {
                IndentChar::Space => ' ',
                IndentChar::Tab => '\t',
            },
            indent_width: value.indent_width,
            show_key_in_keymap: value.show_key_in_keymap,
            icon_config: shared::icons::build_icon_config(&value.icon_style),
            load_errors: Vec::new(),
        })
    }
}

/// The leader keymap is a 3x10 matrix representing three rows of 10 columns.
///
/// Assuming the keyboard layout is Qwerty, then:  
///
/// 1st row is "qwertyuiop",  
/// 2nd row is "asdfghjkl;",  
/// and 3rd row is "zxcvbnm,./".  
#[derive(Clone, Deserialize, Serialize, JsonSchema)]
pub struct LeaderKeymap([[Option<Keybinding>; 10]; 3]);
impl LeaderKeymap {
    pub fn keybindings(&self) -> &[[Option<Keybinding>; 10]; 3] {
        &self.0
    }
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(transparent)]
struct ConfigThemeName(String);

#[derive(Clone)]
struct ConfigTheme(Theme);

impl TryFrom<ConfigThemeName> for ConfigTheme {
    type Error = anyhow::Error;

    fn try_from(value: ConfigThemeName) -> Result<Self, Self::Error> {
        crate::themes::from_name(&value.0).map(Self)
    }
}

impl From<ConfigTheme> for ConfigThemeName {
    fn from(value: ConfigTheme) -> Self {
        Self(value.0.name)
    }
}

impl JsonSchema for ConfigThemeName {
    fn schema_name() -> Cow<'static, str> {
        "Theme".into()
    }

    fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
        let valid_themes: Vec<_> = crate::themes::theme_descriptor::all()
            .into_iter()
            .map(|descriptor| descriptor.name().to_string())
            .sorted()
            .collect();

        schemars::json_schema!({
            "type": "string",
            "enum": valid_themes
        })
    }
}

pub fn ki_workspace_directory() -> anyhow::Result<PathBuf> {
    Ok(std::env::current_dir()?.join(".ki"))
}

pub fn ki_global_directory() -> PathBuf {
    ::grammar::config_dir()
}

pub fn load_script(script_name: &str) -> anyhow::Result<Script> {
    // Trying reading from workspace directory first
    let workspace_path = ki_workspace_directory()?.join("scripts").join(script_name);
    let global_path = ki_global_directory().join("scripts").join(script_name);
    if workspace_path.exists() {
        Ok(Script {
            path: workspace_path.try_into().unwrap(),
            name: script_name.to_string(),
        })
    }
    // Then try reading from global directory
    else if global_path.exists() {
        Ok(Script {
            path: global_path.try_into().unwrap(),
            name: script_name.to_string(),
        })
    } else {
        Err(anyhow::anyhow!(
            "Unable to find script {script_name:?} in both {} and {}",
            workspace_path.display(),
            global_path.display()
        ))
    }
}

/// Number of `RawConfig` fields (excluding `languages`, which is handled
/// per-entry by [`extract_languages`]) attempted by [`extract_lenient`].
const TOP_LEVEL_FIELD_COUNT: usize = 9;

/// Extracts `RawConfig` from `figment` field-by-field instead of in one shot,
/// so a malformed value for one field (or one language entry) falls back to
/// its default instead of taking down the whole config. Returns the resulting
/// config plus a human-readable message for each field that couldn't be parsed.
fn extract_lenient(figment: &figment::Figment) -> (RawConfig, Vec<String>) {
    let default = RawConfig::default();
    let mut errors = Vec::new();
    let mut top_level_error_count = 0;

    macro_rules! extract_field {
        ($field:ident, $key:literal) => {
            match figment.extract_inner($key) {
                Ok(value) => value,
                Err(error) => {
                    top_level_error_count += 1;
                    errors.push(format!(
                        "Failed to parse `{}`, using default value: {error}",
                        $key
                    ));
                    default.$field
                }
            }
        };
    }

    let (languages, languages_root_error) = extract_languages(figment, &mut errors);

    let config = RawConfig {
        languages,
        keyboard_layout: extract_field!(keyboard_layout, "keyboard_layout"),
        theme: extract_field!(theme, "theme"),
        status_lines: extract_field!(status_lines, "status_lines"),
        leader_keymap: extract_field!(leader_keymap, "leader_keymap"),
        custom_keyboard_layouts: extract_field!(custom_keyboard_layouts, "custom_keyboard_layouts"),
        indent_char: extract_field!(indent_char, "indent_char"),
        indent_width: extract_field!(indent_width, "indent_width"),
        show_key_in_keymap: extract_field!(show_key_in_keymap, "show_key_in_keymap"),
        icon_style: extract_field!(icon_style, "icon_style"),
    };

    // If every single field failed, including `languages`, the config file
    // itself is almost certainly syntactically invalid (e.g. empty or
    // malformed JSON/YAML/TOML) rather than containing individually-wrong
    // fields: collapse the noise into one message instead of repeating the
    // same root cause for every field.
    if languages_root_error && top_level_error_count == TOP_LEVEL_FIELD_COUNT {
        (
            config,
            vec!["Ki config could not be parsed (invalid syntax); using defaults.".to_string()],
        )
    } else {
        (config, errors)
    }
}

/// Parses the `languages` map entry-by-entry so a malformed config for one
/// language (e.g. a bad rust-analyzer `lsp_command`) only drops that one
/// language's override instead of the entire config. Returns the resulting
/// map plus whether the `languages` key itself could not be looked up at all
/// (a signal of a whole-file syntax error rather than a single bad entry).
fn extract_languages(
    figment: &figment::Figment,
    errors: &mut Vec<String>,
) -> (HashMap<String, Language>, bool) {
    let mut languages = shared::languages::languages();
    match figment.find_value("languages") {
        Ok(value) => {
            if let Some(dict) = value.into_dict() {
                for (name, entry) in dict {
                    match entry.deserialize::<Language>() {
                        Ok(language) => {
                            languages.insert(name, language);
                        }
                        Err(error) => {
                            errors.push(format!(
                                "Failed to parse config for language `{name}`, ignoring it: {error}"
                            ));
                        }
                    }
                }
            } else {
                errors.push("`languages` config must be an object/table.".to_string());
            }
            (languages, false)
        }
        Err(error) => {
            errors.push(format!("Failed to parse `languages`: {error}"));
            (languages, true)
        }
    }
}

impl AppConfig {
    fn default() -> Self {
        RawConfig::default()
            .try_into()
            .expect("Default config can't be converted to an AppConfig, this is a bug!")
    }

    /// Loads the config leniently: a malformed or missing value for any
    /// individual field (or language entry) falls back to its default
    /// instead of failing the entire config, so that Ki always starts up.
    /// Anything that couldn't be parsed is recorded in [`AppConfig::load_errors`]
    /// so the caller can surface it inside the TUI instead of blocking startup.
    pub fn load_from_current_directory() -> Self {
        let mut errors = Vec::new();

        let workspace_dir = match ki_workspace_directory() {
            Ok(dir) => Some(dir),
            Err(error) => {
                errors.push(format!("Failed to determine workspace directory: {error}"));
                None
            }
        };

        let mut figment =
            figment::Figment::from(providers::Serialized::defaults(&RawConfig::default()));

        #[cfg(not(test))]
        {
            let global_config =
                |extension: &str| ki_global_directory().join(format!("config.{extension}"));
            figment = figment
                .merge(providers::Json::file(global_config("json")))
                .merge(providers::Yaml::file(global_config("yaml")))
                .merge(providers::Toml::file(global_config("toml")));
        }

        if let Some(workspace_dir) = &workspace_dir {
            let workspace_config =
                |extension: &str| workspace_dir.join(format!("config.{extension}"));
            figment = figment
                .merge(providers::Json::file(workspace_config("json")))
                .merge(providers::Yaml::file(workspace_config("yaml")))
                .merge(providers::Toml::file(workspace_config("toml")));
        }

        let (raw_config, mut field_errors) = extract_lenient(&figment);
        errors.append(&mut field_errors);

        let mut app_config = match AppConfig::try_from(raw_config) {
            Ok(app_config) => app_config,
            Err(error) => {
                errors.push(format!("Failed to apply config, using defaults: {error}"));
                AppConfig::default()
            }
        };
        app_config.load_errors = errors;
        app_config
    }

    pub fn singleton() -> &'static AppConfig {
        static INSTANCE: OnceCell<AppConfig> = OnceCell::new();
        INSTANCE.get_or_init(AppConfig::load_from_current_directory)
    }

    /// Human-readable descriptions of any config values that failed to parse
    /// and were replaced by their defaults during [`AppConfig::load_from_current_directory`].
    pub fn load_errors(&self) -> &[String] {
        &self.load_errors
    }

    pub fn languages(&self) -> &HashMap<std::string::String, language::Language> {
        &self.languages
    }

    pub fn keyboard_layout(&self) -> &KeyboardLayout {
        &self.keyboard_layout
    }

    pub fn keyboard_layouts(&self) -> &HashMap<String, KeyboardLayout> {
        &self.keyboard_layouts
    }

    pub fn theme(&self) -> &Theme {
        &self.theme.0
    }

    pub fn status_lines(&self) -> Vec<crate::app::StatusLine> {
        self.status_lines.clone()
    }

    pub fn leader_keymap(&self) -> &LeaderKeymap {
        &self.leader_keymap
    }

    pub fn indent_width(&self) -> usize {
        self.indent_width
    }

    pub fn indent_char(&self) -> char {
        self.indent_char
    }

    pub fn show_key_in_keymap(&self) -> bool {
        self.show_key_in_keymap
    }

    pub fn icon_config(&self) -> &shared::icons::IconsConfig {
        &self.icon_config
    }
}

pub fn from_path(path: &AbsolutePath) -> Option<Language> {
    path.extension()
        .and_then(from_extension)
        .or_else(|| from_filename(path))
}

pub fn from_extension(extension: &str) -> Option<Language> {
    AppConfig::singleton()
        .languages()
        .iter()
        .find(|(_, language)| language.extensions().contains(&extension.to_string()))
        .map(|(_, language)| (*language).clone())
}

pub fn from_filename(path: &AbsolutePath) -> Option<Language> {
    let file_name = path.file_name()?;
    AppConfig::singleton()
        .languages()
        .iter()
        .find(|(_, language)| language.file_names().contains(&file_name))
        .map(|(_, language)| (*language).clone())
}

/// Detect the language from the first line of the file content.
///
/// Standard shebang format is checked as well as vim's `ft=` method and various
/// other editors supporting `mode:`.
///
/// For example, a file opened that has any of the following first lines will be
/// detected as bash.
///
/// - `#!/bin/bash`
/// - `# vim: ft=bash`
/// - `# mode: bash
///
/// Spaces and other content on the line do not matter.
pub fn from_content_directive(content: &str) -> Option<Language> {
    let first_line = content.lines().next()?;

    let re = Regex::new(r"(?:(?:^#!.*/)|(?:mode:)|(?:ft\s*=))\s*(\w+)").unwrap();
    let language_id = re
        .captures(first_line)
        .and_then(|captures| captures.get(1).map(|mode| mode.as_str().to_string()));

    language_id.and_then(|id| {
        AppConfig::singleton()
            .languages()
            .iter()
            .find(|(_, language)| {
                language
                    .lsp_language_id()
                    .clone()
                    .is_some_and(|lsp_id| lsp_id.to_string() == id)
            })
            .map(|(_, language)| (*language).clone())
    })
}

#[cfg(test)]
mod test_language {
    use super::*;
    use std::fs::File;
    #[test]
    fn test_from_path() -> anyhow::Result<()> {
        fn run_test_case(filename: &str, expected_language_id: &'static str) -> anyhow::Result<()> {
            let tempdir = tempfile::tempdir()?;
            let path = tempdir.path().join(filename);
            File::create(path.clone())?;
            let result = from_path(&path.to_string_lossy().to_string().try_into()?).unwrap();
            assert_eq!(
                result.tree_sitter_grammar_id().unwrap(),
                expected_language_id
            );
            Ok(())
        }
        run_test_case("hello.rs", "rust")?;
        run_test_case("justfile", "just")?;
        Ok(())
    }

    #[test]
    fn test_from_content_directive() -> anyhow::Result<()> {
        fn run_test_case(content: &str, expected_language_id: &'static str) -> anyhow::Result<()> {
            let result = from_content_directive(content).unwrap();
            assert_eq!(
                result.tree_sitter_grammar_id().unwrap(),
                expected_language_id
            );
            Ok(())
        }

        run_test_case("#!/bin/bash", "bash")?;
        run_test_case("#!/usr/local/bin/bash", "bash")?;
        run_test_case("// mode: python", "python")?;
        run_test_case("-- tab_spaces: 5, mode: bash, use_tabs: false", "bash")?;
        run_test_case("-- tab_spaces: 5, mode:bash, use_tabs: false", "bash")?;
        run_test_case("-- vim: ft = bash", "bash")?;

        Ok(())
    }
}

mod test_config {
    #[test]
    /// This test case is necessary to prevent invalid `config_default.json`
    /// from being committed to the master branch.
    fn default_app_config_should_be_construtable() {
        super::AppConfig::default();
    }

    #[test]
    fn doc_assets_default_config_json() {
        let path = "docs/static/config_default.json";
        std::fs::write(path, super::DEFAULT_CONFIG).unwrap();
    }

    /// Regression test: an empty (e.g. freshly auto-created) workspace config
    /// file must never fail startup or block on stdin; it should be treated
    /// as "no config" and reported as a single collapsed error.
    #[test]
    fn empty_config_file_falls_back_to_defaults() {
        let tempdir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tempdir.path().join(".ki")).unwrap();
        std::fs::write(tempdir.path().join(".ki/config.json"), "").unwrap();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(tempdir.path()).unwrap();
        let config = super::AppConfig::load_from_current_directory();
        std::env::set_current_dir(original_dir).unwrap();

        assert_eq!(config.load_errors().len(), 1);
        assert_eq!(
            config.indent_width(),
            super::AppConfig::default().indent_width()
        );
    }

    /// A malformed field and a malformed language entry should each fall back
    /// to their own default independently, instead of the whole config (or
    /// even the whole `languages` map) failing.
    #[test]
    fn malformed_field_and_language_fall_back_individually() {
        let tempdir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tempdir.path().join(".ki")).unwrap();
        std::fs::write(
            tempdir.path().join(".ki/config.json"),
            r#"{"indent_width": "two", "languages": {"rust": {"lsp_command": "not-an-object"}}}"#,
        )
        .unwrap();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(tempdir.path()).unwrap();
        let config = super::AppConfig::load_from_current_directory();
        std::env::set_current_dir(original_dir).unwrap();

        assert_eq!(config.load_errors().len(), 2);
        assert_eq!(
            config.indent_width(),
            super::AppConfig::default().indent_width()
        );
        assert!(!config.languages().is_empty());
    }
}
