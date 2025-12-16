use std::{collections::HashMap, io::Read, path::PathBuf, str::FromStr};

use regex::Regex;

use crate::components::editor_keymap::KeyboardLayoutKind;
use figment::providers;
use figment::providers::Format;
use once_cell::sync::OnceCell;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use shared::canonicalized_path::CanonicalizedPath;
use shared::language::{self, Language};

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct AppConfig {
    languages: HashMap<String, Language>,
    keyboard_layout: KeyboardLayoutKind,
    theme: Option<String>,
}

impl AppConfig {
    fn default() -> Self {
        Self {
            languages: shared::languages::languages(),
            keyboard_layout: KeyboardLayoutKind::Qwerty,
            theme: None,
        }
    }

    pub(crate) fn load_from_current_directory() -> anyhow::Result<Self> {
        let config: AppConfig =
            figment::Figment::from(providers::Serialized::defaults(&AppConfig::default()))
                .merge(providers::Json::file(
                    PathBuf::from_str(".")?.join(".ki").join("config.json"),
                ))
                .merge(providers::Json::file(
                    ::grammar::config_dir().join("config.json"),
                ))
                .extract()?;
        Ok(config)
    }

    pub(crate) fn singleton() -> &'static AppConfig {
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

    pub(crate) fn languages(&self) -> &HashMap<std::string::String, language::Language> {
        &self.languages
    }

    pub(crate) fn keyboard_layout_kind(&self) -> KeyboardLayoutKind {
        self.keyboard_layout
    }

    pub(crate) fn theme(&self) -> crate::themes::Theme {
        let theme = self.theme.clone().unwrap_or_default();
        crate::themes::theme_descriptor::all()
            .iter()
            .find(|descriptor| descriptor.name() == theme)
            .map(|descriptor| descriptor.to_theme())
            .unwrap_or_else(|| crate::themes::Theme::default())
    }
}

pub(crate) fn from_path(path: &CanonicalizedPath) -> Option<Language> {
    path.extension()
        .and_then(from_extension)
        .or_else(|| from_filename(path))
}

pub(crate) fn from_extension(extension: &str) -> Option<Language> {
    AppConfig::singleton()
        .languages()
        .iter()
        .find(|(_, language)| language.extensions().contains(&extension.to_string()))
        .map(|(_, language)| (*language).clone())
}

pub(crate) fn from_filename(path: &CanonicalizedPath) -> Option<Language> {
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
pub(crate) fn from_content_directive(content: &str) -> Option<Language> {
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
