---
sidebar_position: 8
---

import {KeymapFallback} from '@site/src/components/KeymapFallback';
import {TutorialFallback} from '@site/src/components/TutorialFallback';

# Universal Keybindings

## Intro

The keybindings presented here work in any mode.

## Keymap

<KeymapFallback filename="Universal Keymap"/>

### `⇋ Align`

Switch view alignment.

This is similar to Vim's `zt`, `zz` and `zb`, however, it works for multiple line selections.

There are 3 kinds of view alignments (in order):

1. Top: align first line of selection to the top
1. Center: align the middle line of selection to the center
1. Bottom: align the last line of the selection to the bottom

Executing this action continuously cycles through the list above in order, starting from Top.

<TutorialFallback filename="align-view"/>

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
