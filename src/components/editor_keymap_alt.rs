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

// Row 1
pub const SELECTION_MODE_LINE: &str = "q";
pub const SELECTION_MODE_FULL_LINE: &str = "Q";
pub const SELECTION_MODE_SYNTAX: &str = "w";
pub const SELECTION_MODE_FINE_SYNTAX: &str = "W";
pub const SELECTION_MODE_TOKEN: &str = "e";
pub const SELECTION_MODE_WORD: &str = "r";
pub const SELECTION_MODE_CHARACTER: &str = "t";

// Row 2
pub const ACTION_INSERT_START: &str = "a";
pub const ACTION_OPEN_START: &str = "A";
pub const ACTION_INSERT_END: &str = "s";
pub const ACTION_OPEN_END: &str = "S";
pub const ACTION_CHANGE: &str = "d";
pub const CLIPBOARD_CHANGE_CUT: &str = "D";
pub const MOVEMENT_MC_ENTER: &str = "f";
pub const SELECTION_MODE_FIND_GLOBAL: &str = "g";

// Row 3
pub const ACTION_UNDO: &str = "z";
pub const ACTION_REDO: &str = "Z";
pub const MOVEMENT_EXCHANGE_MODE: &str = "x";
pub const CLIPBOARD_YANK: &str = "c";
pub const CLIPBOARD_PASTE_END: &str = "v";
pub const CLIPBOARD_PASTE_START: &str = "V";
pub const SELECTION_MODE_LAST_CONTIGUOUS: &str = "b";

// Row 4
pub const CLIPBOARD_REPLACE_CUT: &str = "shift+backspace";
pub const CLIPBOARD_REPLACE_WITH_COPIED_TEXT: &str = "backspace";

//
// Right Hand
//

// Row 1
pub const ACTION_JOIN: &str = "y";
pub const ACTION_BREAK: &str = "Y";
pub const MOVEMENT_CORE_PREV: &str = "u";
pub const MOVEMENT_CORE_UP: &str = "i";
pub const MOVEMENT_CORE_NEXT: &str = "o";

// Row 2
pub const ACTION_TOGGLE_MARK: &str = "h";
pub const MOVEMENT_CORE_LEFT: &str = "j";
pub const MOVEMENT_CORE_DOWN: &str = "k";
pub const MOVEMENT_CORE_RIGHT: &str = "l";
pub const MOVEMENT_CORE_JUMP: &str = ";";

// Row 3
pub const ACTION_ENTER_V_MODE: &str = "n";
pub const ACTION_SELECT_ALL: &str = "N";
pub const ACTION_CONFIGURE_SEARCH: &str = "m";
pub const MOVEMENT_CORE_FIRST: &str = ",";
pub const MOVEMENT_CORE_LAST: &str = ".";
pub const ACTION_SEARCH_FORWARD: &str = "/";
pub const ACTION_SEARCH_BACKWARD: &str = "?";

// Row 4
pub const ACTION_DELETE_END: &str = "delete";
pub const ACTION_DELETE_START: &str = "shift+delete";
pub const ACTION_SAVE: &str = "enter";

// Multi-cursor
pub const ACTION_MC_DELETE_PRIMARY_CURSOR_START: &str = "d";
pub const ACTION_MC_DELETE_PRIMARY_CURSOR_END: &str = "D";
pub const ACTION_MC_MAINTAIN_SELECTIONS: &str = "m";
pub const ACTION_MC_KEEP_ONLY_PRIMARY_CURSOR: &str = "f";
pub const CLIPBOARD_MC_REMOVE_MATCHING_SEARCH: &str = "r";

// Other
pub const MOVEMENT_CORE_TO_INDEX: &str = "0";
pub const MOVEMENT_OTHER_SWAP: &str = "%";
pub const MOVEMENT_OTHER_CYCLE_START: &str = "(";
pub const MOVEMENT_OTHER_CYCLE_END: &str = ")";
pub const MOVEMENT_OTHER_SCROLL_DOWN: &str = "ctrl+d";
pub const MOVEMENT_OTHER_SCROLL_UP: &str = "ctrl+u";
pub const MOVEMENT_OTHER_GO_BACK: &str = "ctrl+o";
pub const MOVEMENT_OTHER_GO_FORWARD: &str = "tab";
pub const MOVEMENT_OTHER_GO_TO_PREVIOUS_FILE: &str = "{";
pub const MOVEMENT_OTHER_GO_TO_NEXT_FILE: &str = "}";
pub const SELECTION_MODE_FIND_LOCAL_BACKWARD: &str = "[";
pub const SELECTION_MODE_FIND_LOCAL_FORWARD: &str = "]";
pub const ACTION_RAISE: &str = "^";
pub const ACTION_SWITCH_EXTENDED_SELECTION_END: &str = "o";
pub const ACTION_REPLACE_WITH_PATTERN: &str = "ctrl+r";
pub const ACTION_REPLACE_WITH_PREVIOUS_COPIED_TEXT: &str = "ctrl+p";
pub const ACTION_REPLACE_WITH_NEXT_COPIED_TEXT: &str = "ctrl+n";
pub const ACTION_TRANSFORM: &str = "!";
pub const ACTION_COLLAPSE_SELECTION: &str = "$";
pub const ACTION_PIPE: &str = "|";
pub const ACTION_INDENT: &str = ">";
pub const ACTION_DEDENT: &str = "<";
pub const UNIVERSAL_CLOSE_WINDOW: &str = "ctrl+c";
pub const UNIVERSAL_SWITCH_VIEW_ALIGNMENT: &str = "ctrl+l";
pub const UNIVERSAL_SWITCH_WINDOW: &str = "ctrl+s";
pub const UNIVERSAL_PASTE: &str = "ctrl+v";
