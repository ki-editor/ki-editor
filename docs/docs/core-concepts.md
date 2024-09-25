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

## 2. Every component is a buffer/editor

This is also a core philosophy of Emacs and Vim, however in the recent modal editors such as Kakoune, Neovim, and Helix, they took another approach (the standard GUI approach) where every component is different.

Although having different components greatly improves the aesthetic, it's not without disadvantages:

1. Users are forced to learn new keymaps for new components. [^2]
2. Some components are weaker than others.
3. Reinventing the wheel everywhere. [^3]

Unlike Emacs and Vim, Ki took this approach to the extreme, literally everything is an editor, including prompt and completion dropdown.

## 3. Minimal configurations

Part of the reason why Ki was created is due to the configuration nightmare that I have been through when using Neovim for the past 4 years.

Thus, I'm in favor of minimal configurations, users should not spend eons configuring something simple (which makes Helix attractive).

That being said, the following components should be configurable:

1. Theme
2. Language-specific configurations:
   - Formatter
   - Tree-sitter grammar
   - LSP
3. Keybindings

## 4. Vim keybindings-compatible

This is because I do not want to alienate existing modal editor users, where most of their keybindings are also based on Vim.

A significant portions of Ki's keybindings are based on Vim's keybindings, but repurposed.

For example, hjkl is also part of Ki, however, their meaning has been generalized to not only work for lines and columns, as mentioned above.

## 5. Keybindings synergy

Most keybindings in Ki synergize with one another, though a minority of them are lone rangers.

But lone rangers are not encouraged, they are only added if they are truly crucial.

[^1]: Vim, Neovim, Kakoune and Helix.
[^2]: For example, in Vim, `p` means paste, but to paste in prompt use `ctrl+r` instead.
[^3]: For example, in the [Helix's File Explorer PR](https://github.com/helix-editor/helix/pull/5768), every movement, including scrolling was reimplemented, although they were implemented in the Editor component.
