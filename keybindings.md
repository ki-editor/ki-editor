# Keybindings

> [!WARNING]
> These keybindings are not finalized yet and are subject to changes.

> [!NOTE]
> Features marks with asterisk(\*) are not implemented yet.

## Design principle

- Consistency is king
- Mnemonics should not be afterthoughts
- Shift-key sucks
- Inherits well-thought keybindings from various editors (Emacs, Vim, Sublime)

> [!NOTE]
> I think shift-key sucks because I'm using [homerow mods](https://precondition.github.io/home-row-mods), and I placed the Shift-key on both pinkies. Also, I'm inspired by [Emac's God Mode](https://github.com/emacsorphanage/god-mode).

## Universal keybindings

The following keybindings work regardless of any mode.

- `ctrl+c`: **C**opy current selection(s)
- `ctrl+d`: Scroll page **d**own [from Vim]
- `ctrl+l`: Toggle view a**l**ignment (top, center, bottom) [from Emacs]
- `ctrl+s`: **S**ave current file
- `ctrl+u`: Scroll page **u**p [from Vim]
- `ctrl+w`: Cycle to next **w**indow
- `ctrl+v`: Paste (overrides current selection, like GUI editor)
- `ctrl+x`: Cut current selection(s)
- `ctrl+y`: Redo
- `ctrl+z`: Undo

## Insert mode

- `ctrl+a`/`home`: Move cursor(s) to the beginning of the line
- `ctrl+e`/`end`: Move cursor(s) to the **e**nd of line
- `alt+backspace`: Delete word backward

## Normal mode

- `a`: Enter insert mode **a**fter current selection
- `b`: Set selection mode to **b**ottom node
- `c`: Set selection mode to **c**haracter
- `d`: Move **d**own
- `e`: Enter **e**xchange mode
- `f`: Open **f**ind (local) menu
- `g`: Open find (**g**lobal) menu
- `h`: Toggle **h**ighlight mode
- `i`: Enter **i**nsert mode before current selection
- `j`: Enter **j**ump mode
- `k`: Kill current selection(s)
- `l`: Set selection mode to **l**ine
- `m`: Enter **m**ulti-cursor mode
- `n`: Move to **n**ext selection(s)
- `o`: Open **o**ther movement menu
- `p`: Move to **p**revious selection(s)
- `q`: Set selection mode to **q**uickfix
- `r`: Raise current selection(s) (Replace parent node with current node)
- `shift+R`: Replace the current selection with copied content, and copy the replaced content
- `s`: Set selection mode to **s**yntax tree
- `t`: Set selection mode to **t**op node
- `u`: Move **u**p
- `v`: (unassigned)
- `w`: Set selection mode to **w**ord
- `x`: Open common rege**x** menu
- `y`: (unassigned)
- `z`: (unassigned)
- `:`: Enter command mode
- `,`: Change to the previous selection(s)
- `*`: Select the whole file
- `%`: Toggle cursor position to start/end of selection
- `'`: Open List menu
- `(`/`{`/`[`/`<`: Enclose current selection(s) with `()`/`{}`/`[]`/`<>`
- `space`: Open context menu

> [!NOTE]
> I might change `i` -> `a` and `a` -> `e` so that it's consistent with `ctrl+a` and `ctrl+e` in Insert mode. I know this sucks for Vimmers (me too), but honestly speaking it makes much more sense for non-Vimmers for two reasons:
>
> 1. `a` is lexicographically smaller than `e`
> 2. `a` is on the left of `e` on the most popular keyboard layouts: Qwerty, Dvorak, and Colemak.

## Movements and selection modes

### Legends:

- (blank) = As implied by the name of the movement

> [!NOTE]
> This table only shows selection modes where next/previous/up/down has special meanings.

| Selection mode      | Next         | Previous         | Up                                | Down                          |
| ------------------- | ------------ | ---------------- | --------------------------------- | ----------------------------- |
| Line                |              |                  | Move to nearest parent line       |                               |
| Quickfix            |              |                  | \*First quickfix of previous file | \*First quickfix of next file |
| Syntax tree         | Next sibling | Previous sibling | Select parent                     | Select first child            |
| Undo Tree (space z) | Next branch  | Previous branch  | Redo                              | Undo                          |

## Exchange mode

In this mode, any movement will be translated into the following:

> Exchange current selection with [movement] selection

For example, if the current selection mode is Line, and the current mode is Exchange, pressing `n` exchange the current line with the next line.

## Raising

Raising ensure syntax correctness, it will not allow modifications that lead to syntax errors.

Note: this guarantee does not work in multi-cursor mode yet, but you can easily undo it by pressing `ctrl+z`.

## Find menu

The keybindings under the Find (local) and the Find (global) menu are almost identical.  
Not every keybindings are listed here because once you press `f` or `g` you will see them.

Local = find in current document only.  
Global = find in current repository.

There are 3 categories of keybindings under the Find menu:

1. Text search

- `a`: [Search by **A**ST-Grep](https://ast-grep.github.io/guide/pattern-syntax.html)
- `c`: Search **c**urrent primary selection
- `l`: **L**iteral (i.e. no characters has special meaning, e.g. a `(` means a `(`)
- `i`: Literal (**i**gnore case)
- `x`: [Rege**x**](https://ast-grep.github.io/guide/pattern-syntax.html) (Rust-flavor)

2. LSP Objects

- `d`: **D**efinition(s)
- `shift+D`: **D**eclaration(s)
- `e`: Diagnostic **E**rror
- `h`: Diagnostic **H**int
- `r`: **R**eference(s)
- `m`: I**m**plementation(s)
- `t`: **T**ype definition
- `s`: **S**ymbols
- `w`: Diagnostic **W**arning
- `y`: An**y** Diagnostic

3. Misc

- `g`: **G**it hunks
- `q`: Latest **q**uickfixes (local mode only)

You might wonder how finding for local LSP objects is useful, they are useful because local objects can be used with multicursor;
For example, placing the cursor on all references of a variable in the current file.

## Multicursor mode

In this mode, you can edit the cursors:

- `a`: Add cursors to all selections of the current selection mode, then enter Normal mode (nestable)
- `k`: **\*K**eep cursors matching conditions
- `n`: Add cursor to the **n**ext selection
- `p`: Add cursor to the **p**revious selection
- `o`: Keep the current cursor **o**nly
- `r`: **\*R**emove cursors matching conditions
- `s`: **\*S**plit current selection

## Highlight mode

This is not really a mode, but it allows extended selections.
Once toggled, each selection consists of two ends, and both ends are also selections on their own.
To switch between ends, press `h` again.
To stop the extended selection, press `esc`.

The first `h` press is like `ctrl+space` in Emacs, or `v` in Vim.
Subsequent `h` presses are like `ctrl+x ctrl+x` in Emacs, and `o` in Vim.

## Jump

Jump is like Vim's Easymotion which allows you to jump to any selection on the screen easily and quickly. How to use it?

First, enter the desired selection mode (if necessary), then press `j`.
After this, you should see colored marks popping up.
Secondly, you should type the **first character** of the selection that you want to jump to.
Then, the character under the colored marks will be reduced and changed, and you should press the character as shown under the desired selection.
The last step will repeat until there's no more ambiguity.

Usually, once you are familiar with each of the selection modes, it should only take on average 4 keystrokes (including changing selection mode) to get to where you want on the screen.

## Kill

Kill also means delete, it does the following things (in order):

1. Delete the current selection
2. Select the next selection (if the gap between the current selection and the next selection is only whitespaces)

If you are familiar with Vim, this behaves like `x`. However, it is not only restricted to character, it works for any selection mode as long as the condition is met.

## Other movements
`n`: Next most selection (i.e. the last selection)
`p`: Previous most selection (i.e. the first selection)
`i`: Go to a selection by index (1-based)

For example: press `l o n` to go to the last line, press `l o p` to go to the first line, and press `l o i 9 enter` to go to line 9. 

## List menu

This is like Neovim's Telescope plugin, where you can search through a list of objects.

Currently, the only searchable objects are files:

- Opened files
- Non-git-ignored files
- Git status files (i.e. modified files)
