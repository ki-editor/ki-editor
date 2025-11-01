use crate::{app::Scope, test_app::execute_test};

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
            // Switch focus to the main panel so that we can expect the selected texts
            App(HandleKeyEvents(keys!("alt+/ alt+/").to_vec())),
            Expect(CurrentSelectedTexts(&["keb"])),
            // Switch focus back to the search prompt
            App(HandleKeyEvents(keys!("alt+/").to_vec())),
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
            // Switch focus to the main panel so that we can expect the selected texts
            App(HandleKeyEvents(keys!("alt+/").to_vec())),
            Expect(CurrentSelectedTexts(&["fo-ba"])),
        ])
    })
}

// TODO: new test cases: the scroll off set should be reset
// TODO: new test case: the search query during live search should not add to search history
