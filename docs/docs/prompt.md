---
sidebar_position: 10
---

# Prompt

The prompt is one of the most commonly used components in Ki, this is because
unlike other editors, it is also the picker!

## History

Every kind of prompt has its own history.

For example, the search prompt stores the history of searches.

Unlike the usual prompts, however, the historical entries of a prompt are shown below the current line,
starting with the most recent entry.

To navigate to historical entries, use [Normal Mode](normal-mode).

`enter` is overridden to mean select the current item, it works in both Insert Mode and Normal mode.

## Groups

The items of a prompt can be grouped, for example, the items of the file picker are grouped by their
parent folder.

The group name of each item is also matched by the search query, in a disjunctive manner, i.e. an item
will be matched if either its group name **or** its own name satisfies the search query.

## Behaviour

The prompt has two behaviours:

| Kind   | Behavior                                  | Examples                     |
| ------ | ----------------------------------------- | ---------------------------- |
| Picker | Select current matching item upon `enter` | symbol picker, file picker   |
| Prompt | Use current search query upon `enter`     | search prompt, rename prompt |

## Special Picker Behaviors

Currently the file picker is the only prompt with special behavior. Other
prompts may get special behaviors in the future.

### File Picker

| Key              | Behavior                                       |
| ---------------- | ---------------------------------------------- |
| `alt+g` (Qwerty) | Open all narrowed files in the Completion list |

Imagine you are going to work on the `widget` in your source code project. You
may have three files: a controller, a view and a test. You type `widget` into
the file picker and that narrows down the completions to `widget.js`,
`widget.html` and `widget_test.js`. Pressing `ctrl+o` will open all three and
close the file picker.
