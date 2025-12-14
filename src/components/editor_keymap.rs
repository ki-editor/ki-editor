use once_cell::sync::Lazy;
use std::collections::HashMap;

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
        SrchL, SWord, SchCS, MultC, Swap_, /****/ First, Prev_, Up___, Next_, Last_,
    ],
    [
        Line_, Word_, Sytx_, Chng_, Extnd, /****/ InstP, Left_, Down_, Right, InstN,
    ],
    [
        Undo_, Rplc_, Copy_, Delte, Mark_, /****/ LSrch, Jump_, Open_, Paste, XAchr,
    ],
];

pub(crate) const KEYMAP_NORMAL_SHIFTED: [[Meaning; 10]; 3] = [
    [
        SchWC, Char_, SchCC, _____, Raise, /****/ _____, RplcP, Join_, RplcN, _____,
    ],
    [
        LineF, _____, FStyx, ChngX, Trsfm, /****/ CrsrP, DeDnt, Break, Indnt, CrsrN,
    ],
    [
        Redo_, PRplc, RplcX, Del0G, _____, /****/ GSrch, ToIdx, _____, Pst0G, SSEnd,
    ],
    // Why is Raise placed at the same Position as Swap?
    // Because Raise is a special-case of Swap where the movement is Up
];

/// Meta also means Alt (Windows) or Option (Mac).
pub(crate) const KEYMAP_META: [[Meaning; 10]; 3] = [
    [
        KilLP, _____, LineU, _____, KilLN, /****/ NBack, _____, ScrlU, _____, NForw,
    ],
    [
        _____, LineP, LineD, LineN, OpenM, /****/ DWrdP, MrkFP, ScrlD, MrkFN, SView,
    ],
    [
        Undo_, _____, _____, WClse, MarkF, /****/ _____, _____, _____, _____, WSwth,
    ],
];

/// Why only the left-side is used for Find Local/Global keybindings?
/// This is to enable hand-alteration, as Find Local (Prev/Next) and Find Global
/// are both located on the right-side.
pub(crate) const KEYMAP_FIND_LOCAL: [[Meaning; 10]; 3] = [
    [
        OneCh, _____, NtrlN, RSrch, Qkfix, /****/ _____, _____, _____, _____, _____,
    ],
    [
        DgAll, DgErr, DgWrn, DgHnt, GHnkC, /****/ _____, _____, _____, _____, _____,
    ],
    [
        LImpl, LDefn, LType, LRfrE, Mark_, /****/ LRept, _____, _____, _____, _____,
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
        _____, LDecl, _____, LRfrI, _____, /****/ _____, _____, _____, _____, _____,
    ],
];

/// This keymap should be almost identical with that of Find Local
pub(crate) const KEYMAP_FIND_GLOBAL: [[Meaning; 10]; 3] = [
    [
        Srch_, _____, SchCS, RSrch, Qkfix, /****/ _____, _____, _____, _____, _____,
    ],
    [
        DgAll, DgErr, DgWrn, DgHnt, GHnkC, /****/ _____, _____, _____, _____, _____,
    ],
    [
        LImpl, LDefn, LType, LRfrE, Mark_, /****/ GRept, _____, _____, _____, _____,
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

pub(crate) const KEYMAP_SURROUND: KeyboardMeaningLayout = [
    [
        _____, _____, _____, _____, _____, /****/ _____, SQuot, DQuot, BckTk, XML__,
    ],
    [
        _____, _____, _____, _____, _____, /****/ _____, Paren, Brckt, Brace, Anglr,
    ],
    [
        _____, _____, _____, _____, _____, /****/ _____, _____, _____, _____, _____,
    ],
];

pub(crate) const KEYMAP_SPACE: KeyboardMeaningLayout = [
    [
        _____, _____, _____, _____, _____, /****/ _____, RevlS, RevlC, RevlM, _____,
    ],
    [
        _____, _____, _____, _____, _____, /****/ _____, SpEdt, SpPck, SpCtx, Explr,
    ],
    [
        _____, _____, _____, _____, _____, /****/ _____, _____, _____, _____, SHelp,
    ],
];

pub(crate) const KEYMAP_SPACE_EDITOR: KeyboardMeaningLayout = [
    [
        _____, _____, _____, _____, _____, /****/ _____, _____, _____, _____, _____,
    ],
    [
        _____, _____, RlBfr, Pipe_, _____, /****/ _____, _____, _____, _____, _____,
    ],
    [
        _____, RplcA, SaveA, QNSav, _____, /****/ _____, _____, _____, _____, _____,
    ],
];

pub(crate) const KEYMAP_SPACE_CONTEXT: KeyboardMeaningLayout = [
    [
        _____, _____, _____, _____, TSNSx, /****/ _____, _____, _____, _____, _____,
    ],
    [
        _____, LHovr, LCdAc, LRnme, RvHkC, /****/ _____, _____, _____, _____, _____,
    ],
    [
        _____, GoFil, _____, _____, GtBlm, /****/ _____, _____, _____, _____, _____,
    ],
];

pub(crate) const KEYMAP_SPACE_CONTEXT_SHIFTED: KeyboardMeaningLayout = [
    [
        _____, _____, _____, _____, _____, /****/ _____, _____, _____, _____, _____,
    ],
    [
        _____, _____, _____, _____, RvHkM, /****/ _____, _____, _____, _____, _____,
    ],
    [
        _____, _____, _____, _____, _____, /****/ _____, _____, _____, _____, _____,
    ],
];

pub(crate) const KEYMAP_SPACE_PICKER: KeyboardMeaningLayout = [
    [
        _____, _____, _____, _____, _____, /****/ _____, _____, _____, _____, _____,
    ],
    [
        Theme, SyblD, File_, Buffr, GitFC, /****/ _____, _____, _____, _____, _____,
    ],
    [
        _____, _____, _____, _____, _____, /****/ _____, _____, _____, _____, _____,
    ],
];

pub(crate) const KEYMAP_SPACE_PICKER_SHIFTED: KeyboardMeaningLayout = [
    [
        _____, _____, _____, _____, _____, /****/ _____, _____, _____, _____, _____,
    ],
    [
        _____, SyblW, _____, _____, GitFM, /****/ _____, _____, _____, _____, _____,
    ],
    [
        _____, _____, _____, _____, _____, /****/ _____, _____, _____, _____, _____,
    ],
];

pub(crate) const KEYMAP_TRANSFORM: KeyboardMeaningLayout = [
    [
        Upper, USnke, Pscal, UKbab, Title, /****/ _____, _____, _____, _____, _____,
    ],
    [
        Lower, Snke_, Camel, Kbab_, _____, /****/ _____, Wrap_, CmtLn, CmtBk, _____,
    ],
    [
        _____, _____, _____, _____, _____, /****/ _____, _____, _____, _____, _____,
    ],
];

pub(crate) const KEYMAP_YES_NO: KeyboardMeaningLayout = [
    [
        _____, _____, _____, _____, _____, /****/ _____, _____, _____, _____, _____,
    ],
    [
        _____, _____, Yes__, _____, _____, /****/ _____, _____, No___, _____, _____,
    ],
    [
        _____, _____, _____, _____, _____, /****/ _____, _____, _____, _____, _____,
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

/// Refer https://workmanlayout.org/
pub(crate) const WORKMAN: KeyboardLayout = [
    ["q", "d", "r", "w", "b", "j", "f", "u", "p", ";"],
    ["a", "s", "h", "t", "g", "y", "n", "e", "o", "i"],
    ["z", "x", "m", "c", "v", "k", "l", ",", ".", "/"],
];

/// Refer http://adnw.de/index.php?n=Main.OptimierungF%c3%bcrDieGeradeTastaturMitDaumen-Shift
pub(crate) const PUQ: KeyboardLayout = [
    ["p", "u", ":", ",", "q", "g", "c", "l", "m", "f"],
    ["h", "i", "e", "a", "o", "d", "t", "r", "n", "s"],
    ["k", "y", ".", "'", "x", "j", "v", "w", "b", "z"],
];

struct KeySet {
    normal: HashMap<Meaning, &'static str>,
    shifted: HashMap<Meaning, &'static str>,
    normal_control: HashMap<Meaning, &'static str>,
    insert_control: HashMap<Meaning, &'static str>,
    find_local: HashMap<Meaning, &'static str>,
    find_global: HashMap<Meaning, &'static str>,
    surround: HashMap<Meaning, &'static str>,
    space: HashMap<Meaning, &'static str>,
    space_context: HashMap<Meaning, &'static str>,
    space_picker: HashMap<Meaning, &'static str>,
    space_editor: HashMap<Meaning, &'static str>,
    transform: HashMap<Meaning, &'static str>,
    yes_no: HashMap<Meaning, &'static str>,
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
                KEYMAP_META
                    .into_iter()
                    .flatten()
                    .zip(layout.into_iter().flatten().map(alted)),
            ),
            insert_control: HashMap::from_iter(
                KEYMAP_META
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
            surround: HashMap::from_iter(
                KEYMAP_SURROUND
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
            space: HashMap::from_iter(
                KEYMAP_SPACE
                    .into_iter()
                    .flatten()
                    .zip(layout.into_iter().flatten()),
            ),
            transform: HashMap::from_iter(
                KEYMAP_TRANSFORM
                    .into_iter()
                    .flatten()
                    .zip(layout.into_iter().flatten()),
            ),
            yes_no: HashMap::from_iter(
                KEYMAP_YES_NO
                    .into_iter()
                    .flatten()
                    .zip(layout.into_iter().flatten()),
            ),
            space_context: HashMap::from_iter(
                KEYMAP_SPACE_CONTEXT
                    .into_iter()
                    .flatten()
                    .zip(layout.into_iter().flatten())
                    .chain(
                        KEYMAP_SPACE_CONTEXT_SHIFTED
                            .into_iter()
                            .flatten()
                            .zip(layout.into_iter().flatten().map(shifted)),
                    ),
            ),
            space_picker: HashMap::from_iter(
                KEYMAP_SPACE_PICKER
                    .into_iter()
                    .flatten()
                    .zip(layout.into_iter().flatten())
                    .chain(
                        KEYMAP_SPACE_PICKER_SHIFTED
                            .into_iter()
                            .flatten()
                            .zip(layout.into_iter().flatten().map(shifted)),
                    ),
            ),
            space_editor: HashMap::from_iter(
                KEYMAP_SPACE_EDITOR
                    .into_iter()
                    .flatten()
                    .zip(layout.into_iter().flatten()),
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
static WORKMAN_KEYSET: Lazy<KeySet> = Lazy::new(|| KeySet::from(WORKMAN));
static PUQ_KEYSET: Lazy<KeySet> = Lazy::new(|| KeySet::from(PUQ));

#[derive(Debug, Clone, strum_macros::EnumIter, PartialEq, Eq)]
pub(crate) enum KeyboardLayoutKind {
    Qwerty,
    Dvorak,
    DvorakIU,
    Colemak,
    ColemakDH,
    ColemakDHSemiQuote,
    Workman,
    Puq,
}

impl KeyboardLayoutKind {
    pub(crate) const fn display(&self) -> &'static str {
        match self {
            KeyboardLayoutKind::Qwerty => "QWERTY",
            KeyboardLayoutKind::Dvorak => "DVORAK",
            KeyboardLayoutKind::Colemak => "COLEMAK",
            KeyboardLayoutKind::ColemakDH => "COLEMAK-DH",
            KeyboardLayoutKind::ColemakDHSemiQuote => "COLEMAK-DH;",
            KeyboardLayoutKind::DvorakIU => "DVORAK-IU",
            KeyboardLayoutKind::Workman => "WORKMAN",
            KeyboardLayoutKind::Puq => "PUQ",
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
            KeyboardLayoutKind::Workman => &WORKMAN,
            KeyboardLayoutKind::Puq => &PUQ,
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

    pub(crate) fn get_space_keymap(&self, meaning: &Meaning) -> &'static str {
        let keyset = self.get_keyset();
        keyset
            .space
            .get(meaning)
            .cloned()
            .unwrap_or_else(|| panic!("Unable to find key binding of {meaning:#?}"))
    }

    pub(crate) fn get_space_context_keymap(&self, meaning: &Meaning) -> &'static str {
        let keyset = self.get_keyset();
        keyset
            .space_context
            .get(meaning)
            .cloned()
            .unwrap_or_else(|| panic!("Unable to find key binding of {meaning:#?}"))
    }

    pub(crate) fn get_space_editor_keymap(&self, meaning: &Meaning) -> &'static str {
        let keyset = self.get_keyset();
        keyset
            .space_editor
            .get(meaning)
            .cloned()
            .unwrap_or_else(|| panic!("Unable to find key binding of {meaning:#?}"))
    }

    pub(crate) fn get_space_picker_keymap(&self, meaning: &Meaning) -> &'static str {
        let keyset = self.get_keyset();
        keyset
            .space_picker
            .get(meaning)
            .cloned()
            .unwrap_or_else(|| panic!("Unable to find key binding of {meaning:#?}"))
    }

    pub(crate) fn get_surround_keymap(&self, meaning: &Meaning) -> &'static str {
        let keyset = self.get_keyset();
        keyset
            .surround
            .get(meaning)
            .cloned()
            .unwrap_or_else(|| panic!("Unable to find key binding of {meaning:#?}"))
    }

    pub(crate) fn get_transform_key(&self, meaning: &Meaning) -> &'static str {
        let keyset = self.get_keyset();
        keyset
            .transform
            .get(meaning)
            .cloned()
            .unwrap_or_else(|| panic!("Unable to find key binding of {meaning:#?}"))
    }

    pub(crate) fn get_yes_no_key(&self, meaning: &Meaning) -> &'static str {
        let keyset = self.get_keyset();
        keyset
            .yes_no
            .get(meaning)
            .cloned()
            .unwrap_or_else(|| panic!("Unable to find key binding of {meaning:#?}"))
    }

    fn get_keyset(&self) -> &Lazy<KeySet> {
        match self {
            KeyboardLayoutKind::Qwerty => &QWERTY_KEYSET,
            KeyboardLayoutKind::Dvorak => &DVORAK_KEYSET,
            KeyboardLayoutKind::Colemak => &COLEMAK_KEYSET,
            KeyboardLayoutKind::ColemakDH => &COLEMAK_DH_KEYSET,
            KeyboardLayoutKind::ColemakDHSemiQuote => &COLEMAK_DH_SEMI_QUOTE_KEYSET,
            KeyboardLayoutKind::DvorakIU => &DVORAK_IU_KEYSET,
            KeyboardLayoutKind::Workman => &WORKMAN_KEYSET,
            KeyboardLayoutKind::Puq => &PUQ_KEYSET,
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
    /// Move to next marked file
    MrkFN,
    /// Move to previous marked file
    MrkFP,
    /// Select Character
    Char_,
    /// Change Cut
    ChngX,
    /// Change Surround
    Chng_,
    /// Copy
    Copy_,
    /// Cycle primary select next
    CrsrN,
    /// Cycle primary selection prev
    CrsrP,
    /// Delete word backward
    DWrdP,
    /// Dedent
    DeDnt,
    /// Delete
    Delte,
    /// Delete No Gap
    Del0G,
    /// Down
    Down_,
    /// Swap
    Swap_,
    /// First
    First,
    /// Navigate back (faster alternative of Go Back, skips contiguous navigation, works across files)
    NBack,
    /// Navigate forward
    NForw,
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
    /// Mark/Unmark Selection
    Mark_,
    /// Mark/Unmark File
    MarkF,
    /// Multi Cursor
    MultC,
    /// Open
    Open_,
    /// Replace with pattern
    PRplc,
    /// Paste
    Paste,
    /// Paste No Gap
    Pst0G,
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
    SchCS,
    /// Search current clipboard content
    SchCC,
    /// Search (local)
    SrchL,
    /// Select Fine Syntax Node
    FStyx,
    /// Select Syntax Node
    Sytx_,
    /// GoToIndex
    ToIdx,
    /// Select Word
    Word_,
    /// Transform
    Trsfm,
    /// Undo
    Undo_,
    /// Up
    Up___,
    /// V-mode
    Extnd,
    /// Close current window
    WClse,
    /// Switch window
    WSwth,
    /// Select Subword
    SWord,
    /// Swap cursor with anchor
    XAchr,
    /// Swap Selection End
    SSEnd,
    /// Search (directionless)
    Srch_,
    /// Repeat Secondary Selection Mode (Local)
    LRept,
    /// Repeat Secondary Selection Mode (Global)
    GRept,
    /// Repeat Search
    RSrch,
    /// Find (Local)
    LSrch,
    /// Find (Global)
    GSrch,
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
    /// Parenthesis
    Paren,
    /// Curly Braces
    Brace,
    /// Square Brackets
    Brckt,
    /// Angular Bracket
    Anglr,
    /// Single quote
    SQuot,
    /// Double quote
    DQuot,
    /// Backtick
    BckTk,
    /// Show Help
    SHelp,
    /// Quit No Save
    QNSav,
    /// Save All
    SaveA,
    /// File Explorer
    Explr,
    /// LSP Rename
    LRnme,
    /// Pick Theme
    Theme,
    /// Pick Symbol (Document)
    SyblD,
    /// Pick Symbol (Workspace)
    SyblW,
    /// Pick File
    File_,
    /// Pick Git Status File (against current branch)
    GitFC,
    /// Pick Git Status File (against main branch)
    GitFM,
    /// LSP Hover
    LHovr,
    /// TS Node Sexp
    TSNSx,
    /// LSP Code Actions
    LCdAc,
    /// Pick Buffers
    Buffr,
    /// Replace All
    RplcA,
    /// UPPER_SNAKE_CASE
    USnke,
    /// PascalCase
    Pscal,
    /// UPPER-KEBAB-CASE
    UKbab,
    /// UPPER CASE
    Upper,
    /// Title Case
    Title,
    /// snake_case
    Snke_,
    /// camelCase
    Camel,
    /// Wrap
    Wrap_,
    /// kebab-case
    Kbab_,
    /// lower case
    Lower,
    /// Yes
    Yes__,
    /// No
    No___,
    /// Pipe selection to shell
    Pipe_,
    /// Open matching files
    OpenM,
    /// Reveal selections
    RevlS,
    /// Reveal cursors
    RevlC,
    /// Reveal marks
    RevlM,
    /// Previous
    Prev_,
    /// Next
    Next_,
    /// XML, used for Surround
    XML__,
    /// Block Comment
    CmtBk,
    /// Line Comment
    CmtLn,
    /// Space Editor
    SpEdt,
    /// Space Pick
    SpPck,
    /// Space Context
    SpCtx,
    /// Revert hunk (to main branch)
    RvHkM,
    /// Revert hunk (to current branch)
    RvHkC,
    /// Git Blame
    GtBlm,
    /// Reload buffer
    RlBfr,
    /// Open search prompt with current selection
    SchWC,
    /// Go to file
    GoFil,
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
