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
        app::{App, Dispatch},
        components::{
            editor::{Direction, DispatchEditor, Movement},
            suggestive_editor::Info,
        },
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
            app.handle_dispatch_editors(&[SelectWholeFile, Copy])?;

            // Open foo.rs
            app.open_file(&path_foo, true)?;

            // Copy the entire file
            app.handle_dispatch_editors(&[SelectWholeFile, Copy])?;

            // Open main.rs
            app.open_file(&path_main, true)?;

            // Select the entire file and paste
            app.handle_dispatch_editors(&[SelectWholeFile, Paste])?;

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
                SetSelectionMode(SelectionMode::BottomNode),
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
                SetSelectionMode(SelectionMode::BottomNode),
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
                SetSelectionMode(SelectionMode::BottomNode),
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
                SetSelectionMode(SelectionMode::BottomNode),
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
                SetSelectionMode(SelectionMode::BottomNode),
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
                SetSelectionMode(SelectionMode::BottomNode),
                ToggleHighlightMode,
                MoveSelection(Movement::Next),
                MoveSelection(Movement::Next),
                MoveSelection(Movement::Next),
            ])?;

            assert_eq!(app.get_selected_texts(&path_main), vec!["fn f()"]);

            app.handle_dispatch_editors(&[
                Copy,
                SetSelectionMode(SelectionMode::TopNode),
                MoveSelection(Movement::Next),
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
                SetSelectionMode(SelectionMode::BottomNode),
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
                SetSelectionMode(SelectionMode::BottomNode),
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
                    Dispatch::TerminalDimensionChanged(crate::app::Dimension {
                        width: 20,
                        height: 5,
                    }),
                ]
                .to_vec(),
            )?;
            app.handle_dispatch_editors(&[
                SelectWholeFile,
                Kill,
                Insert(
                    "
fn main () {
  foo();
  bar();
  spam();
}"
                    .trim()
                    .to_string(),
                ),
                DispatchEditor::MatchLiteral("spam()".to_string()),
                AlignViewTop,
            ])?;

            let result = app.get_grid()?;
            assert_eq!(
                result.grid.to_string(),
                "
src/main.rs 🦀
1│fn main () {
4│  spam();
5│}
[GLOBAL TITLE]
"
                .trim()
            );
            assert_eq!(result.cursor.unwrap().position(), &Position::new(2, 4));

            app.handle_dispatch_editors(&[AlignViewBottom])?;

            let result = app.get_grid()?;
            assert_eq!(
                result.grid.to_string(),
                "
src/main.rs 🦀
1│fn main () {
3│  bar();
4│  spam();
[GLOBAL TITLE]
"
                .trim()
            );
            assert_eq!(result.cursor.unwrap().position(), &Position::new(3, 4));
            Ok(())
        })
    }
}
