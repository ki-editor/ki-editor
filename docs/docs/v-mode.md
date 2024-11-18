---
sidebar_position: 7.5
---

import {TutorialFallback} from '@site/src/components/TutorialFallback';

# V Mode

This mode can be entered by pressing `v`, it is a short-lived mode where the next keypress
leads to one of the following:

1. Surround-related actions (entered by pressing either `a`/`i`/`d`/`c`/`s`)
2. Extending selection (entered by pressing any other keys)

## Surround-related actions

This is a group of actions that is related to "surround" or "enclosures".

| Keybinding | Action                                |
| ---------- | ------------------------------------- |
| `a<x>`     | Select around `<x>`                   |
| `i<x>`     | Select inside `<x>`                   |
| `d<x>`     | Delete surrounding `<x>`              |
| `c<x><y>`  | Change surrounding `<x>` to `<y>`     |
| `s<x>`     | Surround current selection with `<x>` |

`<x>` can be one of the following:

- `(` Parenthesis
- `{` Curly Brace
- `[` Square Bracket
- `<` Angular Bracket
- `'` Single Quote
- `"` Double Quote
- <code>`</code> Backtick

<TutorialFallback filename="surround"/>

## Extending selection

This is used for extending the current selection.

For example, selecting multiple words or multiple lines.

It behaves more or less the same as click-and-drag in the textbox or text area of common GUI applications, but imagine being able to tune **both** ends, unlike using a mouse where an incorrect selection means you have to start over again.

When selection extension is enabled:

1. Each selection is composed of two ranges (originally one range).
1. There's only one moveable range at a time.
1. Press `v` again to change the moveable range to the other range.
1. Every character between the two ranges, including the two ranges, is selected
1. Selection-wise actions work on the extended range
1. Press `ESC` to disable selection extension

<TutorialFallback filename="extend"/>
