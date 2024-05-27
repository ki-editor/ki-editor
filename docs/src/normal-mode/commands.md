# Commands

The command prompt can be brought up by pressing `:`.

Commands (non-exhaustive):

| Name             | Description                           |
| ---------------- | ------------------------------------- |
| `quit-all`       | Quit the editor.                      |
| `write-all`      | Save all buffers.                     |
| `write-quit-all` | Save all buffers and quit the editor. |

Aliases are not supported because you can leverage [fuzzy find](./space-menu.md#searching-filessymbols). For example, `qa` matches `quit-all`, since the first letter of `quit` is `q`, and for `all` it's `a`.
