import {TutorialFallback} from '@site/src/components/TutorialFallback';

# Other Movements

## Keymap

```
╭───┬───┬───┬───┬───┬───┬────────┬──────────┬──────────┬──────────┬──────────╮
│   ┆   ┆   ┆   ┆   ┆ ⌥ ┆        ┆ ← Select ┆ Scroll ↑ ┆ Select → ┆          │
│   ┆   ┆   ┆   ┆   ┆ ⇧ ┆ ← Curs ┆          ┆          ┆          ┆  Curs →  │
│   ┆   ┆   ┆   ┆   ┆ ∅ ┆        ┆          ┆          ┆          ┆          │
├╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌┤
│   ┆   ┆   ┆   ┆   ┆ ⌥ ┆        ┆ ← Buffer ┆ Scroll ↓ ┆ Buffer → ┆          │
│   ┆   ┆   ┆   ┆   ┆ ⇧ ┆        ┆          ┆          ┆          ┆          │
│   ┆   ┆   ┆   ┆   ┆ ∅ ┆        ┆          ┆          ┆          ┆          │
├╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌┼╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌┤
│   ┆   ┆   ┆   ┆   ┆ ⌥ ┆        ┆          ┆          ┆          ┆          │
│   ┆   ┆   ┆   ┆   ┆ ⇧ ┆        ┆          ┆          ┆          ┆ ⇋ Anchor │
│   ┆   ┆   ┆   ┆   ┆ ∅ ┆        ┆          ┆          ┆          ┆  ⇋ Curs  │
╰───┴───┴───┴───┴───┴───┴────────┴──────────┴──────────┴──────────┴──────────╯
```

The movements categorized here are not affected or bounded by [Selection Modes](./selection-modes/index.md).

## Meaning

### `Scroll ↑`/`Scroll ↓`

Scroll half-page up/down.

### `⇋ Curs`

Swap the primary cursor with the secondary cursor.

By default, the primary cursor sits on the first character of the selection, and the secondary cursor sits on the last character of the selection.

For example, if the current selection is `hello world`, then the cursor sits on `h`, while the anchor sits on `d`.

The secondary cursors serves as a visual aid, making it easier to recognize when the selection range has been modified.

This is especially necessary when using selection modes such as [Fine Syntax Node](./selection-modes/primary.md#syntax-1), where occasionally, the start of a selection remains the same while the end of it changes.

Usefulness:

- When your current selection spans more than a visible screen, and you wish to see what's at the end of the current selection.
  - For example, when you selected a very long function.
- When you wish to start a new selection at the end of the current selection
  - For example, when you select a line and wish to change its last word.

<TutorialFallback filename="swap-cursors"/>

### `⇋ Anchor`

Swap extended selection anchors.

This is only applicable when the selection is extended.

By default, when the selection extension is activated, you can only extend the selection forward,
but with this, you can extend the selection backward too.

This is similar to Vim's Visual Mode `o`.

### `← Select`/`Select →`

Go to the previous/next selection. This is similar to Vim's `ctrl+o`/`ctrl+i`, but it onlys work within a file.

This is useful when you messed up the current selection, especially when you are
using [Syntax Node](./selection-modes/primary.md#syntax), and
expanding the current selection to parent node.

Use `← Select` to restore the selection to the previous state.  
Press `Select →` to restore the selection to the current state.

### `← Curs`/`Curs →`

Cycle primary cursor (selection) backward/forward.

### `← Buffer`/`Buffer →`

Go to the previous/next buffer.

### Quick Editor Jumping

You can "tag" editors with the numbers 1 through 5 for quick access during your
editing session. This feature allows you to efficiently manage and switch
between your primary files and other ancillary files. Here's how it works:

#### Number Key Actions

##### Tagging an Editor

If no editors are currently tagged with the number you press, the current editor will be tagged with that number.

##### Clearing a Tag

If the current editor is already tagged with the number you press, the tag will be cleared from that editor.

##### Jumping to a Tagged Editor

If there is an editor tagged with the number you press, the editor will switch to that tagged editor immediately.

#### Workflow Overview

This workflow is designed to streamline your editing process by allowing quick
access to your primary files. During an editing session, you often work on
primary files while occasionally referring to other less important files. Using
the number keys, you can quickly jump back to your main files, enhancing your
productivity and focus.

By utilizing this tagging system, you can efficiently navigate your editing
environment and maintain your workflow's momentum.
