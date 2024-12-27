// Alternate layout for Qwerty by Jeremy
//
// The alternate layout is not mnemonic but positional based. This
// makes it especially versitile for those that use alternate
// keyboard layouts, such as Dvorak, Colemak or others. For example,
// the Vim navigation keys have always been a sore spot for the alt
// keyboard community. They only make sense on a Qwerty keyboard.
//
// This layout, being positional, assigns value to keys based on
// how easy it is to press the key, what type of key is it (movement,
// action, etc...) and grouping of keys in a logical position. One
// does not have to try to come up with why "j" is down, or "h" is
// left and "l" is right.
//
// This comes with tremendous freedom in making a very efficient
// modal layout.
//
// Further, this modal layout is designed for hand usage. Generally
// the right hand concerns itself with moving around the file while
// the left hand manipulating content.
//
// With that in mind, the key commands are in a positional order here
// dealing with hand and keyboard row. Not in order of definition in
// `editor_keymap_legend.rs`.

//
// Left Hand
//

pub const QWERTY_NORMAL: [[&'static str; 10]; 3] = [
    ["q", "w", "e", "r", "t", "y", "u", "i", "o", "p"],
    ["a", "s", "d", "f", "g", "h", "j", "k", "l", ";"],
    ["z", "x", "c", "v", "b", "n", "m", ",", ".", "/"],
];

pub const QWERTY_SHIFTED: [[&'static str; 10]; 3] = [
    ["Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P"],
    ["A", "S", "D", "F", "G", "H", "J", "K", "L", ":"],
    ["Z", "X", "C", "V", "B", "N", "M", "<", ">", "?"],
];

pub const QWERTY_CONTROL: [[&'static str; 10]; 3] = [
    [
        "ctrl+q", "ctrl+w", "ctrl+e", "ctrl+r", "ctrl+t", "ctrl+y", "ctrl+u", "ctrl+i", "ctrl+o",
        "ctrl+p",
    ],
    [
        "ctrl+a", "ctrl+s", "ctrl+d", "ctrl+f", "ctrl+g", "ctrl+h", "ctrl+j", "ctrl+k", "ctrl+l",
        "ctrl+;",
    ],
    [
        "ctrl+z", "ctrl+x", "ctrl+c", "ctrl+v", "ctrl+b", "ctrl+n", "ctrl+m", "ctrl+,", "ctrl+.",
        "ctrl+/",
    ],
];

// -- DVORAK --

pub const DVORAK_NORMAL: [[&'static str; 10]; 3] = [
    ["'", ",", ".", "p", "y", "f", "g", "c", "r", "l"],
    ["a", "o", "e", "i", "u", "d", "h", "t", "n", "s"],
    [";", "q", "j", "k", "x", "b", "m", "w", "v", "z"],
];

pub const DVORAK_SHIFTED: [[&'static str; 10]; 3] = [
    ["\"", "<", ">", "P", "Y", "F", "G", "C", "R", "L"],
    ["A", "O", "E", "I", "U", "D", "H", "T", "N", "S"],
    [":", "Q", "J", "K", "X", "B", "M", "W", "V", "Z"],
];

pub const DVORAK_CONTROL: [[&'static str; 10]; 3] = [
    [
        "ctrl+'", "ctrl+,", "ctrl+.", "ctrl+p", "ctrl+y", "ctrl+f", "ctrl+g", "ctrl+c", "ctrl+r",
        "ctrl+l",
    ],
    [
        "ctrl+a", "ctrl+o", "ctrl+e", "ctrl+i", "ctrl+u", "ctrl+d", "ctrl+h", "ctrl+t", "ctrl+n",
        "ctrl+s",
    ],
    [
        "ctrl+;", "ctrl+q", "ctrl+j", "ctrl+k", "ctrl+x", "ctrl+b", "ctrl+m", "ctrl+w", "ctrl+v",
        "ctrl+z",
    ],
];

// -- COLEMAK --

pub const COLEMAK_NORMAL: [[&'static str; 10]; 3] = [
    ["q", "w", "f", "p", "b", "j", "l", "u", "y", ";"],
    ["a", "r", "s", "t", "g", "m", "n", "e", "i", "o"],
    ["z", "x", "c", "d", "v", "k", "h", ",", ".", "/"],
];

pub const COLEMAK_SHIFTED: [[&'static str; 10]; 3] = [
    ["Q", "W", "F", "P", "B", "J", "L", "U", "Y", ":"],
    ["A", "R", "S", "T", "G", "M", "N", "E", "I", "O"],
    ["Z", "X", "C", "D", "V", "K", "H", "<", ">", "?"],
];

pub const COLEMAK_CONTROL: [[&'static str; 10]; 3] = [
    [
        "ctrl+q", "ctrl+w", "ctrl+f", "ctrl+p", "ctrl+b", "ctrl+j", "ctrl+l", "ctrl+u", "ctrl+y",
        "ctrl+;",
    ],
    [
        "ctrl+a", "ctrl+r", "ctrl+s", "ctrl+t", "ctrl+g", "ctrl+m", "ctrl+n", "ctrl+e", "ctrl+i",
        "ctrl+o",
    ],
    [
        "ctrl+z", "ctrl+x", "ctrl+c", "ctrl+d", "ctrl+v", "ctrl+k", "ctrl+h", "ctrl+,", "ctrl+.",
        "ctrl+/",
    ],
];

use std::{cell::OnceCell, collections::HashMap};

use once_cell::sync::Lazy;
use Meaning::*;

pub const KEYMAP_NORMAL: [[Meaning; 10]; 3] = [
    [
        WORD_, VMODE, CHNG_, MULTC, SRCHC, /****/ MARK_, INSTP, UP___, INSTN, CSRCH,
    ],
    [
        LINE_, TOKEN, SYTX_, DELTN, SRCHN, /****/ PREV_, LEFT_, DOWN_, RIGHT, NEXT_,
    ],
    [
        UNDO_, EXCHG, COPY_, PSTEN, RPLC_, /****/ GLOBL, FIRST, JUMP_, LAST_, TRSFM,
    ],
];

pub const KEYMAP_SHIFTED: [[Meaning; 10]; 3] = [
    [
        CHAR_, DEDNT, CHNGX, INDNT, LSTNC, /****/ FILEP, OPENP, JOIN_, OPENN, FILEN,
    ],
    [
        LINEF, RAISE, SYTXF, DELTP, SRCHP, /****/ BUFFP, FINDP, BREAK, FINDN, BUFFN,
    ],
    [
        REDO_, XACHR, TOIDX, PSTEP, RPLCX, /****/ CRSRP, GBACK, XACHR, GFORW, CRSRN,
    ],
];

pub const KEYMAP_NORMAL_CONTROL: [[Meaning; 10]; 3] = [
    [
        _____, _____, _____, _____, _____, /****/ _____, RPLCP, SCRLU, RPLCN, SVIEW,
    ],
    [
        _____, _____, _____, WCLSE, _____, /****/ _____, _____, SCRLD, _____, _____,
    ],
    [
        UNDO_, _____, _____, UPSTE, PRPLC, /****/ _____, _____, WSWTH, _____, _____,
    ],
];

static QWERTY_NORMAL_KEYS: Lazy<HashMap<Meaning, &str>> = Lazy::new(|| {
    HashMap::from_iter(
        KEYMAP_NORMAL
            .into_iter()
            .flatten()
            .zip(QWERTY_NORMAL.into_iter().flatten()),
    )
});

static QWERTY_SHIFTED_KEYS: Lazy<HashMap<Meaning, &str>> = Lazy::new(|| {
    HashMap::from_iter(
        KEYMAP_SHIFTED
            .into_iter()
            .flatten()
            .zip(QWERTY_SHIFTED.into_iter().flatten()),
    )
});

static QWERTY_NORMAL_CONTROL_KEYS: Lazy<HashMap<Meaning, &str>> = Lazy::new(|| {
    HashMap::from_iter(
        KEYMAP_NORMAL_CONTROL
            .into_iter()
            .flatten()
            .zip(QWERTY_CONTROL.into_iter().flatten()),
    )
});

static DVORAK_NORMAL_KEYS: Lazy<HashMap<Meaning, &str>> = Lazy::new(|| {
    HashMap::from_iter(
        KEYMAP_NORMAL
            .into_iter()
            .flatten()
            .zip(DVORAK_NORMAL.into_iter().flatten()),
    )
});

static DVORAK_SHIFTED_KEYS: Lazy<HashMap<Meaning, &str>> = Lazy::new(|| {
    HashMap::from_iter(
        KEYMAP_SHIFTED
            .into_iter()
            .flatten()
            .zip(DVORAK_SHIFTED.into_iter().flatten()),
    )
});

static DVORAK_NORMAL_CONTROL_KEYS: Lazy<HashMap<Meaning, &str>> = Lazy::new(|| {
    HashMap::from_iter(
        KEYMAP_NORMAL_CONTROL
            .into_iter()
            .flatten()
            .zip(DVORAK_CONTROL.into_iter().flatten()),
    )
});

static COLEMAK_NORMAL_KEYS: Lazy<HashMap<Meaning, &str>> = Lazy::new(|| {
    HashMap::from_iter(
        KEYMAP_NORMAL
            .into_iter()
            .flatten()
            .zip(COLEMAK_NORMAL.into_iter().flatten()),
    )
});

static COLEMAK_SHIFTED_KEYS: Lazy<HashMap<Meaning, &str>> = Lazy::new(|| {
    HashMap::from_iter(
        KEYMAP_SHIFTED
            .into_iter()
            .flatten()
            .zip(COLEMAK_SHIFTED.into_iter().flatten()),
    )
});

static COLEMAK_NORMAL_CONTROL_KEYS: Lazy<HashMap<Meaning, &str>> = Lazy::new(|| {
    HashMap::from_iter(
        KEYMAP_NORMAL_CONTROL
            .into_iter()
            .flatten()
            .zip(COLEMAK_CONTROL.into_iter().flatten()),
    )
});

pub(crate) static KEYBOARD_LAYOUT: Lazy<KeyboardLayout> = Lazy::new(|| {
    use KeyboardLayout::*;
    crate::env::parse_env(
        "KI_EDITOR_KEYBOARD",
        &[QWERTY, DVORAK, COLEMAK],
        |layout| layout.as_str(),
        QWERTY,
    )
});

#[derive(Debug, Clone)]
pub(crate) enum KeyboardLayout {
    QWERTY,
    DVORAK,
    COLEMAK,
}

impl KeyboardLayout {
    const fn as_str(&self) -> &'static str {
        match self {
            KeyboardLayout::QWERTY => "QWERTY",
            KeyboardLayout::DVORAK => "DVORAK",
            KeyboardLayout::COLEMAK => "COLEMAK",
        }
    }
    pub(crate) fn get_key(&self, meaning: &Meaning) -> &'static str {
        let (normal, shifted, control) = match self {
            KeyboardLayout::QWERTY => (
                &QWERTY_NORMAL_KEYS,
                &QWERTY_SHIFTED_KEYS,
                &QWERTY_NORMAL_CONTROL_KEYS,
            ),
            KeyboardLayout::DVORAK => (
                &DVORAK_NORMAL_KEYS,
                &DVORAK_SHIFTED_KEYS,
                &DVORAK_NORMAL_CONTROL_KEYS,
            ),
            KeyboardLayout::COLEMAK => (
                &COLEMAK_NORMAL_KEYS,
                &COLEMAK_SHIFTED_KEYS,
                &COLEMAK_NORMAL_CONTROL_KEYS,
            ),
        };
        normal
            .get(meaning)
            .or_else(|| shifted.get(meaning))
            .or_else(|| control.get(meaning))
            .cloned()
            .expect(&format!("Unable to find key binding of {meaning:#?}"))
    }
}

/// Postfix N = Next, Postfix P = Previous
/// X means Swap/Cut
/// Prefix W means Window
#[derive(Hash, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Meaning {
    /// Empty, not assigned
    _____,
    TOIDX,
    INDNT,
    UPSTE,
    DEDNT,
    OPENP,
    OPENN,
    JOIN_,
    SVIEW,
    LINEF,
    BUFFN,
    BUFFP,
    SYTXF,
    RAISE,
    FILEP,
    FILEN,
    FINDP,
    GBACK,
    GFORW,
    WCLSE,
    WSWTH,
    FINDN,
    BREAK,
    CRSRP,
    CRSRN,
    XACHR,
    UNDO_,
    PRPLC,
    RPLCP,
    RPLCN,
    SCRLU,
    SCRLD,
    LSTNC,
    REDO_,
    EXCHG,
    COPY_,
    PSTEN,
    PSTEP,
    RPLC_,
    RPLCX,
    LEFT_,
    RIGHT,
    DELTP,
    DELTN,
    SRCHN,
    SRCHP,
    PREV_,
    NEXT_,
    DOWN_,
    GLOBL,
    FIRST,
    JUMP_,
    LAST_,
    TRSFM,
    MARK_,
    INSTP,
    INSTN,
    UP___,

    WORD_,
    VMODE,
    CHNG_,
    CHNGX,
    MULTC,
    SRCHC,
    CHAR_,
    LINE_,
    TOKEN,
    SYTX_,
    CSRCH,
}

// Row 1 (qwert)
pub const SELECTION_MODE_WORD: &str = "q";
pub const SELECTION_MODE_CHARACTER: &str = "Q";
pub const MOVEMENT_MC_ENTER: &str = "w";
pub const ACTION_CHANGE: &str = "e";
pub const CLIPBOARD_CHANGE_CUT: &str = "E";
pub const ACTION_ENTER_V_MODE: &str = "r";
pub const ACTION_SELECT_ALL: &str = "R";
pub const ACTION_SEARCH_CURRENT_SELECTION: &str = "t";
pub const SELECTION_MODE_LAST_CONTIGUOUS: &str = "T";

// Row 2 (asdfg)
pub const SELECTION_MODE_LINE: &str = "a";
pub const SELECTION_MODE_FULL_LINE: &str = "A";
pub const SELECTION_MODE_TOKEN: &str = "s";
pub const SELECTION_MODE_SYNTAX: &str = "d";
pub const SELECTION_MODE_FINE_SYNTAX: &str = "D";
pub const ACTION_DELETE_END: &str = "f";
pub const ACTION_DELETE_START: &str = "F";
pub const ACTION_SEARCH_FORWARD: &str = "g";
pub const ACTION_SEARCH_BACKWARD: &str = "G";

// Row 3 (zxcvb)
pub const ACTION_UNDO: &str = "z";
pub const ACTION_REDO: &str = "Z";
pub const MOVEMENT_EXCHANGE_MODE: &str = "x";
pub const CLIPBOARD_YANK: &str = "c";
pub const CLIPBOARD_PASTE_END: &str = "v";
pub const CLIPBOARD_PASTE_START: &str = "V";
pub const CLIPBOARD_REPLACE_WITH_COPIED_TEXT: &str = "b";
pub const CLIPBOARD_REPLACE_CUT: &str = "B";

//
// Right Hand
//

// Row 1 (yuiop)
pub const ACTION_TOGGLE_MARK: &str = "y";
pub const ACTION_INSERT_START: &str = "u";
pub const ACTION_OPEN_START: &str = "U";
pub const MOVEMENT_CORE_UP: &str = "i";
pub const ACTION_JOIN: &str = "I";
pub const ACTION_INSERT_END: &str = "o";
pub const ACTION_OPEN_END: &str = "O";
pub const ACTION_CONFIGURE_SEARCH: &str = "p";

// Row 2 (hjkl;)
pub const MOVEMENT_CORE_PREV: &str = "h";
pub const MOVEMENT_OTHER_GO_TO_PREVIOUS_FILE: &str = "H";
pub const MOVEMENT_CORE_LEFT: &str = "j";
pub const SELECTION_MODE_FIND_LOCAL_BACKWARD: &str = "J";
pub const MOVEMENT_CORE_DOWN: &str = "k";
pub const ACTION_BREAK: &str = "K";
pub const MOVEMENT_CORE_RIGHT: &str = "l";
pub const SELECTION_MODE_FIND_LOCAL_FORWARD: &str = "L";
pub const MOVEMENT_CORE_NEXT: &str = ";";
pub const MOVEMENT_OTHER_GO_TO_NEXT_FILE: &str = ":";

// Row 3 (nm,./)
pub const SELECTION_MODE_FIND_GLOBAL: &str = "n";
pub const MOVEMENT_CORE_FIRST: &str = "m";
pub const ACTION_DEDENT: &str = "M";
pub const MOVEMENT_CORE_JUMP: &str = ",";
pub const MOVEMENT_OTHER_SWAP: &str = "<";
pub const MOVEMENT_CORE_LAST: &str = ".";
pub const ACTION_INDENT: &str = ">";
pub const ACTION_TRANSFORM: &str = "/";

// Multi-cursor
pub const ACTION_MC_DELETE_PRIMARY_CURSOR_START: &str = "h";
pub const ACTION_MC_DELETE_PRIMARY_CURSOR_END: &str = "H";
pub const ACTION_MC_MAINTAIN_SELECTIONS: &str = "n";
pub const ACTION_MC_KEEP_ONLY_PRIMARY_CURSOR: &str = "e";
pub const CLIPBOARD_MC_REMOVE_MATCHING_SEARCH: &str = "E";

// Other
pub const ACTION_SAVE: &str = "enter";
pub const MOVEMENT_CORE_TO_INDEX: &str = "0";
pub const MOVEMENT_OTHER_CYCLE_START: &str = "(";
pub const MOVEMENT_OTHER_CYCLE_END: &str = ")";
pub const MOVEMENT_OTHER_SCROLL_DOWN: &str = "ctrl+d";
pub const MOVEMENT_OTHER_SCROLL_UP: &str = "ctrl+u";
pub const MOVEMENT_OTHER_GO_BACK: &str = "ctrl+o";
pub const MOVEMENT_OTHER_GO_FORWARD: &str = "ctrl+i";
pub const ACTION_RAISE: &str = "^";
pub const ACTION_SWITCH_EXTENDED_SELECTION_END: &str = "o";
pub const ACTION_REPLACE_WITH_PATTERN: &str = "ctrl+r";
pub const ACTION_REPLACE_WITH_PREVIOUS_COPIED_TEXT: &str = "ctrl+p";
pub const ACTION_REPLACE_WITH_NEXT_COPIED_TEXT: &str = "ctrl+n";
pub const ACTION_COLLAPSE_SELECTION: &str = "$";
pub const ACTION_PIPE: &str = "|";
pub const UNIVERSAL_CLOSE_WINDOW: &str = "ctrl+c";
pub const UNIVERSAL_SWITCH_VIEW_ALIGNMENT: &str = "ctrl+l";
pub const UNIVERSAL_SWITCH_WINDOW: &str = "ctrl+s";
pub const UNIVERSAL_PASTE: &str = "ctrl+v";
