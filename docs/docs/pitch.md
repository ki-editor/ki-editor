---
sidebar_position: 2
---

# Why Ki?

The following are reasons that prevented me from escaping the Ki island, although the sirens of the Copilots were irresistible.

No land in the archipelago of the modal editors has more economic keybindings for these actions.

## 1. First-class syntactic selection

> Being first-class means that it is not an extra or even sidekick; it is the **protagonist**.

To select the largest syntax node under the cursor, simply press `s` (Syntax Node).

This feature is handy if you ever asked this question:

_"How can I select the current expression/function/argument/statement?"_

And this works as long as your language is blessed by the Tree-sitter grammarians.

![select-largest-node](https://github.com/user-attachments/assets/1bc1bbf4-d5f2-4233-b2a6-f07f8316fd84)

## 2. First-class syntactic modification

### 2.1 Deletion

To delete multiple sibling syntax nodes in a row, first press `s`, then repeat `d`.

![delete-node](https://github.com/user-attachments/assets/8b2c263d-d05b-4f50-ae1d-ee17914f7c09)

Notice the comma between the current and the next node is also deleted.

This doesn't only work for JSON, it can be used to also delete statements, array elements, arguments, and basically anything within a list of syntax nodes.

### 2.2 Duplication

To duplicate an AST node, press `s`, then `y` (Copy), and then `p` (Paste).

![duplicate-node](https://github.com/user-attachments/assets/c5d67419-1fe9-473b-954b-58912d40109d)

Notice how `comma` is added automatically.

### 2.3 Swap

To swap an AST node, press `s`, then `x` (Exchange mode), and press `n` (Next) or `N` (Previous).

## 3. First-class syntactic navigation

To navigate the syntax tree, press `s`, then press any of the following keys:

- `N` (Previous sibling)
- `n` (Next sibling)
- `,` (First sibling)
- `.` (Last sibling)
- `b` (First child)
- `t` (Parent)

## 4. Multi-cursor

Delete unused imports:

- `] h` (set selection mode to Diagnostics Hint)
- `q` (enter multi-cursor mode)
- `l` (add a cursor to the next selection)
- `s` (set selection mode to Syntax Node)
- `d` (delete)
- `space o` (keep only the primary selection)

Again, notice how the commas are removed automatically.

![remove-unused-imports](https://github.com/user-attachments/assets/1e26cae5-e24d-4010-bebc-c9ee8837293b)
