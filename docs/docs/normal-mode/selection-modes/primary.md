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

| Movement                                       | Meaning                              |
| ---------------------------------------------- | ------------------------------------ |
| [Left/Right](../core-movements.md#--leftright) | Next/Previous **named** sibling node |
| Up                                             | Parent node                          |
| Down                                           | First **named** child                |
| Current                                        | Select the largest node              |
| Jump                                           | Jump to largest node                 |

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

| Movement                                       | Meaning                                          |
| ---------------------------------------------- | ------------------------------------------------ |
| [Left/Right](../core-movements.md#--leftright) | Next/Previous sibling node (including anonymous) |
| Up                                             | Parent node                                      |
| Shrink                                         | First child (including anonymous)                |
| Current                                        | Smallest node that matches the current selection |
| Jump                                           | Jump to smallest node                            |

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
and trailing spaces of each line are not selected.

| Movement   | Meaning                                         |
| ---------- | ----------------------------------------------- |
| Up/Down    | Move to line above/below                        |
| First/Last | Move to the first/last line of the current file |
| Left       | Move to the parent line                         |

Parent lines are highlighted lines that represent the parent nodes of the current selection.

This is useful for example when you are within the body of a function and you want to jump to the function name.

This is also practical in the [File Explorer](../../components/file-explorer.md) because the file explorer is rendered using YAML, so going to Parent Line means going to the parent folder!

<TutorialFallback filename="line"/>

## `Line*`

Full Line.

Same as [Line](#line), however, leading whitespaces are selected, and trailing whitespaces, including newline characters are also selected.

## `Token`

Each unit is a sequence of alphanumeric characters including `-` and `_`.

Prev/Next movement skips symbols, while Up/Down/Left/Right movements does not skip symbols.

This means Prev/Next movements are optimized for navigating alphanumeric tokens,

Why don't Prev/Next movements also navigate symbols? Because they would be too slow; it would take a lot of unnecessary keypresses to reach the target.

Suppose the following example:

```rs
use crate::{components::editor::OpenFile, char_index::CharIndex};
```

If the current selection is selecting `use`, the following table demonstrates how many steps it takes to navigate to `OpenFile`.

| Navigation include/exclude symbols | Steps                                                                | Count |
| ---------------------------------- | -------------------------------------------------------------------- | ----- |
| Include                            | `crate` `:` `:` `{` `components` `:` `:` `editor` `:` `:` `OpenFile` | 11    |
| Exclude                            | `crate` `components` `editor` `OpenFile`                             | 4     |



<TutorialFallback filename="token"/>

[^1]: This is possible because even Prompt is an editor, so the Token mode also works there. See [Core Concepts](../../core-concepts.md#2-every-component-is-a-buffereditor)
[^1]: This is possible because even Prompt is an editor, so the Token mode also works there. See [Core Concepts](../../core-concepts.md#2-every-component-is-a-buffereditor)

## `Word`

This selects word within a word.

For example, `myOatPepperBanana` consists of 4 short word, namely: `my`, `Oat`, `Pepper` and `Banana`.

This is useful for renaming identifiers, especially if we only want to change a single word of the name. [^1]

<TutorialFallback filename="word"/>

## `Char`

Character.

In this selection mode, the movements behave like the usual editor, where [Left/Right](./../core-movements.md#--leftright) means left/right, and so on.

[First/Last](./../core-movements.md#--firstlast) means the first/last character of the current word.

<TutorialFallback filename="char"/>
