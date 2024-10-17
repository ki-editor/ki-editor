---
sidebar_position: 3
---

# Regex-based

## Line

Keybinding: `e`

In this selection mode `h`/`l` behaves exactly like `j`/`k`, and the selection
is trimmed, which means that the leading and trailing spaces of each line are
not selected.

This is usually used in conjunction with `i`/`a` to immediately enter insert mode at the first/last non-whitespace symbol of the current line.

## Full Line

Keybinding: `E`

Same as [Line](#line), however, leading whitespaces are selected, and trailing whitespaces, including newline characters are also selected.

## Subword

Keybinding: `W`

This selects subwords, even if these words are not separated by spaces.

For example, `myOatPepperBanana` consists of 4 short words, namely: `my`, `Oat`, `Pepper` and `Banana`.

This is useful for renaming identifiers, especially if we only want to change a single word of the name. [^1]

## Word

Keybinding: `w`

Like [Word](#word), but it treats each word as a sequence of alphanumeric characters (including `-` and `_`).

[^1]: This is possible because even Prompt is an editor, so the Word mode also works there. See [Core Concepts](../../core-concepts.md#2-every-component-is-a-buffereditor)

## Column

Keybindings:

- `z`: Collapse selection (start)
- `$`: Collapse selection (end)

In this selection mode, the movements behave like the usual editor, where [Left/Right](./../core-movements.md#leftright) means left/right, and so on.

[First/Last](./../core-movements.md#firstlast) means the first/last column of the current line.
