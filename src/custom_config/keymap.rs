//! This file is for user to define custom keymap.
//! The keymap starts with the leader key `\`.

use shared::canonicalized_path::CanonicalizedPath;

use crate::components::editor_keymap::{
    KeyboardMeaningLayout,
    Meaning::{self, *},
};

pub(crate) const KEYMAP_LEADER: KeyboardMeaningLayout = [
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

pub(crate) struct LeaderContext {
    pub(crate) path: Option<CanonicalizedPath>,
    /// 0-based index
    pub(crate) primary_selection_line_index: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum LeaderAction {
    RunCommand(&'static str, Vec<String>),
    DoNothing,
}

use LeaderAction::*;

pub(crate) fn leader_keymap(context: &LeaderContext) -> Vec<(Meaning, &'static str, LeaderAction)> {
    [
        (__Q__, "Sample", sample(context)),
        (__W__, "", DoNothing),
        (__E__, "", DoNothing),
        (__R__, "", DoNothing),
        (__T__, "", DoNothing),
        (__Y__, "", DoNothing),
        (__U__, "", DoNothing),
        (__I__, "", DoNothing),
        (__O__, "", DoNothing),
        (__P__, "", DoNothing),
        // Second row
        (__A__, "", DoNothing),
        (__S__, "", DoNothing),
        (__D__, "", DoNothing),
        (__F__, "", DoNothing),
        (__G__, "", DoNothing),
        (__H__, "", DoNothing),
        (__J__, "", DoNothing),
        (__K__, "", DoNothing),
        (__L__, "", DoNothing),
        (_SEMI, "", DoNothing),
        // Third row
        (__Z__, "", DoNothing),
        (__X__, "", DoNothing),
        (__C__, "", DoNothing),
        (__V__, "", DoNothing),
        (__B__, "", DoNothing),
        (__N__, "", DoNothing),
        (__M__, "", DoNothing),
        (_COMA, "", DoNothing),
        (_DOT_, "", DoNothing),
        (_SLSH, "", DoNothing),
    ]
    .into_iter()
    .collect()
}

fn sample(context: &LeaderContext) -> LeaderAction {
    RunCommand(
        "echo",
        [
            "The current file is".to_string(),
            context
                .path
                .as_ref()
                .map(|path| path.display_absolute())
                .unwrap_or_default(),
            "The current line is".to_string(),
            (context.primary_selection_line_index + 1).to_string(),
        ]
        .to_vec(),
    )
}
