---
sidebar_position: 1
---

# Intro

Momentary Layers (≡), often called MoL, activate while a key is held and deactivate immediately upon release. This provides temporary access to alternate key functions without permanently toggling the layer.

This terminology comes from the keyboard modding community. For more details, see: https://zmk.dev/docs/keymaps/behaviors/layers#momentary-layer.

Momentary Layer is the same as the `⊞ Win` key. You can use it as a modifier, and you can tap it to trigger an action: `⊞ Win` to open Windows menu, `⊞ Win + R` to open the Run dialog.

In ki, for example, tapping `≡ Insert` deletes the current selection and enters Insert mode. Holding `≡ Insert` additionally displays a menu with additional actions like opening new line below.

Windows keyboards use `⊞` for Win, MacBook keyboards use e.g., `⌘` for Command, to visually denote modifier keys. The `≡` symbol here mimics that convention by representing layered or "stacked" functionality, similar to a hamburger menu.

In documentation and in ki itself, we will use this `≡` icon like a general symbol for modifiers.

So that you will have `≡ Buffer`, `≡ Insert` etc.

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
