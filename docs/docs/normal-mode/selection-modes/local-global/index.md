# Local/Global

Selection modes that are categorized here are those that are applicable both locally (current file) and globally (across the workspace).

There are 3 categories of Local/Global selection modes:

1. [Text Search](./text-search.md)
1. [LSP-based](./lsp-based.md)
1. [Misc](./misc.md)

> These selection modes are nested under `T`/`t` (Find (Local)) or `g` (Find (Global)).

When using Find (Local) mode, `T` searches backward and `t` searchs forward if
no matching selection is found under cursor.

## Shortcut (`;`)

All selection modes under this Local/Global category are non-contiguous,
and since they require at least two keypresses, Ki provides a shortcut:

> To set the current selection mode back to the last non-contiguous selection modes,
> press `;`.

For example, after you searched for a term (either locally or globally),
and you've changed to another contiguous selection mode (such as Word),
pressing `;` will set the selection mode back to the term search.
