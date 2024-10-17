---
sidebar_position: 1
---

# Text Search

This is one of the most useful selection modes.

Keybindings:

| Keybinding | Meaning                      |
| ---------- | ---------------------------- |
| `/`        | Open search prompt           |
| `*`        | Search current selection     |
| `p`        | Search using previous search |
| `'`        | Configure search             |

By default, the search prompt searches using Literal mode.

Other modes can be toggled/chosen by opening the search configurator.

## Configurator

The following modes/options are configurable in the configurator.

## Modes

There are 4 search modes, and only one of them can be chosen at any time.

### 1. Literal

Keybinding: `l`

The default and the most commonly used search mode. In this mode, every
characters are treated verbatim, a `(` means a `(`, it does not mean the start
of a capture group.

### 2. Regex

Keybinding: `x`

This is based on Rust-flavored regex.

### 3. AST-Grep

Keybinding: `a`

This is based on [AST Grep](https://github.com/ast-grep/ast-grep), useful for structural search and replacement.

### 4. Case Agnostic

Keybinding: `c`

Case as in `camelCase` or `snake_case`, not upper-case or lower-case.

This is one of my favorite search modes, especially when dealing with business code.

In this mode, the search will be expanded to all different cases such as (non-exhaustive):

1. `kebab-case`
2. `UPPER-KEBAB`
3. `MACRO_CASE`
4. `snake_case`
5. `Title Case`
6. `lower case`
7. `UPPER CASE`
8. `camelCase`
9. `PascalCase`

For example, searching `hello world` in this mode matches (non-exhaustive):

1. `helloWorld`
2. `HELLO_WORLD`
3. `HelloWorld`

etc.

This is most powerful when used with [Replace with Pattern](../../actions/index.mdx#replace-with-pattern).

## Options

Alongside modes, there are multiple options (not mutually exclusive) that can be turned on or off.

### 1. Match whole word

Keybinding: `w`

When turned on, the search will be restricted to match word boundary (`\b`). For example, `hello` will not match itself in `helloWorld`, it will only match standalone `hello`s.

### 2. Case-sensitive

Keybinding: `i`

When turned on, the uppercase or lowercase of each alphabet of the search becomes important. For example, `hello` will not match `Hello`.

## Globbing

This only works in Global text search.  
This is useful when you want to include/exclude certain files/directories during
global search/replace.

This feature is powered by the [globset](https://docs.rs/globset/latest/globset/#syntax) library, which supports alternation such as:

- `*.{js,jsx}`

Keybindings:

| Keybinding | Action           |
| ---------- | ---------------- |
| `I`        | Set include glob |
| `E`        | Set exclude glob |

## Replace All

This is a global action that replaces all matches with the replacement pattern[^1].

[^1]: See more at [Replace with Pattern](../../actions/index.mdx#replace-with-pattern)
