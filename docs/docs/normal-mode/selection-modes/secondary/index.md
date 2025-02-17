---
sidebar_position: 3
---

import {KeymapFallback} from '@site/src/components/KeymapFallback';

# Secondary

Secondary selection modes are also non-contiguous selection modes.

Secondary selection modes can operate in two scopes:

- Local: Selections apply only within the current file/buffer you're editing
- Global: Selections apply across all files in your workspace/project

For example, when searching for text:

- Local search finds matches only in your current file
- Global search finds matches in all project files"

## Keymap

### Initialization

Most secondary selection modes are nested below the 3 keybindings below,
with the exception of Search and Seacrh Current, which are placed on the
first layer due to their ubiquity.

<KeymapFallback filename="Secondary Selection Modes Init"/>

Local Find is directional, meaning that if the cursor position does not overlap
with any selections of the chosen secondary selection mode, the cursor will
jump to the nearest selection in the chosen direction

Global Find however is non-directional.

Notice that the keybindings here are all located on the right side of the keyboard,
this is because all the secondary selection modes are placed on the left side of the
keyboard, which allows for efficient execution via hand-alternation.

There are 3 sets of keymap for secondary selection modes:

1. Local (Forward)
2. Local (Backward)
3. Global

They are almost identical except:

1. `One` and `Int` are only applicable for the Local keymaps
2. `Search` and `This` are only applicable for the Global keymap
3. Position of `Repeat` is different all 3 keymaps to enable easy combo:  
   a. To repeat the last secondary selection backward, press `y` (Qwerty) twice  
   b. To repeat the last secondary selection forward, press `p` (Qwerty) twice  
   c. To repeat the last secondary selection globally, press `n` (Qwerty) twice

### Local (Forward)

<KeymapFallback filename="Secondary Selection Modes (Local Forward)"/>

### Local (Backward)

<KeymapFallback filename="Secondary Selection Modes (Local Backward)"/>

### Global

<KeymapFallback filename="Secondary Selection Modes (Global)"/>

## Search-related

### `One`

Find one character, this is simlar to Vim's `f`/`t`.

### `Last`

Repeat the last search.

### `Config`

Configure search settings.

### `Int`

Integer. Useful for jumping to numbers.

## LSP Diagnostics

### `All`

All diagnostics.

### `Error`

Only Diagnostics Error.

### `Warn`

Only Diagnostics Warning.

### `Hint`

Only Diagnostics Hint.

### `Info`

Only Diagnostics Information.

## LSP Location

### `Impl`

Implementation.

### `Decl`

Declaration.

### `Def`

Definition.

### `Type`

Type definition.

### `Ref-`/`Ref+`

`Ref-`: References excluding declaration  
`Ref+`: References including declaration

In most cases, the Goto selection modes do not make sense in the Local (current
file) context, however `r` and `R` are exceptional, because finding local
references are very useful, especially when used in conjunction with Multi-
cursor.

## Misc

### `Repeat`

Repeats the last used secondary selection mode, this is particularly valuable when dealing with scenarios where standard multi-cursor operations are insufficient due to varying modification requirements.

#### Example

When removing unused imports:

```python
from math import cos  # Unused import 'cos'
from datetime import datetime, date  # Unused import 'date'
```

In this case, we need t

- Delete entire first line
- Remove only 'date' from second line

The `Repeat` command lets you reuse the last selection mode without manual reactivation, making these varied modifications more efficient.

### `Quickfix`

When getting selections using the Global mode, the matches will be stored into
the Quickfix List.

The quickfix selection mode behaves slightly differently in the Global/Local context:

| Context | Meaning                                                              |
| ------- | -------------------------------------------------------------------- |
| Global  | Navigate using the current quickfix list                             |
| Local   | Use matches of the current quickfix list that is of the current file |

#### When is global quickfix useful?

When you entered another selection mode but wish to use back the quickfix list.

#### When is local quickfix useful?

When you wanted to use Multi-cursor with the quickfix matches of the current file.

### `Hunk@`/`Hunk^`

`@` means compare against current branch.  
`^` means compare against main/master branch.

Git hunks are the diffs of the current Git repository.

It is computed by comparing the current file contents with the content on the latest commit of the current/main branch.

This is useful when you want to navigate to your recent changes, but forgot where they are.

### `Marks`

Mark is a powerful feature that allows you to jump to files that contain marks (which can be toggled).

It also allows you to swap two sections of the same file.
