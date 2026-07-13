use std::borrow::Cow;
use std::io::Read;
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

mod lenient;
use lenient::{extract_lenient, Extracted};

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

/// A JSON [`providers::Format`] backed by the [`json5`] parser, so
/// user-facing `config.json` files can contain `//` and `/* */` comments
/// (and other JSON5 conveniences like trailing commas).
struct Jsonc;

impl providers::Format for Jsonc {
    type Error = json5::Error;

    const NAME: &'static str = "JSON";

    fn from_str<T: serde::de::DeserializeOwned>(string: &str) -> Result<T, Self::Error> {
        json5::from_str(string)
    }
}

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

impl AppConfig {
    fn default() -> Self {
        RawConfig::default()
            .try_into()
            .expect("Default config can't be converted to an AppConfig, this is a bug!")
    }

    /// Loads the config leniently: a malformed or missing value for any
    /// individual field (or language entry) falls back to its default
    /// instead of failing the entire config, so that Ki always starts up.
    /// Anything that couldn't be parsed is recorded in [`AppConfig::load_errors`];
    /// [`AppConfig::singleton`] is responsible for surfacing it to the user.
    pub fn load_from_current_directory() -> Self {
        let (workspace_dir, workspace_dir_error) = match ki_workspace_directory() {
            Ok(dir) => (Some(dir), None),
            Err(error) => (
                None,
                Some(format!("Failed to determine workspace directory: {error}")),
            ),
        };

        let figment =
            figment::Figment::from(providers::Serialized::defaults(&RawConfig::default()));

        #[cfg(not(test))]
        let figment = {
            let global_config =
                |extension: &str| ki_global_directory().join(format!("config.{extension}"));
            figment
                .merge(Jsonc::file(global_config("json")))
                .merge(providers::Yaml::file(global_config("yaml")))
                .merge(providers::Toml::file(global_config("toml")))
        };

        let figment = if let Some(workspace_dir) = &workspace_dir {
            let workspace_config =
                |extension: &str| workspace_dir.join(format!("config.{extension}"));
            figment
                .merge(Jsonc::file(workspace_config("json")))
                .merge(providers::Yaml::file(workspace_config("yaml")))
                .merge(providers::Toml::file(workspace_config("toml")))
        } else {
            figment
        };

        let Extracted {
            value: raw_config,
            errors: field_errors,
        } = extract_lenient(&figment);

        let (app_config, apply_error) = match AppConfig::try_from(raw_config) {
            Ok(app_config) => (app_config, None),
            Err(error) => (
                AppConfig::default(),
                Some(format!("Failed to apply config, using defaults: {error}")),
            ),
        };

        let load_errors = workspace_dir_error
            .into_iter()
            .chain(field_errors)
            .chain(apply_error)
            .collect();

        AppConfig {
            load_errors,
            ..app_config
        }
    }

    pub fn singleton() -> &'static AppConfig {
        static INSTANCE: OnceCell<AppConfig> = OnceCell::new();
        INSTANCE.get_or_init(|| {
            let app_config = AppConfig::load_from_current_directory();
            if !app_config.load_errors.is_empty() {
                eprintln!(
                    "Ki config warning:\n{}",
                    app_config.load_errors.join("\n\n")
                );
                println!("\n[Press any key to continue]");
                // Best-effort: if stdin isn't interactive (e.g. under a test
                // harness or piped input), don't block startup on it.
                let _ = std::io::stdin().read(&mut [0u8]);
            }
            app_config
        })
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
        assert!(config.load_errors()[0].contains(".ki/config.json"));
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
        assert!(config
            .load_errors()
            .iter()
            .all(|error| error.contains(".ki/config.json")));
    }

    /// `config.json` is allowed to contain `//` and `/* */` comments (JSONC),
    /// since that's convenient for annotating settings.
    #[test]
    fn config_json_allows_comments() {
        let tempdir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tempdir.path().join(".ki")).unwrap();
        std::fs::write(
            tempdir.path().join(".ki/config.json"),
            r#"{
                // line comment
                "indent_width": 7 /* inline comment */
            }"#,
        )
        .unwrap();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(tempdir.path()).unwrap();
        let config = super::AppConfig::load_from_current_directory();
        std::env::set_current_dir(original_dir).unwrap();

        assert!(config.load_errors().is_empty());
        assert_eq!(config.indent_width(), 7);
    }

    /// A malformed field *within* a language override (e.g. `formatter`)
    /// should only reset that field, not drop the whole language override.
    #[test]
    fn malformed_language_field_falls_back_individually() {
        let tempdir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tempdir.path().join(".ki")).unwrap();
        std::fs::write(
            tempdir.path().join(".ki/config.json"),
            r#"{"languages": {"javascript": {
                "formatter": {"command": "prettier", "arguments": "format --stdin-file-path=_.js"},
                "line_comment_prefix": "//"
            }}}"#,
        )
        .unwrap();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(tempdir.path()).unwrap();
        let config = super::AppConfig::load_from_current_directory();
        std::env::set_current_dir(original_dir).unwrap();

        assert_eq!(config.load_errors().len(), 1);
        assert!(config.load_errors()[0].contains("languages.javascript.formatter.arguments"));
        assert!(config.load_errors()[0].contains(".ki/config.json"));

        let javascript = config.languages().get("javascript").unwrap();
        // `line_comment_prefix` was valid and should still be applied...
        assert_eq!(javascript.line_comment_prefix(), Some("//".to_string()));
        // ...while `formatter` should fall back to the builtin default
        // (which is `Some`), instead of the whole language override being
        // discarded.
        assert!(javascript.formatter().is_some());
    }

    /// One malformed status line (e.g. a typo'd component name) should only
    /// drop that line, not reset `status_lines` to the default set.
    #[test]
    fn malformed_status_line_falls_back_individually() {
        let tempdir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tempdir.path().join(".ki")).unwrap();
        std::fs::write(
            tempdir.path().join(".ki/config.json"),
            r#"{"status_lines": [
                {"components": ["Mode"]},
                {"components": ["NotAComponent"]}
            ]}"#,
        )
        .unwrap();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(tempdir.path()).unwrap();
        let config = super::AppConfig::load_from_current_directory();
        std::env::set_current_dir(original_dir).unwrap();

        assert_eq!(config.load_errors().len(), 1);
        assert!(config.load_errors()[0].contains("status_lines.1"));
        assert!(config.load_errors()[0].contains(".ki/config.json"));
        assert_eq!(config.status_lines().len(), 1);
    }

    /// One malformed keybinding in `leader_keymap` should only clear that
    /// cell, not wipe out every other configured keybinding.
    #[test]
    fn malformed_leader_keymap_cell_falls_back_individually() {
        let tempdir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tempdir.path().join(".ki")).unwrap();
        let mut grid = vec![vec![serde_json::Value::Null; 10]; 3];
        grid[0][0] = serde_json::json!({"name": "not-an-object"});
        std::fs::write(
            tempdir.path().join(".ki/config.json"),
            serde_json::json!({ "leader_keymap": grid }).to_string(),
        )
        .unwrap();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(tempdir.path()).unwrap();
        let config = super::AppConfig::load_from_current_directory();
        std::env::set_current_dir(original_dir).unwrap();

        assert_eq!(config.load_errors().len(), 1);
        assert!(config.load_errors()[0].contains("leader_keymap.0.0"));
        assert!(config.load_errors()[0].contains(".ki/config.json"));
        assert!(config.leader_keymap().keybindings()[0][0].is_none());
    }

    /// One malformed custom keyboard layout should only drop that entry,
    /// not every custom layout the user defined.
    #[test]
    fn malformed_custom_keyboard_layout_falls_back_individually() {
        let tempdir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tempdir.path().join(".ki")).unwrap();
        std::fs::write(
            tempdir.path().join(".ki/config.json"),
            r#"{"custom_keyboard_layouts": {"bad": "not-a-grid"}}"#,
        )
        .unwrap();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(tempdir.path()).unwrap();
        let config = super::AppConfig::load_from_current_directory();
        std::env::set_current_dir(original_dir).unwrap();

        assert_eq!(config.load_errors().len(), 1);
        assert!(config.load_errors()[0].contains("custom_keyboard_layouts.bad"));
        assert!(config.load_errors()[0].contains(".ki/config.json"));
    }
}
