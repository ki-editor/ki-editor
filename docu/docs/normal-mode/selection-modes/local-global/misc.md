# Misc

These are selections mode that are neither based on [Text Search](./text-search.md) nor [LSP](./lsp-based.md).

## Git Hunk

Keybindings:

- `g`: Get git hunks against current branch
- `G`: Get git hunks against main branch

Git hunks are the diffs of the current Git repository.

It is computed by comparing the current file contents with the content on the latest commit of the current/main branch.

This is useful when you want to navigate to your recent changes, but forgot where they are.

## Marks

Keybinding: `m`

Marks or bookmarks is a powerful feature that allows you to jump to files that contain marks (which can be toggled).

It also allows you to exchange two sections of the same file.

## Quickfix

Keybinding: `q`

When getting selections using the Global mode, the matches will be stored into
the Quickfix List.

The quickfix selection mode behaves slightly differently in the Global/Local context:

| Context | Meaning                                                              |
| ------- | -------------------------------------------------------------------- |
| Global  | Navigate using the current quickfix list                             |
| Local   | Use matches of the current quickfix list that is of the current file |

### When is global quickfix useful?

When you entered another selection mode but wish to use back the quickfix list.

### When is local quickfix useful?

When you wanted to use Multi-cursor with the quickfix matches of the current file.
