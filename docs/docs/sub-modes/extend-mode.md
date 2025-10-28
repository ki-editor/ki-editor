---
sidebar_position: 7.5
---

import {TutorialFallback} from '@site/src/components/TutorialFallback';
import {KeymapFallback} from '@site/src/components/KeymapFallback';

# Extend Mode

## Intro

This mode can be entered by pressing `f` (Qwerty), it is a short-lived mode where the next keypress
leads to one of the following:

1. Surround-related actions
2. Extending selection

## Surround-related actions

### Keymap

<KeymapFallback filename="Extend"/>

This is a group of actions that is related to "surround" or "enclosures".

| Label             | Action                                                                                                                                                                   |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `Around`          | Select around `<x>`                                                                                                                                                      |
| `Inside`          | Select inside `<x>`                                                                                                                                                      |
| `Delete Surround` | Delete surrounding `<x>`                                                                                                                                                 |
| `Change Surround` | Change surrounding `<x>` to `<y>`                                                                                                                                        |
| `Surround`        | Surround current selection with `<x>`                                                                                                                                    |
| `Select All`      | Select the from first until the last selection of the current selection mode (use with [Line](../normal-mode/selection-modes/primary.md#line) to select the whole file). |

`<x>` or `<y>` can be one of the following:

- `()` Parenthesis
- `{}` Curly Brace
- `[]` Square Bracket
- `<>` Angular Bracket
- `'` Single Quote
- `"` Double Quote
- <code>`</code> Backtick
- `<></>` XML Tag 

<TutorialFallback filename="surround"/>

## Extending selection

This is used for extending the current selection.

For example, selecting multiple words or multiple lines.

It behaves more or less the same as click-and-drag in the textbox or text area of common GUI applications, but imagine being able to tune **both** ends, unlike using a mouse where an incorrect selection means you have to start over again.

When selection extension is enabled:

1. Each selection is composed of two ranges (originally one range).
1. There's only one moveable range at a time.
1. Every character between the two ranges, including the two ranges, is selected
1. Selection-wise actions work on the extended range
1. Press `ESC` to disable selection extension

<TutorialFallback filename="extend"/>
