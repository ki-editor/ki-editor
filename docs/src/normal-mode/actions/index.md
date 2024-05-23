# Actions

## Notes for reading

1. When "selection" is mentioned, you should read it as "selection(s)", because
   these actions work with multi-cursors.

## Enter [insert mode](../../insert-mode/index.md)

Keybindings:

- `i`: Enter insert mode before selection
- `a`: Enter insert mode after selection

## Copy

Keybinding: `y`  
Memory aid: y stands for yank, yank to the clipboard.

This action copies the current selected text.

Copy behaves different depending on the number of cursor.

When there is only one cursor, the content is copied to the system clipboard.

When there is more than one cursors, the selected texts of each cursor will be
copied to a cursor-specific clipboard instead.

## Paste

Keybindings:

- `p`: Paste after selection
- `P`: Paste before selection

This action paste the content from the clipboard (either system clipboard or
cursor-specific clipboard) after/before the current selection.

Notes:

- It does not replace the current selection.
- The pasted text will be selected.

### Smart Paste

When the selection mode is [contiguous](../selection-modes/index.md#contiguity), Smart Paste will be executed.

Smart Paste works by analyzing the gap between the current selection and the
previous/next selection, then insert the gap before/after the pasted text.

For example, consider the following Javascript code:

```js
hello(x, y);
```

Assuming the current selection mode is [Syntax Tree (Coarse)](../selection-modes/syntax-tree-based.md#syntax-tree-coarse), and the current selection is `y`, and the
copied text is `z`, performing a `p` results in the following:

```js
hello(x, y, z);
```

## Open

Keybindings:

- `o`: Open after selection
- `O`: Open before selection

If the current selection mode is **not** [contiguous](../selection-modes/index.md#contiguity), then `o` behaves like `a`, and `O` behaves like `i`.

Otherwise, it inserts a gap before/after the current selection, and enter Insert mode.

For example, consider the following Javascript code:

```js
hello(x, y);
```

Assuming the current selection mode is [Syntax Tree (Coarse)](../selection-modes/syntax-tree-based.md#syntax-tree-coarse), and the current selection is `y`, pressing `o` results in the following (Note that `│` represents the cursor):

```js
hello(x, y, │);
```

## Delete

Keybindings:

- `d`: Delete
- `D`: Delete (Cut), copies the deleted content into clipboard

This deletes the current selection(s), however, if the current selection mode is
[contiguous](../selection-modes/index.md#contiguity), it will delete until the
next selection, and selects the next selection.

But, if the current selection is the last selection, it will delete until the
previous selection instead, and selects the previous selection.

For example, consider the following Javascript code:

```js
hello(x, y);
```

Assuming the current selection mode is [Syntax Tree (Coarse)](../selection-modes/syntax-tree-based.md#syntax-tree-coarse), and the current selection is `x`, pressing `d` results in the following:

```js
hello(y);
```

## Change

Keybindings:

- `c`: Change
- `C`: Change (Cut), copies the deleted content into clipboard

This is similar to [Delete](#delete), however, after deleting the current
selected text, the editor will enter [Insert mode](../modes.md#insert).

## Replace

Keybindings:

- `r`: Replace
- `R`: Replace (Cut), copies the replaced content into clipboard

This replaces the current selected text with the copied text.

## Replace with pattern

Keybinding: `ctrl+r`

This replaces the current selection using the search pattern and replacement
pattern specified in the [Text Search Configurator](../selection-modes/native-global/text-search.md#configurator).

For example:

| Mode          | Selected text | Search   | Replacement | Result  |
| ------------- | ------------- | -------- | ----------- | ------- |
| Literal       | `f`           | `f`      | `g`         | `g(x)`  |
| Regex         | `"yo"`        | `"(.*)"` | `[$1]`      | `[yo]`  |
| AST Grep      | `f(x)`        | `f($Z)`  | `$Z(f)`     | `x(f)`  |
| Case Agnostic | `a_bu`        | `a bu`   | `to li`     | `to_li` |

## Raise

Keybinding: `^`

This is one of my favourite action, it only works for [syntax tree](../selection-modes/syntax-tree-based.md#syntax-tree) selection modes.

This replace the parent node of the current node, with the current node.

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
mode is [Syntax Tree (Coarse)](/docs/src/selection-modes/syntax-tree-based.md#syntax-tree-coarse), pressing `^` results in the following:

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

After formatting, the [Current](/docs/src/normal-mode/core-movements.md#current) movement will be executed, to reduce disorientation caused by the misplaced selection due to content changes.
