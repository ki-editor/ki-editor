/// NOTE: all test cases that involves the clipboard should not be run in parallel
///   otherwise the the test suite will fail because multiple tests are trying to
///   access the clipboard at the same time.
#[cfg(test)]
mod test_app {
    use itertools::Itertools;
    use my_proc_macros::key;
    use pretty_assertions::assert_eq;
    use serial_test::serial;

    use std::sync::{Arc, Mutex};
    use Dispatch::*;
    use DispatchEditor::*;

    use shared::canonicalized_path::CanonicalizedPath;

    use crate::{
        app::{App, Dimension, Dispatch},
        components::{
            editor::{Direction, DispatchEditor, Movement},
            suggestive_editor::Info,
        },
        context::GlobalMode,
        frontend::mock::MockFrontend,
        integration_test::integration_test::TestRunner,
        lsp::{process::LspNotification, signature_help::SignatureInformation},
        position::Position,
        quickfix_list::{Location, QuickfixListItem},
        selection::SelectionMode,
    };

    fn run_test(
        callback: impl Fn(App<MockFrontend>, CanonicalizedPath) -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        TestRunner::run(|temp_dir| {
            let mock_frontend = Arc::new(Mutex::new(MockFrontend::new()));
            let mut app = App::new(mock_frontend, temp_dir.clone())?;
            app.disable_lsp();
            callback(app, temp_dir)
        })
    }

    #[test]
    #[serial]
    fn copy_paste_from_different_file() -> anyhow::Result<()> {
        run_test(|mut app, temp_dir| {
            let path_main = temp_dir.join("src/main.rs")?;
            let path_foo = temp_dir.join("src/foo.rs")?;

            // Open main.rs
            app.open_file(&path_main, true)?;

            // Copy the entire file
            app.handle_dispatch_editors(&[SetSelectionMode(SelectionMode::Line), SelectAll, Copy])?;

            // Open foo.rs
            app.open_file(&path_foo, true)?;

            // Copy the entire file
            app.handle_dispatch_editors(&[SetSelectionMode(SelectionMode::Line), SelectAll, Copy])?;

            // Open main.rs
            app.open_file(&path_main, true)?;

            // Select the entire file and paste
            app.handle_dispatch_editors(&[SelectAll, Paste])?;

            // Expect the content of main.rs to be that of foo.rs
            let content_main = app.get_file_content(&path_main);
            let content_foo = app.get_file_content(&path_foo);
            assert_eq!(content_main, content_foo);
            Ok(())
        })
    }

    #[test]
    #[serial]
    fn copy_replace() -> anyhow::Result<()> {
        run_test(|mut app, temp_dir| {
            let path_main = temp_dir.join("src/main.rs")?;
            app.open_file(&path_main, true)?;

            app.handle_dispatch_editors(&[
                SetContent("fn main() { let x = 1; }".to_string()),
                SetSelectionMode(SelectionMode::Token),
                Copy,
                MoveSelection(Movement::Next),
                Replace,
            ])?;

            assert_eq!(app.get_file_content(&path_main), "fn fn() { let x = 1; }");

            app.handle_dispatch_editors(&[Replace])?;

            assert_eq!(app.get_file_content(&path_main), "fn main() { let x = 1; }");
            assert_eq!(app.get_selected_texts(&path_main), vec!["main"]);

            Ok(())
        })
    }

    #[test]
    #[serial]
    fn copy_paste() -> anyhow::Result<()> {
        run_test(|mut app, temp_dir| {
            let path_main = temp_dir.join("src/main.rs")?;
            app.open_file(&path_main, true)?;

            app.handle_dispatch_editors(&[
                SetContent("fn main() { let x = 1; }".to_string()),
                SetSelectionMode(SelectionMode::Token),
                Copy,
                MoveSelection(Movement::Next),
                Paste,
            ])?;

            assert_eq!(app.get_file_content(&path_main), "fn fn() { let x = 1; }");
            assert_eq!(app.get_selected_texts(&path_main), vec![""]);

            app.handle_dispatch_editors(&[MoveSelection(Movement::Next), Paste])?;

            assert_eq!(app.get_file_content(&path_main), "fn fn(fn { let x = 1; }");
            Ok(())
        })
    }

    #[test]
    #[serial]
    fn cut_paste() -> anyhow::Result<()> {
        run_test(|mut app, temp_dir| {
            let path_main = temp_dir.join("src/main.rs")?;
            app.open_file(&path_main, true)?;

            app.handle_dispatch_editors(&[
                SetContent("fn main() { let x = 1; }".to_string()),
                SetSelectionMode(SelectionMode::Token),
                Cut,
            ])?;

            assert_eq!(app.get_file_content(&path_main), " main() { let x = 1; }");

            app.handle_dispatch_editors(&[MoveSelection(Movement::Current)])?;

            assert_eq!(app.get_selected_texts(&path_main), vec!["main"]);

            app.handle_dispatch_editors(&[Paste])?;

            assert_eq!(app.get_file_content(&path_main), " fn() { let x = 1; }");

            Ok(())
        })
    }

    #[test]
    #[serial]
    fn highlight_mode_cut() -> anyhow::Result<()> {
        run_test(|mut app, temp_dir| {
            let path_main = temp_dir.join("src/main.rs")?;
            app.open_file(&path_main, true)?;

            app.handle_dispatch_editors(&[
                SetContent("fn f(){ let x = S(a); let y = S(b); }".to_string()),
                SetSelectionMode(SelectionMode::Token),
                ToggleHighlightMode,
                MoveSelection(Movement::Next),
                MoveSelection(Movement::Next),
                MoveSelection(Movement::Next),
            ])?;

            assert_eq!(app.get_selected_texts(&path_main), vec!["fn f()"]);

            app.handle_dispatch_editors(&[Cut])?;

            assert_eq!(
                app.get_file_content(&path_main),
                "{ let x = S(a); let y = S(b); }"
            );

            app.handle_dispatch_editors(&[Paste])?;

            assert_eq!(
                app.get_file_content(&path_main),
                "fn f(){ let x = S(a); let y = S(b); }"
            );

            Ok(())
        })
    }

    #[test]
    #[serial]
    fn highlight_mode_copy() -> anyhow::Result<()> {
        run_test(|mut app, temp_dir| {
            let path_main = temp_dir.join("src/main.rs")?;
            app.open_file(&path_main, true)?;

            app.handle_dispatch_editors(&[
                SetContent("fn f(){ let x = S(a); let y = S(b); }".to_string()),
                SetSelectionMode(SelectionMode::Token),
                ToggleHighlightMode,
                MoveSelection(Movement::Next),
                MoveSelection(Movement::Next),
                MoveSelection(Movement::Next),
            ])?;
            assert_eq!(app.get_selected_texts(&path_main), vec!["fn f()"]);
            app.handle_dispatch_editors(&[Copy, MoveSelection(Movement::Next)])?;
            assert_eq!(app.get_selected_texts(&path_main), vec!["{"]);
            app.handle_dispatch_editors(&[Paste])?;
            assert_eq!(
                app.get_file_content(&path_main),
                "fn f()fn f() let x = S(a); let y = S(b); }"
            );
            Ok(())
        })
    }

    #[test]
    #[serial]
    fn highlight_mode_replace() -> anyhow::Result<()> {
        run_test(|mut app, temp_dir| {
            let path_main = temp_dir.join("src/main.rs")?;
            app.open_file(&path_main, true)?;

            app.handle_dispatch_editors(&[
                SetContent("fn f(){ let x = S(a); let y = S(b); }".to_string()),
                SetSelectionMode(SelectionMode::Token),
                ToggleHighlightMode,
                MoveSelection(Movement::Next),
                MoveSelection(Movement::Next),
                MoveSelection(Movement::Next),
            ])?;

            assert_eq!(app.get_selected_texts(&path_main), vec!["fn f()"]);

            app.handle_dispatch_editors(&[
                Copy,
                MatchLiteral("{".to_string()),
                SetSelectionMode(SelectionMode::SyntaxTree),
            ])?;

            assert_eq!(
                app.get_selected_texts(&path_main),
                vec!["{ let x = S(a); let y = S(b); }"]
            );

            app.handle_dispatch_editors(&[Replace])?;

            assert_eq!(app.get_file_content(&path_main), "fn f()fn f()");

            Ok(())
        })
    }

    #[test]
    #[serial]
    fn highlight_mode_paste() -> anyhow::Result<()> {
        run_test(|mut app, temp_dir| {
            let path_main = temp_dir.join("src/main.rs")?;
            app.open_file(&path_main, true)?;

            app.handle_dispatch_editors(&[
                SetContent("fn f(){ let x = S(a); let y = S(b); }".to_string()),
                SetSelectionMode(SelectionMode::Token),
                ToggleHighlightMode,
                Copy,
            ])?;

            assert_eq!(app.get_selected_texts(&path_main), vec!["fn"]);

            app.handle_dispatch_editors(&[
                ToggleHighlightMode,
                MoveSelection(Movement::Next),
                MoveSelection(Movement::Next),
                MoveSelection(Movement::Next),
            ])?;

            assert_eq!(app.get_selected_texts(&path_main), vec!["fn f()"]);

            app.handle_dispatch_editors(&[Paste])?;

            assert_eq!(
                app.get_file_content(&path_main),
                "fn{ let x = S(a); let y = S(b); }"
            );

            Ok(())
        })
    }

    #[test]
    fn esc_should_close_signature_help() -> anyhow::Result<()> {
        run_test(|mut app, temp_dir| {
            let path_main = temp_dir.join("src/main.rs")?;
            app.open_file(&path_main, true)?;

            assert_eq!(app.components().len(), 1);

            app.handle_dispatch_editors(&[
                SetContent("fn f(){ let x = S(a); let y = S(b); }".to_string()),
                SetSelectionMode(SelectionMode::Token),
                EnterInsertMode(Direction::End),
            ])?;

            let component_id = app.components()[0].borrow().id();
            app.handle_lsp_notification(LspNotification::SignatureHelp(
                crate::lsp::process::ResponseContext {
                    component_id,
                    request_kind: None,
                    description: None,
                },
                Some(crate::lsp::signature_help::SignatureHelp {
                    signatures: [SignatureInformation {
                        label: "Signature Help".to_string(),
                        documentation: Some(crate::lsp::documentation::Documentation {
                            content: "spongebob".to_string(),
                        }),
                        active_parameter_byte_range: None,
                    }]
                    .to_vec(),
                }),
            ))?;
            assert_eq!(app.components().len(), 2);

            app.handle_dispatch(HandleKeyEvent(key!("esc")))?;
            assert_eq!(app.components().len(), 1);

            Ok(())
        })
    }

    #[test]
    pub fn repo_git_hunks() -> Result<(), anyhow::Error> {
        run_test(|mut app, temp_dir| {
            let path_main = temp_dir.join("src/main.rs")?;
            let path_foo = temp_dir.join("src/foo.rs")?;
            let path_new_file = temp_dir.join_as_path_buf("new_file.md");

            app.handle_dispatches(
                [
                    // Delete the first line of main.rs
                    OpenFile {
                        path: path_main.clone(),
                    },
                    DispatchEditor(SetSelectionMode(SelectionMode::Line)),
                    DispatchEditor(Kill),
                    // Insert a comment at the first line of foo.rs
                    OpenFile {
                        path: path_foo.clone(),
                    },
                    DispatchEditor(Insert("// Hello".to_string())),
                    // Save the files,
                    SaveAll,
                    // Add a new file
                    AddPath(path_new_file.clone()),
                    // Get the repo hunks
                    GetRepoGitHunks,
                ]
                .to_vec(),
            )?;

            fn strs_to_strings(strs: &[&str]) -> Option<Info> {
                Some(Info::new(
                    strs.iter().map(|s| s.to_string()).join("\n").to_string(),
                ))
            }

            let expected_quickfixes = [
                QuickfixListItem::new(
                    Location {
                        path: path_new_file.try_into()?,
                        range: Position { line: 0, column: 0 }..Position { line: 0, column: 0 },
                    },
                    strs_to_strings(&["[This file is untracked by Git]"]),
                ),
                QuickfixListItem::new(
                    Location {
                        path: path_foo,
                        range: Position { line: 0, column: 0 }..Position { line: 1, column: 0 },
                    },
                    strs_to_strings(&["pub struct Foo {", "// Hellopub struct Foo {"]),
                ),
                QuickfixListItem::new(
                    Location {
                        path: path_main,
                        range: Position { line: 0, column: 0 }..Position { line: 0, column: 0 },
                    },
                    strs_to_strings(&["mod foo;"]),
                ),
            ];
            let actual_quickfixes = app
                .get_quickfixes()
                .into_iter()
                .map(|quickfix| {
                    let info = quickfix
                        .info()
                        .as_ref()
                        .map(|info| info.clone().set_decorations(Vec::new()));
                    quickfix.set_info(info)
                })
                .collect_vec();
            assert_eq!(actual_quickfixes, expected_quickfixes);

            Ok(())
        })
    }

    #[test]
    pub fn non_git_ignored_files() -> Result<(), anyhow::Error> {
        run_test(|mut app, temp_dir| {
            let path_git_ignore = temp_dir.join(".gitignore")?;

            app.handle_dispatches(
                [
                    // Ignore *.txt files
                    OpenFile {
                        path: path_git_ignore.clone(),
                    },
                    DispatchEditor(Insert("*.txt\n".to_string())),
                    SaveAll,
                    // Add new txt file
                    AddPath(temp_dir.join_as_path_buf("temp.txt")),
                    // Add a new Rust file
                    AddPath(temp_dir.join_as_path_buf("src/rust.rs")),
                ]
                .to_vec(),
            )?;

            let paths = crate::git::GitRepo::try_from(&temp_dir)?.non_git_ignored_files()?;

            // Expect all the paths are files, not directory for example
            assert!(paths.iter().all(|file| file.is_file()));

            let paths = paths
                .into_iter()
                .flat_map(|path| path.display_relative_to(&temp_dir))
                .collect_vec();

            // Expect "temp.txt" is not in the list, since it is git-ignored
            assert!(!paths.contains(&"temp.txt".to_string()));

            // Expect the unstaged file "src/rust.rs" is in the list
            assert!(paths.contains(&"src/rust.rs".to_string()));

            // Expect the staged file "main.rs" is in the list
            assert!(paths.contains(&"src/main.rs".to_string()));

            Ok(())
        })
    }

    #[test]
    fn align_view_bottom_with_outbound_parent_lines() -> anyhow::Result<()> {
        run_test(|mut app, temp_dir| {
            let path_main = temp_dir.join_as_path_buf("src/main.rs");

            app.handle_dispatches(
                [
                    Dispatch::SetGlobalTitle("[GLOBAL TITLE]".to_string()),
                    OpenFile {
                        path: path_main.try_into()?,
                    },
                    TerminalDimensionChanged(Dimension {
                        width: 200,
                        height: 6,
                    }),
                ]
                .to_vec(),
            )?;
            app.handle_dispatch_editors(&[
                SetSelectionMode(SelectionMode::Line),
                SelectAll,
                Kill,
                Insert(
                    "
fn first () {
  second();
  third();
  fourth(); // this line is long
  fifth();
}"
                    .trim()
                    .to_string(),
                ),
                DispatchEditor::MatchLiteral("fifth()".to_string()),
                AlignViewTop,
            ])?;

            let result = app.get_grid()?;
            assert_eq!(
                result.to_string(),
                "
src/main.rs 🦀
1│fn first () {
5│  █ifth();
6│}

[GLOBAL TITLE]
"
                .trim()
            );

            app.handle_dispatch_editors(&[AlignViewBottom])?;

            let result = app.get_grid()?;
            assert_eq!(
                result.to_string(),
                "
src/main.rs 🦀
1│fn first () {
3│  third();
4│  fourth(); // this line is long
5│  █ifth();
[GLOBAL TITLE]
"
                .trim()
            );

            // Resize the terminal dimension sucht that the fourth line will be wrapped
            app.handle_dispatches(
                [
                    TerminalDimensionChanged(Dimension {
                        width: 20,
                        height: 6,
                    }),
                    DispatchEditor(AlignViewBottom),
                ]
                .to_vec(),
            )?;

            let result = app.get_grid()?;
            assert_eq!(
                result.to_string(),
                "
src/main.rs 🦀
1│fn first () {
4│  fourth(); //
↪│this line is long
5│  █ifth();
[GLOBAL TITLE]
"
                .trim()
            );
            Ok(())
        })
    }

    #[test]
    fn navigation() -> Result<(), anyhow::Error> {
        run_test(|mut app, temp_dir| {
            let file = |filename: &str| -> anyhow::Result<CanonicalizedPath> {
                temp_dir.join_as_path_buf(filename).try_into()
            };
            let open = |filename: &str| -> anyhow::Result<Dispatch> {
                Ok(OpenFile {
                    path: file(filename)?,
                })
            };

            app.handle_dispatches(
                [
                    open("src/main.rs")?,
                    open("src/foo.rs")?,
                    open(".gitignore")?,
                    open("Cargo.toml")?,
                    // Open "Cargo.toml" again to test that the navigation tree does not take duplicated entry
                    open("Cargo.toml")?,
                ]
                .to_vec(),
            )?;

            assert_eq!(app.get_current_file_path(), Some(file("Cargo.toml")?));
            app.handle_dispatches([SetGlobalMode(Some(GlobalMode::FileNavigation))].to_vec())?;

            app.handle_dispatch_editors(&[
                MoveSelection(Movement::Previous),
                MoveSelection(Movement::Previous),
            ])?;
            assert_eq!(app.get_current_file_path(), Some(file("src/foo.rs")?));

            app.handle_dispatches(
                [
                    // After moving back, open "src/foo.rs" again
                    // This is to make sure that "src/foo.rs" will not be
                    // added as a new entry
                    open("src/foo.rs")?,
                    open("Cargo.lock")?,
                ]
                .to_vec(),
            )?;
            assert_eq!(app.get_current_file_path(), Some(file("Cargo.lock")?));

            app.handle_dispatch_editors(&[MoveSelection(Movement::Previous)])?;
            assert_eq!(app.get_current_file_path(), Some(file("src/foo.rs")?));

            app.handle_dispatch_editors(&[MoveSelection(Movement::Next)])?;
            assert_eq!(app.get_current_file_path(), Some(file("Cargo.lock")?));

            let expected_display = "
* 1-3 [HEAD] Cargo.lock
| * 0-4 Cargo.toml
| * 0-3 .gitignore
|/
* 1-2 src/foo.rs
* 1-1 src/main.rs
* 1-0 [SAVED]
"
            .trim()
            .to_string();
            pretty_assertions::assert_eq!(app.get_current_info().unwrap(), expected_display);

            app.handle_dispatch_editors(&[MoveSelection(Movement::Down)])?;
            assert_eq!(app.get_current_file_path(), Some(file("Cargo.toml")?));

            let expected_display = "
* 0-4 [HEAD] Cargo.toml
* 0-3 .gitignore
| * 1-3 Cargo.lock
|/
* 0-2 src/foo.rs
* 0-1 src/main.rs
* 0-0 [SAVED]
"
            .trim()
            .to_string();
            pretty_assertions::assert_eq!(app.get_current_info().unwrap(), expected_display);

            app.handle_dispatch_editors(&[MoveSelection(Movement::Up)])?;
            assert_eq!(app.get_current_file_path(), Some(file("Cargo.lock")?));

            Ok(())
        })
    }
}
