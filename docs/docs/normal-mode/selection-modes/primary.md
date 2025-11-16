---
sidebar_position: 2
---

import {TutorialFallback} from '@site/src/components/TutorialFallback';
import {KeymapFallback} from '@site/src/components/KeymapFallback';

# Primary

## Keymap

<KeymapFallback filename="Primary Selection Modes"/>

## `Syntax`

Syntax Node (Coarse).

This selection mode is powered by [Tree-sitter](https://github.com/tree-sitter).

This is one of my favourite selection mode, as it enable structural editing.

There are two Syntax Node selection modes:

- Coarse: faster movement, lower accuracy
- Fine: higher accuracy, slower movement

| Movement                                           | Meaning                                              |
| -------------------------------------------------- | ---------------------------------------------------- |
| [Left/Right](../core-movements.md#--leftright)     | Next/Previous **named** sibling node                 |
| [Previous/Next](../core-movements.md#previousnext) | Next/Previous sibling node, including anonymous ones |
| Up                                                 | Parent node                                          |
| Down                                               | First **named** child                                |
| Current                                            | Select the largest node                              |
| Jump                                               | Jump to largest node                                 |

### Largest Node

Using the following Javascript expression as example:

```js
fox.bar();
```

There are several syntax nodes that start with `f`[^1]:

- `fox` (identifier)
- `fox.bar` (member expression)
- `fox.bar()` (call expression)

Suppose the cursor is below `f`, pressing `s` selects `fox.bar()`, because `fox.bar()` is the largest node that starts with `f`.

[^1]: You can try it out at [https://astexplorer.net/](https://astexplorer.net/), using the `@typescript-eslint/parser`.

### Named node

When creating a Tree sitter grammar file for a language, the author can choose
to not give names to a certain kind of nodes.

For example, "," are usually unnamed (
anonymous) in most language grammars, thus it will be skipped when using the
Previous/Next movement in Syntax Node.

See more at [https://tree-sitter.github.io/tree-sitter/using-parsers/2-basic-parsing.html#named-vs-anonymous-nodes](https://tree-sitter.github.io/tree-sitter/using-parsers/2-basic-parsing.html#named-vs-anonymous-nodes).

### Examples

<TutorialFallback filename="syntax-node"/>

## `Syntax*`

Fine Syntax Node.

| Movement                                           | Meaning                                              |
| -------------------------------------------------- | ---------------------------------------------------- |
| [Left/Right](../core-movements.md#--leftright)     | Next/Previous **named** sibling node                 |
| [Previous/Next](../core-movements.md#previousnext) | Next/Previous sibling node, including anonymous ones |
| Up                                                 | Parent node                                          |
| Shrink                                             | First child (including anonymous)                    |
| Current                                            | Smallest node that matches the current selection     |
| Jump                                               | Jump to smallest node                                |

Fine Syntax Node is useful when you start to expand the selection starting from the current smallest node.

Suppose we have the following Javascript expression, and the current selection is `hello`, and we want to select `hello.world()`.

```js
hello.world().foo().bar().spam().wise();
```

If we press `d`, the whole expression will be selected[^1], and we will need to press `k` several times to shrink the selection down to `hello.world()`.

However, if we use `D` instead, the selection will remain as `hello`, and pressing `k` multiple times will get us to `hello.world()`.

[^1]: See [Largest Node](#largest-node)

## `Line`

In this selection mode, the selection is trimmed, which means that the leading
and trailing spaces of each line are not meaningful.

The meaningful selection of this mode is the trimmed portion of any non-empty line.

The meaningless selections are empty lines.

| Movement      | Meaning                                                   |
| ------------- | --------------------------------------------------------- |
| Up/Down       | Move to the nearest empty lines above/below               |
| Previous/Next | Move to all kinds of line portions                        |
| First/Last    | Move to the first/last non-empty line of the current file |
| Left/Right    | Move to the previous/next non-empty line                  |

<TutorialFallback filename="line"/>

## `Line*`

Full Line.

Same as [Line](#line), however, leading whitespaces and trailing whitespaces, including newline characters are also selected. And, Right/Left goes to the next empty (whitespaces only) line, this behavior is similar to move by paragraph.

## `Word`

Each unit is a sequence of alphanumeric characters including `-` and `_`.

| Movement              | Meaning                                      |
| --------------------- | -------------------------------------------- |
| Up/Down/Previous/Next | Move to all kinds of word, including symbols |
| Left/Right            | Move to non-symbol word only                 |

Suppose the following example:

```rs
use crate::{components::editor::OpenFile, char_index::CharIndex};
```

If the current selection is selecting `use`, the following table demonstrates how many steps it takes to navigate to `OpenFile`.

| Navigation include/exclude symbols | Steps                                                                | Count |
| ---------------------------------- | -------------------------------------------------------------------- | ----- |
| Include                            | `crate` `:` `:` `{` `components` `:` `:` `editor` `:` `:` `OpenFile` | 11    |
| Exclude                            | `crate` `components` `editor` `OpenFile`                             | 4     |

<TutorialFallback filename="word"/>

[^1]: This is possible because even Prompt is an editor, so the Word mode also works there. See [Core Concepts](../../core-concepts.md#2-every-component-is-a-buffereditor)
[^1]: This is possible because even Prompt is an editor, so the Word mode also works there. See [Core Concepts](../../core-concepts.md#2-every-component-is-a-buffereditor)

## `Subword`

This selects subword within a subword.

For example, `myOatPepperBanana` consists of 4 short subword, namely: `my`, `Oat`, `Pepper` and `Banana`.

This is useful for renaming identifiers, especially if we only want to change a single subword of the name. [^1]

| Movement              | Meaning                                         |
| --------------------- | ----------------------------------------------- |
| Up/Down/Previous/Next | Move to all kinds of subword, including symbols |
| Left/Right            | Move to non-symbol subword only                 |

<TutorialFallback filename="subword"/>

## `Char`

Character.

In this selection mode, the movements behave like the usual editor, where [Left/Right](./../core-movements.md#--leftright) means left/right, and so on.

[First/Last](./../core-movements.md#firstlast) means the first/last character of the current word.

<TutorialFallback filename="char"/>
