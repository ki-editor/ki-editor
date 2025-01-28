---
sidebar_position: 8
---

# Universal Keybindings

## Intro

The keybindings presented here work in any [Modes](./modes.md).

## Keymap

```
╭───┬────────┬───────┬─────────┬───┬───┬───┬──────┬───┬───┬──────────╮
│   ┆ Config ┆       ┆         ┆   ┆ ⌥ ┆   ┆      ┆   ┆   ┆          │
│   ┆        ┆       ┆         ┆   ┆ ⇧ ┆   ┆      ┆   ┆   ┆          │
│   ┆        ┆       ┆         ┆   ┆ ∅ ┆   ┆      ┆   ┆   ┆          │
├╌╌╌┼╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌╌╌╌╌╌╌╌┤
│   ┆        ┆       ┆         ┆   ┆ ⌥ ┆   ┆      ┆   ┆   ┆  ⇋ Align │
│   ┆        ┆       ┆         ┆   ┆ ⇧ ┆   ┆      ┆   ┆   ┆          │
│   ┆        ┆       ┆         ┆   ┆ ∅ ┆   ┆      ┆   ┆   ┆          │
├╌╌╌┼╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌╌╌╌╌╌╌╌┤
│   ┆        ┆ Close ┆ Paste → ┆   ┆ ⌥ ┆   ┆ Help ┆   ┆   ┆ ⇋ Window │
│   ┆        ┆       ┆         ┆   ┆ ⇧ ┆   ┆      ┆   ┆   ┆          │
│   ┆        ┆       ┆         ┆   ┆ ∅ ┆   ┆      ┆   ┆   ┆          │
╰───┴────────┴───────┴─────────┴───┴───┴───┴──────┴───┴───┴──────────╯
```

### `⇋ Align`

Switch view alignment.

There are 3 kinds of view alignments (in order):

1. Top
1. Center
1. Bottom

Executing this action continuously cycles through the list above in order, starting from Top.

### `⇋ Window`

Cycle window.

This cycles the cursor to the next window on the screen.

This is useful when you want to scroll the content of another window or copy the content out of another window.

Examples of such windows are:

1. Hover Info
2. Completion Info

### `Close`

Close current window

Note: when the current window is closed, all of its children will be unmounted (removed) from the screen as well.

### `Paste →`

Although there's already a [Paste](./normal-mode/actions/index.md#paste-paste-) action
in Normal mode, `alt+v` is more efficient sometimes than hopping between
Insert mode and Normal mode for minuscule changes.

For example, assuming the clipboard contains `hello`, and you wanted the result to be `<div>hello</div>`, and the current mode is Insert mode:

| Mode   | Keys sequence                   | Keypress count |
| ------ | ------------------------------- | -------------- |
| Insert | `< d i v > alt+v < / d i v >`   | 12             |
| Normal | `< d i v > esc p a < / d i v >` | 14             |

### `Config`

Open the [search config](./normal-mode/search-config.md).
