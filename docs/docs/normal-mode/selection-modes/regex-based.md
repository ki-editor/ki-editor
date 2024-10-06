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

[^1]: This is possible because even Prompt is an editor, so the Word mode also works there. See [Core Concepts](../../core-concepts.md#2-every-component-is-a-buffereditor)

## Column

Keybindings:

- `^`: Collapse selection (start)
- `$`: Collapse selection (end)

In this selection mode, the movements behave like the usual editor, where [Previous/Next](./../core-movements.md#previousnext) means left/right, and so on.

[First/Last](./../core-movements.md#firstlast) means the first/last column of the current line.

## Till

The "Till" command moves the cursor to the position just before a specified
character on the current line. This feature is similar to Vim's `t` command
but works across multiple lines.

Keybindings:

- `t`: Till forward (move cursor forward to just before the next occurrence of a character)
- `T`: Till backward (move cursor backward to just after the previous occurrence of a character)

Usage:

1. Press `t` (forward) or `T` (backward).
2. Type the character you want to move to.
3. The cursor will move to the position immediately before the specified character.

Examples:

Given the line: `The quick brown fox jumps over the lazy dog`

- With the cursor at the start, pressing `t f` will move the cursor to just before the 'f' in "fox".
- With the cursor at the end, pressing `T o` will move the cursor to just after the 'o' in "dog".
