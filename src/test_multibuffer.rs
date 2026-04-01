use lazy_regex::regex;
use my_proc_macros::keys;

use crate::{
    app::{Dimension, Dispatch::*, Scope},
    buffer::BufferOwner,
    components::editor::{Direction, DispatchEditor::*, IfCurrentNotFound},
    test_app::{execute_test, ExpectKind::*, Step::*},
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
            Editor(CursorAddToAllSelections),
            Expect(AppGrid(
                "
src/foo.rs
1│// third █oo

src/main.rs
1│// first █oo
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
            Editor(CursorAddToAllSelections),
            Expect(AppGrid(
                "
src/foo.rs
1│// second █oo


src/main.rs
1│// first █oo"
                    .trim_matches('\n')
                    .to_string(),
            )),
        ])
    })
}

#[test]
fn able_to_open_search_prompt_when_multibuffer_enabled() -> Result<(), anyhow::Error> {
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
            Editor(CursorAddToAllSelections),
            App(OpenSearchPrompt {
                scope: Scope::Local,
                if_current_not_found: IfCurrentNotFound::LookForward,
            }),
            Expect(AppGrid(
                "
src/foo.rs
1│// █oo y bar
src/main.rs
1│// █oo x bar
Local search             │Completion
1│foo                    │1│█
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
1│// █oo x bar
Local search (Literal)   │Completion
1│foo                    │1│█ar
2│bar█                   │
"
                .trim()
                .to_string(),
            )),
            App(HandleKeyEvents(keys!("enter").to_vec())),
            Expect(CurrentSelectedTexts(&["bar"])),
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
            Editor(CursorAddToAllSelections),
            Expect(CurrentSelectedTexts(&["foo"])),
            Expect(CurrentGlobalMode(None)),
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
            Editor(CursorAddToAllSelections),
            // Enter word selection mode, then delete right
            App(HandleKeyEvents(keys!("s v l release-v").to_vec())),
            App(SaveAll),
            Expect(FileContent(s.main_rs(), "// xxx yyy\n".to_string())),
            Expect(FileContent(s.foo_rs(), "// aaa bbb\n".to_string())),
        ])
    })
}
