// Alternate layout for Colemak-DH by Jeremy
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

// Row 1 (qwfpb)
pub const SELECTION_MODE_WORD: &str = "q";
pub const SELECTION_MODE_CHARACTER: &str = "Q";
pub const MOVEMENT_MC_ENTER: &str = "w";
pub const ACTION_CHANGE: &str = "f";
pub const CLIPBOARD_CHANGE_CUT: &str = "F";
pub const ACTION_ENTER_V_MODE: &str = "p";
pub const ACTION_SELECT_ALL: &str = "P";
pub const ACTION_SEARCH_CURRENT_SELECTION: &str = "b";
pub const SELECTION_MODE_LAST_CONTIGUOUS: &str = "B";

// Row 2 (arstg)
pub const SELECTION_MODE_LINE: &str = "a";
pub const SELECTION_MODE_FULL_LINE: &str = "A";
pub const SELECTION_MODE_TOKEN: &str = "r";
pub const SELECTION_MODE_SYNTAX: &str = "s";
pub const SELECTION_MODE_FINE_SYNTAX: &str = "S";
pub const ACTION_DELETE_END: &str = "t";
pub const ACTION_DELETE_START: &str = "T";
pub const ACTION_SEARCH_FORWARD: &str = "g";
pub const ACTION_SEARCH_BACKWARD: &str = "G";

// Row 3 (zxcdv)
pub const ACTION_UNDO: &str = "z";
pub const ACTION_REDO: &str = "Z";
pub const MOVEMENT_EXCHANGE_MODE: &str = "x";
pub const CLIPBOARD_YANK: &str = "c";
pub const CLIPBOARD_PASTE_END: &str = "d";
pub const CLIPBOARD_PASTE_START: &str = "D";
pub const CLIPBOARD_REPLACE_WITH_COPIED_TEXT: &str = "v";
pub const CLIPBOARD_REPLACE_CUT: &str = "V";

//
// Right Hand
//

// Row 1 (jluy;)
pub const ACTION_TOGGLE_MARK: &str = "j";
pub const ACTION_INSERT_START: &str = "l";
pub const ACTION_OPEN_START: &str = "L";
pub const MOVEMENT_CORE_UP: &str = "u";
pub const ACTION_JOIN: &str = "U";
pub const ACTION_INSERT_END: &str = "y";
pub const ACTION_OPEN_END: &str = "Y";
pub const ACTION_CONFIGURE_SEARCH: &str = ";";

// Row 2 (hneio;)
pub const MOVEMENT_CORE_PREV: &str = "h";
pub const MOVEMENT_OTHER_GO_TO_PREVIOUS_FILE: &str = "H";
pub const MOVEMENT_CORE_LEFT: &str = "n";
pub const SELECTION_MODE_FIND_LOCAL_BACKWARD: &str = "N";
pub const MOVEMENT_CORE_DOWN: &str = "e";
pub const ACTION_BREAK: &str = "E";
pub const MOVEMENT_CORE_RIGHT: &str = "i";
pub const SELECTION_MODE_FIND_LOCAL_FORWARD: &str = "I";
pub const MOVEMENT_CORE_NEXT: &str = "o";
pub const MOVEMENT_OTHER_GO_TO_NEXT_FILE: &str = "O";

// Row 3 (kh,./)
pub const SELECTION_MODE_FIND_GLOBAL: &str = "k";
pub const MOVEMENT_CORE_FIRST: &str = "h";
pub const ACTION_DEDENT: &str = "H";
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
pub const ACTION_COMMAND_MODE: &str = "-";
pub const ACTION_SAVE: &str = "enter";
pub const MOVEMENT_CORE_TO_INDEX: &str = "0";
pub const MOVEMENT_OTHER_CYCLE_START: &str = "(";
pub const MOVEMENT_OTHER_CYCLE_END: &str = ")";
pub const MOVEMENT_OTHER_SCROLL_DOWN: &str = "ctrl+d";
pub const MOVEMENT_OTHER_SCROLL_UP: &str = "ctrl+u";
pub const MOVEMENT_OTHER_GO_BACK: &str = "ctrl+o";
pub const MOVEMENT_OTHER_GO_FORWARD: &str = "tab";
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
