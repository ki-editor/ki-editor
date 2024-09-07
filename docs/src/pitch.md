# Why Ki?

## 1. First-class structural selection

> Being first-class means that it is not an extra or even sidekick; it is the **protagonist**.

To select the largest AST node under the cursor, simply press `s` (Syntax Node). 

This feature is handy if you ever asked this question: 

_"How can I select the current expression/function/argument/statement?"_

And this works as long as your language is blessed by the Tree-sitter grammarians.

![select-largest-node](https://github.com/user-attachments/assets/1bc1bbf4-d5f2-4233-b2a6-f07f8316fd84)


## 2. First-class structural modification

### 2.1 Deletion

To delete multiple sibling AST nodes in a row, first press `s`, then repeat `d`.

![delete-node](https://github.com/user-attachments/assets/8b2c263d-d05b-4f50-ae1d-ee17914f7c09)

### 2.2 Duplication

To duplicate an AST node, first press `s`, then `y` (Copy), and then `p` (Paste).

![duplicate-node](https://github.com/user-attachments/assets/c5d67419-1fe9-473b-954b-58912d40109d)



## 3. First-class structural navigation

## 4. Built-in global search and replace

## 3. Same keybindings at every components
