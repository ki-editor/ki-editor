use crossterm::event::KeyCode;
use event::KeyEvent;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const KEYMAP_SCORE: [[char; 10]; 3] = [
    // a = Easiest to access
    // o = Hardest to access
    // Left side (a-o)        Right side (a-o)
    ['m', 'h', 'f', 'i', 'n', /*|*/ 'n', 'i', 'f', 'h', 'm'], // Top row
    ['d', 'b', 'a', 'c', 'e', /*|*/ 'e', 'c', 'a', 'b', 'd'], // Home row
    ['j', 'k', 'l', 'g', 'o', /*|*/ 'o', 'g', 'l', 'k', 'j'], // Bottom row
];

pub type KeyboardLayout = [[&'static str; 10]; 3];

pub const QWERTY: KeyboardLayout = [
    ["q", "w", "e", "r", "t", "y", "u", "i", "o", "p"],
    ["a", "s", "d", "f", "g", "h", "j", "k", "l", ";"],
    ["z", "x", "c", "v", "b", "n", "m", ",", ".", "/"],
];

pub const ABNT2: KeyboardLayout = [
    ["q", "w", "e", "r", "t", "y", "u", "i", "o", "p"],
    ["a", "s", "d", "f", "g", "h", "j", "k", "l", "ç"],
    ["z", "x", "c", "v", "b", "n", "m", ",", ".", ";"],
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

/// Semi-colon and Quote are swapped
/// Refer https://colemakmods.github.io/mod-dh/
pub const COLEMAK_DH_SEMI_QUOTE: KeyboardLayout = [
    ["q", "w", "f", "p", "b", "j", "l", "u", "y", "'"],
    ["a", "r", "s", "t", "g", "m", "n", "e", "i", "o"],
    ["z", "x", "c", "d", "v", "k", "h", ",", ".", "/"],
];

pub const COLEMAK_ANSI: KeyboardLayout = [
    ["q", "w", "f", "p", "g", "j", "l", "u", "y", ";"],
    ["a", "r", "s", "t", "d", "h", "n", "e", "i", "o"],
    ["z", "x", "c", "v", "b", "k", "m", ",", ".", "/"],
];

// https://colemakmods.github.io/mod-dh/keyboards.html#ansi-keyboards
pub const COLEMAK_ANSI_DH: KeyboardLayout = [
    ["q", "w", "f", "p", "b", "j", "l", "u", "y", ";"],
    ["a", "r", "s", "t", "g", "m", "n", "e", "i", "o"],
    ["x", "c", "d", "v", "z", "k", "h", ",", ".", "/"],
];

/// Refer https://workmanlayout.org/
pub const WORKMAN: KeyboardLayout = [
    ["q", "d", "r", "w", "b", "j", "f", "u", "p", ";"],
    ["a", "s", "h", "t", "g", "y", "n", "e", "o", "i"],
    ["z", "x", "m", "c", "v", "k", "l", ",", ".", "/"],
];

/// Refer http://adnw.de/index.php?n=Main.OptimierungF%c3%bcrDieGeradeTastaturMitDaumen-Shift
pub const PUQ: KeyboardLayout = [
    ["p", "u", ":", ",", "q", "g", "c", "l", "m", "f"],
    ["h", "i", "e", "a", "o", "d", "t", "r", "n", "s"],
    ["k", "y", ".", "'", "x", "j", "v", "w", "b", "z"],
];

#[derive(
    Debug, Clone, strum_macros::EnumIter, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Copy,
)]
pub enum KeyboardLayoutKind {
    Qwerty,
    Abnt2,
    Dvorak,
    DvorakIu,
    Colemak,
    ColemakDh,
    ColemakDhSemiQuote,
    ColemakAnsi,
    ColemakAnsiDh,
    Workman,
    Puq,
}

impl KeyboardLayoutKind {
    pub const fn display(&self) -> &'static str {
        match self {
            Self::Qwerty => "QWERTY",
            Self::Abnt2 => "ABNT2",
            Self::Dvorak => "DVORAK",
            Self::Colemak => "COLEMAK",
            Self::ColemakDh => "COLEMAK-DH",
            Self::ColemakDhSemiQuote => "COLEMAK-DH;",
            Self::ColemakAnsi => "COLEMAK (ANSI)",
            Self::ColemakAnsiDh => "COLEMAK-DH (ANSI)",
            Self::DvorakIu => "DVORAK-IU",
            Self::Workman => "WORKMAN",
            Self::Puq => "PUQ",
        }
    }

    pub fn get_keyboard_layout(&self) -> &KeyboardLayout {
        match self {
            Self::Qwerty => &QWERTY,
            Self::Abnt2 => &ABNT2,
            Self::Dvorak => &DVORAK,
            Self::Colemak => &COLEMAK,
            Self::ColemakDh => &COLEMAK_DH,
            Self::ColemakDhSemiQuote => &COLEMAK_DH_SEMI_QUOTE,
            Self::ColemakAnsi => &COLEMAK_ANSI,
            Self::ColemakAnsiDh => &COLEMAK_ANSI_DH,
            Self::DvorakIu => &DVORAK_IU,
            Self::Workman => &WORKMAN,
            Self::Puq => &PUQ,
        }
    }

    pub fn translate_char_to_qwerty(&self, char_to_translate: char) -> char {
        let zipped_chars = || {
            self.get_keyboard_layout()
                .iter()
                .flatten()
                .zip(QWERTY.iter().flatten())
                .map(|(this, qwerty)| {
                    // Crossterm uses char, so we need to convert
                    (this.chars().next().unwrap(), qwerty.chars().next().unwrap())
                })
        };
        zipped_chars()
            .chain(zipped_chars().map(|(this, qwerty)| (shifted_char(this), shifted_char(qwerty))))
            .find_map(|(this, qwerty)| (this == char_to_translate).then_some(qwerty))
            .unwrap_or(char_to_translate)
    }

    pub fn translate_key_event_to_qwerty(&self, event: KeyEvent) -> KeyEvent {
        match event.code {
            KeyCode::Char(pressed_char) => {
                let translated_char = self.translate_char_to_qwerty(pressed_char);
                let shift = translated_char.is_uppercase();
                KeyEvent {
                    code: KeyCode::Char(translated_char),
                    modifiers: event.modifiers.set_shift(shift),
                    kind: event.kind,
                }
            }
            _ => event,
        }
    }
}

pub fn shifted(c: &'static str) -> &'static str {
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

pub fn shifted_char(c: char) -> char {
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

pub fn alted(c: &'static str) -> &'static str {
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
