use std::collections::HashMap;

use crossterm::event::KeyCode;
use event::KeyEvent;
use my_proc_macros::key;

pub const KEYMAP_SCORE: [[char; 10]; 3] = [
    // a = Easiest to access
    // o = Hardest to access
    // Left side (a-o)        Right side (a-o)
    ['m', 'h', 'f', 'i', 'n', /*|*/ 'n', 'i', 'f', 'h', 'm'], // Top row
    ['d', 'b', 'a', 'c', 'e', /*|*/ 'e', 'c', 'a', 'b', 'd'], // Home row
    ['j', 'k', 'l', 'g', 'o', /*|*/ 'o', 'g', 'l', 'k', 'j'], // Bottom row
];

pub type KeyboardLayoutKeys = [[char; 10]; 3];

pub const QWERTY: KeyboardLayoutKeys = [
    ['q', 'w', 'e', 'r', 't', 'y', 'u', 'i', 'o', 'p'],
    ['a', 's', 'd', 'f', 'g', 'h', 'j', 'k', 'l', ';'],
    ['z', 'x', 'c', 'v', 'b', 'n', 'm', ',', '.', '/'],
];

pub const QWERTY_STR: [[&str; 10]; 3] = [
    ["q", "w", "e", "r", "t", "y", "u", "i", "o", "p"],
    ["a", "s", "d", "f", "g", "h", "j", "k", "l", ";"],
    ["z", "x", "c", "v", "b", "n", "m", ",", ".", "/"],
];

pub const QWERTY_EVENT: [[KeyEvent; 10]; 3] = [
    [
        key!("q"),
        key!("w"),
        key!("e"),
        key!("r"),
        key!("t"),
        key!("y"),
        key!("u"),
        key!("i"),
        key!("o"),
        key!("p"),
    ],
    [
        key!("a"),
        key!("s"),
        key!("d"),
        key!("f"),
        key!("g"),
        key!("h"),
        key!("j"),
        key!("k"),
        key!("l"),
        key!(";"),
    ],
    [
        key!("z"),
        key!("x"),
        key!("c"),
        key!("v"),
        key!("b"),
        key!("n"),
        key!("m"),
        key!(","),
        key!("."),
        key!("/"),
    ],
];

pub const BUILTIN_KEYBOARD_LAYOUTS: &[(&str, KeyboardLayoutKeys)] = &[
    ("QWERTY", QWERTY),
    (
        "DVORAK",
        [
            ['\'', ',', '.', 'p', 'y', 'f', 'g', 'c', 'r', 'l'],
            ['a', 'o', 'e', 'u', 'i', 'd', 'h', 't', 'n', 's'],
            [';', 'q', 'j', 'k', 'x', 'b', 'm', 'w', 'v', 'z'],
        ],
    ),
    (
        "DVORAK-IU",
        // I and U swapped.
        // Refer https://www.reddit.com/r/dvorak/comments/tfz53r/have_anyone_tried_swapping_u_with_i/
        [
            ['\'', ',', '.', 'p', 'y', 'f', 'g', 'c', 'r', 'l'],
            ['a', 'o', 'e', 'i', 'u', 'd', 'h', 't', 'n', 's'],
            [';', 'q', 'j', 'k', 'x', 'b', 'm', 'w', 'v', 'z'],
        ],
    ),
    (
        "COLEMAK",
        [
            ['q', 'w', 'f', 'p', 'b', 'j', 'l', 'u', 'y', ';'],
            ['a', 'r', 's', 't', 'g', 'm', 'n', 'e', 'i', 'o'],
            ['z', 'x', 'c', 'd', 'v', 'k', 'h', ',', '.', '/'],
        ],
    ),
    (
        "COLEMAK-DH",
        // Refer https://colemakmods.github.io/mod-dh/
        [
            ['q', 'w', 'f', 'p', 'b', 'j', 'l', 'u', 'y', ';'],
            ['a', 'r', 's', 't', 'g', 'm', 'n', 'e', 'i', 'o'],
            ['z', 'x', 'c', 'd', 'v', 'k', 'h', ',', '.', '/'],
        ],
    ),
    (
        "COLEMAK-DH;",
        // Semi-colon and Quote are swapped
        // Refer https://colemakmods.github.io/mod-dh/
        [
            ['q', 'w', 'f', 'p', 'b', 'j', 'l', 'u', 'y', '\''],
            ['a', 'r', 's', 't', 'g', 'm', 'n', 'e', 'i', 'o'],
            ['z', 'x', 'c', 'd', 'v', 'k', 'h', ',', '.', '/'],
        ],
    ),
    (
        "COLEMAK (ANSI)",
        [
            ['q', 'w', 'f', 'p', 'g', 'j', 'l', 'u', 'y', ';'],
            ['a', 'r', 's', 't', 'd', 'h', 'n', 'e', 'i', 'o'],
            ['z', 'x', 'c', 'v', 'b', 'k', 'm', ',', '.', '/'],
        ],
    ),
    (
        "COLEMAK-DH (ANSI)",
        // https://colemakmods.github.io/mod-dh/keyboards.html#ansi-keyboards
        [
            ['q', 'w', 'f', 'p', 'b', 'j', 'l', 'u', 'y', ';'],
            ['a', 'r', 's', 't', 'g', 'm', 'n', 'e', 'i', 'o'],
            ['x', 'c', 'd', 'v', 'z', 'k', 'h', ',', '.', '/'],
        ],
    ),
    (
        "WORKMAN",
        // Refer https://workmanlayout.org/
        [
            ['q', 'd', 'r', 'w', 'b', 'j', 'f', 'u', 'p', ';'],
            ['a', 's', 'h', 't', 'g', 'y', 'n', 'e', 'o', 'i'],
            ['z', 'x', 'm', 'c', 'v', 'k', 'l', ',', '.', '/'],
        ],
    ),
    (
        "PUQ",
        // Refer http://adnw.de/index.php?n=Main.OptimierungF%c3%bcrDieGeradeTastaturMitDaumen-Shift
        [
            ['p', 'u', ':', ',', 'q', 'g', 'c', 'l', 'm', 'f'],
            ['h', 'i', 'e', 'a', 'o', 'd', 't', 'r', 'n', 's'],
            ['k', 'y', '.', '\'', 'x', 'j', 'v', 'w', 'b', 'z'],
        ],
    ),
    (
        "ERGO-L",
        // Refer https://ergol.org/
        [
            ['q', 'c', 'o', 'p', 'w', 'j', 'm', 'd', '’', 'y'],
            ['a', 's', 'e', 'n', 'f', 'l', 'r', 't', 'i', 'u'],
            ['z', 'x', '?', 'v', 'b', ':', 'h', 'g', ';', 'k'],
        ],
    ),
];

pub fn builtin_layout_map() -> HashMap<String, KeyboardLayout> {
    BUILTIN_KEYBOARD_LAYOUTS
        .iter()
        .map(|(name, keys)| {
            (
                name.to_string(),
                KeyboardLayout {
                    name: name.to_string(),
                    keys: *keys,
                },
            )
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyboardLayout {
    name: String,
    keys: KeyboardLayoutKeys,
}

impl KeyboardLayout {
    pub fn new(name: String, keys: KeyboardLayoutKeys) -> Self {
        Self { name, keys }
    }
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn get_keyboard_layout(&self) -> &KeyboardLayoutKeys {
        &self.keys
    }

    pub fn translate_char_to_qwerty(&self, char_to_translate: char) -> char {
        let zipped_chars = || {
            self.get_keyboard_layout()
                .iter()
                .flatten()
                .zip(QWERTY.iter().flatten())
                .map(|(a, b)| (*a, *b))
        };
        zipped_chars()
            .chain(zipped_chars().map(|(this, qwerty)| (shifted_char(this), shifted_char(qwerty))))
            .find_map(|(this, qwerty)| (this == char_to_translate).then_some(qwerty))
            .unwrap_or(char_to_translate)
    }

    pub fn translate_char_from_qwerty(&self, qwerty_char: char) -> char {
        let zipped_chars = || {
            self.get_keyboard_layout()
                .iter()
                .flatten()
                .zip(QWERTY.iter().flatten())
                .map(|(char, qwerty)| (*char, *qwerty))
        };
        zipped_chars()
            .chain(zipped_chars().map(|(this, qwerty)| (shifted_char(this), shifted_char(qwerty))))
            .find_map(|(this, qwerty)| (qwerty == qwerty_char).then_some(this))
            .unwrap_or(qwerty_char)
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

pub fn shifted(mut key_event: KeyEvent) -> KeyEvent {
    if let KeyCode::Char(ref mut c) = key_event.code {
        *c = shifted_char(*c);
    }
    key_event.modifiers.shift = true;
    key_event
}

pub fn possibly_alted(key_event: KeyEvent, is_alted: bool) -> KeyEvent {
    if is_alted {
        alted(key_event)
    } else {
        key_event
    }
}

pub fn alted(mut key_event: KeyEvent) -> KeyEvent {
    key_event.modifiers.alt = true;
    key_event
}
