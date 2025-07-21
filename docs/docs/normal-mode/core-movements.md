---
sidebar_position: 2
---

import {TutorialFallback} from '@site/src/components/TutorialFallback';
import {KeymapFallback} from '@site/src/components/KeymapFallback';

# Core Movements

## Intro

Core Movements is one of the main concepts in Ki, because it is standardized for
every [selection modes](./selection-modes/index.md).

There are 9 movements in total:

- [Core Movements](#core-movements)
  - [Intro](#intro)
  - [Keymap](#keymap)
    - [`◀` `▶` Left/Right](#--leftright)
    - [Previous/Next](#previousnext)
    - [`▲` `▼` Up/Down](#--updown)
      - [Sticky Column](#sticky-column)
    - [First/Last](#firstlast)
    - [`Jump`](#jump)
    - [`Index` Jump to Index](#index-jump-to-index)
    - [Current](#current)

## Keymap

<KeymapFallback filename="Movements"/>

### `◀` `▶` Left/Right

Left/Right means move to the previous/next **meaningful** selection of the current selection mode.

For example:

| Selection Mode | Meaning                     |
| -------------- | --------------------------- |
| Syntax Node    | Next/Previous named sibling |
| Token          | Non-symbol token            |

### Previous/Next

Previous/Next means move to the previous/next selection of the current selection mode.

For example:

| Selection Mode | Meaning                                |
| -------------- | -------------------------------------- |
| Syntax Node    | Sibling nodes including anonymous ones |
| Token          | All tokens including symbols           |
| Line           | Empty lines                            |

### `▲` `▼` Up/Down

Up/Down means move to the nearest selection above/below the current line, except for
the following selection modes:

| Selection Mode | Meaning                             |
| -------------- | ----------------------------------- |
| Syntax Node    | Parent or First-Sibling             |
| Quickfix       | To first item of next/previous file |

#### Sticky Column

When a vertical movement is executed, the current cursor column will be stored as
the sticky column, such that subsequent vertical movements will try to adhere as much
as possible to that sticky column.

The sticky column will be cleared once any non-vertical movement is executed.

<TutorialFallback filename="sticky-column"/>

### First/Last

By default, First/Last moves to the first/last selection of the current selection mode.

| Selection Mode   | Meaning                              |
| ---------------- | ------------------------------------ |
| Syntax Node      | First/Last named sibling             |
| Quickfix         | First/Last item                      |
| Char             | First/Last char in the current word  |
| Word             | First/Last word in the current token |
| Token            | Previous/Next symbolic tokens        |
| Line & Full Line | First/Last line of the current file  |

### `Jump`

This is my favorite movement, which is inspired by [Vim Easymotion](https://github.com/easymotion/vim-easymotion) and friends [^1].

It allows you to jump to your desired position (as long as it is within the screen), with just 4 keypresses most of the time.

It works like this:

1. Choose your selection mode
1. Press `;`
1. Press the first letter of the selection that you want to jump to.
1. Press the letter that appears on top of the selection.
1. Done.

Recommended selection modes:

1. Syntax Node
1. Word
1. Token

This movement can also work with the Swap mode to swap two syntax expressions that are far apart.

[^1]: hop.nvim, leap.nvim, lightspeed.nvim etc.

<TutorialFallback filename="jump"/>

Note: All letters after the first will be selected based on key accessibility in the chosen keyboard layout.

### `Index` Jump to Index

When this is activated, you will be prompted to key in a 1-based index, which after Enter
will take you to the nth selection of the current selection mode.

Recommended selection modes:

1. Line (For going to a specific line number)
2. Char (For going to a specific column number)

### Current

This is not really a movement, since its not "moving" the selections per se.

There's no specific keybinding for Current because it is triggered whenever a
selection mode is chosen.

For example, choosing the Line selection mode causes the current line to be
selected, choosing the Word selection mode causes the current word to be selected.

In cases where there's no matching selection under the cursor, the Current movement chooses the nearest selection based on the following criteria (in order):

1. Same line as cursor (if possible)
2. Nearest to cursor (in terms of horizontal movements)
