# Core concepts

## 1. All selection modes are equal

Unlike other modal editors [^1], the line & column movements/actions are not given special treatment.

As mentioned by Rob Pike in [Structural Regular Expression](https://doc.cat-v.org/bell_labs/structural_regexps/se.pdf):

> The current UNIX® text processing tools are weakened by the built-in concept of a line.

In Ki, all movements must be paired by a pre-selected selection mode.

Consequently, all movements are bounded to the same keymaps.

By combining movements and selection modes, it's easy to perform any kind of movements imaginable.

## 2. Every component is a buffer/editor

This is also a core philosophy of Emacs and Vim, however in the recent modal editors such as Kakoune, Neovim and Helix, they took another approach (the standard GUI approach) where every components are different.

Although having different components greatly improves the aesthetic, it's not without disadvantages:

1. Users are forced to learn new keymaps for new components. [^2]
2. Some components are weaker than the others. [^2]
3. Reinventing the wheel everywhere. [^3]

Unlike Emacs and Vim, Ki took this approach to the extreme, literally everything is an editor, including prompt and completion dropdown.

[^1]: Vim, Neovim, Kakoune and Helix.
[^2]: For example, in Vim, `p` means paste, but to paste in prompt use `ctrl+r` instead.
[^3]: For example, in the [Helix's File Explorer PR](https://github.com/helix-editor/helix/pull/5768), every movements, including scrolling are reimplemented, although they were implemented in the Editor component.
