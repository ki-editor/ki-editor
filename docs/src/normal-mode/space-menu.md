# Space Menu

The space menu is a handy shortcut for (not restricted to):

- Contextual actions
- Searching files/symbols
- Multi-cursor management
- Opening other components

The space menu can be brought up by pressing `space`.

## Contextual actions

Contextual actions are actions that are only applicable within a specific context.

LSP Actions (only applicable in main editor):

| Keybinding | Action                |
| ---------- | --------------------- |
| `c`        | Request code actions  |
| `h`        | Request hover info    |
| `r`        | Rename current symbol |

[File Explorer](../components/file-explorer.md) Actions (only applicable in File Explorer):

| Keybinding | Action                                           |
| ---------- | ------------------------------------------------ |
| `a`        | Add a new file/folder under the current path[^1] |
| `d`        | Delete current file/folder                       |
| `m`        | Move (or rename) the current file/folder[^2]     |
| `r`        | Refresh the file explorer[^3]                    |

[^1]: To add folder, append `/` to the file name. Can be nested, new directories will be created as required.
[^2]: Works like `mkdir -p`, it will create new directories when required.
[^3]: This is necessary sometimes because the file system is modified by external factors, and Ki do not watch for file changes.

## Searching files/symbols

| Keybinding | Action                  |
| ---------- | ----------------------- |
| b          | Buffers (opened files)  |
| f          | Files (Not git ignored) |
| g          | Git status [^1]         |
| s          | LSP Symbols             |

[^1]: It means files that are modified when compared to their latest version found on the latest commit of this Git repository.

Searching is powered by [Helix's Nucleo](https://github.com/helix-editor/nucleo), and some [fzf](https://github.com/junegunn/fzf?tab=readme-ov-file#search-syntax)-esque search syntax works here:

| Token   | Description                                               |
| ------- | --------------------------------------------------------- |
| `sbt`   | Items that match `sbt`, for example `serbian-bear-tinker` |
| `'wild` | Items that must include `wild`                            |
| `.mp3$` | Items that end with `.mp3`                                |

Search terms can be separated by space, which means AND, and their order is unimportant.

For example, the search query `stb 'wild` matches `wild-serbian-bear-tiger` and also `stubbornly_wild`.

Because [every component is a buffer/editor](/docs/src/core-concepts.md#2-every-component-is-a-buffereditor), fuzzy search logic is also used for filtering LSP completions.

## Multi-cursor

| Keybinding | Action                                                               |
| ---------- | -------------------------------------------------------------------- |
| `a`        | Add cursor to all selections in the current [selection mode][1] [^1] |
| `o`        | Keeps **o**nly the primary selections                                |

[1]: ./selection-modes/index.md

[^1]: Especially useful when used with [Text Search](./selection-modes/native-global/text-search.md) or [Syntax Tree](./selection-modes/syntax-tree-based.md).

## Opening other components

| Keybinding | Action                                   |
| ---------- | ---------------------------------------- |
| `e`        | Reveal current file in file **e**xplorer |
| `z`        | Opens the Undo Tree [^1]                 |

[^1]: This is an obscure feature, although it is functional, it is hardly useful, because the undo history is too granular (character-by-character), see [undo/redo](../universal-keybindings.md#undoredo).