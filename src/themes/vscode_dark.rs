use super::{DiagnosticStyles, HighlightName, Theme, UiStyles};
use crate::{
    style::{fg, Style},
    themes::SyntaxStyles,
};
use my_proc_macros::hex;

pub fn VSCODE_DARK() -> Theme {
    Theme {
        name: "vscode-dark".to_string(),
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
            window_title: Style::new()
                .foreground_color(hex!("#969696"))
                .background_color(hex!("#2D2D2D")),
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
            line_number_separator: Style::new().foreground_color(hex!("#1E1E1E")),
            bookmark: Style::new().background_color(hex!("#ffcc00")),
            possible_selection_background: hex!("#5C3521"),
            keymap_hint: Style::new().underline(hex!("#af00db")),
            keymap_key: Style::new().bold().foreground_color(hex!("#af00db")),
            keymap_arrow: Style::new().foreground_color(hex!("#808080")),
            keymap_description: Style::new().foreground_color(hex!("#FFFFFF")),
            fuzzy_matched_char: Style::new().foreground_color(hex!("#55A8F8")),
        },
        diagnostic: DiagnosticStyles::default(),
        hunk_new_background: hex!("#383D2C"),
        hunk_old_background: hex!("#47221F"),
        hunk_old_emphasized_background: hex!("#682520"),
        hunk_new_emphasized_background: hex!("#4E5A32"),
    }
}
