use lazy_regex::regex;
use my_proc_macros::{key, keys};

use crate::{
    app::{
        Dimension,
        Dispatch::{self, *},
        Scope,
    },
    buffer::BufferOwner,
    components::editor::{Direction, DispatchEditor::*, IfCurrentNotFound, Mode, Movement},
    context::GlobalMode,
    grid::StyleKey,
    position::Position,
    test_app::{
        execute_test,
        ExpectKind::{self, *},
        Step::*,
    },
    ui_tree::ComponentKind,
};

#[test]
fn should_render_only_one_cursor_even_when_multiple_files_are_shown() -> Result<(), anyhow::Error> {
    execute_test(|s| {
        Box::new([
            App(TerminalDimensionChanged(Dimension {
                width: 100,
                height: 8,
            })),
            App(SetFileContent(s.foo_rs(), "// x \n// foo1".to_string())),
            App(SetFileContent(s.main_rs(), "// x \n// foo2".to_string())),
            App(OpenSearchPrompt {
                scope: Scope::Global,
                if_current_not_found: IfCurrentNotFound::LookForward,
            }),
            App(HandleKeyEvents(keys!("r / f o o . enter").to_vec())),
            WaitForAppMessage(regex!("GlobalSearchFinished")),
            App(ToggleRevealSelections),
            Expect(AppGrid(
                "
src/foo.rs
1│// x
2│// █oo1

src/main.rs
1│// x
2│// foo2"
                    .trim_matches('\n')
                    .to_string(),
            )),
            Expect(AppRangeStyleKey(
                "foo2",
                Some(StyleKey::UiPossibleSelection),
            )),
            // Toggling movement should move to next selection
            Editor(MoveSelection(Movement::Right)),
            Expect(AppGrid(
                "
src/foo.rs
1│// x
2│// foo1

src/main.rs
1│// x
2│// █oo2"
                    .trim_matches('\n')
                    .to_string(),
            )),
            Expect(AppRangeStyleKey(
                "foo1",
                Some(StyleKey::UiPossibleSelection),
            )),
        ])
    })
}

#[test]
fn should_work_when_one_file_has_more_than_one_matches() -> Result<(), anyhow::Error> {
    execute_test(|s| {
        Box::new([
            App(TerminalDimensionChanged(Dimension {
                width: 100,
                height: 8,
            })),
            App(SetFileContent(
                s.foo_rs(),
                "// x\n// foo1\n// foo2".to_string(),
            )),
            App(SetFileContent(s.main_rs(), "// x \n// foo3".to_string())),
            App(OpenSearchPrompt {
                scope: Scope::Global,
                if_current_not_found: IfCurrentNotFound::LookForward,
            }),
            App(HandleKeyEvents(keys!("r / f o o . enter").to_vec())),
            WaitForAppMessage(regex!("GlobalSearchFinished")),
            App(ToggleRevealSelections),
            Expect(AppGrid(
                "
src/foo.rs
1│// x
2│// █oo1
2│// foo1
3│// foo2
src/main.rs
2│// foo3"
                    .trim_matches('\n')
                    .to_string(),
            )),
            Expect(AppRangeStyleKey(
                "foo2",
                Some(StyleKey::UiPossibleSelection),
            )),
            Expect(AppRangeStyleKey(
                "foo3",
                Some(StyleKey::UiPossibleSelection),
            )),
            Editor(MoveSelection(Movement::Right)),
            Expect(AppGrid(
                "
src/foo.rs
1│// x
2│// foo1
2│// foo1
3│// █oo2
src/main.rs
2│// foo3"
                    .trim_matches('\n')
                    .to_string(),
            )),
            Expect(AppRangeStyleKey(
                "foo3",
                Some(StyleKey::UiPossibleSelection),
            )),
        ])
    })
}
