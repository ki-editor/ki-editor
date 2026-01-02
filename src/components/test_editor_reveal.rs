use crate::buffer::BufferOwner;
use crate::components::editor::{DispatchEditor::*, Movement::*, ViewAlignment};
use crate::rectangle::Rectangle;
use crate::test_app::*;

use crate::{grid::StyleKey, position::Position, selection::SelectionMode};

use itertools::Itertools;
use SelectionMode::*;

use super::editor::{IfCurrentNotFound, Reveal};

#[test]
fn reveal_styling() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetRectangle(Rectangle {
                origin: Position::new(0, 0),
                width: 50,
                height: 10,
            })),
            Editor(SetContent("foo\nbar".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Line)),
            Expect(CurrentSelectedTexts(&["foo"])),
            Editor(MoveSelection(Right)),
            Expect(CurrentSelectedTexts(&["bar"])),
            Editor(ToggleReveal(Reveal::Cursor)),
            Expect(GridCellStyleKey(
                Position::new(2, 2),
                Some(StyleKey::UiPrimarySelectionAnchors),
            )),
            Expect(GridCellStyleKey(
                Position::new(2, 3),
                Some(StyleKey::UiPrimarySelectionAnchors),
            )),
            Expect(GridCellStyleKey(
                Position::new(2, 4),
                Some(StyleKey::UiPrimarySelectionSecondaryCursor),
            )),
        ])
    })
}

#[test]
/// When Reveal Mark is activated
/// All the marks should be always visible
fn reveal_mark() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetLanguage(Box::new(
                crate::config::from_extension("md").unwrap(),
            ))),
            Editor(SetRectangle(Rectangle {
                origin: Position::new(0, 0),
                width: 50,
                height: 4,
            })),
            Editor(SetContent(
                "
beta
mark-x
phi
mark-y
zeta
"
                .trim()
                .to_string(),
            )),
            Editor(MatchLiteral("mark-x".to_string())),
            Editor(ToggleMark),
            Editor(MatchLiteral("mark-y".to_string())),
            Editor(ToggleMark),
            Editor(MatchLiteral("zeta".to_string())),
            Expect(EditorGrid(
                "🦀  main.rs [*]
4│mark-y
5│█eta
",
            )),
            Editor(ToggleReveal(Reveal::Mark)),
            Expect(EditorGrid(
                "
🦀  main.rs [*]
2│mark-x
4│mark-y
5│█eta
"
                .trim(),
            )),
            Editor(MatchLiteral("phi".to_string())),
            Expect(EditorGrid(
                "
🦀  main.rs [*]
2│mark-x
3│█hi
4│mark-y
"
                .trim(),
            )),
            Editor(MatchLiteral("beta".to_string())),
            Expect(EditorGrid(
                "
🦀  main.rs [*]
1│█eta
2│mark-x
4│mark-y
"
                .trim(),
            )),
        ])
    })
}

#[test]
fn reveal_selections() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetRectangle(Rectangle {
                origin: Position::new(0, 0),
                width: 50,
                height: 4,
            })),
            Editor(SetContent(
                "
fn main() {
}
fn bar() {
}
fn spam() {
}
"
                .trim()
                .to_string(),
            )),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Line)),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, SyntaxNode)),
            Expect(CurrentSelectedTexts(&["fn main() {\n}"])),
            Expect(EditorGrid(
                "
🦀  main.rs [*]
1│█n main() {
2│}
3│fn bar() {
"
                .trim(),
            )),
            Editor(ToggleReveal(Reveal::CurrentSelectionMode)),
            Expect(EditorGrid(
                "
🦀  main.rs [*]
1│█n main() {
3│fn bar() {
5│fn spam() {
"
                .trim(),
            )),
        ])
    })
}

#[test]
fn reveal_selections_should_be_deactivated_when_selection_mode_changed() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Line)),
            Editor(ToggleReveal(Reveal::CurrentSelectionMode)),
            Expect(CurrentReveal(Some(Reveal::CurrentSelectionMode))),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, SyntaxNode)),
            Expect(CurrentReveal(None)),
        ])
    })
}

#[test]
fn reveal_cursors() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetRectangle(Rectangle {
                origin: Position::new(0, 0),
                width: 50,
                height: 4,
            })),
            Editor(SetContent(
                "
foo
x
foo
x
foo
"
                .trim()
                .to_string(),
            )),
            Editor(MatchLiteral("foo".to_string())),
            Editor(CursorAddToAllSelections),
            Expect(CurrentReveal(Some(Reveal::Cursor))),
            Expect(EditorGrid(
                "
🦀  main.rs [*]
1│█oo
3│foo
5│foo
"
                .trim(),
            )),
            Editor(CursorKeepPrimaryOnly),
            Expect(CurrentReveal(None)),
            Expect(EditorGrid(
                "
🦀  main.rs [*]
1│█oo
2│x
3│foo
"
                .trim(),
            )),
        ])
    })
}

#[test]
fn each_revealed_section_should_show_parent_lines() -> anyhow::Result<()> {
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
  foo();
  bar();
}
fn two() {
  foo();
  bar();
}
"
                .trim()
                .to_string(),
            )),
            Editor(SetRectangle(Rectangle {
                origin: Position::new(0, 0),
                width: 50,
                height: 5,
            })),
            Editor(MatchLiteral("bar".to_string())),
            Editor(CursorAddToAllSelections),
            Expect(CurrentReveal(Some(Reveal::Cursor))),
            Expect(EditorGrid(
                "
🦀  main.rs [*]
1│fn main() {
3│  █ar();
5│fn two() {
7│  bar();
"
                .trim(),
            )),
        ])
    })
}

#[test]
/// When there are not enough spaces, trimmed the hidden parent lines of each section
fn each_revealed_section_selections_should_always_be_visible() -> anyhow::Result<()> {
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
  foo();
  bar();
}
fn two() {
  foo();
  bar();
}
"
                .trim()
                .to_string(),
            )),
            Editor(SetRectangle(Rectangle {
                origin: Position::new(0, 0),
                width: 50,
                height: 3,
            })),
            Editor(MatchLiteral("bar".to_string())),
            Editor(CursorAddToAllSelections),
            Expect(CurrentReveal(Some(Reveal::Cursor))),
            // The parent lines of the both `bar();` are trimmed due to space constrained
            Expect(EditorGrid(
                "
🦀  main.rs [*]
3│  █ar();
7│  bar();
"
                .trim(),
            )),
        ])
    })
}
#[test]
/// The first selection of each section should be visible even when wrapped.
/// When there are not enough spaces, trimmed the hidden parent lines of each section
fn each_revealed_section_first_selection_should_always_be_visible_although_wrapped(
) -> anyhow::Result<()> {
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
  foo();
  spamspamsam barXXX();
}
fn two() {
  foo();
  spamspamspam barYYY();
}
"
                .trim()
                .to_string(),
            )),
            Editor(SetRectangle(Rectangle {
                origin: Position::new(0, 0),
                width: 20,
                height: 3,
            })),
            Editor(MatchLiteral("bar".to_string())),
            Editor(CursorAddToAllSelections),
            Expect(CurrentReveal(Some(Reveal::Cursor))),
            // The parent lines of the both `bar();` are trimmed due to space constrained
            Expect(EditorGrid(
                "
🦀  main.rs [*]
↪│█arXXX();
↪│barYYY();
"
                .trim(),
            )),
        ])
    })
}

#[test]
fn all_selections_on_same_line_but_all_wrapped() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(
                "foofooApple barbar foofooBanana".trim().to_string(),
            )),
            Editor(SetRectangle(Rectangle {
                origin: Position::new(0, 0),
                width: 20,
                height: 3,
            })),
            Editor(MatchLiteral("foofoo".to_string())),
            Editor(ToggleReveal(Reveal::CurrentSelectionMode)),
            Expect(EditorGrid(
                "
🦀  main.rs [*]
1│█oofooApple
↪│foofooBanana"
                    .trim(),
            )),
        ])
    })
}

#[test]
fn total_count_of_highlighted_ranges_should_equal_total_count_of_possible_selections(
) -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("foo foo foo".trim().to_string())),
            Editor(SetRectangle(Rectangle {
                origin: Position::new(0, 0),
                width: 20,
                height: 4,
            })),
            Editor(MatchLiteral("foo".to_string())),
            Editor(ToggleReveal(Reveal::CurrentSelectionMode)),
            Expect(EditorGrid(
                "
🦀  main.rs [*]
1│█oo foo foo
1│foo foo foo
1│foo foo foo
"
                .trim(),
            )),
            Expect(ExpectKind::CountHighlightedCells(
                StyleKey::UiPossibleSelection,
                6, // 2 x 3 characters of "foo" (excluding the primarily selected "foo")
            )),
            Expect(ExpectKind::CountHighlightedCells(
                StyleKey::UiPrimarySelectionAnchors,
                2, // Only the 1st and 2nd characters of "foo" of the primary selection
                   // The 3rd character is the secondary
            )),
        ])
    })
}

#[test]
fn total_count_of_rendered_secondary_selections_should_equal_total_count_of_actual_secondary_selections(
) -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("foo foo foo".trim().to_string())),
            Editor(SetRectangle(Rectangle {
                origin: Position::new(0, 0),
                width: 20,
                height: 4,
            })),
            Editor(MatchLiteral("foo".to_string())),
            Editor(CursorAddToAllSelections),
            Expect(EditorGrid(
                "
🦀  main.rs [*]
1│█oo foo foo
1│foo foo foo
1│foo foo foo
"
                .trim(),
            )),
            Expect(ExpectKind::CountHighlightedCells(
                StyleKey::UiPrimarySelectionAnchors,
                2, // Only the 1st and 2nd characters of "foo" of the primary selection
                   // The 3rd character is the secondary
            )),
            Expect(ExpectKind::CountHighlightedCells(
                StyleKey::UiSecondarySelectionAnchors,
                2, // 2 secondary selections x 1 the middle character of "foo"
                   // the first character is secondary selection primary cursor
                   // the third character is secondary selection secondary cursor
            )),
        ])
    })
}

#[test]
fn total_count_of_rendered_marks_should_equal_total_count_of_actual_marks() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("foo foo foo".trim().to_string())),
            Editor(SetRectangle(Rectangle {
                origin: Position::new(0, 0),
                width: 20,
                height: 4,
            })),
            Editor(MatchLiteral("foo".to_string())),
            Editor(CursorAddToAllSelections),
            Editor(ToggleMark),
            Editor(CursorKeepPrimaryOnly),
            Editor(ToggleReveal(Reveal::Mark)),
            Expect(EditorGrid(
                "
🦀  main.rs [*]
1│█oo foo foo
1│foo foo foo
1│foo foo foo
"
                .trim(),
            )),
            Expect(ExpectKind::CountHighlightedCells(
                StyleKey::UiPrimarySelectionSecondaryCursor,
                1,
            )),
            Expect(ExpectKind::CountHighlightedCells(
                StyleKey::UiMark,
                8, // 2 secondary selections x 3 characters of "foo"
                   // + 1 primary selection x first 2 characters of "foo"
                   // (last character is primary selection secondary cursor)
            )),
        ])
    })
}

#[test]
/// The first of each section (height > 1) is styled as section divider
fn section_divider_style() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("foo\nbar\nbar".trim().to_string())),
            Editor(SetRectangle(Rectangle {
                origin: Position::new(0, 0),
                width: 20,
                height: 4,
            })),
            Editor(MatchLiteral("bar".to_string())),
            Editor(CursorAddToAllSelections),
            Editor(SwitchViewAlignment),
            Editor(SwitchViewAlignment),
            Editor(SwitchViewAlignment),
            Expect(EditorGrid(
                "
🦀  main.rs [*]
1│foo
2│█ar
3│bar
"
                .trim(),
            )),
            // Expect the first line of the first section (line 1) is styled as section divider
            // because the height of the first section is more than 1
            ExpectMulti(
                (0..5)
                    .map(|column| {
                        GridCellStyleKey(Position::new(1, column), Some(StyleKey::UiSectionDivider))
                    })
                    .collect_vec(),
            ),
            // Expect the first line of the second section (line 3) is NOT styled as section divider
            // because the height of the second section is NOT more than 1
            ExpectMulti(
                (0..5)
                    .map(|column| {
                        Not(Box::new(GridCellStyleKey(
                            Position::new(3, column),
                            Some(StyleKey::UiSectionDivider),
                        )))
                    })
                    .collect_vec(),
            ),
        ])
    })
}

#[test]
fn reveal_cursor_selection_extension() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(
                "foo\nbar spam\nfoo\nbar spam".trim().to_string(),
            )),
            Editor(SetRectangle(Rectangle {
                origin: Position::new(0, 0),
                width: 20,
                height: 6,
            })),
            Editor(MatchLiteral("bar".to_string())),
            Editor(CursorAddToAllSelections),
            Expect(EditorGrid(
                "
🦀  main.rs [*]
1│foo
2│█ar spam
3│foo
3│foo
4│bar spam
"
                .trim(),
            )),
            Editor(EnableSelectionExtension),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
            Editor(MoveSelection(Right)),
            Expect(EditorGrid(
                "
🦀  main.rs [*]
1│foo
2│bar █pam
3│foo
3│foo
4│bar spam
"
                .trim(),
            )),
            Expect(GridCellStyleKey(
                Position::new(2, 5),
                Some(StyleKey::UiPrimarySelection),
            )),
            Expect(GridCellsStyleKey(
                [
                    Position::new(2, 2),
                    Position::new(2, 3),
                    Position::new(2, 4),
                    Position::new(2, 7),
                    Position::new(2, 8),
                ]
                .to_vec(),
                Some(StyleKey::UiPrimarySelectionAnchors),
            )),
            Expect(GridCellStyleKey(
                Position::new(2, 9),
                Some(StyleKey::UiPrimarySelectionSecondaryCursor),
            )),
            Expect(GridCellStyleKey(
                Position::new(5, 5),
                Some(StyleKey::UiSecondarySelection),
            )),
            Expect(GridCellsStyleKey(
                [
                    Position::new(5, 2),
                    Position::new(5, 3),
                    Position::new(5, 4),
                    Position::new(5, 7),
                    Position::new(5, 8),
                ]
                .to_vec(),
                Some(StyleKey::UiSecondarySelectionAnchors),
            )),
            Expect(GridCellStyleKey(
                Position::new(5, 6),
                Some(StyleKey::UiSecondarySelectionPrimaryCursor),
            )),
            Expect(GridCellStyleKey(
                Position::new(5, 9),
                Some(StyleKey::UiSecondarySelectionSecondaryCursor),
            )),
        ])
    })
}

#[test]
fn revealed_section_view_aligment() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(
                "
a();
b();
spam();
c();
d();
"
                .trim()
                .to_string(),
            )),
            Editor(SetRectangle(Rectangle {
                origin: Position::new(0, 0),
                width: 50,
                height: 4,
            })),
            Editor(MatchLiteral("spam".to_string())),
            Editor(CursorAddToAllSelections),
            Expect(CurrentReveal(Some(Reveal::Cursor))),
            Editor(SwitchViewAlignment),
            Expect(CurrentViewAlignment(Some(ViewAlignment::Top))),
            Expect(EditorGrid(
                "
🦀  main.rs [*]
3│█pam();
4│c();
5│d();
"
                .trim(),
            )),
            Editor(SwitchViewAlignment),
            Expect(CurrentViewAlignment(Some(ViewAlignment::Center))),
            Expect(EditorGrid(
                "
🦀  main.rs [*]
2│b();
3│█pam();
4│c();
"
                .trim(),
            )),
            Editor(SwitchViewAlignment),
            Expect(CurrentViewAlignment(Some(ViewAlignment::Bottom))),
            Expect(EditorGrid(
                "
🦀  main.rs [*]
1│a();
2│b();
3│█pam();
"
                .trim(),
            )),
        ])
    })
}
