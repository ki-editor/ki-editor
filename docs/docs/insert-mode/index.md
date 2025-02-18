---
sidebar_position: 6
---

import {KeymapFallback} from '@site/src/components/KeymapFallback';

# Insert Mode

In this mode, Ki functions like the usual editor, where pressing keys on
the keyboard types them into the current opened file.

## Completion dropdown

The following keybindings only work when the completion dropdown is opened.

<KeymapFallback filename="Completion Items"/>

| Label         | Meaning                        |
| ------------- | ------------------------------ |
| `Comp →`      | Next completion item           |
| `← Comp`      | Previous completion item       |
| `Select Comp` | Select current completion item |

## Other

<KeymapFallback filename="Insert"/>

| Label/Keybinding | Meaning               |
| ---------------- | --------------------- |
| `Line ←`         | Move to line start    |
| `Line →`         | Move to line end      |
| `Kill Line ←`    | Kill line backward    |
| `Kill Line →`    | Kill line forward     |
| `Delete Token ←` | Delete token backward |
| `alt+backspace`  | Delete word backward  |
