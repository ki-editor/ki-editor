# LSP-based

These are selection modes powered by LSP, so it's not always available.

## Diagnostics

| Keybinding | Meaning                 |
| ---------- | ----------------------- |
| `a`        | Any kind of diagnostic  |
| `e`        | Diagnostic Error only   |
| `h`        | Diagnostic Hint only    |
| `i`        | Diagnostic Info only    |
| `w`        | Diagnostic Warning only |

## Goto

| Keybinding | Meaning                          |
| ---------- | -------------------------------- |
| `d`        | Definitions                      |
| `D`        | Declarations                     |
| `i`        | Implementations                  |
| `r`        | References                       |
| `R`        | References (include declaration) |
| `t`        | Type definitions                 |

In most cases, the Goto selection modes do not makes sense in the Native (
current file) context, however `r` and `R` are exceptional, because finding
native references is very useful, especially when used in conjunction with Multi-cursor.
