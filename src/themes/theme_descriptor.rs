use super::{from_zed_theme, vscode_dark, vscode_light, Theme};
use itertools::Itertools;

pub(crate) type ThemeFn = fn() -> Theme;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ThemeDescriptor {
    pub(super) name: String,
}

impl ThemeDescriptor {
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn to_theme(&self) -> Theme {
        match self.name.as_str() {
            "VS Code (Light)" => vscode_light(),
            "VS Code (Dark)" => vscode_dark(),
            // This is ok, because this module will never construct a theme descriptor with an invalid name.
            name => from_zed_theme::from_name(name),
        }
    }
}

impl Default for ThemeDescriptor {
    fn default() -> Self {
        Self {
            name: "VS Code (Light)".to_string(),
        }
    }
}

pub(crate) fn all() -> Vec<ThemeDescriptor> {
    let theme_descriptors: Vec<ThemeDescriptor> = [
        ThemeDescriptor {
            name: "VS Code (Light)".to_string(),
        },
        ThemeDescriptor {
            name: "VS Code (Dark)".to_string(),
        },
    ]
    .into_iter()
    .chain(from_zed_theme::theme_descriptors())
    .sorted_by_key(|theme| theme.name().to_owned())
    .collect();

    theme_descriptors
}
