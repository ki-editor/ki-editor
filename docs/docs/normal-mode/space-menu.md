# Space Menu

The space menu is a handy shortcut for (not restricted to):

- Contextual actions
- File and quit actions
- Searching files/symbols
- Multi-cursor management
- Opening other components

The space menu can be brought up by pressing `space`.

## Keymap

```
╭───────────────┬───────────┬─────────────┬───────────┬──────────────┬───┬───┬────────────┬──────────────┬──────┬───╮
│               ┆           ┆             ┆           ┆              ┆ ⌥ ┆   ┆            ┆              ┆      ┆   │
│  Quit No Save ┆           ┆             ┆           ┆              ┆ ⇧ ┆   ┆            ┆              ┆      ┆   │
│ Save All Quit ┆  Save All ┆   Explorer  ┆           ┆              ┆ ∅ ┆   ┆            ┆              ┆      ┆   │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌┼╌╌╌┤
│               ┆           ┆             ┆           ┆              ┆ ⌥ ┆   ┆            ┆              ┆      ┆   │
│               ┆           ┆             ┆           ┆ Git status ^ ┆ ⇧ ┆   ┆            ┆              ┆      ┆   │
│     Theme     ┆   Symbol  ┆    Buffer   ┆    File   ┆ Git status @ ┆ ∅ ┆   ┆    Hover   ┆ Code Actions ┆ Pipe ┆   │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌┼╌╌╌┤
│               ┆           ┆             ┆           ┆              ┆ ⌥ ┆   ┆            ┆              ┆      ┆   │
│               ┆           ┆ + Replace X ┆ + Paste ← ┆              ┆ ⇧ ┆   ┆ + Change X ┆              ┆      ┆   │
│   Undo Tree   ┆ + Replace ┆    + Copy   ┆ + Paste → ┆ TS Node Sexp ┆ ∅ ┆   ┆   Rename   ┆              ┆      ┆   │
╰───────────────┴───────────┴─────────────┴───────────┴──────────────┴───┴───┴────────────┴──────────────┴──────┴───╯
```

### LSP Actions (only applicable in the main editor):

| Label          | Action                |
| -------------- | --------------------- |
| `Code Actions` | Request code actions  |
| `Hover`        | Request hover info    |
| `Rename`       | Rename current symbol |

## Pickers

| Label          | Object                                   |
| -------------- | ---------------------------------------- |
| `Buffer`       | Buffers (opened files)                   |
| `File`         | Files (Not git ignored)                  |
| `Git status @` | Git status (against current branch) [^1] |
| `Git status ^` | Git status (against main branch) [^2]    |
| `Symbol`       | LSP Symbols                              |
| `Theme`        | [Themes](../themes.md)                   |

[^1]: See more at [Git hunk](./selection-modes/secondary/index.md#hunkhunk)
[^2]: This is very useful when you want to get the modified/added files commited into the current branch that you are working on.

Searching is powered by [Helix's Nucleo](https://github.com/helix-editor/nucleo), and some [fzf](https://github.com/junegunn/fzf?tab=readme-ov-file#search-syntax)-esque search syntax works here:

| Token   | Description                                               |
| ------- | --------------------------------------------------------- |
| `sbt`   | Items that match `sbt`, for example `serbian-bear-tinker` |
| `'wild` | Items that must include `wild`                            |
| `.mp3$` | Items that end with `.mp3`                                |

Search terms can be separated by space, which means AND, and their order is unimportant.

For example, the search query `stb 'wild` matches `wild-serbian-bear-tiger` and also `stubbornly_wild`.

Also, you can use the initals to search for a file, for example, `ekl` matches `editor_keymap_legend.rs`.

Because [every component is a buffer/editor](../core-concepts.md#3-every-component-is-a-buffereditor), fuzzy search logic is also used for filtering LSP completions.

### Buffer Behavior

The buffer navigation, including the Buffer List and Previous/Next Buffer options, displays only files
that have been directly opened or edited by the user. Files that are merely displayed, such as those
from search results or diagnostic messages, do not automatically become part of the buffer list.

For example, if you rename a symbol in `file1.rs` and this causes an error in `file2.rs`, `file2.rs`
will be shown when navigating diagnostic messages. However, unless you edit `file2.rs`, it will not be
added to the buffer list. Similarly, if you search your project and view results in multiple files,
these files will not be included in the buffer list unless you edit them.

## Other components

| Label       | Action                               |
| ----------- | ------------------------------------ |
| `Explorer`  | Reveal current file in file explorer |
| `Undo Tree` | Opens the Undo Tree [^1]             |

[^1]: This is an obscure feature, although it is functional, it is hardly useful, because the undo history is too granular (character-by-character), see [undo/redo](../universal-keybindings.md#undoredo).

## System Clipboard Actions

These actions are the same as the actions [here](./actions/index.md#clipboard), except
that they uses the system clipboard instead of the editor clipboard, and their labels prefixed by `+`.

## Misc

| Label          | Meaning                                                                                         |
| -------------- | ----------------------------------------------------------------------------------------------- |
| `Pipe`         | Pipe current selection(s) to a shell command, replace the current selection(s) with the STDOUT. |
| `TS Node Sexp` | Show the Tree-sitter node S-expression of the current selection.                                |
