# Actions

## Notes for reading

1. When "selection" is mentioned, you should read it as "selection(s)", because
   these actions work with multiple cursors.

## Select Parent / Extend Selection

Keybinding: `t`  
Memory aid: "t" stands for Top

## Select First Child / Shrink Selection

Keybinding: `b`  
Memory aid: "b" stands for Bottom

## Enter [insert mode](../../insert-mode/index.md)

Keybindings:

- `i`: Enter insert mode before selection
- `a`: Enter insert mode after selection

## Open

Keybindings:

- `o`: Open after selection
- `O`: Open before selection

If the current selection mode is **not** [contiguous](../selection-modes/index.md#contiguity),
then `o`/`O` inserts one space after/before the current
selection.

Otherwise, it inserts a gap before/after the current selection, and enter Insert mode.

For example, consider the following Javascript code:

```js
hello(x, y);
```

Assuming the current selection mode is [Syntax Node](../selection-modes/syntax-node-based.md#syntax-node), and the current selection is `y`, pressing `o` results in the following (Note that `│` represents the cursor):

```js
hello(x, y, │);
```

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
](../../modes.md#insert).

## Replace with previous/next copied text

Keybindings:

- `ctrl+n`: Replace current selection with next copied text in the clipboard history
- `ctrl+p`: Replace current selection with previous copied text in the clipboard history

This is similar to [Yanking Earlier Kills](https://www.gnu.org/software/emacs/manual/html_node/emacs/Earlier-Kills.html) in Emacs.

This is useful when you want to retrieve earlier copies.

## Replace with pattern

Keybinding: `ctrl+r`

This replaces the current selection using the search pattern and replacement
pattern specified in the [Text Search Configurator](../selection-modes/local-global/text-search.md#configurator).

For example:

| Mode          | Selected text | Search   | Replacement | Result  |
| ------------- | ------------- | -------- | ----------- | ------- |
| Literal       | `f`           | `f`      | `g`         | `g(x)`  |
| Regex         | `"yo"`        | `"(.*)"` | `[$1]`      | `[yo]`  |
| AST Grep      | `f(x)`        | `f($Z)`  | `$Z(f)`     | `x(f)`  |
| Case Agnostic | `a_bu`        | `a bu`   | `to li`     | `to_li` |

## Raise

Keybinding: `^`

This is one of my favorite actions, it only works for [syntax node](../selection-modes/syntax-node-based.md#syntax-node) selection modes.

This replaces the parent node of the current node, with the current node.

For example, with the following Rust code:

```rs
fn main() {
  if x > 0 {
    println!("hello")
  }
  else {
    panic!();
  }
}

```

Assuming the current selection is `println!("hello")` and the current selection
mode is [Syntax Node](../selection-modes/syntax-node-based.md#syntax-node), pressing `^` results in the following:

```rs
fn main() {
  println!("hello")
}
```

Notes:

- Raise works not only for if-else expressions, it works for any syntax node
- Raise should never cause syntax error (if it does that's a bug)
- Raise preserve the node type of the current node

## Transform

Keybinding: `!`

Transformative actions are nested under here, such as (non-exhaustive):

- `j`: Join (Joins current selection into a single line)
- `w`: Wrap (Wrap current selection into multiple lines)
- `l`: Convert to `lower case`
- `s`: Convert to `snake_case`

## Save

Keybinding: `enter`  
Reason: The `esc enter` combo is sweet.

Upon saving, formatting will be applied if possible.

After formatting, the [Current](../core-movements.md#current) movement will be executed, to reduce disorientation caused by the misplaced selection due to content changes.

## Undo/Redo

Keybindings:

- `u`: Undo
- `U`: Redo

Notes:

1. Undo/redo works for multi-cursors as well
2. The current implementation is naive, it undoes/redoes character-by-character, instead of chunk-by-chunk, so it can be mildly frustrating
