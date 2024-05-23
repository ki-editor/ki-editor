# Regex-based

## Line (Trimmed)

Keybinding: `e`  
Memory aid: lin**e**

In this selection mode `h`/`l` behaves exactly like `j`/`k`, and trimmed means
that the leading and trailing spaces of each line is not selected.

This is usally used in conjuction with `i`/`a` to immediately enter insert mode at the first/last non-whitespace symbol of the current line.

## Line (Full)

Keybinding: `E`

Same as [Line (Trimmed)](#line-trimmed), however, leading whitespaces are selected, and trailing whitespaces, including newline character are also selected.

## Word (Short)

Keybinding: `w`

This select short words, even if these words are not separated by spaces.

For example, `myOatPepperBanana` consists of 4 short words, namely: `my`, `Oat`, `Pepper` and `Banana`.

This is useful for renaming identifiers, especially if we only want to change a single word of the name. [^1]

It is also useful when we want to modify the content of literal string, because the [Token](./syntax-tree-based.md#token) selection mode skips every words in a literal string.

## Word (Long)

Keybinding: `W`

Like [Word (Short)](#word-short), but it treats each word as a sequence of alphanumeric characters (including `-` and `_`).

[^1]: This is possible because even Prompt is an editor, so the Word (Short) mode also works there. See [Core Concepts](../../core-concepts.md#2-every-component-is-a-buffereditor)

## Column

Keybinding: `u`  
Memory aid: col**u**mn

In this selection mode, the movements behaves like the usual editor, where [Previous/Next](./../core-movements.md#previousnext) means left/right, and so on and so forth.

[First/Last](./../core-movements.md#firstlast) means the first/last column of the current line.
