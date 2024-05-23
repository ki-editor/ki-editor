# Configurations

At the moment, configuration files are not supported, because I'm in favor of compile-time configuration [^1], for the following reasons:

1. Easier to update
1. Running with incompatible configurations are impossible [^2]
1. Configuration as code
   - Free typechecking
   - Free formatting[^3]
   - Ability to reduce duplications using functions
   - Easy backup (fork Ki-editor and push your modified config)

## Files for configurations

| Type      | Path                                 |
| --------- | ------------------------------------ |
| Languages | `shared/src/languages.rs`            |
| Theme     | (Not yet as there is only one theme) |

[^1]: For example, see [dwm](https://wiki.archlinux.org/title/dwm#Configuration) and [Xmonad](https://xmonad.org/TUTORIAL.html)
[^2]: Neovim usually let's you glide through until it commits kamikaze
[^3]: Rant: [TOML does not endorse an official formatter](https://github.com/toml-lang/toml/issues/532#issuecomment-384313745)
