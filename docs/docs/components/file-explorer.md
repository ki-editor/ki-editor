# File Explorer

import {KeymapFallback} from '@site/src/components/KeymapFallback';

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

## Keymap

<KeymapFallback filename="File Explorer Actions"/>

## Meanings

| Label          | Action                                            |
| -------------- | ------------------------------------------------- |
| `Add Path`     | Add a new file/folder under the current path [^1] |
| `Dup path`     | Duplicate current file to a new path              |
| `Delete Paths` | Delete the selected paths [^2]                    |
| `Move Path`    | Move (or rename) the current file/folder [^3]     |
| `Refresh`      | Refresh the file explorer [^4]                    |

[^1]: To add a folder, append `/` to the file name. Can be nested, and new directories will be created as required.
[^2]: You can delete multiple paths at once by using [selection extension](../sub-modes/extend-mode.md) or [multi-cursors](../sub-modes/multi-cursor-mode.md)
[^3]: Works like `mkdir -p`, it will create new directories when required.
[^4]: This is necessary sometimes because the file system is modified by external factors, and Ki does not watch for file changes.

## Other keybinding

`enter` is override to mean:

- Expand/collapse folder OR
- Open file

## Tips

Because the File Explorer is just a YAML file, the following actions are free[^1]:

| Action                                                                       | How?                                      |
| ---------------------------------------------------------------------------- | ----------------------------------------- |
| Go to parent folder                                                          | Use [`a j`][^4]                           |
| Go to first/last file in current folder                                      | Use [First/Last][2] with [Syntax Node][3] |
| Go to next/previous file/folder at current level, skipping expanded children | Use [Left/Right][4] with [Syntax Node][3] |

[1]: ../normal-mode/selection-modes/primary.md#line
[2]: ../normal-mode/core-movements.md#firstlast
[3]: ../normal-mode/selection-modes/primary.md#syntax-1
[4]: ../normal-mode/core-movements.md#--leftright

[^4]: Free as in no extra implementations required
