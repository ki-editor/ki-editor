---
slug: loosening-definitions
title: Loosening Definitions
subtitle: for more explicable bindings
authors:
  - name: Alice Alysia
    title: Contributor
    url: https://codeberg.org/alicealysia
tags: [documentation]
---

Ki is a terminal text editor which uses a special "Selection Mode" to define a small number of powerful, easily memorable, and extremely ergonomic keybindings. In Selection Mode your left hand switches which objects you're currently navigating by, be that letters, words, lines or elements within the programming language the file you're editing is written in. Meanwhile your right hand moves the cursor between the specified objects via an expanded variation of the ijkl movement scheme. Ki also uses a modified version of familiar bindings like z (Undo), x (Cut), and c (Copy), which when combined with the movement (ijkl) keys can engage in additional, more powerful actions.

Hi. I'm Alice Alysia. I have an admission to make! I'm... a Ki addict. In fact, going against all common sense, I'm writing this blog post in ki right now. I'm guessing most of you are too! You may be wondering why I just described Ki to you, and it's because Ki is surprisingly difficult to describe, despite how self explanatory it begins to feel once you start using it. I've been thinking about this odd disconnect for a while. Ki is simple. Much simpler than many, many other text editors, even those notorious for their simplicity. Yet explaining it feels complicated. I think the reason why is the exact reason more seasoned users may feel like I actually lied a few times during the above explanation. Because I say "an expanded variation of the ijkl movement scheme" but what I mean is:

On a QWERTY keyboard

| Key | Binding |
|---|---|
| Y/P | First/Last |
| U/O | Prev/Next (Granular) |
| J/L | Prev/Next (Neutral) |
| I/K | Up/Down (Fastest) |

The above keys are positional, and as such, simply maintain their physical position on other layouts despite being different letters.

This can introduce some... unintentional behaviour, and also makes the editor harder to describe. As a user's fist interaction with it will be in line mode. The default selection mode immediately frustrates the ijkl bindings concept because while it is, strictly speaking, still true that we are using ijkl bindings, those bindings don't do what a user would immediately assume. The assumption a user may have is that i/k will go up/down 1 line, while the other directional bindings will perhaps be used in more interesting ways. More interestingly though, is how the commitment to the above table can result in a lot of inefficiencies. Several modes have duplicate bindings, while missing fairly useful bindings. Take the character mode for example. Presently, we have bindings for up, down, left right, and jump to start/end of... word. Not line, paragraph, or file, but word.

Finding answers to these problems however, isn't easy. Particularly because while our current bindings are easy to write down. They're not so easy to solve these problems with. So what's the answer? Well, the short version is to be more willing to break our own rules.

While we don't want to remove any sense of logic from our bindings, we do want to let ourselves go with the flow a bit more. Hence, we now have the following, slightly adjusted philosophy:

| Key | Binding |
|---|---|
| Y/P | First/Last |
| U/O | Prev/Next (Flexible) |
| J/L | Prev/Next (Common) |
| I/K | Up/Down (Gridwise) |

 In practice, this means that in some modes, u/o will be used for bindings that are more granular than j/l currently are, but from time to time, we may instead make u/o move in a more pronounced way than j/l does. Using our character example from above, this *could* mean having u/o move to the start/end of a word, to allow y/p to become the start/end of line options. Given, this is just an example. Not something set in stone.
 
Getting the philosophical change out of the way was necessary before we could begin considering more complex changes like the functionality of things like word and line mode, but with this done, I'm sure you can see the freedom this introduces, and the ways this can make ki an easier to understand editor, that is also far, far more powerful!
