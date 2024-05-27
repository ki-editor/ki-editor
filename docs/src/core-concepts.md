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

[^1]: Vim, Neovim, Kakoune and Helix.
[^2]: For example, in Vim, `p` means paste, but to paste in prompt use `ctrl+r` instead.
[^3]: For example, in the [Helix's File Explorer PR](https://github.com/helix-editor/helix/pull/5768), every movement, including scrolling was reimplemented, although they were implemented in the Editor component.

## 3. Minimal configurations

Part of the reason why Ki was created is due to the configuration nightmare that I have been through when using Neovim for the past 4 years.

Thus, I'm in favor of minimal configurations, users should not spend days configuring something simple (which makes Helix attractive).

However, unlike Helix editor, I will take this step even further: _keybindings should not be configurable_.

A ridiculous amount of time is spent thinking about keybindings: their coherence, their rationality, their ease of typing etc.

Thus, if you feel like certain keybindings should be remapped, discuss them with me, and we will sort it out, I want every Ki user to experience the best keybindings that they ever had.

That being said, the following components should be configurable:

1. Theme
2. Language-specific configurations:
   - Formatter
   - Tree-sitter grammar
   - LSP

## 4. Keybindings synergy

Most keybindings in Ki synergize with one another, though a minority of them are lone rangers.

But lone rangers are not encouraged, they are only added if they are truly crucial.
