---
sidebar_position: 1
---

# Intro

Momentary Layers (≡) activate when a key is held down and deactivate immediately upon release. This allows temporary access to an alternate set of key functions without toggling the layer on permanently.

This terminology comes from the keyboard modding community. For more details, see: https://zmk.dev/docs/keymaps/behaviors/layers#momentary-layer.

This behavior is akin to keyboard modifiers (e.g., `Win + R` to open the Run dialog), which also apply changes only while held. In ki, holding a key (e.g. `Insert ≡`) additionally triggers displaying a menu for quick reference. Also, pressing and releasing `Insert ≡` is a command in itself, similar to the `Win` key.

Windows keyboards use `Win` icon, MacBook keyboards use e.g., ⌘ for Command, to visually denote modifier keys. The `≡` symbol here mimics that convention by representing layered or "stacked" functionality, similar to a hamburger menu.

In documentation and in ki itself, we will use this `≡` icon like a general symbol for modifiers.

So that you will have `Buffer ≡`, `Insert ≡` etc.

## Terminal Support

MoL requires terminals that support the [Kitty Keyboard Protocol](https://sw.kovidgoyal.net/kitty/keyboard-protocol/) (KKP):

- Alacritty
- Ghostty
- Foot
- iTerm2
- Rio
- WezTerm
- TuiOS terminal

## Fallback

If your terminal doesn't support KKP, Ki will still function, but you'll need to manually press `esc` to deactivate layers instead of simply releasing the key.
