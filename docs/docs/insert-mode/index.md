---
sidebar_position: 6
---

import {KeymapFallback} from '@site/src/components/KeymapFallback';
import {TutorialFallback} from '@site/src/components/TutorialFallback';

# Insert Mode

In this mode, Ki functions like the usual editor, where pressing keys on
the keyboard types them into the current opened file.

## Enter Normal Mode

To enter the normal mode, press `esc` (regardless of keyboard layout).

If the current selection mode is any of the following, then the selection before the cursor will be selected:

1. Line
2. Line Full
3. Token
4. Word

Otherwise, only one character before the cursor will be selected, this is because except the selection modes above,
the cursor might jump beyond the current view, causing unintended disorientation.

<TutorialFallback filename="enter-normal-mode"/>

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
