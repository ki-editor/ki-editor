# Other Movements

The movements categorized here are not affected or bounded by [Selection Modes](./selection-modes/index.md).

## Scrolling

Keybindings:

- `ctrl+u`: scroll half-page up
- `ctrl+d`: scroll half-page down

## Swap cursor with anchor

Keybinding: `%`  
Memory aid: `%` looks contains two circles which look like two ends.

In Ki, each selection contains a cursor and an anchor.

By default, the cursor sits on the first character of the selection, and the anchor sits on the last character of the selection.

For example, if the current selection is `hello world`, then the cursor sits on `h`, while the anchor sits on `d`.

The anchor serves as a visual aid, making it easier to recognize when the selection range has been modified.

This is especially necessary when using selection modes such as [Fine Syntax Node](./selection-modes/syntax-node-based.md#fine-syntax-node), where occasionally, the start of a selection remains the same while the end of it changes.

Usefulness of `%`:

- When your current selection spans more than a visible screen, and you wish to see what's at the end of the current selection.
  - For example, when you selected a very long function
- When you wish to start a new selection at the end of the current selection
  - For example, when you select a line and wish to change its last word, you can do: `e % w c` [^1]

[^1]: Explanation: `e` selects the current line, `%` sets the anchor of the last character of the current line, `w` selects the current word under the cursor, `c` deletes the word and enters insert mode.

## Go back/forward

Keybindings:

- `ctrl+o`: Go back
- `ctrl+i`/`tab`: Go forward

`ctrl+o` is useful when you messed up the current selection, especially when you are
using [Syntax Node](./selection-modes/syntax-node-based.md#syntax-node), and
expanding the current selection to parent node.

Simply press `ctrl+o` to restore the selection to the previous state.  
Press `ctrl+i`/`tab` to restore the selection to the current state.

## Cycle primary selection

Keybindings:

- `(`: Cycle primary selection (backward)
- `)`: Cycle primary selection (forward)

## Go to the previous/next opened file

Keybindings:

- `{`: Go to previously opened file
- `}`: Go to the next opened file

## Go to the previous/next buffer

Keybindings:

- `-`: Go to previous buffer
- `=`: Go to next buffer
