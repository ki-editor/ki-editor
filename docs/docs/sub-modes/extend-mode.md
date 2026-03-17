---
sidebar_position: 7.5
---

import {TutorialFallback} from '@site/src/components/TutorialFallback';
import {KeymapFallback} from '@site/src/components/KeymapFallback';

# Extend Mode

This is used for extending the current selection.

For example, selecting multiple words or multiple lines.

It behaves more or less the same as click-and-drag in the textbox or text area of common GUI applications, but imagine being able to tune **both** ends, unlike using a mouse where an incorrect selection means you have to start over again.

When selection extension is enabled:

1. Each selection is composed of two ranges (originally one range).
1. There's only one moveable range at a time.
1. Every character between the two ranges, including the two ranges, is selected
1. Selection-wise actions work on the extended range
1. Press `ESC` to disable selection extension

<TutorialFallback filename="extend"/>
