pub mod vscode_light;
pub use vscode_light::VSCODE_LIGHT;

use crate::grid::Style;

pub struct Theme {
    pub name: &'static str,
    pub styles: ThemeStyles,
}

#[derive(Default)]
pub struct ThemeStyles {
    attribute: Option<Style>,
    constant: Option<Style>,
    function_builtin: Option<Style>,
    function: Option<Style>,
    keyword: Option<Style>,
    operator: Option<Style>,
    property: Option<Style>,
    punctuation: Option<Style>,
    punctuation_bracket: Option<Style>,
    punctuation_delimiter: Option<Style>,
    string: Option<Style>,
    string_special: Option<Style>,
    tag: Option<Style>,
    type_: Option<Style>,
    type_builtin: Option<Style>,
    variable: Option<Style>,
    variable_builtin: Option<Style>,
    variable_parameter: Option<Style>,
}

impl ThemeStyles {
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
            _ => None,
        }
    }
}
