/// NOTE: all test cases that involves the clipboard should not be run in parallel
///   otherwise the the test suite will fail because multiple tests are trying to
///   access the clipboard at the same time.
#[cfg(test)]
use itertools::Itertools;

use lazy_regex::regex;
use lsp_types::Url;
use my_proc_macros::{hex, key, keys};

use serde::Serialize;
use serial_test::serial;
use strum::IntoEnumIterator;

use std::{
    path::PathBuf,
    rc::Rc,
    sync::{Arc, Mutex},
};
pub(crate) use Dispatch::*;
pub(crate) use DispatchEditor::*;

pub(crate) use Movement::*;
pub(crate) use SelectionMode::*;

use shared::{
    canonicalized_path::CanonicalizedPath,
    language::{self, LanguageId},
};

#[cfg(test)]
use crate::layout::BufferContentsMap;

use crate::{
    app::{
        App, Dimension, Dispatch, LocalSearchConfigUpdate, RequestParams, Scope,
        StatusLineComponent,
    },
    buffer::{Buffer, BufferOwner},
    char_index_range::CharIndexRange,
    clipboard::CopiedTexts,
    components::{
        component::Component,
        editor::{
            Direction, DispatchEditor, IfCurrentNotFound, Mode, Movement, Reveal, ViewAlignment,
        },
        editor_keymap::KeyboardLayoutKind,
        editor_keymap_printer::KeymapPrintSections,
        keymap_legend::Keymap,
        prompt::PromptHistoryKey,
        suggestive_editor::{DispatchSuggestiveEditor, Info, SuggestiveEditorFilter},
    },
    context::{GlobalMode, LocalSearchConfigMode},
    frontend::{mock::MockFrontend, MyWriter, NullWriter, StringWriter},
    grid::{IndexedHighlightGroup, StyleKey},
    integration_test::{TestOutput, TestRunner},
    list::grep::RegexConfig,
    lsp::{
        code_action::CodeAction,
        completion::{Completion, CompletionItem, CompletionItemEdit, PositionalEdit},
        documentation::Documentation,
        goto_definition_response::GotoDefinitionResponse,
        process::FromEditor,
        signature_help::SignatureInformation,
        workspace_edit::{TextDocumentEdit, WorkspaceEdit},
    },
    position::Position,
    quickfix_list::{DiagnosticSeverityRange, Location, QuickfixListItem, QuickfixListType},
    rectangle::Rectangle,
    selection::{CharIndex, SelectionMode},
    style::Style,
    themes::Theme,
    ui_tree::ComponentKind,
};
use crate::{lsp::process::LspNotification, themes::Color};

#[allow(clippy::large_enum_variant)]
pub(crate) enum Step {
    App(Dispatch),
    AppLater(Box<dyn Fn() -> Dispatch>),
    ExpectMulti(Vec<ExpectKind>),
    Expect(ExpectKind),
    Editor(DispatchEditor),
    SuggestiveEditor(DispatchSuggestiveEditor),
    ExpectLater(Box<dyn Fn() -> ExpectKind>),
    ExpectCustom(Box<dyn Fn()>),
    /// This is to simulate the main event loop,
    /// necessary for testing async features like Global Search
    WaitForAppMessage(&'static lazy_regex::Lazy<regex::Regex>),
}

impl Step {
    fn variant_name(&self) -> String {
        match self {
            Step::App(app) => format!("Step::App({app:?})"),
            AppLater(_) => "AppLater(_)".to_string(),
            ExpectMulti(expects) => format!("ExpectMulti({expects:?})"),
            Expect(expect) => format!("Expect({expect:?})"),
            Editor(editor) => format!("Editor({editor:?})"),
            SuggestiveEditor(editor) => format!("SuggestiveEditor({editor:?})"),
            ExpectLater(_) => "ExpectLater(_)".to_string(),
            ExpectCustom(_) => "ExpectCustom(_)".to_string(),
            WaitForAppMessage(regex) => format!("WaitForAppMessage({regex:?})"),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum ExpectKind {
    // This is just a placeholder for ending a test case without actual assertions.
    // Such test cases are those that just expect the series of actions to not result in failure.
    NoError,
    FileExplorerContent(String),
    EditorInfoContents(&'static [&'static str]),
    GlobalInfoContents(&'static [&'static str]),
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
    CurrentSearch(Scope, &'static str),
    EditorCursorPosition(Position),
    EditorGridCursorPosition(Position),
    EditorIsDirty(),
    CurrentMode(Mode),
    FileContent(CanonicalizedPath, String),
    FileContentEqual(CanonicalizedPath, CanonicalizedPath),
    CurrentSelectedTexts(&'static [&'static str]),
    CurrentPrimarySelection(&'static str),
    CurrentCursorDirection(Direction),
    CurrentViewAlignment(Option<ViewAlignment>),
    ComponentsLength(usize),
    Quickfixes(Box<[QuickfixListItem]>),
    AppGrid(String),
    AppGridContains(&'static str),
    EditorGrid(&'static str),
    CurrentPath(CanonicalizedPath),
    GridCellBackground(
        /*Row*/ usize,
        /*Column*/ usize,
        /*Background color*/ Color,
    ),
    GridCellLine(/*Row*/ usize, /*Column*/ usize, Color),
    GridCellStyleKey(Position, Option<StyleKey>),
    GridCellsStyleKey(Vec<Position>, Option<StyleKey>),
    RangeStyleKey(/*Search*/ &'static str, Option<StyleKey>),
    HighlightSpans(std::ops::Range<usize>, StyleKey),
    DiagnosticsRanges(Vec<CharIndexRange>),
    BufferQuickfixListItems(Vec<CharIndexRange>),
    ComponentCount(usize),
    CurrentComponentPath(Option<CanonicalizedPath>),
    OpenedFilesCount(usize),
    GlobalInfo(&'static str),
    ComponentsOrder(Vec<ComponentKind>),
    CurrentComponentTitle(String),
    CurrentSelectionMode(SelectionMode),
    CurrentGlobalMode(Option<GlobalMode>),
    LspRequestSent(FromEditor),
    LspServerInitializedArgs(Option<(LanguageId, Vec<CanonicalizedPath>)>),
    CurrentCopiedTextHistoryOffset(isize),
    CurrentReveal(Option<Reveal>),
    CountHighlightedCells(StyleKey, usize),
    SelectionExtensionEnabled(bool),
    PromptHistory(PromptHistoryKey, Vec<String>),
}
fn log<T: std::fmt::Debug>(s: T) {
    if !is_ci::cached() {
        println!("===========\n{s:?}",);
    }
}
impl ExpectKind {
    fn run(&self, app: &mut App<MockFrontend>) -> anyhow::Result<()> {
        log(self);
        let (result, context) = self.get_result(app).unwrap();
        if result {
            Ok(())
        } else {
            Err(anyhow::anyhow!("{context}"))
        }
    }
    fn get_result(&self, app: &mut App<MockFrontend>) -> anyhow::Result<(bool, String)> {
        let context = app.context();
        fn contextualize<T: PartialEq + std::fmt::Debug>(a: T, b: T) -> (bool, String) {
            (a == b, format!("\n{a:?}\n == \n{b:?}\n",))
        }
        fn to_vec(strs: &[&str]) -> Vec<String> {
            strs.iter().map(|t| t.to_string()).collect()
        }
        let component = app.current_component();
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
                                app.get_quickfix_list()
                                    .map(|q| {
                                        q.items()
                                            .into_iter()
                                            .map(|quickfix| {
                                                let info = quickfix
                                                    .info()
                                                    .as_ref()
                                                    .map(|info| info.clone().set_decorations(Vec::new()));
                                                quickfix.set_info(info)
                                            })
                                            .collect_vec()
                                            .into_boxed_slice()
                                    })
                                    .unwrap_or_default(),
                                expected_quickfixes.clone(),
                            ),
            EditorGrid(grid) => contextualize(
                component
                    .borrow_mut()
                    .editor_mut()
                    .get_grid(context, false)
                    .to_string(),
                grid.to_string(),
            ),
            AppGrid(grid) => {
                                let expected = grid.to_string().trim_matches('\n').to_string();
                                let actual = app.get_screen()?.stringify().trim_matches('\n').to_string();
                                println!("Expected=\n{expected}");
                                println!("Actual=\n{actual}");
                                contextualize(actual, expected)
                            }
            CurrentPath(path) => contextualize(app.get_current_file_path().unwrap(), path.clone()),
            Not(expect_kind) => {
                                let (result, context) = expect_kind.get_result(app)?;
                                (!result, format!("NOT ({context})"))
                            }
            EditorIsDirty() => contextualize(&component.borrow().editor().buffer().dirty(), &true),
            CurrentMode(mode) => contextualize(&component.borrow().editor().mode, mode),
            EditorCursorPosition(position) => contextualize(
                                &component.borrow().editor().get_cursor_position().unwrap(),
                                position,
                            ),
            EditorGridCursorPosition(position) => contextualize(
                                component
                                    .borrow_mut()
                                    .editor_mut()
                                    .get_grid(context, false)
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
                                contextualize(
                                    component.borrow().editor().jump_chars().into_iter().sorted().collect_vec(),
                                    chars.iter().sorted().cloned().collect_vec()
                                )
                            }
            CurrentViewAlignment(view_alignment) => contextualize(
                                component.borrow().editor().current_view_alignment(),
                                *view_alignment,
                            ),
            GridCellBackground(row_index, column_index, background_color) => {
                let grid = component
                        .borrow_mut()
                        .editor_mut()
                        .get_grid(context, false)
                        .grid;
                contextualize(
                    grid
                        .rows[*row_index][*column_index]
                        .background_color,
                    *background_color,
                )
            },
            GridCellLine(row_index, column_index, underline_color) => contextualize(
                                component
                                    .borrow_mut()
                                    .editor_mut()
                                    .get_grid(context, false)
                                    .grid
                                    .rows[*row_index][*column_index]
                                    .line
                                    .unwrap()
                                    .color,
                                *underline_color,
                            ),
            GridCellStyleKey(position, style_key) => {
                                println!(
                                    "ExpectKind::get_result grid styles = {:?}",
                                    component
                                        .borrow_mut()
                                        .editor_mut()
                                        .get_grid(context, false)
                                        .grid
                                        .rows
                                        .iter()
                                        .map(|row|row.iter().map(|cell| cell.source.clone()).collect_vec())
                                        .collect_vec()
                                );
                                contextualize(
                                    component
                                        .borrow_mut()
                                        .editor_mut()
                                        .get_grid(context, true)
                                        .grid
                                        .rows[position.line][position.column]
                                        .source
                                        .clone(),
                                    style_key.clone(),
                                )
                            },
            GridCellsStyleKey(positions, style_key) => (
                                positions.iter().all(|position| {
                                    let actual_style_key = &component
                                        .borrow_mut()
                                        .editor_mut()
                                        .get_grid(context, false)
                                        .grid
                                        .rows[position.line][position.column]
                                        .source;
                                    if actual_style_key!=style_key {
                                        log(format!("Expected {position:?} to be styled as {style_key:?}, but got {actual_style_key:?}"));
                                    }
                                    actual_style_key == style_key
                                }),
                                format!("Expected positions {positions:?} to be styled as {style_key:?}"),
                            ),
            RangeStyleKey(search, style_key) => {
                                let grid = component.borrow_mut().editor_mut().get_grid(context, false);
                                let grid_string = grid.to_string();
                                let matches = grid_string.match_indices(search).collect_vec();
                                let byte_range = match matches.split_first() {
                                    Some(((byte_start, str),[])) => *byte_start..byte_start + str.len(),
                                    Some((_,_)) =>
                                        panic!("{search:?} should only match 1 range, but it matches {} ranges.", matches.len()),
                                    None =>
                                        panic!("{search:?} should only match 1 range, but it matches nothing."),
                                };
                                // We use Buffer to obtain the position range given the byte range
                                let buffer = Buffer::new(None, &grid_string);
                                let positions = byte_range.map(|byte|buffer.byte_to_position(byte).unwrap()).collect_vec();
                                if positions.is_empty() {
                                    panic!("There are 0 positions");
                                }
                                (positions.iter().all(|position| {
                                    let actual_style_key = &grid
                                        .grid
                                        .rows[position.line][position.column]
                                        .source;
                                    if actual_style_key!=style_key {
                                        log(format!("Expected {position:?} to be styled as {style_key:?}, but got {actual_style_key:?}"));
                                    }
                                    actual_style_key == style_key
                                }),
                                format!("Expected positions {positions:?} to be styled as {style_key:?}"))
                            },
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
                                    .editor()
                                    .get_selected_texts()[0]
                                    .trim(),
                                item,
                            ),
            QuickfixListContent(content) => {
                                let actual = app.get_quickfix_list().unwrap().render().content;
                                let expected = content.to_string();
                                println!("Expected =\n{expected}");
                                println!("Actual =\n{actual}");
                                contextualize(expected, actual)
                            }
            DropdownInfosCount(expected) => {
                                contextualize(app.get_dropdown_infos_count(), *expected)
                            }
            QuickfixListCurrentLine(expected) => {
                                let component = app
                                    .get_component_by_kind(ComponentKind::QuickfixList)
                                    .unwrap();
                                let actual = component.borrow().editor().current_line().unwrap();
                                contextualize(actual, expected.to_string())
                            }
            EditorInfoContents(expected) => {
                                contextualize(
                                    app.editor_info_contents(),
                                    expected.iter().map(|s|s.to_string()).collect()
                                )
                            }
            GlobalInfoContents(expected) => {
                                contextualize(
                                    app.global_info_contents(),
                                    expected.iter().map(|s|s.to_string()).collect()
                                )
                            }
            AppGridContains(substring) => {
                                let content = app.get_screen().unwrap().stringify();
                                contextualize(content.contains(substring), true)
                            }
            FileExplorerContent(expected) => contextualize(expected, &app.file_explorer_content()),
            CurrentCursorDirection(expected) => contextualize(
                                expected,
                                &app.current_component().borrow().editor().cursor_direction,
                            ),
            HighlightSpans(expected_range, expected_key) => contextualize(
                                expected_key,
                                &app.current_component()
                                    .borrow()
                                    .editor()
                                    .buffer()
                                    .highlighted_spans()
                                    .iter()
                                    .inspect(|&span| {
                                        // For debugging purposes
                                        log(format!("xx {span:?} {}", span.style_key.display()));
                                    })
                                    .collect_vec()
                                    .into_iter()
                                    .find(|span| &span.byte_range == expected_range)
                                    .unwrap()
                                    .style_key,
                            ),
            DiagnosticsRanges(expected) => contextualize(
                                expected.to_vec(),
                                app.current_component()
                                    .borrow()
                                    .editor()
                                    .buffer()
                                    .diagnostics()
                                    .into_iter()
                                    .map(|d| d.range)
                                    .collect_vec(),
                            ),
            BufferQuickfixListItems(expected) => contextualize(
                                expected,
                                &app.context()
                                    .quickfix_list_items()
                                    .into_iter()
                                    .map(|d| d.location().range)
                                    .collect_vec(),
                            ),
            ComponentCount(expected) => contextualize(expected, &app.components().len()),
            CurrentComponentPath(expected) => {
                                contextualize(expected, &app.current_component().borrow().path())
                            }
            OpenedFilesCount(expected) => contextualize(expected, &app.opened_files_count()),
            GlobalInfo(expected) => {
                                contextualize(*expected, &app.global_info().unwrap())
                            }
            ComponentsOrder(expected) => contextualize(expected, &app.components_order()),
            CurrentComponentTitle(expected) => {
                                // Provide a minimal height and width
                                // so that the tabline can be rendered properly,
                                // so that we do not need keep adding Editor(SetRectangle(rectangle))
                                // to test cases that are testing for CurrentComponentTitle.
                                app.handle_dispatch(Dispatch::TerminalDimensionChanged(Dimension {
                                    height: 10,
                                    width: 30,
                                }))?;
                                contextualize(expected, &app.current_component().borrow().title(app.context()))
                            }
            CurrentSelectionMode(expected) => contextualize(
                                expected,
                                app.current_component().borrow().editor().selection_set.mode(),
                            ),
            LspRequestSent(from_editor) => contextualize(true, app.lsp_request_sent(from_editor)),
            LspServerInitializedArgs(expected) => contextualize(expected, &app.lsp_server_initialized_args()),
            CurrentCopiedTextHistoryOffset(expected) => contextualize(
                                expected,
                                &app.current_component()
                                    .borrow()
                                    .editor()
                                    .copied_text_history_offset(),
                            ),
            CurrentPrimarySelection(expected) => contextualize(
                                *expected,
                                &app.current_component()
                                    .borrow()
                                    .editor()
                                    .primary_selection()?,
                            ),
            CurrentGlobalMode(expected) => contextualize(expected, &app.context().mode()),
            CurrentReveal(expected) => {
                                contextualize(expected, &app.current_component().borrow().editor().reveal)
                            }
            CountHighlightedCells(style_key, expected_count) => contextualize(
                                expected_count,
                                &app.current_component()
                                    .borrow_mut()
                                    .editor_mut()
                                    .get_grid(context, false)
                                    .grid
                                    .rows
                                    .into_iter()
                                    .map(|row| {
                                        row.iter()
                                            .filter(|cell| {
                                                log(format!("style = {:?}", cell.source));
                                                cell.source.as_ref() == Some(style_key)
                                            })
                                            .count()
                                    })
                                    .sum::<usize>(),
                            ),
            SelectionExtensionEnabled(expected) => contextualize(expected, &app.current_component().borrow().editor().selection_extension_enabled()),
            CurrentSearch(scope,expected) => contextualize(*expected, &app.context().local_search_config(*scope).search()),
            PromptHistory(key, expected) => contextualize(
                        expected,
                        &app.context().get_prompt_history(*key)
                    ),
            NoError => (true, String::new()),
        })
    }
}

pub(crate) use ExpectKind::*;
pub(crate) use Step::*;
pub(crate) struct State {
    temp_dir: CanonicalizedPath,
    main_rs: CanonicalizedPath,
    foo_rs: CanonicalizedPath,
    hello_ts: CanonicalizedPath,
    git_ignore: CanonicalizedPath,
}
impl State {
    pub(crate) fn main_rs(&self) -> CanonicalizedPath {
        self.main_rs.clone()
    }

    pub(crate) fn foo_rs(&self) -> CanonicalizedPath {
        self.foo_rs.clone()
    }

    pub(crate) fn hello_ts(&self) -> CanonicalizedPath {
        self.hello_ts.clone()
    }

    pub(crate) fn new_path(&self, path: &str) -> PathBuf {
        self.temp_dir.to_path_buf().join(path)
    }

    pub(crate) fn gitignore(&self) -> CanonicalizedPath {
        self.git_ignore.clone()
    }

    pub(crate) fn temp_dir(&self) -> CanonicalizedPath {
        self.temp_dir.clone()
    }
}

pub(crate) fn execute_test(callback: impl Fn(State) -> Box<[Step]>) -> anyhow::Result<()> {
    execute_test_helper(
        || Box::new(NullWriter),
        false,
        [StatusLineComponent::LastDispatch].to_vec(),
        callback,
        true,
    )?;
    Ok(())
}

pub(crate) fn execute_recipe(
    callback: impl Fn(State) -> Box<[Step]>,
    assert_last_step_is_expect: bool,
) -> anyhow::Result<TestOutput> {
    execute_test_helper(
        || Box::new(StringWriter::new()),
        true,
        [
            StatusLineComponent::Mode,
            StatusLineComponent::SelectionMode,
            StatusLineComponent::LastSearchString,
            StatusLineComponent::LastDispatch,
        ]
        .to_vec(),
        callback,
        assert_last_step_is_expect,
    )
}

fn execute_test_helper(
    writer: fn() -> Box<dyn MyWriter>,
    render: bool,
    status_line_components: Vec<StatusLineComponent>,
    callback: impl Fn(State) -> Box<[Step]>,
    assert_last_step_is_expect: bool,
) -> anyhow::Result<TestOutput> {
    run_test(writer, status_line_components, |mut app, temp_dir| {
        let steps = {
            callback(State {
                main_rs: temp_dir.join("src/main.rs").unwrap(),
                foo_rs: temp_dir.join("src/foo.rs").unwrap(),
                hello_ts: temp_dir.join("src/hello.ts").unwrap(),
                git_ignore: temp_dir.join(".gitignore").unwrap(),
                temp_dir,
            })
        };

        if render {
            app.render()?
        }
        if assert_last_step_is_expect {
            debug_assert!(
                matches!(
                    steps.iter().last(),
                    None | Some(
                        Step::Expect(_)
                            | Step::ExpectLater(_)
                            | Step::ExpectMulti(_)
                            | Step::ExpectCustom(_)
                    )
                ),
                "The last step of each recipe must be an assertion but got {:?}",
                steps.iter().last().map(|step| { step.variant_name() })
            );
        }

        for step in steps.iter() {
            match step.to_owned() {
                Step::WaitForAppMessage(regex) => app.handle_next_app_messages(regex)?,
                Step::App(dispatch) => {
                    log(dispatch);
                    app.handle_dispatch(dispatch.to_owned())?
                }
                Step::AppLater(get_dispatch) => {
                    let dispatch = get_dispatch();
                    log(&dispatch);
                    app.handle_dispatch(dispatch.to_owned())?
                }
                Step::Expect(expect_kind) => expect_kind.run(&mut app)?,
                ExpectLater(f) => f().run(&mut app)?,
                Editor(dispatch) => {
                    log(dispatch);
                    app.handle_dispatch_editor(dispatch.to_owned())?
                }
                ExpectCustom(f) => {
                    f();
                }
                ExpectMulti(expect_kinds) => {
                    for expect_kind in expect_kinds.iter() {
                        expect_kind.run(&mut app)?
                    }
                }
                SuggestiveEditor(dispatch) => {
                    log(dispatch);
                    app.handle_dispatch_suggestive_editor(dispatch.to_owned())?
                }
            };
        }

        if render {
            app.render()?
        }
        let buffer_contents = app.get_buffer_contents_map();
        Ok(buffer_contents)
    })
}

fn run_test(
    writer: fn() -> Box<dyn MyWriter>,
    status_line_components: Vec<StatusLineComponent>,
    callback: impl Fn(App<MockFrontend>, CanonicalizedPath) -> anyhow::Result<BufferContentsMap>,
) -> anyhow::Result<TestOutput> {
    TestRunner::run(move |temp_dir| {
        let frontend = Rc::new(Mutex::new(MockFrontend::new(writer())));
        let mut app = App::new(
            frontend.clone(),
            temp_dir.clone(),
            status_line_components.clone(),
        )?;
        app.disable_lsp();
        let buffer_contents_map = callback(app, temp_dir)?;
        use std::borrow::Borrow;
        let term_output = frontend.lock().unwrap().borrow().string_content();
        let output = TestOutput {
            term_output,
            buffer_contents_map,
        };

        Ok(output)
    })
}

#[serial]
#[test]
fn copy_replace_from_different_file() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            App(OpenFile {
                path: s.foo_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Line)),
            Editor(SelectAll),
            Editor(Copy),
            App(OpenFile {
                path: s.foo_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Line)),
            Editor(SelectAll),
            Editor(Copy),
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Line)),
            Editor(SelectAll),
            Editor(ReplaceWithCopiedText { cut: false }),
            Expect(FileContentEqual(s.main_rs, s.foo_rs)),
        ])
    })
}

#[serial]
#[test]
/// Should work across different files
fn replace_cut() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("fn main() { call_main() }".to_string())),
            App(OpenFile {
                path: s.foo_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("fn foo() { call_foo() }".to_string())),
            Editor(MatchLiteral("call_foo()".to_string())),
            Editor(Copy),
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(MatchLiteral("call_main()".to_string())),
            Editor(ReplaceWithCopiedText { cut: true }),
            Expect(CurrentComponentContent("fn main() { call_foo() }")),
            App(OpenFile {
                path: s.foo_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(ReplaceWithCopiedText { cut: false }),
            Expect(CurrentComponentContent("fn foo() { call_main() }")),
        ])
    })
}

#[serial]
#[test]
fn copy_replace() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("fn main() { let x = 1; }".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
            Editor(Copy),
            Editor(MoveSelection(Right)),
            Editor(ReplaceWithCopiedText { cut: false }),
            Expect(CurrentComponentContent("fn fn() { let x = 1; }")),
            Expect(CurrentSelectedTexts(&["fn"])),
            Editor(MoveSelection(Next)),
            Editor(ReplaceWithCopiedText { cut: false }),
            Expect(CurrentComponentContent("fn fnfn) { let x = 1; }")),
        ])
    })
}

#[serial]
#[test]
fn cut_replace() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("fn main() { let x = 1; }".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
            Editor(ChangeCut),
            Editor(EnterNormalMode),
            Expect(CurrentComponentContent(" main() { let x = 1; }")),
            Editor(MatchLiteral("main".to_string())),
            Editor(ReplaceWithCopiedText { cut: false }),
            Expect(CurrentComponentContent(" fn() { let x = 1; }")),
        ])
    })
}

#[serial]
#[test]
fn highlight_mode_cut() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(
                "fn f(){ let x = S(a); let y = S(b); }".to_string(),
            )),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
            Editor(EnableSelectionExtension),
            Editor(MoveSelection(Right)),
            Editor(MoveSelection(Next)),
            Editor(MoveSelection(Next)),
            Expect(CurrentSelectedTexts(&["fn f()"])),
            Editor(ChangeCut),
            Expect(CurrentComponentContent("{ let x = S(a); let y = S(b); }")),
            Editor(ReplaceWithCopiedText { cut: false }),
            Expect(CurrentComponentContent(
                "fn f(){ let x = S(a); let y = S(b); }",
            )),
        ])
    })
}

#[serial]
#[test]
fn highlight_mode_copy() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(
                "fn f(){ let x = S(a); let y = S(b); }".to_string(),
            )),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
            Editor(EnableSelectionExtension),
            Editor(MoveSelection(Right)),
            Editor(MoveSelection(Next)),
            Editor(MoveSelection(Next)),
            Expect(CurrentSelectedTexts(&["fn f()"])),
            Editor(Copy),
            Editor(Reset),
            Editor(MoveSelection(Next)),
            Expect(CurrentSelectedTexts(&["{"])),
            Editor(ReplaceWithCopiedText { cut: false }),
            Expect(CurrentComponentContent(
                "fn f()fn f() let x = S(a); let y = S(b); }",
            )),
        ])
    })
}

#[serial]
#[test]
fn highlight_mode_replace() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(
                "fn f(){ let x = S(a); let y = S(b); }".to_string(),
            )),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
            Editor(EnableSelectionExtension),
            Editor(MoveSelection(Movement::Right)),
            Editor(MoveSelection(Movement::Next)),
            Editor(MoveSelection(Movement::Next)),
            Expect(CurrentSelectedTexts(&["fn f()"])),
            Editor(Copy),
            Editor(Reset),
            Editor(MatchLiteral("{".to_string())),
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                SelectionMode::SyntaxNode,
            )),
            Expect(CurrentSelectedTexts(&["{ let x = S(a); let y = S(b); }"])),
            Editor(ReplaceWithCopiedText { cut: false }),
            Expect(CurrentComponentContent("fn f()fn f()")),
        ])
    })
}

#[serial]
#[test]
fn multi_paste() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Expect(CurrentComponentPath(Some(s.main_rs()))),
            Editor(SetContent(
                "fn f(){ let x = S(spongebob_squarepants); let y = S(b); }".to_string(),
            )),
            Editor(MatchLiteral("let x = S(spongebob_squarepants);".to_owned())),
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                SelectionMode::SyntaxNode,
            )),
            Expect(CurrentSelectedTexts(&["let x = S(spongebob_squarepants);"])),
            Editor(CursorAddToAllSelections),
            Editor(MoveSelection(Movement::Down)),
            Editor(MoveSelection(Movement::Right)),
            Expect(CurrentSelectedTexts(&["S(spongebob_squarepants)", "S(b)"])),
            Editor(ChangeCut),
            Editor(EnterInsertMode(Direction::Start)),
            Editor(Insert("Some(".to_owned())),
            Editor(Paste),
            Editor(Insert(")".to_owned())),
            Expect(CurrentComponentContent(
                "fn f(){ let x = Some(S(spongebob_squarepants)); let y = Some(S(b)); }",
            )),
            Editor(CursorKeepPrimaryOnly),
            App(SetClipboardContent {
                copied_texts: CopiedTexts::one(".hello".to_owned()),
            }),
            Editor(Paste),
            Expect(CurrentComponentContent(
                "fn f(){ let x = Some(S(spongebob_squarepants)).hello; let y = Some(S(b)); }",
            )),
        ])
    })
}

#[test]
fn signature_help() -> anyhow::Result<()> {
    execute_test(|s| {
        fn signature_help() -> LspNotification {
            LspNotification::SignatureHelp(Some(crate::lsp::signature_help::SignatureHelp {
                signatures: [SignatureInformation {
                    label: "Signature Help".to_string(),
                    documentation: Some(crate::lsp::documentation::Documentation {
                        content: "spongebob".to_string(),
                    }),
                    active_parameter_byte_range: None,
                }]
                .to_vec(),
            }))
        }
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Expect(ComponentsLength(1)),
            Editor(SetContent(
                "fn f(){ let x = S(a); let y = S(b); }".to_string(),
            )),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
            Expect(CurrentMode(Mode::Normal)),
            //
            // Signature help should not be shown in normal mode
            App(HandleLspNotification(signature_help())),
            Expect(ExpectKind::ComponentsOrder(vec![
                ComponentKind::SuggestiveEditor,
            ])),
            //
            // Signature help should only be shown in insert mode
            Editor(EnterInsertMode(Direction::End)),
            App(HandleLspNotification(signature_help())),
            Expect(ExpectKind::ComponentsOrder(vec![
                ComponentKind::SuggestiveEditor,
                ComponentKind::EditorInfo,
            ])),
            //
            // Receiving signature help again should increase the components length
            App(HandleLspNotification(signature_help())),
            Expect(ExpectKind::ComponentsOrder(vec![
                ComponentKind::SuggestiveEditor,
                ComponentKind::EditorInfo,
            ])),
            //
            // Pressing esc should close signature help
            App(HandleKeyEvent(key!("esc"))),
            Expect(ExpectKind::ComponentsOrder(vec![
                ComponentKind::SuggestiveEditor,
            ])),
            //
            // Receiving null signature help should close the signature help
            Editor(EnterInsertMode(Direction::End)),
            App(HandleLspNotification(signature_help())),
            Expect(ExpectKind::ComponentsOrder(vec![
                ComponentKind::SuggestiveEditor,
                ComponentKind::EditorInfo,
            ])),
            App(HandleLspNotification(LspNotification::SignatureHelp(None))),
            Expect(ExpectKind::ComponentsOrder(vec![
                ComponentKind::SuggestiveEditor,
            ])),
        ])
    })
}

#[test]
pub(crate) fn repo_git_hunks() -> Result<(), anyhow::Error> {
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
            App(OpenFile {
                path: s.main_rs().clone(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Line)),
            Editor(Delete),
            // Insert a comment at the first line of foo.rs
            App(OpenFile {
                path: s.foo_rs().clone(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(EnterInsertMode(Direction::Start)),
            App(HandleKeyEvents(keys!("/ / space H e l l o").to_vec())),
            // Save the files,
            App(SaveAll),
            // Add a new file
            App(AddPath(path_new_file.display().to_string())),
            // Get the repo hunks
            App(GetRepoGitHunks(
                crate::git::DiffMode::UnstagedAgainstCurrentBranch,
            )),
            Step::ExpectLater(Box::new(move || {
                Quickfixes(Box::new([
                    QuickfixListItem::new(
                        Location {
                            path: path_new_file.clone().try_into().unwrap(),
                            range: (CharIndex(0)..CharIndex(0)).into(),
                        },
                        strs_to_strings(&["[This file is untracked or renamed]"]),
                        None,
                    ),
                    QuickfixListItem::new(
                        Location {
                            path: s.foo_rs(),
                            range: (CharIndex(0)..CharIndex(32)).into(),
                        },
                        strs_to_strings(&[
                            "pub(crate) struct Foo {",
                            "// Hellopub(crate) struct Foo {",
                        ]),
                        None,
                    ),
                    QuickfixListItem::new(
                        Location {
                            path: s.main_rs(),
                            range: (CharIndex(0)..CharIndex(0)).into(),
                        },
                        strs_to_strings(&["mod foo;"]),
                        None,
                    ),
                ]))
            })),
        ])
    })
}

#[test]
pub(crate) fn revert_git_hunk() -> Result<(), anyhow::Error> {
    let original_content = "pub(crate) struct Foo {
    a: (),
    b: (),
}

pub(crate) fn foo() -> Foo {
    Foo { a: (), b: () }
}
";
    execute_test(|s| {
        Box::new([
            // Insert a comment at the first line of foo.rs
            App(OpenFile {
                path: s.foo_rs().clone(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Expect(CurrentComponentContent(original_content)),
            Editor(EnterInsertMode(Direction::Start)),
            App(HandleKeyEvents(keys!("/ / space H e l l o").to_vec())),
            Expect(CurrentComponentContent(
                "// Hellopub(crate) struct Foo {
    a: (),
    b: (),
}

pub(crate) fn foo() -> Foo {
    Foo { a: (), b: () }
}
",
            )),
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                GitHunk(crate::git::DiffMode::UnstagedAgainstMainBranch),
            )),
            Expect(CurrentSelectedTexts(&["// Hellopub(crate) struct Foo {\n"])),
            Editor(RevertHunk(
                crate::git::DiffMode::UnstagedAgainstCurrentBranch,
            )),
            Expect(CurrentComponentContent(original_content)),
            Expect(CurrentSelectedTexts(&["pub(crate) struct Foo {\n"])),
        ])
    })
}

#[test]
pub(crate) fn non_git_ignored_files() -> Result<(), anyhow::Error> {
    execute_test(|s| {
        let temp_dir = s.temp_dir();
        Box::new([
            // Ignore *.txt files
            App(OpenFile {
                path: s.gitignore(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(EnterInsertMode(Direction::End)),
            App(HandleKeyEvents(keys!("enter * . t x t").to_vec())),
            App(SaveAll),
            // Add new txt file
            App(AddPath(s.new_path("temp.txt").display().to_string())),
            // Add a hidden file
            App(AddPath(s.new_path(".bashrc").display().to_string())),
            // Add a file under `.git` folder
            App(AddPath(s.new_path(".git/hello").display().to_string())),
            // Add a new Rust file
            App(AddPath(s.new_path("src/rust.rs").display().to_string())),
            ExpectCustom(Box::new(move || {
                let (sender, receiver) = std::sync::mpsc::channel();
                crate::list::WalkBuilderConfig::get_non_git_ignored_files(
                    temp_dir.clone().to_path_buf().clone(),
                    Arc::new(move |path| {
                        let _ = sender.send(path);
                    }),
                );

                let paths = receiver.into_iter().collect_vec();

                // Expect all the paths are files, not directory for example
                assert!(paths.iter().all(|file| file.is_file()));

                let paths = paths
                    .into_iter()
                    .flat_map(|path| {
                        CanonicalizedPath::try_from(path)
                            .unwrap()
                            .display_relative_to(&s.temp_dir())
                    })
                    .collect_vec();

                // Expect "temp.txt" is not in the list, since it is git-ignored
                assert!(!paths.contains(&"temp.txt".to_string()));

                // Expect the unstaged file "src/rust.rs" is in the list
                assert!(paths.contains(&"src/rust.rs".to_string()));

                // Expect the staged file "main.rs" is in the list
                assert!(paths.contains(&"src/main.rs".to_string()));

                // Expect the hidden file ".bashrc" is in the list
                assert!(paths.contains(&".bashrc".to_string()));

                // Expect files under ".git" is ignored
                assert!(!paths.contains(&".git/hello".to_string()));
            })),
        ])
    })
}

#[test]
fn number_of_lines_rendered_should_equal_to_number_of_newline_characters_plus_one(
) -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            App(TerminalDimensionChanged(Dimension {
                width: 200,
                height: 6,
            })),
            Editor(SetContent("hello\n".to_string())),
            Expect(AppGrid(
                " 🦀  main.rs [*]
1│█ello
2│"
                .to_string(),
            )),
        ])
    })
}

#[test]
fn align_view_bottom_with_outbound_parent_lines() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(SetGlobalTitle("[GLOBAL TITLE]".to_string())),
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            App(TerminalDimensionChanged(Dimension {
                width: 200,
                height: 6,
            })),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Line)),
            Editor(SelectAll),
            Editor(Delete),
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
 🦀  main.rs [*]
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
 🦀  main.rs [*]
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
                width: 21,
                height: 6,
            })),
            Editor(AlignViewBottom),
            Expect(AppGrid(
                "
 🦀  main.rs [*]
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
fn global_marks() -> Result<(), anyhow::Error> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Subword)),
            Editor(ToggleMark),
            App(OpenFile {
                path: s.foo_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Subword)),
            Editor(ToggleMark),
            App(SetQuickfixList(
                crate::quickfix_list::QuickfixListType::Mark,
            )),
            Expect(Quickfixes(Box::new([
                QuickfixListItem::new(
                    Location {
                        path: s.foo_rs(),
                        range: (CharIndex(0)..CharIndex(3)).into(),
                    },
                    None,
                    None,
                ),
                QuickfixListItem::new(
                    Location {
                        path: s.main_rs(),
                        range: (CharIndex(0)..CharIndex(3)).into(),
                    },
                    None,
                    None,
                ),
            ]))),
        ])
    })
}

#[test]
fn esc_global_quickfix_mode() -> Result<(), anyhow::Error> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("foo bar foo bar".to_string())),
            Editor(ToggleMark),
            App(OpenFile {
                path: s.foo_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("foo bar foo bar".to_string())),
            App(SaveAll),
            App(UpdateLocalSearchConfig {
                update: LocalSearchConfigUpdate::Search("bar".to_string()),
                scope: Scope::Global,
                if_current_not_found: IfCurrentNotFound::LookForward,
                run_search_after_config_updated: true,
            }),
            WaitForAppMessage(regex!("AddQuickfixListEntries")),
            Expect(CurrentGlobalMode(Some(GlobalMode::QuickfixListItem))),
            Expect(Quickfixes(Box::new([
                QuickfixListItem::new(
                    Location {
                        path: s.foo_rs(),
                        range: (CharIndex(4)..CharIndex(7)).into(),
                    },
                    None,
                    Some("foo bar foo bar".to_string()),
                ),
                QuickfixListItem::new(
                    Location {
                        path: s.foo_rs(),
                        range: (CharIndex(12)..CharIndex(15)).into(),
                    },
                    None,
                    Some("foo bar foo bar".to_string()),
                ),
                QuickfixListItem::new(
                    Location {
                        path: s.main_rs(),
                        range: (CharIndex(4)..CharIndex(7)).into(),
                    },
                    None,
                    Some("foo bar foo bar".to_string()),
                ),
                QuickfixListItem::new(
                    Location {
                        path: s.main_rs(),
                        range: (CharIndex(12)..CharIndex(15)).into(),
                    },
                    None,
                    Some("foo bar foo bar".to_string()),
                ),
            ]))),
            App(HandleKeyEvent(key!("esc"))),
            Expect(CurrentGlobalMode(None)),
            Expect(CurrentSelectionMode(LocalQuickfix {
                title: "Global search".to_string(),
            })),
        ])
    })
}

#[test]
fn local_lsp_references() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(
                "fn f(){ let x = S(spongebob_squarepants); let y = S(b); }".to_string(),
            )),
            App(HandleLspNotification(LspNotification::References(
                crate::lsp::process::ResponseContext {
                    scope: Some(Scope::Local),
                    description: None,
                },
                [
                    Location {
                        path: s.main_rs(),
                        range: (CharIndex(0)..CharIndex(2)).into(),
                    },
                    Location {
                        path: s.main_rs(),
                        range: (CharIndex(3)..CharIndex(4)).into(),
                    },
                ]
                .to_vec(),
            ))),
            Editor(CursorAddToAllSelections),
            Expect(CurrentSelectedTexts(&["fn", "f"])),
        ])
    })
}

#[test]
fn global_diagnostics() -> Result<(), anyhow::Error> {
    execute_test(|s| {
        let publish_diagnostics = |path: CanonicalizedPath| {
            LspNotification::PublishDiagnostics(lsp_types::PublishDiagnosticsParams {
                uri: path.to_url().unwrap(),
                diagnostics: [lsp_types::Diagnostic {
                    range: lsp_types::Range::new(
                        lsp_types::Position {
                            line: 0,
                            character: 0,
                        },
                        lsp_types::Position {
                            line: 0,
                            character: 3,
                        },
                    ),
                    message: "To err is normal, but to err again is not.".to_string(),
                    ..Default::default()
                }]
                .to_vec(),
                version: None,
            })
        };
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            App(HandleLspNotification(publish_diagnostics(s.main_rs()))),
            App(HandleLspNotification(publish_diagnostics(s.foo_rs()))),
            App(SetQuickfixList(
                crate::quickfix_list::QuickfixListType::Diagnostic(DiagnosticSeverityRange::All),
            )),
            Expect(Quickfixes(Box::new([
                QuickfixListItem::new(
                    Location {
                        path: s.foo_rs(),
                        range: (CharIndex(0)..CharIndex(3)).into(),
                    },
                    Some(Info::new(
                        "Diagnostics".to_string(),
                        "To err is normal, but to err again is not.".to_string(),
                    )),
                    None,
                ),
                QuickfixListItem::new(
                    Location {
                        path: s.main_rs(),
                        range: (CharIndex(0)..CharIndex(3)).into(),
                    },
                    Some(Info::new(
                        "Diagnostics".to_string(),
                        "To err is normal, but to err again is not.".to_string(),
                    )),
                    None,
                ),
            ]))),
        ])
    })
}

fn test_global_search_replace(
    TestGlobalSearchReplaceArgs {
        mode,
        main_content,
        foo_content,
        search,
        replacement,
        main_replaced,
        foo_replaced,
    }: TestGlobalSearchReplaceArgs,
) -> anyhow::Result<()> {
    execute_test(|s| {
        let new_dispatch = |update: LocalSearchConfigUpdate| -> Dispatch {
            UpdateLocalSearchConfig {
                update,
                scope: Scope::Global,
                if_current_not_found: IfCurrentNotFound::LookForward,
                run_search_after_config_updated: true,
            }
        };
        let main_rs = s.main_rs();
        Box::new([
            App(OpenFile {
                path: s.foo_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(foo_content.to_string())),
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(main_content.to_string())),
            App(SaveAll),
            App(new_dispatch(LocalSearchConfigUpdate::Mode(mode))),
            App(new_dispatch(LocalSearchConfigUpdate::Search(
                search.to_string(),
            ))),
            App(new_dispatch(LocalSearchConfigUpdate::Replacement(
                replacement.to_string(),
            ))),
            App(Dispatch::Replace {
                scope: Scope::Global,
            }),
            Expect(FileContent(s.main_rs(), main_replaced.to_string())),
            Expect(FileContent(s.foo_rs(), foo_replaced.to_string())),
            // Expect the main.rs buffer to be updated as well
            ExpectLater(Box::new(move || {
                FileContent(main_rs.clone(), main_rs.read().unwrap())
            })),
            // Apply undo to main_rs
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(Undo),
            // Expect the content of the main.rs buffer to be reverted
            Expect(FileContent(s.main_rs(), main_content.to_string())),
        ])
    })
}

#[test]
fn test_global_repeat_search() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.foo_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("bye world".to_string())),
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("bye world".to_string())),
            App(SaveAll),
            // Local search "world"
            Editor(MatchLiteral("world".to_string())),
            // Global search "bye"
            App(UpdateLocalSearchConfig {
                update: LocalSearchConfigUpdate::Search("bye".to_string()),
                scope: Scope::Global,
                if_current_not_found: IfCurrentNotFound::LookForward,
                run_search_after_config_updated: true,
            }),
            WaitForAppMessage(regex!("AddQuickfixListEntries")),
            Expect(CurrentSelectedTexts(&["bye"])),
            // Change the selection mode
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                SelectionMode::Line,
            )),
            Expect(CurrentSelectedTexts(&["bye world"])),
            Editor(RepeatSearch(
                Scope::Global,
                IfCurrentNotFound::LookForward,
                None,
            )),
            WaitForAppMessage(regex!("AddQuickfixListEntries")),
            Expect(CurrentSelectedTexts(&["bye"])),
        ])
    })
}
struct TestGlobalSearchReplaceArgs {
    mode: LocalSearchConfigMode,
    main_content: &'static str,
    foo_content: &'static str,
    search: &'static str,
    replacement: &'static str,
    main_replaced: &'static str,
    foo_replaced: &'static str,
}

#[test]
fn global_search_replace_regex() -> Result<(), anyhow::Error> {
    test_global_search_replace(TestGlobalSearchReplaceArgs {
        mode: LocalSearchConfigMode::Regex(RegexConfig {
            escaped: true,
            case_sensitive: false,
            match_whole_word: false,
        }),
        main_content: "main foo",
        foo_content: "foo foo",
        search: "foo",
        replacement: "haha",
        main_replaced: "main haha",
        foo_replaced: "haha haha",
    })
}

#[test]
fn global_search_replace_ast_grep() -> Result<(), anyhow::Error> {
    test_global_search_replace(TestGlobalSearchReplaceArgs {
        mode: LocalSearchConfigMode::AstGrep,
        main_content: "fn main() {\n    let x = a.b.foo();\n}\n",
        foo_content: "fn main() { let x = (1+1).foo(); }",
        search: "$X.foo()",
        replacement: "foo($X)",
        // Note: the replaced content has newline characters because after replacement,
        // the formatter will be applied
        main_replaced: "fn main() {\n    let x = foo(a.b);\n}\n",
        foo_replaced: "fn main() {\n    let x = foo((1 + 1));\n}\n",
    })
}

#[test]
fn global_search_replace_naming_convention_agnostic() -> Result<(), anyhow::Error> {
    test_global_search_replace(TestGlobalSearchReplaceArgs {
        mode: LocalSearchConfigMode::NamingConventionAgnostic,
        main_content: "HelloWorld, this is good",
        foo_content: "im-lisp (hello-world and say 'HELLO_WORLD')",
        search: "hello world",
        replacement: "bye sky",
        main_replaced: "ByeSky, this is good",
        foo_replaced: "im-lisp (bye-sky and say 'BYE_SKY')",
    })
}

#[test]
fn quickfix_list_basic() -> Result<(), anyhow::Error> {
    execute_test(|s| {
        let new_dispatch = |update: LocalSearchConfigUpdate| -> Dispatch {
            UpdateLocalSearchConfig {
                update,
                scope: Scope::Global,
                if_current_not_found: IfCurrentNotFound::LookForward,
                run_search_after_config_updated: true,
            }
        };
        Box::new([
            App(OpenFile { path: s.foo_rs(), owner: BufferOwner::User, focus: true }),
            Editor(SetContent(
                "
hello
foo balatuga // Line 2 (this line is purposely made longer than Line 10 to test sorting)







foo a // Line 10
"
                .trim()
                .to_string(),
            )),
            App(OpenFile { path: s.main_rs(), owner: BufferOwner::User, focus: true }),
            Editor(SetContent("foo d\nfoo c".to_string())),
            App(SaveAll),
            App(new_dispatch(LocalSearchConfigUpdate::Search(
                "foo".to_string(),
            ))),
            WaitForAppMessage(regex!("AddQuickfixListEntries")),
            Expect(QuickfixListContent(
                // Line 10 should be placed below Line 2 (sorted numerically, not lexicograhically)
                "
src/foo.rs
    2:1  foo balatuga // Line 2 (this line is purposely made longer than Line 10 to test sorting)
    10:1  foo a // Line 10

src/main.rs
    1:1  foo d
    2:1  foo c
               ".to_string()
                .trim()
                .to_string(),
            )),
            Expect(QuickfixListCurrentLine("    2:1  foo balatuga // Line 2 (this line is purposely made longer than Line 10 to test sorting)")),
            Expect(CurrentPath(s.foo_rs())),
            Expect(CurrentLine("foo balatuga // Line 2 (this line is purposely made longer than Line 10 to test sorting)")),
            Expect(CurrentSelectedTexts(&["foo"])),
            Expect(ComponentCount(2)),
            Editor(MoveSelection(Right)),
            Expect(ComponentCount(2)),
            Expect(QuickfixListCurrentLine("    10:1  foo a // Line 10")),
            Expect(CurrentLine("foo a // Line 10")),
            Expect(CurrentSelectedTexts(&["foo"])),
            Editor(MoveSelection(Right)),
            Expect(CurrentLine("foo d")),
            Expect(CurrentSelectedTexts(&["foo"])),
            Editor(MoveSelection(Right)),
            Expect(CurrentLine("foo c")),
            Expect(CurrentSelectedTexts(&["foo"])),
            Editor(MoveSelection(Left)),
            Expect(CurrentLine("foo d")),
            Expect(CurrentSelectedTexts(&["foo"])),
            Editor(MoveSelection(Left)),
            Expect(CurrentLine("foo a // Line 10")),
            Expect(CurrentSelectedTexts(&["foo"])),
            Editor(MoveSelection(Left)),
            Expect(CurrentLine("foo balatuga // Line 2 (this line is purposely made longer than Line 10 to test sorting)")),
            Expect(CurrentSelectedTexts(&["foo"])),
        ])
    })
}

#[test]
fn quickfix_list_show_info_if_possible() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(
                "
fn main() { 
  let x = 123 
}
"
                .trim()
                .to_string(),
            )),
            App(SetQuickfixList(
                crate::quickfix_list::QuickfixListType::Items(
                    [QuickfixListItem::new(
                        Location {
                            path: s.main_rs(),
                            range: (CharIndex(2)..CharIndex(5)).into(),
                        },
                        Some(Info::new(
                            "Hello world".to_string(),
                            "This is fine".to_string(),
                        )),
                        None,
                    )]
                    .to_vec(),
                ),
            )),
            App(SetGlobalMode(Some(GlobalMode::QuickfixListItem))),
            Expect(ExpectKind::GlobalInfo("This is fine")),
            App(OpenFile {
                path: s.foo_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Expect(CurrentComponentPath(Some(s.foo_rs()))),
            App(UseLastNonContiguousSelectionMode(
                IfCurrentNotFound::LookForward,
            )),
            Expect(CurrentComponentPath(Some(s.main_rs()))),
        ])
    })
}

#[serial] // This test has to be run in serial otherwise it will fail
#[test]
fn quickfix_list_header_should_be_highlighted_as_keyword() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(SetQuickfixList(
                crate::quickfix_list::QuickfixListType::Items(
                    [QuickfixListItem::new(
                        Location {
                            path: s.main_rs(),
                            range: (CharIndex(2)..CharIndex(5)).into(),
                        },
                        None,
                        None,
                    )]
                    .to_vec(),
                ),
            )),
            App(OtherWindow),
            Expect(CurrentComponentContent(
                "
src/main.rs
    1:3  mod foo;
"
                .trim(),
            )),
            // We wait for 2 SyntaxHighlightResponse here because one is for main.rs,
            // and the other one is for the quickfix list
            WaitForAppMessage(regex!("SyntaxHighlightResponse")),
            WaitForAppMessage(regex!("SyntaxHighlightResponse")),
            App(TerminalDimensionChanged(Dimension {
                height: 20,
                width: 50,
            })),
            // Expect "src/main.rs" is highlighted with "keyword"
            Expect(RangeStyleKey(
                "src/main.rs",
                Some(StyleKey::Syntax(
                    IndexedHighlightGroup::from_str("keyword").unwrap(),
                )),
            )),
        ])
    })
}

#[test]
fn diagnostic_info() -> Result<(), anyhow::Error> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.foo_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
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
            Expect(ComponentsOrder([ComponentKind::SuggestiveEditor].to_vec())),
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                Diagnostic(DiagnosticSeverityRange::All),
            )),
            Expect(ComponentsOrder(
                [ComponentKind::SuggestiveEditor, ComponentKind::GlobalInfo].to_vec(),
            )),
            Expect(GlobalInfoContents(&["Hello world"])),
            App(HandleKeyEvent(key!("esc"))),
            Expect(ComponentsOrder([ComponentKind::SuggestiveEditor].to_vec())),
            App(Dispatch::HandleLspNotification(
                LspNotification::PublishDiagnostics(lsp_types::PublishDiagnosticsParams {
                    uri: Url::from_file_path(s.foo_rs()).unwrap(),
                    // No diagnostic
                    diagnostics: Default::default(),
                    version: None,
                }),
            )),
            Editor(MoveSelection(Right)),
            // Expect no global info is shown, since there is no diagnostic
            Expect(ComponentsOrder([ComponentKind::SuggestiveEditor].to_vec())),
        ])
    })
}

#[test]
fn diagnostic_severity_decoration_precedence() -> Result<(), anyhow::Error> {
    use lsp_types::DiagnosticSeverity as S;
    let diagnostics = [(1, 2, S::ERROR), (0, 3, S::HINT)];
    let expect = |column: usize, underline_color: Color| {
        GridCellLine(
            // The columns of the following assertions are added by 2,
            // because of line number and the separator between the line number and the
            // content.
            1,
            column + 2,
            underline_color,
        )
    };
    execute_test(|s| {
        let diagnostic =
            |column_start: u32, column_end: u32, severity: lsp_types::DiagnosticSeverity| {
                lsp_types::Diagnostic {
                    range: lsp_types::Range::new(
                        lsp_types::Position::new(0, column_start),
                        lsp_types::Position::new(0, column_end),
                    ),
                    severity: Some(severity),
                    ..Default::default()
                }
            };
        let hint_color = hex!("#abcdef");
        let error_color = hex!("#fedbac");
        let theme = {
            let mut theme = Theme::default();
            theme.diagnostic.hint = Style::default().undercurl(hint_color);
            theme.diagnostic.error = Style::default().undercurl(error_color);
            theme
        };
        Box::new([
            App(OpenFile {
                path: s.foo_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            App(SetTheme(theme.clone())),
            Editor(SetContent(
                "who lives in a pineapple? spongebob squarepants".to_string(),
            )),
            App(TerminalDimensionChanged(Dimension {
                height: 3,
                width: 80,
            })),
            App(Dispatch::HandleLspNotification(
                LspNotification::PublishDiagnostics(lsp_types::PublishDiagnosticsParams {
                    uri: Url::from_file_path(s.foo_rs()).unwrap(),
                    diagnostics: diagnostics
                        .into_iter()
                        .map(|(start, end, severity)| diagnostic(start, end, severity))
                        .collect_vec(),
                    version: None,
                }),
            )),
            ExpectMulti(
                (0..1)
                    .map(|column| expect(column, hint_color))
                    .collect_vec(),
            ),
            ExpectMulti(
                (1..2)
                    .map(|column| expect(column, error_color))
                    .collect_vec(),
            ),
            ExpectMulti(
                (2..3)
                    .map(|column| expect(column, hint_color))
                    .collect_vec(),
            ),
        ])
    })
}

#[test]
fn same_range_diagnostics_should_be_merged() -> Result<(), anyhow::Error> {
    execute_test(|s| {
        let diagnostic = |info: &str| {
            lsp_types::Diagnostic::new_simple(
                lsp_types::Range::new(
                    lsp_types::Position::new(0, 1),
                    lsp_types::Position::new(0, 2),
                ),
                info.to_string(),
            )
        };
        let expected_info = &["foo\n=======\nbar\n=======\nspam"];
        Box::new([
            App(OpenFile {
                path: s.foo_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            App(Dispatch::HandleLspNotification(
                LspNotification::PublishDiagnostics(lsp_types::PublishDiagnosticsParams {
                    uri: Url::from_file_path(s.foo_rs()).unwrap(),
                    diagnostics: [diagnostic("foo"), diagnostic("bar"), diagnostic("spam")]
                        .to_vec(),
                    version: None,
                }),
            )),
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                Diagnostic(DiagnosticSeverityRange::All),
            )),
            Expect(GlobalInfoContents(expected_info)),
            // Expect there's only one diagnostic, by the fact that moving to the first and
            // last diagnostic still renders the same info
            Editor(MoveSelection(First)),
            Expect(GlobalInfoContents(expected_info)),
            Editor(MoveSelection(Last)),
            Expect(GlobalInfoContents(expected_info)),
        ])
    })
}

#[test]
fn code_action() -> anyhow::Result<()> {
    execute_test(|s| {
        let code_action = |new_text: &str| CodeAction {
            title: format!("Use {new_text}"),
            kind: None,
            edit: Some(WorkspaceEdit {
                edits: [TextDocumentEdit {
                    path: s.main_rs(),
                    edits: [PositionalEdit {
                        range: Position::new(0, 2)..Position::new(0, 6),
                        new_text: new_text.to_string(),
                    }]
                    .to_vec(),
                }]
                .to_vec(),
                resource_operations: Vec::new(),
            }),
            command: None,
        };
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("a.to_s".to_string())),
            App(ReceiveCodeActions(
                [code_action("to_soup"), code_action("to_string")].to_vec(),
            )),
            App(HandleKeyEvents(keys!("i n g enter").to_vec())),
            Expect(CurrentComponentContent("a.to_string")),
        ])
    })
}

#[test]
fn opening_new_file_should_replace_current_window() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Expect(ExpectKind::ComponentCount(1)),
            App(OpenFile {
                path: s.foo_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Expect(ExpectKind::ComponentCount(1)),
        ])
    })
}

#[test]
fn should_be_able_to_handle_key_event_even_when_no_file_is_opened() -> anyhow::Result<()> {
    execute_test(|_| {
        Box::new([
            Expect(CurrentComponentContent("")),
            App(HandleKeyEvents(keys!("h h e l l o").to_vec())),
            Expect(CurrentComponentContent("hello")),
        ])
    })
}

#[test]
fn cycle_window() -> anyhow::Result<()> {
    {
        let completion_item = |label: &str, documentation: Option<&str>| CompletionItem {
            label: label.to_string(),
            edit: Some(CompletionItemEdit::PositionalEdit(PositionalEdit {
                range: Position::new(0, 0)..Position::new(0, 6),
                new_text: label.to_string(),
            })),
            documentation: documentation.map(Documentation::new),
            sort_text: None,
            kind: None,
            detail: None,
            insert_text: None,
            completion_item: Default::default(),
        };

        execute_test(|s| {
            let completion = Completion {
                trigger_characters: vec![".".to_string()],
                items: [
                    completion_item("Patrick", Some("hacker")),
                    completion_item("Spongebob squarepants", Some("krabby patty maker")),
                ]
                .into_iter()
                .map(|item| item.into())
                .collect(),
            };
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("".to_string())),
                Editor(EnterInsertMode(Direction::Start)),
                SuggestiveEditor(DispatchSuggestiveEditor::CompletionFilter(
                    SuggestiveEditorFilter::CurrentWord,
                )),
                Editor(EnterInsertMode(Direction::End)),
                SuggestiveEditor(DispatchSuggestiveEditor::Completion(completion.clone())),
                Expect(ComponentCount(3)),
                // Move to the next completion item (which is 'Spongebob squarepants')
                App(HandleKeyEvent(key!("alt+k"))),
                Expect(CurrentComponentContent("")),
                App(OtherWindow),
                Expect(ComponentCount(3)),
                Expect(CurrentComponentContent(" Patrick\n Spongebob squarepants")),
                Expect(CurrentSelectedTexts(&[" Spongebob squarepants"])),
                App(OtherWindow),
                Expect(CurrentComponentContent("krabby patty maker")),
                App(OtherWindow),
                Expect(CurrentComponentContent("")),
                App(OtherWindow),
                App(CloseCurrentWindow),
                Expect(CurrentComponentContent("")),
            ])
        })
    }
}

#[test]
fn esc_in_normal_mode_in_suggestive_editor_should_close_all_other_windows() -> anyhow::Result<()> {
    {
        let completion_item = |label: &str, documentation: Option<&str>| CompletionItem {
            label: label.to_string(),
            edit: Some(CompletionItemEdit::PositionalEdit(PositionalEdit {
                range: Position::new(0, 0)..Position::new(0, 6),
                new_text: label.to_string(),
            })),
            documentation: documentation.map(Documentation::new),
            sort_text: None,
            kind: None,
            detail: None,
            insert_text: None,
            completion_item: Default::default(),
        };
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("".to_string())),
                Editor(EnterInsertMode(Direction::Start)),
                SuggestiveEditor(DispatchSuggestiveEditor::CompletionFilter(
                    SuggestiveEditorFilter::CurrentWord,
                )),
                // Pretend that the LSP server returned a completion
                SuggestiveEditor(DispatchSuggestiveEditor::Completion(Completion {
                    trigger_characters: vec![".".to_string()],
                    items: Some(completion_item(
                        "Spongebob squarepants",
                        Some("krabby patty maker"),
                    ))
                    .into_iter()
                    .map(|item| item.into())
                    .collect(),
                })),
                Expect(ComponentCount(3)),
                App(HandleKeyEvent(key!("esc"))),
                Expect(ComponentCount(1)),
                Expect(CurrentComponentPath(Some(s.main_rs()))),
            ])
        })
    }
}

#[test]
fn saving_in_insert_mode_in_suggestive_editor_should_close_all_other_windows() -> anyhow::Result<()>
{
    {
        let completion_item = |label: &str, documentation: Option<&str>| CompletionItem {
            label: label.to_string(),
            edit: Some(CompletionItemEdit::PositionalEdit(PositionalEdit {
                range: Position::new(0, 0)..Position::new(0, 6),
                new_text: label.to_string(),
            })),
            documentation: documentation.map(Documentation::new),
            sort_text: None,
            kind: None,
            detail: None,
            insert_text: None,
            completion_item: Default::default(),
        };
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("".to_string())),
                Editor(EnterInsertMode(Direction::Start)),
                SuggestiveEditor(DispatchSuggestiveEditor::CompletionFilter(
                    SuggestiveEditorFilter::CurrentWord,
                )),
                // Pretend that the LSP server returned a completion
                SuggestiveEditor(DispatchSuggestiveEditor::Completion(Completion {
                    trigger_characters: vec![".".to_string()],
                    items: Some(completion_item(
                        "Spongebob squarepants",
                        Some("krabby patty maker"),
                    ))
                    .into_iter()
                    .map(|item| item.into())
                    .collect(),
                })),
                Expect(ComponentCount(3)),
                Editor(EnterInsertMode(Direction::Start)),
                Editor(Insert("hello".to_string())),
                Editor(Save),
                Expect(ComponentCount(1)),
                Expect(CurrentComponentPath(Some(s.main_rs()))),
            ])
        })
    }
}

#[test]
fn closing_current_file_should_replace_current_window_with_another_file() -> anyhow::Result<()> {
    {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                App(OpenFile {
                    path: s.foo_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Expect(CurrentComponentPath(Some(s.foo_rs()))),
                App(CloseCurrentWindow),
                Expect(CurrentComponentPath(Some(s.main_rs()))),
                Expect(OpenedFilesCount(1)),
                App(CloseCurrentWindow),
                Expect(OpenedFilesCount(0)),
                Expect(CurrentComponentPath(None)),
            ])
        })
    }
}

#[test]
fn editor_info_should_always_come_after_dropdown() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("".to_string())),
            Editor(EnterInsertMode(Direction::Start)),
            SuggestiveEditor(DispatchSuggestiveEditor::CompletionFilter(
                SuggestiveEditorFilter::CurrentWord,
            )),
            // Show editor info first
            App(ShowEditorInfo(Info::new(
                "Title".to_string(),
                "hello".to_string(),
            ))),
            // The show dropdown
            SuggestiveEditor(DispatchSuggestiveEditor::Completion(Completion {
                trigger_characters: vec![".".to_string()],
                items: Some(CompletionItem::from_label(
                    "Spongebob squarepants".to_string(),
                ))
                .into_iter()
                .map(|item| item.into())
                .collect(),
            })),
            Expect(ComponentCount(3)),
            // But dropdown still comes before editor info
            Expect(ExpectKind::ComponentsOrder(vec![
                ComponentKind::SuggestiveEditor,
                ComponentKind::Dropdown,
                ComponentKind::EditorInfo,
            ])),
        ])
    })
}

#[test]
fn dropdown_can_only_be_rendered_on_suggestive_editor_or_prompt() -> anyhow::Result<()> {
    execute_test(|s| {
        let received_completion = || {
            SuggestiveEditor(DispatchSuggestiveEditor::Completion(Completion {
                trigger_characters: vec![".".to_string()],
                items: Some(CompletionItem::from_label(
                    "Spongebob squarepants".to_string(),
                ))
                .into_iter()
                .map(|item| item.into())
                .collect(),
            }))
        };
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("hello".to_string())),
            Editor(EnterInsertMode(Direction::Start)),
            SuggestiveEditor(DispatchSuggestiveEditor::CompletionFilter(
                SuggestiveEditorFilter::CurrentWord,
            )),
            // The show dropdown
            received_completion(),
            Expect(ComponentCount(2)),
            Expect(CurrentComponentContent("hello")),
            App(OtherWindow),
            Expect(CurrentComponentContent(" Spongebob squarepants")),
            received_completion(),
            Expect(ComponentCount(2)),
        ])
    })
}

#[test]
fn only_children_of_root_can_remove_all_other_components() -> anyhow::Result<()> {
    execute_test(|s| {
        let received_completion = || {
            SuggestiveEditor(DispatchSuggestiveEditor::Completion(Completion {
                trigger_characters: vec![".".to_string()],
                items: Some(CompletionItem::from_label(
                    "Spongebob squarepants".to_string(),
                ))
                .into_iter()
                .map(|item| item.into())
                .collect(),
            }))
        };
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("hello".to_string())),
            Editor(EnterInsertMode(Direction::Start)),
            SuggestiveEditor(DispatchSuggestiveEditor::CompletionFilter(
                SuggestiveEditorFilter::CurrentWord,
            )),
            // The show dropdown
            received_completion(),
            Expect(ComponentCount(2)),
            Expect(CurrentComponentContent("hello")),
            App(OtherWindow),
            Expect(CurrentComponentContent(" Spongebob squarepants")),
            App(RemainOnlyCurrentComponent),
            Expect(ComponentCount(2)),
        ])
    })
}

#[test]
fn preserve_selection_after_file_changes() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("hello world bar".to_string())),
            Editor(MatchLiteral("world".to_string())),
            Expect(CurrentSelectedTexts(&["world"])),
            App(OpenFile {
                path: s.foo_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Expect(CurrentSelectedTexts(&["world"])),
        ])
    })
}

#[test]
fn open_search_prompt_in_file_explorer() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            App(RevealInExplorer(s.main_rs())),
            Expect(CurrentComponentTitle("File Explorer".to_string())),
            App(OpenSearchPrompt {
                scope: Scope::Local,
                if_current_not_found: IfCurrentNotFound::LookForward,
            }),
            Expect(Not(Box::new(CurrentComponentTitle(
                "File Explorer".to_string(),
            )))),
            App(HandleKeyEvents(keys!("m a i n enter").to_vec())),
            Expect(CurrentComponentTitle("File Explorer".to_string())),
        ])
    })
}

#[test]
fn global_search_should_not_using_empty_pattern() -> anyhow::Result<()> {
    execute_test(|_| {
        Box::new([
            App(UpdateLocalSearchConfig {
                update: LocalSearchConfigUpdate::Search("".to_string()),
                scope: Scope::Global,
                if_current_not_found: IfCurrentNotFound::LookForward,
                run_search_after_config_updated: true,
            }),
            Expect(ExpectKind::Quickfixes(Box::new([]))),
        ])
    })
}

#[test]
fn workspace_edit() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("who lives in a pineapple".to_string())),
            Editor(MatchLiteral("pineapple".to_string())),
            Expect(CurrentSelectedTexts(&["pineapple"])),
            App(Dispatch::ApplyWorkspaceEdit(WorkspaceEdit {
                edits: [TextDocumentEdit {
                    path: s.main_rs(),
                    edits: [PositionalEdit {
                        range: Position::new(0, 0)..Position::new(0, 0),
                        new_text: "hello ".to_string(),
                    }]
                    .to_vec(),
                }]
                .to_vec(),
                resource_operations: Vec::new(),
            })),
            Expect(CurrentComponentContent("hello who lives in a pineapple")),
            // Expect the selection is still "pineapple"
            Expect(CurrentSelectedTexts(&["pineapple"])),
        ])
    })
}

#[test]
fn request_signature_help() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("f()".to_string())),
            Editor(MatchLiteral("f()".to_string())),
            Editor(EnterInsertMode(Direction::End)),
            Expect(ExpectKind::LspRequestSent(
                FromEditor::TextDocumentSignatureHelp(RequestParams {
                    path: s.main_rs(),
                    position: Position::new(0, 3),
                    context: Default::default(),
                }),
            )),
            App(HandleKeyEvent(key!("left"))),
            Expect(ExpectKind::LspRequestSent(
                FromEditor::TextDocumentSignatureHelp(RequestParams {
                    path: s.main_rs(),
                    position: Position::new(0, 2),
                    context: Default::default(),
                }),
            )),
        ])
    })
}

#[test]
/// These JSON files will be used in docs
fn doc_assets_export_keymaps_json() {
    #[derive(Serialize, Clone)]
    struct KeyboardLayoutJson {
        name: String,
        keys: Vec<[&'static str; 10]>,
    }

    #[derive(Serialize)]
    struct KeyJson {
        normal: Option<String>,
        alted: Option<String>,
        shifted: Option<String>,
    }

    #[derive(Serialize)]
    struct RowsJson(Vec<KeyJson>);

    #[derive(Serialize)]
    struct KeymapSectionJson {
        name: String,
        rows: Vec<RowsJson>,
        keyboard_layouts: Vec<KeyboardLayoutJson>,
    }

    let get_path = |name: &str| format!("docs/static/keymaps/{name}.json");
    let keyboard_layouts = KeyboardLayoutKind::iter()
        .map(|keyboard_layout| KeyboardLayoutJson {
            name: keyboard_layout.display().to_string(),
            keys: keyboard_layout.get_keyboard_layout().to_vec(),
        })
        .collect_vec();
    let sections = KeymapPrintSections::new()
        .sections()
        .iter()
        .map(|section| {
            let name = section.name().to_string();
            let rows = section
                .keys()
                .iter()
                .map(|keys| {
                    RowsJson(
                        keys.iter()
                            .map(|key| {
                                let normal = key.normal.as_ref().map(Keymap::display);
                                let alted = key.alted.as_ref().map(Keymap::display);
                                let shifted = key.shifted.as_ref().map(Keymap::display);

                                KeyJson {
                                    normal,
                                    alted,
                                    shifted,
                                }
                            })
                            .collect_vec(),
                    )
                })
                .collect_vec();
            KeymapSectionJson {
                rows,
                name,
                keyboard_layouts: keyboard_layouts.clone(),
            }
        })
        .collect_vec();

    sections.into_iter().for_each(|section| {
        let path = get_path(&section.name);
        let json = serde_json::to_string(&section).unwrap();
        std::fs::write(path, json).unwrap()
    });
}

#[serial]
#[test]
fn multi_paste_2() -> Result<(), anyhow::Error> {
    execute_test(|s| {
        {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent(
                    "
a1 a2 a3
b1 b2 b3
c1 c2 c3"
                        .trim()
                        .to_string(),
                )),
                Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Line)),
                Editor(CursorAddToAllSelections),
                Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
                Expect(CurrentSelectedTexts(&["a1", "b1", "c1"])),
                Editor(Copy),
                Editor(MoveSelection(Right)),
                Editor(MoveSelection(Right)),
                Expect(CurrentSelectedTexts(&["a3", "b3", "c3"])),
                Editor(Paste),
                Expect(CurrentSelectedTexts(&["a1", "b1", "c1"])),
                Expect(CurrentComponentContent(
                    "
a1 a2 a3 a1
b1 b2 b3 b1
c1 c2 c3 c1
"
                    .trim(),
                )),
            ])
        }
    })
}

#[test]
#[serial]
fn pasting_when_clipboard_html_is_set_by_other_app() -> Result<(), anyhow::Error> {
    execute_test(|s| {
        {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                App(Dispatch::SetSystemClipboardHtml {
                    html: "<div source=\"from Microsoft Word\">hello</div>",
                    alt_text: "hello",
                }),
                Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Character)),
                Editor(SetContent("".to_string())),
                Editor(Paste),
                Expect(CurrentComponentContent("hello")),
            ])
        }
    })
}

#[serial]
#[test]
fn multi_replace() -> Result<(), anyhow::Error> {
    execute_test(|s| {
        {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent(
                    "
a1 a2 a3
b1 b2 b3
c1 c2 c3"
                        .trim()
                        .to_string(),
                )),
                Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Line)),
                Editor(CursorAddToAllSelections),
                Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
                Expect(CurrentSelectedTexts(&["a1", "b1", "c1"])),
                Editor(Copy),
                Editor(MoveSelection(Right)),
                Editor(MoveSelection(Right)),
                Expect(CurrentSelectedTexts(&["a3", "b3", "c3"])),
                Editor(ReplaceWithCopiedText { cut: false }),
                Expect(CurrentSelectedTexts(&["a1", "b1", "c1"])),
                Expect(CurrentComponentContent(
                    "
a1 a2 a1
b1 b2 b1
c1 c2 c1
"
                    .trim(),
                )),
            ])
        }
    })
}

#[test]
fn test_navigate_back_from_open_file() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            App(OpenFile {
                path: s.foo_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Expect(CurrentComponentPath(Some(s.foo_rs()))),
            App(NavigateBack),
            Expect(CurrentComponentPath(Some(s.main_rs()))),
            App(NavigateForward),
            Expect(CurrentComponentPath(Some(s.foo_rs()))),
            App(NavigateBack),
            Expect(CurrentComponentPath(Some(s.main_rs()))),
            App(OpenFile {
                path: s.gitignore(),
                owner: BufferOwner::User,
                focus: true,
            }),
            App(NavigateBack),
            Expect(CurrentComponentPath(Some(s.main_rs()))),
        ])
    })
}

#[test]
fn test_navigate_back_from_go_to_location() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(GotoLocation(Location {
                path: s.main_rs(),
                range: Default::default(),
            })),
            App(GotoLocation(Location {
                path: s.foo_rs(),
                range: Default::default(),
            })),
            App(GotoLocation(Location {
                path: s.gitignore(),
                range: Default::default(),
            })),
            Expect(CurrentComponentPath(Some(s.gitignore()))),
            App(NavigateBack),
            Expect(CurrentComponentPath(Some(s.foo_rs()))),
            App(NavigateBack),
            Expect(CurrentComponentPath(Some(s.main_rs()))),
        ])
    })
}

#[test]
fn test_navigate_back_from_quickfix_list() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            App(HandleLspNotification(LspNotification::Definition(
                Default::default(),
                GotoDefinitionResponse::Multiple(
                    [
                        Location {
                            path: s.foo_rs(),
                            range: (CharIndex(0)..CharIndex(1)).into(),
                        },
                        Location {
                            path: s.foo_rs(),
                            range: (CharIndex(0)..CharIndex(1)).into(),
                        },
                    ]
                    .to_vec(),
                ),
            ))),
            Expect(CurrentComponentPath(Some(s.foo_rs()))),
            App(NavigateBack),
            Expect(CurrentComponentPath(Some(s.main_rs()))),
        ])
    })
}

#[test]
fn mark_files_tabline_wrapping_no_word_break() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            App(ToggleFileMark),
            App(OpenFile {
                path: s.foo_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetRectangle(Rectangle {
                origin: Position::default(),
                width: 20,
                height: 3,
            })),
            Expect(EditorGrid("🦀  foo.rs\n# 🦀  main.rs\n1│█ub(crate) struct")),
        ])
    })
}

#[test]
fn mark_files_tabline_wrapping_with_word_break() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            App(ToggleFileMark),
            App(OpenFile {
                path: s.gitignore(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetRectangle(Rectangle {
                origin: Position::default(),
                width: 11,
                height: 5,
            })),
            Expect(EditorGrid(
                "
🙈  .gitig
nore
# 🦀  main
.rs
1│█arget/
"
                .trim(),
            )),
        ])
    })
}

#[test]
fn unmark_file_still_focus_current_file_if_no_more_marked_files() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            App(ToggleFileMark),
            App(ToggleFileMark),
            Expect(CurrentComponentPath(Some(s.main_rs))),
        ])
    })
}

#[test]
/// The file to be unmarked is the lowest-rank marked file
/// Expect the focused file to be the second file
/// File rank: .gitignore -> foo.rs -> main.rs
fn unmark_file_focuses_next_marked_file_if_other_marked_files_exists() -> anyhow::Result<()> {
    // Latin gerundive-inspired variable naming convention used below:
    // The '-andum' suffix comes from Latin gerundives, indicating "that which is to be [verb]ed"
    // Examples: memorandum (to be remembered), referendum (to be referred), agenda (to be done)
    // In our code, 'unmarkandum_file' means "file to be unmarked",
    // 'focusandum_file' means "file to be focused"

    #[derive(Clone, Copy)]
    enum TestFile {
        First,
        Middle,
        Last,
    }
    let run = |unmarkandum_file: TestFile, focusandum_file: TestFile| {
        execute_test(|s| {
            let to_path = |test_file: TestFile| match test_file {
                TestFile::First => s.gitignore(),
                TestFile::Middle => s.foo_rs(),
                TestFile::Last => s.main_rs(),
            };
            Box::new([
                App(OpenFile {
                    path: s.gitignore(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                App(ToggleFileMark),
                App(OpenFile {
                    path: s.foo_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                App(ToggleFileMark),
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                App(ToggleFileMark),
                App(OpenFile {
                    path: to_path(unmarkandum_file),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                App(ToggleFileMark),
                Expect(CurrentComponentPath(Some(to_path(focusandum_file)))),
            ])
        })
    };
    // Test with different combinations of files to be unmarked and subsequently focused
    run(TestFile::First, TestFile::Middle)?;
    run(TestFile::Middle, TestFile::Last)?;
    run(TestFile::Last, TestFile::Middle)?;
    Ok(())
}

#[test]
fn close_buffer_should_remove_mark() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            App(ToggleFileMark),
            App(OpenFile {
                path: s.foo_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            App(ToggleFileMark),
            App(CloseCurrentWindow),
            App(CycleMarkedFile(Direction::End)),
            Expect(CurrentComponentPath(Some(s.main_rs()))),
        ])
    })
}

#[test]
fn using_suggested_search_term() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("foo bar spam".to_string())),
            App(OpenSearchPrompt {
                scope: Scope::Local,
                if_current_not_found: IfCurrentNotFound::LookForward,
            }),
            Expect(CompletionDropdownContent("bar\nfoo\nspam")),
            App(HandleKeyEvents(keys!("f o alt+l").to_vec())),
            Expect(CurrentComponentContent("foo")),
        ])
    })
}

#[test]
fn cursor_line_number_style_handle_text_wrapping() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            App(TerminalDimensionChanged(Dimension {
                height: 4,
                width: 20,
            })),
            Editor(SetContent("foo bar spongebob spam".to_string())),
            Editor(MatchLiteral("spam".to_string())),
            Expect(AppGrid(
                " 🦀  main.rs [*]
1│foo bar spongebob
↪│ █pam"
                    .to_string(),
            )),
            // Expect the highlighted cursor line number is not `1`, but `↪`
            Expect(GridCellStyleKey(
                Position::new(2, 0),
                Some(StyleKey::UiCursorLineNumber),
            )),
            Expect(Not(Box::new(GridCellStyleKey(
                Position::new(1, 0),
                Some(StyleKey::UiCursorLineNumber),
            )))),
        ])
    })
}

#[test]
fn cursor_line_number_style_only_focused_split() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(
                "
bar
XXX XXX
spam
"
                .trim()
                .to_string(),
            )),
            App(TerminalDimensionChanged(Dimension {
                height: 6,
                width: 20,
            })),
            Editor(MatchLiteral("XXX".to_string())),
            Editor(CursorAddToAllSelections),
            Expect(AppGrid(
                " 🦀  main.rs [*]
1│bar
2│█XX XXX
1│bar
2│XXX XXX"
                    .to_string(),
            )),
            Expect(GridCellStyleKey(
                Position::new(2, 0),
                Some(StyleKey::UiCursorLineNumber),
            )),
            // Expect the 5th line of the grid is not styled as UiCursorLineNumber
            // Although the selection is as the same line of the cursor
            Expect(Not(Box::new(GridCellStyleKey(
                Position::new(4, 0),
                Some(StyleKey::UiCursorLineNumber),
            )))),
        ])
    })
}

#[test]
fn cursor_line_number_style() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(
                "
bar
XXX XXX
spam
"
                .trim()
                .to_string(),
            )),
            App(TerminalDimensionChanged(Dimension {
                height: 6,
                width: 20,
            })),
            Editor(MatchLiteral("XXX".to_string())),
            Editor(CursorAddToAllSelections),
            Expect(AppGrid(
                " 🦀  main.rs [*]
1│bar
2│█XX XXX
1│bar
2│XXX XXX"
                    .to_string(),
            )),
            Expect(GridCellStyleKey(
                Position::new(2, 0),
                Some(StyleKey::UiCursorLineNumber),
            )),
            // Expect the 5th line of the grid is not styled as UiCursorLineNumber
            // Although the selection is as the same line of the cursor
            Expect(Not(Box::new(GridCellStyleKey(
                Position::new(4, 0),
                Some(StyleKey::UiCursorLineNumber),
            )))),
        ])
    })
}

#[test]
fn surround_with_xml_tag() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("hello".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
            App(OpenSurroundXmlPrompt),
            App(HandleKeyEvents(keys!("f o o enter").to_vec())),
            Expect(CurrentComponentContent("<foo>hello</foo>")),
            Expect(CurrentSelectedTexts(&["<foo>hello</foo>"])),
        ])
    })
}
#[test]
fn surround_with_empty_xml_tag() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("hello".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
            App(OpenSurroundXmlPrompt),
            App(HandleKeyEvents(keys!("enter").to_vec())),
            Expect(CurrentComponentContent("<>hello</>")),
            Expect(CurrentSelectedTexts(&["<>hello</>"])),
        ])
    })
}

#[test]
fn lsp_initialization_should_only_send_relevant_opened_documents() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            App(OpenFile {
                path: s.hello_ts(),
                owner: BufferOwner::User,
                focus: true,
            }),
            App(HandleLspNotification(LspNotification::Initialized(
                language::from_extension("ts").unwrap(),
            ))),
            Expect(LspServerInitializedArgs(Some((
                LanguageId::new("typescript"),
                // Expect only hello.ts is sent to the Typescript LSP server
                // although main.rs is opened before
                [s.hello_ts()].to_vec(),
            )))),
        ])
    })
}

#[test]
fn navigate_back_should_skip_files_that_were_renamed_or_deleted() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            App(OpenFile {
                path: s.hello_ts(),
                owner: BufferOwner::User,
                focus: true,
            }),
            App(DeletePath(s.main_rs())),
            App(NavigateBack),
            Expect(NoError),
        ])
    })
}

#[test]
fn navigate_forward_should_skip_files_that_were_renamed_or_deleted() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            App(OpenFile {
                path: s.hello_ts(),
                owner: BufferOwner::User,
                focus: true,
            }),
            App(NavigateBack),
            Expect(CurrentPath(s.main_rs())),
            App(DeletePath(s.hello_ts())),
            App(NavigateForward),
            Expect(NoError),
        ])
    })
}

#[test]
fn navigating_to_marked_file_that_is_deleted_should_not_cause_error() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            App(ToggleFileMark),
            App(OpenFile {
                path: s.hello_ts(),
                owner: BufferOwner::User,
                focus: true,
            }),
            App(ToggleFileMark),
            App(OpenFile {
                path: s.gitignore(),
                owner: BufferOwner::User,
                focus: true,
            }),
            App(ToggleFileMark),
            Expect(AppGrid(
                r#" # 🙈  .gitignore  # 📘  hello.ts  # 🦀  main.rs
1│█arget/
2│"#
                .to_string(),
            )),
            Expect(CurrentPath(s.gitignore())),
            App(DeletePath(s.main_rs())),
            App(CycleMarkedFile(Direction::Start)),
            Expect(NoError),
            Expect(CurrentPath(s.hello_ts())),
            // Expect main.rs is removed from the tab
            Expect(AppGrid(
                r#" # 🙈  .gitignore  # 📘  hello.ts
1│█onsole.log("hello");
2│"#
                .to_string(),
            )),
        ])
    })
}

#[test]
fn renaming_marked_files_should_update_file_marks() -> anyhow::Result<()> {
    execute_test(|s| {
        let new_path = s.temp_dir.to_path_buf().join("new_name");
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            App(ToggleFileMark),
            App(OpenFile {
                path: s.hello_ts(),
                owner: BufferOwner::User,
                focus: true,
            }),
            App(ToggleFileMark),
            App(OpenFile {
                path: s.gitignore(),
                owner: BufferOwner::User,
                focus: true,
            }),
            App(ToggleFileMark),
            Expect(AppGrid(
                r#" # 🙈  .gitignore  # 📘  hello.ts  # 🦀  main.rs
1│█arget/
2│"#
                .to_string(),
            )),
            Expect(CurrentPath(s.gitignore())),
            App(MoveFile {
                from: s.gitignore(),
                to: new_path.clone(),
            }),
            // Press enter to hide File Explorer and focus the renamed file
            App(HandleKeyEvent(key!("enter"))),
            Expect(AppGrid(
                r#" # 📄  new_name  # 📘  hello.ts  # 🦀  main.rs
1│█arget/
2│"#
                .to_string(),
            )),
        ])
    })
}

#[test]
fn escape_global_diagnostics_should_not_change_selection() -> Result<(), anyhow::Error> {
    execute_test(|s| {
        let diagnostic = |path: CanonicalizedPath| {
            Dispatch::HandleLspNotification(LspNotification::PublishDiagnostics(
                lsp_types::PublishDiagnosticsParams {
                    uri: Url::from_file_path(path).unwrap(),
                    diagnostics: [lsp_types::Diagnostic::new_simple(
                        lsp_types::Range::new(
                            lsp_types::Position::new(0, 0),
                            lsp_types::Position::new(0, 3),
                        ),
                        "".to_string(),
                    )]
                    .to_vec(),
                    version: None,
                },
            ))
        };
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                SelectionMode::Line,
            )),
            App(diagnostic(s.foo_rs())),
            App(diagnostic(s.main_rs())),
            App(Dispatch::SetQuickfixList(QuickfixListType::Diagnostic(
                DiagnosticSeverityRange::All,
            ))),
            Expect(CurrentComponentPath(Some(s.foo_rs()))),
            Expect(CurrentSelectedTexts(&["pub"])),
            Editor(MoveSelection(Right)),
            Expect(CurrentComponentPath(Some(s.main_rs()))),
            Expect(CurrentSelectedTexts(&["mod"])),
            App(HandleKeyEvent(key!("esc"))),
            Expect(CurrentComponentPath(Some(s.main_rs()))),
            Expect(CurrentSelectedTexts(&["mod"])),
        ])
    })
}
