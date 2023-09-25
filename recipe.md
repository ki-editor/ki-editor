# Recipe
## LUO 
`l u o` to select the current body

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
you cannot just raise `123`, as that would do nothing, since replace the block with `123` does not make sense.

To make that work, wrap `123` with curly bracket (by pressing `{`) before raising.
