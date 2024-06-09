use super::{DiagnosticStyles, HighlightName, Theme, UiStyles};
use crate::{
    style::{fg, Style},
    themes::SyntaxStyles,
};
use my_proc_macros::hex;

pub fn vscode_light() -> Theme {
    Theme {
        name: "VS Code (Light)".to_string(),
        syntax: SyntaxStyles::new({
            use HighlightName::*;
            &[
                (Variable, fg(hex!("#001080"))),
                (Keyword, fg(hex!("#af00db"))),
                (KeywordModifier, fg(hex!("#0000ff"))),
                (Function, fg(hex!("#795e26"))),
                (Type, fg(hex!("#267f99"))),
                (TypeBuiltin, fg(hex!("#0000ff"))),
                (String, fg(hex!("#a31515"))),
                (Comment, fg(hex!("#008000"))),
                (Tag, fg(hex!("#267f99"))),
                (TagAttribute, fg(hex!("#e50000"))),
            ]
        }),
        ui: UiStyles {
            global_title: Style::new()
                .foreground_color(hex!("#ffffff"))
                .background_color(hex!("#3478c6")),
            window_title_focused: Style::new()
                .foreground_color(hex!("#FFFFFF"))
                .background_color(hex!("#2C2C2C")),
            window_title_unfocused: Style::new()
                .foreground_color(hex!("#FFFFFF"))
                .background_color(hex!("#aaaaaa")),
            parent_lines_background: hex!("#E6EBF0"),
            jump_mark_odd: Style::new()
                .background_color(hex!("#b5485d"))
                .foreground_color(hex!("#ffffff")),
            jump_mark_even: Style::new()
                .background_color(hex!("#84b701"))
                .foreground_color(hex!("#ffffff")),
            background_color: hex!("#ffffff"),
            text_foreground: hex!("#333333"),
            primary_selection_background: hex!("#c7e6ff"),
            primary_selection_anchor_background: hex!("#add6ff"),
            primary_selection_secondary_cursor: Style::new()
                .background_color(hex!("#808080"))
                .foreground_color(hex!("#ffffff")),
            secondary_selection_background: hex!("#ebebeb"),
            secondary_selection_anchor_background: hex!("#d7d7d7"),
            secondary_selection_primary_cursor: Style::new()
                .background_color(hex!("#000000"))
                .foreground_color(hex!("#ffffff")),
            secondary_selection_secondary_cursor: Style::new()
                .background_color(hex!("#808080"))
                .foreground_color(hex!("#ffffff")),
            line_number: Style::new().foreground_color(hex!("#6a9955")),
            border: Style::new()
                .foreground_color(hex!("#aaaaaa"))
                .background_color(hex!("#ffffff")),
            bookmark: Style::new().background_color(hex!("#ffcc00")),
            possible_selection_background: hex!("#f6f7b2"),
            keymap_hint: Style::new().underline(hex!("#af00db")),
            keymap_key: Style::new().bold().foreground_color(hex!("#af00db")),
            keymap_arrow: Style::new().foreground_color(hex!("#808080")),
            fuzzy_matched_char: Style::new().foreground_color(hex!("#ff0000")),
        },
        diagnostic: DiagnosticStyles::default(),
        hunk: super::HunkStyles::light(),
    }
}
