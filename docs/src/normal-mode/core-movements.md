# Core Movements

Core Movements is one of the main concepts in Ki, because it is standardized for
every [selection modes](./selection-modes/index.md).

There are 9 movements in total:

1. [Next/Previous](#nextprevious)
1. [First-child/Parent](#first-childparent)
1. [Last/First](#lastfirst)
1. [Jump](#jump)
1. [Parent Line](#parent-line)
1. [To Index](#to-index)
1. [Current](#current)

## Next/Previous

Keybinding: `n`/`N`

## First-child/Parent

Keybinding: `k`/`K`  
Memory aid: k means kid, which means child.

This is only useful in [Syntax Tree-based](./selection-modes/syntax-tree-based.md) selection modes.

## Last/First

Keybinding: `z`/`Z`  
Memory aid: z is the last alphabet.

Recommended selection modes:

1. Line
1. Column
1. Search

## Jump

Keybinding: `j`

This is my favorite movement, which is inspired by [Vim Easymotion](https://github.com/easymotion/vim-easymotion) and friends [^1].

It allows you to jump to your desired position (as long as it is within the screen), with just 4 keypresses most of the time.

It works like this:

1. Choose your selection mode
1. Press `j`
1. Press the first letter of the selection that you want to jump to.
1. Press the letter that appears on top of the selection.
1. Done.

Recommended selection modes:

1. Syntax Tree
1. Word
1. Token

This movement can also work with the Exchange movement-action submode to swap two syntax expressions that are far apart.

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
