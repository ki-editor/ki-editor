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

## Sub Word

Keybinding: `b`

This selects sub words, even if these words are not separated by spaces.

For example, `myOatPepperBanana` consists of 4 short words, namely: `my`, `Oat`, `Pepper` and `Banana`.

This is useful for renaming identifiers, especially if we only want to change a single word of the name. [^1]

It is also useful when we want to modify the content of a literal string because the [Token](./syntax-node-based.md#token) selection mode skips every word in a literal string.

## Word

Keybinding: `w`

Like [Word](#word), but it treats each word as a sequence of alphanumeric characters (including `-` and `_`).

Also, it skips punctuations as it is designed to allow economic and efficient movements.

[^1]: This is possible because even Prompt is an editor, so the Word mode also works there. See [Core Concepts](../../core-concepts.md#2-every-component-is-a-buffereditor)

## Column

Keybinding: `z`

In this selection mode, the movements behave like the usual editor, where [Previous/Next](./../core-movements.md#previousnext) means left/right, and so on.

[First/Last](./../core-movements.md#firstlast) means the first/last column of the current line.
