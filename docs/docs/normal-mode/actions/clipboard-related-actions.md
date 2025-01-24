import {TutorialFallback} from '@site/src/components/TutorialFallback';

# Clipboard-related Actions

The following actions are grouped separately on this page because they interact with the clipboard.

There are two kinds of clipboards:

1. The editor clipboard
2. The system clipboard

By default, the editor clipboard is used, to use the system clipboard, press
`\` before pressing the keybindings of the following actions.

The editor clipboard works for multiple cursors, the text of each cursor can be
copied to and pasted from the editor clipboard respectively.

The system clipboard however does not support multiple cursors.
When there are multiple cursors:

- Copy joins every selection into a single string and then place it in the system clipboard
- Paste uses the same string from the system clipboard for every cursor

Note: when new content are copied to the system clipboard, it will also be
copied to the editor clipboard.

## Copy

Keybindings:

- `y`: Copy to editor clipboard

Memory aid: y stands for yank, yank to the clipboard.

This action copies the current selected text.

Copy behaves differently depending on the number of cursors.

When there is more than one cursor, the selected texts of each cursor will be
copied to the cursor-specific clipboard.

## Paste

Keybindings:

- `p`: Paste after selection
- `P`: Paste before selection

This action pastes the content from the clipboard (either the system clipboard or
cursor-specific clipboard) after/before the current selection.

Notes:

- It does not replace the current selection.
- The pasted text will be selected.

### Smart Paste

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

## Change Cut

Keybindings: `C`

This is similar to [Change](./index.md#change), but it copies the deleted text into the system clipboard.  
Like `ctrl+x` in Windows and `cmd+x` in macOS.

## Replace

Keybindings:

- `r`: Replace
- `R`: Replace (Cut), copies the replaced content into the clipboard

This replaces the current selected text with the copied text.

<TutorialFallback filename="replace-cut"/>
