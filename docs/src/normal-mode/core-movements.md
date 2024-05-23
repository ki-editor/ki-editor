# Core Movements

Core Movements is one of the main concept in Ki Editor, because it is standardized for
every [selection modes](./selection-modes/index.md).

There are 9 movements in total:

1. [Previous/Next](#previousnext)
1. [Up/Down](#updown)
1. [Down](#down)
1. [First/Last](#firstlast)
1. [Fly](#fly)
1. [Parent Line](#parent-line)
1. [To Index](#to-index)
1. [Current](#current)

## Previous/Next

Keybinding: `h`/`l`  
Memory aid:

- h stands for higher, which means previous.
- l stands for lower, which means next.

## Up/Down

Keybinding: `k`/`j`  
Memory aid:

- k means king, which means up.
- j means jack, which means down.

In most selection modes, up/down moves the selections upwards/downwards, except for the following selection modes:

| Selection Mode | Up                                | Down                            |
| -------------- | --------------------------------- | ------------------------------- |
| Syntax Tree    | Expand selection to parent        | Shirnk selection to first child |
| Quickfix       | Go to first item of previous file | Go to first item of next file   |

## First/Last

Keybinding: `,`/`.`  
Memory aid:

- `,`/`.` looks like `<`/`>` on the keyboard

Recommended selection modes:

1. Line
1. Column
1. Search

## Fly

Keybinding: `f`

This is my favourite movement, which is inspired by [Vim Easymotion](https://github.com/easymotion/vim-easymotion) and friends [^1].

It allows you to fly to your desired position (as long as it is within the screen), with just 4 keypresses most of the time.

It works like this:

1. Choose your selection mode
1. Press `f`
1. Press the first letter of the selection that you want to fly to.
1. Press the letter that appears on top of the selection.
1. Done.

Recommended selection modes:

1. Syntax Tree
1. Word
1. Token

This movement can also work with the Exchange mode to swap two syntax expressions that are far apart.

[^1]: hop.nvim, leap.nvim, lightspeed.nvim etc.

## Parent line

Keybinding: `-`

It moves the current selection to its nearest parent line.

Parent lines are highlighted lines which represents the parent nodes of the current selection.

This is useful for example when you are within the body of a function and you want to jump to the function name.

## To Index

Keybinding: `0`  
Memory aid:

- `0` is related to index.

When `0` is pressed, you will be prompted to key in a numerical index (where 1 represent first), and it will jump the current selection to the nth selection of the current selection mode.

Recommended selection modes:

1. Line (For going to a specific line number)
1. Column (For going to a specific column number)

## Current

This is not really a movement, since it's not "moving" the selections per se.

There's no specific keybinding for Current because it is triggered whenever a
selection mode is chosen.

For example, choosing the Line selection mode causes the current line to be
selected, choosing the Word selection mode causes the current word to be selected.

In case where there's no matching selection under the cursor, the Current movement chooses the nearest selection based on the following criteria (in order):

1. Same line as cursor (if possible)
2. Nearest to cursor (in terms of horizontal movements)
