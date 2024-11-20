---
sidebar_position: 3
---

import {TutorialFallback} from '@site/src/components/TutorialFallback';

# Regex-based

## Line

Keybinding: `e`

In this selection mode, the selection is trimmed, which means that the leading
and trailing spaces of each line are not selected.

This is usually used in conjunction with `i`/`a` to immediately enter insert mode at the first/last non-whitespace symbol of the current line.

| Movement      | Meaning                                         |
| ------------- | ----------------------------------------------- |
| Up/Down       | Move to line above/below                        |
| Previous/Next | Move to the nearest empty line below/above      |
| First/Last    | Move to the first/last line of the current file |
| Left          | Move to the parent line                         |

Parent lines are highlighted lines that represent the parent nodes of the current selection.

This is useful for example when you are within the body of a function and you want to jump to the function name.

This is also practical in the [File Explorer](../../components/file-explorer.md) because the file explorer is rendered using YAML, so going to Parent Line means going to the parent folder!

<TutorialFallback filename="line"/>

## Full Line

Keybinding: `E`

Same as [Line](#line), however, leading whitespaces are selected, and trailing whitespaces, including newline characters are also selected.

## Word

Keybinding: `w`

This selects word within a token.

For example, `myOatPepperBanana` consists of 4 short words, namely: `my`, `Oat`, `Pepper` and `Banana`.

This is useful for renaming identifiers, especially if we only want to change a single word of the name. [^1]

<TutorialFallback filename="word"/>

## Token

Keybinding: `t`

Like [Word](#word), but it treats each unit as a sequence of alphanumeric characters (including `-` and `_`).

<TutorialFallback filename="token"/>

[^1]: This is possible because even Prompt is an editor, so the Word mode also works there. See [Core Concepts](../../core-concepts.md#2-every-component-is-a-buffereditor)

## Column

Keybindings:

- `z`: Collapse selection (start)
- `$`: Collapse selection (end)

In this selection mode, the movements behave like the usual editor, where [Left/Right](./../core-movements.md#leftright) means left/right, and so on.

[First/Last](./../core-movements.md#firstlast) means the first/last column of the current line.
