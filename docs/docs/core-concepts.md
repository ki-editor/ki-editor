---
sidebar_position: 4
---

# Core concepts

## 1. All selection modes are equal

Unlike other modal editors [^1], the line & column movements/actions are not given special treatment.

As mentioned by Rob Pike in [Structural Regular Expression](https://doc.cat-v.org/bell_labs/structural_regexps/se.pdf):

> The current UNIX® text processing tools are weakened by the built-in concept of a line.

In Ki, all movements must be paired by a pre-selected selection mode.

Consequently, all movements are bound to the same keymaps.

By combining movements and selection modes, it's easy to perform any kind of movements imaginable.

## 2. Positional Keymaps

The keymaps of Ki are strictly positional, meaning they no longer rely on mnemonics (for example, `p` for Put in Vim).

This entails:

### A. Keyboard Layout Agnostic

No matter which keyboard layout you use, be it QWERTY, Dvorak, Colemak, etc., the keymap of Ki remains unchanged.

### B. Bigram Optimization

Because we are no longer bound by mnemonics, we can optimize common bigrams using either Colemak's Rolling
or Dvorak's Hand Alternation.

For example, copying and pasting the current selection is done by pressing `c` then `b` on QWERTY.

### C. Positional Coherence

Actions with similar meanings are placed in the same position across the shift, alt, or menu layers.

For example, the actions on the position of `f` on QWERTY roughly relate to the concept of "Change".

| Mode          | Meaning         |
| ------------- | --------------- |
| Normal        | Change          |
| File Explorer | Rename File     |
| Extend        | Change Surround |
| Space Menu    | LSP Rename      |

### D. Travel Distance Optimization

The placement of actions are also guided by their ubiquity, more commonly used actions
will placed on better positions such as the homerow.

## 3. Every component is a buffer/editor

This is also a core philosophy of Emacs and Vim, however in the recent modal editors such as Kakoune, Neovim, and Helix, they took another approach (the standard GUI approach) where every component is different.

Although having different components greatly improves the aesthetic, it's not without disadvantages:

1. Users are forced to learn new keymaps for new components. [^2]
2. Some components are weaker than others.
3. Reinventing the wheel everywhere. [^3]

Unlike Emacs and Vim, Ki took this approach to the extreme, literally everything is an editor, including prompt and completion dropdown.

## 4. Minimal configurations

Part of the reason why Ki was created is due to the configuration nightmare that I have been through when using Neovim for the past 4 years.

Thus, I'm in favor of minimal configurations, users should not spend eons configuring something simple (which makes Helix attractive).

That being said, the following components should be configurable:

1. Theme
2. Language-specific configurations:
   - Formatter
   - Tree-sitter grammar
   - LSP

## 5. Keybindings synergy

Most keybindings in Ki synergize with one another, though a minority of them are lone rangers.

But lone rangers are not encouraged, they are only added if they are truly crucial.

[^1]: Vim, Neovim, Kakoune and Helix.

[^2]: For example, in Vim, `p` means paste, but to paste in prompt use `ctrl+r` instead.

[^3]: For example, in the [Helix's File Explorer PR](https://github.com/helix-editor/helix/pull/5768), every movement, including scrolling was reimplemented, although they were implemented in the Editor component.
