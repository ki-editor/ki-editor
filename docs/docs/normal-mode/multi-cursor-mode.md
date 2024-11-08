import {TutorialFallback} from '@site/src/components/TutorialFallback';

# Multi-cursor mode

Keybinding: `q`  
Reason: `q` is used to start recording a macro in Vim, but I realized 80% of the time what I need is multi-cursors, not a macro.

### Movements

In the Multi-cursor submode, every core movement means:

> Add cursor with \<movement\>

Use the following text as an example:

```txt
hello ki, hello vim, hello helix
```

Suppose:

- The current selection mode is [Find Literal "hello"](./selection-modes/local-global/text-search.md#1-literal)
- The current selection is the first `hello`
- The current submode is Multi-cursor

... then executing [Next][1] adds a new cursor to the second `hello`.

### Selection Mode Changes

In the Multi-cursor submode, changing the selection mode means:

> Split each selections by the new selection mode

<TutorialFallback filename="split-selections"/>

[1]: ./core-movements.mdx#leftright
