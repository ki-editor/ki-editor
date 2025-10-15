import {TutorialFallback} from '@site/src/components/TutorialFallback';
import {KeymapFallback} from '@site/src/components/KeymapFallback';

# Space Menu

The space menu contains two major keymaps:

1. Left
2. Right

The Left side contains the keymap of [Global secondary selection modes](docs/normal-mode/selection-modes/secondary/index.md).

The right side is a handy shortcut for (not restricted to):

- Contextual actions
- File and quit actions
- Searching files/symbols
- Multi-cursor management
- Opening other components

The space menu can be brought up by pressing `space`.

## Keymap

<KeymapFallback filename="Space"/>

## LSP Actions (only applicable in the main editor):

<KeymapFallback filename="Space LSP"/>

| Label          | Action                |
| -------------- | --------------------- |
| `Code Actions` | Request code actions  |
| `Hover`        | Request hover info    |
| `Rename`       | Rename current symbol |

## `Pick`

<KeymapFallback filename="Space Pick"/>

### Searching behavior

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

### `Theme`

[Themes](../themes.md)

### `Symbol`

LSP Symbols

### `File`

Files (Not git ignored)

### `Buffer`

Buffers (opened files)

### `Git status @`

Git status (against current branch) [^1]

### `Git status ^`

Git status (against main branch) [^2]
[^1]: See more at [Git hunk](./selection-modes/secondary/index.md#hunkhunk)
[^2]: This is very useful when you want to get the modified/added files commited into the current branch that you are working on.

## `Editor`

<KeymapFallback filename="Space Editor"/>

### `Quit No Save`

Quit Ki without saving any unsaved files.

### `Save All Quit`

Save all files and quit Ki.

### `Save All`

Save all files.

### `Pipe`

Open a prompt which takes a shell command that treats the current selections as STDIN, and replace the current selections with the STDOUT of the command.

### `Replace all`

Replace all matches across all files of the current working directory with the specified replacement.

For example, if the search query is `l hello_world bye_moon`, it will replaces all `hello_world` across all files with `bye_moon`.

See more at [Search in Ki](docs/normal-mode/search-config.md).

### `TS Node Sexp`

Show the Tree-sitter node S-expression of the current selection.

## `Explorer`

Reveal current file in file explorer.

## `Help`

Shows the Normal mode keymap.

## Reveal

Reveal is a powerful viewport management feature that provides a bird's-eye view of your code or text. It automatically divides your viewport horizontally to show all relevant selections simultaneously, eliminating the need for scrolling (unless selections exceed the viewport height).

There are 3 kinds of Reveals:

### `÷ Selection` (Reveal Selections)

Reveal Selections dynamically creates viewports based on the selections of the current selection mode. This is particularly powerful for non-contiguous (secondary) selections created through Search, LSP Diagnostics, Git Hunks, and other multi-selection modes.

When used with Syntax Node selection mode, it can effectively emulate Code Folding, allowing you to view all sibling nodes of the current selected node, such as:

1. Viewing all functions of the current module
2. Viewing all methods of the current class
3. Viewing all statements of the current block
4. Viewing all subheaders under a header in a Markdown file

<TutorialFallback filename="reveal-selections"/>

### `÷ Cursor` (Reveal Cursors)

Reveal Cursors is not just useful, but essential when working with multiple cursors. It provides visual confirmation and confidence that your editing operations will be correctly applied across all cursor positions. This is particularly valuable for bulk editing operations where precision is crucial.

<TutorialFallback filename="reveal-cursors"/>

### `÷ Mark` (Reveal Marks)

Reveal Mark offers a modern alternative to traditional window splitting. Rather than manually managing multiple editor windows, you can mark and instantly view important sections simultaneously. You can think of it as automated window splitting.

<TutorialFallback filename="reveal-marks"/>
