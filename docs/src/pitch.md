# Why Ki?

The following are reasons that prevented me from escaping the Ki island, although the sirens of the Copilots were irresistible.

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

### 2.2 Duplication

To duplicate an AST node, press `s`, then `y` (Copy), and then `p` (Paste).

![duplicate-node](https://github.com/user-attachments/assets/c5d67419-1fe9-473b-954b-58912d40109d)

### 2.3 Swap

To swap an AST node, press `s`, then `x` (Exchange mode)`, and press `l` (Next) or `h` (Previous).

![swap-node](https://github.com/user-attachments/assets/14d314c3-4d15-4f48-bda2-3efa33b4725b)


## 3. First-class syntactic navigation

To navigate the syntax tree, press `s`, then press any of the following keys:
- `h` (Previous sibling)
- `l` (Next sibling)
- `,` (First sibling)
- `.` (Last sibling)
- `j` (First child)
- `k` (Parent)

![node-navigation](https://github.com/user-attachments/assets/549f225c-835e-4c3e-a69f-eca053f987eb)

