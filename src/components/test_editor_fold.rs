use crate::buffer::BufferOwner;
use crate::components::editor::{DispatchEditor::*, Movement::*};
use crate::rectangle::Rectangle;
use crate::test_app::*;

use crate::{grid::StyleKey, position::Position, selection::SelectionMode};

use SelectionMode::*;

use super::editor::{Fold, IfCurrentNotFound};

#[test]
fn fold_styling() -> anyhow::Result<()> {
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
            Editor(MoveSelection(Down)),
            Expect(CurrentSelectedTexts(&["bar"])),
            Editor(ToggleFold(Fold::Cursor)),
            Expect(GridCellStyleKey(
                Position::new(2, 0 + 2),
                Some(StyleKey::UiPrimarySelectionAnchors),
            )),
            Expect(GridCellStyleKey(
                Position::new(2, 1 + 2),
                Some(StyleKey::UiPrimarySelectionAnchors),
            )),
            Expect(GridCellStyleKey(
                Position::new(2, 2 + 2),
                Some(StyleKey::UiPrimarySelectionSecondaryCursor),
            )),
        ])
    })
}

#[test]
/// When Fold by Mark is activated
/// All the marks should be always visible
fn fold_by_mark() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetLanguage(shared::language::from_extension("md").unwrap())),
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
                "
🦀  src/main.rs [*]
3│phi
4│mark-y
5│█eta
"
                .trim(),
            )),
            Editor(ToggleFold(Fold::Mark)),
            Expect(EditorGrid(
                "
🦀  src/main.rs [*]
2│mark-x
4│mark-y
5│█eta
"
                .trim(),
            )),
            Editor(MatchLiteral("phi".to_string())),
            Expect(EditorGrid(
                "
🦀  src/main.rs [*]
2│mark-x
3│█hi
4│mark-y
"
                .trim(),
            )),
            Editor(MatchLiteral("beta".to_string())),
            Expect(EditorGrid(
                "
🦀  src/main.rs [*]
1│█eta
2│mark-x
4│mark-y
"
                .trim(),
            )),
        ])
    })
}

/// Fold by current selection mode
#[test]
fn fold_by_current_selection_mode() -> anyhow::Result<()> {
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
🦀  src/main.rs [*]
1│█n main() {
2│}
3│fn bar() {
"
                .trim(),
            )),
            Editor(ToggleFold(Fold::CurrentSelectionMode)),
            Expect(EditorGrid(
                "
🦀  src/main.rs [*]
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
fn fold_by_current_selection_mode_should_be_deactivated_when_selection_mode_changed(
) -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Line)),
            Editor(ToggleFold(Fold::CurrentSelectionMode)),
            Expect(CurrentFold(Some(Fold::CurrentSelectionMode))),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, SyntaxNode)),
            Expect(CurrentFold(None)),
        ])
    })
}

/// Fold by cursors
#[test]
fn fold_by_cursors() -> anyhow::Result<()> {
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
            Expect(CurrentFold(Some(Fold::Cursor))),
            Expect(EditorGrid(
                "
🦀  src/main.rs [*]
1│█oo
3│foo
5│foo
"
                .trim(),
            )),
            Editor(CursorKeepPrimaryOnly),
            Expect(CurrentFold(None)),
            Expect(EditorGrid(
                "
🦀  src/main.rs [*]
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
fn each_folded_section_should_show_parent_lines() -> anyhow::Result<()> {
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
            Expect(CurrentFold(Some(Fold::Cursor))),
            Expect(EditorGrid(
                "
🦀  src/main.rs [*]
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
fn each_folded_section_selections_should_always_be_visible() -> anyhow::Result<()> {
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
            Expect(CurrentFold(Some(Fold::Cursor))),
            // The parent lines of the both `bar();` are trimmed due to space constrained
            Expect(EditorGrid(
                "
🦀  src/main.rs [*]
3│  █ar();
7│  bar();
"
                .trim(),
            )),
        ])
    })
}
