---
sidebar_position: 13
---

# Global Reveal & Global Multicursor

These two features let you act on an entire quickfix list — populated by a [global selection mode](normal-mode/selection-modes/secondary.md) — across all matching files simultaneously.

## Workflow

### Step 1: Activate a global selection mode

Global selection modes search across all files in your project and populate the quickfix list. For example:

- `space f` — search for the current selection across all files
- `space s` — show all LSP Diagnostic errors globally

See [Secondary Selection Modes](normal-mode/selection-modes/secondary.md) for the full list.

### Step 2: Apply Global Reveal or Global Multicursor

Once the quickfix list is populated:

| Action             | Key         | Effect                                                          |
| ------------------ | ----------- | --------------------------------------------------------------- |
| Global Reveal      | `space u`   | Splits the viewport to show all quickfix entries simultaneously |
| Global Multicursor | `b space j` | Places a cursor at every quickfix entry, across all files       |

## Global Reveal (`space u`)

Toggles a split-viewport view of every entry in the quickfix list — across files — giving you a bird's-eye overview without scrolling.

This is useful for:

- Reviewing all LSP diagnostic errors across your project at a glance
- Auditing all references to a symbol spread across files before editing
- Inspecting all search matches project-wide before committing to a bulk change

## Global Multicursor (`b space j`)

Places a cursor at every entry in the quickfix list, across all matching files. From there, any edit you make is applied at every cursor simultaneously.

This is useful for, but not limited to:

- Renaming a term across all files (as a complement to LSP rename)
- Deleting or replacing all occurrences of a pattern project-wide
- Applying the same syntactical transformation to every LSP diagnostic error

### Supported multicursor actions

Not all [multicursor actions](momentary-layers/multi-cursor-mol.mdx) are available in Global Multicursor mode. The following are supported:

| Action                     | Description                                                                                                                       |
| -------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| Cycle cursor               | Switch the primary cursor between files                                                                                           |
| Delete cursor              | Remove the primary cursor; if it was the only cursor in its file, and it's not the only file left, that file is removed from view |
| Keep matching selections   | Filter down to cursors whose selection content matches a pattern                                                                  |
| Remove matching selections | Filter out cursors whose selection content matches a pattern                                                                      |
| Keep primary cursor only   | Collapse back to a single cursor, deactivating Global Multicursor                                                                 |

Notably, adding cursors via movements and splitting selections by selection mode change are **not** supported.

## Local equivalents

The same keys work identically for local selection modes, scoped to the current file only.

| Scope                 | `space u` effect                | `b space j` effect                      |
| --------------------- | ------------------------------- | --------------------------------------- |
| Local selection mode  | Reveal matches in current file  | Cursors at all matches in current file  |
| Global selection mode | Reveal matches across all files | Cursors at all matches across all files |

The only difference is the scope of the selection mode activated in Step 1.
