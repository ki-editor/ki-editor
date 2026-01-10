use std::{collections::HashMap, sync::LazyLock};

use super::{from_zed_theme, vscode_dark, vscode_light, Theme};
use zed_theme::get_zed_themes;

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
