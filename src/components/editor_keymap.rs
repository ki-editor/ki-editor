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

use std::collections::HashMap;

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
    /// GoToIndex
    TOIDX,
    /// Indent
    INDNT,
    /// Paste (End)
    UPSTE,
    /// Dedent
    DEDNT,
    /// Open (Prev)
    OPENP,
    /// Open (Next)
    OPENN,
    /// Join
    JOIN_,
    /// Switch view alignment
    SVIEW,
    /// Select full line
    LINEF,
    /// Buffer next
    BUFFN,
    /// Buffer previous
    BUFFP,
    /// Select Syntax Node
    SYTXF,
    /// Raise
    RAISE,
    /// File previous
    FILEP,
    /// File next
    FILEN,
    /// Local find backward
    FINDP,
    /// Go back
    GBACK,
    /// Go forward
    GFORW,
    /// Close current window
    WCLSE,
    /// Switch window
    WSWTH,
    /// Local find forward
    FINDN,
    /// Break line
    BREAK,
    /// Cycle primary selection prev
    CRSRP,
    /// Cycle primary select next
    CRSRN,
    /// Swap cursor with anchor
    XACHR,
    /// Undo
    UNDO_,
    /// Replace with pattern
    PRPLC,
    /// Replace (with previous copied text)
    RPLCP,
    /// Replace (with next copied text)
    RPLCN,
    /// Scroll up
    SCRLU,
    /// Scroll down
    SCRLD,
    /// Select last non-contiguous selection mode
    LSTNC,
    /// Redo
    REDO_,
    /// Switch extended selection end
    EXCHG,
    /// Copy
    COPY_,
    /// Paste end
    PSTEN,
    /// Paste previous
    PSTEP,
    /// Replace
    RPLC_,
    /// Replace cut
    RPLCX,
    /// Left
    LEFT_,
    /// Right
    RIGHT,
    /// Delete start
    DELTP,
    /// Delete end
    DELTN,
    /// Search (local) next
    SRCHN,
    /// Search (local) previous
    SRCHP,
    /// Previous
    PREV_,
    /// Next
    NEXT_,
    /// Down
    DOWN_,
    /// Find (global)
    GLOBL,
    /// First
    FIRST,
    /// Jump
    JUMP_,
    /// Last
    LAST_,
    /// Transform
    TRSFM,
    /// Mark
    MARK_,
    /// Keep selections matching search
    INSTP,
    /// Remove selections matching search
    INSTN,
    /// Up
    UP___,

    /// Select Word
    WORD_,
    /// V-mode
    VMODE,
    /// Change Surround
    CHNG_,
    /// Change Cut
    CHNGX,
    /// Multi Cursor
    MULTC,
    /// Search current selection
    SRCHC,
    /// Select Character
    CHAR_,
    /// Select Line
    LINE_,
    /// Select Token
    TOKEN,
    /// Select Syntax
    SYTX_,
    /// Configure Search
    CSRCH,
}
