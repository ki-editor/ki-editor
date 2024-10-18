# Movement-action Submodes

Movement-actions are actions that have to be used with [Core Movements](./core-movements.md) and [Selection Modes](./selection-modes/index.md).

Movement-actions should be considered as the submodes of the Normal mode.

To return back to normal mode from one of these movement-action modes, press `esc`.

## 1. Exchange

Keybinding: `x`  
Memory aid: e**x**change

In the Exchange submode, every core movement means:

> exchange with \<movement\>.

For example, using the following Javascript code:

```js
f(x, 1 + 1);
```

Suppose:

- The current selection mode is [Syntax Node][1]
- The current selection is `x`
- The current submode is Exchange

... then executing [Next][2] swaps the first argument of `f` with its second argument:

```js
f(1 + 1, x);
```

### Tips

Since exchange works with every core movement, it can be used with [Jump](./core-movements.md#jump) and [Syntax Node][1] to swap two distant expressions.

For example, using the following Rust code:

```rs
if x > 0 {
  println!("Yes")
}
else {
  x += 1;
  println!("no")
}
```

...we can swap the body of the if-else expression by:

1. Set selection mode to [Syntax Node][1] by pressing `s`
2. Jump to the body of `if` by pressing `f {`, then press the letter that appears on top of the first `{`.
3. Enter Exchange submode by pressing `x`
4. Press `f {`, then press the letter that appears on top of the second `{`
5. Done

## 2. Multi-cursor

Keybinding: `q`  
Reason: `q` is used to start recording a macro in Vim, but I realized 80% of the time what I need is multi-cursors, not a macro.

In the Multi-cursor submode, every core movement means:

> Add cursor with \<movement\>

Use the following text as an example:

```txt
hello ki, hello vim, hello helix
```

Suppose:

- The current selection mode is [Find Literal "hello"](./selection-modes/local-global/text-search.md#1-literal)
- The current selection is the first `hello`
- The current submode is Multi-cursor

... then executing [Next][2] adds a new cursor to the second `hello`.

## 3. Replace (to be removed)

In the Replace submode, every core movement means:

> Replace current selection until \<movement\>

Unlike [Exchange](#1-exchange) and [Multi-cursor](#2-multi-cursor), this submode is not essential, it is a kind of shortcut for certain operations.

Suppose you have the following text:

```js
"hello     world";
```

and wish to turn it to:

```js
"hello";
```

In this case, you can select `world`, [Change](./actions/index.mdx#change) it, and press `backspace` 5 times.

However, that's inefficient, and that can be shortened by:

1. Select `hello`
1. Set selection mode to [Word](./selection-modes/regex-based.mdx#word)
1. [Copy](./actions/clipboard-related-actions.md#copy)
1. [Enable selection extension](../v-mode#extending-selection)
1. Move to `world` (by pressing `l`)
1. [Replace](./actions/clipboard-related-actions.md#replace)

And seeing that Steps 3 to 6 is a common chore, the Replace mode is actually a shortcut for that.

Here's how it works using the Replace mode (starting from step 3):

3. Enter the Replace submode
4. Press `l`
5. Press `esc` to return to Normal mode

The rigorous readers might have noticed the similarity of the Replace submode
with the [Raise](./actions/index.mdx#raise) action, that is in fact the case,
under the hood, Raise is but a specialized version of the Replace mode which
only executes the [Up](./core-movements.md#updown) movement.

[1]: ./selection-modes/syntax-node-based.md#syntax-node
[2]: ./core-movements.md#leftright
