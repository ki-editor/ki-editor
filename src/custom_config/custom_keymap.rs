//! This file is for you to define your custom keymap.
//! The keymap starts with the leader key `\`.
//! The keymap help starts with the leader key `|`.

#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]

use crate::components::editor_keymap::KeyboardMeaningLayout;
use crate::components::editor_keymap::Meaning::{self, *};

use crate::handle_custom_action::{CustomAction, CustomAction::*, CustomContext, Placeholder::*};
use crate::handle_custom_action::{DirWorking, FileCurrent, SelectionPrimary};

pub(crate) const CUSTOM_KEYMAP_LAYOUT: KeyboardMeaningLayout = [
    [
        __Q__, __W__, __E__, __R__, __T__, /****/ __Y__, __U__, __I__, __O__, __P__,
    ],
    [
        __A__, __S__, __D__, __F__, __G__, /****/ __H__, __J__, __K__, __L__, _SEMI,
    ],
    [
        __Z__, __X__, __C__, __V__, __B__, /****/ __N__, __M__, _COMA, _DOT_, _SLSH,
    ],
];

fn sample_run_command(ctx: &CustomContext) -> CustomAction {
    // Search selected content using Google and Chromium
    RunCommand(
        "chromium",
        vec![
            Str("https://www.google.com/search?q="),
            NoSpace,
            SelectionPrimary::content(),
        ],
    )
}

fn sample_toggle_process(ctx: &CustomContext) -> CustomAction {
    // Render the current file in a new window of Chromium
    if FileCurrent::extension().resolve(ctx) == "html" {
        ToggleProcess(
            "chromium",
            vec![FileCurrent::path_root(), Str("--new-window")],
        )
    } else if FileCurrent::extension().resolve(ctx) == "typ" {
        ToggleProcess(
            "tinymist",
            vec![
                Str("preview"),
                Str("--invert-colors=auto"),
                Str("--open"),
                FileCurrent::path_root(),
            ],
        )
    } else {
        DoNothing
    }
}

fn sample_to_clipboard(ctx: &CustomContext) -> CustomAction {
    ToClipboard(vec![
        Str("Referring to:"),
        SelectionPrimary::content(),
        Str("\nIn file:"),
        FileCurrent::path_local(),
        Str("\nOn line:"),
        SelectionPrimary::row_index(),
    ])
}

fn kitty_cargo_test(ctx: &CustomContext) -> CustomAction {
    RunCommand(
        "kitty",
        vec![
            Str("@"),
            Str("launch"),
            Str("--hold"),
            Str("--no-response"),
            Str("--cwd"),
            DirWorking::path_root(),
            Str("cargo"),
            Str("test"),
            SelectionPrimary::content(),
        ],
    )
}

fn tmux_build(ctx: &CustomContext) -> CustomAction {
    // Compile Rust
    if DirWorking::file_exists("Cargo.toml").resolve(ctx) {
        RunCommand(
            "sh",
            vec![
                Str("-c"),
                Str("tmux select-window -t :1 && tmux send-keys -t :1 \"cargo build\" C-m"),
            ],
        )
    // Compile Zig
    } else if FileCurrent::extension().resolve(ctx) == "zig" {
        RunCommand(
            "sh",
            vec![
                Str("-c"),
                Str("tmux select-window -t :1 && tmux send-keys -t :1 \"zig build\" C-m"),
            ],
        )
    // Compile Cobol
    } else if FileCurrent::extension().resolve(ctx) == "cbl" {
        RunCommand(
            "sh",
            vec![
                Str("-c"),
                Str("tmux select-window -t :1 && tmux send-keys -t :1 \"cobc -x "),
                NoSpace,
                FileCurrent::path_root(),
                NoSpace,
                Str("\" C-m"),
            ],
        )
    // Continue adding hipster languages as needed
    } else {
        DoNothing
    }
}

pub(crate) fn custom_keymap() -> Vec<(
    Meaning,
    &'static str,
    Option<fn(&CustomContext) -> CustomAction>,
)> {
    let custom_keymap: [(Meaning, &str, Option<fn(&CustomContext) -> CustomAction>); 30] = [
        (__Q__, "Sample RunCommand", Some(sample_run_command)),
        (__W__, "Sample ToggleProcess", Some(sample_toggle_process)),
        (__E__, "Sample ToClipboard", Some(sample_to_clipboard)),
        (__R__, "", None),
        (__T__, "", None),
        (__Y__, "", None),
        (__U__, "", None),
        (__I__, "", None),
        (__O__, "", None),
        (__P__, "", None),
        // Second row
        (__A__, "Tmux build", Some(tmux_build)),
        (__S__, "Kitty cargo test", Some(kitty_cargo_test)),
        (__D__, "", None),
        (__F__, "", None),
        (__G__, "", None),
        (__H__, "", None),
        (__J__, "", None),
        (__K__, "", None),
        (__L__, "", None),
        (_SEMI, "", None),
        // Third row
        (__Z__, "", None),
        (__X__, "", None),
        (__C__, "", None),
        (__V__, "", None),
        (__B__, "", None),
        (__N__, "", None),
        (__M__, "", None),
        (_COMA, "", None),
        (_DOT_, "", None),
        (_SLSH, "", None),
    ];
    custom_keymap.into_iter().collect()
}
