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
    pub jump_mark_odd: Style,
    pub jump_mark_even: Style,
    pub text: Style,
    pub primary_selection: Style,
    pub primary_selection_secondary_cursor: Style,
    pub secondary_selection: Style,
    pub secondary_selection_primary_cursor: Style,
    pub secondary_selection_secondary_cursor: Style,
    pub line_number: Style,
    pub line_number_separator: Style,
    pub bookmark: Style,
}

#[derive(Default, Clone)]
pub struct SyntaxStyles {
    pub attribute: Option<Style>,
    pub constant: Option<Style>,
    pub function_builtin: Option<Style>,
    pub function: Option<Style>,
    pub keyword: Option<Style>,
    pub operator: Option<Style>,
    pub property: Option<Style>,
    pub punctuation: Option<Style>,
    pub punctuation_bracket: Option<Style>,
    pub punctuation_delimiter: Option<Style>,
    pub string: Option<Style>,
    pub string_special: Option<Style>,
    pub tag: Option<Style>,
    pub type_: Option<Style>,
    pub type_builtin: Option<Style>,
    pub variable: Option<Style>,
    pub variable_builtin: Option<Style>,
    pub variable_parameter: Option<Style>,
    pub comment: Option<Style>,
}

pub const HIGHLIGHT_NAMES: [&str; 19] = [
    "attribute",
    "constant",
    "function.builtin",
    "function",
    "keyword",
    "operator",
    "property",
    "punctuation",
    "punctuation.bracket",
    "punctuation.delimiter",
    "string",
    "string.special",
    "tag",
    "type",
    "type.builtin",
    "variable",
    "variable.builtin",
    "variable.parameter",
    "comment",
];

impl SyntaxStyles {
    /// The `index` should tally with the `HIGHLIGHT_NAMES` array.
    pub fn get_color(&self, index: usize) -> Option<Style> {
        match index {
            0 => self.attribute,
            1 => self.constant,
            2 => self.function_builtin,
            3 => self.function,
            4 => self.keyword,
            5 => self.operator,
            6 => self.property,
            7 => self.punctuation,
            8 => self.punctuation_bracket,
            9 => self.punctuation_delimiter,
            10 => self.string,
            11 => self.string_special,
            12 => self.tag,
            13 => self.type_,
            14 => self.type_builtin,
            15 => self.variable,
            16 => self.variable_builtin,
            17 => self.variable_parameter,
            18 => self.comment,
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
