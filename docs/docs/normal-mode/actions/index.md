import {TutorialFallback} from '@site/src/components/TutorialFallback';
import {KeymapFallback} from '@site/src/components/KeymapFallback';

# Actions

## Keymap

<KeymapFallback filename="Actions"/>

## Notes for reading

1. When "selection" is mentioned, you should read it as "selection(s)", because
   these actions work with multiple cursors.

## Search

### `Search`

Open search prompt.

### `This`

Search this selection.

<TutorialFallback filename="search-current-selection"/>

## Modifications

### `Raise`

This is one of my favorite actions, it only works for [syntax node](../selection-modes/primary.md#syntax) selection modes.

This replaces the parent node of the current node, with the current node.

<TutorialFallback filename="raise"/>

Note: Raise should never cause any syntax errors, if it does that's a bug.

### `← Replace`/`Replace →`

Replace current selection with previous/next copied text in the clipboard history.

This is similar to [Yanking Earlier Kills](https://www.gnu.org/software/emacs/manual/html_node/emacs/Earlier-Kills.html) in Emacs.

This is useful when you want to retrieve earlier copies.

### `Open`

Open next to current selection.

`Open` is directional[^directionality].

`Open` inserts a newline with the respective indent of the current line,
In Syntax Mode, exceptionally, it inserts a gap next to the current selection.

<TutorialFallback filename="open"/>

### `Delete`

Delete until the left/right selection.

`Delete` is directional[^directionality].

This deletes the current selection(s), however, if the current selection mode is
[contiguous](../selection-modes/index.md#contiguity), it will delete until the
next/previous selection, and selects the next/previous selection.

But, if the current selection is the last/first selection, it will delete until the
previous/next selection instead, and selects the previous/next selection.

For example, consider the following Javascript code:

```js
hello(x, y);
```

Assuming the current selection mode is [Syntax Node](../selection-modes/primary.md#syntax), and the current selection is `x`, pressing `d` results in the following:

```js
hello(y);
```

<TutorialFallback filename="delete"/>

### `Delete 0 Gap`

Delete until the previous/next selection.

This is similar to `Delete`, but it doesn't delete the meaningless gaps between selections.

Meaningless gaps are usually whitespaces, or insignificant nodes like comma or semicolon in the Syntax Node selection mode.

<TutorialFallback filename="delete-0-gap"/>

### `Change`

This deletes the current selected text, and enter [Insert mode](../../insert-mode/index.md).

### `Replace #`

Replace with pattern.

This replaces the current selection using the search pattern and replacement
pattern specified in the [Text Search Configuration](../search-config.md#replacement).

For example:

| Mode                       | Selected text | Search   | Replacement | Result  |
| -------------------------- | ------------- | -------- | ----------- | ------- |
| Literal                    | `f`           | `f`      | `g`         | `g(x)`  |
| Regex                      | `"yo"`        | `"(.*)"` | `[$1]`      | `[yo]`  |
| AST Grep                   | `f(x)`        | `f($Z)`  | `$Z(f)`     | `x(f)`  |
| Naming Convention Agnostic | `a_bu`        | `a bu`   | `to li`     | `to_li` |

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

<KeymapFallback filename="Transform"/>

Transformative actions are nested under here, such as (non-exhaustive):

- Casing conversion
- Wrap
- Toggle line comment
- Toggle block comment

## Meta

### [`← Insert`/`Insert →`](../../insert-mode/index.md)

Enter insert mode before/after selection.

### `Mark Sel`

Toggles a bookmark at the current selection, allowing you to navigate elsewhere
in the codebase while maintaining a reference to your focal point without
memorizing its exact location.

### `Mark File`

Mark/unmark the current file. This feature allows you to efficiently manage and switch
between your primary files and other ancillary files.

File unmarking has two behaviors:

1. When the current file is the only marked file: File remains unmarked and focused.
2. When the current file is NOT the only marked file: File is unmarked and focus shifts to the next marked file, similar to closing a tab.

To move between marked files, see [here](../other-movements#-markedmarked-).

#### Workflow Overview

This workflow is designed to streamline your editing process by allowing quick
access to your primary files. During an editing session, you often work on
primary files while occasionally referring to other less important files. Using
the number keys, you can quickly jump back to your main files, enhancing your
productivity and focus.

By utilizing file marking, you can efficiently navigate your editing
environment and maintain your workflow's momentum.

<TutorialFallback filename="mark-file"/>

### `Undo`/`Redo`

Notes:

1. Undo/redo works for multi-cursors as well
2. The current implementation is naive, it undoes/redoes character-by-character, instead of chunk-by-chunk, so it can be mildly frustrating

### Save

Keybinding: `enter`

Upon saving, formatting will be applied if possible.

After formatting, the [Current](../core-movements.md#current) movement will be executed, to reduce disorientation caused by the misplaced selection due to content changes.

## Clipboard

The usual disntinction between two kinds of (system and editor) clipboards,
like in other editors like vim or helix, are unified in ki's user interface:

For single cursor,

- Copy copies to system clipboard also adds to editor clipboard history
- Paste uses the system clipboard content but adds the pasted content if it is new.

For multiple cursors,

- Copy copies to system clipboard a html formatted text containing list of div tags
- Paste uses the system clipboard's html formatted text containing list of div tags.

Note: Effectively, the user only interacts with the system clipboard, the editor
clipboard provides clipboard history.

### `Copy`

This action copies the current selected text.

Copy behaves differently depending on the number of cursors.

When there is more than one cursor, the selected texts of each cursor will be
copied to the cursor-specific clipboard.

### `Paste`

Paste copied content next to current selection.

`Paste` is directional[^directionality].

This action pastes the content from the clipboard, next to the current selection.

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

Assuming the current selection mode is [Syntax Node](../selection-modes/primary.md#syntax), and the current selection is `y`, and the
copied text is `z`, performing a `p` results in the following:

```js
hello(x, y, z);
```

<TutorialFallback filename="paste"/>

### `Change X`

This is similar to [Change](#change), but it copies the deleted text into the clipboard.Like `ctrl+x` in Windows and `cmd+x` in macOS.

### `Replace`

This replaces the current selected text with the copied text.

### `Replace X`

Replace Cut, swaps the current selection with the content in the clipboard.

<TutorialFallback filename="replace-cut"/>

### `Change keyboard layout`

Keybinding: `*`

This has a special keybinding that is non-positional so that the keyboard layout can be switched easily.

[^directionality]: Actions can have Directionality which can be changed using [`⇋ Curs`](../../normal-mode/other-movements/#-curs). Directionality means, that the result of that action can be applied in two opposite directions. For example, deleting backward and deleting forward, both are the same action only directionally opposite. To change the direction of the action make sure to first swap the cursor using [`⇋ Curs`](../../normal-mode/other-movements/#-curs) before applying the action.
