use std::borrow::Cow;
use std::{collections::HashMap, io::Read, path::PathBuf};

use itertools::Itertools;
use regex::Regex;

use crate::app::StatusLine;
use crate::components::editor_keymap::KeyboardLayoutKind;
use crate::scripting::{Keybinding, Script};
use crate::themes::Theme;
use figment::providers;
use figment::providers::Format;
use once_cell::sync::OnceCell;
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Serialize};
use shared::canonicalized_path::CanonicalizedPath;
use shared::language::{self, Language};

#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AppConfig {
    languages: HashMap<String, Language>,
    keyboard_layout: KeyboardLayoutKind,
    theme: ConfigTheme,
    status_lines: Vec<StatusLine>,
    leader_keymap: LeaderKeymap,
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

#[derive(Clone, Deserialize, Serialize, JsonSchema)]
#[serde(try_from = "ConfigThemeName", into = "ConfigThemeName")]
struct ConfigTheme(Theme);

impl TryFrom<ConfigThemeName> for ConfigTheme {
    type Error = String;

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

const DEFAULT_CONFIG: &str = include_str!("config_default.json");

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
    if let Ok(path) = CanonicalizedPath::try_from(workspace_path.clone()) {
        Ok(Script {
            path,
            name: script_name.to_string(),
        })
    }
    // Then try reading from global directory
    else if let Ok(path) = CanonicalizedPath::try_from(global_path.clone()) {
        Ok(Script {
            path,
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
        let deserializer = &mut serde_json::Deserializer::from_str(DEFAULT_CONFIG);
        Self {
            languages: shared::languages::languages(),
            ..serde_path_to_error::deserialize(deserializer)
                .map_err(|err| anyhow::anyhow!("{err}\n\nINPUT=\n\n{DEFAULT_CONFIG}"))
                .unwrap()
        }
    }

    pub fn load_from_current_directory() -> anyhow::Result<Self> {
        let workspace_dir = ki_workspace_directory()?;
        let workspace_config = |extension: &str| workspace_dir.join(format!("config.{extension}"));
        let global_config =
            |extension: &str| ki_global_directory().join(format!("config.{extension}"));
        let config: AppConfig =
            figment::Figment::from(providers::Serialized::defaults(&AppConfig::default()))
                .merge(providers::Json::file(global_config("json")))
                .merge(providers::Yaml::file(global_config("yaml")))
                .merge(providers::Toml::file(global_config("toml")))
                .merge(providers::Json::file(workspace_config("json")))
                .merge(providers::Yaml::file(workspace_config("yaml")))
                .merge(providers::Toml::file(workspace_config("toml")))
                .extract()?;
        Ok(config)
    }

    pub fn singleton() -> &'static AppConfig {
        static INSTANCE: OnceCell<AppConfig> = OnceCell::new();
        INSTANCE.get_or_init(|| match AppConfig::load_from_current_directory() {
            Ok(config) => config,
            Err(error) => {
                eprintln!("Error parsing Ki config: {error}");
                println!("\nConfig will be ignored. [Press any key to continue]");
                let _ = std::io::stdin().read(&mut [0u8]).unwrap();

                AppConfig::default()
            }
        })
    }

    pub fn languages(&self) -> &HashMap<std::string::String, language::Language> {
        &self.languages
    }

    pub fn keyboard_layout_kind(&self) -> KeyboardLayoutKind {
        self.keyboard_layout
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
}

pub fn from_path(path: &CanonicalizedPath) -> Option<Language> {
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

pub fn from_filename(path: &CanonicalizedPath) -> Option<Language> {
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
        std::fs::write(path, super::DEFAULT_CONFIG).unwrap()
    }
}
