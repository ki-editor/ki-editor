# Custom Keymaps

A custom keymap can be specified in `src/custom/custom_keymap.rs`, and is accessed after the leader keymap `\`.

There are currently 3 `CustomAction`s.

1. `RunCommand` | Allows you to run a CLI command. Ki holds no memory of the command.
2. `ToggleProcess` | Toggles the stopping and starting of long-living CLI processes. On their first initiation, these are started and stored by Ki. On the second, they will be killed and forgotten again. All processes are killed upon Ki closing.
3. `ToClipboard` | Copies text to the host system's clipboard.

The content of `CustomAction`s are constructed with `Placeholder`s. The Core `Placeholder`s are:

| Type | Description | Examples |
| :--- | :--- | :--- |
| `Str("")` | Holds a string of text. | `Str("This is a string of text")` |
| `FileCurrent` | Groups the context of the current file. | `FileCurrent::extension()`, `FileCurrent::path_root()`, `FileCurrent::path_local()` |
| `DirWorking` | Groups the context of the current working directory. | `DirCurrent::path_root()`, `DirCurrent::file_exists("Cargo.toml")`, `DirCurrent::file_exists_dynamic(SelectionPrimary::content())` |
| `SelectionPrimary` | Groups the context of the primary selection. | `SelectionPrimary::content()`, `SelectionPrimary::row_index()` |

These can be used together in `CustomAction`s like so:

```rust
fn sample_to_clipboard(ctx: &CustomContext) -> CustomAction {
    ToClipboard(vec![
        Str("Referring to:"),
        SelectionPrimary::content(),
        Str("\nIn file:"),
        FileCurrent::path_local(),
        Str("\nOn line:"),
        SelectionPrimary::row_index(),
    ])
}
```

Now we can select any text, say the signature of this function:

```rust
fn sample_to_clipboard(ctx: &CustomContext) -> CustomAction {
```

And then press `sample_to_clipboard`'s assigned keybind, to copy this text:

```text
Referring to: fn sample_to_clipboard(ctx: &CustomContext) -> CustomAction { 
In file: src/custom_config/custom_keymap.rs 
On line: 48
```

Note that the directory starts a `src`, because Ki was started in `src`'s parent directory.

Spaces are added between `Placeholder` arguments by default. Remove a space between arguments by adding a `NoSpace` `Placeholder`. Here we use this to open a link in a web browser:

```rust
fn sample_run_command(ctx: &CustomContext) -> CustomAction {
    // Search selected content using Google and Chromium
    RunCommand(
        "chromium",
        vec![
            Str("https://www.google.com/search?q="),
            NoSpace,
            SelectionPrimary::content(),
        ],
    )
}
```

[https://www.google.com/search?q=chromium](https://www.google.com/search?q=chromium)No gap between `=` and `c`!

`Placeholder`s can be evaluated outside a `CustomAction` with `.resolve(ctx)`. This can be used in control flow, like so:

```rust
fn sample_toggle_process(ctx: &CustomContext) -> CustomAction {
    // Render the current file in a new window of Chromium
    if FileCurrent::extension().resolve(ctx) == "html" {
        ToggleProcess(
            "chromium",
            vec![FileCurrent::path_root(), Str("--new-window")],
        )
    } else if FileCurrent::extension().resolve(ctx) == "typ" {
        ToggleProcess(
            "tinymist",
            vec![
                Str("preview"),
                Str("--invert-colors=auto"),
                Str("--open"),
                FileCurrent::path_root(),
            ],
        )
    } else {
        DoNothing
    }
}
```

Functions are assigned keybinds using the `custom_keymap()` function.

```rust
pub(crate) fn custom_keymap() -> Vec<(
    Meaning,
    &'static str,
    Option<fn(&CustomContext) -> CustomAction>,
)> {
    let custom_keymap: [(Meaning, &str, Option<fn(&CustomContext) -> CustomAction>); 30] = [
        // Key, Description, Function
        (__Q__, "Sample RunCommand", Some(sample_run_command)),
        (__W__, "Sample ToggleProcess", Some(sample_toggle_process)),
        (__E__, "Sample ToClipboard", Some(sample_to_clipboard)),
        (__R__, "", None),
        (__T__, "", None),
        (__Y__, "", None),
        (__U__, "", None),
        (__I__, "", None),
        (__O__, "", None),
        (__P__, "", None),
        // Second row
        (__A__, "Tmux build", Some(tmux_build)),
        (__S__, "Kitty cargo test", Some(kitty_cargo_test)),
        (__D__, "", None),
        (__F__, "", None),
        (__G__, "", None),
        (__H__, "", None),
        (__J__, "", None),
        (__K__, "", None),
        (__L__, "", None),
        (_SEMI, "", None),
        // Third row
        (__Z__, "", None),
        (__X__, "", None),
        (__C__, "", None),
        (__V__, "", None),
        (__B__, "", None),
        (__N__, "", None),
        (__M__, "", None),
        (_COMA, "", None),
        (_DOT_, "", None),
        (_SLSH, "", None),
    ];
    custom_keymap.into_iter().collect()
}
```

# Someday:

- `Macro`s will be available as a `CustomAction`.
- `UserInput` will be available as a `Placeholder`.
- Configuration will be moved outside the codebase.