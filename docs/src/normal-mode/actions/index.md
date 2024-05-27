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

Copy behaves differently depending on the number of cursors.

When there is only one cursor, the content is copied to the system clipboard.

When there is more than one cursor, the selected texts of each cursor will be
copied to a cursor-specific clipboard instead.

## Paste

Keybindings:

- `p`: Paste after selection
- `P`: Paste before selection

This action pastes the content from the clipboard (either the system clipboard or
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

Assuming the current selection mode is [Syntax Tree (Coarse)](../selection-modes/syntax-tree-based.md#syntax-tree-coarse), and the current selection is `x`, pressing `d` results in the following:

```js
hello(y);
```

## Exchange

Keybindings:

- `x`: Exchange current selection with next selection
- `X`: Exchange current selection with previous selection

Memory aid: e**x**change

## Add cursor

Keybindings:

- `q`: Add cursor to the next selection
- `Q`: Add cursor to the previous selection

## Extend

Keybindings:

- `e`: Extend selection until the next selection
- `E`: Extend selection until the previous selection

This is used for grouping multiple selections into a single selection.

For example, selecting multiple words or multiple lines.

It behaves more or less the same as click-and-drag in the textbox or text area of common GUI applications, but imagine being able to tune **both** ends, unlike using a mouse where an incorrect selection means you have to start over again.

When selection extension is enabled:

1. Each selection is composed of 2 ranges (formerly 1 range).
1. There's only one moveable range at a time.
1. If the current moveable range is the right one, then pressing `E` changes the movable range to the left one without expanding the selection leftward.
1. If the current moveable range is the left one, then pressing `e` changes the movable range to the right one without expanding the selection rightward.
1. Every character between the two ranges, including the two ranges, is selected
1. Selection-wise actions work on the extended range
1. Press `ESC` to disable selection extension

## Change

Keybindings:

- `c`: Change
- `C`: Change (Cut), copies the deleted content into the clipboard

This deletes the current selected text, and enter [Insert mode
](../../modes.md#insert).

## Replace

Keybindings:

- `r`: Replace
- `R`: Replace (Cut), copies the replaced content into the clipboard

This replaces the current selected text with the copied text.

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

## Hoist

Keybinding: `h`

This is one of my favorite actions, it only works for [syntax tree](../selection-modes/syntax-tree-based.md#syntax-tree) selection modes.

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
mode is [Syntax Tree (Coarse)](../selection-modes/syntax-tree-based.md#syntax-tree-coarse), pressing `^` results in the following:

```rs
fn main() {
  println!("hello")
}
```

Notes:

- Hoist works not only for if-else expressions, it works for any syntax node
- Hoist should never cause syntax error (if it does that's a bug)
- Hoist preserve the node type of the current node

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
