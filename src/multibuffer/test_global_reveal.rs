use lazy_regex::regex;
use my_proc_macros::keys;

use crate::{
    app::{Dimension, Dispatch::*, Scope},
    components::editor::{DispatchEditor::*, IfCurrentNotFound, Movement},
    grid::StyleKey,
    test_app::{execute_test, ExpectKind::*, Step::*},
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

#[test]
fn should_only_render_one_cursor_when_same_line_has_multiple_matches() -> Result<(), anyhow::Error>
{
    execute_test(|s| {
        Box::new([
            App(TerminalDimensionChanged(Dimension {
                width: 100,
                height: 8,
            })),
            App(SetFileContent(
                s.foo_rs(),
                "// x \n// qux1 qux2".to_string(),
            )),
            App(OpenSearchPrompt {
                scope: Scope::Global,
                if_current_not_found: IfCurrentNotFound::LookForward,
            }),
            App(HandleKeyEvents(keys!("r / q u x . enter").to_vec())),
            WaitForAppMessage(regex!("GlobalSearchFinished")),
            App(ToggleRevealSelections),
            Expect(AppGrid(
                "
src/foo.rs
1│// x
2│// █ux1 qux2

1│// x
2│// qux1 qux2
"
                .trim_matches('\n')
                .to_string(),
            )),
            // Expect the first qux1 to be styled as the primary selection
            Expect(AppRangeStyleKey(
                "█ux",
                Some(StyleKey::UiPrimarySelectionAnchors),
            )),
            // Expect the second qux1 to not be styled, since the second split is highlighting qux2, not qux1
            Expect(AppRangeStyleKey("qux1", Some(StyleKey::Default))),
            // Expect the first qux2 is not styled, because the first split should highlight qux1
            Expect(AppRangeIndexStyleKey("qux2", 0, Some(StyleKey::Default))),
            // Expect the second qux2 is styled as UiPossibleSelection, because the second split should highlight qux2
            Expect(AppRangeIndexStyleKey(
                "qux2",
                1,
                Some(StyleKey::UiPossibleSelection),
            )),
        ])
    })
}

#[test]
fn should_prioritize_showing_selection_and_hide_file_path_if_space_is_limited(
) -> Result<(), anyhow::Error> {
    execute_test(|s| {
        Box::new([
            App(TerminalDimensionChanged(Dimension {
                width: 100,
                height: 5,
            })),
            App(SetFileContent(
                s.foo_rs(),
                "// x \n// qux1 qux2".to_string(),
            )),
            App(SetFileContent(
                s.main_rs(),
                "// x \n// qux3 qux4".to_string(),
            )),
            App(OpenSearchPrompt {
                scope: Scope::Global,
                if_current_not_found: IfCurrentNotFound::LookForward,
            }),
            App(HandleKeyEvents(keys!("r / q u x . enter").to_vec())),
            WaitForAppMessage(regex!("GlobalSearchFinished")),
            App(ToggleRevealSelections),
            // When there's not enough space, all selections within foo.rs should be shown
            // sacrificing the rendering of the main.rs split.
            //
            // This is because whenwhenile path is shown, all of its selections should be shown.
            // This is the file path will be hidden if not all of its selections are shown,
            // as showing the selection takes priority over showing the file path
            Expect(AppGrid(
                "
2│// █ux1 qux2
2│// qux1 qux2
2│// qux3 qux4
2│// qux3 qux4
"
                .trim_matches('\n')
                .to_string(),
            )),
            // Increase height by 1
            App(TerminalDimensionChanged(Dimension {
                width: 100,
                height: 6,
            })),
            // Expect the name of foo.rs is rendered, since there are more than enough spaces to show all ranges
            Expect(AppGrid(
                "
src/foo.rs
2│// █ux1 qux2
2│// qux1 qux2
2│// qux3 qux4
2│// qux3 qux4
"
                .trim_matches('\n')
                .to_string(),
            )),
        ])
    })
}
