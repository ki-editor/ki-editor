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

If the current selection mode is **not** [contiguous](../selection-modes/index.md#contiguity),
then `o`/`O` inserts one space after/before the current
selection.

Otherwise, it inserts a gap before/after the current selection, and enter Insert mode.

For example, consider the following Javascript code:

```js
hello(x, y);
```

Assuming the current selection mode is [Syntax Node (Coarse)](../selection-modes/syntax-node-based.md#syntax-node-coarse), and the current selection is `y`, pressing `o` results in the following (Note that `│` represents the cursor):

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

Assuming the current selection mode is [Syntax Node (Coarse)](../selection-modes/syntax-node-based.md#syntax-node-coarse), and the current selection is `x`, pressing `d` results in the following:

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
mode is [Syntax Node (Coarse)](../selection-modes/syntax-node-based.md#syntax-node-coarse), pressing `^` results in the following:

```rs
fn main() {
  println!("hello")
}
```

Notes:

- Raise works not only for if-else expressions, it works for any syntax node
- Raise should never cause syntax error (if it does that's a bug)
- Raise preserve the node type of the current node

## Between

Keybinding: `b`

This is a group of actions that is related to "surround" or "enclosures".

| Keybinding | Action                                |
| ---------- | ------------------------------------- |
| `a<x>`     | Select around `<x>`                   |
| `i<x>`     | Select inside `<x>`                   |
| `d<x>`     | Delete surrounding `<x>`              |
| `c<x><y>`  | Change surrounding `<x>` to `<y>`     |
| `<x>`      | Surround current selection with `<x>` |

`<x>` can be one of the following:

- `(` Parenthesis
- `{` Curly Brace
- `[` Square Bracket
- `<` Angular Bracket
- `'` Single Quote
- `"` Double Quote
- <code>`</code> Backtick

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

## Visual

Keybinding: `v`

This should also be known as "enable selection extension", it got its name from Vim's Visual mode.

This is used for grouping multiple selections into a single selection.

For example, selecting multiple words or multiple lines.

It behaves more or less the same as click-and-drag in the textbox or text area of common GUI applications, but imagine being able to tune **both** ends, unlike using a mouse where an incorrect selection means you have to start over again.

When selection extension is enabled:

1. Each selection is composed of 2 ranges (formerly 1 range).
1. There's only one moveable range at a time.
1. Press `v` again to change the moveable range to the other range.
1. Every character between the two ranges, including the two ranges, is selected
1. Selection-wise actions work on the extended range
1. Press `ESC` to disable selection extension
