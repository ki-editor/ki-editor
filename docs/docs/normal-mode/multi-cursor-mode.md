import {TutorialFallback} from '@site/src/components/TutorialFallback';

# Multi-cursor mode

Keybinding: `q`  
Reason: `q` is used to start recording a macro in Vim, but I realized 80% of the time what I need is multi-cursors, not a macro.

Multi-cursor mode works through two main mechanisms: [Movement](./core-movements.md) and [Selection Mode](./selection-modes).

Unlike other editors where there are specific keybindings for adding cursors in specific ways,
Ki gives you the freedom to add cursors by either:

- Using Movement commands to place additional cursors
- Changing the Selection Mode to split existing selections into multiple cursors

This flexibility allows you to:

1. Add a cursor to the next word
2. Add cursors until the last line
3. Add a cursor to the previous diagnostic
4. Add a cursor to an oddly specific place
5. Add cursors to all lines within current selection(s)

These are just examples - the true power of multi-cursor mode comes from combining Movement and Selection Mode in creative ways. Unleash your imagination!

## 1. Movements

In the Multi-cursor mode, every core movement means:

> Add a cursor with \<movement\>

<TutorialFallback filename="add-cursor-with-movement"/>

## 2. Selection Mode Changes

In the Multi-cursor mode, changing the selection mode means:

> Split each selection by the new selection mode

<TutorialFallback filename="split-selections"/>

[1]: ./core-movements.md#leftright

## 3. Filter selections

Keybindings:

- `m`: Maintain matching selections
- `r`: Remove matching selections

This is only used when there's more than 1 selection/cursor, and you want to remove some selections.

<TutorialFallback filename="filter-matching-selections"/>

## 4. Add to all matching selections

Keybinding: `q`

<TutorialFallback filename="add-cursor-to-all-matching-selections"/>

## 5. Keep primary cursor only

Keybinding: `o`

<TutorialFallback filename="keep-primary-cursor-only"/>

## 6. Delete primary cursor

Keybindings:

- `d`: Delete primary cursor forward
- `D`: Delete primary cursor backward

<TutorialFallback filename="delete-cursor"/>
