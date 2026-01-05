---
sidebar_position: 1
---

# Intro

Selection modes [^1] dictates how [core movements](../core-movements.md) works.

There are 2 categories of selection modes:

1. [Primary](./primary.md)
2. [Secondary](./secondary.md)

[^1]: For Vim users, selection mode means text objects.

## Contiguity

Selection modes are categorized by their contiguity.

If a selection mode is contiguous, it means that there are no meaningful gaps between each of the selections.

A gap is meaningful if it's neither whitespaces only nor separators like `,` or `;`.

Primary selection modes are contiguous, while secondary selection modes are non-contiguous.
