---
slug: evoluation-of-delete
title: Evolution of Delete
authors:
  - name: Jia Hau
    title: Ki maintainer
    url: https://github.com/wongjiahau
tags: [documentation]
---

# Evolution of Delete

In this article, I'll discuss how the Delete operation went through several revisions before arriving at its current behavior.

## Version 1: Selection mode-specific

In the initial version, Delete already worked quite differently from how it works in other modal editors like Vim/Kakoune.

In Vim/Kakoune, once a selection is deleted, the new selection collapses into a single-character selection.

For example (assuming `[` and `]` represent the boundaries of a selection):

```
foo [bar] spam
```

After Delete, the result is:

```
foo [ ]spam
```

But using Version 1 Delete in Ki, the result is:

```
foo [spam]
```

What are the differences?

1. `spam` is automatically selected.
2. The gap between `bar` and `spam` (a single whitespace ` `) is also deleted.

Why? This allows users to easily delete the next word by executing Delete again.

You might ask: why doesn't the editor select `foo` after deleting `bar`, instead of `spam`?

That's because the default direction moves forward (rightward) after deleting the current selection.

There was also another action called Delete Backward.

So what was the problem with Version 1?

In Ki, there are two kinds of lateral movements:

1. **Left/Right**: skips insignificant selections
2. **Previous/Next**: does not skip insignificant selections

You can think of the possible selections of Previous/Next as a superset of those of Left/Right.

This creates a choice: should Delete use Previous/Next or Left/Right to determine which gap to delete?

Here's where the conflict arises.

In Word selection mode (called Token in this version), which selects common identifiers such as `snake_case`, `kebab-case`, `camelCase`, etc., the insignificant selections are symbols such as `-`, `::`, `/`, etc.

If Delete uses the Left/Right movement in Word selection mode, we'd have unexpected deletions like this:

Initial state:

```
spam
[foo].
bar
```

Executing Delete would delete the `.` and `\n` as well:

```
spam
[bar]
```

This could be desirable or surprising depending on what you intended to achieve.

Therefore, the less surprising choice was to use Previous/Next for Delete, which would result in:

```
spam
[.]
bar
```

However, using Previous/Next for Delete wasn't ideal in Syntax Node selection mode. For example:

Initial state:

```
fn main([x: X], y: Y, z: Z) {}
```

If Delete uses Previous/Next, the result would be:

```
fn main([,] y: Y, z: Z) {}
```

This is undesirable because when deleting a sibling node in Syntax Node selection mode, we typically want to delete its trailing insignificant symbol (in this case, `,`) to ensure the code remains syntactically valid after deletion:

```
fn main([y: Y], z: Z) {}
```

Because different selection modes called for different lateral movements, the initial version had to use different lateral movements for different selection modes.

## Version 2: Resolving the inconsistencies

Reference commit: https://github.com/ki-editor/ki-editor/commit/b6747ecb07130aedb8edd53392936d878db55108

@vishal noticed that Delete behaved inconsistently across different selection modes, and suggested we make Delete's behavior consistent, as consistency is one of Ki's design principles.

In this version, we decided that Delete should use the Left/Right movement for all selection modes.

This raised a question: what if you want Delete to use the Previous/Next movement?

That's why we introduced a new action called _Delete 0 Gap_, which does exactly that. (This name is actually somewhat misleading because it does delete gaps.)

However, Delete 0 Gap was bound to the shift layer, making it less ergonomic to use.

## Version 3: Delete submode

Before introducing this version, we need to mention https://github.com/ki-editor/ki-editor/commit/2bc355ba22783abe3541a425462c396ac3fb571b, where @vishal used Swap Cursor to reverse the direction of actions such as Delete, Paste, and Open.

After this change, the Delete Backward action (which used to be a shifted key) was removed.

To delete backward, you simply execute Swap Cursor first.

The introduction of Swap Cursor for reversing actions, combined with Version 2's Delete behavior, made certain actions quite unergonomic. For example, to delete backward using the Previous/Next movement, you had to first execute Swap Cursor, then press shift to execute Delete 0 Gap.

To make all kinds of Delete equally ergonomic, in https://github.com/ki-editor/ki-editor/commit/fa09130cf93945c60550849ed76a8e590ceaef93, the Delete action was turned into a submode (similar to the Multi-cursor and Swap submodes).

As a submode, all kinds of delete take the same number of steps—minimally 3:

1. Enter Delete Submode
2. Execute a Movement
3. Escape Delete submode

## Version 4: What you see is what you get

Although making all delete operations equally easy to execute solved one problem, it also meant they were all equally **tiring** to execute.

@\_**Vishal|981015** [said](https://ki-editor.zulipchat.com/#narrow/channel/551672-Feature-Idea-.F0.9F.92.A1/topic/Expand.20Selection.20Action/near/564614863):

> As currently although Space solves for unergonomic of esc on normal keybaprds, the delete submode's Action Motion Motion feels a bit tiring tbh.

To resolve this issue while still allowing gaps to be deleted, this version introduced two new actions: Expand Forward and Expand Backward.

Expand Forward expands the current selection rightward until just before the Right selection.

For example, in Word selection mode:

```
foo [bar] spam
```

Executing Expand Forward results in:

```
foo [bar ]spam
```

Expand Backward works identically but in the opposite direction.

With this change, Delete no longer **automatically deletes gaps**. Now what you select is what gets deleted—no more surprises or unexpected behaviors.

If a gap you intended to delete wasn't deleted, it means you didn't select it in the first place.

This raises a question: will the following selection still be selected as Version 1 Delete did?

Yes, but only if the upcoming selection will occupy the same range after deleting the current selection.

This ensures that the cursor position (start of the selection) doesn't change after executing Delete.

For example, in Word selection mode:

```
foo [bar ]spam
```

Executing Delete results in the new selection being `spam`:

```
foo [spam]
```

because the position of the first character of `spam` intersects with the cursor position.

In contrast, if the starting state is:

```
foo [bar] spam
```

Then after Delete, only the whitespace following `bar` is selected:

```
foo [ ]spam
```

This version of Delete reduces the minimal number of steps from 3 to 1.

Example:

- Delete Forward without deleting Gap: 1 step
- Delete Forward including Gap: 2 step (execute Expand Forward first)
- Delete Backward without deleting Gap: 2 step (execute Swap Cursor first)
- Delete Backward including Gap: 3 Step (execute Expand Backward and Swap Cursor first)

## Version 5: Delete Menu

The issues with Version 4 are:

1. The gap between the current selection and the adjacent selection is not deleted unless Expand Selection was executed first
2. The adjacent selection will not be selected, which breaks the flow

Version 3 (Delete Submode) was already very good in terms of effectiveness—the capability for users to execute what they have in mind accurately.

Its only downside was lethargy: a simple deletion that includes a gap takes at least 3 keypresses (`v <movement> space`), where the last key exits the submode.

To make Delete less tiring, we decided to make Delete a **menu** instead of a submode. The key difference is that **a menu automatically closes after selecting one option**, while **a submode stays active until explicitly exited**. This means a simple deletion that includes a gap now takes only 2 keypresses: `v <movement>`.

The tradeoff is that you cannot chain multiple deletions in one invocation. However, this can be compensated with the Extend action (`g`) for multi-selection workflows.

For example, to delete two selections:

- **With Delete Submode**: Enter Delete Submode → Right → Right → Escape Delete Submode
- **With Extend + Delete Menu**: Extend → Right → Open Delete Menu → Right

Since most deletions involve only one selection, the menu approach is more efficient for the common case while keeping the Delete Submode available for users who prefer chaining deletions.
