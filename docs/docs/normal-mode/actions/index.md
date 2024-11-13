import {TutorialFallback} from '@site/src/components/TutorialFallback';

# Actions

## Notes for reading

1. When "selection" is mentioned, you should read it as "selection(s)", because
   these actions work with multiple cursors.

## Enter [insert mode](../../insert-mode/index.md)

Keybindings:

- `i`: Enter insert mode before selection
- `a`: Enter insert mode after selection

## Open

Keybindings:

- `o`: Open after selection
- `O`: Open before selection

If the current selection mode is not Syntax Node,
then `o`/`O` inserts a newline with the respective indent after/before the current line.

Otherwise, it inserts a gap before/after the current selection, and enter Insert mode.

<TutorialFallback filename="open"/>

## Delete

Keybindings:

- `d`: Delete until next selection
- `D`: Delete until previous selection

This deletes the current selection(s), however, if the current selection mode is
[contiguous](../selection-modes/index.md#contiguity), it will delete until the
next/previous selection, and selects the next/previous selection.

But, if the current selection is the last/first selection, it will delete until the
previous/next selection instead, and selects the previous/next selection.

For example, consider the following Javascript code:

```js
hello(x, y);
```

Assuming the current selection mode is [Syntax Node](../selection-modes/syntax-node-based.md#syntax-node), and the current selection is `x`, pressing `d` results in the following:

```js
hello(y);
```

## Change

Keybindings:

- `c`: Change

This deletes the current selected text, and enter [Insert mode
](../../insert-mode/index.md).

## Replace with previous/next copied text

Keybindings:

- `ctrl+n`: Replace current selection with next copied text in the clipboard history
- `ctrl+p`: Replace current selection with previous copied text in the clipboard history

This is similar to [Yanking Earlier Kills](https://www.gnu.org/software/emacs/manual/html_node/emacs/Earlier-Kills.html) in Emacs.

This is useful when you want to retrieve earlier copies.

## Replace with pattern

Keybinding: `ctrl+r`

This replaces the current selection using the search pattern and replacement
pattern specified in the [Text Search Configuration](../selection-modes/local-global/text-search.md#configuration).

For example:

| Mode                       | Selected text | Search   | Replacement | Result  |
| -------------------------- | ------------- | -------- | ----------- | ------- |
| Literal                    | `f`           | `f`      | `g`         | `g(x)`  |
| Regex                      | `"yo"`        | `"(.*)"` | `[$1]`      | `[yo]`  |
| AST Grep                   | `f(x)`        | `f($Z)`  | `$Z(f)`     | `x(f)`  |
| Naming Convention Agnostic | `a_bu`        | `a bu`   | `to li`     | `to_li` |

<TutorialFallback filename="replace-with-pattern"/>

## Raise

Keybinding: `T`

This is one of my favorite actions, it only works for [syntax node](../selection-modes/syntax-node-based.md#syntax-node) selection modes.

This replaces the parent node of the current node, with the current node.

<TutorialFallback filename="raise"/>

Note: Raise should never cause any syntax errors, if it does that's a bug.

## Join

Keybinding: `J`

Joins multiple lines within the current selection(s) into a single line.

<TutorialFallback filename="join"/>

## Break

Keybinding: `K`

Break the current selection(s) to the next line, with the indentation of the current line.

This is a shortcut of `i enter esc`.

<TutorialFallback filename="break"/>

## Transform

Keybinding: `!`

Transformative actions are nested under here, such as (non-exhaustive):

- `w`: Wrap (Wrap current selection into multiple lines)
- `l`: Convert to `lower case`
- `s`: Convert to `snake_case`

## Save

Keybinding: `enter`  
Reason: The `esc enter` combo is sweet.

Upon saving, formatting will be applied if possible.

After formatting, the [Current](../core-movements.mdx#current) movement will be executed, to reduce disorientation caused by the misplaced selection due to content changes.

## Undo/Redo

Keybindings:

- `u`: Undo
- `U`: Redo

Notes:

1. Undo/redo works for multi-cursors as well
2. The current implementation is naive, it undoes/redoes character-by-character, instead of chunk-by-chunk, so it can be mildly frustrating
