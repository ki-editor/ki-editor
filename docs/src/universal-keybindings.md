# Universal Keybindings

The keybindings presented here work in any [Modes](./modes.md).

## Switch view alignment

Keybinding: `ctrl+l`  
Memory aid: In bash/fish shell, press `ctrl+l` clears the screen and brings the cursor to the top.

There are 3 kinds of view alignments (in order):

1. Top
1. Center
1. Bottom

Pressing `ctrl+l` continuously cycles through the list above in order, starting from Top.

## Other Window

Keybinding: `ctrl+o`

This cycles the cursor to the next window on the screen.

This is useful when you want to scroll the content of another window or copy the content out of another window.

Examples of such windows are:

1. Hover Info
2. Completion Info

## Close current window

Keybinding: `ctrl+c`  
Memory aid: c stands for close

Note: when the current window is closed, all of its children will be unmounted (removed) from the screen as well.

## Paste

Keybinding: `ctrl+v`  
Memory aid: same as Windows or macOS

Although there's already a [Paste](./normal-mode/actions/index.md#paste) action
in Normal mode, `ctrl+v` is more efficient sometimes than hopping between
Insert mode and Normal mode for minuscule changes.

For example, assuming the clipboard contains `hello`, and you wanted the result to be `<div>hello</div>`, and the current mode is Insert mode:

| Mode   | Keys sequence                   | Keypress count |
| ------ | ------------------------------- | -------------- |
| Insert | `< d i v > ctrl+v < / d i v >`  | 12             |
| Normal | `< d i v > esc p a < / d i v >` | 14             |
