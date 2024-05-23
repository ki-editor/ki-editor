# Other Movements

The movements categorized here are not affected or bounded by [Selection Modes](./selection-modes/index.md).

## Scrolling

Keybindings:

- `ctrl+u`: scroll half-page up
- `ctrl+d`: scroll half-page down

## Swap cursor with anchor

Keybinding: `%`  
Memory aid: `%` looks contains two circles which looks like two ends.

In Ki, each selection contains a cursor and the anchor.

By default, the cursor sits on the first character of the selection, and the anchor sits on the last character of the selection.

For example, if the current selection is `hello world`, then the cursor sits on `h`, while the anchor sits on `d`.

The anchor serves as a visual aid, making it easier for you to recognize when the selection range has been modified.

This is especially necessary when using selection modes such as [Syntax Tree (Fine)](./selection-modes/syntax-tree-based.md#syntax-tree-fine), where occasionally, the start of a selection remains the same while the end of it changes.

Usefulness of `%`:

- When your current selection spans more than a visible screen, and you wish to see what's at the end of the current selection.
  - For example, when you selected a very long function
- When you wish to start a new selection at the end of the current selection
  - For example, when you selected a line and wish to change its last word, you can do: `e % w c` [^1]

[^1]: Explanation: `e` selects the current line, `%` sets the anchor of the last character of the current line, `w` select the current word under the cursor, `c` deletes the word and enter insert mode.

## Go back/forward

Keybindings:

- `[`: Go back
- `]`: Go forward

`[` is useful when you messed up the current selection, especially when you are
using [Syntax Tree](./selection-modes/syntax-tree-based.md#syntax-tree), and
expanding the current selection to parent node.

Simply press `[` to restore the selection to the previous state.  
Press `]` to restore the selection to the current state.

## Go to previous/next opened file

Keybindings:

- `{`: Go to previous opened file
- `}`: Go to next opened file
