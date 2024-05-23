# Recipes

## Raise current expression in a block (Rust)

Say we have the following code:

```rs
fn main() -> usize {
  let x = 123;
  Some(x)
}
```

If you want to change the code to become:

```rs
fn main() -> usize {
  123
}
```

... you cannot just [Raise](./normal-mode/actions/index.md#raise) `123`, as that would do nothing, since replacing the block with `123` introduces syntax error.

To make that work, surround `123` with curly bracket (by pressing `b {` [^1]) before raising.

[^1]: See [Between](./normal-mode/actions/index.md#between)
