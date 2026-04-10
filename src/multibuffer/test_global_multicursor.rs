use lazy_regex::regex;
use my_proc_macros::{key, keys};

use crate::{
    app::{
        Dimension,
        Dispatch::{self, *},
        Scope,
    },
    buffer::BufferOwner,
    components::editor::{Direction, DispatchEditor::*, IfCurrentNotFound, Mode},
    context::GlobalMode,
    grid::StyleKey,
    position::Position,
    selection::CharIndex,
    test_app::{
        execute_test,
        ExpectKind::{self, *},
        Step::*,
    },
    ui_tree::ComponentKind,
};

#[test]
fn render_should_show_all_quickfix_items_and_all_files() -> Result<(), anyhow::Error> {
    execute_test(|s| {
        Box::new([
            App(TerminalDimensionChanged(Dimension {
                width: 100,
                height: 8,
            })),
            App(SetFileContent(
                s.main_rs(),
                "// first foo\n\n\nsecond foo".to_string(),
            )),
            App(SetFileContent(s.foo_rs(), "// third foo".to_string())),
            App(OpenSearchPrompt {
                scope: Scope::Global,
                if_current_not_found: IfCurrentNotFound::LookForward,
            }),
            App(HandleKeyEvents(keys!("f o o enter").to_vec())),
            WaitForAppMessage(regex!("GlobalSearchFinished")),
            App(AddCursorToAllSelections),
            Expect(AppGrid(
                "
src/foo.rs
1│// third █oo

src/main.rs
1│// first foo
2│
4│second foo
"
                .trim_matches('\n')
                .to_string(),
            )),
        ])
    })
}

#[test]
fn rendered_filename_should_exclude_tabline() -> Result<(), anyhow::Error> {
    execute_test(|s| {
        Box::new([
            App(TerminalDimensionChanged(Dimension {
                width: 100,
                height: 8,
            })),
            App(OpenFile {
                path: s.gitignore(),
                owner: BufferOwner::User,
                focus: true,
            }),
            App(ToggleFileMark),
            App(SetFileContent(s.main_rs(), "// first foo".to_string())),
            App(SetFileContent(s.foo_rs(), "// second foo".to_string())),
            App(OpenSearchPrompt {
                scope: Scope::Global,
                if_current_not_found: IfCurrentNotFound::LookForward,
            }),
            App(HandleKeyEvents(keys!("f o o enter").to_vec())),
            WaitForAppMessage(regex!("GlobalSearchFinished")),
            App(AddCursorToAllSelections),
            Expect(AppGrid(
                "
src/foo.rs
1│// second █oo


src/main.rs
1│// first foo"
                    .trim_matches('\n')
                    .to_string(),
            )),
        ])
    })
}

#[test]
fn should_only_have_one_selection_styled_as_primary() -> Result<(), anyhow::Error> {
    execute_test(|s| {
        Box::new([
            App(TerminalDimensionChanged(Dimension {
                width: 100,
                height: 10,
            })),
            App(OpenFile {
                path: s.gitignore(),
                owner: BufferOwner::User,
                focus: true,
            }),
            App(ToggleFileMark),
            App(SetFileContent(s.foo_rs(), "hello\nfoo 1".to_string())),
            App(SetFileContent(s.main_rs(), "hello\nfoo 2".to_string())),
            App(OpenSearchPrompt {
                scope: Scope::Global,
                if_current_not_found: IfCurrentNotFound::LookForward,
            }),
            App(HandleKeyEvents(keys!("f o o enter").to_vec())),
            WaitForAppMessage(regex!("GlobalSearchFinished")),
            App(AddCursorToAllSelections),
            Expect(AppGrid(
                "
src/foo.rs
1│hello
2│█oo 1


src/main.rs
1│hello
2│foo 2"
                    .trim_matches('\n')
                    .to_string(),
            )),
            Expect(CurrentComponentPath(Some(s.foo_rs()))),
            // The selection of `foo` in `main.rs` should be rendered as secondary selection
            // because the current focused file is `foo.rs`
            Expect(AppGridCellStyleKey(
                Position::new(7, 3),
                Some(StyleKey::UiSecondarySelectionAnchors),
            )),
        ])
    })
}

#[test]
fn cycle_cursor_should_switch_file_focus() -> Result<(), anyhow::Error> {
    execute_test(|s| {
        Box::new([
            App(TerminalDimensionChanged(Dimension {
                width: 100,
                height: 10,
            })),
            App(OpenFile {
                path: s.gitignore(),
                owner: BufferOwner::User,
                focus: true,
            }),
            App(ToggleFileMark),
            App(SetFileContent(
                s.foo_rs(),
                "hello\nfoo 1/\nfoo 2".to_string(),
            )),
            App(SetFileContent(
                s.main_rs(),
                "hello\nfoo 3\nfoo4".to_string(),
            )),
            App(OpenSearchPrompt {
                scope: Scope::Global,
                if_current_not_found: IfCurrentNotFound::LookForward,
            }),
            App(HandleKeyEvents(keys!("f o o enter").to_vec())),
            WaitForAppMessage(regex!("GlobalSearchFinished")),
            App(AddCursorToAllSelections),
            Expect(AppGrid(
                "
src/foo.rs
1│hello
2│█oo 1/
2│foo 1/
3│foo 2
src/main.rs
1│hello
2│foo 3
3│foo4
"
                .trim_matches('\n')
                .to_string(),
            )),
            Expect(CurrentComponentPath(Some(s.foo_rs()))),
            // Expect foo.rs is focused, and main.rs is unfocused
            Expect(AppRangeStyleKey(
                "src/foo.rs",
                Some(StyleKey::FocusedWindowTitle),
            )),
            Expect(AppRangeStyleKey(
                "src/main.rs",
                Some(StyleKey::UnfocusedWindowTitle),
            )),
            // Cycle to the next cursor
            App(Dispatch::CycleCursor(Direction::End)),
            // Expect foo.rs is still focused, because foo.rs has two cursors
            Expect(AppRangeStyleKey(
                "src/foo.rs",
                Some(StyleKey::FocusedWindowTitle),
            )),
            // Cycle to the next cursor again
            App(Dispatch::CycleCursor(Direction::End)),
            // Expect main.rs is focused, and foo.rs is unfocused
            Expect(AppRangeStyleKey(
                "src/main.rs",
                Some(StyleKey::FocusedWindowTitle),
            )),
            Expect(AppCursorPosition(Position::new(7, 2))),
            Expect(AppGrid(
                "
src/foo.rs
1│hello
2│foo 1/
2│foo 1/
3│foo 2
src/main.rs
1│hello
2│█oo 3
3│foo4"
                    .trim_matches('\n')
                    .to_string(),
            )),
            // Cycle to the previous cursor
            App(Dispatch::CycleCursor(Direction::Start)),
            // Expect foo.rs is focused again
            Expect(AppRangeStyleKey(
                "src/foo.rs",
                Some(StyleKey::FocusedWindowTitle),
            )),
            // Expect the second foo of foo.rs is selected (not the first foo of foo.rs)
            Expect(AppGrid(
                "
src/foo.rs
1│hello
2│foo 1/
2│foo 1/
3│█oo 2
src/main.rs
1│hello
2│foo 3
3│foo4"
                    .trim_matches('\n')
                    .to_string(),
            )),
        ])
    })
}

#[test]
fn global_multicursor_marks() -> Result<(), anyhow::Error> {
    execute_test(|s| {
        Box::new([
            App(TerminalDimensionChanged(Dimension {
                width: 100,
                height: 10,
            })),
            App(OpenFile {
                path: s.gitignore(),
                owner: BufferOwner::User,
                focus: true,
            }),
            App(ToggleFileMark),
            App(SetFileContent(s.foo_rs(), "foo1".to_string())),
            App(SetFileContent(s.main_rs(), "foo2".to_string())),
            App(OpenSearchPrompt {
                scope: Scope::Global,
                if_current_not_found: IfCurrentNotFound::LookForward,
            }),
            App(HandleKeyEvents(keys!("r / f o o . enter").to_vec())),
            WaitForAppMessage(regex!("GlobalSearchFinished")),
            App(AddCursorToAllSelections),
            App(ToggleSelectionMark),
            Expect(CurrentMarks(
                [
                    (s.foo_rs(), [(CharIndex(0)..CharIndex(4)).into()].to_vec()),
                    (s.main_rs(), [(CharIndex(0)..CharIndex(4)).into()].to_vec()),
                ]
                .to_vec(),
            )),
            App(ToggleSelectionMark),
            Expect(CurrentMarks([].to_vec())),
        ])
    })
}

#[test]
fn delete_cursor_should_remove_file_if_the_selection_is_the_only_selection_of_the_file(
) -> Result<(), anyhow::Error> {
    execute_test(|s| {
        Box::new([
            App(TerminalDimensionChanged(Dimension {
                width: 100,
                height: 10,
            })),
            App(OpenFile {
                path: s.gitignore(),
                owner: BufferOwner::User,
                focus: true,
            }),
            App(ToggleFileMark),
            App(SetFileContent(s.foo_rs(), "foo1".to_string())),
            App(SetFileContent(s.main_rs(), "foo2".to_string())),
            App(OpenSearchPrompt {
                scope: Scope::Global,
                if_current_not_found: IfCurrentNotFound::LookForward,
            }),
            App(HandleKeyEvents(keys!("f o o enter").to_vec())),
            WaitForAppMessage(regex!("GlobalSearchFinished")),
            App(AddCursorToAllSelections),
            Expect(AppGrid(
                "
src/foo.rs
1│█oo1



src/main.rs
1│foo2
"
                .trim_matches('\n')
                .to_string(),
            )),
            App(CycleCursor(Direction::End)),
            Expect(AppRangeStyleKey(
                "src/main.rs",
                Some(StyleKey::FocusedWindowTitle),
            )),
            App(DeleteCursor),
            // Expect main.rs is removed, because its only selection is deleted
            Expect(AppGrid(
                "
src/foo.rs
1│█oo1
"
                .trim_matches('\n')
                .to_string(),
            )),
            Expect(AppRangeStyleKey(
                "src/foo.rs",
                Some(StyleKey::FocusedWindowTitle),
            )),
        ])
    })
}

#[test]
fn keep_matching_selections() -> Result<(), anyhow::Error> {
    execute_test(|s| {
        Box::new([
            App(TerminalDimensionChanged(Dimension {
                width: 100,
                height: 10,
            })),
            App(OpenFile {
                path: s.gitignore(),
                owner: BufferOwner::User,
                focus: true,
            }),
            App(ToggleFileMark),
            App(SetFileContent(s.foo_rs(), "foox1".to_string())),
            App(SetFileContent(s.main_rs(), "fooy2".to_string())),
            App(OpenSearchPrompt {
                scope: Scope::Global,
                if_current_not_found: IfCurrentNotFound::LookForward,
            }),
            App(HandleKeyEvents(keys!("f o o enter").to_vec())),
            WaitForAppMessage(regex!("GlobalSearchFinished")),
            App(AddCursorToAllSelections),
            Expect(AppGrid(
                "
src/foo.rs
1│█oox1



src/main.rs
1│fooy2
"
                .trim_matches('\n')
                .to_string(),
            )),
            // Enter line selection mode
            App(HandleKeyEvent(key!("a"))),
            App(OpenFilterSelectionsPrompt { maintain: true }),
            App(HandleKeyEvents(keys!("x enter").to_vec())),
            // Expect main.rs is removed, because its selection `fooy2` does not contain `x`
            Expect(AppGrid(
                "
src/foo.rs
1│█oox1







 LINE"
                    .trim_matches('\n')
                    .to_string(),
            )),
        ])
    })
}

#[test]
fn cycle_cursor_should_warp_around() -> Result<(), anyhow::Error> {
    execute_test(|s| {
        Box::new([
            App(TerminalDimensionChanged(Dimension {
                width: 100,
                height: 7,
            })),
            App(OpenFile {
                path: s.gitignore(),
                owner: BufferOwner::User,
                focus: true,
            }),
            App(ToggleFileMark),
            App(SetFileContent(s.foo_rs(), "foo1".to_string())),
            App(SetFileContent(s.main_rs(), "foo2".to_string())),
            App(OpenSearchPrompt {
                scope: Scope::Global,
                if_current_not_found: IfCurrentNotFound::LookForward,
            }),
            App(HandleKeyEvents(keys!("f o o enter").to_vec())),
            WaitForAppMessage(regex!("GlobalSearchFinished")),
            App(AddCursorToAllSelections),
            Expect(AppGrid(
                "
src/foo.rs
1│█oo1

src/main.rs
1│foo2
"
                .trim_matches('\n')
                .to_string(),
            )),
            Expect(CurrentComponentPath(Some(s.foo_rs()))),
            Expect(AppRangeStyleKey(
                "src/foo.rs",
                Some(StyleKey::FocusedWindowTitle),
            )),
            // Cycle to the previous cursor, except in goes to the last file in the list,
            // since the first selection of the first file is already the primary cursor
            App(Dispatch::CycleCursor(Direction::Start)),
            Expect(AppRangeStyleKey(
                "src/main.rs",
                Some(StyleKey::FocusedWindowTitle),
            )),
            // Cycle to the next cursor, except in goes to the first file in the list,
            // since the last selection of the last file is already the primary cursor
            App(Dispatch::CycleCursor(Direction::End)),
            Expect(AppRangeStyleKey(
                "src/foo.rs",
                Some(StyleKey::FocusedWindowTitle),
            )),
        ])
    })
}

#[test]
fn should_always_render_focused_selection_if_not_enough_vertical_space() -> Result<(), anyhow::Error>
{
    execute_test(|s| {
        Box::new([
            App(TerminalDimensionChanged(Dimension {
                width: 100,
                height: 2,
            })),
            App(OpenFile {
                path: s.gitignore(),
                owner: BufferOwner::User,
                focus: true,
            }),
            App(ToggleFileMark),
            App(SetFileContent(s.foo_rs(), "foo1".to_string())),
            App(SetFileContent(s.main_rs(), "foo2".to_string())),
            App(OpenSearchPrompt {
                scope: Scope::Global,
                if_current_not_found: IfCurrentNotFound::LookForward,
            }),
            App(HandleKeyEvents(keys!("f o o enter").to_vec())),
            WaitForAppMessage(regex!("GlobalSearchFinished")),
            App(AddCursorToAllSelections),
            Expect(AppGrid("1│█oo1".to_string())),
            App(Dispatch::CycleCursor(Direction::End)),
            Expect(AppGrid("1│█oo2".to_string())),
        ])
    })
}

#[test]
fn able_to_open_search_prompt_when_global_multicursor_enabled() -> Result<(), anyhow::Error> {
    execute_test(|s| {
        Box::new([
            App(TerminalDimensionChanged(Dimension {
                width: 50,
                height: 8,
            })),
            App(OpenFile {
                path: s.gitignore(),
                owner: BufferOwner::User,
                focus: true,
            }),
            App(ToggleFileMark),
            App(SetFileContent(s.main_rs(), "// foo x bar".to_string())),
            App(SetFileContent(s.foo_rs(), "// foo y bar".to_string())),
            App(OpenSearchPrompt {
                scope: Scope::Global,
                if_current_not_found: IfCurrentNotFound::LookForward,
            }),
            App(HandleKeyEvents(keys!("f o o enter").to_vec())),
            WaitForAppMessage(regex!("GlobalSearchFinished")),
            App(AddCursorToAllSelections),
            App(OpenSearchPrompt {
                scope: Scope::Local,
                if_current_not_found: IfCurrentNotFound::LookForward,
            }),
            Expect(AppGrid(
                "
src/foo.rs
1│// █oo y bar
src/main.rs
1│// foo x bar
Local search             │Completion
1│foo                    │1│y
2│█                      │2│bar
"
                .to_string(),
            )),
            App(HandleKeyEvents(keys!("b a r").to_vec())),
            Expect(AppGrid(
                "
src/foo.rs
1│// █oo y bar
src/main.rs
1│// foo x bar
Local search (Literal)   │Completion
1│foo                    │1│bar
2│bar█                   │
"
                .trim()
                .to_string(),
            )),
            App(HandleKeyEvents(keys!("enter").to_vec())),
            Expect(CurrentSelectedTexts(&["bar", "bar"])),
            App(HandleKeyEvents(keys!("f release-f z").to_vec())),
            App(SaveAll),
            Expect(FileContent(s.main_rs(), "// foo x z\n".to_string())),
            Expect(FileContent(s.foo_rs(), "// foo y z\n".to_string())),
        ])
    })
}

#[test]
fn text_insertion() -> Result<(), anyhow::Error> {
    execute_test(|s| {
        Box::new([
            App(SetFileContent(
                s.main_rs(),
                "// foo xxx yyy\n// second foo".to_string(),
            )),
            App(SetFileContent(s.foo_rs(), "// foo aaa bbb".to_string())),
            App(OpenSearchPrompt {
                scope: Scope::Global,
                if_current_not_found: IfCurrentNotFound::LookForward,
            }),
            App(HandleKeyEvents(keys!("f o o enter").to_vec())),
            WaitForAppMessage(regex!("GlobalSearchFinished")),
            Expect(QuickfixListContent(
                "
src/foo.rs
    1: 4  // foo aaa bbb

src/main.rs
    1: 4  // foo xxx yyy
    2:11  // second foo
"
                .trim()
                .to_string(),
            )),
            App(AddCursorToAllSelections),
            Editor(Change),
            App(HandleKeyEvents(keys!("b a r esc").to_vec())),
            App(SaveAll),
            Expect(FileContent(
                s.main_rs(),
                "// bar xxx yyy\n// second bar\n".to_string(),
            )),
            Expect(FileContent(s.foo_rs(), "// bar aaa bbb\n".to_string())),
        ])
    })
}

#[test]
fn toggling_global_multicursor_mode_should_unset_global_mode() -> Result<(), anyhow::Error> {
    execute_test(|s| {
        Box::new([
            App(SetFileContent(
                s.main_rs(),
                "// foo xxx yyy\n// second foo".to_string(),
            )),
            App(SetFileContent(s.foo_rs(), "// foo aaa bbb".to_string())),
            App(OpenSearchPrompt {
                scope: Scope::Global,
                if_current_not_found: IfCurrentNotFound::LookForward,
            }),
            App(HandleKeyEvents(keys!("f o o enter").to_vec())),
            WaitForAppMessage(regex!("GlobalSearchFinished")),
            Expect(CurrentGlobalMode(Some(GlobalMode::QuickfixListItem))),
            App(AddCursorToAllSelections),
            Expect(CurrentGlobalMode(None)),
        ])
    })
}

#[test]
fn primary_cursor_of_secondary_buffers_should_be_highlighted_as_ui_primary_selection_primary_cursor(
) -> Result<(), anyhow::Error> {
    execute_test(|s| {
        Box::new([
            App(TerminalDimensionChanged(Dimension {
                width: 20,
                height: 6,
            })),
            App(SetFileContent(s.main_rs(), "// foo bar".to_string())),
            App(SetFileContent(s.foo_rs(), "// foo spam".to_string())),
            App(OpenSearchPrompt {
                scope: Scope::Global,
                if_current_not_found: IfCurrentNotFound::LookForward,
            }),
            App(HandleKeyEvents(keys!("f o o enter").to_vec())),
            WaitForAppMessage(regex!("GlobalSearchFinished")),
            App(AddCursorToAllSelections),
            App(HandleKeyEvents(keys!("h").to_vec())),
            Expect(CurrentMode(Mode::Insert)),
            Expect(AppGrid(
                "
src/foo.rs
1│// █oo spam

src/main.rs
1│// foo bar
 ← Insert
"
                .to_string(),
            )),
            Expect(AppGridCellStyleKey(
                Position::new(4, 5),
                Some(StyleKey::UiPrimarySelectionPrimaryCursor),
            )),
        ])
    })
}

#[test]
fn simple_normal_mode_action_should_not_be_duplicated() -> Result<(), anyhow::Error> {
    execute_test(|s| {
        Box::new([
            App(SetFileContent(s.main_rs(), "// foo xxx yyy".to_string())),
            App(SetFileContent(s.foo_rs(), "// foo aaa bbb".to_string())),
            App(OpenSearchPrompt {
                scope: Scope::Global,
                if_current_not_found: IfCurrentNotFound::LookForward,
            }),
            App(HandleKeyEvents(keys!("f o o enter").to_vec())),
            WaitForAppMessage(regex!("GlobalSearchFinished")),
            App(AddCursorToAllSelections),
            // Enter word selection mode, then delete right
            App(HandleKeyEvents(keys!("s v l release-v").to_vec())),
            App(SaveAll),
            Expect(FileContent(s.main_rs(), "// xxx yyy\n".to_string())),
            Expect(FileContent(s.foo_rs(), "// aaa bbb\n".to_string())),
        ])
    })
}

#[test]
fn use_keep_primary_cursor_to_deactivate_global_multicursor() -> Result<(), anyhow::Error> {
    execute_test(|s| {
        Box::new([
            App(SetFileContent(s.foo_rs(), "// foo1 foo2".to_string())),
            App(SetFileContent(s.main_rs(), "// foo3 foo4".to_string())),
            App(OpenSearchPrompt {
                scope: Scope::Global,
                if_current_not_found: IfCurrentNotFound::LookForward,
            }),
            App(HandleKeyEvents(keys!("r / f o o . enter").to_vec())),
            WaitForAppMessage(regex!("GlobalSearchFinished")),
            App(AddCursorToAllSelections),
            Expect(CurrentSelectedTexts(&["foo1", "foo2", "foo3", "foo4"])),
            Expect(ExpectKind::GlobalMultiCursorActivated(true)),
            App(Dispatch::KeepCursorPrimaryOnly),
            Expect(CurrentSelectedTexts(&["foo1"])),
            Expect(ExpectKind::GlobalMultiCursorActivated(false)),
        ])
    })
}

#[test]
fn save_should_deactivate_global_multicursor() -> Result<(), anyhow::Error> {
    execute_test(|s| {
        Box::new([
            App(SetFileContent(s.foo_rs(), "// foo1 foo2".to_string())),
            App(SetFileContent(s.main_rs(), "// foo3 foo4".to_string())),
            App(OpenSearchPrompt {
                scope: Scope::Global,
                if_current_not_found: IfCurrentNotFound::LookForward,
            }),
            App(HandleKeyEvents(keys!("r / f o o . enter").to_vec())),
            WaitForAppMessage(regex!("GlobalSearchFinished")),
            App(AddCursorToAllSelections),
            Expect(CurrentSelectedTexts(&["foo1", "foo2", "foo3", "foo4"])),
            Expect(ExpectKind::GlobalMultiCursorActivated(true)),
            App(Dispatch::SaveFile),
            Expect(CurrentSelectedTexts(&["foo1"])),
            Expect(ExpectKind::GlobalMultiCursorActivated(false)),
        ])
    })
}

#[test]
fn quickfix_list_should_be_closed_when_global_multicursor_is_activated() -> Result<(), anyhow::Error>
{
    execute_test(|s| {
        Box::new([
            App(SetFileContent(s.main_rs(), "// foo xxx yyy".to_string())),
            App(SetFileContent(s.foo_rs(), "// foo aaa bbb".to_string())),
            App(OpenSearchPrompt {
                scope: Scope::Global,
                if_current_not_found: IfCurrentNotFound::LookForward,
            }),
            App(HandleKeyEvents(keys!("f o o enter").to_vec())),
            WaitForAppMessage(regex!("GlobalSearchFinished")),
            Expect(ComponentsOrder(
                [ComponentKind::SuggestiveEditor, ComponentKind::QuickfixList].to_vec(),
            )),
            App(AddCursorToAllSelections),
            Expect(ComponentsOrder([ComponentKind::SuggestiveEditor].to_vec())),
        ])
    })
}
