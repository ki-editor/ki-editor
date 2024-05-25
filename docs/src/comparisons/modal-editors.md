# Comparisons with other modal editors

| Aspect                      | Ki                                 | Vim/Neovim         | Helix              |
| --------------------------- | ---------------------------------- | ------------------ | ------------------ |
| Mental model                | Selection mode → Movement → Action | Action → Selection | Selection → Action |
| Structural editing          | First-class                        | With plugin        | Second class[^1]   |
| Multi-cursor                | Good                               | With plugin        | Extensive          |
| Built-in file explorer      | Yes                                | Yes (but buggy)    | No                 |
| Built-in LSP                | Yes                                | Require config     | Yes                |
| Performance                 | Good                               | Great              | Fastest            |
| Keybindings coherence       | High                               | Low                | Low                |
| Multi-width Unicode support | Yes                                | Yes                | Yes                |
| GNU Readline Support        | Everywhere [^2]                    | Minimal            | Inconsistent [^3]  |

[^1]:
    The default keybindings for structural navigation in Helix are hard to access: `alt+n`, `alt+p`, `alt+i` and `alt+o`.
    Also, there's no easy way to [revert to previous selection](../normal-mode/other-movements.md#go-backforward), which is crucial for structural manipulation.

[^2]: Not all GNU Readline keybindings are implemented, but they are welcomed.
[^3]: Extensive support in Prompt, but minimal support in Editor.

## Keybindings coherence

Coherence means the quality of being logical and consistent.

Ki keybindings are exceptionally coherent due to its mental model.

The following table demonstrates the incoherence of Vim keybindings:

| Selection mode / Action      | Next      | Previous |
| ---------------------------- | --------- | -------- |
| Word                         | `e` / `w` | `b`      |
| Long word                    | `E` / `W` | `B`      |
| Search matches               | `n`       | `N`      |
| Line                         | `j`       | `k`      |
| Column                       | `l`       | `h`      |
| Paragraph (empty lines)      | `}`       | `{`      |
| Git hunk [^4]                | `]c`      | `[c`     |
| One character                | `f`/`t`   | `F`/`T`  |
| Repeat latest `f`/`t` motion | `;`       | `,`      |
| Quickfix                     | `:cnext`  | `:cprev` |
| Search current word          | `*`       | `#`      |

The following table demonstrates the incoherence of Helix keybindings [^5]:

| Selection mode / Action | Next    | Previous |
| ----------------------- | ------- | -------- |
| Sibling node            | `alt+n` | `alt+p`  |
| Add cursor (line-wise)  | `C`     | `alt+c`  |
| Extend line             | `x`     | None     |
| LSP Diagnostics         | `]d`    | `[d`     |

As you can see, there's no single logical categorization for these keymaps, they are either lowercase-uppercase, normal-alt, left-right bracket, or outright unexplainable.

> In Ki, all of these boil down to `h` and `l` only!

You only have to memorize the movement keybindings once, and then the selection mode keybindings, and you will be able to explore new ways of navigation on your own.

Once you've learned the Ki keybindings, it's tough to look back (at least for me).

Note that the Ki keybindings cannot be simply implemented in Vim/Helix via key-
remapping, due to the lack of the concept of [Selelection Mode][1], and implementing
that requires major architectural changes in the core.

[^4]: With [vim-unimpaired](https://github.com/tpope/vim-unimpaired).
[^5]: Keybindings inherited from Vim are omitted.

[1]: ../normal-mode/selection-modes/index.md
