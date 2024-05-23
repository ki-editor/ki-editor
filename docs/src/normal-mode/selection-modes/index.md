# Selection Modes

Selection modes [^1] dictates how [core movements](../core-movements.md) works.

There are roughly 3 categories of selection modes (not clear cut):

1. [Syntax tree-based](./syntax-tree-based.md)
2. [Regex-based](./regex-based.md)
3. [Native/Global](./native-global/index.md)

[^1]: For Vim users, selection mode means text objects.

## Contiguity

Besides the categorization above, selection modes are also separated based on their contiguity.

A selection mode is consider contiguous, if there's no meaningful gap between each of the selections.

A gap is meaningful is it's neither whitespaces only, nor separators like `,` or `;`.

The following selection modes are contiguous (exhaustive):

1. Line
1. Column
1. Word
1. Syntax Tree
1. Token
