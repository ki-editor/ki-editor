pub const QWERTY_NORMAL: [[&str; 10]; 3] = [
    ["q", "w", "e", "r", "t", "y", "u", "i", "o", "p"],
    ["a", "s", "d", "f", "g", "h", "j", "k", "l", ";"],
    ["z", "x", "c", "v", "b", "n", "m", ",", ".", "/"],
];

pub const QWERTY_SHIFTED: [[&str; 10]; 3] = [
    ["Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P"],
    ["A", "S", "D", "F", "G", "H", "J", "K", "L", ":"],
    ["Z", "X", "C", "V", "B", "N", "M", "<", ">", "?"],
];

pub const QWERTY_CONTROL: [[&str; 10]; 3] = [
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

pub const DVORAK_NORMAL: [[&str; 10]; 3] = [
    ["'", ",", ".", "p", "y", "f", "g", "c", "r", "l"],
    ["a", "o", "e", "i", "u", "d", "h", "t", "n", "s"],
    [";", "q", "j", "k", "x", "b", "m", "w", "v", "z"],
];

pub const DVORAK_SHIFTED: [[&str; 10]; 3] = [
    ["\"", "<", ">", "P", "Y", "F", "G", "C", "R", "L"],
    ["A", "O", "E", "I", "U", "D", "H", "T", "N", "S"],
    [":", "Q", "J", "K", "X", "B", "M", "W", "V", "Z"],
];

pub const DVORAK_CONTROL: [[&str; 10]; 3] = [
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

pub const COLEMAK_NORMAL: [[&str; 10]; 3] = [
    ["q", "w", "f", "p", "b", "j", "l", "u", "y", ";"],
    ["a", "r", "s", "t", "g", "m", "n", "e", "i", "o"],
    ["z", "x", "c", "d", "v", "k", "h", ",", ".", "/"],
];

pub const COLEMAK_SHIFTED: [[&str; 10]; 3] = [
    ["Q", "W", "F", "P", "B", "J", "L", "U", "Y", ":"],
    ["A", "R", "S", "T", "G", "M", "N", "E", "I", "O"],
    ["Z", "X", "C", "D", "V", "K", "H", "<", ">", "?"],
];

pub const COLEMAK_CONTROL: [[&str; 10]; 3] = [
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

// -- COLEMAK_DH --

pub const COLEMAK_DH_NORMAL: [[&str; 10]; 3] = [
    ["q", "w", "f", "p", "b", "j", "l", "u", "y", ";"],
    ["a", "r", "s", "t", "g", "m", "n", "e", "i", "o"],
    ["z", "x", "c", "d", "v", "k", "h", ",", ".", "/"],
];

pub const COLEMAK_DH_SHIFTED: [[&str; 10]; 3] = [
    ["Q", "W", "F", "P", "B", "J", "L", "U", "Y", ":"],
    ["A", "R", "S", "T", "G", "M", "N", "E", "I", "O"],
    ["Z", "X", "C", "D", "V", "K", "H", "<", ">", "?"],
];

pub const COLEMAK_DH_CONTROL: [[&str; 10]; 3] = [
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

use std::collections::HashMap;

use once_cell::sync::Lazy;
use Meaning::*;

pub const KEYMAP_NORMAL: [[Meaning; 10]; 3] = [
    [
        Word_, VMode, Chng_, MultC, SrchC, /****/ Mark_, InstP, Up___, InstN, CSrch,
    ],
    [
        Line_, Token, Sytx_, DeltN, SrchN, /****/ Prev_, Left_, Down_, Right, Next_,
    ],
    [
        Undo_, Exchg, Copy_, PsteN, Rplc_, /****/ Globl, First, Jump_, Last_, Trsfm,
    ],
];

pub const KEYMAP_SHIFTED: [[Meaning; 10]; 3] = [
    [
        Char_, DeDnt, ChngX, Indnt, LstNc, /****/ FileP, OpenP, Join_, OpenN, FileN,
    ],
    [
        LineF, Raise, StyxF, DeltP, SrchP, /****/ BuffP, FindP, Break, FindN, BuffN,
    ],
    [
        Redo_, XAnchr, ToIdx, PsteP, RplcX, /****/ CrsrP, GBack, XAnchr, GForw, GrsrN,
    ],
];

pub const KEYMAP_NORMAL_CONTROL: [[Meaning; 10]; 3] = [
    [
        _____, _____, _____, _____, _____, /****/ _____, RplcP, ScrlU, RplcN, SView,
    ],
    [
        _____, _____, _____, WClse, _____, /****/ _____, _____, ScrlD, _____, _____,
    ],
    [
        Undo_, _____, _____, UPstE, PRplc, /****/ _____, _____, WSwth, _____, _____,
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

static COLEMAK_DH_NORMAL_KEYS: Lazy<HashMap<Meaning, &str>> = Lazy::new(|| {
    HashMap::from_iter(
        KEYMAP_NORMAL
            .into_iter()
            .flatten()
            .zip(COLEMAK_DH_NORMAL.into_iter().flatten()),
    )
});

static COLEMAK_DH_SHIFTED_KEYS: Lazy<HashMap<Meaning, &str>> = Lazy::new(|| {
    HashMap::from_iter(
        KEYMAP_SHIFTED
            .into_iter()
            .flatten()
            .zip(COLEMAK_DH_SHIFTED.into_iter().flatten()),
    )
});

static COLEMAK_DH_NORMAL_CONTROL_KEYS: Lazy<HashMap<Meaning, &str>> = Lazy::new(|| {
    HashMap::from_iter(
        KEYMAP_NORMAL_CONTROL
            .into_iter()
            .flatten()
            .zip(COLEMAK_DH_CONTROL.into_iter().flatten()),
    )
});

pub(crate) static KEYBOARD_LAYOUT: Lazy<KeyboardLayout> = Lazy::new(|| {
    use KeyboardLayout::*;
    crate::env::parse_env(
        "KI_EDITOR_KEYBOARD",
        &[Qwerty, Dvorak, Colemak, ColemakDh],
        |layout| layout.as_str(),
        Qwerty,
    )
});

#[derive(Debug, Clone)]
pub(crate) enum KeyboardLayout {
    Qwerty,
    Dvorak,
    Colemak,
    ColemakDh,
}

impl KeyboardLayout {
    const fn as_str(&self) -> &'static str {
        match self {
            KeyboardLayout::Qwerty => "QWERTY",
            KeyboardLayout::Dvorak => "DVORAK",
            KeyboardLayout::Colemak => "COLEMAK",
            KeyboardLayout::ColemakDh => "COLEMAK_DH",
        }
    }
    pub(crate) fn get_key(&self, meaning: &Meaning) -> &'static str {
        let (normal, shifted, control) = match self {
            KeyboardLayout::Qwerty => (
                &QWERTY_NORMAL_KEYS,
                &QWERTY_SHIFTED_KEYS,
                &QWERTY_NORMAL_CONTROL_KEYS,
            ),
            KeyboardLayout::Dvorak => (
                &DVORAK_NORMAL_KEYS,
                &DVORAK_SHIFTED_KEYS,
                &DVORAK_NORMAL_CONTROL_KEYS,
            ),
            KeyboardLayout::Colemak => (
                &COLEMAK_NORMAL_KEYS,
                &COLEMAK_SHIFTED_KEYS,
                &COLEMAK_NORMAL_CONTROL_KEYS,
            ),
            KeyboardLayout::ColemakDh => (
                &COLEMAK_DH_NORMAL_KEYS,
                &COLEMAK_DH_SHIFTED_KEYS,
                &COLEMAK_DH_NORMAL_CONTROL_KEYS,
            ),
        };
        normal
            .get(meaning)
            .or_else(|| shifted.get(meaning))
            .or_else(|| control.get(meaning))
            .cloned()
            .unwrap_or_else(|| panic!("Unable to find key binding of {meaning:#?}"))
    }
}

/// Postfix N = Next, Postfix P = Previous
/// X means Swap/Cut
/// Prefix W means Window
#[derive(Hash, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Meaning {
    /// Empty, not assigned
    _____,
    /// GoToIndex
    ToIdx,
    /// Indent
    Indnt,
    /// Paste (End)
    UPstE,
    /// Dedent
    DeDnt,
    /// Open (Prev)
    OpenP,
    /// Open (Next)
    OpenN,
    /// Join
    Join_,
    /// Switch view alignment
    SView,
    /// Select full line
    LineF,
    /// Buffer next
    BuffN,
    /// Buffer previous
    BuffP,
    /// Select Syntax Node
    StyxF,
    /// Raise
    Raise,
    /// File previous
    FileP,
    /// File next
    FileN,
    /// Local find backward
    FindP,
    /// Go back
    GBack,
    /// Go forward
    GForw,
    /// Close current window
    WClse,
    /// Switch window
    WSwth,
    /// Local find forward
    FindN,
    /// Break line
    Break,
    /// Cycle primary selection prev
    CrsrP,
    /// Cycle primary select next
    GrsrN,
    /// Swap cursor with anchor
    XAnchr,
    /// Undo
    Undo_,
    /// Replace with pattern
    PRplc,
    /// Replace (with previous copied text)
    RplcP,
    /// Replace (with next copied text)
    RplcN,
    /// Scroll up
    ScrlU,
    /// Scroll down
    ScrlD,
    /// Select last non-contiguous selection mode
    LstNc,
    /// Redo
    Redo_,
    /// Switch extended selection end
    Exchg,
    /// Copy
    Copy_,
    /// Paste end
    PsteN,
    /// Paste previous
    PsteP,
    /// Replace
    Rplc_,
    /// Replace cut
    RplcX,
    /// Left
    Left_,
    /// Right
    Right,
    /// Delete start
    DeltP,
    /// Delete end
    DeltN,
    /// Search (local) next
    SrchN,
    /// Search (local) previous
    SrchP,
    /// Previous
    Prev_,
    /// Next
    Next_,
    /// Down
    Down_,
    /// Find (global)
    Globl,
    /// First
    First,
    /// Jump
    Jump_,
    /// Last
    Last_,
    /// Transform
    Trsfm,
    /// Mark
    Mark_,
    /// Keep selections matching search
    InstP,
    /// Remove selections matching search
    InstN,
    /// Up
    Up___,

    /// Select Word
    Word_,
    /// V-mode
    VMode,
    /// Change Surround
    Chng_,
    /// Change Cut
    ChngX,
    /// Multi Cursor
    MultC,
    /// Search current selection
    SrchC,
    /// Select Character
    Char_,
    /// Select Line
    Line_,
    /// Select Token
    Token,
    /// Select Syntax
    Sytx_,
    /// Configure Search
    CSrch,
}
