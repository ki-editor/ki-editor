use std::time::Duration;

use lazy_regex::regex;
use my_proc_macros::{key, keys};

use crate::{
    app::{
        Dimension,
        Dispatch::{self, *},
        Scope,
    },
    buffer::BufferOwner,
    components::editor::{Direction, DispatchEditor::*, IfCurrentNotFound},
    grid::StyleKey,
    selection::SelectionMode,
    selection_mode::GetGapMovement,
    test_app::{execute_test, execute_test_custom, ExpectKind::*, RunTestOptions, Step::*},
};

#[test]
fn case_1() -> Result<(), anyhow::Error> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("foo xxx yyy".to_string())),
            Editor(Save),
            App(OpenFile {
                path: s.foo_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("foo aaa bbb".to_string())),
            Editor(Save),
            App(OpenSearchPrompt {
                scope: Scope::Global,
                if_current_not_found: IfCurrentNotFound::LookForward,
            }),
            App(HandleKeyEvents(keys!("f o o enter").to_vec())),
            WaitForAppMessage(regex!("GlobalSearchFinished")),
            Expect(QuickfixListContent(
                "
src/foo.rs
    1:1  foo aaa bbb

src/main.rs
    1:1  foo xxx yyy
"
                .trim()
                .to_string(),
            )),
            Editor(CursorAddToAllSelections),
            Expect(CurrentGlobalMode(None)),
            Editor(Change),
            App(HandleKeyEvents(keys!("b a r esc").to_vec())),
            App(SaveAll),
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Expect(CurrentComponentContent("bar xxx yyy")),
            App(OpenFile {
                path: s.foo_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Expect(CurrentComponentContent("bar aaa bbb")),
        ])
    })
}
