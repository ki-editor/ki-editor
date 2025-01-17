use itertools::Itertools as _;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use strum::IntoEnumIterator as _;
use Meaning::*;

use crate::app::Scope;

pub(crate) const KEYMAP_SCORE: [[char; 10]; 3] = [
    // a = Easiest to access
    // o = Hardest to access
    // Left side (a-o)        Right side (a-o)
    ['m', 'h', 'f', 'i', 'n', /*|*/ 'n', 'i', 'f', 'h', 'm'], // Top row
    ['d', 'b', 'a', 'c', 'e', /*|*/ 'e', 'c', 'a', 'b', 'd'], // Home row
    ['j', 'k', 'l', 'g', 'o', /*|*/ 'o', 'g', 'l', 'k', 'j'], // Bottom row
];

pub(crate) const KEYMAP_NORMAL: [[Meaning; 10]; 3] = [
    [
        SrchN, Word_, SrchC, MultC, Mark_, /****/ FindP, InstP, Up___, InstN, FindN,
    ],
    [
        Line_, Tokn_, Sytx_, VMode, OpenN, /****/ DeltN, Left_, Down_, Right, Jump_,
    ],
    [
        Undo_, Exchg, Copy_, PsteN, Rplc_, /****/ Globl, Chng_, First, Last_, XAchr,
    ],
];

pub(crate) const KEYMAP_NORMAL_SHIFTED: [[Meaning; 10]; 3] = [
    [
        SrchP, WordF, Char_, _____, Trsfm, /****/ CrsrP, RplcP, Join_, RplcN, CrsrN,
    ],
    [
        LineF, ToknF, StyxF, _____, OpenP, /****/ DeltP, DeDnt, Break, Indnt, ToIdx,
    ],
    [
        Redo_, Raise, RplcX, PsteP, PRplc, /****/ _____, ChngX, _____, _____, SSEnd,
    ],
    // Why is Raise placed at the same Position as Exchange?
    // Because Raise is a special-case of Exchange where the movement is Up
];

pub(crate) const KEYMAP_CONTROL: [[Meaning; 10]; 3] = [
    // TODO: Implement Up Line and Down Line
    // The cursor should be placed at the of the line
    [
        CSrch, LineP, LineU, LineN, _____, /****/ _____, _____, ScrlU, _____, _____,
    ],
    [
        KilLP, CItmP, LineD, CItmN, KilLN, /****/ DWrdP, BuffP, ScrlD, BuffN, SView,
    ],
    [
        Undo_, WSwth, WClse, UPstE, _____, /****/ DTknP, GBack, _____, GForw, _____,
    ],
];

/// Why only the left-side is used for Find Local/Global keybindings?
/// This is to enable hand-alteration, as Find Local (Prev/Next) and Find Global
/// are both located on the right-side.
pub(crate) const KEYMAP_FIND_LOCAL: [[Meaning; 10]; 3] = [
    [
        OneCh, PSrch, NtrlN, LNcSM, Mark_, /****/ _____, _____, _____, _____, L2Fnd,
    ],
    [
        DgAll, DgErr, DgWrn, DgHnt, GHnkC, /****/ _____, _____, _____, _____, _____,
    ],
    [
        LImpl, LDefn, LType, LRfrE, Qkfix, /****/ _____, _____, _____, _____, _____,
    ],
];
pub(crate) const KEYMAP_FIND_LOCAL_SHIFTED: [[Meaning; 10]; 3] = [
    [
        _____, _____, _____, _____, _____, /****/ _____, _____, _____, _____, _____,
    ],
    [
        _____, _____, _____, DgInf, GHnkM, /****/ _____, _____, _____, _____, _____,
    ],
    [
        LDecl, _____, _____, LRfrI, _____, /****/ _____, _____, _____, _____, _____,
    ],
];

/// This keymap should be almost identical with that of Find Local
pub(crate) const KEYMAP_FIND_GLOBAL: [[Meaning; 10]; 3] = [
    [
        Srch_, PSrch, SrchC, LNcSM, Mark_, /****/ _____, _____, _____, _____, _____,
    ],
    [
        DgAll, DgErr, DgWrn, DgHnt, GHnkC, /****/ _____, _____, _____, _____, _____,
    ],
    [
        LImpl, LDefn, LType, LRfrE, Qkfix, /****/ CSrch, _____, _____, _____, _____,
    ],
];
pub(crate) type KeyboardMeaningLayout = [[Meaning; 10]; 3];
pub(crate) const KEYMAP_FIND_GLOBAL_SHIFTED: KeyboardMeaningLayout = [
    [
        _____, _____, _____, _____, _____, /****/ _____, _____, _____, _____, _____,
    ],
    [
        _____, _____, _____, DgInf, GHnkM, /****/ _____, _____, _____, _____, _____,
    ],
    [
        _____, LDecl, _____, LRfrI, _____, /****/ _____, _____, _____, _____, _____,
    ],
];

pub(crate) type KeyboardLayout = [[&'static str; 10]; 3];

pub(crate) const QWERTY: KeyboardLayout = [
    ["q", "w", "e", "r", "t", "y", "u", "i", "o", "p"],
    ["a", "s", "d", "f", "g", "h", "j", "k", "l", ";"],
    ["z", "x", "c", "v", "b", "n", "m", ",", ".", "/"],
];

pub(crate) const DVORAK: KeyboardLayout = [
    ["'", ",", ".", "p", "y", "f", "g", "c", "r", "l"],
    ["a", "o", "e", "u", "i", "d", "h", "t", "n", "s"],
    [";", "q", "j", "k", "x", "b", "m", "w", "v", "z"],
];

/// I and U swapped.
/// Refer https://www.reddit.com/r/dvorak/comments/tfz53r/have_anyone_tried_swapping_u_with_i/
pub(crate) const DVORAK_IU: KeyboardLayout = [
    ["'", ",", ".", "p", "y", "f", "g", "c", "r", "l"],
    ["a", "o", "e", "i", "u", "d", "h", "t", "n", "s"],
    [";", "q", "j", "k", "x", "b", "m", "w", "v", "z"],
];

pub(crate) const COLEMAK: KeyboardLayout = [
    ["q", "w", "f", "p", "b", "j", "l", "u", "y", ";"],
    ["a", "r", "s", "t", "g", "m", "n", "e", "i", "o"],
    ["z", "x", "c", "d", "v", "k", "h", ",", ".", "/"],
];

/// Refer https://colemakmods.github.io/mod-dh/
pub(crate) const COLEMAK_DH: KeyboardLayout = [
    ["q", "w", "f", "p", "b", "j", "l", "u", "y", ";"],
    ["a", "r", "s", "t", "g", "m", "n", "e", "i", "o"],
    ["z", "x", "c", "d", "v", "k", "h", ",", ".", "/"],
];

/// Semi-colon and Quote are swapped
/// Refer https://colemakmods.github.io/mod-dh/
pub(crate) const COLEMAK_DH_SEMI_QUOTE: KeyboardLayout = [
    ["q", "w", "f", "p", "b", "j", "l", "u", "y", "'"],
    ["a", "r", "s", "t", "g", "m", "n", "e", "i", "o"],
    ["z", "x", "c", "d", "v", "k", "h", ",", ".", "/"],
];

struct KeySet {
    normal: HashMap<Meaning, &'static str>,
    shifted: HashMap<Meaning, &'static str>,
    normal_control: HashMap<Meaning, &'static str>,
    insert_control: HashMap<Meaning, &'static str>,
    find_local: HashMap<Meaning, &'static str>,
    find_global: HashMap<Meaning, &'static str>,
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
            shifted: HashMap::from_iter(
                KEYMAP_NORMAL_SHIFTED
                    .into_iter()
                    .flatten()
                    .zip(layout.into_iter().flatten().map(shifted)),
            ),
            normal_control: HashMap::from_iter(
                KEYMAP_CONTROL
                    .into_iter()
                    .flatten()
                    .zip(layout.into_iter().flatten().map(alted)),
            ),
            insert_control: HashMap::from_iter(
                KEYMAP_CONTROL
                    .into_iter()
                    .flatten()
                    .zip(layout.into_iter().flatten().map(alted)),
            ),
            find_local: HashMap::from_iter(
                KEYMAP_FIND_LOCAL
                    .into_iter()
                    .flatten()
                    .zip(layout.into_iter().flatten())
                    .chain(
                        KEYMAP_FIND_LOCAL_SHIFTED
                            .into_iter()
                            .flatten()
                            .zip(layout.into_iter().flatten().map(shifted)),
                    ),
            ),
            find_global: HashMap::from_iter(
                KEYMAP_FIND_GLOBAL
                    .into_iter()
                    .flatten()
                    .zip(layout.into_iter().flatten())
                    .chain(
                        KEYMAP_FIND_GLOBAL_SHIFTED
                            .into_iter()
                            .flatten()
                            .zip(layout.into_iter().flatten().map(shifted)),
                    ),
            ),
        }
    }
}

static QWERTY_KEYSET: Lazy<KeySet> = Lazy::new(|| KeySet::from(QWERTY));
static COLEMAK_KEYSET: Lazy<KeySet> = Lazy::new(|| KeySet::from(COLEMAK));
static COLEMAK_DH_KEYSET: Lazy<KeySet> = Lazy::new(|| KeySet::from(COLEMAK_DH));
static COLEMAK_DH_SEMI_QUOTE_KEYSET: Lazy<KeySet> =
    Lazy::new(|| KeySet::from(COLEMAK_DH_SEMI_QUOTE));
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
    ColemakDHSemiQuote,
}

impl KeyboardLayoutKind {
    const fn as_str(&self) -> &'static str {
        match self {
            KeyboardLayoutKind::Qwerty => "QWERTY",
            KeyboardLayoutKind::Dvorak => "DVORAK",
            KeyboardLayoutKind::Colemak => "COLEMAK",
            KeyboardLayoutKind::ColemakDH => "COLEMAK_DH",
            KeyboardLayoutKind::ColemakDHSemiQuote => "COLEMAK_DH_SEMI_QUOTE",
            KeyboardLayoutKind::DvorakIU => "DVORAK_IU",
        }
    }

    pub(crate) fn get_keyboard_layout(&self) -> &KeyboardLayout {
        match self {
            KeyboardLayoutKind::Qwerty => &QWERTY,
            KeyboardLayoutKind::Dvorak => &DVORAK,
            KeyboardLayoutKind::Colemak => &COLEMAK,
            KeyboardLayoutKind::ColemakDH => &COLEMAK_DH,
            KeyboardLayoutKind::ColemakDHSemiQuote => &COLEMAK_DH_SEMI_QUOTE,
            KeyboardLayoutKind::DvorakIU => &DVORAK_IU,
        }
    }

    pub(crate) fn get_key(&self, meaning: &Meaning) -> &'static str {
        let keyset = self.get_keyset();
        keyset
            .normal
            .get(meaning)
            .or_else(|| keyset.shifted.get(meaning))
            .or_else(|| keyset.normal_control.get(meaning))
            .cloned()
            .unwrap_or_else(|| panic!("Unable to find key binding of {meaning:#?}"))
    }

    pub(crate) fn get_insert_key(&self, meaning: &Meaning) -> &'static str {
        let keyset = self.get_keyset();
        keyset
            .insert_control
            .get(meaning)
            .cloned()
            .unwrap_or_else(|| panic!("Unable to find key binding of {meaning:#?}"))
    }

    pub(crate) fn get_find_keymap(&self, scope: Scope, meaning: &Meaning) -> &'static str {
        let keyset = self.get_keyset();
        match scope {
            Scope::Local => keyset
                .find_local
                .get(meaning)
                .cloned()
                .unwrap_or_else(|| panic!("Unable to find key binding of {meaning:#?}")),
            Scope::Global => keyset
                .find_global
                .get(meaning)
                .cloned()
                .unwrap_or_else(|| panic!("Unable to find key binding of {meaning:#?}")),
        }
    }

    fn get_keyset(&self) -> &Lazy<KeySet> {
        match self {
            KeyboardLayoutKind::Qwerty => &QWERTY_KEYSET,
            KeyboardLayoutKind::Dvorak => &DVORAK_KEYSET,
            KeyboardLayoutKind::Colemak => &COLEMAK_KEYSET,
            KeyboardLayoutKind::ColemakDH => &COLEMAK_DH_KEYSET,
            KeyboardLayoutKind::ColemakDHSemiQuote => &COLEMAK_DH_SEMI_QUOTE_KEYSET,
            KeyboardLayoutKind::DvorakIU => &DVORAK_IU_KEYSET,
        }
    }
}

/// Postfix N = Next, Postfix P = Previous
/// X means Swap/Cut
/// Prefix W means Window
#[derive(Hash, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) enum Meaning {
    /// Empty, not assigned
    _____,
    /// Break line
    Break,
    /// Move to next buffer
    BuffN,
    /// Move to previous buffer
    BuffP,
    /// Configure Search
    CSrch,
    /// Select Character
    Char_,
    /// Change Cut
    ChngX,
    /// Change Surround
    Chng_,
    /// Next Completion Item
    CItmN,
    /// Previous Completion Item
    CItmP,
    /// Copy
    Copy_,
    /// Cycle primary select next
    CrsrN,
    /// Cycle primary selection prev
    CrsrP,
    /// Delete token backward
    DTknP,
    /// Delete word backward
    DWrdP,
    /// Dedent
    DeDnt,
    /// Delete end
    DeltN,
    /// Delete start
    DeltP,
    /// Down
    Down_,
    /// Switch extended selection end
    Exchg,
    /// Local find forward
    FindN,
    /// Local find backward
    FindP,
    /// First
    First,
    /// Go back
    GBack,
    /// Go forward
    GForw,
    /// Find (global)
    Globl,
    /// Indent
    Indnt,
    /// Remove selections matching search
    InstN,
    /// Keep selections matching search
    InstP,
    /// Join
    Join_,
    /// Jump
    Jump_,
    /// Kill to line end
    KilLN,
    /// Kill to line start
    KilLP,
    /// Last
    Last_,
    /// Left
    Left_,
    /// Line Up
    LineU,
    /// Line Down
    LineD,
    /// Select full line
    LineF,
    /// Move to line end
    LineN,
    /// Move to line start
    LineP,
    /// Select Line
    Line_,
    /// Mark
    Mark_,
    /// Multi Cursor
    MultC,
    /// Open (Next)
    OpenN,
    /// Open (Prev)
    OpenP,
    /// Replace with pattern
    PRplc,
    /// Paste end
    PsteN,
    /// Paste previous
    PsteP,
    /// Raise
    Raise,
    /// Redo
    Redo_,
    /// Right
    Right,
    /// Replace (with next copied text)
    RplcN,
    /// Replace (with previous copied text)
    RplcP,
    /// Replace cut
    RplcX,
    /// Replace
    Rplc_,
    /// Switch view alignment
    SView,
    /// Scroll down
    ScrlD,
    /// Scroll up
    ScrlU,
    /// Search current selection
    SrchC,
    /// Search (local) next
    SrchN,
    /// Search (local) previous
    SrchP,
    /// Select Syntax Node
    StyxF,
    /// Select Syntax
    Sytx_,
    /// GoToIndex
    ToIdx,
    /// Select Token
    Tokn_,
    /// Select Token Fine
    ToknF,
    /// Transform
    Trsfm,
    /// Paste (End)
    UPstE,
    /// Undo
    Undo_,
    /// Up
    Up___,
    /// V-mode
    VMode,
    /// Close current window
    WClse,
    /// Switch window
    WSwth,
    /// Select Word
    Word_,
    /// Select Word Fine
    WordF,
    /// Swap cursor with anchor
    XAchr,
    /// Swap Selection End
    SSEnd,
    /// Search (directionless)
    Srch_,
    /// Search (using previous search)
    PSrch,
    /// Last non-contiguous selection mode
    LNcSM,
    /// Quickfix
    Qkfix,
    /// Git Hunk (against current branch)
    GHnkC,
    /// Git Hunk (against main branch)
    GHnkM,
    /// Diagnostic All
    DgAll,
    /// Diagnostic Error
    DgErr,
    /// Diagnostic Hint
    DgHnt,
    /// Diagnostic Warning
    DgWrn,
    /// Diagonstic Info
    DgInf,
    /// LSP Definitions
    LDefn,
    /// LSP Declarations
    LDecl,
    /// Lsp Implementations
    LImpl,
    /// Lsp References (exclude declaration)
    LRfrE,
    /// Lsp Referencs (include declaration)
    LRfrI,
    /// Lsp Type Definition
    LType,
    /// Natural Number
    NtrlN,
    /// One Character
    OneCh,
    /// Level 2 Find
    L2Fnd,
}
pub(crate) fn shifted(c: &'static str) -> &'static str {
    match c {
        "." => ">",
        "," => "<",
        "/" => "?",
        ";" => ":",
        "'" => "\"",
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

pub(crate) fn shifted_char(c: char) -> char {
    match c {
        '.' => '>',
        ',' => '<',
        '/' => '?',
        ';' => ':',
        '\'' => '\'',
        '[' => '{',
        ']' => '}',
        '1' => '!',
        '2' => '@',
        '3' => '#',
        '4' => '$',
        '5' => '%',
        '6' => '^',
        '7' => '&',
        '8' => '*',
        '9' => '(',
        '0' => ')',
        '-' => '_',
        '=' => '+',
        'a' => 'A',
        'b' => 'B',
        'c' => 'C',
        'd' => 'D',
        'e' => 'E',
        'f' => 'F',
        'g' => 'G',
        'h' => 'H',
        'i' => 'I',
        'j' => 'J',
        'k' => 'K',
        'l' => 'L',
        'm' => 'M',
        'n' => 'N',
        'o' => 'O',
        'p' => 'P',
        'q' => 'Q',
        'r' => 'R',
        's' => 'S',
        't' => 'T',
        'u' => 'U',
        'v' => 'V',
        'w' => 'W',
        'x' => 'X',
        'y' => 'Y',
        'z' => 'Z',
        // Uppercase letters remain unchanged when shifted
        'A' => 'A',
        'B' => 'B',
        'C' => 'C',
        'D' => 'D',
        'E' => 'E',
        'F' => 'F',
        'G' => 'G',
        'H' => 'H',
        'I' => 'I',
        'J' => 'J',
        'K' => 'K',
        'L' => 'L',
        'M' => 'M',
        'N' => 'N',
        'O' => 'O',
        'P' => 'P',
        'Q' => 'Q',
        'R' => 'R',
        'S' => 'S',
        'T' => 'T',
        'U' => 'U',
        'V' => 'V',
        'W' => 'W',
        'X' => 'X',
        'Y' => 'Y',
        'Z' => 'Z',
        c => c, // return unchanged if no shift mapping exists
    }
}

pub(crate) fn alted(c: &'static str) -> &'static str {
    match c {
        "." => "alt+.",
        "," => "alt+,",
        "/" => "alt+/",
        ";" => "alt+;",
        "\"" => "alt+\"",
        "'" => "alt+'",
        "[" => "alt+[",
        "]" => "alt+]",
        "1" => "alt+1",
        "2" => "alt+2",
        "3" => "alt+3",
        "4" => "alt+4",
        "5" => "alt+5",
        "6" => "alt+6",
        "7" => "alt+7",
        "8" => "alt+8",
        "9" => "alt+9",
        "0" => "alt+0",
        "-" => "alt+-",
        "=" => "alt+=",
        "a" => "alt+a",
        "b" => "alt+b",
        "c" => "alt+c",
        "d" => "alt+d",
        "e" => "alt+e",
        "f" => "alt+f",
        "g" => "alt+g",
        "h" => "alt+h",
        "i" => "alt+i",
        "j" => "alt+j",
        "k" => "alt+k",
        "l" => "alt+l",
        "m" => "alt+m",
        "n" => "alt+n",
        "o" => "alt+o",
        "p" => "alt+p",
        "q" => "alt+q",
        "r" => "alt+r",
        "s" => "alt+s",
        "t" => "alt+t",
        "u" => "alt+u",
        "v" => "alt+v",
        "w" => "alt+w",
        "x" => "alt+x",
        "y" => "alt+y",
        "z" => "alt+z",
        c => c, // return unchanged if no shift mapping exists
    }
}
