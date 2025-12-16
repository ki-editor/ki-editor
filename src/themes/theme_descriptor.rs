use super::{from_zed_theme, vscode_dark, vscode_light, Theme};
use itertools::Itertools;

pub(crate) type ThemeFn = fn() -> Theme;

pub(crate) enum BuiltInTheme {
    VscodeDark,
    VscodeLight,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ThemeDescriptor {
    ThemeFn(String, ThemeFn),
    ZedTheme(&'static str, &'static str),
}

impl ThemeDescriptor {
    pub(crate) fn name(&self) -> &str {
        match self {
            ThemeDescriptor::ThemeFn(name, _) => name,
            ThemeDescriptor::ZedTheme(name, _) => name,
        }
    }

    pub(crate) fn to_theme(&self) -> Theme {
        match self {
            ThemeDescriptor::ThemeFn(_, theme_fn) => theme_fn(),
            ThemeDescriptor::ZedTheme(name, url) => {
                from_zed_theme::from_url(name, url).unwrap_or_else(|_| vscode_light())
            }
        }
    }
}

impl Default for ThemeDescriptor {
    fn default() -> Self {
        ThemeDescriptor::ThemeFn("VS Code (Light)".to_string(), vscode_light)
    }
}

pub(crate) fn all() -> Vec<ThemeDescriptor> {
    let theme_descriptors: Vec<ThemeDescriptor> = [
        ThemeDescriptor::ThemeFn("VS Code (Light)".to_string(), vscode_light),
        ThemeDescriptor::ThemeFn("VS Code (Dark)".to_string(), vscode_dark),
    ]
    .into_iter()
    .chain(from_zed_theme::theme_descriptors())
    .sorted_by_key(|theme| theme.name().to_owned())
    .collect();

    theme_descriptors
}
