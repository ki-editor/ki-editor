use super::{DiagnosticStyles, Theme, UiStyles};
use crate::{
    style::{fg, Style},
    themes::SyntaxStyles,
};
use my_proc_macros::hex;

pub static VSCODE_LIGHT: Theme = Theme {
    name: "vscode-light",
    syntax: SyntaxStyles::new(&[
        ("variable", fg(hex!("#001080"))),
        ("keyword", fg(hex!("#af00db"))),
        ("keyword.modifier", fg(hex!("#0000ff"))),
        ("function", fg(hex!("#795e26"))),
        ("type", fg(hex!("#267f99"))),
        ("type.builtin", fg(hex!("#0000ff"))),
        ("string", fg(hex!("#a31515"))),
        ("comment", fg(hex!("#008000"))),
        ("tag", fg(hex!("#267f99"))),
        ("tag.attribute", fg(hex!("#e50000"))),
    ]),
    ui: UiStyles {
        global_title: Style::new()
            .foreground_color(hex!("#ffffff"))
            .background_color(hex!("#000000")),
        window_title: Style::new()
            .foreground_color(hex!("#ffffff"))
            .background_color(hex!("#505050")),
        parent_lines_background: hex!("#efefef"),
        jump_mark_odd: Style::new()
            .background_color(hex!("#B5485D"))
            .foreground_color(hex!("#ffffff")),
        jump_mark_even: Style::new()
            .background_color(hex!("#84B701"))
            .foreground_color(hex!("#ffffff")),
        text: Style::new()
            .background_color(hex!("#ffffff"))
            .foreground_color(hex!("#333333")),
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
        line_number: Style::new()
            .background_color(hex!("#ffffff"))
            .foreground_color(hex!("#6a9955")),
        line_number_separator: Style::new()
            .background_color(hex!("#ffffff"))
            .foreground_color(hex!("#d7d7d7")),
        bookmark: Style::new().background_color(hex!("#ffcc00")),
        possible_selection_background: hex!("#f6f7b2"),
        keymap_hint: Style::new().underline(hex!("#af00db")),
        keymap_key: Style::new().bold().foreground_color(hex!("#af00db")),
        keymap_arrow: Style::new().foreground_color(hex!("#808080")),
        keymap_description: Style::new().foreground_color(hex!("#000000")),
    },
    diagnostic: DiagnosticStyles {
        error: Style::new().undercurl(hex!("#ff0000")),
        warning: Style::new().undercurl(hex!("#ffa500")),
        information: Style::new().undercurl(hex!("#007acc")),
        hint: Style::new().undercurl(hex!("#008000")),
        default: Style::new(),
    },
    hunk_new_background: hex!("#EBFEED"),
    hunk_old_background: hex!("#FCECEA"),
    hunk_old_emphasized_background: hex!("#F9D8D6"),
    hunk_new_emphasized_background: hex!("#BAF0C0"),
};
