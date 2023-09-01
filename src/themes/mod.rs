pub mod vscode_light;
pub use vscode_light::VSCODE_LIGHT;

use crate::grid::Style;

#[derive(Clone)]
pub struct Theme {
    pub name: &'static str,
    pub syntax: SyntaxStyles,
    pub ui: UiStyles,
    pub diagnostic: DiagnosticStyles,
}

#[derive(Clone)]
pub enum StyleKey {
    UiPrimarySelection,
}

impl StyleKey {
    pub fn get_style(&self, theme: &Theme) -> Style {
        match self {
            StyleKey::UiPrimarySelection => theme.ui.primary_selection,
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        VSCODE_LIGHT
    }
}

#[derive(Clone)]
pub struct DiagnosticStyles {
    pub error: Style,
    pub warning: Style,
    pub info: Style,
    pub hint: Style,
    pub default: Style,
}

#[derive(Default, Clone)]
pub struct UiStyles {
    pub global_title: Style,
    pub window_title: Style,
    pub jump_mark_odd: Style,
    pub jump_mark_even: Style,
    pub text: Style,
    pub primary_selection: Style,
    pub primary_selection_anchor: Style,
    pub primary_selection_secondary_cursor: Style,
    pub secondary_selection: Style,
    pub secondary_selection_anchor: Style,
    pub secondary_selection_primary_cursor: Style,
    pub secondary_selection_secondary_cursor: Style,
    pub line_number: Style,
    pub line_number_separator: Style,
    pub bookmark: Style,
}

#[derive(Default, Clone)]
pub struct SyntaxStyles {
    pub function: Option<Style>,
    pub keyword: Option<Style>,
    pub string: Option<Style>,
    pub type_: Option<Style>,
    pub comment: Option<Style>,
    pub default: Style,
}

pub const HIGHLIGHT_NAMES: &[&str] = &["comment", "keyword", "string", "type", "function"];

impl SyntaxStyles {
    /// The `index` should tally with the `HIGHLIGHT_NAMES` array.
    pub fn get_color(&self, index: usize) -> Option<Style> {
        match index {
            0 => self.comment,
            1 => self.keyword,
            2 => self.string,
            3 => self.type_,
            4 => self.function,
            _ => None,
        }
    }
}

/// This should be constructed using the `color!` macro.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

impl From<Color> for crossterm::style::Color {
    fn from(val: Color) -> Self {
        crossterm::style::Color::Rgb {
            r: val.r,
            g: val.g,
            b: val.b,
        }
    }
}

pub mod icons {
    use once_cell::sync::OnceCell;
    use std::collections::HashMap;

    static ICON_CONFIG: OnceCell<IconsConfig> = OnceCell::new();

    use serde::Deserialize;
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct IconsConfig {
        pub file: String,
        pub folder: String,
        pub folder_expanded: String,
        pub file_extensions: HashMap<String, String>,
        pub file_names: HashMap<String, String>,
    }
    pub fn get_icon_config() -> &'static IconsConfig {
        ICON_CONFIG.get_or_init(|| {
            serde_json::from_str(include_str!("../../contrib/emoji-icon-theme.json")).unwrap()
        })
    }
}
