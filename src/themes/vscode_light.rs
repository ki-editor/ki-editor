use super::{DiagnosticStyles, Theme, UiStyles};
use crate::{grid::Style, themes::SyntaxStyles};
use lazy_static::lazy_static;
use my_proc_macros::hex;

pub fn theme() -> Theme {
    Theme {
        name: "vscode-light",
        syntax: SyntaxStyles {
            keyword: Some(Style::new().foreground_color(hex!("#0000ff"))),
            variable: Some(Style::new().foreground_color(hex!("#000000"))),
            function: Some(Style::new().foreground_color(hex!("#795e26"))),
            type_: Some(Style::new().foreground_color(hex!("#267f99"))),
            attribute: Some(Style::new().foreground_color(hex!("#0000ff"))),
            constant: Some(Style::new().foreground_color(hex!("#098658"))),
            function_builtin: Some(Style::new().foreground_color(hex!("#795e26"))),
            operator: Some(Style::new().foreground_color(hex!("#000000"))),
            property: Some(Style::new().foreground_color(hex!("#000000"))),
            punctuation: Some(Style::new().foreground_color(hex!("#000000"))),
            punctuation_bracket: Some(Style::new().foreground_color(hex!("#000000"))),
            punctuation_delimiter: Some(Style::new().foreground_color(hex!("#000000"))),
            string: Some(Style::new().foreground_color(hex!("#a31515"))),
            string_special: Some(Style::new().foreground_color(hex!("#a31515"))),
            tag: Some(Style::new().foreground_color(hex!("#800000"))),
            type_builtin: Some(Style::new().foreground_color(hex!("#267f99"))),
            variable_builtin: Some(Style::new().foreground_color(hex!("#001080"))),
            variable_parameter: Some(Style::new().foreground_color(hex!("#001080"))),
            comment: Some(Style::new().foreground_color(hex!("#6a9955"))),
        },
        ui: UiStyles {
            jump_mark_odd: Style::new()
                .background_color(hex!("#B5485D"))
                .foreground_color(hex!("#ffffff")),
            jump_mark_even: Style::new()
                .background_color(hex!("#84B701"))
                .foreground_color(hex!("#ffffff")),
            text: Style::new()
                .background_color(hex!("#ffffff"))
                .foreground_color(hex!("#333333")),
            primary_selection: Style::new()
                .background_color(hex!("#add6ff"))
                .foreground_color(hex!("#333333")),
            primary_selection_secondary_cursor: Style::new()
                .background_color(hex!("#808080"))
                .foreground_color(hex!("#ffffff")),
            secondary_selection: Style::new()
                .background_color(hex!("#d7d7d7"))
                .foreground_color(hex!("#333333")),
            secondary_selection_primary_cursor: Style::new()
                .background_color(hex!("#000000"))
                .foreground_color(hex!("#ffffff")),
            secondary_selection_secondary_cursor: Style::new()
                .background_color(hex!("#808080"))
                .foreground_color(hex!("#ffffff")),
        },
        diagnostic: DiagnosticStyles {
            error: Style::new().undercurl(Some(hex!("#ff0000"))),
            warning: Style::new().undercurl(Some(hex!("#ffa500"))),
            info: Style::new().undercurl(Some(hex!("#007acc"))),
            hint: Style::new().undercurl(Some(hex!("#008000"))),
            default: Style::new(),
        },
    }
}

lazy_static! {
    pub static ref VSCODE_LIGHT: Theme = theme();
}
