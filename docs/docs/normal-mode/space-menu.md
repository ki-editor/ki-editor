# Space Menu

The space menu is a handy shortcut for (not restricted to):

- Contextual actions
- File and quit actions
- Searching files/symbols
- Multi-cursor management
- Opening other components

The space menu can be brought up by pressing `space`.

## Contextual actions

Contextual actions are actions that are only applicable within a specific context.

## File and quit actions

| Keybinding | Action                                           |
| ---------- | ------------------------------------------------ |
| `space`    | Write current file, even if no changes were made |
| `w`        | Write all files                                  |
| `q`        | Write all files and quit                         |
| `Q`        | Quit _without_ writing unsaved files             |

### LSP Actions (only applicable in the main editor):

| Keybinding | Action                |
| ---------- | --------------------- |
| `c`        | Request code actions  |
| `h`        | Request hover info    |
| `r`        | Rename current symbol |

### File Explorer Actions:

| Keybinding | Action                                                           |
| ---------- | ---------------------------------------------------------------- |
| `a`        | Add a new file/folder under the current path [^1]                |
| `c`        | Copy current file to a new path                                  |
| `d`        | Delete current file/folder                                       |
| `m`        | Move (or rename) the current file/folder [^2]                    |
| `r`        | Refresh the [file explorer](../components/file-explorer.md) [^3] |

[^1]: To add a folder, append `/` to the file name. Can be nested, and new directories will be created as required.
[^2]: Works like `mkdir -p`, it will create new directories when required.
[^3]: This is necessary sometimes because the file system is modified by external factors, and Ki does not watch for file changes.

## Pickers

| Keybinding | Object                                   |
| ---------- | ---------------------------------------- |
| b          | Buffers (opened files)                   |
| f          | Files (Not git ignored)                  |
| g          | Git status (against current branch) [^1] |
| G          | Git status (against main branch) [^2]    |
| s          | LSP Symbols                              |
| t          | Themes                                   |

[^1]: See more at [Git hunk](./selection-modes/local-global/misc.md#git-hunk)
[^2]: This is very useful when you want to get the modified/added files commited into the current branch that you are working on.

Searching is powered by [Helix's Nucleo](https://github.com/helix-editor/nucleo), and some [fzf](https://github.com/junegunn/fzf?tab=readme-ov-file#search-syntax)-esque search syntax works here:

| Token   | Description                                               |
| ------- | --------------------------------------------------------- |
| `sbt`   | Items that match `sbt`, for example `serbian-bear-tinker` |
| `'wild` | Items that must include `wild`                            |
| `.mp3$` | Items that end with `.mp3`                                |

Search terms can be separated by space, which means AND, and their order is unimportant.

For example, the search query `stb 'wild` matches `wild-serbian-bear-tiger` and also `stubbornly_wild`.

Because [every component is a buffer/editor](../core-concepts.md#2-every-component-is-a-buffereditor), fuzzy search logic is also used for filtering LSP completions.

### Buffer Behavior

The buffer navigation, including the Buffer List and Previous/Next Buffer options, displays only files
that have been directly opened or edited by the user. Files that are merely displayed, such as those
from search results or diagnostic messages, do not automatically become part of the buffer list.

For example, if you rename a symbol in `file1.rs` and this causes an error in `file2.rs`, `file2.rs`
will be shown when navigating diagnostic messages. However, unless you edit `file2.rs`, it will not be
added to the buffer list. Similarly, if you search your project and view results in multiple files,
these files will not be included in the buffer list unless you edit them.

## Opening other components

| Keybinding | Action                                   |
| ---------- | ---------------------------------------- |
| `e`        | Reveal current file in file **e**xplorer |
| `z`        | Opens the Undo Tree [^1]                 |

[^1]: This is an obscure feature, although it is functional, it is hardly useful, because the undo history is too granular (character-by-character), see [undo/redo](../universal-keybindings.md#undoredo).

## Picking themes

See more at [Themes](../themes.md)
