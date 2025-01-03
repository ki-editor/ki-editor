use super::{from_zed_theme, vscode_dark, vscode_light, Theme};
use zed_theme::ThemeContent;

pub type ThemeFn = fn() -> Theme;

#[derive(Clone, Debug)]
pub enum ThemeDescriptor {
    ThemeFn(String, ThemeFn),
    ZedTheme(String, Box<ThemeContent>),
}

impl ThemeDescriptor {
    pub(crate) fn name(&self) -> &str {
        match self {
            ThemeDescriptor::ThemeFn(name, _) => name,
            ThemeDescriptor::ZedTheme(name, _) => name,
        }
    }
}

impl From<ThemeDescriptor> for Theme {
    fn from(theme_descriptor: ThemeDescriptor) -> Self {
        match theme_descriptor {
            ThemeDescriptor::ThemeFn(_, theme_fn) => theme_fn(),
            ThemeDescriptor::ZedTheme(_, theme_content) => theme_content.into(),
        }
    }
}

pub(crate) fn all() -> Vec<ThemeDescriptor> {
    let vscode_themes = vec![
        ThemeDescriptor::ThemeFn("VSCode (Light)".to_string(), vscode_light),
        ThemeDescriptor::ThemeFn("VSCode (Dark)".to_string(), vscode_dark),
    ];
    let zed_themes = from_zed_theme::theme_descriptors();

    let mut theme_descriptors: Vec<ThemeDescriptor> = vscode_themes
        .into_iter()
        .chain(zed_themes.into_iter())
        .collect();

    theme_descriptors.sort_by(|a, b| a.name().cmp(b.name()));
    theme_descriptors
}
