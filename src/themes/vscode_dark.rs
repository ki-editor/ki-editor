use super::{DiagnosticStyles, HighlightName, Theme, UiStyles};
use crate::{
    style::{fg, Style},
    themes::SyntaxStyles,
};
use my_proc_macros::hex;

pub fn vscode_dark() -> Theme {
    Theme {
        name: "VS Code (Dark)".to_string(),
        syntax: SyntaxStyles::new({
            use HighlightName::*;
            &[
                (Variable, fg(hex!("#aadafa"))),
                (Keyword, fg(hex!("#679ad1"))),
                (KeywordModifier, fg(hex!("#679ad1"))),
                (Function, fg(hex!("#dcdcaf"))),
                (Type, fg(hex!("#71c6b1"))),
                (TypeBuiltin, fg(hex!("#71c6b1"))),
                (String, fg(hex!("#c5947c"))),
                (Comment, fg(hex!("#74985d"))),
                (Tag, fg(hex!("#71c6b1"))),
                (TagAttribute, fg(hex!("#aadafa"))),
            ]
        }),
        ui: UiStyles {
            global_title: Style::new()
                .foreground_color(hex!("#ffffff"))
                .background_color(hex!("#3478c6")),
            window_title_focused: Style::new()
                .foreground_color(hex!("#444444"))
                .background_color(hex!("#ffffff")),
            window_title_unfocused: Style::new()
                .foreground_color(hex!("#969696"))
                .background_color(hex!("#444444")),
            parent_lines_background: hex!("#3B3D41"),
            jump_mark_odd: Style::new()
                .background_color(hex!("#b5485d"))
                .foreground_color(hex!("#ffffff")),
            jump_mark_even: Style::new()
                .background_color(hex!("#84b701"))
                .foreground_color(hex!("#ffffff")),
            background_color: hex!("#1E1E1E"),
            text_foreground: hex!("#FFFFFF"),
            primary_selection_background: hex!("#304E75"),
            primary_selection_anchor_background: hex!("#304E75"),
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
            line_number: Style::new().foreground_color(hex!("#858585")),
            border: Style::new()
                .background_color(hex!("#1E1E1E"))
                .foreground_color(hex!("#858585")),
            mark: Style::new().background_color(hex!("#ffcc00")),
            possible_selection_background: hex!("#5C3521"),
            keymap_hint: Style::new().underline(hex!("#af00db")),
            keymap_key: Style::new().bold().foreground_color(hex!("#af00db")),
            keymap_arrow: Style::new().foreground_color(hex!("#808080")),
            fuzzy_matched_char: Style::new().foreground_color(hex!("#55A8F8")),
        },
        diagnostic: DiagnosticStyles::default(),
        hunk: super::HunkStyles::dark(),
    }
}
