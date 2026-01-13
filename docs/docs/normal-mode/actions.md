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

Opens the search prompt.

### `Search This`

Searches the current selection.

<TutorialFallback filename="search-current-selection"/>

### `Search Clipboard`

Searches with the content of the clipboard.

### `With`

Opens the search prompt with the currently selected text pre-filled in the search field.

For example, if you have "foo" selected, executing this action will open the search prompt with "foo" already entered, allowing you to build upon it to create a new query.

This is most useful when you want to search for the current selection with some modifications.

## Modifications

### `Raise`

This is one of my favorite actions, it only works for [syntax node](selection-modes/primary.md#syntax) selection modes.

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

Open inserts a gap between the current and the previous/next selection.

For example:

1. In the Line selection mode, Open opens a newline.
2. In Syntax Node selection mode, Open will insert the separator under certain conditions.

<TutorialFallback filename="open"/>

### `Change`

This deletes the current selected text, and enter [Insert mode](../insert-mode.md).

### `Delete`

Activates the [Delete MOL](../momentary-layers/delete-mol.mdx).

### `Cut`

Activates the [Cut MOL](../momentary-layers/cut-mol.mdx).

### `Replace #`

Replace with pattern.

This replaces the current selection using the search pattern and replacement
pattern specified in the [Text Search Configuration](search-config.md#replacement).

For example:

| Mode                       | Selected text | Search   | Replacement | Result  |
| -------------------------- | ------------- | -------- | ----------- | ------- |
| Literal                    | `f`           | `f`      | `g`         | `g(x)`  |
| Regex                      | `"yo"`        | `"(.*)"` | `[$1]`      | `[yo]`  |
| AST Grep                   | `f(x)`        | `f($Z)`  | `$Z(f)`     | `x(f)`  |
| Naming Convention Agnostic | `a_bu`        | `a bu`   | `to li`     | `to_li` |

### `Join`

Join the current line with the line above.

<TutorialFallback filename="join"/>

### `Break`

Break the current selection(s) to the next line, with the indentation of the current line.

This is a shortcut of `i enter esc`.

<TutorialFallback filename="break"/>

### `Dedent`/`Indent`

Dedent/Indent the current selection by 4 spaces.

### `← Align`/`Align →`

Align selections to the left or right. Similar to Kakoune's `&`.

<TutorialFallback filename="align-selections"/>

### `Transform`

<KeymapFallback filename="Transform"/>

Transformative actions are nested under here, such as (non-exhaustive):

- Casing conversion
- Wrap (converts a single line selection into multiple lines)
- Unwrap (converts a multiline selection into a single line)
- Toggle line comment
- Toggle block comment

## Meta

### [`← Insert`/`Insert →`](../insert-mode.md)

Enter insert mode before/after selection.

### `Mark Sel`

Toggles a bookmark at the current selection, allowing you to navigate elsewhere
in the codebase while maintaining a reference to your focal point without
memorizing its exact location.

Marking a selection additionally also marks the file, however unmarking all of
the marked selections does not unmark the file.

### `Mark File`

Mark/unmark the current file. This feature allows you to efficiently manage and switch
between your primary files and other ancillary files.

File unmarking has two behaviors:

1. When the current file is the only marked file: File remains unmarked and focused.
2. When the current file is NOT the only marked file: File is unmarked and focus shifts to the next marked file, similar to closing a tab.

To move between marked files, see [here](other-movements#-markedmarked-).

#### Workflow Overview

This workflow is designed to streamline your editing process by allowing quick
access to your primary files. During an editing session, you often work on
primary files while occasionally referring to other less important files. Using
`alt+j` and `alt+l` (on Qwerty), you can quickly jump back to your main files, enhancing your
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

After formatting, the [Current](core-movements.md#current) movement will be executed, to reduce disorientation caused by the misplaced selection due to content changes.

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

Activates the [Paste MOL](../momentary-layers/paste-mol.mdx).

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

[^directionality]: Actions can have Directionality which can be changed using [`⇋ Curs`](../normal-mode/other-movements/#-curs). Directionality means, that the result of that action can be applied in two opposite directions. For example, deleting backward and deleting forward, both are the same action only directionally opposite. To change the direction of the action make sure to first swap the cursor using [`⇋ Curs`](../normal-mode/other-movements/#-curs) before applying the action.
