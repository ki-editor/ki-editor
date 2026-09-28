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

/// German QWERTZ (ISO), which is QWERTY with Y and Z swapped,
/// `ö` in place of `;`, and `-` in place of `/`.
/// Refer https://en.wikipedia.org/wiki/QWERTZ
pub const QWERTZ: KeyboardLayoutKeys = [
    ['q', 'w', 'e', 'r', 't', 'z', 'u', 'i', 'o', 'p'],
    ['a', 's', 'd', 'f', 'g', 'h', 'j', 'k', 'l', 'ö'],
    ['y', 'x', 'c', 'v', 'b', 'n', 'm', ',', '.', '-'],
];

/// Unlike QWERTY, shifting `,` `.` `-` on QWERTZ yields `;` `:` `_`.
const QWERTZ_SHIFTED: KeyboardLayoutKeys = [
    ['Q', 'W', 'E', 'R', 'T', 'Z', 'U', 'I', 'O', 'P'],
    ['A', 'S', 'D', 'F', 'G', 'H', 'J', 'K', 'L', 'Ö'],
    ['Y', 'X', 'C', 'V', 'B', 'N', 'M', ';', ':', '_'],
];

/// Shifted keys of built-in layouts whose shifted characters
/// cannot be derived via [`shifted_char`].
const BUILTIN_SHIFTED_KEYS_OVERRIDES: &[(&str, KeyboardLayoutKeys)] = &[("QWERTZ", QWERTZ_SHIFTED)];

pub const BUILTIN_KEYBOARD_LAYOUTS: &[(&str, KeyboardLayoutKeys)] = &[
    ("QWERTY", QWERTY),
    ("QWERTZ", QWERTZ),
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
        "ENTHIUM",
        // Refer https://github.com/sunaku/enthium
        [
            ['q', 'y', 'o', 'u', '=', 'x', 'l', 'd', 'p', 'z'],
            ['c', 'i', 'a', 'e', '-', 'k', 'h', 't', 'n', 's'],
            ['\'', ',', '.', ';', '/', 'j', 'm', 'g', 'f', 'v'],
        ],
    ),
];

pub fn builtin_layout_map() -> HashMap<String, KeyboardLayout> {
    BUILTIN_KEYBOARD_LAYOUTS
        .iter()
        .map(|(name, keys)| {
            let layout = KeyboardLayout::new(name.to_string(), *keys);
            let layout = BUILTIN_SHIFTED_KEYS_OVERRIDES
                .iter()
                .find(|(override_name, _)| override_name == name)
                .map(|(_, shifted_keys)| layout.clone().with_shifted_keys(*shifted_keys))
                .unwrap_or(layout);
            (name.to_string(), layout)
        })
        .collect()
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct CombinedKeyEvent {
    pub original: KeyEvent,
    pub translated: KeyEvent,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyboardLayout {
    name: String,
    keys: KeyboardLayoutKeys,
    /// The characters produced when each key is pressed with Shift.
    shifted_keys: KeyboardLayoutKeys,
}

impl KeyboardLayout {
    pub fn new(name: String, keys: KeyboardLayoutKeys) -> Self {
        Self {
            name,
            keys,
            shifted_keys: keys.map(|row| row.map(shifted_char)),
        }
    }

    fn with_shifted_keys(self, shifted_keys: KeyboardLayoutKeys) -> Self {
        Self {
            shifted_keys,
            ..self
        }
    }
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn get_keyboard_layout(&self) -> &KeyboardLayoutKeys {
        &self.keys
    }

    pub fn translate_char_to_qwerty(&self, char_to_translate: char) -> char {
        let unshifted = self.keys.iter().flatten().zip(QWERTY.iter().flatten());
        let shifted = self
            .shifted_keys
            .iter()
            .flatten()
            .zip(QWERTY.iter().flatten())
            .map(|(this, qwerty)| (this, shifted_char(*qwerty)));
        unshifted
            .map(|(this, qwerty)| (*this, *qwerty))
            .chain(shifted.map(|(this, qwerty)| (*this, qwerty)))
            .find_map(|(this, qwerty)| (this == char_to_translate).then_some(qwerty))
            .unwrap_or(char_to_translate)
    }

    pub fn make_combined_key_event(&self, event: KeyEvent) -> CombinedKeyEvent {
        let qwerty = match event.code {
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
        };
        CombinedKeyEvent {
            original: event,
            translated: qwerty,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qwertz_translates_to_same_physical_keys_as_qwerty() {
        let qwertz = &builtin_layout_map()["QWERTZ"];
        [
            ('z', 'y'),
            ('y', 'z'),
            ('ö', ';'),
            ('-', '/'),
            ('Z', 'Y'),
            ('Y', 'Z'),
            ('Ö', ':'),
            (';', '<'),
            (':', '>'),
            ('_', '?'),
            ('a', 'a'),
            (',', ','),
        ]
        .into_iter()
        .for_each(|(pressed, expected)| {
            assert_eq!(
                qwertz.translate_char_to_qwerty(pressed),
                expected,
                "pressed {pressed:?}"
            )
        });
    }
}
