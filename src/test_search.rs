use crate::{
    app::{Dimension, Scope},
    rectangle::Rectangle,
    test_app::{execute_test, ExpectKind},
};

use my_proc_macros::keys;

use crate::{
    app::Dispatch::*,
    buffer::BufferOwner,
    components::editor::{DispatchEditor::*, IfCurrentNotFound},
    selection::SelectionMode,
    test_app::{ExpectKind::*, Step::*},
};

#[test]
fn live_search_preview() -> anyhow::Result<()> {
    execute_test(move |s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("snake_case kebab-case camelCase".to_string())),
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                SelectionMode::Word,
            )),
            Expect(CurrentSelectedTexts(&["snake_case"])),
            App(OpenSearchPrompt {
                scope: Scope::Local,
                if_current_not_found: IfCurrentNotFound::LookForward,
            }),
            App(HandleKeyEvents(keys!("k e b").to_vec())),
            Expect(CurrentEditorSelectedTexts(&["keb"])),
            // When the prompt is closed, expect the selection is restored
            App(HandleKeyEvents(keys!("esc esc").to_vec())),
            Expect(CurrentSelectedTexts(&["snake_case"])),
        ])
    })
}

#[test]
fn live_search_preview_should_work_with_custom_search_mode() -> anyhow::Result<()> {
    execute_test(move |s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("hello fo-ba fo_ba".to_string())),
            App(OpenSearchPrompt {
                scope: Scope::Local,
                if_current_not_found: IfCurrentNotFound::LookForward,
            }),
            App(HandleKeyEvents(keys!("n / f o space b a").to_vec())),
            Expect(CurrentEditorSelectedTexts(&["fo-ba"])),
        ])
    })
}

#[test]
fn live_search_preview_should_restore_scroll_offset_upon_cancelled() -> anyhow::Result<()> {
    execute_test(move |s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            App(TerminalDimensionChanged(Dimension {
                width: 200,
                height: 10,
            })),
            Editor(SetRectangle(Rectangle {
                origin: Default::default(),
                width: 100,
                height: 5,
            })),
            Editor(SetContent("x\n\n\n\n\n\n\n\n\n\ny".to_string())),
            Expect(ExpectKind::CurrentScrollOffSet(0)),
            App(OpenSearchPrompt {
                scope: Scope::Local,
                if_current_not_found: IfCurrentNotFound::LookForward,
            }),
            Expect(ExpectKind::CurrentScrollOffSet(0)),
            App(HandleKeyEvents(keys!("y").to_vec())),
            App(HandleKeyEvents(keys!("alt+/ alt+/").to_vec())),
            // Switch focus to the main panel so that we can expect the scroll offest,
            Expect(CurrentSelectedTexts(&["y"])),
            // Expect the scroll offset is non-zero (since "y" is so many lines below "x")
            Expect(ExpectKind::CurrentScrollOffSet(9)),
            // Switch focus back to the prompt and cancel it
            App(HandleKeyEvents(keys!("alt+/ esc esc").to_vec())),
            // Expect the scroll offset is reset to the previous scroll offset
            Expect(ExpectKind::CurrentScrollOffSet(0)),
        ])
    })
}

#[test]
fn live_search_preview_should_not_affect_prompt_history() -> anyhow::Result<()> {
    execute_test(move |s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            App(OpenSearchPrompt {
                scope: Scope::Local,
                if_current_not_found: IfCurrentNotFound::LookForward,
            }),
            // Search for "m" and cancel the prompt
            App(HandleKeyEvents(keys!("m esc esc").to_vec())),
            // Open search prompt again, expect there's no history of "m"
            App(OpenSearchPrompt {
                scope: Scope::Local,
                if_current_not_found: IfCurrentNotFound::LookForward,
            }),
            Expect(CurrentComponentContent("")),
        ])
    })
}

#[test]
fn live_search_preview_should_work_with_tab_completion_and_backspace() -> anyhow::Result<()> {
    execute_test(move |s| {
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
            // Search for "f" and then uses tab to use the first completion "foo"
            App(HandleKeyEvents(keys!("f tab").to_vec())),
            // Expect the current selection of the editor is "foo"
            Expect(CurrentEditorSelectedTexts(&["foo"])),
            // Press backspace
            App(HandleKeyEvents(keys!("backspace").to_vec())),
            Expect(CurrentEditorSelectedTexts(&["fo"])),
        ])
    })
}

// TODO: new test case: update component title with search mode

// TODO: new test case: when search query matches nothing, selection set should be reset, and users should be notified
// TODO: new test case: when user use tab, the live search should also run (right now after tabbing nothings happen)
// TODO: new test case: live search preview should not update selection set history
// TODO: live preview direction follows opposite of CursorDirection
// TODO: each key change should trigger a re-search based on the pre-prompt cursor position
