# Secondary

Secondary selection modes are also non-contiguous selection modes.

Secondary selection modes can operate in two scopes:

- Local: Selections apply only within the current file/buffer you're editing
- Global: Selections apply across all files in your workspace/project

For example, when searching for text:

- Local search finds matches only in your current file
- Global search finds matches in all project files"

There are 3 categories of Secondary selection modes:

1. [Text Search](./text-search.md)
1. [LSP-based](./lsp-based.md)
1. [Misc](./misc.md)

## Keymap

Most secondary selection modes are nested below the 3 keybindings below,
with the exception of Search and Seacrh Current, which are placed on the
first layer due to their ubiquity.

```
╭───┬───┬───┬───┬───┬───┬────────┬───┬───┬───┬────────╮
│   ┆   ┆   ┆   ┆   ┆ ⌥ ┆        ┆   ┆   ┆   ┆        │
│   ┆   ┆   ┆   ┆   ┆ ⇧ ┆        ┆   ┆   ┆   ┆        │
│   ┆   ┆   ┆   ┆   ┆ ∅ ┆ ← Find ┆   ┆   ┆   ┆ Find → │
├╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌╌╌╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌╌╌╌╌╌┤
│   ┆   ┆   ┆   ┆   ┆ ⌥ ┆        ┆   ┆   ┆   ┆        │
│   ┆   ┆   ┆   ┆   ┆ ⇧ ┆        ┆   ┆   ┆   ┆        │
│   ┆   ┆   ┆   ┆   ┆ ∅ ┆        ┆   ┆   ┆   ┆        │
├╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌╌╌╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌╌╌╌╌╌┤
│   ┆   ┆   ┆   ┆   ┆ ⌥ ┆        ┆   ┆   ┆   ┆        │
│   ┆   ┆   ┆   ┆   ┆ ⇧ ┆        ┆   ┆   ┆   ┆        │
│   ┆   ┆   ┆   ┆   ┆ ∅ ┆ Global ┆   ┆   ┆   ┆        │
╰───┴───┴───┴───┴───┴───┴────────┴───┴───┴───┴────────╯
```

Local Find is directional, meaning that if the cursor position does not overlap
with any selections of the chosen secondary selection mode, the cursor will
jump to the nearest selection in the chosen direction

Global Find however is non-directional.

Notice that the keybindings here are all located on the right side of the keyboard,
this is because all the secondary selection modes are placed on the left side of the
keyboard, which allows for efficient execution via hand-alternation.

## Shortcut (`;`)

All selection modes under this Local/Global category are non-contiguous,
and since they require at least two keypresses, Ki provides a shortcut:

> To set the current selection mode back to the last non-contiguous selection modes,
> press `;`.

For example, after you searched for a term (either locally or globally),
and you've changed to another contiguous selection mode (such as Word),
pressing `;` will set the selection mode back to the term search.

## Quickfix List

After applying a global selection mode:

1. Matching items will be populated into the Quickfix List
2. The global mode will be set to Quickfix, where movements will navigate the list
3. Pressing `esc` will close the Quickfix List and change the current editor selection mode to Local Quickfix
