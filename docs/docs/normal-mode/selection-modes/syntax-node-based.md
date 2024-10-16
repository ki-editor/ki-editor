---
sidebar_position: 2
---

# Syntax Node-based

The following selection modes are based on the syntax node of the file, which are
powered by [Tree-sitter](https://github.com/tree-sitter).

## Syntax Node

This is one of my favourite selection mode, as it enable structural editing.

There are two Syntax Node selection modes:

- Coarse: faster movement, lower accuracy
- Fine: higher accuracy, slower movement

## Syntax Node

Keybinding: `s`

| Movement                                        | Meaning                          |
| ----------------------------------------------- | -------------------------------- |
| [Previous/Next](../core-movements.md#leftright) | Previous/Next named sibling node |
| Up                                              | Parent node                      |
| Down                                            | First named child node           |
| Current                                         | Largest node                     |
| Jump                                            | Jump to largest node             |

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

See more at [https://tree-sitter.github.io/tree-sitter/using-parsers#named-vs-anonymous-nodes](https://tree-sitter.github.io/tree-sitter/using-parsers#named-vs-anonymous-nodes).

## Fine Syntax Node

Keybinding: `S`  
Reason: Coarse is more commonly used than Fine, thus Fine is assigned a harder-to-press key.

| Movement                                        | Meaning                                          |
| ----------------------------------------------- | ------------------------------------------------ |
| [Previous/Next](../core-movements.md#leftright) | Previous/Next sibling node                       |
| Up                                              | Parent node                                      |
| Down                                            | First child                                      |
| Current                                         | Smallest node that matches the current selection |
| Jump                                            | Jump to smallest node                            |

Fine Syntax Node is useful when you start to expand the selection starting from the current token.

Suppose we have the following Javascript expression, and the current selection is `hello`, and we want to select `hello.world()`.

```js
hello.world().foo().bar().spam().wise();
```

If we press `s`, the whole expression will be selected[^1], and we will need to press `j` several times to shrink the selection down to `hello.world()`.

However, if we use `S` instead, the selection will remain as `hello`, and pressing `k` multiple times will get us to `hello.world()`.

[^1]: See [Largest Node](#largest-node)
