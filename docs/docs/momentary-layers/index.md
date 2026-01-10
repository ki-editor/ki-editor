---
sidebar_position: 1
---

# Intro

Momentary Layers (MOL) are layers that activate when a key is held and deactivate when released, similar to how the Shift key works.

This terminology comes from the keyboard modding community. For more details, see: https://zmk.dev/docs/keymaps/behaviors/layers#momentary-layer.

## Terminal Support

MOL requires terminals that support the [Kitty Keyboard Protocol](https://sw.kovidgoyal.net/kitty/keyboard-protocol/) (KKP):

- Alacritty
- Ghostty
- Foot
- iTerm2
- Rio
- WezTerm
- TuiOS terminal

## Fallback

If your terminal doesn't support KKP, Ki will still function, but you'll need to manually press `esc` to deactivate layers instead of simply releasing the key.
