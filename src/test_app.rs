/// NOTE: all test cases that involves the clipboard should not be run in parallel
///   otherwise the the test suite will fail because multiple tests are trying to
///   access the clipboard at the same time.
#[cfg(test)]
mod test_app {
    use itertools::Itertools;
    use my_proc_macros::{hex, key};
    use pretty_assertions::assert_eq;
    use serial_test::serial;

    use std::{
        ops::Range,
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
            component::{Component, ComponentId},
            editor::{Direction, DispatchEditor, Mode, Movement, ViewAlignment},
            suggestive_editor::Info,
        },
        context::{GlobalMode, LocalSearchConfigMode},
        frontend::mock::MockFrontend,
        grid::{Style, StyleKey},
        integration_test::integration_test::TestRunner,
        list::grep::RegexConfig,
        lsp::{process::LspNotification, signature_help::SignatureInformation},
        position::Position,
        quickfix_list::{Location, QuickfixListItem},
        rectangle::Rectangle,
        selection::SelectionMode,
        selection_mode::inside::InsideKind,
        themes::{Color, Theme},
    };

    enum Step {
        App(Dispatch),
        WithApp(Box<dyn Fn(&App<MockFrontend>) -> Dispatch>),
        ExpectMulti(Vec<ExpectKind>),
        Expect(ExpectKind),
        Editor(DispatchEditor),
        ExpectLater(Box<dyn Fn() -> ExpectKind>),
        ExpectCustom(Box<dyn Fn()>),
    }

    #[derive(Debug)]
    enum ExpectKind {
        JumpChars(&'static [char]),
        CurrentLine(&'static str),
        Not(Box<ExpectKind>),
        CurrentFileContent(&'static str),
        EditorCursorPosition(Position),
        EditorGridCursorPosition(Position),
        CurrentMode(Mode),
        FileContent(CanonicalizedPath, String),
        FileContentEqual(CanonicalizedPath, CanonicalizedPath),
        CurrentSelectedTexts(&'static [&'static str]),
        CurrentViewAlignment(Option<ViewAlignment>),
        ComponentsLength(usize),
        Quickfixes(Box<[QuickfixListItem]>),
        AppGrid(&'static str),
        EditorGrid(&'static str),
        CurrentPath(CanonicalizedPath),
        LocalSearchConfigSearches(&'static [&'static str]),
        LocalSearchConfigReplacements(&'static [&'static str]),
        GlobalSearchConfigSearches(&'static [&'static str]),
        GlobalSearchConfigReplacements(&'static [&'static str]),
        GlobalSearchConfigIncludeGlobs(&'static [&'static str]),
        GlobalSearchConfigExcludeGlobs(&'static [&'static str]),
        FileContentContains(CanonicalizedPath, &'static str),
        GridCellBackground(
            /*Row*/ usize,
            /*Column*/ usize,
            /*Background color*/ Color,
        ),
        GridCellStyleKey(Position, Option<StyleKey>),
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
            let mut context = app.context();
            fn contextualize<T: PartialEq + std::fmt::Debug>(a: T, b: T) -> (bool, String) {
                (a == b, format!("\n{a:?}\n == \n{b:?}\n",))
            }
            fn to_vec(strs: &[&str]) -> Vec<String> {
                strs.into_iter().map(|t| t.to_string()).collect()
            }
            let component = app.current_component().unwrap();
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
                EditorGrid(grid) => contextualize(
                    component.borrow().editor().get_grid(context).to_string(),
                    grid.to_string(),
                ),
                AppGrid(grid) => contextualize(app.get_grid()?.to_string(), grid.to_string()),
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
                CurrentMode(mode) => contextualize(&component.borrow().editor().mode, mode),
                EditorCursorPosition(position) => contextualize(
                    &component.borrow().editor().get_cursor_position().unwrap(),
                    position,
                ),
                EditorGridCursorPosition(position) => contextualize(
                    component
                        .borrow()
                        .editor()
                        .get_grid(context)
                        .cursor
                        .unwrap()
                        .position(),
                    position,
                ),
                CurrentLine(line) => contextualize(
                    component.borrow().editor().current_line().unwrap(),
                    line.to_string(),
                ),
                JumpChars(chars) => {
                    contextualize(component.borrow().editor().jump_chars(), chars.to_vec())
                }
                CurrentViewAlignment(view_alignment) => contextualize(
                    component.borrow().editor().current_view_alignment(),
                    view_alignment.clone(),
                ),
                GridCellBackground(row_index, column_index, background_color) => contextualize(
                    component.borrow().editor().get_grid(&mut context).grid.rows[*row_index]
                        [*column_index]
                        .background_color,
                    *background_color,
                ),
                GridCellStyleKey(position, style_key) => contextualize(
                    component.borrow().editor().get_grid(&mut context).grid.rows[position.line]
                        [position.column]
                        .source,
                    style_key.clone(),
                ),
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
                    ExpectMulti(expect_kinds) => {
                        for expect_kind in expect_kinds.into_iter() {
                            expect_kind.run(&mut app)
                        }
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
                Expect(ExpectKind::AppGrid(
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
                Expect(AppGrid(
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
                Expect(AppGrid(
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

    #[test]
    fn kill_line_to_end() -> anyhow::Result<()> {
        let input = "lala\nfoo bar spam\nyoyo";
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent(input.to_string())),
                // Killing to the end of line WITH trailing newline character
                Editor(MatchLiteral("bar".to_string())),
                Editor(KillLine(Direction::End)),
                Editor(Insert("sparta".to_string())),
                Expect(CurrentFileContent("lala\nfoo sparta\nyoyo")),
                Expect(CurrentMode(Mode::Insert)),
                Expect(CurrentSelectedTexts(&[""])),
                // Remove newline character if the character after cursor is a newline character
                Editor(KillLine(Direction::End)),
                Expect(CurrentFileContent("lala\nfoo spartayoyo")),
                // Killing to the end of line WITHOUT trailing newline character
                Editor(KillLine(Direction::End)),
                Expect(CurrentFileContent("lala\nfoo sparta")),
            ])
        })
    }

    #[test]
    fn kill_line_to_start() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("lala\nfoo bar spam\nyoyo".to_string())),
                // Killing to the start of line WITH leading newline character
                Editor(MatchLiteral("bar".to_string())),
                Editor(KillLine(Direction::Start)),
                Editor(Insert("sparta".to_string())),
                Expect(CurrentFileContent("lala\nspartabar spam\nyoyo")),
                Expect(CurrentMode(Mode::Insert)),
                Editor(KillLine(Direction::Start)),
                Expect(CurrentFileContent("lala\nbar spam\nyoyo")),
                // Remove newline character if the character before cursor is a newline character
                Editor(KillLine(Direction::Start)),
                Expect(CurrentFileContent("lalabar spam\nyoyo")),
                Expect(EditorCursorPosition(Position { line: 0, column: 4 })),
                // Killing to the start of line WITHOUT leading newline character
                Editor(KillLine(Direction::Start)),
                Expect(CurrentFileContent("bar spam\nyoyo")),
            ])
        })
    }

    #[test]
    fn undo_tree() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("\n".to_string())),
                Editor(Insert("a".to_string())),
                Editor(Insert("bc".to_string())),
                Editor(EnterUndoTreeMode),
                // Previous = undo
                Editor(MoveSelection(Previous)),
                Expect(CurrentFileContent("a\n")),
                // Next = redo
                Editor(MoveSelection(Next)),
                Expect(CurrentFileContent("abc\n")),
                Editor(MoveSelection(Previous)),
                Expect(CurrentFileContent("a\n")),
                Editor(Insert("de".to_string())),
                Editor(EnterUndoTreeMode),
                // Down = go to previous history branch
                Editor(MoveSelection(Down)),
                // We are able to retrive the "bc" insertion, which is otherwise impossible without the undo tree
                Expect(CurrentFileContent("abc\n")),
                // Up = go to next history branch
                Editor(MoveSelection(Up)),
                Expect(CurrentFileContent("ade\n")),
            ])
        })
    }

    #[test]
    fn multi_exchange_sibling() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("fn f(x:a,y:b){} fn g(x:a,y:b){}".to_string())),
                Editor(MatchLiteral("fn f(x:a,y:b){}".to_string())),
                Expect(CurrentSelectedTexts(&["fn f(x:a,y:b){}"])),
                Editor(SetSelectionMode(SyntaxTree)),
                Editor(CursorAddToAllSelections),
                Expect(CurrentSelectedTexts(&[
                    "fn f(x:a,y:b){}",
                    "fn g(x:a,y:b){}",
                ])),
                Editor(MoveSelection(Down)),
                Editor(MoveSelection(Next)),
                Editor(MoveSelection(Down)),
                Expect(CurrentSelectedTexts(&["x:a", "x:a"])),
                Editor(SetSelectionMode(SyntaxTree)),
                Editor(EnterExchangeMode),
                Editor(MoveSelection(Next)),
                Expect(CurrentFileContent("fn f(y:b,x:a){} fn g(y:b,x:a){}")),
                Expect(CurrentSelectedTexts(&["x:a", "x:a"])),
                Editor(MoveSelection(Previous)),
                Expect(CurrentFileContent("fn f(x:a,y:b){} fn g(x:a,y:b){}")),
            ])
        })
    }

    #[test]
    fn update_bookmark_position() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("foo bar spim".to_string())),
                Editor(SetSelectionMode(Word)),
                Editor(MoveSelection(Next)),
                Editor(MoveSelection(Next)),
                Editor(ToggleBookmark),
                Editor(SetSelectionMode(Bookmark)),
                Expect(CurrentSelectedTexts(&["spim"])),
                Editor(SetSelectionMode(Word)),
                Editor(MoveSelection(Previous)),
                Editor(MoveSelection(Previous)),
                // Kill "foo"
                Editor(Kill),
                Expect(CurrentFileContent("bar spim")),
                Editor(SetSelectionMode(Bookmark)),
                // Expect bookmark position is updated, and still selects "spim"
                Expect(CurrentSelectedTexts(&["spim"])),
                // Remove "m" from "spim"
                Editor(EnterInsertMode(Direction::End)),
                Editor(Backspace),
                Expect(CurrentFileContent("bar spi")),
                Editor(EnterNormalMode),
                Editor(SetSelectionMode(Bookmark)),
                // Expect the "spim" bookmark is removed
                // By the fact that "spi" is not selected
                Expect(CurrentSelectedTexts(&["i"])),
            ])
        })
    }

    #[test]
    fn move_to_line_start_end() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("hello\n".to_string())),
                Editor(EnterInsertMode(Direction::Start)),
                Editor(MoveToLineEnd),
                Editor(Insert(" world".to_string())),
                Expect(CurrentFileContent("hello world\n")),
                Editor(MoveToLineStart),
                Editor(Insert("hey ".to_string())),
                Expect(CurrentFileContent("hey hello world\n")),
            ])
        })
    }

    #[test]
    fn exchange_sibling() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("fn main(x: usize, y: Vec<A>) {}".to_string())),
                // Select first statement
                Editor(MatchLiteral("x: usize".to_string())),
                Editor(SetSelectionMode(SyntaxTree)),
                Editor(EnterExchangeMode),
                Editor(MoveSelection(Next)),
                Expect(CurrentFileContent("fn main(y: Vec<A>, x: usize) {}")),
                Editor(MoveSelection(Previous)),
                Expect(CurrentFileContent("fn main(x: usize, y: Vec<A>) {}")),
            ])
        })
    }

    #[test]
    fn exchange_sibling_2() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("use a;\nuse b;\nuse c;".to_string())),
                // Select first statement
                Editor(SetSelectionMode(TopNode)),
                Editor(SetSelectionMode(SyntaxTree)),
                Expect(CurrentSelectedTexts(&["use a;"])),
                Editor(EnterExchangeMode),
                Editor(MoveSelection(Next)),
                Expect(CurrentFileContent("use b;\nuse a;\nuse c;")),
                Editor(MoveSelection(Next)),
                Expect(CurrentFileContent("use b;\nuse c;\nuse a;")),
            ])
        })
    }

    #[test]
    fn select_character() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("fn main() { let x = 1; }".to_string())),
                Editor(SetSelectionMode(Character)),
                Expect(CurrentSelectedTexts(&["f"])),
                Editor(MoveSelection(Next)),
                Expect(CurrentSelectedTexts(&["n"])),
                Editor(MoveSelection(Previous)),
                Expect(CurrentSelectedTexts(&["f"])),
            ])
        })
    }

    #[test]
    fn raise() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("fn main() { let x = a.b(c()); }".to_string())),
                Editor(MatchLiteral("c()".to_string())),
                Editor(SetSelectionMode(SyntaxTree)),
                Editor(Raise),
                Expect(CurrentFileContent("fn main() { let x = c(); }")),
                Editor(Raise),
                Expect(CurrentFileContent("fn main() { c() }")),
            ])
        })
    }

    #[test]
    fn select_kids() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("fn main(x: usize, y: Vec<A>) {}".to_string())),
                Editor(MatchLiteral("x".to_string())),
                Editor(MoveSelection(Next)),
                Expect(CurrentSelectedTexts(&["x"])),
                Editor(SelectKids),
                Expect(CurrentSelectedTexts(&["x: usize, y: Vec<A>"])),
            ])
        })
    }

    #[test]
    /// After raise the node kind should be the same
    /// Raising `(a).into()` in `Some((a).into())`
    /// should result in `(a).into()`
    /// not `Some(a).into()`
    fn raise_preserve_current_node_structure() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("fn main() { Some((a).b()) }".to_string())),
                Editor(MatchLiteral("(a).b()".to_string())),
                Editor(SetSelectionMode(SyntaxTree)),
                Editor(Raise),
                Expect(CurrentFileContent("fn main() { (a).b() }")),
            ])
        })
    }

    #[test]
    fn multi_raise() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent(
                    "fn f(){ let x = S(a); let y = S(b); }".to_string(),
                )),
                Editor(MatchLiteral("let x = S(a);".to_string())),
                Editor(SetSelectionMode(SyntaxTree)),
                Editor(CursorAddToAllSelections),
                Editor(MoveSelection(Down)),
                Editor(MoveSelection(Next)),
                Editor(MoveSelection(Down)),
                Editor(MoveSelection(Next)),
                Editor(MoveSelection(Down)),
                Editor(MoveSelection(Next)),
                Expect(CurrentSelectedTexts(&["a", "b"])),
                Editor(Raise),
                Expect(CurrentFileContent("fn f(){ let x = a; let y = b; }")),
                Editor(Undo),
                Expect(CurrentFileContent("fn f(){ let x = S(a); let y = S(b); }")),
                Expect(CurrentSelectedTexts(&["a", "b"])),
                Editor(Redo),
                Expect(CurrentFileContent("fn f(){ let x = a; let y = b; }")),
                Expect(CurrentSelectedTexts(&["a", "b"])),
            ])
        })
    }

    #[test]
    fn open_new_line() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent(
                    "
fn f() {
    let x = S(a);
}
"
                    .trim()
                    .to_string(),
                )),
                Editor(MatchLiteral("let x = ".to_string())),
                Editor(OpenNewLine),
                Editor(Insert("let y = S(b);".to_string())),
                Expect(CurrentFileContent(
                    "
fn f() {
    let x = S(a);
    let y = S(b);
}"
                    .trim(),
                )),
            ])
        })
    }

    #[test]
    fn exchange_line() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent(
                    // Multiline source code
                    "
fn main() {
    let x = 1;
    let y = 2;
}"
                    .trim()
                    .to_string(),
                )),
                Editor(SetSelectionMode(LineTrimmed)),
                Editor(Exchange(Next)),
                Expect(CurrentFileContent(
                    "
let x = 1;
    fn main() {
    let y = 2;
}"
                    .trim(),
                )),
                Editor(Exchange(Previous)),
                Expect(CurrentFileContent(
                    "
fn main() {
    let x = 1;
    let y = 2;
}"
                    .trim(),
                )),
            ])
        })
    }

    #[test]
    fn exchange_character() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("fn main() { let x = 1; }".to_string())),
                Editor(SetSelectionMode(Character)),
                Editor(Exchange(Next)),
                Expect(CurrentFileContent("nf main() { let x = 1; }")),
                Editor(Exchange(Next)),
                Expect(CurrentFileContent("n fmain() { let x = 1; }")),
                Editor(Exchange(Previous)),
                Expect(CurrentFileContent("nf main() { let x = 1; }")),
                Editor(Exchange(Previous)),
                Expect(CurrentFileContent("fn main() { let x = 1; }")),
            ])
        })
    }

    #[test]
    fn multi_insert() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("struct A(usize, char)".to_string())),
                Editor(MatchLiteral("usize".to_string())),
                Editor(SetSelectionMode(SyntaxTree)),
                Editor(CursorAddToAllSelections),
                Expect(CurrentSelectedTexts(&["usize", "char"])),
                Editor(EnterInsertMode(Direction::Start)),
                Editor(Insert("pub ".to_string())),
                Expect(CurrentFileContent("struct A(pub usize, pub char)")),
                Editor(Backspace),
                Expect(CurrentFileContent("struct A(pubusize, pubchar)")),
                Expect(CurrentSelectedTexts(&["", ""])),
            ])
        })
    }

    #[serial]
    #[test]
    fn paste_from_clipboard() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent(
                    "fn f(){ let x = S(a); let y = S(b); }".to_string(),
                )),
                App(SetClipboardContent("let z = S(c);".to_string())),
                Editor(Paste),
                Expect(CurrentFileContent(
                    "let z = S(c);fn f(){ let x = S(a); let y = S(b); }",
                )),
            ])
        })
    }

    #[test]
    fn enter_newline() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("".to_string())),
                Editor(EnterInsertMode(Direction::Start)),
                Editor(Insert("hello".to_string())),
                App(HandleKeyEvent(key!("enter"))),
                Editor(Insert("world".to_string())),
                Expect(CurrentFileContent("hello\nworld")),
                App(HandleKeyEvent(key!("left"))),
                App(HandleKeyEvent(key!("enter"))),
                Expect(CurrentFileContent("hello\nworl\nd")),
            ])
        })
    }

    #[test]
    fn insert_mode_start() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("fn main() {}".to_string())),
                Editor(SetSelectionMode(Word)),
                Editor(EnterInsertMode(Direction::Start)),
                Editor(Insert("hello".to_string())),
                Expect(CurrentFileContent("hellofn main() {}")),
            ])
        })
    }

    #[test]
    fn insert_mode_end() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("fn main() {}".to_string())),
                Editor(SetSelectionMode(Word)),
                Editor(EnterInsertMode(Direction::End)),
                Editor(Insert("hello".to_string())),
                Expect(CurrentFileContent("fnhello main() {}")),
            ])
        })
    }

    #[test]
    fn highlight_kill() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("fn main() {}".to_string())),
                Editor(SetSelectionMode(BottomNode)),
                Editor(ToggleHighlightMode),
                Editor(MoveSelection(Next)),
                Expect(CurrentSelectedTexts(&["fn main"])),
                Editor(Kill),
                Expect(CurrentSelectedTexts(&["("])),
            ])
        })
    }

    #[test]
    fn multicursor_add_all() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent(
                    "mod m { fn a(j:J){} fn b(k:K,l:L){} fn c(m:M,n:N,o:O){} }".to_string(),
                )),
                Editor(MatchLiteral("fn a".to_string())),
                Editor(SetSelectionMode(SyntaxTree)),
                Expect(CurrentSelectedTexts(&["fn a(j:J){}"])),
                Editor(CursorAddToAllSelections),
                Editor(MoveSelection(Down)),
                Editor(MoveSelection(Next)),
                Editor(MoveSelection(Down)),
                Expect(CurrentSelectedTexts(&["j:J", "k:K", "m:M"])),
                Editor(CursorAddToAllSelections),
                Expect(CurrentSelectedTexts(&[
                    "j:J", "k:K", "l:L", "m:M", "n:N", "o:O",
                ])),
            ])
        })
    }

    #[test]
    fn enter_normal_mode_should_highlight_one_character() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent(
                    "fn\nmain()\n{ x.y(); x.y(); x.y(); }".to_string(),
                )),
                Editor(MatchLiteral("x.y()".to_string())),
                Editor(EnterInsertMode(Direction::End)),
                Editor(EnterNormalMode),
                Expect(CurrentSelectedTexts(&[")"])),
            ])
        })
    }

    #[test]
    fn highlight_change() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("hello world yo".to_string())),
                Editor(SetSelectionMode(Word)),
                Editor(ToggleHighlightMode),
                Editor(MoveSelection(Next)),
                Expect(CurrentSelectedTexts(&["hello world"])),
                Editor(Change),
                Editor(Insert("wow".to_string())),
                Expect(CurrentSelectedTexts(&[""])),
                Expect(CurrentFileContent("wow yo")),
            ])
        })
    }

    #[test]
    fn scroll_page() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("1\n2 hey\n3".to_string())),
                Editor(SetRectangle(Rectangle {
                    origin: Position::default(),
                    width: 100,
                    height: 3,
                })),
                Editor(ScrollPageDown),
                Expect(CurrentLine("2 hey")),
                Editor(ScrollPageDown),
                Editor(MatchLiteral("hey".to_string())),
                Expect(CurrentSelectedTexts(&["hey"])),
                Editor(ScrollPageDown),
                Expect(CurrentLine("3")),
                Editor(ScrollPageDown),
                Expect(CurrentLine("3")),
                Editor(ScrollPageUp),
                Expect(CurrentLine("2 hey")),
                Editor(ScrollPageUp),
                Expect(CurrentLine("1")),
                Editor(ScrollPageUp),
                Expect(CurrentLine("1")),
            ])
        })
    }

    #[test]
    fn jump() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent(
                    "Who lives on sea shore?\n yonky donkey".to_string(),
                )),
                Editor(SetRectangle(Rectangle {
                    origin: Position::default(),
                    width: 100,
                    height: 1,
                })),
                // In jump mode, the first stage labels each selection using their starting character,
                // On subsequent stages, the labels are random alphabets
                Expect(JumpChars(&[])),
                Editor(SetSelectionMode(Word)),
                Editor(DispatchEditor::Jump),
                // Expect the jump to be the first character of each word
                // Note 'y' and 'd' are excluded because they are out of view,
                // since the viewbox has only height of 1
                Expect(JumpChars(&['w', 'l', 'o', 's', 's', '?'])),
                App(HandleKeyEvent(key!("s"))),
                Expect(JumpChars(&['a', 'b'])),
                App(HandleKeyEvent(key!("a"))),
                Expect(JumpChars(&[])),
                Expect(CurrentSelectedTexts(&["sea"])),
            ])
        })
    }

    #[test]
    fn highlight_and_jump() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent(
                    "Who lives on sea shore?\n yonky donkey".to_string(),
                )),
                Editor(SetRectangle(Rectangle {
                    origin: Position::default(),
                    width: 100,
                    height: 1,
                })),
                Editor(SetSelectionMode(Word)),
                Editor(MoveSelection(Next)),
                Editor(ToggleHighlightMode),
                Editor(DispatchEditor::Jump),
                // Expect the jump to be the first character of each word
                // Note 'y' and 'd' are excluded because they are out of view,
                // since the viewbox has only height of 1
                Expect(JumpChars(&['w', 'l', 'o', 's', 's', '?'])),
                App(HandleKeyEvent(key!("s"))),
                App(HandleKeyEvent(key!("b"))),
                Expect(CurrentSelectedTexts(&["lives on sea shore"])),
            ])
        })
    }

    #[test]
    fn jump_all_selection_start_with_same_char() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("who who who who".to_string())),
                Editor(SetRectangle(Rectangle {
                    origin: Position::default(),
                    width: 100,
                    height: 1,
                })),
                Editor(SetSelectionMode(Word)),
                Editor(DispatchEditor::Jump),
                // Expect the jump to NOT be the first character of each word
                // Since, the first character of each selection are the same, which is 'w'
                Expect(JumpChars(&['a', 'b', 'c', 'd'])),
            ])
        })
    }

    #[test]
    fn switch_view_alignment() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent(
                    "abcde"
                        .split("")
                        .collect_vec()
                        .join("\n")
                        .trim()
                        .to_string(),
                )),
                Editor(SetRectangle(Rectangle {
                    origin: Position::default(),
                    width: 100,
                    height: 4,
                })),
                Editor(SetSelectionMode(Word)),
                Editor(MoveSelection(Next)),
                Editor(MoveSelection(Next)),
                Expect(CurrentSelectedTexts(&["c"])),
                Expect(CurrentViewAlignment(None)),
                Editor(SwitchViewAlignment),
                Expect(CurrentViewAlignment(Some(ViewAlignment::Top))),
                Editor(SwitchViewAlignment),
                Expect(CurrentViewAlignment(Some(ViewAlignment::Center))),
                Editor(SwitchViewAlignment),
                Expect(CurrentViewAlignment(Some(ViewAlignment::Bottom))),
                Editor(MoveSelection(Previous)),
                Expect(CurrentViewAlignment(None)),
            ])
        })
    }

    #[test]
    fn get_grid_parent_line() -> anyhow::Result<()> {
        let parent_lines_background = hex!("#badbad");
        let bookmark_background_color = hex!("#cebceb");
        let theme = {
            let mut theme = Theme::default();
            theme.ui.parent_lines_background = parent_lines_background;
            theme.ui.bookmark = Style::default().background_color(bookmark_background_color);
            theme
        };
        let width = 20;
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent(
                    "
// hello
fn main() {
  let x = 1;
  let y = 2;
  for a in b {
    let z = 4;
    print()
  }
}
"
                    .trim()
                    .to_string(),
                )),
                Editor(SetRectangle(Rectangle {
                    origin: Position::default(),
                    width,
                    height: 6,
                })),
                App(SetTheme(theme.clone())),
                // Go to "print()" and skip the first 3 lines for rendering
                Editor(MatchLiteral("print()".to_string())),
                Editor(SetScrollOffset(3)),
                // Expect `fn main()` is visible although it is out of view,
                // because it is amongst the parent lines of the current selection
                Expect(EditorGrid(
                    "
src/main.rs 🦀
2│fn main() {
4│  let y = 2;
5│  for a in b {
6│    let z = 4;
7│    █rint()
"
                    .trim(),
                )),
                // Bookmart "z"
                Editor(MatchLiteral("z".to_string())),
                Editor(ToggleBookmark),
                // Expect the parent lines of the current selections are highlighted with parent_lines_background,
                // regardless of whether the parent lines are inbound or outbound
                ExpectMulti(
                    [1, 3]
                        .into_iter()
                        .flat_map(|row_index| {
                            [0, width - 1].into_iter().map(move |column_index| {
                                GridCellBackground(
                                    row_index,
                                    column_index as usize,
                                    parent_lines_background,
                                )
                            })
                        })
                        .collect(),
                ),
                // Expect the current line is not treated as parent line
                ExpectMulti(
                    [0, width - 1]
                        .into_iter()
                        .map(|column_index| {
                            Not(Box::new(GridCellBackground(
                                5,
                                column_index as usize,
                                parent_lines_background,
                            )))
                        })
                        .collect(),
                ),
                // Bookmark the "fn" token
                Editor(MatchLiteral("fn".to_string())),
                Editor(ToggleBookmark),
                // Go to "print()" and skip the first 3 lines for rendering
                Editor(MatchLiteral("print()".to_string())),
                Editor(SetScrollOffset(3)),
                Expect(EditorGrid(
                    "
src/main.rs 🦀
2│fn main() {
4│  let y = 2;
5│  for a in b {
6│    let z = 4;
7│    █rint()
"
                    .trim(),
                )),
                // Expect the bookmarks of outbound parent lines are rendered properly
                // In this case, the outbound parent line is "fn main() {"
                ExpectMulti(
                    [2, 3]
                        .into_iter()
                        .map(|column_index| {
                            GridCellBackground(1, column_index as usize, bookmark_background_color)
                        })
                        .collect(),
                ),
                // Expect the bookmarks of inbound lines are rendered properly
                // In this case, we want to check that the bookmark on "z" is rendered
                Expect(GridCellBackground(4, 10, bookmark_background_color)),
            ])
        })
    }

    #[test]
    fn test_wrapped_lines() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent(
                    "
// hello world\n hey
"
                    .trim()
                    .to_string(),
                )),
                Editor(SetRectangle(Rectangle {
                    origin: Position::default(),
                    width: 13,
                    height: 4,
                })),
                Editor(MatchLiteral("world".to_string())),
                Editor(EnterInsertMode(Direction::End)),
                Expect(EditorGrid(
                    "
src/main.rs
1│// hello
↪│world█
2│ hey
"
                    .trim(),
                )),
                // Expect the cursor is after 'd'
                Expect(EditorGridCursorPosition(Position { line: 2, column: 7 })),
            ])
        })
    }

    #[test]
    fn syntax_highlighting() -> anyhow::Result<()> {
        execute_test(|s| {
            let theme = Theme::default();
            Box::new([
                App(OpenFile(s.main_rs())),
                App(SetTheme(theme.clone())),
                Editor(SetContent(
                    "
fn main() { // too long
  let foo = 1;
  let bar = baba; let wrapped = coco;
}
"
                    .trim()
                    .to_string(),
                )),
                Editor(SetRectangle(Rectangle {
                    origin: Position::default(),
                    width: 13,
                    height: 4,
                })),
                Editor(SetLanguage(shared::language::from_extension("rs").unwrap())),
                Editor(MatchLiteral("bar".to_string())),
                Editor(DispatchEditor::ApplySyntaxHighlight),
                Editor(SetRectangle(Rectangle {
                    origin: Position::default(),
                    width: 20,
                    height: 4,
                })),
                Editor(SwitchViewAlignment),
                // The "long" of "too long" is not shown, because it exceeded the view width
                Expect(EditorGrid(
                    "
src/main.rs 🦀
1│fn main() { // too
3│  let █ar = baba;
↪│let wrapped = coco
"
                    .trim(),
                )),
                ExpectMulti(
                    [
                        //
                        // Expect the `fn` keyword of the outbound parent line "fn main() { // too long" is highlighted properly
                        Position::new(1, 2),
                        Position::new(1, 3),
                        //
                        // Expect the `let` keyword of line 3 (which is inbound and not wrapped) is highlighted properly
                        Position::new(2, 4),
                        Position::new(2, 5),
                        Position::new(2, 6),
                        //
                        // Expect the `let` keyword of line 3 (which is inbound but wrapped) is highlighted properly
                        Position::new(3, 2),
                        Position::new(3, 3),
                        Position::new(3, 4),
                    ]
                    .into_iter()
                    .map(|position| {
                        ExpectKind::GridCellStyleKey(position, Some(StyleKey::SyntaxKeyword))
                    })
                    .collect(),
                ),
                // Expect decorations overrides syntax highlighting
                Editor(MatchLiteral("fn".to_string())),
                Editor(ToggleBookmark),
                // Move cursor to next line, so that "fn" is not selected,
                //  so that we can test the style applied to "fn" ,
                // otherwise the style of primary selection anchors will override the bookmark style
                Editor(MatchLiteral("let".to_string())),
                Expect(EditorGrid(
                    "
src/main.rs 🦀
1│fn main() { // too
↪│ long
2│  █et foo = 1;
"
                    .trim(),
                )),
                ExpectMulti(
                    [Position::new(1, 2), Position::new(1, 3)]
                        .into_iter()
                        .map(|position| {
                            ExpectKind::GridCellStyleKey(position, Some(StyleKey::UiBookmark))
                        })
                        .collect(),
                ),
            ])
        })
    }

    #[test]
    fn empty_content_should_have_one_line() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("".to_string())),
                Editor(SetRectangle(Rectangle {
                    origin: Position::default(),
                    width: 20,
                    height: 2,
                })),
                Expect(EditorGrid(
                    "
src/main.rs 🦀
1│█
"
                    .trim(),
                )),
            ])
        })
    }

    #[test]
    fn update_bookmark_position_with_undo_and_redo() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("foo bar spim".to_string())),
                Editor(SetSelectionMode(Word)),
                Editor(MoveSelection(Next)),
                Editor(MoveSelection(Next)),
                Editor(ToggleBookmark),
                Editor(SetSelectionMode(Bookmark)),
                Expect(CurrentSelectedTexts(&["spim"])),
                Editor(SetSelectionMode(Word)),
                Editor(MoveSelection(Previous)),
                Editor(MoveSelection(Previous)),
                // Kill "foo"
                Editor(Kill),
                Expect(CurrentFileContent("bar spim")),
                // Expect bookmark position is updated (still selects "spim")
                Editor(SetSelectionMode(Bookmark)),
                Expect(CurrentSelectedTexts(&["spim"])),
                Editor(Undo),
                Expect(CurrentFileContent("foo bar spim")),
                // Expect bookmark position is updated (still selects "spim")
                Editor(SetSelectionMode(Bookmark)),
                Expect(CurrentSelectedTexts(&["spim"])),
                Editor(Redo),
                // Expect bookmark position is updated (still selects "spim")
                Editor(SetSelectionMode(Bookmark)),
                Expect(CurrentSelectedTexts(&["spim"])),
            ])
        })
    }

    #[test]
    fn saving_should_not_destroy_bookmark_if_selections_not_modified() -> anyhow::Result<()> {
        let input = "// foo bar spim\n    fn foo() {}\n";

        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent(input.to_string())),
                Editor(SetLanguage(shared::language::from_extension("rs").unwrap())),
                Editor(SetSelectionMode(Word)),
                Editor(MoveSelection(Next)),
                Editor(MoveSelection(Next)),
                Editor(ToggleBookmark),
                Editor(SetSelectionMode(Bookmark)),
                Expect(CurrentSelectedTexts(&["bar"])),
                Editor(Save),
                // Expect the content is formatted (second line dedented)
                Expect(CurrentFileContent("// foo bar spim\nfn foo() {}\n")),
                Editor(SetSelectionMode(Character)),
                Expect(CurrentSelectedTexts(&["b"])),
                // Expect the bookmark on "bar" is not destroyed
                Editor(SetSelectionMode(Bookmark)),
                Expect(CurrentSelectedTexts(&["bar"])),
            ])
        })
    }
}
