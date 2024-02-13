/// NOTE: all test cases that involves the clipboard should not be run in parallel
///   otherwise the the test suite will fail because multiple tests are trying to
///   access the clipboard at the same time.
#[cfg(test)]
mod test_app {
    use itertools::Itertools;
    use my_proc_macros::key;
    use pretty_assertions::assert_eq;
    use serial_test::serial;

    use std::{
        path::PathBuf,
        sync::{Arc, Mutex},
    };
    use Dispatch::*;
    use DispatchEditor::*;
    use Movement::*;
    use SelectionMode::*;

    use shared::canonicalized_path::CanonicalizedPath;

    use crate::{
        app::{
            App, Dimension, Dispatch, GlobalSearchConfigUpdate, GlobalSearchFilterGlob,
            LocalSearchConfigUpdate, Scope,
        },
        components::{
            component::ComponentId,
            editor::{Direction, DispatchEditor, Movement},
            suggestive_editor::Info,
        },
        context::{GlobalMode, LocalSearchConfigMode},
        frontend::mock::MockFrontend,
        integration_test::integration_test::TestRunner,
        list::grep::RegexConfig,
        lsp::{process::LspNotification, signature_help::SignatureInformation},
        position::Position,
        quickfix_list::{Location, QuickfixListItem},
        selection::SelectionMode,
        selection_mode::inside::InsideKind,
    };

    enum Step {
        App(Dispatch),
        WithApp(Box<dyn Fn(&App<MockFrontend>) -> Dispatch>),
        Expect(ExpectKind),
        Editor(DispatchEditor),
        ExpectLater(Box<dyn Fn() -> ExpectKind>),
        ExpectCustom(Box<dyn Fn()>),
    }

    #[derive(Debug)]
    enum ExpectKind {
        Not(Box<ExpectKind>),
        CurrentFileContent(&'static str),
        FileContent(CanonicalizedPath, String),
        FileContentEqual(CanonicalizedPath, CanonicalizedPath),
        CurrentSelectedTexts(&'static [&'static str]),
        ComponentsLength(usize),
        Quickfixes(Box<[QuickfixListItem]>),
        Grid(&'static str),
        CurrentPath(CanonicalizedPath),
        LocalSearchConfigSearches(&'static [&'static str]),
        LocalSearchConfigReplacements(&'static [&'static str]),
        GlobalSearchConfigSearches(&'static [&'static str]),
        GlobalSearchConfigReplacements(&'static [&'static str]),
        GlobalSearchConfigIncludeGlobs(&'static [&'static str]),
        GlobalSearchConfigExcludeGlobs(&'static [&'static str]),
        FileContentContains(CanonicalizedPath, &'static str),
    }
    fn log<T: std::fmt::Debug>(s: T) {
        println!("===========\n{s:?}",);
    }
    impl ExpectKind {
        fn run(&self, app: &mut App<MockFrontend>) {
            log(self);
            let (result, context) = self.get_result(app).unwrap();
            assert!(result, "{context}",)
        }
        fn get_result(&self, app: &mut App<MockFrontend>) -> anyhow::Result<(bool, String)> {
            let context = app.context();
            fn contextualize<T: PartialEq + std::fmt::Debug>(a: T, b: T) -> (bool, String) {
                (a == b, format!("{a:?} == {b:?}",))
            }
            fn to_vec(strs: &[&str]) -> Vec<String> {
                strs.into_iter().map(|t| t.to_string()).collect()
            }
            Ok(match self {
                CurrentFileContent(expected_content) => {
                    contextualize(app.get_current_file_content(), expected_content.to_string())
                }
                FileContent(path, expected_content) => {
                    contextualize(app.get_file_content(path), expected_content.clone())
                }
                FileContentEqual(left, right) => {
                    contextualize(app.get_file_content(&left), app.get_file_content(&right))
                }
                CurrentSelectedTexts(selected_texts) => {
                    contextualize(app.get_current_selected_texts().1, to_vec(selected_texts))
                }
                ComponentsLength(length) => contextualize(app.components().len(), *length),
                Quickfixes(expected_quickfixes) => contextualize(
                    app.get_quickfixes()
                        .into_iter()
                        .map(|quickfix| {
                            let info = quickfix
                                .info()
                                .as_ref()
                                .map(|info| info.clone().set_decorations(Vec::new()));
                            quickfix.set_info(info)
                        })
                        .collect_vec()
                        .into_boxed_slice(),
                    expected_quickfixes.clone(),
                ),

                Grid(grid) => contextualize(app.get_grid()?.to_string(), grid.to_string()),
                CurrentPath(path) => {
                    contextualize(app.get_current_file_path().unwrap(), path.clone())
                }
                LocalSearchConfigSearches(searches) => {
                    contextualize(context.local_search_config().searches(), to_vec(searches))
                }
                LocalSearchConfigReplacements(replacements) => contextualize(
                    context.local_search_config().replacements(),
                    to_vec(replacements),
                ),
                GlobalSearchConfigSearches(searches) => contextualize(
                    context.get_local_search_config(Scope::Global).searches(),
                    to_vec(searches),
                ),
                GlobalSearchConfigReplacements(replacements) => contextualize(
                    context
                        .get_local_search_config(Scope::Global)
                        .replacements(),
                    to_vec(replacements),
                ),
                GlobalSearchConfigIncludeGlobs(include_globs) => contextualize(
                    context.global_search_config().include_globs(),
                    to_vec(include_globs),
                ),
                GlobalSearchConfigExcludeGlobs(exclude_globs) => contextualize(
                    context.global_search_config().exclude_globs(),
                    to_vec(exclude_globs),
                ),
                FileContentContains(path, substring) => {
                    let left = app.get_file_content(path);
                    (
                        left.contains(substring),
                        format!("{left:?} contains {substring:?}"),
                    )
                }
                Not(expect_kind) => {
                    let (result, context) = expect_kind.get_result(app)?;
                    (!result, format!("NOT ({context})"))
                }
            })
        }
    }

    use ExpectKind::*;
    use Step::*;
    struct State {
        temp_dir: CanonicalizedPath,
        main_rs: CanonicalizedPath,
        foo_rs: CanonicalizedPath,
        git_ignore: CanonicalizedPath,
    }
    impl State {
        fn main_rs(&self) -> CanonicalizedPath {
            self.main_rs.clone()
        }

        fn foo_rs(&self) -> CanonicalizedPath {
            self.foo_rs.clone()
        }

        fn new_path(&self, path: &str) -> PathBuf {
            self.temp_dir.to_path_buf().join(path)
        }

        fn gitignore(&self) -> CanonicalizedPath {
            self.git_ignore.clone()
        }

        fn temp_dir(&self) -> CanonicalizedPath {
            self.temp_dir.clone()
        }
    }

    fn execute_test(callback: impl Fn(State) -> Box<[Step]>) -> anyhow::Result<()> {
        run_test(|mut app, temp_dir| {
            let steps = {
                callback(State {
                    main_rs: temp_dir.join("src/main.rs").unwrap(),
                    foo_rs: temp_dir.join("src/foo.rs").unwrap(),
                    git_ignore: temp_dir.join(".gitignore").unwrap(),
                    temp_dir,
                })
            };

            for step in steps.into_iter() {
                match step.to_owned() {
                    Step::App(dispatch) => {
                        log(dispatch);
                        app.handle_dispatch(dispatch.to_owned())?
                    }
                    Step::Expect(expect_kind) => expect_kind.run(&mut app),
                    ExpectLater(f) => f().run(&mut app),
                    Editor(dispatch) => {
                        log(dispatch);
                        app.handle_dispatch_editor(dispatch.to_owned())?
                    }
                    WithApp(f) => {
                        let dispatch = f(&app);
                        app.handle_dispatch(dispatch)?
                    }
                    ExpectCustom(f) => {
                        f();
                    }
                };
            }
            Ok(())
        })
    }

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
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                App(OpenFile(s.foo_rs())),
                Editor(SetSelectionMode(LineTrimmed)),
                Editor(SelectAll),
                Editor(Copy),
                App(OpenFile(s.foo_rs())),
                Editor(SetSelectionMode(LineTrimmed)),
                Editor(SelectAll),
                Editor(Copy),
                App(OpenFile(s.main_rs())),
                Editor(SelectAll),
                Editor(Paste),
                Expect(FileContentEqual(s.main_rs, s.foo_rs)),
            ])
        })
    }

    #[test]
    #[serial]
    fn copy_replace() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("fn main() { let x = 1; }".to_string())),
                Editor(SetSelectionMode(SelectionMode::BottomNode)),
                Editor(Copy),
                Editor(MoveSelection(Movement::Next)),
                Editor(ReplaceSelectionWithCopiedText),
                Expect(CurrentFileContent("fn fn() { let x = 1; }")),
                Editor(ReplaceSelectionWithCopiedText),
                Expect(CurrentSelectedTexts(&["main"])),
            ])
        })
    }

    #[test]
    #[serial]
    fn copy_paste() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("fn main() { let x = 1; }".to_string())),
                Editor(SetSelectionMode(SelectionMode::BottomNode)),
                Editor(Copy),
                Editor(MoveSelection(Movement::Next)),
                Editor(Paste),
                Expect(CurrentFileContent("fn fn() { let x = 1; }")),
                Expect(CurrentSelectedTexts(&[""])),
                Editor(MoveSelection(Next)),
                Editor(Paste),
                Expect(CurrentFileContent("fn fn(fn { let x = 1; }")),
            ])
        })
    }

    #[test]
    #[serial]
    fn cut_paste() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("fn main() { let x = 1; }".to_string())),
                Editor(SetSelectionMode(BottomNode)),
                Editor(Cut),
                Editor(EnterNormalMode),
                Expect(CurrentFileContent(" main() { let x = 1; }")),
                Editor(MoveSelection(Current)),
                Expect(CurrentSelectedTexts(&["main"])),
                Editor(Paste),
                Expect(CurrentFileContent(" fn() { let x = 1; }")),
            ])
        })
    }

    #[test]
    #[serial]
    fn highlight_mode_cut() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent(
                    "fn f(){ let x = S(a); let y = S(b); }".to_string(),
                )),
                Editor(SetSelectionMode(BottomNode)),
                Editor(ToggleHighlightMode),
                Editor(MoveSelection(Next)),
                Editor(MoveSelection(Next)),
                Editor(MoveSelection(Next)),
                Expect(CurrentSelectedTexts(&["fn f()"])),
                Editor(Cut),
                Expect(CurrentFileContent("{ let x = S(a); let y = S(b); }")),
                Editor(Paste),
                Expect(CurrentFileContent("fn f(){ let x = S(a); let y = S(b); }")),
            ])
        })
    }

    #[test]
    #[serial]
    fn highlight_mode_copy() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent(
                    "fn f(){ let x = S(a); let y = S(b); }".to_string(),
                )),
                Editor(SetSelectionMode(SelectionMode::BottomNode)),
                Editor(ToggleHighlightMode),
                Editor(MoveSelection(Movement::Next)),
                Editor(MoveSelection(Movement::Next)),
                Editor(MoveSelection(Movement::Next)),
                Expect(CurrentSelectedTexts(&["fn f()"])),
                Editor(Copy),
                Editor(MoveSelection(Next)),
                Expect(CurrentSelectedTexts(&["{"])),
                Editor(Paste),
                Expect(CurrentFileContent(
                    "fn f()fn f() let x = S(a); let y = S(b); }",
                )),
            ])
        })
    }

    #[test]
    #[serial]
    fn highlight_mode_replace() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent(
                    "fn f(){ let x = S(a); let y = S(b); }".to_string(),
                )),
                Editor(SetSelectionMode(SelectionMode::BottomNode)),
                Editor(ToggleHighlightMode),
                Editor(MoveSelection(Movement::Next)),
                Editor(MoveSelection(Movement::Next)),
                Editor(MoveSelection(Movement::Next)),
                Expect(CurrentSelectedTexts(&["fn f()"])),
                Editor(Copy),
                Editor(MatchLiteral("{".to_string())),
                Editor(SetSelectionMode(SelectionMode::TopNode)),
                Expect(CurrentSelectedTexts(&["{ let x = S(a); let y = S(b); }"])),
                Editor(ReplaceSelectionWithCopiedText),
                Expect(CurrentFileContent("fn f()fn f()")),
            ])
        })
    }

    #[test]
    #[serial]
    fn highlight_mode_paste() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent(
                    "fn f(){ let x = S(a); let y = S(b); }".to_string(),
                )),
                Editor(SetSelectionMode(SelectionMode::BottomNode)),
                Editor(Copy),
                Expect(CurrentSelectedTexts(&["fn"])),
                Editor(ToggleHighlightMode),
                Editor(MoveSelection(Next)),
                Editor(MoveSelection(Next)),
                Editor(MoveSelection(Next)),
                Expect(CurrentSelectedTexts(&["fn f()"])),
                Editor(Paste),
                Expect(CurrentFileContent("fn{ let x = S(a); let y = S(b); }")),
            ])
        })
    }

    #[test]
    #[serial]
    fn multi_paste() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent(
                    "fn f(){ let x = S(spongebob_squarepants); let y = S(b); }".to_string(),
                )),
                Editor(MatchLiteral("let x = S(spongebob_squarepants);".to_owned())),
                Editor(SetSelectionMode(SelectionMode::SyntaxTree)),
                Editor(CursorAddToAllSelections),
                Editor(MoveSelection(Movement::Down)),
                Editor(MoveSelection(Movement::Next)),
                Expect(CurrentSelectedTexts(&["S(spongebob_squarepants)", "S(b)"])),
                Editor(Cut),
                Editor(EnterInsertMode(Direction::Start)),
                Editor(Insert("Some(".to_owned())),
                Editor(Paste),
                Editor(Insert(")".to_owned())),
                Expect(CurrentFileContent(
                    "fn f(){ let x = Some(S(spongebob_squarepants)); let y = Some(S(b)); }",
                )),
                Editor(CursorKeepPrimaryOnly),
                App(SetClipboardContent(".hello".to_owned())),
                Editor(Paste),
                Expect(CurrentFileContent(
                    "fn f(){ let x = Some(S(spongebob_squarepants).hello; let y = Some(S(b)); }",
                )),
            ])
        })
    }

    #[test]
    fn esc_should_close_signature_help() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Expect(ComponentsLength(1)),
                Editor(SetContent(
                    "fn f(){ let x = S(a); let y = S(b); }".to_string(),
                )),
                Editor(SetSelectionMode(SelectionMode::BottomNode)),
                Editor(EnterInsertMode(Direction::End)),
                WithApp(Box::new(|app: &App<MockFrontend>| {
                    HandleLspNotification(LspNotification::SignatureHelp(
                        crate::lsp::process::ResponseContext {
                            component_id: app.components()[0].borrow().id(),
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
                    ))
                })),
                Expect(ComponentsLength(2)),
                App(HandleKeyEvent(key!("esc"))),
                Expect(ComponentsLength(1)),
            ])
        })
    }

    #[test]
    pub fn repo_git_hunks() -> Result<(), anyhow::Error> {
        execute_test(|s| {
            let path_new_file = s.new_path("new_file.md");
            fn strs_to_strings(strs: &[&str]) -> Option<Info> {
                Some(Info::new(
                    strs.iter().map(|s| s.to_string()).join("\n").to_string(),
                ))
            }

            Box::new([
                // Delete the first line of main.rs
                App(OpenFile(s.main_rs().clone())),
                Editor(SetSelectionMode(LineTrimmed)),
                Editor(Kill),
                // Insert a comment at the first line of foo.rs
                App(OpenFile(s.foo_rs().clone())),
                Editor(Insert("// Hello".to_string())),
                // Save the files,
                App(SaveAll),
                // Add a new file
                App(AddPath(path_new_file.display().to_string())),
                // Get the repo hunks
                App(GetRepoGitHunks),
                Step::ExpectLater(Box::new(move || {
                    Quickfixes(Box::new([
                        QuickfixListItem::new(
                            Location {
                                path: path_new_file.clone().try_into().unwrap(),
                                range: Position { line: 0, column: 0 }..Position {
                                    line: 0,
                                    column: 0,
                                },
                            },
                            strs_to_strings(&["[This file is untracked by Git]"]),
                        ),
                        QuickfixListItem::new(
                            Location {
                                path: s.foo_rs(),
                                range: Position { line: 0, column: 0 }..Position {
                                    line: 1,
                                    column: 0,
                                },
                            },
                            strs_to_strings(&["pub struct Foo {", "// Hellopub struct Foo {"]),
                        ),
                        QuickfixListItem::new(
                            Location {
                                path: s.main_rs(),
                                range: Position { line: 0, column: 0 }..Position {
                                    line: 0,
                                    column: 0,
                                },
                            },
                            strs_to_strings(&["mod foo;"]),
                        ),
                    ]))
                })),
            ])
        })
    }

    #[test]
    pub fn non_git_ignored_files() -> Result<(), anyhow::Error> {
        execute_test(|s| {
            let temp_dir = s.temp_dir();
            Box::new([
                // Ignore *.txt files
                App(OpenFile(s.gitignore())),
                Editor(Insert("*.txt\n".to_string())),
                App(SaveAll),
                // Add new txt file
                App(AddPath(s.new_path("temp.txt").display().to_string())),
                // Add a new Rust file
                App(AddPath(s.new_path("src/rust.rs").display().to_string())),
                ExpectCustom(Box::new(move || {
                    let paths = crate::git::GitRepo::try_from(&temp_dir)
                        .unwrap()
                        .non_git_ignored_files()
                        .unwrap();

                    // Expect all the paths are files, not directory for example
                    assert!(paths.iter().all(|file| file.is_file()));

                    let paths = paths
                        .into_iter()
                        .flat_map(|path| path.display_relative_to(&s.temp_dir()))
                        .collect_vec();

                    // Expect "temp.txt" is not in the list, since it is git-ignored
                    assert!(!paths.contains(&"temp.txt".to_string()));

                    // Expect the unstaged file "src/rust.rs" is in the list
                    assert!(paths.contains(&"src/rust.rs".to_string()));

                    // Expect the staged file "main.rs" is in the list
                    assert!(paths.contains(&"src/main.rs".to_string()));
                })),
            ])
        })
    }

    #[test]
    fn align_view_bottom_with_outbound_parent_lines() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(SetGlobalTitle("[GLOBAL TITLE]".to_string())),
                App(OpenFile(s.main_rs())),
                App(TerminalDimensionChanged(Dimension {
                    width: 200,
                    height: 6,
                })),
                Editor(SetSelectionMode(LineTrimmed)),
                Editor(SelectAll),
                Editor(Kill),
                Editor(Insert(
                    "
fn first () {
  second();
  third();
  fourth(); // this line is long
  fifth();
}"
                    .trim()
                    .to_string(),
                )),
                Editor(DispatchEditor::MatchLiteral("fifth()".to_string())),
                Editor(AlignViewTop),
                Expect(ExpectKind::Grid(
                    "
src/main.rs 🦀
1│fn first () {
5│  █ifth();
6│}

[GLOBAL TITLE]
"
                    .trim(),
                )),
                Editor(AlignViewBottom),
                Expect(Grid(
                    "
src/main.rs 🦀
1│fn first () {
3│  third();
4│  fourth(); // this line is long
5│  █ifth();
[GLOBAL TITLE]
"
                    .trim(),
                )),
                // Resize the terminal dimension sucht that the fourth line will be wrapped
                App(TerminalDimensionChanged(Dimension {
                    width: 20,
                    height: 6,
                })),
                Editor(AlignViewBottom),
                Expect(Grid(
                    "
src/main.rs 🦀
1│fn first () {
4│  fourth(); //
↪│this line is long
5│  █ifth();
[GLOBAL TITLE]
"
                    .trim(),
                )),
            ])
        })
    }

    #[test]
    fn selection_history_contiguous() -> Result<(), anyhow::Error> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetSelectionMode(LineTrimmed)),
                Expect(CurrentSelectedTexts(&["mod foo;"])),
                Editor(SetSelectionMode(Character)),
                Expect(CurrentSelectedTexts(&["m"])),
                App(GoToPreviousSelection),
                Expect(CurrentSelectedTexts(&["mod foo;"])),
                App(GoToNextSelection),
                Expect(CurrentSelectedTexts(&["m"])),
                App(GotoLocation(Location {
                    path: s.foo_rs(),
                    range: Position::new(0, 0)..Position::new(0, 4),
                })),
                Expect(ExpectKind::CurrentPath(s.foo_rs())),
                Expect(CurrentSelectedTexts(&["pub "])),
                App(GoToPreviousSelection),
                Expect(CurrentPath(s.main_rs())),
                Expect(CurrentSelectedTexts(&["m"])),
                App(GoToNextSelection),
                Expect(ExpectKind::CurrentPath(s.foo_rs())),
                Expect(CurrentSelectedTexts(&["pub "])),
            ])
        })
    }

    #[test]
    /// TODO: might need to remove this test case
    fn selection_history_file() -> Result<(), anyhow::Error> {
        run_test(|mut app, temp_dir| {
            let file = |filename: &str| -> anyhow::Result<CanonicalizedPath> {
                temp_dir.join_as_path_buf(filename).try_into()
            };
            let open =
                |filename: &str| -> anyhow::Result<Dispatch> { Ok(OpenFile(file(filename)?)) };

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
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetSelectionMode(Word)),
                Editor(ToggleBookmark),
                App(OpenFile(s.foo_rs())),
                Editor(SetSelectionMode(Word)),
                Editor(ToggleBookmark),
                App(SetQuickfixList(
                    crate::quickfix_list::QuickfixListType::Bookmark,
                )),
                Expect(Quickfixes(Box::new([
                    QuickfixListItem::new(
                        Location {
                            path: s.foo_rs(),
                            range: Position { line: 0, column: 0 }..Position { line: 0, column: 3 },
                        },
                        None,
                    ),
                    QuickfixListItem::new(
                        Location {
                            path: s.main_rs(),
                            range: Position { line: 0, column: 0 }..Position { line: 0, column: 3 },
                        },
                        None,
                    ),
                ]))),
            ])
        })
    }

    #[test]
    fn search_config_history() -> Result<(), anyhow::Error> {
        let owner_id = ComponentId::new();
        let update = |scope: Scope, update: LocalSearchConfigUpdate| -> Step {
            App(UpdateLocalSearchConfig {
                owner_id,
                update,
                scope,
                show_legend: true,
            })
        };
        let update_global = |update: GlobalSearchConfigUpdate| -> Step {
            App(UpdateGlobalSearchConfig { owner_id, update })
        };
        use GlobalSearchConfigUpdate::*;
        use GlobalSearchFilterGlob::*;
        use LocalSearchConfigUpdate::*;
        use Scope::*;
        execute_test(|_| {
            Box::new([
                update(Local, SetSearch("L-Search1".to_string())),
                update(Local, SetSearch("L-Search2".to_string())),
                update(Local, SetSearch("L-Search1".to_string())),
                update(Local, SetReplacement("L-Replacement1".to_string())),
                update(Local, SetReplacement("L-Replacement2".to_string())),
                update(Local, SetReplacement("L-Replacement1".to_string())),
                update(Global, SetSearch("G-Search1".to_string())),
                update(Global, SetSearch("G-Search2".to_string())),
                update(Global, SetSearch("G-Search1".to_string())),
                update(Global, SetReplacement("G-Replacement1".to_string())),
                update(Global, SetReplacement("G-Replacement2".to_string())),
                update(Global, SetReplacement("G-Replacement1".to_string())),
                update_global(SetGlob(Exclude, "ExcludeGlob1".to_string())),
                update_global(SetGlob(Exclude, "ExcludeGlob2".to_string())),
                update_global(SetGlob(Exclude, "ExcludeGlob1".to_string())),
                update_global(SetGlob(Include, "IncludeGlob1".to_string())),
                update_global(SetGlob(Include, "IncludeGlob2".to_string())),
                update_global(SetGlob(Include, "IncludeGlob1".to_string())),
                // Expect the histories are stored, where:
                // 1. There's no duplication
                // 2. The insertion order is up-to-date
                Expect(LocalSearchConfigSearches(&["L-Search2", "L-Search1"])),
                Expect(LocalSearchConfigReplacements(&[
                    "L-Replacement2",
                    "L-Replacement1",
                ])),
                Expect(GlobalSearchConfigSearches(&["G-Search2", "G-Search1"])),
                Expect(GlobalSearchConfigReplacements(&[
                    "G-Replacement2",
                    "G-Replacement1",
                ])),
                Expect(GlobalSearchConfigIncludeGlobs(&[
                    "IncludeGlob2",
                    "IncludeGlob1",
                ])),
                Expect(GlobalSearchConfigExcludeGlobs(&[
                    "ExcludeGlob2",
                    "ExcludeGlob1",
                ])),
            ])
        })
    }

    #[test]
    fn global_search_and_replace() -> Result<(), anyhow::Error> {
        execute_test(|s| {
            let owner_id = ComponentId::new();
            let new_dispatch = |update: LocalSearchConfigUpdate| -> Dispatch {
                UpdateLocalSearchConfig {
                    owner_id,
                    update,
                    scope: Scope::Global,
                    show_legend: true,
                }
            };
            let main_rs = s.main_rs();
            let main_rs_initial_content = main_rs.read().unwrap();
            Box::new([
                App(OpenFile(s.foo_rs())),
                App(OpenFile(s.main_rs())),
                // Initiall, expect main.rs and foo.rs to contain the word "foo"
                Expect(FileContentContains(s.main_rs(), "foo")),
                Expect(FileContentContains(s.foo_rs(), "foo")),
                App(new_dispatch(LocalSearchConfigUpdate::SetMode(
                    LocalSearchConfigMode::Regex(RegexConfig {
                        escaped: true,
                        case_sensitive: false,
                        match_whole_word: false,
                    }),
                ))),
                // Replace "foo" with "haha" globally
                App(new_dispatch(LocalSearchConfigUpdate::SetSearch(
                    "foo".to_string(),
                ))),
                App(new_dispatch(LocalSearchConfigUpdate::SetReplacement(
                    "haha".to_string(),
                ))),
                App(Dispatch::Replace {
                    scope: Scope::Global,
                }),
                // Expect main.rs and foo.rs to not contain the word "foo"
                Expect(Not(Box::new(FileContentContains(s.main_rs(), "foo")))),
                Expect(Not(Box::new(FileContentContains(s.foo_rs(), "foo")))),
                // Expect main.rs and foo.rs to contain the word "haha"
                Expect(FileContentContains(s.main_rs(), "haha")),
                Expect(FileContentContains(s.foo_rs(), "haha")),
                // Expect the main.rs buffer to be updated as well
                ExpectLater(Box::new(move || {
                    FileContent(main_rs.clone(), main_rs.read().unwrap())
                })),
                // Apply undo to main_rs
                App(OpenFile(s.main_rs())),
                Editor(Undo),
                // Expect the content of the main.rs buffer to be reverted
                Expect(FileContent(s.main_rs(), main_rs_initial_content)),
            ])
        })
    }

    #[test]
    /// Example: from "hello" -> hello
    fn raise_inside() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("fn main() { (a, b) }".to_string())),
                Editor(MatchLiteral("b".to_string())),
                Editor(SetSelectionMode(Inside(InsideKind::Parentheses))),
                Expect(CurrentSelectedTexts(&["a, b"])),
                Editor(Raise),
                Expect(CurrentFileContent("fn main() { a, b }")),
            ])
        })
    }

    #[test]
    fn toggle_highlight_mode() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent(
                    "fn f(){ let x = S(a); let y = S(b); }".to_string(),
                )),
                Editor(SetSelectionMode(BottomNode)),
                Editor(ToggleHighlightMode),
                Editor(MoveSelection(Next)),
                Editor(MoveSelection(Next)),
                Expect(CurrentSelectedTexts(&["fn f("])),
                // Toggle the second time should inverse the initial_range
                Editor(ToggleHighlightMode),
                Editor(MoveSelection(Next)),
                Expect(CurrentSelectedTexts(&["f("])),
                Editor(Reset),
                Expect(CurrentSelectedTexts(&["f"])),
                Editor(MoveSelection(Next)),
                Expect(CurrentSelectedTexts(&["("])),
            ])
        })
    }

    #[test]
    /// Kill means delete until the next selection
    fn delete_should_kill_if_possible_1() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("fn main() {}".to_string())),
                Editor(SetSelectionMode(BottomNode)),
                Editor(Kill),
                Expect(CurrentFileContent("main() {}")),
                Expect(CurrentSelectedTexts(&["main"])),
            ])
        })
    }

    #[test]
    /// No gap between current and next selection
    fn delete_should_kill_if_possible_2() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("fn main() {}".to_string())),
                Editor(SetSelectionMode(Character)),
                Editor(Kill),
                Expect(CurrentFileContent("n main() {}")),
                Expect(CurrentSelectedTexts(&["n"])),
            ])
        })
    }

    #[test]
    /// No next selection
    fn delete_should_kill_if_possible_3() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("fn main() {}".to_string())),
                Editor(SetSelectionMode(BottomNode)),
                Editor(MoveSelection(Last)),
                Editor(Kill),
                Expect(CurrentFileContent("fn main() {")),
            ])
        })
    }

    #[test]
    /// The selection mode is contiguous
    fn delete_should_kill_if_possible_4() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("fn main(a:A,b:B) {}".to_string())),
                Editor(MatchLiteral("a:A".to_string())),
                Editor(SetSelectionMode(SyntaxTree)),
                Editor(Kill),
                Expect(CurrentFileContent("fn main(b:B) {}")),
                Expect(CurrentSelectedTexts(&["b:B"])),
            ])
        })
    }

    #[test]
    fn delete_should_not_kill_if_not_possible() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("fn maima() {}".to_string())),
                Editor(MatchLiteral("ma".to_string())),
                Editor(Kill),
                Expect(CurrentFileContent("fn ima() {}")),
                // Expect the current selection is the character after "ma"
                Expect(CurrentSelectedTexts(&["i"])),
            ])
        })
    }

    #[test]
    fn toggle_untoggle_bookmark() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("foo bar spam".to_string())),
                Editor(SetSelectionMode(Word)),
                Editor(ToggleBookmark),
                Editor(MoveSelection(Next)),
                Editor(MoveSelection(Next)),
                Editor(ToggleBookmark),
                Editor(SetSelectionMode(Bookmark)),
                Editor(CursorAddToAllSelections),
                Expect(CurrentSelectedTexts(&["foo", "spam"])),
                Editor(CursorKeepPrimaryOnly),
                Expect(CurrentSelectedTexts(&["spam"])),
                Editor(ToggleBookmark),
                Editor(MoveSelection(Current)),
                Editor(CursorAddToAllSelections),
                Expect(CurrentSelectedTexts(&["foo"])),
            ])
        })
    }

    #[test]
    fn test_delete_word_backward_from_end_of_file() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent(
                    "fn snake_case(camelCase: String) {}".to_string(),
                )),
                Editor(SetSelectionMode(LineTrimmed)),
                // Go to the end of the file
                Editor(EnterInsertMode(Direction::End)),
                Editor(DeleteWordBackward),
                Expect(CurrentFileContent("fn snake_case(camelCase: String) ")),
                Editor(DeleteWordBackward),
                Expect(CurrentFileContent("fn snake_case(camelCase: String")),
                Editor(DeleteWordBackward),
                Expect(CurrentFileContent("fn snake_case(camelCase: ")),
                Editor(DeleteWordBackward),
            ])
        })
    }

    #[test]
    fn test_delete_word_backward_from_middle_of_file() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent(
                    "fn snake_case(camelCase: String) {}".to_string(),
                )),
                Editor(SetSelectionMode(BottomNode)),
                // Go to the middle of the file
                Editor(MoveSelection(Index(3))),
                Expect(CurrentSelectedTexts(&["camelCase"])),
                Editor(EnterInsertMode(Direction::End)),
                Editor(DeleteWordBackward),
                Expect(CurrentFileContent("fn snake_case(camel: String) {}")),
                Editor(DeleteWordBackward),
                Expect(CurrentFileContent("fn snake_case(: String) {}")),
                Editor(DeleteWordBackward),
                Expect(CurrentFileContent("fn snake_case: String) {}")),
                Editor(DeleteWordBackward),
                Expect(CurrentFileContent("fn snake_: String) {}")),
                Editor(DeleteWordBackward),
                Expect(CurrentFileContent("fn : String) {}")),
                Editor(DeleteWordBackward),
                Expect(CurrentFileContent(": String) {}")),
                Editor(DeleteWordBackward),
                Expect(CurrentFileContent(": String) {}")),
                Editor(DeleteWordBackward),
            ])
        })
    }
}
