use itertools::Itertools as _;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use strum::IntoEnumIterator as _;
use Meaning::*;

pub const KEYMAP_NORMAL: [[Meaning; 10]; 3] = [
    [
        Word_, Token, SrchC, MultC, OpenN, /****/ Mark_, InstP, Up___, InstN, CSrch,
    ],
    [
        Line_, Sytx_, DeltN, Chng_, VMode, /****/ Prev_, Left_, Down_, Right, Next_,
    ],
    [
        Undo_, Exchg, Copy_, PsteN, Rplc_, /****/ Globl, First, Jump_, Last_, SrchN,
    ],
];

pub const KEYMAP_NORMAL_SHIFTED: [[Meaning; 10]; 3] = [
    [
        Char_, _____, Raise, _____, OpenP, /****/ Indnt, CrsrP, Join_, CrsrN, DeDnt,
    ],
    [
        LineF, StyxF, DeltP, ChngX, LstNc, /****/ BuffP, FindP, Break, FindN, BuffN,
    ],
    [
        Redo_, XAchr, _____, PsteP, RplcX, /****/ Trsfm, GBack, ToIdx, GForw, SrchP,
    ],
];

pub const KEYMAP_NORMAL_CONTROL: [[Meaning; 10]; 3] = [
    [
        _____, _____, _____, _____, _____, /****/ _____, RplcP, ScrlU, RplcN, _____,
    ],
    [
        _____, _____, WClse, _____, _____, /****/ _____, _____, ScrlD, SView, _____,
    ],
    [
        Undo_, WSwth, _____, UPstE, PRplc, /****/ _____, _____, _____, _____, _____,
    ],
];

type KeyboardLayout = [[&'static str; 10]; 3];

pub const QWERTY: KeyboardLayout = [
    ["q", "w", "e", "r", "t", "y", "u", "i", "o", "p"],
    ["a", "s", "d", "f", "g", "h", "j", "k", "l", ";"],
    ["z", "x", "c", "v", "b", "n", "m", ",", ".", "/"],
];

pub const DVORAK: KeyboardLayout = [
    ["'", ",", ".", "p", "y", "f", "g", "c", "r", "l"],
    ["a", "o", "e", "u", "i", "d", "h", "t", "n", "s"],
    [";", "q", "j", "k", "x", "b", "m", "w", "v", "z"],
];

/// I and U swapped.
/// Refer https://www.reddit.com/r/dvorak/comments/tfz53r/have_anyone_tried_swapping_u_with_i/
pub const DVORAK_IU: KeyboardLayout = [
    ["'", ",", ".", "p", "y", "f", "g", "c", "r", "l"],
    ["a", "o", "e", "i", "u", "d", "h", "t", "n", "s"],
    [";", "q", "j", "k", "x", "b", "m", "w", "v", "z"],
];

pub const COLEMAK: KeyboardLayout = [
    ["q", "w", "f", "p", "b", "j", "l", "u", "y", ";"],
    ["a", "r", "s", "t", "g", "m", "n", "e", "i", "o"],
    ["z", "x", "c", "d", "v", "k", "h", ",", ".", "/"],
];

/// Refer https://colemakmods.github.io/mod-dh/
pub const COLEMAK_DH: KeyboardLayout = [
    ["q", "w", "f", "p", "b", "j", "l", "u", "y", ";"],
    ["a", "r", "s", "t", "g", "m", "n", "e", "i", "o"],
    ["z", "x", "c", "d", "v", "k", "h", ",", ".", "/"],
];

struct KeySet {
    normal: HashMap<Meaning, &'static str>,
    normal_shifted: HashMap<Meaning, &'static str>,
    normal_control: HashMap<Meaning, &'static str>,
}

impl KeySet {
    fn from(layout: KeyboardLayout) -> Self {
        Self {
            normal: HashMap::from_iter(
                KEYMAP_NORMAL
                    .into_iter()
                    .flatten()
                    .zip(layout.into_iter().flatten()),
            ),
            normal_shifted: HashMap::from_iter(
                KEYMAP_NORMAL_SHIFTED
                    .into_iter()
                    .flatten()
                    .zip(layout.into_iter().flatten().map(shifted)),
            ),
            normal_control: HashMap::from_iter(
                KEYMAP_NORMAL_CONTROL
                    .into_iter()
                    .flatten()
                    .zip(layout.into_iter().flatten().map(controlled)),
            ),
        }
    }
}

static QWERTY_KEYSET: Lazy<KeySet> = Lazy::new(|| KeySet::from(QWERTY));
static COLEMAK_KEYSET: Lazy<KeySet> = Lazy::new(|| KeySet::from(COLEMAK));
static COLEMAK_DH_KEYSET: Lazy<KeySet> = Lazy::new(|| KeySet::from(COLEMAK_DH));
static DVORAK_KEYSET: Lazy<KeySet> = Lazy::new(|| KeySet::from(DVORAK));
static DVORAK_IU_KEYSET: Lazy<KeySet> = Lazy::new(|| KeySet::from(DVORAK_IU));

pub(crate) static KEYBOARD_LAYOUT: Lazy<KeyboardLayoutKind> = Lazy::new(|| {
    use KeyboardLayoutKind::*;
    crate::env::parse_env(
        "KI_EDITOR_KEYBOARD",
        &KeyboardLayoutKind::iter().collect_vec(),
        |layout| layout.as_str(),
        Qwerty,
    )
});

#[derive(Debug, Clone, strum_macros::EnumIter)]
pub(crate) enum KeyboardLayoutKind {
    Qwerty,
    Dvorak,
    DvorakIU,
    Colemak,
    ColemakDH,
}

impl KeyboardLayoutKind {
    const fn as_str(&self) -> &'static str {
        match self {
            KeyboardLayoutKind::Qwerty => "QWERTY",
            KeyboardLayoutKind::Dvorak => "DVORAK",
            KeyboardLayoutKind::Colemak => "COLEMAK",
            KeyboardLayoutKind::ColemakDH => "COLEMAK_DH",
            KeyboardLayoutKind::DvorakIU => "DVORAK_IU",
        }
    }
    pub(crate) fn get_key(&self, meaning: &Meaning) -> &'static str {
        let keyset = match self {
            KeyboardLayoutKind::Qwerty => &QWERTY_KEYSET,
            KeyboardLayoutKind::Dvorak => &DVORAK_KEYSET,
            KeyboardLayoutKind::Colemak => &COLEMAK_KEYSET,
            KeyboardLayoutKind::ColemakDH => &COLEMAK_DH_KEYSET,
            KeyboardLayoutKind::DvorakIU => &DVORAK_IU_KEYSET,
        };
        keyset
            .normal
            .get(meaning)
            .or_else(|| keyset.normal_shifted.get(meaning))
            .or_else(|| keyset.normal_control.get(meaning))
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
    CrsrN,
    /// Swap cursor with anchor
    XAchr,
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

fn shifted(c: &'static str) -> &'static str {
    match c {
        "." => ">",
        "," => "<",
        "/" => "?",
        ";" => ":",
        "\"" => "\"",
        "[" => "{",
        "]" => "}",
        "1" => "!",
        "2" => "@",
        "3" => "#",
        "4" => "$",
        "5" => "%",
        "6" => "^",
        "7" => "&",
        "8" => "*",
        "9" => "(",
        "0" => ")",
        "-" => "_",
        "=" => "+",
        "a" => "A",
        "b" => "B",
        "c" => "C",
        "d" => "D",
        "e" => "E",
        "f" => "F",
        "g" => "G",
        "h" => "H",
        "i" => "I",
        "j" => "J",
        "k" => "K",
        "l" => "L",
        "m" => "M",
        "n" => "N",
        "o" => "O",
        "p" => "P",
        "q" => "Q",
        "r" => "R",
        "s" => "S",
        "t" => "T",
        "u" => "U",
        "v" => "V",
        "w" => "W",
        "x" => "X",
        "y" => "Y",
        "z" => "Z",
        // Uppercase letters remain unchanged when shifted
        "A" => "A",
        "B" => "B",
        "C" => "C",
        "D" => "D",
        "E" => "E",
        "F" => "F",
        "G" => "G",
        "H" => "H",
        "I" => "I",
        "J" => "J",
        "K" => "K",
        "L" => "L",
        "M" => "M",
        "N" => "N",
        "O" => "O",
        "P" => "P",
        "Q" => "Q",
        "R" => "R",
        "S" => "S",
        "T" => "T",
        "U" => "U",
        "V" => "V",
        "W" => "W",
        "X" => "X",
        "Y" => "Y",
        "Z" => "Z",
        c => c, // return unchanged if no shift mapping exists
    }
}

fn controlled(c: &'static str) -> &'static str {
    match c {
        "." => "ctrl+.",
        "," => "ctrl+,",
        "/" => "ctrl+/",
        ";" => "ctrl+;",
        "\"" => "ctrl+\"",
        "[" => "ctrl+[",
        "]" => "ctrl+]",
        "1" => "ctrl+1",
        "2" => "ctrl+2",
        "3" => "ctrl+3",
        "4" => "ctrl+4",
        "5" => "ctrl+5",
        "6" => "ctrl+6",
        "7" => "ctrl+7",
        "8" => "ctrl+8",
        "9" => "ctrl+9",
        "0" => "ctrl+0",
        "-" => "ctrl+-",
        "=" => "ctrl+=",
        "a" => "ctrl+a",
        "b" => "ctrl+b",
        "c" => "ctrl+c",
        "d" => "ctrl+d",
        "e" => "ctrl+e",
        "f" => "ctrl+f",
        "g" => "ctrl+g",
        "h" => "ctrl+h",
        "i" => "tab", // Lookup ASCII Control Character 9, where ctrl+i means tab
        "j" => "ctrl+j",
        "k" => "ctrl+k",
        "l" => "ctrl+l",
        "m" => "ctrl+m",
        "n" => "ctrl+n",
        "o" => "ctrl+o",
        "p" => "ctrl+p",
        "q" => "ctrl+q",
        "r" => "ctrl+r",
        "s" => "ctrl+s",
        "t" => "ctrl+t",
        "u" => "ctrl+u",
        "v" => "ctrl+v",
        "w" => "ctrl+w",
        "x" => "ctrl+x",
        "y" => "ctrl+y",
        "z" => "ctrl+z",
        c => c, // return unchanged if no shift mapping exists
    }
}
