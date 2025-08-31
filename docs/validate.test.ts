import { extractArgumentFileNames } from './validate';

import { assert, describe, expect, it } from 'vitest';

describe('Validate Static Resource Access', () => {
  it('empty mdx', () => {
    const mdx = '';
    expect(extractArgumentFileNames(mdx)).toEqual([]);
  })
  
  it('single tag', () => {
    const mdx = '<TutorialFallback filename="example-config" />';
    expect(extractArgumentFileNames(mdx)).toEqual(['example-config']);
  })
  
  it('multiple tags', () => {
    const mdx = `
<TutorialFallback filename="resource1" />
<TutorialFallback filename="resource2" />`;
    expect(extractArgumentFileNames(mdx)).toEqual(['resource1', 'resource2']);
  })

  it('commented tags', () => {
    const mdx = `
<TutorialFallback filename="resource1" />
<TutorialFallback filename="resource2" />
{/* <TutorialFallback filename="resource3" /> */}`;
    expect(extractArgumentFileNames(mdx)).toEqual(['resource1', 'resource2']);
  })
  
  it('frontmatter mdx', () => {
    const mdx = `
---
title: Hello World
---
<TutorialFallback filename="resource" />`;
    expect(extractArgumentFileNames(mdx)).toEqual(['resource']);
  })
  
  it('ignore other tags', () => {
    const mdx = `
<SomeOtherComponent filename="ignore-me" />
<TutorialFallback filename="keep-me" />`;
    expect(extractArgumentFileNames(mdx)).toEqual(['keep-me']);
    });
  
  it('inlining single tag', () => {
    const mdx = `
Here is an inline use of <TutorialFallback filename="resource" /> resource`
    expect(extractArgumentFileNames(mdx)).toEqual(['resource']);
  })
  
  it('inlining multiple tags', () => {
    const mdx = `
Multiple <TutorialFallback filename="resource1" /> tags <TutorialFallback filename="resource2" />`;
    expect(extractArgumentFileNames(mdx)).toEqual(['resource1', 'resource2']);
  })
  
  it('inlining tags with regular tags', () => {
    const mdx = `
Multiple <TutorialFallback filename="resource1" /> <TutorialFallback filename="resource2" />`;
    expect(extractArgumentFileNames(mdx)).toEqual(['resource1', 'resource2']);
  })
  
  it('long example with multiple tags', () => {
    const mdx = `
import {TutorialFallback} from '@site/src/components/TutorialFallback';
import {KeymapFallback} from '@site/src/components/KeymapFallback';

# Multi-cursor mode

## Intro

Multi-cursor mode works through two main mechanisms: [Movement](../normal-mode/core-movements.md) and [Selection Mode](../normal-mode/selection-modes/index.md).

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

> Add a cursor with \\<movement\\>

<TutorialFallback filename="add-cursor-with-movement"/>

## 2. Selection Mode Changes

In the Multi-cursor mode, changing the selection mode means:

> Split each selection by the new selection mode

<TutorialFallback filename="split-selections"/>

[1]: ../normal-mode/core-movements.md#leftright

## 3. Other multicursor actions

Keymap:

<KeymapFallback filename="Multi-cursor"/>

### A. \`Keep Match\`/\`Remove Match\`

Keep/Remove selections matching search.

This is only useful when there's more than 1 selection/cursor, and you want to remove some selections.

<TutorialFallback filename="filter-matching-selections"/>

### B. \`Curs All\`

Add cursor to all matching selections.

<TutorialFallback filename="add-cursor-to-all-matching-selections"/>

### C. \`Keep Prime Curs\`

Keep primary cursor only.

<TutorialFallback filename="keep-primary-cursor-only"/>

### D. \`Delete Curs ←\`/\`Delete Curs →\`

Delete primary cursor backward/forward.

<TutorialFallback filename="delete-cursor"/>`;
    expect(extractArgumentFileNames(mdx)).toEqual([
      "add-cursor-with-movement",
      "split-selections",
      "filter-matching-selections",
      "add-cursor-to-all-matching-selections",
      "keep-primary-cursor-only",
      "delete-cursor",
    ]);
  })
})
