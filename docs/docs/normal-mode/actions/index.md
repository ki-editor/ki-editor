import {TutorialFallback} from '@site/src/components/TutorialFallback';

# Actions

## Keymap

```
╭──────────┬───────────┬───────────┬───────────┬────────┬───┬──────────┬───────────┬───────┬───────────┬───╮
│          ┆           ┆           ┆           ┆        ┆ ⌥ ┆          ┆           ┆       ┆           ┆   │
│ ← Search ┆           ┆           ┆           ┆  Raise ┆ ⇧ ┆          ┆ ← Replace ┆  Join ┆ Replace → ┆   │
│ Search → ┆           ┆    This   ┆           ┆        ┆ ∅ ┆          ┆  ← Insert ┆       ┆  Insert → ┆   │
├╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌┼╌╌╌┼╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌┤
│          ┆           ┆           ┆           ┆        ┆ ⌥ ┆          ┆           ┆       ┆           ┆   │
│          ┆           ┆           ┆ Transform ┆ ← Open ┆ ⇧ ┆ ← Delete ┆   Dedent  ┆ Break ┆   Indent  ┆   │
│          ┆           ┆           ┆           ┆ Open → ┆ ∅ ┆ Delete → ┆           ┆       ┆           ┆   │
├╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌┼╌╌╌┼╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌┤
│          ┆           ┆           ┆           ┆        ┆ ⌥ ┆          ┆           ┆       ┆           ┆   │
│   Redo   ┆ Replace # ┆ Replace X ┆  Paste ←  ┆        ┆ ⇧ ┆          ┆  Change X ┆       ┆           ┆   │
│   Undo   ┆  Replace  ┆    Copy   ┆  Paste →  ┆  Mark  ┆ ∅ ┆          ┆   Change  ┆       ┆           ┆   │
╰──────────┴───────────┴───────────┴───────────┴────────┴───┴──────────┴───────────┴───────┴───────────┴───╯
```

## Notes for reading

1. When "selection" is mentioned, you should read it as "selection(s)", because
   these actions work with multiple cursors.

## Search

### `← Search`/`Search →`

Open search prompt.

### `This`

Search this selection.

<TutorialFallback filename="search-current-selection"/>

## Modifications

### `Raise`

This is one of my favorite actions, it only works for [syntax node](../selection-modes/primary.md#syntax-node) selection modes.

This replaces the parent node of the current node, with the current node.

<TutorialFallback filename="raise"/>

Note: Raise should never cause any syntax errors, if it does that's a bug.

### `← Replace`/`Replace →`

Replace current selection with previous/next copied text in the clipboard history.

This is similar to [Yanking Earlier Kills](https://www.gnu.org/software/emacs/manual/html_node/emacs/Earlier-Kills.html) in Emacs.

This is useful when you want to retrieve earlier copies.

### `← Open`/`Open →`

Open before/after selection.

If the current selection mode is not Syntax Node,
then Open inserts a newline with the respective indent after/before the current line.

Otherwise, it inserts a gap before/after the current selection, and then enter Insert mode.

<TutorialFallback filename="open"/>

### `← Delete`/`Delete →`

Delete until previous/next selection.

This deletes the current selection(s), however, if the current selection mode is
[contiguous](../selection-modes/index.md#contiguity), it will delete until the
next/previous selection, and selects the next/previous selection.

But, if the current selection is the last/first selection, it will delete until the
previous/next selection instead, and selects the previous/next selection.

For example, consider the following Javascript code:

```js
hello(x, y);
```

Assuming the current selection mode is [Syntax Node](../selection-modes/primary.md#syntax-node), and the current selection is `x`, pressing `d` results in the following:

```js
hello(y);
```

<TutorialFallback filename="delete"/>

### `Change`

This deletes the current selected text, and enter [Insert mode](../../insert-mode/index.md).

### `Replace #`

Replace with pattern.

This replaces the current selection using the search pattern and replacement
pattern specified in the [Text Search Configuration](../search-config.md).

For example:

| Mode                       | Selected text | Search   | Replacement | Result  |
| -------------------------- | ------------- | -------- | ----------- | ------- |
| Literal                    | `f`           | `f`      | `g`         | `g(x)`  |
| Regex                      | `"yo"`        | `"(.*)"` | `[$1]`      | `[yo]`  |
| AST Grep                   | `f(x)`        | `f($Z)`  | `$Z(f)`     | `x(f)`  |
| Naming Convention Agnostic | `a_bu`        | `a bu`   | `to li`     | `to_li` |

<TutorialFallback filename="replace-with-pattern"/>

### `Join`

Joins multiple lines within the current selection(s) into a single line.

<TutorialFallback filename="join"/>

### `Break`

Break the current selection(s) to the next line, with the indentation of the current line.

This is a shortcut of `i enter esc`.

<TutorialFallback filename="break"/>

### `Dedent`/`Indent`

Dedent/Indent the current selection by 4 spaces.

### `Transform`

Transformative actions are nested under here, such as (non-exhaustive):

- `w`: Wrap (Wrap current selection into multiple lines)
- `l`: Convert to `lower case`
- `s`: Convert to `snake_case`

## Meta

### [`← Insert`/`Insert →`](../../insert-mode/index.md)

Enter insert mode before/after selection.

### `Mark`

Toggles a bookmark at the current selection, allowing you to navigate elsewhere
in the codebase while maintaining a reference to your focal point without
memorizing its exact location.

### `Undo`/`Redo`

Notes:

1. Undo/redo works for multi-cursors as well
2. The current implementation is naive, it undoes/redoes character-by-character, instead of chunk-by-chunk, so it can be mildly frustrating

### Save

Keybinding: `enter`

Upon saving, formatting will be applied if possible.

After formatting, the [Current](../core-movements.md#current) movement will be executed, to reduce disorientation caused by the misplaced selection due to content changes.

## Clipboard

There are two kinds of clipboards:

1. The editor clipboard
2. The system clipboard

By default, the editor clipboard is used, to use the system clipboard, press
`space` before pressing the keybindings of the following actions.

The editor clipboard works for multiple cursors, the text of each cursor can be
copied to and pasted from the editor clipboard respectively.

The system clipboard however does not support multiple cursors.
When there are multiple cursors:

- Copy joins every selection into a single string and then place it in the system clipboard
- Paste uses the same string from the system clipboard for every cursor

Note: when new content are copied to the system clipboard, it will also be
copied to the editor clipboard.

### `Copy`

This action copies the current selected text.

Copy behaves differently depending on the number of cursors.

When there is more than one cursor, the selected texts of each cursor will be
copied to the cursor-specific clipboard.

### `Paste ←`/`Paste →`

Paste before/after selection.

This action pastes the content from the clipboard (either the system clipboard or
cursor-specific clipboard) after/before the current selection.

Notes:

- It does not replace the current selection.
- The pasted text will be selected.

#### Smart Paste

Smart Paste will be executed when the selection mode is [contiguous](../selection-modes/index.md#contiguity).

Smart Paste works by analyzing the gap between the current selection and the
previous/next selection, then insert the gap before/after the pasted text.

For example, consider the following Javascript code:

```js
hello(x, y);
```

Assuming the current selection mode is [Syntax Node](../selection-modes/primary.md#syntax-node), and the current selection is `y`, and the
copied text is `z`, performing a `p` results in the following:

```js
hello(x, y, z);
```

<TutorialFallback filename="paste"/>

### `Change X`

This is similar to [Change](#change), but it copies the deleted text into the system clipboard.  
Like `ctrl+x` in Windows and `cmd+x` in macOS.

### `Replace`

This replaces the current selected text with the copied text.

### `Replace X`

Replace Cut, swaps the current selection with the content in the clipboard.

<TutorialFallback filename="replace-cut"/>
