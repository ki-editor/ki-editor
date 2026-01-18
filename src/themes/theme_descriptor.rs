use std::{collections::HashMap, sync::LazyLock};

use super::{from_zed_theme, vscode_dark, vscode_light, Theme};
use zed_theme::{get_config_themes, get_zed_themes};

#[derive(Clone, Debug, PartialEq)]
pub struct ThemeDescriptor(String);

impl ThemeDescriptor {
    pub fn name(&self) -> &str {
        &self.0
    }

    pub fn to_theme(&self) -> Theme {
        let theme_map = &*THEMES;
        theme_map
            .get(&self.0)
            .expect("Theme descriptor had no matching theme?")
            .clone()
    }
}

impl Default for ThemeDescriptor {
    fn default() -> Self {
        Self("VS Code (Light)".to_string())
    }
}

static THEMES: LazyLock<HashMap<String, Theme>> = LazyLock::new(|| {
    let mut themes = HashMap::new();

    // Non-zed builtin themes
    themes.insert("VS Code (Light)".to_string(), vscode_light());
    themes.insert("VS Code (Dark)".to_string(), vscode_dark());

    // Zed builtin themes
    for (name, theme) in get_zed_themes() {
        themes.insert(name, from_zed_theme::from_theme_content(theme));
    }

    // Zed ~/.config/themes/ directory loaded themes
    let global_glob_os_string = crate::config::ki_global_directory()
        .join("themes/*.json")
        .into_os_string();
    let global_glob = global_glob_os_string
        .to_str()
        .expect("Not able to convert global glob os string to str");

    get_config_themes(global_glob)
        .into_iter()
        .for_each(|(name, theme)| {
            themes.insert(name, from_zed_theme::from_theme_content(theme));
        });

    // Zed .ki/themes/ directory loaded themes
    let workspace_glob_os_string = crate::config::ki_workspace_directory()
        .map(|path| path.join("themes/*.json").into_os_string());

    if let Ok(workspace_glob_os_string) = workspace_glob_os_string {
        let workspace_glob = workspace_glob_os_string
            .to_str()
            .expect("Not able to convert workspace glob os string to str");
        get_config_themes(workspace_glob)
            .into_iter()
            .for_each(|(name, theme)| {
                themes.insert(name, from_zed_theme::from_theme_content(theme));
            });
    };

    themes
});

pub fn all() -> Vec<ThemeDescriptor> {
    THEMES.keys().cloned().map(ThemeDescriptor).collect()
}

#[cfg(test)]
mod test {
    #[test]
    fn test_all_themes_work() {
        for theme_descriptor in super::all() {
            theme_descriptor.to_theme();
        }
    }
}
