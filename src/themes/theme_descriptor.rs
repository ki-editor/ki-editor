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
                from_zed_theme::from_url(name, url).unwrap_or_else(|_| vscode_light())
            }
        }
    }
}

pub(crate) fn all() -> Vec<ThemeDescriptor> {
    let mut theme_descriptors: Vec<ThemeDescriptor> = [
        ThemeDescriptor::ThemeFn("VS Code (Light)".to_string(), vscode_light),
        ThemeDescriptor::ThemeFn("VS Code (Dark)".to_string(), vscode_dark),
    ]
    .into_iter()
    .chain(from_zed_theme::theme_descriptors())
    .collect();

    theme_descriptors.sort_by(|a, b| a.name().cmp(b.name()));
    theme_descriptors
}
