---
sidebar_position: 6
---

# Insert Mode

In this mode, Ki functions like the usual editor, where pressing keys on
the keyboard types them into the current opened file.

## Completion dropdown keybindings

The following keybindings only work when the completion dropdown is opened.

| Keybinding       | Meaning          |
| ---------------- | ---------------- |
| `ctrl+n`         | Next item        |
| `ctrl+p`         | Previous item    |
| `ctrl+space`[^1] | Use current item |

[^1]: Why not `enter` or `tab`? Because often times, you actually wanted to insert a newline or a tab, so you press `esc` to close the dropdown menu, but by doing so you've also escaped the Insert mode, and that is infuriating.

## GNU Readline Keybindings

Although [Normal Mode](../normal-mode/index.md) is the main sauce of Ki, it also
implements a subset of [GNU Readline Keybindings](https://www.gnu.org/software/bash/manual/html_node/Bindable-Readline-Commands.html).

I highly recommend every terminal user to learn these keybindings, as they work
in almost every terminal UI (TUI) application, for example:

- macOS native textbox
- fish
- bash
- zsh
- pgcli
- emacs
- mongosh

Implemented keybindings [^1]:

| Keybinding      | Meaning                  |
| --------------- | ------------------------ |
| `ctrl+b`        | Move back a character    |
| `ctrl+f`        | Move forward a character |
| `ctrl+a`        | Move to line start       |
| `ctrl+e`        | Move to line end         |
| `ctrl+k`        | Kill line forward        |
| `ctrl+u`        | Kill line backward       |
| `ctrl+w`        | Delete token backward    |
| `alt+backspace` | Delete word backward     |

[^1]: Not every unimplemented keybinding is incompatible/meaningless with/in Ki, but because I do not have time for them, so feel free to submit PR!
