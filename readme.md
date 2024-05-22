# Tim Editor

> **Warning**
> This editor is not stable yet, in particular, the keybindings are not fully settled yet.

## Showcase

[Click here](./showcase.md)

## Disclaimer

This documentation does not and will not in the future include features that are not yet implemented.

## Core concepts

### 1. Movable objects are movable using the same set of keybindings

Almost all kinds of movement in Tim have can be moved based on the same set of keybindings.

This results in a consistency.

For example, in Vim, different moveable objects has very different set of keybindings.

| Object             | Previous | Next   | Relationship between keybindings      |
| ------------------ | -------- | ------ | ------------------------------------- |
| Word               | b        | e      | None                                  |
| Tab                | gT       | gt     | Letter casing                         |
| Jumps              | Ctrl+o   | Ctrl+i | None                                  |
| Paragraph          | {        | }      | Opening and closing                   |
| Line               | k        | j      | Next to each other on QWERTY keyboard |
| Git hunk           | [c       | ]c     | Opening and closing                   |
| Quickfix list item | :cnext   | :cprev | Command wording                       |

By looking at the relationships, we can see that almost each sets of
keybindings are defined by different types of relationships, if any.

Because of the randomness, memorizing is hard, discovering new keybindings are virtually impossible.

Tim addresses this issue by introducing a submode in the Normal mode, where you
have to set the current selection mode (a.k.a. moveable object type) first
before moving, and moving across different object uses the same set of keybindings, namely:

- n (next)
- p (previous)
- u (up)
- d (down)
- j (jump, similar to Vim's quickmotion, but better)

### 2. First-class structural navigation and editing

None of the modal editors mentioned above supports first-class structural navigation and editing.

Tim supports this by having the following motion/edit:

- Go to parent/child node
- Go to sibling nodes
- Move by token
- Select outermost node
- Raise (replace parent node with current node)

### 3. Modifier keys sucks

Inspired by Emacs God Mode, most keybindings in Tim does not require pressing any modifier keys such as Shift, Control, Alt and etc.

Instead, Tim expect the user to press a key combo. For example, to set the current selection mode as Diagnostics (Error), press `e` twice.

### 4. First-class multi-cursor

### 5. First-class LSP

### 6. Every component is an Editor

Similar to Emacs and Vim, all components are just editor, be it dropdown, info menu, prompt, file explorer and etc.

It means that all of the keybindings you've learnt work in these components, except for those extra components-specific keybindings.

### 7. Minimal config

I have spent countless hours modding my Neovim and debugging package issues,
and I had enough of that. Thus, Tim will be super uncustomizable so that you do
not need to go through the pain of configuration over and over again.

So Tim is not only an editor, it's also like an editor distro such as NvChad, Doom Emacs and etc.
