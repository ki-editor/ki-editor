use super::{from_zed_theme, vscode_dark, vscode_light, Theme};

pub type ThemeFn = fn() -> Theme;

#[derive(Clone, Debug)]
pub enum ThemeDescriptor {
    ThemeFn(String, ThemeFn),
    ZedThemeURLMap(&'static str, &'static str),
}

impl ThemeDescriptor {
    pub(crate) fn name(&self) -> &str {
        match self {
            ThemeDescriptor::ThemeFn(name, _) => name,
            ThemeDescriptor::ZedThemeURLMap(name, _) => name,
        }
    }
}

impl From<ThemeDescriptor> for Theme {
    fn from(theme_descriptor: ThemeDescriptor) -> Self {
        match theme_descriptor {
            ThemeDescriptor::ThemeFn(_, theme_fn) => theme_fn(),
            ThemeDescriptor::ZedThemeURLMap(name, url) => {
                match from_zed_theme::from_url(name, url) {
                    Ok(theme) => theme,
                    Err(_) => vscode_light(),
                }
            }
        }
    }
}

pub(crate) fn all() -> Vec<ThemeDescriptor> {
    let vscode_themes = vec![
        ThemeDescriptor::ThemeFn("VSCode (Light)".to_string(), vscode_light),
        ThemeDescriptor::ThemeFn("VSCode (Dark)".to_string(), vscode_dark),
    ];
    let zed_themes = from_zed_theme::theme_descriptors();

    let mut theme_descriptors: Vec<ThemeDescriptor> =
        vscode_themes.into_iter().chain(zed_themes).collect();

    theme_descriptors.sort_by(|a, b| a.name().cmp(b.name()));
    theme_descriptors
}
