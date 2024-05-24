# File Explorer

Ki's file explorer is rendered using YAML, for example:

```yaml
- 📂  docs/:
    - 🙈  .gitignore
    - 📁  book/:
    - 📄  book.toml
    - 📂  src/:
        - 📚  SUMMARY.md
        - 📂  components/:
            - 📚  file-explorer.md
            - 📚  index.md
        - 📚  configurations.md
        - 📚  core-concepts.md
        - 📚  features.md
        - 📁  insert-mode/:
        - 📚  installation.md
        - 📚  modes.md
        - 📁  normal-mode/:
        - 📁  selection-modes/:
        - 📚  themes.md
        - 📚  universal-keybindings.md
- 📚  dummy-todo.md
- 📁  event/:
- 📁  grammar/:
- 📄  justfile
```

## Keybindings

Only `enter` is overridden to mean:

- Expand/collapse folder OR
- Open file

Other keybindings can be found at [contextual keybindings](../normal-mode/space-menu.md#file-explorer-actions).

## Tips

Because the File Explorer is just a YAML file, the following actions are free[^1]:

| Action                                                                        | How?                                         |
| ----------------------------------------------------------------------------- | -------------------------------------------- |
| Go to parent folder                                                           | Use [Parent Line][1]                         |
| Go to first/last file in current folder                                       | Use [First/Last][2] with [Syntax Tree][3]    |
| Go to next/previous file/folder at current level, skipping expanded children | Use [Previous/Next][4] with [Syntax Tree][3] |

[^1]: Free as in no extra implementations required

[1]: ../normal-mode/core-movements.md#parent-line
[2]: ../normal-mode/core-movements.md#firstlast
[3]: ../normal-mode/selection-modes/syntax-tree-based.md#syntax-tree
[4]: ../normal-mode/core-movements.md#previousnext
