/// NOTE: all test cases that involves the clipboard should not be run in parallel
///   otherwise the the test suite will fail because multiple tests are trying to
///   access the clipboard at the same time.
#[cfg(test)]
use itertools::Itertools;

use lsp_types::Url;
use my_proc_macros::key;

use serial_test::serial;

use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
pub use Dispatch::*;
pub use DispatchEditor::*;

pub use Movement::*;
pub use SelectionMode::*;

use shared::canonicalized_path::CanonicalizedPath;

use crate::{
    app::{
        App, Dimension, Dispatch, GlobalSearchConfigUpdate, GlobalSearchFilterGlob,
        LocalSearchConfigUpdate, Scope,
    },
    components::{
        component::{Component, ComponentId},
        editor::{Direction, DispatchEditor, Mode, Movement, ViewAlignment},
        suggestive_editor::{DispatchSuggestiveEditor, Info},
    },
    context::LocalSearchConfigMode,
    frontend::mock::MockFrontend,
    grid::StyleKey,
    integration_test::TestRunner,
    list::grep::RegexConfig,
    lsp::signature_help::SignatureInformation,
    position::Position,
    quickfix_list::{Location, QuickfixListItem},
    selection::SelectionMode,
};
use crate::{lsp::process::LspNotification, themes::Color};

type WithApp = Box<dyn Fn(&App<MockFrontend>) -> Dispatch>;

pub enum Step {
    App(Dispatch),
    WithApp(WithApp),
    ExpectMulti(Vec<ExpectKind>),
    Expect(ExpectKind),
    Editor(DispatchEditor),
    SuggestiveEditor(DispatchSuggestiveEditor),
    ExpectLater(Box<dyn Fn() -> ExpectKind>),
    ExpectCustom(Box<dyn Fn()>),
}

#[derive(Debug)]
pub enum ExpectKind {
    CurrentCodeActions(&'static [crate::lsp::code_action::CodeAction]),
    EditorInfoContent(&'static str),
    EditorInfoOpen(bool),
    QuickfixListCurrentLine(&'static str),
    DropdownInfosCount(usize),
    QuickfixListContent(String),
    CompletionDropdownContent(&'static str),
    CompletionDropdownIsOpen(bool),
    CompletionDropdownSelectedItem(&'static str),
    JumpChars(&'static [char]),
    CurrentLine(&'static str),
    Not(Box<ExpectKind>),
    CurrentComponentContent(&'static str),
    EditorCursorPosition(Position),
    EditorGridCursorPosition(Position),
    CurrentMode(Mode),
    FileContent(CanonicalizedPath, String),
    FileContentEqual(CanonicalizedPath, CanonicalizedPath),
    CurrentSelectedTexts(&'static [&'static str]),
    CurrentViewAlignment(Option<ViewAlignment>),
    ComponentsLength(usize),
    Quickfixes(Box<[QuickfixListItem]>),
    AppGrid(String),
    AppGridContains(&'static str),
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
        let context = app.context();
        fn contextualize<T: PartialEq + std::fmt::Debug>(a: T, b: T) -> (bool, String) {
            (a == b, format!("\n{a:?}\n == \n{b:?}\n",))
        }
        fn to_vec(strs: &[&str]) -> Vec<String> {
            strs.iter().map(|t| t.to_string()).collect()
        }
        let component = app.current_component().unwrap();
        Ok(match self {
            CurrentComponentContent(expected_content) => contextualize(
                app.get_current_component_content(),
                expected_content.to_string(),
            ),
            FileContent(path, expected_content) => {
                contextualize(app.get_file_content(path), expected_content.clone())
            }
            FileContentEqual(left, right) => {
                contextualize(app.get_file_content(left), app.get_file_content(right))
            }
            CurrentSelectedTexts(selected_texts) => {
                contextualize(app.get_current_selected_texts(), to_vec(selected_texts))
            }
            ComponentsLength(length) => contextualize(app.components().len(), *length),
            Quickfixes(expected_quickfixes) => contextualize(
                app.get_quickfixes()
                    .unwrap_or_default()
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
            AppGrid(grid) => {
                let actual = app.get_screen()?.stringify().trim_matches('\n').to_string();
                println!("actual =\n{}", actual);
                contextualize(actual, grid.to_string().trim_matches('\n').to_string())
            }
            CurrentPath(path) => contextualize(app.get_current_file_path().unwrap(), path.clone()),
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
                *view_alignment,
            ),
            GridCellBackground(row_index, column_index, background_color) => contextualize(
                component.borrow().editor().get_grid(context).grid.rows[*row_index][*column_index]
                    .background_color,
                *background_color,
            ),
            GridCellStyleKey(position, style_key) => contextualize(
                component.borrow().editor().get_grid(context).grid.rows[position.line]
                    [position.column]
                    .source,
                *style_key,
            ),
            CompletionDropdownIsOpen(is_open) => {
                contextualize(app.completion_dropdown_is_open(), *is_open)
            }
            CompletionDropdownContent(content) => contextualize(
                app.current_completion_dropdown()
                    .unwrap()
                    .borrow()
                    .content(),
                content.to_string(),
            ),
            CompletionDropdownSelectedItem(item) => contextualize(
                app.current_completion_dropdown()
                    .unwrap()
                    .borrow()
                    .get_selected_texts()[0]
                    .trim(),
                item,
            ),
            QuickfixListContent(content) => contextualize(
                app.quickfix_list().unwrap().borrow().content(),
                content.to_string(),
            ),
            DropdownInfosCount(actual) => contextualize(app.get_dropdown_infos_count(), *actual),
            QuickfixListCurrentLine(actual) => contextualize(
                app.quickfix_list().unwrap().borrow().current_line()?,
                actual.to_string(),
            ),
            EditorInfoOpen(actual) => contextualize(app.editor_info_open(), *actual),
            EditorInfoContent(actual) => {
                contextualize(app.editor_info_content(), Some(actual.to_string()))
            }
            CurrentCodeActions(code_actions) => {
                contextualize(app.current_code_actions(), code_actions.to_vec())
            }
            AppGridContains(substring) => {
                let content = app.get_screen().unwrap().stringify();
                println!("content =\n{}", content);
                contextualize(content.contains(substring), true)
            }
        })
    }
}

pub use ExpectKind::*;
pub use Step::*;
pub struct State {
    temp_dir: CanonicalizedPath,
    main_rs: CanonicalizedPath,
    foo_rs: CanonicalizedPath,
    git_ignore: CanonicalizedPath,
}
impl State {
    pub fn main_rs(&self) -> CanonicalizedPath {
        self.main_rs.clone()
    }

    pub fn foo_rs(&self) -> CanonicalizedPath {
        self.foo_rs.clone()
    }

    pub fn new_path(&self, path: &str) -> PathBuf {
        self.temp_dir.to_path_buf().join(path)
    }

    pub fn gitignore(&self) -> CanonicalizedPath {
        self.git_ignore.clone()
    }

    pub fn temp_dir(&self) -> CanonicalizedPath {
        self.temp_dir.clone()
    }
}

pub fn execute_test(callback: impl Fn(State) -> Box<[Step]>) -> anyhow::Result<()> {
    run_test(|mut app, temp_dir| {
        let steps = {
            callback(State {
                main_rs: temp_dir.join("src/main.rs").unwrap(),
                foo_rs: temp_dir.join("src/foo.rs").unwrap(),
                git_ignore: temp_dir.join(".gitignore").unwrap(),
                temp_dir,
            })
        };

        for step in steps.iter() {
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
                    for expect_kind in expect_kinds.iter() {
                        expect_kind.run(&mut app)
                    }
                }
                SuggestiveEditor(dispatch) => {
                    log(dispatch);
                    app.handle_dispatch_suggestive_editor(dispatch.to_owned())?
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
        let mock_frontend = Arc::new(Mutex::new(MockFrontend::default()));
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
            Editor(ReplaceWithClipboard),
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
            Expect(CurrentComponentContent("fn fn() { let x = 1; }")),
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
            Editor(ReplaceWithClipboard),
            Expect(CurrentComponentContent("fn fn() { let x = 1; }")),
            Expect(CurrentSelectedTexts(&[""])),
            Editor(MoveSelection(Next)),
            Editor(ReplaceWithClipboard),
            Expect(CurrentComponentContent("fn fn(fn { let x = 1; }")),
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
            Expect(CurrentComponentContent(" main() { let x = 1; }")),
            Editor(MoveSelection(Current)),
            Expect(CurrentSelectedTexts(&["main"])),
            Editor(ReplaceWithClipboard),
            Expect(CurrentComponentContent(" fn() { let x = 1; }")),
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
            Expect(CurrentComponentContent("{ let x = S(a); let y = S(b); }")),
            Editor(ReplaceWithClipboard),
            Expect(CurrentComponentContent(
                "fn f(){ let x = S(a); let y = S(b); }",
            )),
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
            Editor(ReplaceWithClipboard),
            Expect(CurrentComponentContent(
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
            Expect(CurrentComponentContent("fn f()fn f()")),
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
            Editor(ReplaceWithClipboard),
            Expect(CurrentComponentContent("fn{ let x = S(a); let y = S(b); }")),
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
            Editor(MoveSelection(Movement::FirstChild)),
            Editor(MoveSelection(Movement::Next)),
            Expect(CurrentSelectedTexts(&["S(spongebob_squarepants)", "S(b)"])),
            Editor(Cut),
            Editor(EnterInsertMode(Direction::Start)),
            Editor(Insert("Some(".to_owned())),
            Editor(ReplaceWithClipboard),
            Editor(Insert(")".to_owned())),
            Expect(CurrentComponentContent(
                "fn f(){ let x = Some(S(spongebob_squarepants)); let y = Some(S(b)); }",
            )),
            Editor(CursorKeepPrimaryOnly),
            App(SetClipboardContent(".hello".to_owned())),
            Editor(ReplaceWithClipboard),
            Expect(CurrentComponentContent(
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
                "Git Hunk Diff".to_string(),
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
                            range: Position { line: 0, column: 0 }..Position { line: 0, column: 0 },
                        },
                        strs_to_strings(&["[This file is untracked by Git]"]),
                    ),
                    QuickfixListItem::new(
                        Location {
                            path: s.foo_rs(),
                            range: Position { line: 0, column: 0 }..Position { line: 1, column: 0 },
                        },
                        strs_to_strings(&["pub struct Foo {", "// Hellopub struct Foo {"]),
                    ),
                    QuickfixListItem::new(
                        Location {
                            path: s.main_rs(),
                            range: Position { line: 0, column: 0 }..Position { line: 0, column: 0 },
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
            Editor(MatchLiteral("fifth()".to_string())),
            Editor(AlignViewTop),
            Expect(AppGrid(
                "
 src/main.rs 🦀
1│fn first () {
5│  █ifth();
6│}

[GLOBAL TITLE]
"
                .to_string(),
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
                .to_string(),
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
                .to_string(),
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
            show_config_after_enter: true,
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
            update(Local, Search("L-Search1".to_string())),
            update(Local, Search("L-Search2".to_string())),
            update(Local, Search("L-Search1".to_string())),
            update(Local, Replacement("L-Replacement1".to_string())),
            update(Local, Replacement("L-Replacement2".to_string())),
            update(Local, Replacement("L-Replacement1".to_string())),
            update(Global, Search("G-Search1".to_string())),
            update(Global, Search("G-Search2".to_string())),
            update(Global, Search("G-Search1".to_string())),
            update(Global, Replacement("G-Replacement1".to_string())),
            update(Global, Replacement("G-Replacement2".to_string())),
            update(Global, Replacement("G-Replacement1".to_string())),
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
                show_config_after_enter: true,
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
            App(new_dispatch(LocalSearchConfigUpdate::Mode(
                LocalSearchConfigMode::Regex(RegexConfig {
                    escaped: true,
                    case_sensitive: false,
                    match_whole_word: false,
                }),
            ))),
            // Replace "foo" with "haha" globally
            App(new_dispatch(LocalSearchConfigUpdate::Search(
                "foo".to_string(),
            ))),
            App(new_dispatch(LocalSearchConfigUpdate::Replacement(
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
fn quickfix_list() -> Result<(), anyhow::Error> {
    execute_test(|s| {
        let owner_id = ComponentId::new();
        let new_dispatch = |update: LocalSearchConfigUpdate| -> Dispatch {
            UpdateLocalSearchConfig {
                owner_id,
                update,
                scope: Scope::Global,
                show_config_after_enter: false,
            }
        };
        Box::new([
            App(OpenFile(s.foo_rs())),
            Editor(SetContent("foo b\nfoo a".to_string())),
            App(OpenFile(s.main_rs())),
            Editor(SetContent("foo d\nfoo c".to_string())),
            App(SaveAll),
            App(new_dispatch(LocalSearchConfigUpdate::Search(
                "foo".to_string(),
            ))),
            Expect(QuickfixListContent(
                format!(
                    "
■┬ {}
 ├ 1: foo b
 └ 2: foo a
■┬ {}
 ├ 1: foo d
 └ 2: foo c
",
                    s.foo_rs().display_absolute(),
                    s.main_rs().display_absolute()
                )
                .trim()
                .to_string(),
            )),
            Expect(QuickfixListCurrentLine("├ 1: foo b")),
            Expect(CurrentPath(s.foo_rs())),
            Expect(CurrentLine("foo b")),
            Expect(CurrentSelectedTexts(&["foo"])),
            Editor(MoveSelection(Next)),
            Expect(QuickfixListCurrentLine("└ 2: foo a")),
            Expect(CurrentLine("foo a")),
            Expect(CurrentSelectedTexts(&["foo"])),
            Editor(MoveSelection(Next)),
            Expect(CurrentLine("foo d")),
            Expect(CurrentSelectedTexts(&["foo"])),
            Editor(MoveSelection(Next)),
            Expect(CurrentLine("foo c")),
            Expect(CurrentSelectedTexts(&["foo"])),
            Editor(MoveSelection(Previous)),
            Expect(CurrentLine("foo d")),
            Expect(CurrentSelectedTexts(&["foo"])),
            Editor(MoveSelection(Previous)),
            Expect(CurrentLine("foo a")),
            Expect(CurrentSelectedTexts(&["foo"])),
            Editor(MoveSelection(Previous)),
            Expect(CurrentLine("foo b")),
            Expect(CurrentSelectedTexts(&["foo"])),
        ])
    })
}

#[test]
fn diagnostic_info() -> Result<(), anyhow::Error> {
    execute_test(|s| {
        Box::new([
            App(OpenFile(s.foo_rs())),
            App(Dispatch::HandleLspNotification(
                LspNotification::PublishDiagnostics(lsp_types::PublishDiagnosticsParams {
                    uri: Url::from_file_path(s.foo_rs()).unwrap(),
                    diagnostics: [lsp_types::Diagnostic::new_simple(
                        lsp_types::Range::new(
                            lsp_types::Position::new(0, 1),
                            lsp_types::Position::new(0, 2),
                        ),
                        "Hello world".to_string(),
                    )]
                    .to_vec(),
                    version: None,
                }),
            )),
            Editor(SetSelectionMode(Diagnostic(None))),
            Expect(EditorInfoOpen(true)),
            Expect(EditorInfoContent("Hello world")),
        ])
    })
}
