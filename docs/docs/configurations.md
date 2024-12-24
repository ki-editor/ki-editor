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

Until a method of basic configuration is decided upon, the theme for Ki can be configured via the environment variable
`KI_EDITOR_THEME`. The theme defaults to "VS Code (Light)". You can find a list of known themes by launching `ki` and
using the Pick Theme option, `SPACE t`.

[^1]: For example, see [dwm](https://wiki.archlinux.org/title/dwm#Configuration) and [Xmonad](https://xmonad.org/TUTORIAL.html)
[^2]: Neovim usually let's you glide through until it commits kamikaze
[^3]: Rant: [TOML does not endorse an official formatter](https://github.com/toml-lang/toml/issues/532#issuecomment-384313745)
