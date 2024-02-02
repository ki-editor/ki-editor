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
            app.handle_dispatch_editors(&[
                SetSelectionMode(SelectionMode::LineTrimmed),
                SelectAll,
                Copy,
            ])?;

            // Open foo.rs
            app.open_file(&path_foo, true)?;

            // Copy the entire file
            app.handle_dispatch_editors(&[
                SetSelectionMode(SelectionMode::LineTrimmed),
                SelectAll,
                Copy,
            ])?;

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
                SetSelectionMode(SelectionMode::BottomNode),
                Copy,
                MoveSelection(Movement::Next),
                ReplaceSelectionWithCopiedText,
            ])?;

            assert_eq!(app.get_file_content(&path_main), "fn fn() { let x = 1; }");

            app.handle_dispatch_editors(&[ReplaceSelectionWithCopiedText])?;

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
                EnterNormalMode,
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
                MatchLiteral("{".to_string()),
                SetSelectionMode(SelectionMode::TopNode),
            ])?;

            assert_eq!(
                app.get_selected_texts(&path_main),
                vec!["{ let x = S(a); let y = S(b); }"]
            );

            app.handle_dispatch_editors(&[ReplaceSelectionWithCopiedText])?;

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
    #[serial]
    fn multi_paste() -> anyhow::Result<()> {
        run_test(|mut app, temp_dir| {
            let path_main = temp_dir.join("src/main.rs")?;
            app.handle_dispatch(OpenFile {
                path: path_main.clone(),
            })?;
            app.handle_dispatch_editors(&[
                SetContent("fn f(){ let x = S(spongebob_squarepants); let y = S(b); }".to_string()),
                MatchLiteral("let x = S(spongebob_squarepants);".to_owned()),
                SetSelectionMode(SelectionMode::SyntaxTree),
            ])?;
            app.handle_dispatch_editors(&[
                CursorAddToAllSelections,
                MoveSelection(Movement::Down),
                MoveSelection(Movement::Next),
            ])?;
            assert_eq!(
                app.get_selected_texts(&path_main),
                vec!["S(spongebob_squarepants)", "S(b)"]
            );

            app.handle_dispatch_editors(&[
                Cut,
                EnterInsertMode(Direction::Start),
                Insert("Some(".to_owned()),
                Paste,
                Insert(")".to_owned()),
            ])?;

            assert_eq!(
                app.get_file_content(&path_main),
                "fn f(){ let x = Some(S(spongebob_squarepants)); let y = Some(S(b)); }"
            );

            app.handle_dispatch_editors(&[CursorKeepPrimaryOnly])?;
            app.handle_dispatch(SetClipboardContent(".hello".to_owned()))?;
            app.handle_dispatches(
                [
                    DispatchEditor(CursorKeepPrimaryOnly),
                    SetClipboardContent(".hello".to_owned()),
                    DispatchEditor(Paste),
                ]
                .to_vec(),
            )?;

            assert_eq!(
                app.get_file_content(&path_main),
                "fn f(){ let x = Some(S(spongebob_squarepants).hello; let y = Some(S(b)); }"
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
                    scope: None,
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
                    DispatchEditor(SetSelectionMode(SelectionMode::LineTrimmed)),
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
                SetSelectionMode(SelectionMode::LineTrimmed),
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
    fn selection_history_contiguous() -> Result<(), anyhow::Error> {
        run_test(|mut app, temp_dir| {
            let file = |filename: &str| -> anyhow::Result<CanonicalizedPath> {
                temp_dir.join_as_path_buf(filename).try_into()
            };
            let open = |filename: &str| -> anyhow::Result<Dispatch> {
                Ok(OpenFile {
                    path: file(filename)?,
                })
            };

            app.handle_dispatch(open("src/main.rs")?)?;

            app.handle_dispatch_editors(&[SetSelectionMode(SelectionMode::LineTrimmed)])?;
            assert_eq!(app.get_selected_texts(&file("src/main.rs")?), ["mod foo;"]);

            app.handle_dispatch_editors(&[SetSelectionMode(SelectionMode::Character)])?;
            assert_eq!(app.get_selected_texts(&file("src/main.rs")?), ["m"]);

            app.handle_dispatches(
                [
                    SetGlobalMode(Some(GlobalMode::SelectionHistoryContiguous)),
                    DispatchEditor(MoveSelection(Movement::Previous)),
                ]
                .to_vec(),
            )?;
            assert_eq!(app.get_selected_texts(&file("src/main.rs")?), ["mod foo;"]);

            app.handle_dispatches([DispatchEditor(MoveSelection(Movement::Next))].to_vec())?;
            assert_eq!(app.get_selected_texts(&file("src/main.rs")?), ["m"]);

            Ok(())
        })
    }

    #[test]
    fn selection_history_file() -> Result<(), anyhow::Error> {
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
                    // Move some selection to test that this movement ignore movement within the same file
                    DispatchEditor(SetSelectionMode(SelectionMode::LineTrimmed)),
                    DispatchEditor(MoveSelection(Movement::Next)),
                    // Open "Cargo.toml" again to test that the navigation tree does not take duplicated entry
                    open("Cargo.toml")?,
                ]
                .to_vec(),
            )?;

            assert_eq!(app.get_current_file_path(), Some(file("Cargo.toml")?));
            app.handle_dispatches(
                [SetGlobalMode(Some(GlobalMode::SelectionHistoryFile))].to_vec(),
            )?;
            app.handle_dispatch_editors(&[MoveSelection(Movement::Previous)])?;
            assert_eq!(app.get_current_file_path(), Some(file(".gitignore")?));

            app.handle_dispatch_editors(&[MoveSelection(Movement::Previous)])?;
            assert_eq!(app.get_current_file_path(), Some(file("src/foo.rs")?));

            // Test Movement::Next to src/foo.rs where no selection has been moved in src/foo.rs
            app.handle_dispatch_editors(&[
                MoveSelection(Movement::Previous),
                MoveSelection(Movement::Next),
            ])?;
            assert_eq!(app.get_current_file_path(), Some(file("src/foo.rs")?));

            app.handle_dispatches(
                [
                    // After moving back, open "src/foo.rs" again
                    // This is to make sure that "src/foo.rs" will not be
                    // added as a new entry
                    open("src/foo.rs")?,
                    open("Cargo.lock")?,
                    // Move some selection to test that the modified selection set is preserved when going to the next FileSelectionSet in the history
                    DispatchEditor(SetSelectionMode(SelectionMode::LineTrimmed)),
                    DispatchEditor(MoveSelection(Movement::Next)),
                    SetGlobalMode(Some(GlobalMode::SelectionHistoryFile)),
                ]
                .to_vec(),
            )?;
            assert_eq!(app.get_current_file_path(), Some(file("Cargo.lock")?));
            let cargo_lock_selection_set = app.get_current_selection_set();

            app.handle_dispatch_editors(&[MoveSelection(Movement::Previous)])?;
            assert_eq!(app.get_current_file_path(), Some(file("src/foo.rs")?));
            app.handle_dispatch_editors(&[MoveSelection(Movement::Next)])?;
            assert_eq!(app.get_current_file_path(), Some(file("Cargo.lock")?));
            assert_eq!(app.get_current_selection_set(), cargo_lock_selection_set);

            app.handle_dispatch(Dispatch::HandleKeyEvent(key!("esc")))?;
            assert_eq!(app.context().mode(), None);

            Ok(())
        })
    }

    #[test]
    fn global_bookmarks() -> Result<(), anyhow::Error> {
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
                    DispatchEditor(SetSelectionMode(SelectionMode::Word)),
                    DispatchEditor(ToggleBookmark),
                    open("src/foo.rs")?,
                    DispatchEditor(SetSelectionMode(SelectionMode::Word)),
                    DispatchEditor(ToggleBookmark),
                    SetQuickfixList(crate::quickfix_list::QuickfixListType::Bookmark),
                ]
                .to_vec(),
            )?;
            assert_eq!(
                app.get_quickfixes(),
                [
                    QuickfixListItem::new(
                        Location {
                            path: file("src/foo.rs")?,
                            range: Position { line: 0, column: 0 }..Position { line: 0, column: 3 },
                        },
                        None,
                    ),
                    QuickfixListItem::new(
                        Location {
                            path: file("src/main.rs")?,
                            range: Position { line: 0, column: 0 }..Position { line: 0, column: 3 },
                        },
                        None,
                    ),
                ],
            );

            Ok(())
        })
    }
}
