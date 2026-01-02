//! This file is for you to define your custom keymap.
//! The keymap starts with the leader key `\`.
//! The keymap help starts with the leader key `|`.

#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]

use crate::components::editor_keymap::KeyboardMeaningLayout;
use crate::components::editor_keymap::Meaning::{self, *};
use crate::config::AppConfig;
use crate::handle_custom_action::{
    CustomAction, CustomAction::*, CustomActionKeymap, CustomContext,
};
use crate::handle_custom_action::{DirWorking, FileCurrent, Placeholder::*, SelectionPrimary};

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

// Assign keybinds here, with respect to the qwerty layout above
// Compile, relaunch and press `\` (direct) or `|` (help) to use these
pub(crate) fn custom_keymap() -> Vec<CustomActionKeymap> {
    let meanings: [Meaning; 30] = [
        // First row
        __Q__, __W__, __E__, __R__, __T__, __Y__, __U__, __I__, __O__, __P__,
        // Second row
        __A__, __S__, __D__, __F__, __G__, __H__, __J__, __K__, __L__, _SEMI,
        // Third row
        __Z__, __X__, __C__, __V__, __B__, __N__, __M__, _COMA, _DOT_, _SLSH,
    ];
    AppConfig::singleton()
        .leader_keymap()
        .keybindings()
        .into_iter()
        .flat_map(|keybindings| {
            keybindings
                .iter()
                .filter_map(|keybinding| keybinding.clone())
        })
        .zip(meanings)
        .map(|(keybinding, meaning)| {
            (
                meaning,
                keybinding.description.clone(),
                keybinding.action.clone(),
            )
        })
        .collect()
}
