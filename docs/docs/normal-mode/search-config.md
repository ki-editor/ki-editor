---
sidebar_position: 4
---

import { TutorialFallback } from '@site/src/components/TutorialFallback';

# Search in Ki

Ki has its own powerful searching language that offers flexibility and precision when searching through code and text.

## Text-based vs Menu-based Search Interfaces

You might wonder why Ki uses a text-based language approach for search configuration instead of a menu-based UI (like VS Code and many other editors). There's a specific reason for this design choice:

Ki Motion is designed to be exportable to other environments, similar to how Neovim can be embedded in other applications (like the VS Code Neovim plugin). The text-based language approach ensures that Ki's core functionality can be exported and integrated easily with minimal functionality mappings. Converting menu interfaces between different editors would require significant adaptation work and maintenance.

While Ki previously used a more traditional menu-based approach, the current design prioritizes:

1. Exportability to different host environments
2. Speed and efficiency for experienced users
3. Consistent behavior regardless of which application Ki is embedded in
4. Keyboard-centric workflow that keeps your hands on the keyboard

This approach allows for a consistent experience across all environments where Ki Motion might be embedded, and enables complex search patterns to be composed, saved, and shared more easily than would be possible with a menu-based interface.

## Search Config Structure

When you open a search prompt, you'll construct a search configuration with these components:

1. **Search mode** - Determines how your search pattern is interpreted
2. **Search query** - The actual pattern you want to search for
3. **Replacement** - Used for search and replace operations
4. **Include globs** - File patterns to include (only for global search)
5. **Exclude globs** - File patterns to exclude (only for global search)

These components are separated by a delimiter character which must be non-alphanumeric and not a backslash. For example:

```
r hello world **.js node_modules/**
```

In this example:

| Component    | Value             |
| ------------ | ----------------- |
| Mode         | `r` (Regex)       |
| Separator    | space             |
| Search query | `hello`           |
| Replacement  | `world`           |
| Include glob | `**.js`           |
| Exclude glob | `node_modules/**` |

## Choosing a Separator

The recommended separator is a space, but you should pick one that minimizes the need to escape characters in your search components. For example, if your search query contains spaces, you might choose `/` as your separator instead.

Throughout this document, you'll see examples using different separators (like spaces or slashes). These different separators are chosen specifically to avoid character escaping in each example, demonstrating how to select the most efficient separator for different search patterns.

## Rules

### Escaping Rules

- Backslash (`\`) is only used to escape the separator. In all other cases, it's treated as a literal backslash.
- You don't need to use double backslashes (`\\`) to represent a single backslash character.
- A backslash can never be used as the separator itself.

For example, in the input `a/hell\o\/`:

- The first backslash before `o` is treated as a literal character (not escaping anything)
- The second backslash before `/` is escaping the separator
- The resulting search query is `hello\/` (searching for the text "hello/")

### No Error Policy

Invalid combinations of search mode options will not result in an error. Instead, Ki will assume that the entire string is what you wanted to search for:

- If you type `hello` (which isn't a valid mode option), Ki will use the default search mode (Literal) and search for the text "hello"
- If you type something like `xyz/search`, where "xyz" isn't a valid mode, Ki will search for the literal text "xyz/search"

### No Empty Search Policy

If your configuration results in an empty search query, the entire raw input will be treated as a literal search:

- For example, if you enter `r//f`, this would parse as: mode = Regex, search = "" (empty), replacement = "f"
- Since an empty search is not allowed, Ki will instead search for the literal text "r//f"

This policy ensures that if you intended to search for something literally that starts with specific symbols (such as C comments `//` or Python comments `#`), it will work as intended without having to explicitly prefix your search with the literal mode `l<separator>`.

However, other than search, all other components can be empty, for example, `l/hello//*.js` is parsed as search for literal "hello", but only in files matching the `*.js` glob pattern.

### Minimal Separator Policy

You do not need to include all 5 separators in all configurations, you only need to include them as needed.

For example, if you only need to specify the mode and the search query, then one separator is enough.

### Two-phase Parsing

Understanding Ki's two-phase parsing approach is crucial for building the correct mental model of how search works, helping you avoid confusion about when and how characters are escaped (unlike the "escaping hell" often experienced in CI config YAML files).

Ki processes your search input in two distinct phases:

1. **Configuration Phase** - During this first phase, backslashes only function to escape separators. Character sequences like `\n` or `\t` are treated as literal characters (a backslash followed by 'n' or 't'), not as special characters.

2. **Search Engine Phase** - Once the search configuration is constructed, your search query is passed to the appropriate search engine based on the specified mode. It's only in this second phase that special character sequences might gain specific meanings.

For example:

- In Regex mode, `\n` will be interpreted as a newline character
- In AST Grep mode, `$X` will be treated as a node capture
- In Literal mode, all characters maintain their literal meaning in both phases

This two-phase approach creates a clean separation between configuration syntax and search pattern language, making the system more predictable and easier to use. Once you grasp this separation, constructing complex search patterns becomes significantly more intuitive.

## Search Modes

Ki offers four primary search modes, each serving different searching needs:

### 1. Literal Mode

**Short form:** `l`

The default and most commonly used search mode. Every character is treated literally - a `(` means a `(`, not the start of a capture group.

Options for Literal mode:

- `c` - Case-sensitive (e.g., `c/Hello` matches "Hello" but not "hello")
- `w` - Match whole word (e.g., `w/hello` matches "hello" but not "helloWorld")
- `s`, `wc`, or `cw` - Strict mode (both case-sensitive and whole word)

<TutorialFallback filename="literal-search"/>

### 2. Regex Mode

**Short form:** `r`

Powered by the [Fancy Regex](https://github.com/fancy-regex/fancy-regex) engine, supporting features like look-around and backtracking.

Options for Regex mode:

- `rc` - Case-sensitive regex
- `rw` - Whole word regex matching
- `rs`, `rcw`, or `rwc` - Strict regex (both case-sensitive and whole word)

<TutorialFallback filename="regex"/>

### 3. AST-Grep Mode

**Short form:** `a`

Based on [AST Grep](https://github.com/ast-grep/ast-grep), this mode is useful for structural search and replacement in code.

<TutorialFallback filename="ast-grep"/>

### 4. Naming Convention Agnostic Mode

**Short form:** `n`

One of the most powerful modes, especially when dealing with business code. This mode expands your search across different naming conventions:

- `kebab-case`
- `UPPER-KEBAB`
- `MACRO_CASE`
- `snake_case`
- `Title Case`
- `lower case`
- `UPPER CASE`
- `camelCase`
- `PascalCase`

For example, searching for `hello world` in this mode will match:

- `helloWorld`
- `HELLO_WORLD`
- `HelloWorld`
- And many more variations

<TutorialFallback filename="naming-convention-agnostic"/>

## Search Options

While the search modes above determine how your search pattern is interpreted, the following search options can be applied to modify how matches are evaluated within those modes (only applicable to Regex and Literal mode):

### 1. Match Whole Word

When enabled, restricts matches to word boundaries (`\b`). For example, `hello` will not match itself in `helloWorld`, only standalone occurrences.

<TutorialFallback filename="match-whole-word"/>

### 2. Case-Sensitive

When enabled, the case of each character becomes significant. For example, `hello` will not match `Hello`.

<TutorialFallback filename="case-sensitive"/>

### 3. Strict

A shortcut for enabling both Case-sensitive and Match Whole Word options.

## Globbing in Global Search

Globbing patterns allow you to include or exclude specific files and directories during global search operations. This feature is powered by the [globset](https://docs.rs/globset/latest/globset/#syntax) library.

### Include/Exclude Patterns

You can specify which files to search within or exclude from your search. For example:

```
l hello world **.{js,jsx} node_modules/**
```

This will search for "hello" and replace it with "world" in all `.js` and `.jsx` files, excluding the `node_modules` directory.

Globbing supports:

- `*` - Matches any sequence of characters except `/`
- `**` - Matches any sequence of characters including `/`
- `?` - Matches any single character except `/`
- `{a,b}` - Matches either pattern a or pattern b
- `[abc]` - Matches any of the specified characters

## Replacement

By default, submitting the search input will not trigger a replacement although the replacement is not empty.

Usually, you do not need to use the replacement, because often times you can get it done by using multicursor and changing all selections by just typing. Multicursor allows you to edit at multiple positions simultaneously.

However, in complex use cases where you want the updated result to contain part of the original string, this can sometimes be too difficult or outright impossible if you go with the multicursor plus a series of actions route.

### Replace with Pattern

To update the current selections with the replacement, use `shift+X` (Qwerty).

<TutorialFallback filename="replace-all"/>

<TutorialFallback filename="replace-with-pattern"/>

### Replace all

Replace all matches across the repository with a specified replacement pattern.

This is a global action affecting all matching occurrences.

Keybinding: `space X`

## Overcoming Unintended Gotchas

When working with Ki's search, you might occasionally run into situations where your input is parsed differently than you intended. Here's how to handle some common scenarios:

### Forcing Literal Mode

If you intended to perform a literal search, but your input accidentally qualifies as a multi-component configuration, you can always force literal mode by prefixing your search with `l<separator>`.

For example, you might want to search for `s.to_string()`, but since `s` is a valid option and `.` is a valid separator, this will be parsed as:

- search = "s"
- separator = "."
- replacement = "to_string()"

To fix this and search for the exact text "s.to_string()", you can rerun the search by prefixing it with `l ` (using a space character as separator since it's the easiest to type and the original search query contains no spaces):

```
l s.to_string()
```

This approach works for any input that might be unintentionally parsed as a search configuration when you just want to search for it literally, as long as you choose a separator that avoids the need to escape characters in your original search query.

## Summary of Search Mode Syntax

| Search Mode            | Description                | Example                  |
| ---------------------- | -------------------------- | ------------------------ |
| `l` or none            | Literal (default)          | `hello` or `l hello`     |
| `c`                    | Case-sensitive literal     | `c Hello`                |
| `w`                    | Whole word literal         | `w hello`                |
| `s` or `wc` or `cw`    | Strict literal             | `s Hello`                |
| `r`                    | Regex                      | `r hel+o`                |
| `rc`                   | Case-sensitive regex       | `rc Hel+o`               |
| `rw`                   | Whole word regex           | `rw hel+o`               |
| `rs` or `rcw` or `rwc` | Strict regex               | `rs Hel+o`               |
| `a`                    | AST grep                   | `a/if ($cond) { $body }` |
| `n`                    | Naming convention agnostic | `n/hello world`          |
