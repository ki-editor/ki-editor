---
sidebar_position: 7
---

import {KeymapFallback} from '@site/src/components/KeymapFallback';
import {TutorialFallback} from '@site/src/components/TutorialFallback';

# Insert Mode

In this mode, Ki functions like the usual editor, where pressing keys on
the keyboard types them into the current opened file.

## Enter Normal Mode

To enter the normal mode, press `esc` (regardless of keyboard layout).

When entering normal mode, only one character before the cursor will be selected, this is because except the selection modes above,
the cursor might jump beyond the current view, causing unintended disorientation.

<TutorialFallback filename="enter-normal-mode"/>

## Completion dropdown

The following keybindings only work when the completion dropdown is opened.

<KeymapFallback filename="Completion Items"/>

| Label          | Meaning                         |
| -------------- | ------------------------------- |
| `Comp →`       | Next completion item            |
| `← Comp`       | Previous completion item        |
| `Replace Comp` | Replace current completion item |

## Insert Mode Delete MoL

<KeymapFallback filename="Insert Mode Delete MoL"/>

| Label/Keybinding   | Meaning                 |
| ------------------ | ----------------------- |
| `← Kill Line`      | Kill line backward      |
| `Kill Line →`      | Kill line forward       |
| `← Delete Word`    | Delete word backward    |
| `← Delete Subword` | Delete subword backward |
| `Delete Word →`    | Delete word forward     |
| `Delete Subword →` | Delete subword forward  |

## Other

<KeymapFallback filename="Insert"/>

| Label/Keybinding | Meaning            |
| ---------------- | ------------------ |
| `← Line`         | Move to line start |
| `Line →`         | Move to line end   |
