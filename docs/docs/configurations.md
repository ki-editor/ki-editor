# Configurations

At the moment, configuration files are not supported, because I'm in favor of compile-time configuration [^1], for the following reasons:

1. Easier to update
1. Running with incompatible configurations is impossible [^2]
1. Configuration as code
   - Free type-checking
   - Free formatting[^3]
   - Ability to reduce duplications using functions
   - Easy backup (fork Ki-editor and push your modified config)

However, I'm open to suggestions, I might even create a new language for that.

## Files for configurations

| Type      | Path                      |
| --------- | ------------------------- |
| Languages | `shared/src/languages.rs` |

## Environment variables for configurations

### Intro

Until a method of basic configuration is decided upon, settings for Ki will be configured via the environment variables.

Note that these environment variables are loaded at runtime, not build time.

### `KI_EDITOR_THEME`

The theme defaults to "VS Code (Light)". You can find a list of known themes by launching `ki` and
using the Pick Theme option, `space a`.

### `KI_EDITOR_KEYBOARD`

For configuring keyboard layout, with the following possible values:

1. `qwerty` (Default)
1. `dvorak`
1. `colemak`
1. `colemak_dh`
1. `colemak_dh_semi_quote`
1. `dvorak_iu`
1. `workman`

[^1]: For example, see [dwm](https://wiki.archlinux.org/title/dwm#Configuration) and [Xmonad](https://xmonad.org/TUTORIAL.html)
[^2]: Neovim usually let's you glide through until it commits kamikaze
[^3]: Rant: [TOML does not endorse an official formatter](https://github.com/toml-lang/toml/issues/532#issuecomment-384313745)
