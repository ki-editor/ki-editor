---
sidebar_position: 2
---

# Core Movements

Core Movements is one of the main concepts in Ki, because it is standardized for
every [selection modes](./selection-modes/index.md).

There are 13 movements in total:

1. [Left/Right](#leftright)
1. [Up/Down](#updown)
1. [First/Last](#firstlast)
1. [Expand/Shrink](#expandshrink)
1. [Jump](#jump)
1. [Parent Line](#parent-line)
1. [To Index](#to-index)
1. [Current](#current)

## Left/Right

Keybinding: `h`/`l`  
Memory aid:

- h stands for higher, which means "move to the selection on the left".
- l stands for lower, which means "move to the selection on the right".

## Up/Down

Keybinding: `k`/`j`  
Memory aid:

- k means king, which means "move to the nearest selection above the current line".
- j means jack, which means "move to the nearest selection below the current line".

## Next/Previous

Keybinding: `n`/`N`

These movemest similar to the 4 movements above, however, they are not restricted to vertical or horizontal movements.
They are assigned special meaning in different selection modes.

| Selection Mode   | Meaning                                   |
| ---------------- | ----------------------------------------- |
| Syntax Node      | Next/Previous named sibling               |
| Quickfix         | To first item of next/previous file       |
| Word & Subword   | Next/Previous word skipping symbols (TBD) |
| Line & Full Line | Next/Previous empty line (TBD)            |

## First/Last

Keybinding: `,`/`.`  
Memory aid: `,`/`.` looks like `<`/`>` on the keyboard

| Selection Mode   | Meaning                                       |
| ---------------- | --------------------------------------------- |
| Syntax Node      | First/Last named sibling                      |
| Quickfix         | First/Last item                               |
| Subword          | First/Last subword in the current word (TBD)  |
| Word             | First/Last word in the current sentence (TBD) |
| Line & Full Line | First/Last line of the current file           |

## Expand/Shrink

Keybinding: `t`/`b`  
Memory aid: t stands for Top (implying expand), while b Bottom (implying shrink)

| Selection Mode        | Meaning                                           |
| --------------------- | ------------------------------------------------- |
| Syntax Node           | Parent/First-child                                |
| Line                  | Select whole file                                 |
| Other selection modes | Expand selection to nearest brackets/quotes (TBD) |

## Jump

Keybinding: `f`  
Reason: This keybinding is used by Vimium.

This is my favorite movement, which is inspired by [Vim Easymotion](https://github.com/easymotion/vim-easymotion) and friends [^1].

It allows you to jump to your desired position (as long as it is within the screen), with just 4 keypresses most of the time.

It works like this:

1. Choose your selection mode
1. Press `f`
1. Press the first letter of the selection that you want to jump to.
1. Press the letter that appears on top of the selection.
1. Done.

Recommended selection modes:

1. Syntax Node
1. Word
1. Token

This movement can also work with the Exchange mode to swap two syntax expressions that are far apart.

[^1]: hop.nvim, leap.nvim, lightspeed.nvim etc.

## Parent line

Keybinding: `-`

It moves the current selection to its nearest parent line.

Parent lines are highlighted lines that represent the parent nodes of the current selection.

This is useful for example when you are within the body of a function and you want to jump to the function name.

This is also practical in the [File Explorer](../components/file-explorer.md) because the file explorer is rendered using YAML, so going to Parent Line means going to the parent folder!

## To Index

Keybinding: `0`  
Memory aid:

- `0` is related to the index.

When `0` is pressed, you will be prompted to key in a numerical index (where 1 represents first), and it will jump the current selection to the nth selection of the current selection mode.

Recommended selection modes:

1. Line (For going to a specific line number)
1. Column (For going to a specific column number)

## Current

This is not really a movement, since it's not "moving" the selections per se.

There's no specific keybinding for Current because it is triggered whenever a
selection mode is chosen.

For example, choosing the Line selection mode causes the current line to be
selected, choosing the Word selection mode causes the current word to be selected.

In cases where there's no matching selection under the cursor, the Current movement chooses the nearest selection based on the following criteria (in order):

1. Same line as cursor (if possible)
2. Nearest to cursor (in terms of horizontal movements)
