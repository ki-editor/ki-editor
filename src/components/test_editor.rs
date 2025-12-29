use crate::app::{Dimension, LocalSearchConfigUpdate, Scope};
use crate::buffer::BufferOwner;
use crate::char_index_range::CharIndexRange;
use crate::clipboard::CopiedTexts;
use crate::components::editor::{
    DispatchEditor::{self, *},
    Movement::*,
    PriorChange,
};
use crate::context::{Context, GlobalMode, LocalSearchConfigMode, Search};
use crate::git::DiffMode;
use crate::grid::IndexedHighlightGroup;
use crate::list::grep::RegexConfig;
use crate::lsp::process::LspNotification;
use crate::quickfix_list::{Location, QuickfixListItem};
use crate::rectangle::Rectangle;
use crate::selection::CharIndex;
use crate::style::Style;
use crate::test_app::*;

use crate::themes::GitGutterStyles;
use crate::ui_tree::ComponentKind;
use crate::{
    components::editor::{Direction, Mode, ViewAlignment},
    grid::StyleKey,
    position::Position,
    selection::SelectionMode,
    themes::Theme,
};

use itertools::Itertools;
use lazy_regex::regex;
use my_proc_macros::{hex, key, keys};
use serial_test::serial;

use SelectionMode::*;

use super::editor::IfCurrentNotFound;
use super::editor::SurroundKind;
use super::prompt::PromptHistoryKey;
use super::render_editor::markup_focused_tab;

#[test]
fn raise_bottom_node() -> anyhow::Result<()> {
    execute_test(|s| {
        let input = "fn main() { x + 1 }";
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(input.to_string())),
            Editor(MatchLiteral("x".to_string())),
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                SelectionMode::SyntaxNodeFine,
            )),
            Expect(CurrentSelectedTexts(&["x"])),
            Editor(Replace(Up)),
            Expect(CurrentComponentContent("fn main() { x }")),
        ])
    })
}

#[test]
fn toggle_visual_mode() -> anyhow::Result<()> {
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
            Expect(CurrentSelectedTexts(&["fn f("])),
            Editor(SwapExtensionAnchor),
            Editor(MoveSelection(Right)),
            Expect(CurrentSelectedTexts(&["f("])),
            Editor(Reset),
            Expect(CurrentSelectedTexts(&["f"])),
            Editor(MoveSelection(Next)),
            Expect(CurrentSelectedTexts(&["("])),
        ])
    })
}

#[test]
/// Kill means delete until the next selection
fn delete_should_kill_if_possible_1() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("fn main() {}".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
            Editor(DeleteWithMovement(Right)),
            Expect(CurrentComponentContent("main() {}")),
            Expect(CurrentSelectedTexts(&["main"])),
        ])
    })
}

#[test]
/// No gap between current and next selection
fn delete_should_kill_if_possible_2() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("fn main() {}".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Character)),
            Editor(DeleteWithMovement(Right)),
            Expect(CurrentComponentContent("n main() {}")),
            Expect(CurrentSelectedTexts(&["n"])),
        ])
    })
}

#[test]
/// No next selection
fn delete_should_kill_if_possible_3() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("fn main() {}".to_string())),
            Editor(MatchLiteral("}".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
            Editor(DeleteWithMovement(Next)),
            Expect(CurrentComponentContent("fn main() {")),
        ])
    })
}

#[test]
/// The selection mode is contiguous
fn delete_should_kill_if_possible_4() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("fn main(a:A,b:B) {}".to_string())),
            Editor(MatchLiteral("a:A".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, SyntaxNode)),
            Editor(DeleteWithMovement(Right)),
            Expect(CurrentComponentContent("fn main(b:B) {}")),
            Expect(CurrentSelectedTexts(&["b:B"])),
        ])
    })
}

#[test]
/// Should delete backward if current selection is the last selection in the current selection mode
fn delete_should_kill_if_possible_5() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("fn main(a:A,b:B) {}".to_string())),
            Editor(MatchLiteral("b:B".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, SyntaxNode)),
            Editor(DeleteWithMovement(Right)),
            Expect(CurrentComponentContent("fn main(a:A) {}")),
            Expect(CurrentSelectedTexts(&["a:A"])),
        ])
    })
}

#[test]
/// If the current selection is the only selection in the selection mode
fn delete_should_not_kill_if_not_possible() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("fn main(a:A) {}".to_string())),
            Editor(MatchLiteral("a:A".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, SyntaxNode)),
            Editor(DeleteWithMovement(Right)),
            Expect(CurrentComponentContent("fn main() {}")),
            Expect(CurrentSelectedTexts(&[")"])),
        ])
    })
}

#[test]
fn toggle_untoggle_mark() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("foo bar spam".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Subword)),
            App(MarkFileAndToggleMark),
            Editor(MoveSelection(Right)),
            Editor(MoveSelection(Right)),
            App(MarkFileAndToggleMark),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Mark)),
            Editor(CursorAddToAllSelections),
            Expect(CurrentSelectedTexts(&["foo", "spam"])),
            Editor(CursorKeepPrimaryOnly),
            Expect(CurrentSelectedTexts(&["spam"])),
            App(MarkFileAndToggleMark),
            Editor(MoveSelection(Current(IfCurrentNotFound::LookForward))),
            Editor(CursorAddToAllSelections),
            Expect(CurrentSelectedTexts(&["foo"])),
        ])
    })
}

#[test]
fn test_delete_word_short_backward_from_end_of_file() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(
                "fn snake_case(camelCase: String) {}".to_string(),
            )),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Line)),
            // Go to the end of the file
            Editor(EnterInsertMode(Direction::End)),
            Editor(DeleteWordBackward { short: true }),
            Expect(CurrentComponentContent(
                "fn snake_case(camelCase: String) {",
            )),
            Editor(DeleteWordBackward { short: true }),
            Expect(CurrentComponentContent("fn snake_case(camelCase: String) ")),
            Editor(DeleteWordBackward { short: true }),
            Expect(CurrentComponentContent("fn snake_case(camelCase: String")),
            Editor(DeleteWordBackward { short: true }),
            Expect(CurrentComponentContent("fn snake_case(camelCase: ")),
        ])
    })
}

#[test]
fn test_delete_word_long() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("hello_world itsMe".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Line)),
            // Go to the end of the file
            Editor(EnterInsertMode(Direction::End)),
            Editor(DeleteWordBackward { short: false }),
            Expect(CurrentComponentContent("hello_world ")),
            Editor(DeleteWordBackward { short: false }),
            Expect(CurrentComponentContent("")),
        ])
    })
}

#[test]
fn test_delete_extended_selection_forward() -> anyhow::Result<()> {
    execute_test(move |s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("who lives in a pineapple".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Subword)),
            Editor(MoveSelection(Right)),
            Editor(EnableSelectionExtension),
            Editor(MoveSelection(Right)),
            Expect(CurrentSelectedTexts(&["lives in"])),
            Editor(DeleteWithMovement(Right)),
            Expect(CurrentComponentContent("who a pineapple")),
            Expect(CurrentSelectedTexts(&["a"])),
        ])
    })
}

#[test]
fn test_delete_extended_selection_backward() -> anyhow::Result<()> {
    execute_test(move |s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("who lives in a pineapple".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Subword)),
            Editor(MoveSelection(Right)),
            Editor(EnableSelectionExtension),
            Editor(MoveSelection(Right)),
            Expect(CurrentSelectedTexts(&["lives in"])),
            Editor(DeleteWithMovement(Left)),
            Expect(CurrentComponentContent("who a pineapple")),
            Expect(CurrentSelectedTexts(&["who"])),
        ])
    })
}

#[test]
fn extend_jump() -> anyhow::Result<()> {
    execute_test(move |s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("apple banana cake durian egg".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
            Editor(MoveSelection(Right)),
            Expect(CurrentSelectedTexts(&["banana"])),
            Editor(EnableSelectionExtension),
            Editor(SetRectangle(Rectangle {
                origin: Position::default(),
                width: 100,
                height: 1,
            })),
            Editor(ShowJumps {
                use_current_selection_mode: true,
                prior_change: None,
            }),
            App(HandleKeyEvents(keys!("d").to_vec())),
            Expect(CurrentSelectedTexts(&["banana cake durian"])),
            Editor(MoveSelection(Right)),
            Expect(CurrentSelectedTexts(&["banana cake durian egg"])),
        ])
    })
}

#[test]
fn test_delete_extended_selection_is_last_selection() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("who lives in".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
            Editor(MoveSelection(Right)),
            Editor(EnableSelectionExtension),
            Editor(MoveSelection(Right)),
            Expect(CurrentSelectedTexts(&["lives in"])),
            Editor(DeleteWithMovement(Right)),
            Expect(CurrentComponentContent("who")),
            Expect(CurrentSelectedTexts(&["who"])),
        ])
    })
}

#[test]
fn test_delete_extended_selection_is_first_selection() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("who lives in".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Subword)),
            Editor(EnableSelectionExtension),
            Editor(MoveSelection(Right)),
            Expect(CurrentSelectedTexts(&["who lives"])),
            Editor(DeleteWithMovement(Right)),
            Expect(CurrentComponentContent("in")),
            Expect(CurrentSelectedTexts(&["in"])),
        ])
    })
}

#[test]
fn test_delete_extended_selection_whole_file() -> anyhow::Result<()> {
    execute_test(move |s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("who lives\nin a\npineapple".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Line)),
            Editor(MoveSelection(Right)),
            Editor(SelectAll),
            Editor(DeleteWithMovement(Right)),
            Expect(CurrentComponentContent("")),
            Expect(CurrentSelectedTexts(&[""])),
        ])
    })
}

#[test]
fn test_delete_word_short_backward_from_middle_of_file() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(
                "fn snake_case(camelCase: String) {}".to_string(),
            )),
            Editor(MatchLiteral("camelCase".to_string())),
            Expect(CurrentSelectedTexts(&["camelCase"])),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
            Editor(EnterInsertMode(Direction::End)),
            Editor(DeleteWordBackward { short: true }),
            Expect(CurrentComponentContent("fn snake_case(camel: String) {}")),
            Editor(DeleteWordBackward { short: true }),
            Expect(CurrentComponentContent("fn snake_case(: String) {}")),
            Editor(DeleteWordBackward { short: true }),
            Expect(CurrentComponentContent("fn snake_case: String) {}")),
            Editor(DeleteWordBackward { short: true }),
            Expect(CurrentComponentContent("fn snake_: String) {}")),
            Editor(DeleteWordBackward { short: true }),
            Expect(CurrentComponentContent("fn snake: String) {}")),
            Editor(DeleteWordBackward { short: true }),
            Expect(CurrentComponentContent("fn : String) {}")),
            Editor(DeleteWordBackward { short: true }),
            Expect(CurrentComponentContent(": String) {}")),
            Editor(DeleteWordBackward { short: true }),
            Expect(CurrentComponentContent(": String) {}")),
        ])
    })
}

#[test]
fn test_pipe_to_shell_1() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("snake_case".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
            Editor(PipeToShell {
                command: "tr '_' ' '".to_string(),
            }),
            Expect(CurrentComponentContent("snake case")),
            Expect(CurrentSelectedTexts(&["snake case"])),
        ])
    })
}

#[test]
fn kill_line_to_end() -> anyhow::Result<()> {
    let input = "lala\nfoo bar spam\nyoyo";
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(input.to_string())),
            // Killing to the end of line WITH trailing newline character
            Editor(MatchLiteral("bar".to_string())),
            Editor(KillLine(Direction::End)),
            Editor(Insert("sparta".to_string())),
            Expect(CurrentComponentContent("lala\nfoo sparta\nyoyo")),
            Expect(CurrentMode(Mode::Insert)),
            Expect(CurrentSelectedTexts(&[""])),
            // Remove newline character if the character after cursor is a newline character
            Editor(KillLine(Direction::End)),
            Expect(CurrentComponentContent("lala\nfoo spartayoyo")),
            // Killing to the end of line WITHOUT trailing newline character
            Editor(KillLine(Direction::End)),
            Expect(CurrentComponentContent("lala\nfoo sparta")),
        ])
    })
}

#[test]
fn kill_line_to_start() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("lala\nfoo bar spam\nyoyo".to_string())),
            // Killing to the start of line WITH leading newline character
            Editor(MatchLiteral("bar".to_string())),
            Editor(KillLine(Direction::Start)),
            Editor(Insert("sparta".to_string())),
            Expect(CurrentComponentContent("lala\nspartabar spam\nyoyo")),
            Expect(CurrentMode(Mode::Insert)),
            Editor(KillLine(Direction::Start)),
            Expect(CurrentComponentContent("lala\nbar spam\nyoyo")),
            // Remove newline character if the character before cursor is a newline character
            Editor(KillLine(Direction::Start)),
            Expect(CurrentComponentContent("lalabar spam\nyoyo")),
            Expect(EditorCursorPosition(Position { line: 0, column: 4 })),
            // Killing to the start of line WITHOUT leading newline character
            Editor(KillLine(Direction::Start)),
            Expect(CurrentComponentContent("bar spam\nyoyo")),
        ])
    })
}

#[test]
fn multi_swap_sibling() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("fn f(x:a,y:b){} fn g(x:a,y:b){}".to_string())),
            Editor(MatchLiteral("fn f(x:a,y:b){}".to_string())),
            Expect(CurrentSelectedTexts(&["fn f(x:a,y:b){}"])),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, SyntaxNode)),
            Editor(CursorAddToAllSelections),
            Expect(CurrentSelectedTexts(&[
                "fn f(x:a,y:b){}",
                "fn g(x:a,y:b){}",
            ])),
            Editor(MoveSelection(Down)),
            Editor(MoveSelection(Right)),
            Editor(MoveSelection(Down)),
            Expect(CurrentSelectedTexts(&["x:a", "x:a"])),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, SyntaxNode)),
            Editor(EnterSwapMode),
            Editor(MoveSelection(Right)),
            Expect(CurrentComponentContent("fn f(y:b,x:a){} fn g(y:b,x:a){}")),
            Expect(CurrentSelectedTexts(&["x:a", "x:a"])),
            Editor(MoveSelection(Left)),
            Expect(CurrentComponentContent("fn f(x:a,y:b){} fn g(x:a,y:b){}")),
        ])
    })
}

#[test]
fn update_mark_position() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("foo bar spim".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Subword)),
            Editor(MoveSelection(Right)),
            Editor(MoveSelection(Right)),
            App(MarkFileAndToggleMark),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Mark)),
            Expect(CurrentSelectedTexts(&["spim"])),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Subword)),
            Editor(MoveSelection(Left)),
            Editor(MoveSelection(Left)),
            // Kill "foo"
            Editor(DeleteWithMovement(Right)),
            Expect(CurrentComponentContent("bar spim")),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Mark)),
            // Expect mark position is updated, and still selects "spim"
            Expect(CurrentSelectedTexts(&["spim"])),
            // Remove "spim"
            Editor(Change),
            Expect(CurrentComponentContent("bar ")),
            Editor(EnterNormalMode),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Subword)),
            Expect(CurrentSelectedTexts(&["bar"])),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Mark)),
            // Expect the "spim" mark is removed
            // By the fact that "bar" is still selected
            Expect(CurrentSelectedTexts(&["bar"])),
        ])
    })
}

#[test]
fn move_to_line_start_end() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("hello\nnext line".to_string())),
            Editor(EnterInsertMode(Direction::Start)),
            Editor(MoveToLineEnd),
            Editor(Insert(" world".to_string())),
            Expect(CurrentComponentContent("hello world\nnext line")),
            Editor(MoveToLineStart),
            Editor(Insert("hey ".to_string())),
            Expect(CurrentComponentContent("hey hello world\nnext line")),
        ])
    })
}

#[test]
fn swap_sibling() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("fn main(x: usize, y: Vec<A>) {}".to_string())),
            // Select first statement
            Editor(MatchLiteral("x: usize".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, SyntaxNode)),
            Editor(EnterSwapMode),
            Editor(MoveSelection(Right)),
            Expect(CurrentComponentContent("fn main(y: Vec<A>, x: usize) {}")),
            Editor(MoveSelection(Left)),
            Expect(CurrentComponentContent("fn main(x: usize, y: Vec<A>) {}")),
        ])
    })
}

#[test]
fn swap_sibling_2() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("use a;\nuse b;\nuse c;".to_string())),
            // Select first statement
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, SyntaxNode)),
            Expect(CurrentSelectedTexts(&["use a;"])),
            Editor(EnterSwapMode),
            Editor(MoveSelection(Right)),
            Expect(CurrentComponentContent("use b;\nuse a;\nuse c;")),
            Editor(MoveSelection(Right)),
            Expect(CurrentComponentContent("use b;\nuse c;\nuse a;")),
        ])
    })
}

#[test]
fn select_character() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("fn main() { let x = 1; }".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Character)),
            Expect(CurrentSelectedTexts(&["f"])),
            Editor(MoveSelection(Right)),
            Expect(CurrentSelectedTexts(&["n"])),
            Editor(MoveSelection(Left)),
            Expect(CurrentSelectedTexts(&["f"])),
        ])
    })
}

#[test]
fn raise() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("fn main() { let x = a.b(c()); }".to_string())),
            Editor(MatchLiteral("c()".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, SyntaxNode)),
            Editor(Replace(Expand)),
            Expect(CurrentComponentContent("fn main() { let x = c(); }")),
            Editor(Replace(Expand)),
            Expect(CurrentComponentContent("fn main() { c() }")),
        ])
    })
}

#[test]
/// After raise the node kind should be the same
/// Raising `(a).into()` in `Some((a).into())`
/// should result in `(a).into()`
/// not `Some(a).into()`
fn raise_preserve_current_node_structure() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("fn main() { Some((a).b()) }".to_string())),
            Editor(MatchLiteral("(a).b()".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, SyntaxNode)),
            Editor(Replace(Expand)),
            Expect(CurrentComponentContent("fn main() { (a).b() }")),
        ])
    })
}

#[test]
fn multi_raise() -> anyhow::Result<()> {
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
            Editor(MatchLiteral("let x = S(a);".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, SyntaxNode)),
            Editor(CursorAddToAllSelections),
            Editor(MoveSelection(Down)),
            Editor(MoveSelection(Right)),
            Editor(MoveSelection(Down)),
            Editor(MoveSelection(Right)),
            Editor(MoveSelection(Down)),
            Editor(MoveSelection(Right)),
            Expect(CurrentSelectedTexts(&["a", "b"])),
            Editor(Replace(Expand)),
            Expect(CurrentComponentContent("fn f(){ let x = a; let y = b; }")),
            Editor(Undo),
            Expect(CurrentComponentContent(
                "fn f(){ let x = S(a); let y = S(b); }",
            )),
            Expect(CurrentSelectedTexts(&["a", "b"])),
            Editor(Redo),
            Expect(CurrentComponentContent("fn f(){ let x = a; let y = b; }")),
            Expect(CurrentSelectedTexts(&["a", "b"])),
        ])
    })
}

#[test]
fn open_before_selection() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("fn x(a:A, b:B){}".trim().to_string())),
            Editor(MatchLiteral("a:A".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, SyntaxNode)),
            Editor(SwapCursor),
            Editor(Open),
            Expect(CurrentMode(Mode::Insert)),
            Editor(Insert("c:C".to_string())),
            Expect(CurrentComponentContent("fn x(c:C, a:A, b:B){}".trim())),
            Editor(EnterNormalMode),
            Editor(MatchLiteral("b:B".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, SyntaxNode)),
            Editor(SwapCursor),
            Editor(Open),
            Editor(Insert("d:D".to_string())),
            Expect(CurrentComponentContent("fn x(c:C, a:A, d:D, b:B){}".trim())),
        ])
    })
}

#[test]
fn open_before_use_min_gap() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(
                "
def main():
  hello
    world
"
                .trim()
                .to_string(),
            )),
            Editor(MatchLiteral("hello".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Line)),
            Editor(SwapCursor),
            Editor(Open),
            Expect(CurrentComponentContent(
                "
def main():
  
  hello
    world
"
                .trim(),
            )),
        ])
    })
}

#[test]
fn open_after_selection() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("fn x(a:A, b:B){}".trim().to_string())),
            Editor(MatchLiteral("a:A".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, SyntaxNode)),
            Editor(Open),
            Expect(CurrentMode(Mode::Insert)),
            Editor(Insert("c:C".to_string())),
            Expect(CurrentComponentContent("fn x(a:A, c:C, b:B){}".trim())),
            Editor(EnterNormalMode),
            Editor(MatchLiteral("b:B".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, SyntaxNode)),
            Expect(CurrentSelectedTexts(&["b:B"])),
            Editor(Open),
            Editor(Insert("d:D".to_string())),
            Expect(CurrentComponentContent("fn x(a:A, c:C, b:B, d:D){}".trim())),
        ])
    })
}

#[test]
fn open_after_use_max_gap() -> anyhow::Result<()> {
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
  // hello
}
"
                .trim()
                .to_string(),
            )),
            Editor(MatchLiteral("hello".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Line)),
            Expect(CurrentSelectedTexts(&["// hello"])),
            Editor(Open),
            Editor(Insert("// world".to_string())),
            Expect(CurrentComponentContent(
                "
fn main() {
  // hello
  // world
}
"
                .trim(),
            )),
        ])
    })
}

#[test]
fn open_max_gap_contains_at_most_one_newline_character() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(
                "
foo
    
    bar

spam
"
                .trim()
                .to_string(),
            )),
            Editor(MatchLiteral("bar".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Line)),
            Editor(Open),
            Editor(Insert("world".to_string())),
            Expect(CurrentComponentContent(
                "
foo
    
    bar
    world

spam"
                    .trim(),
            )),
        ])
    })
}

#[test]
fn test_copy_current_file_path() -> anyhow::Result<()> {
    execute_test(|s| {
        // Multiline source code
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Expect(Not(Box::new(CurrentComponentContentMatches(regex!(
                "main.rs"
            ))))),
            Editor(CopyAbsolutePath),
            Editor(Paste),
            Expect(CurrentComponentContentMatches(regex!("main.rs"))),
        ])
    })
}

#[test]
fn swap_line() -> anyhow::Result<()> {
    execute_test(|s| {
        // Multiline source code
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(
                "
fn main() {
    let x = 1;
    let y = 2;
}"
                .trim()
                .to_string()
                .clone(),
            )),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Line)),
            Expect(CurrentComponentContent(
                "
fn main() {
    let x = 1;
    let y = 2;
}"
                .trim(),
            )),
            Editor(EnterSwapMode),
            Editor(MoveSelection(Right)),
            Expect(CurrentComponentContent(
                "
let x = 1;
    fn main() {
    let y = 2;
}"
                .trim(),
            )),
            Editor(MoveSelection(Left)),
            Expect(CurrentComponentContent(
                "
fn main() {
    let x = 1;
    let y = 2;
}"
                .trim(),
            )),
        ])
    })
}

#[test]
fn swap_character() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("fn main() { let x = 1; }".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Character)),
            Editor(EnterSwapMode),
            Editor(MoveSelection(Right)),
            Expect(CurrentComponentContent("nf main() { let x = 1; }")),
            Editor(MoveSelection(Right)),
            Expect(CurrentComponentContent("n fmain() { let x = 1; }")),
            Editor(MoveSelection(Left)),
            Expect(CurrentComponentContent("nf main() { let x = 1; }")),
            Editor(MoveSelection(Left)),
            Expect(CurrentComponentContent("fn main() { let x = 1; }")),
        ])
    })
}

#[test]
fn multi_insert() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("struct A(usize, char)".to_string())),
            Editor(MatchLiteral("usize".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, SyntaxNode)),
            Editor(CursorAddToAllSelections),
            Expect(CurrentSelectedTexts(&["usize", "char"])),
            Editor(EnterInsertMode(Direction::Start)),
            Editor(Insert("pub ".to_string())),
            Expect(CurrentComponentContent("struct A(pub usize, pub char)")),
            Editor(Backspace),
            Expect(CurrentComponentContent("struct A(pubusize, pubchar)")),
            Expect(CurrentSelectedTexts(&["", ""])),
        ])
    })
}

#[serial]
#[test]
fn paste_in_insert_mode_1() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("foo bar spam".to_string())),
            App(SetClipboardContent {
                copied_texts: CopiedTexts::one("haha".to_string()),
            }),
            Editor(MatchLiteral("bar".to_string())),
            Editor(EnterInsertMode(Direction::End)),
            Editor(Paste),
            Expect(CurrentComponentContent("foo barhaha spam")),
            Editor(Insert("Hello".to_string())),
            Expect(CurrentComponentContent("foo barhahaHello spam")),
        ])
    })
}

#[serial]
#[test]
fn paste_in_insert_mode_2() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("fn main(a:A,b:B){}".to_string())),
            Editor(MatchLiteral("a:A".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, SyntaxNode)),
            Editor(Copy),
            Editor(EnterInsertMode(Direction::End)),
            Editor(Paste),
            Expect(CurrentComponentContent("fn main(a:Aa:A,b:B){}")),
            Editor(Insert("Hello".to_string())),
            Expect(CurrentComponentContent("fn main(a:Aa:AHello,b:B){}")),
        ])
    })
}

#[serial]
#[test]
fn paste_after() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("foo bar spam".to_string())),
            App(SetClipboardContent {
                copied_texts: CopiedTexts::one("haha".to_string()),
            }),
            Editor(MatchLiteral("bar".to_string())),
            Editor(Paste),
            Expect(CurrentComponentContent("foo barhaha spam")),
            Expect(CurrentSelectedTexts(&["haha"])),
        ])
    })
}

#[serial]
#[test]
fn paste_after_line() -> anyhow::Result<()> {
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
}"
                .trim()
                .to_string(),
            )),
            Editor(MatchLiteral("bar();".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Line)),
            Editor(Copy),
            Editor(Paste),
            Expect(CurrentComponentContent(
                "fn main() {
    foo();
    bar();
    bar();
}",
            )),
        ])
    })
}

#[serial]
#[test]
fn smart_paste_forward() -> anyhow::Result<()> {
    execute_test(move |s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("fn main(a:A, b:B) {}".to_string())),
            App(SetClipboardContent {
                copied_texts: CopiedTexts::one("c:C".to_string()),
            }),
            Editor(MatchLiteral("a:A".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, SyntaxNode)),
            Expect(CurrentSelectedTexts(&["a:A"])),
            Editor(Paste),
            Expect(CurrentComponentContent("fn main(a:A, c:C, b:B) {}")),
            Expect(CurrentSelectedTexts(&["c:C"])),
        ])
    })
}

#[serial]
#[test]
fn paste_no_gap() -> anyhow::Result<()> {
    execute_test(move |s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("foo\nbar".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Line)),
            Editor(Copy),
            Editor(PasteNoGap),
            Expect(CurrentComponentContent("foofoo\nbar")),
        ])
    })
}

#[serial]
#[test]
fn smart_paste_backward() -> anyhow::Result<()> {
    execute_test(move |s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("fn main(a:A, b:B) {}".to_string())),
            App(SetClipboardContent {
                copied_texts: CopiedTexts::one("c:C".to_string()),
            }),
            Editor(MatchLiteral("a:A".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, SyntaxNode)),
            Expect(CurrentSelectedTexts(&["a:A"])),
            Editor(SwapCursor),
            Editor(Paste),
            Expect(CurrentComponentContent("fn main(c:C, a:A, b:B) {}")),
            Expect(CurrentSelectedTexts(&["c:C"])),
        ])
    })
}

#[serial]
#[test]
fn paste_before() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("foo bar spam".to_string())),
            App(SetClipboardContent {
                copied_texts: CopiedTexts::one("haha".to_string()),
            }),
            Editor(MatchLiteral("bar".to_string())),
            Editor(SwapCursor),
            Editor(Paste),
            Expect(CurrentComponentContent("foo hahabar spam")),
            Expect(CurrentSelectedTexts(&["haha"])),
        ])
    })
}

#[serial]
#[test]
fn replace_from_clipboard() -> anyhow::Result<()> {
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
            App(SetClipboardContent {
                copied_texts: CopiedTexts::one("let z = S(c);".to_string()),
            }),
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                SelectionMode::Word,
            )),
            Editor(ReplaceWithCopiedText { cut: false }),
            Expect(CurrentComponentContent(
                "let z = S(c); f(){ let x = S(a); let y = S(b); }",
            )),
        ])
    })
}

#[test]
fn enter_newline() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("".to_string())),
            Editor(EnterInsertMode(Direction::Start)),
            Editor(Insert("hello".to_string())),
            App(HandleKeyEvent(key!("enter"))),
            Editor(Insert("world".to_string())),
            Expect(CurrentComponentContent("hello\nworld")),
            App(HandleKeyEvent(key!("left"))),
            App(HandleKeyEvent(key!("enter"))),
            Expect(CurrentComponentContent("hello\nworl\nd")),
        ])
    })
}

#[test]
fn insert_mode_start() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("fn main() {}".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Subword)),
            Editor(EnterInsertMode(Direction::Start)),
            Editor(Insert("hello".to_string())),
            Expect(CurrentComponentContent("hellofn main() {}")),
        ])
    })
}

#[test]
fn insert_mode_end() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("fn main() {}".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Subword)),
            Editor(EnterInsertMode(Direction::End)),
            Editor(Insert("hello".to_string())),
            Expect(CurrentComponentContent("fnhello main() {}")),
        ])
    })
}

#[test]
fn delete_extended_selection() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("fn main() {}".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
            Editor(EnableSelectionExtension),
            Editor(MoveSelection(Right)),
            Expect(CurrentSelectedTexts(&["fn main"])),
            Editor(DeleteWithMovement(Next)),
            Expect(CurrentSelectedTexts(&["("])),
        ])
    })
}

#[test]
fn delete_extended_selection_2() -> anyhow::Result<()> {
    execute_test(move |s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(
                "
fn main() {
    foo
       .bar(
           spam
       );
}
"
                .to_string(),
            )),
            Editor(MatchLiteral("spam".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, SyntaxNode)),
            Editor(MoveSelection(Up)),
            Editor(EnableSelectionExtension),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Line)),
            Expect(CurrentSelectedTexts(&[".bar(\n           spam\n       )"])),
            Editor(DeleteWithMovement(Right)),
            Expect(CurrentSelectedTexts(&[""])),
            Expect(CurrentComponentContent(
                "
fn main() {
    foo
       ;
}
",
            )),
        ])
    })
}

#[test]
fn multicursor_add_all() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(
                "mod m { fn a(j:J){} fn b(k:K,l:L){} fn c(m:M,n:N,o:O){} }".to_string(),
            )),
            Editor(MatchLiteral("fn a".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, SyntaxNode)),
            Expect(CurrentSelectedTexts(&["fn a(j:J){}"])),
            Editor(CursorAddToAllSelections),
            Editor(MoveSelection(Down)),
            Editor(MoveSelection(Right)),
            Editor(MoveSelection(Down)),
            Expect(CurrentSelectedTexts(&["j:J", "k:K", "m:M"])),
            Editor(CursorAddToAllSelections),
            Expect(CurrentSelectedTexts(&[
                "j:J", "k:K", "l:L", "m:M", "n:N", "o:O",
            ])),
        ])
    })
}

#[test]
fn enter_normal_mode_should_highlight_one_character() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(
                "fn\nmain()\n{ x.y(); x.y(); x.y(); }".to_string(),
            )),
            Editor(MatchLiteral("x.y()".to_string())),
            Editor(EnterInsertMode(Direction::End)),
            Editor(EnterNormalMode),
            Expect(CurrentSelectedTexts(&[")"])),
        ])
    })
}

#[test]
fn change_extended_selection() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("hello world yo".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Subword)),
            Editor(EnableSelectionExtension),
            Editor(MoveSelection(Right)),
            Expect(CurrentSelectedTexts(&["hello world"])),
            Editor(Change),
            Editor(Insert("wow".to_string())),
            Expect(CurrentSelectedTexts(&[""])),
            Expect(CurrentComponentContent("wow yo")),
        ])
    })
}

#[test]
fn scroll_page() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("1\n2 hey\n3".to_string())),
            Editor(SetRectangle(Rectangle {
                origin: Position::default(),
                width: 100,
                height: 3,
            })),
            Editor(ScrollPageDown),
            Expect(CurrentLine("2 hey")),
            Editor(ScrollPageDown),
            Editor(MatchLiteral("hey".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
            Expect(CurrentSelectedTexts(&["hey"])),
            Editor(ScrollPageDown),
            Expect(CurrentLine("3")),
            Expect(CurrentSelectedTexts(&["3"])),
            Editor(ScrollPageDown),
            Expect(CurrentLine("3")),
            Expect(CurrentSelectedTexts(&["3"])),
            Editor(ScrollPageUp),
            Expect(CurrentLine("2 hey")),
            Expect(CurrentSelectedTexts(&["2"])),
            Editor(ScrollPageUp),
            Expect(CurrentLine("1")),
            Expect(CurrentSelectedTexts(&["1"])),
            Editor(ScrollPageUp),
            Expect(CurrentLine("1")),
            Expect(CurrentSelectedTexts(&["1"])),
            Expect(CurrentSelectionMode(Word)),
        ])
    })
}

#[test]
fn scroll_offset() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("alpha\nbeta\ngamma\nlok".to_string())),
            Editor(SetRectangle(Rectangle {
                origin: Position::default(),
                width: 100,
                height: 3,
            })),
            Editor(MatchLiteral("gamma".to_string())),
            Editor(SetScrollOffset(2)),
            Expect(EditorGrid("🦀  main.rs [*]\n3│█amma\n4│lok")),
        ])
    })
}

#[test]
fn jump() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(
                "Who lives on sea shore?\n yonky donkey".to_string(),
            )),
            Editor(SetRectangle(Rectangle {
                origin: Position::default(),
                width: 100,
                height: 1,
            })),
            // In jump mode, the first stage labels each selection using their starting character,
            // On subsequent stages, the labels are random alphabets
            Expect(JumpChars(&[])),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Subword)),
            Editor(ShowJumps {
                use_current_selection_mode: false,
                prior_change: None,
            }),
            // Expect the jump to be the first character of each subword
            // Note 'y' and 'd' are excluded because they are out of view,
            // since the viewbox has only height of 1
            Expect(JumpChars(&['w', 'l', 'o', 's', 's', '?'])),
            App(HandleKeyEvent(key!("s"))),
            Expect(JumpChars(&['d', 'k'])),
            App(HandleKeyEvent(key!("d"))),
            Expect(JumpChars(&[])),
            Expect(CurrentSelectedTexts(&["sea"])),
        ])
    })
}

#[test]
fn jump_to_hidden_parent_lines() -> anyhow::Result<()> {
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
  alpha()
  beta()
}
"
                .trim()
                .to_string(),
            )),
            Editor(SetRectangle(Rectangle {
                origin: Position::default(),
                width: 14,
                height: 4,
            })),
            Editor(MatchLiteral("beta".to_string())),
            Editor(SetRectangle(Rectangle {
                origin: Position::default(),
                width: 21,
                height: 4,
            })),
            Editor(SwitchViewAlignment),
            // The "long" of "too long" is not shown, because it exceeded the view width
            Expect(EditorGrid(
                "
🦀  main.rs [*]
1│fn main() {
3│  █eta()
4│}
"
                .trim(),
            )),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Line)),
            Expect(CurrentSelectedTexts(&["beta()"])),
            Editor(ShowJumps {
                use_current_selection_mode: true,
                prior_change: None,
            }),
            Expect(JumpChars(&['b', 'f', '}'])),
            App(HandleKeyEvent(key!("f"))),
            Expect(CurrentSelectedTexts(&["fn main() {"])),
        ])
    })
}

#[test]
fn extend_and_jump() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(
                "Who lives on sea shore?\n yonky donkey".to_string(),
            )),
            Editor(SetRectangle(Rectangle {
                origin: Position::default(),
                width: 100,
                height: 1,
            })),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Subword)),
            Editor(MoveSelection(Right)),
            Expect(CurrentSelectedTexts(&["lives"])),
            Editor(EnableSelectionExtension),
            Editor(ShowJumps {
                use_current_selection_mode: false,
                prior_change: None,
            }),
            // Expect the jump to be the first character of each word
            // Note 'y' and 'd' are excluded because they are out of view,
            // since the viewbox has only height of 1
            Expect(JumpChars(&['w', 'l', 'o', 's', 's', '?'])),
            App(HandleKeyEvent(key!("s"))),
            App(HandleKeyEvent(key!("k"))),
            Expect(CurrentSelectedTexts(&["lives on sea shore"])),
        ])
    })
}

#[test]
fn jump_all_selection_start_with_same_char() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("who who who who".to_string())),
            Editor(SetRectangle(Rectangle {
                origin: Position::default(),
                width: 100,
                height: 1,
            })),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Subword)),
            Editor(ShowJumps {
                use_current_selection_mode: false,
                prior_change: None,
            }),
            // Expect the jump to NOT be the first character of each word
            // Since, the first character of each selection are the same, which is 'w'
            Expect(JumpChars(&['d', 'k', 's', 'l'])),
        ])
    })
}

#[test]
fn switch_view_alignment() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(
                "abcde"
                    .split("")
                    .collect_vec()
                    .join("\n")
                    .trim()
                    .to_string(),
            )),
            Editor(SetRectangle(Rectangle {
                origin: Position::default(),
                width: 100,
                height: 4,
            })),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Subword)),
            Editor(MoveSelection(Right)),
            Editor(MoveSelection(Right)),
            Expect(CurrentSelectedTexts(&["c"])),
            Expect(CurrentViewAlignment(None)),
            Editor(SwitchViewAlignment),
            Expect(CurrentViewAlignment(Some(ViewAlignment::Top))),
            Editor(SwitchViewAlignment),
            Expect(CurrentViewAlignment(Some(ViewAlignment::Center))),
            Editor(SwitchViewAlignment),
            Expect(CurrentViewAlignment(Some(ViewAlignment::Bottom))),
            Editor(MoveSelection(Left)),
            Expect(CurrentViewAlignment(None)),
        ])
    })
}

#[test]
fn get_grid_parent_line() -> anyhow::Result<()> {
    let parent_lines_background = hex!("#badbad");
    let mark_background_color = hex!("#cebceb");
    let theme = {
        let mut theme = Theme::default();
        theme.ui.parent_lines_background = parent_lines_background;
        theme.ui.mark = Style::default().background_color(mark_background_color);
        theme
    };
    let width = 20;
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(
                "
// hello
fn main() {
  let x = 1;
  let y = 2; // too long, wrapped
  for a in b {
    let z = 4;
    print()
  }
}
"
                .trim()
                .to_string(),
            )),
            Editor(SetRectangle(Rectangle {
                origin: Position::default(),
                width,
                height: 7,
            })),
            App(SetTheme(theme.clone())),
            // Go to "print()" and skip the first 3 lines for rendering
            Editor(MatchLiteral("print()".to_string())),
            Editor(SetScrollOffset(3)),
            // Expect `fn main()` is visible although it is out of view,
            // because it is amongst the parent lines of the current selection
            Expect(EditorGrid(
                "
🦀  main.rs [*]
2│fn main() {
4│  let y = 2; //
↪│too long, wrapped
5│  for a in b {
6│    let z = 4;
7│    █rint()
"
                .trim(),
            )),
            // Bookmart "z"
            Editor(MatchLiteral("z".to_string())),
            App(MarkFileAndToggleMark),
            // Expect the parent lines of the current selections are highlighted with parent_lines_background,
            // regardless of whether the parent lines are inbound or outbound
            ExpectMulti(
                // Line 1 = `fn main() {`
                // Line 4 = `for a in b`
                [(1, 12), (4, 15)]
                    .into_iter()
                    .flat_map(|(row_index, max_column)| {
                        let line_number_ui_width = 2;
                        (0..max_column).map(move |column_index| {
                            GridCellBackground(
                                row_index,
                                column_index + line_number_ui_width as usize,
                                parent_lines_background,
                            )
                        })
                    })
                    .collect(),
            ),
            // Expect the current line is not treated as parent line
            ExpectMulti(
                (0..width - 1)
                    .map(|column_index| {
                        Not(Box::new(GridCellBackground(
                            5,
                            column_index,
                            parent_lines_background,
                        )))
                    })
                    .collect(),
            ),
            // Mark the "fn" word
            Editor(MatchLiteral("fn".to_string())),
            App(MarkFileAndToggleMark),
            // Go to "print()" and skip the first 3 lines for rendering
            Editor(MatchLiteral("print()".to_string())),
            Editor(SetScrollOffset(3)),
            Expect(EditorGrid(
                "
# 🦀  main.rs [*]
2│fn main() {
4│  let y = 2; //
↪│too long, wrapped
5│  for a in b {
6│    let z = 4;
7│    █rint()"
                    .trim(),
            )),
            // Expect the marks of outbound parent lines are rendered properly
            // In this case, the outbound parent line is "fn main() {"
            ExpectMulti(
                [2, 3]
                    .into_iter()
                    .map(|column_index| {
                        GridCellBackground(1, column_index as usize, mark_background_color)
                    })
                    .collect(),
            ),
            // Expect the marks of inbound lines are rendered properly
            // In this case, we want to check that the mark on "z" is rendered
            Expect(GridCellBackground(5, 10, mark_background_color)),
            // Expect no cells of the line `let y = 2` is not decorated with
            // `mark_background_color`
            ExpectMulti(
                (0..12)
                    .map(|column_index| {
                        Not(Box::new(GridCellBackground(
                            2,
                            column_index,
                            mark_background_color,
                        )))
                    })
                    .collect(),
            ),
        ])
    })
}

#[test]
fn test_wrapped_lines() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(
                "
// hellohello worldworld\n heyhey
"
                .trim()
                .to_string(),
            )),
            Editor(SetRectangle(Rectangle {
                origin: Position::default(),
                width: 22,
                height: 4,
            })),
            Editor(MatchLiteral("world".to_string())),
            Editor(EnterInsertMode(Direction::End)),
            Expect(EditorGrid(
                "
🦀  main.rs [*]
1│// hellohello
↪│world█orld
2│ heyhey"
                    .trim(),
            )),
            // Expect the cursor is after 'd'
            Expect(EditorGridCursorPosition(Position { line: 2, column: 7 })),
        ])
    })
}

#[test]
fn diagnostics_range_updated_by_edit() -> anyhow::Result<()> {
    execute_test(|s| {
        let hello = &"hello";
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("fn main() { let x = 123 }".trim().to_string())),
            App(HandleLspNotification(LspNotification::PublishDiagnostics(
                lsp_types::PublishDiagnosticsParams {
                    uri: s.main_rs().to_url().unwrap(),
                    diagnostics: [lsp_types::Diagnostic {
                        range: lsp_types::Range::new(
                            lsp_types::Position {
                                line: 0,
                                character: 3,
                            },
                            lsp_types::Position {
                                line: 0,
                                character: 7,
                            },
                        ),
                        ..Default::default()
                    }]
                    .to_vec(),
                    version: None,
                },
            ))),
            Expect(ExpectKind::DiagnosticsRanges(
                [CharIndexRange::from(CharIndex(3)..CharIndex(7))].to_vec(),
            )),
            Editor(MatchLiteral("fn".to_string())),
            Editor(EnterInsertMode(Direction::Start)),
            Editor(Insert(hello.to_string())),
            Expect(ExpectKind::DiagnosticsRanges(
                [CharIndexRange::from(
                    CharIndex(3 + hello.len())..CharIndex(7 + hello.len()),
                )]
                .to_vec(),
            )),
        ])
    })
}

#[test]
fn quickfix_list_items_updated_by_edit() -> anyhow::Result<()> {
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
                        None,
                        None,
                    )]
                    .to_vec(),
                ),
            )),
            Expect(ExpectKind::BufferQuickfixListItems(
                [(CharIndex(2)..CharIndex(5)).into()].to_vec(),
            )),
            // Testing edit that affects the line of the quickfix item
            Editor(MatchLiteral("fn".to_string())),
            Editor(EnterInsertMode(Direction::Start)),
            Editor(Insert("hello".to_string())),
            // The position range should be updated
            Expect(ExpectKind::BufferQuickfixListItems(
                [(CharIndex(7)..CharIndex(10)).into()].to_vec(),
            )),
        ])
    })
}

#[test]
fn syntax_highlight_spans_updated_by_edit() -> anyhow::Result<()> {
    execute_test(|s| {
        let theme = Theme::default();
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            App(SetTheme(theme.clone())),
            Editor(SetContent(
                "
/* hello */ fn main() { let x = 123 }
// range of highlight spans of this line should not be updated as they are out of visible range
                "
                .trim()
                .to_string(),
            )),
            Editor(SetLanguage(Box::new(
                crate::config::from_extension("rs").unwrap(),
            ))),
            Editor(SetRectangle(Rectangle {
                origin: Position::default(),
                width: 100,
                height: 2,
            })),
            Editor(ApplySyntaxHighlight),
            Expect(ExpectKind::HighlightSpans(
                0..11,
                StyleKey::Syntax(IndexedHighlightGroup::from_str("comment").unwrap()),
            )),
            Expect(ExpectKind::HighlightSpans(
                12..14,
                StyleKey::Syntax(IndexedHighlightGroup::from_str("keyword.function").unwrap()),
            )),
            Expect(ExpectKind::HighlightSpans(
                36..37,
                StyleKey::Syntax(IndexedHighlightGroup::from_str("punctuation.bracket").unwrap()),
            )),
            Expect(ExpectKind::HighlightSpans(
                38..133,
                StyleKey::Syntax(IndexedHighlightGroup::from_str("comment").unwrap()),
            )),
            Editor(MatchLiteral("fn".to_string())),
            Editor(EnterInsertMode(Direction::Start)),
            Editor(Insert("bqoxu".to_string())),
            // Expect the range of highlight spans of `/* hello */` is not updated,
            // because the edit range is beyond its range
            Expect(ExpectKind::HighlightSpans(
                0..11,
                StyleKey::Syntax(IndexedHighlightGroup::from_str("comment").unwrap()),
            )),
            // Expect the range of highlight spans of `fn` and `}`
            // are updated because they are in visible line ranges
            Expect(ExpectKind::HighlightSpans(
                17..19,
                StyleKey::Syntax(IndexedHighlightGroup::from_str("keyword.function").unwrap()),
            )),
            // Expect the last comment highlight span is not updated as it is not within visible view
            Expect(ExpectKind::HighlightSpans(
                38..133,
                StyleKey::Syntax(IndexedHighlightGroup::from_str("comment").unwrap()),
            )),
        ])
    })
}

#[test]
fn syntax_highlighting() -> anyhow::Result<()> {
    execute_test(|s| {
        let theme = Theme::default();
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            App(SetTheme(theme.clone())),
            Editor(SetContent(
                "
fn main() { // too long
  let foo = 1;
  let bar = baba; let wrapped = coco;
}
"
                .trim()
                .to_string(),
            )),
            Editor(SetRectangle(Rectangle {
                origin: Position::default(),
                width: 14,
                height: 4,
            })),
            Editor(SetLanguage(Box::new(
                crate::config::from_extension("rs").unwrap(),
            ))),
            Editor(MatchLiteral("bar".to_string())),
            Editor(ApplySyntaxHighlight),
            Editor(SetRectangle(Rectangle {
                origin: Position::default(),
                width: 21,
                height: 4,
            })),
            Editor(SwitchViewAlignment),
            // The "long" of "too long" is not shown, because it exceeded the view width
            Expect(EditorGrid(
                "
🦀  main.rs [*]
1│fn main() { // too
3│  let █ar = baba;
↪│let wrapped = coco
"
                .trim(),
            )),
            ExpectMulti(
                [
                    //
                    // Expect the `fn` keyword of the outbound parent line "fn main() { // too long" is highlighted properly
                    Position::new(1, 2),
                    Position::new(1, 3),
                ]
                .into_iter()
                .map(|position| {
                    ExpectKind::GridCellStyleKey(
                        position,
                        Some(StyleKey::Syntax(
                            IndexedHighlightGroup::from_str("keyword.function").unwrap(),
                        )),
                    )
                })
                .collect(),
            ),
            Expect(
                // Expect the left parenthesis of the outbound parent line "fn main() { // too long" is highlighted properly
                ExpectKind::GridCellStyleKey(
                    Position::new(1, 9),
                    Some(StyleKey::Syntax(
                        IndexedHighlightGroup::from_str("punctuation.bracket").unwrap(),
                    )),
                ),
            ),
            ExpectMulti(
                [
                    // Expect the `let` keyword of line 3 (which is inbound and not wrapped) is highlighted properly
                    Position::new(2, 4),
                    Position::new(2, 5),
                    Position::new(2, 6),
                    //
                    // Expect the `let` keyword of line 3 (which is inbound but wrapped) is highlighted properly
                    Position::new(3, 2),
                    Position::new(3, 3),
                    Position::new(3, 4),
                ]
                .into_iter()
                .map(|position| {
                    ExpectKind::GridCellStyleKey(
                        position,
                        Some(StyleKey::Syntax(
                            IndexedHighlightGroup::from_str("keyword").unwrap(),
                        )),
                    )
                })
                .collect(),
            ),
            // Expect decorations overrides syntax highlighting
            Editor(MatchLiteral("fn".to_string())),
            App(MarkFileAndToggleMark),
            // Move cursor to next line, so that "fn" is not selected,
            //  so that we can test the style applied to "fn" ,
            // otherwise the style of primary selection anchors will override the mark style
            Editor(MatchLiteral("let".to_string())),
            Expect(EditorGrid(
                "
# 🦀  main.rs [*]
1│fn main() { // too
↪│ long
2│  █et foo = 1;
"
                .trim(),
            )),
            ExpectMulti(
                [Position::new(1, 2), Position::new(1, 3)]
                    .into_iter()
                    .map(|position| ExpectKind::GridCellStyleKey(position, Some(StyleKey::UiMark)))
                    .collect(),
            ),
        ])
    })
}

#[test]
fn empty_content_should_have_one_line() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetRectangle(Rectangle {
                origin: Position::default(),
                width: 20,
                height: 2,
            })),
            Editor(SetContent("".to_string())),
            Expect(EditorGrid(
                "
🦀  main.rs [*]
1│█
"
                .trim(),
            )),
        ])
    })
}

#[test]
fn update_mark_position_with_undo_and_redo() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("foo bar spim".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Subword)),
            Editor(MoveSelection(Right)),
            Editor(MoveSelection(Right)),
            App(MarkFileAndToggleMark),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Mark)),
            Expect(CurrentSelectedTexts(&["spim"])),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Subword)),
            Editor(MoveSelection(Left)),
            Editor(MoveSelection(Left)),
            // Kill "foo"
            Editor(DeleteWithMovement(Right)),
            Expect(CurrentComponentContent("bar spim")),
            // Expect mark position is updated (still selects "spim")
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Mark)),
            Expect(CurrentSelectedTexts(&["spim"])),
            Editor(Undo),
            Expect(CurrentComponentContent("foo bar spim")),
            // Expect mark position is updated (still selects "spim")
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Mark)),
            Expect(CurrentSelectedTexts(&["spim"])),
            Editor(Redo),
            // Expect mark position is updated (still selects "spim")
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Mark)),
            Expect(CurrentSelectedTexts(&["spim"])),
        ])
    })
}

#[test]
fn saving_should_not_destroy_mark_if_selections_not_modified() -> anyhow::Result<()> {
    let input = "// foo bar spim\n    fn foo() {}\n";

    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(input.to_string())),
            Editor(SetLanguage(Box::new(
                crate::config::from_extension("rs").unwrap(),
            ))),
            Editor(MatchLiteral("bar".to_string())),
            App(MarkFileAndToggleMark),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Mark)),
            Editor(ForceSave),
            // Expect the content is formatted (second line dedented)
            Expect(CurrentComponentContent("// foo bar spim\nfn foo() {}\n")),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Character)),
            Expect(CurrentSelectedTexts(&["b"])),
            // Expect the mark on "bar" is not destroyed
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Mark)),
            Expect(CurrentSelectedTexts(&["bar"])),
        ])
    })
}

#[test]
fn surround() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("fn main() { x.y() }".to_string())),
            Editor(MatchLiteral("x.y()".to_string())),
            App(HandleKeyEvents(keys!("g , j").to_vec())),
            Expect(CurrentComponentContent("fn main() { (x.y()) }")),
            Expect(SelectionExtensionEnabled(false)),
        ])
    })
}

#[test]
fn swap_cursor_with_anchor() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetRectangle(Rectangle {
                origin: Position::default(),
                width: 20,
                height: 3,
            })),
            Editor(SetContent("fn main() { x.y() }  // hello ".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, SyntaxNode)),
            Editor(SwapCursor),
            Expect(EditorGrid(
                "
🦀  main.rs [*]
1│fn main() { x.y
↪│() █  // hello
"
                .trim(),
            )),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Character)),
            Expect(CurrentSelectedTexts(&["}"])),
            // Expect cursor direction is reset to `Start` if selection mode is changed
            Expect(CurrentCursorDirection(Direction::Start)),
        ])
    })
}

#[test]
/// Line with emoji: not wrapped
fn consider_unicode_width() -> anyhow::Result<()> {
    let content = "👩 abc";
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            App(TerminalDimensionChanged(crate::app::Dimension {
                height: 10,
                // Set width longer than content so that there's no wrapping
                width: 20,
            })),
            Editor(SetContent(content.to_string())),
            Editor(MatchLiteral("a".to_string())),
            // Expect the cursor is on the letter 'a'
            // Expect an extra space is added between 'a' and the emoji
            // because, the unicode width of the emoji is 2
            Expect(EditorGrid("🦀  main.rs [*]\n1│👩  █bc\n\n\n\n\n\n\n")),
        ])
    })
}

#[test]
fn delete_backward() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("hello world yo".to_string())),
            Editor(MatchLiteral("world".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Subword)),
            Expect(CurrentSelectedTexts(&["world"])),
            Editor(DeleteWithMovement(Left)),
            Expect(CurrentSelectedTexts(&["hello"])),
            Expect(CurrentComponentContent("hello yo")),
        ])
    })
}

#[test]
fn tree_sitter_should_not_reparse_in_insert_mode() -> anyhow::Result<()> {
    let mut editor = crate::components::editor::Editor::from_text(
        Some(tree_sitter_md::LANGUAGE.into()),
        "fn main() {}",
    );
    let context = Context::default();
    let _ = editor.enter_insert_mode(Direction::End, &context)?;

    let current_range = editor.buffer().tree().unwrap().root_node().range();
    let _ = editor.insert("fn hello() {}", &context)?;
    // Modifying the content in insert mode should not cause the tree to be reparsed
    let new_range = editor.buffer().tree().unwrap().root_node().range();
    assert_eq!(current_range, new_range);

    // Entering normal mode should reparse the tree
    editor.enter_normal_mode(&context)?;
    let new_range = editor.buffer().tree().unwrap().root_node().range();
    assert_ne!(current_range, new_range);

    Ok(())
}

#[test]
fn next_prev_after_current_selection_is_deleted() -> anyhow::Result<()> {
    let run_test = |next: bool| {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("1 a 2 b 3 c".to_string())),
                Editor(MatchLiteral(if next { "1" } else { "3" }.to_string())),
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    SelectionMode::Find {
                        search: crate::context::Search {
                            mode: LocalSearchConfigMode::Regex(RegexConfig {
                                escaped: false,
                                case_sensitive: false,
                                match_whole_word: false,
                            }),
                            search: r"\d+".to_string(),
                        },
                    },
                )),
                Editor(DeleteOne),
                Editor(MoveSelection(if next { Right } else { Left })),
                Expect(CurrentSelectedTexts(&["2"])),
            ])
        })
    };
    run_test(true)?;
    run_test(false)
}

#[test]
fn entering_insert_mode_from_visual_mode() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("hello world hey".to_string())),
            Editor(MatchLiteral("world".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Subword)),
            Editor(EnableSelectionExtension),
            Editor(MoveSelection(Right)),
            Expect(CurrentSelectedTexts(&["world hey"])),
            Editor(EnterInsertMode(Direction::Start)),
            App(HandleKeyEvents(keys!("x x space").to_vec())),
            Expect(CurrentComponentContent("hello xx world hey")),
            Expect(CurrentSelectedTexts(&[""])),
        ])
    })
}

#[test]
fn modifying_editor_causes_dirty_state() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Expect(Not(Box::new(EditorIsDirty()))),
            Expect(CurrentComponentTitle(markup_focused_tab(" 🦀 main.rs "))),
            Editor(EnterInsertMode(Direction::Start)),
            App(HandleKeyEvents(keys!("a a esc").to_vec())),
            Expect(EditorIsDirty()),
            Expect(CurrentComponentTitle(markup_focused_tab(
                " 🦀 main.rs [*] ",
            ))),
        ])
    })
}

#[test]
fn saving_editor_clears_dirty_state() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Expect(Not(Box::new(EditorIsDirty()))),
            Editor(EnterInsertMode(Direction::Start)),
            App(HandleKeyEvents(keys!("a a esc").to_vec())),
            Expect(EditorIsDirty()),
            Expect(CurrentComponentTitle(markup_focused_tab(
                " 🦀 main.rs [*] ",
            ))),
            Editor(Save),
            Expect(Not(Box::new(EditorIsDirty()))),
            Expect(CurrentComponentTitle(markup_focused_tab(" 🦀 main.rs "))),
        ])
    })
}

#[test]
fn after_save_select_current() -> anyhow::Result<()> {
    fn test(
        selection_mode: SelectionMode,
        expected_selected_texts: &'static [&'static str],
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
     let foo = 1;
}
"
                    .trim()
                    .to_string(),
                )),
                Editor(SetLanguage(Box::new(
                    crate::config::from_extension("rs").unwrap(),
                ))),
                Editor(MatchLiteral("let foo = 1;".to_string())),
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    selection_mode.clone(),
                )),
                Editor(ForceSave),
                Expect(CurrentComponentContent(
                    "
fn main() {
    let foo = 1;
}
"
                    .trim_start(),
                )),
                Expect(CurrentSelectedTexts(expected_selected_texts)),
            ])
        })
    }
    // The SyntaxNode selection mode is contiguous, thus it should select current after save
    test(SyntaxNode, &["let foo = 1;"])?;

    // The Find selection mode is not contigouos, thus it should not select current after save
    test(
        SelectionMode::Find {
            search: crate::context::Search {
                search: "let foo = 1;".to_string(),
                mode: LocalSearchConfigMode::Regex(RegexConfig {
                    escaped: true,
                    case_sensitive: false,
                    match_whole_word: false,
                }),
            },
        },
        &["et foo = 1;\n"],
    )
}

#[serial]
#[test]
fn undo_till_empty_should_not_crash_in_insert_mode() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("".to_string())),
            App(SetClipboardContent {
                copied_texts: CopiedTexts::one("foo".to_string()),
            }),
            Editor(EnterInsertMode(Direction::Start)),
            Editor(Paste),
            Expect(CurrentComponentContent("foo")),
            Editor(Undo),
            Expect(CurrentComponentContent("")),
        ])
    })
}

#[test]
fn selection_set_history() -> Result<(), anyhow::Error> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Line)),
            Expect(CurrentSelectedTexts(&["mod foo;"])),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Character)),
            Expect(CurrentSelectedTexts(&["m"])),
            App(ToEditor(GoBack)),
            Expect(CurrentSelectedTexts(&["mod foo;"])),
            App(ToEditor(GoForward)),
            Expect(CurrentSelectedTexts(&["m"])),
        ])
    })
}

#[test]
fn select_surround_inside_with_multiwidth_character() -> Result<(), anyhow::Error> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("(hello (w🦀orld))".to_string())),
            Editor(MatchLiteral("rl".to_string())),
            Editor(SelectSurround {
                enclosure: crate::surround::EnclosureKind::Parentheses,
                kind: SurroundKind::Inside,
            }),
            Expect(CurrentSelectedTexts(&["w🦀orld"])),
            Expect(CurrentSelectionMode(SelectionMode::Custom)),
        ])
    })
}

#[test]
fn select_surround_inside() -> Result<(), anyhow::Error> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("(hello (world))".to_string())),
            Editor(MatchLiteral("rl".to_string())),
            App(HandleKeyEvents(keys!("g h j").to_vec())),
            Expect(CurrentSelectedTexts(&["world"])),
            Expect(CurrentSelectionMode(SelectionMode::Custom)),
        ])
    })
}

#[test]
fn select_surround_around() -> Result<(), anyhow::Error> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("(hello (world))".to_string())),
            Editor(MatchLiteral("rl".to_string())),
            App(HandleKeyEvents(keys!("g ; j").to_vec())),
            Expect(CurrentSelectedTexts(&["(world)"])),
            Expect(CurrentSelectionMode(SelectionMode::Custom)),
        ])
    })
}

#[test]
fn select_surround_inside_same_symbols() -> Result<(), anyhow::Error> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("hello 'world'".to_string())),
            Editor(MatchLiteral("rl".to_string())),
            Editor(DeleteSurround(crate::surround::EnclosureKind::SingleQuotes)),
            Expect(CurrentSelectedTexts(&["world"])),
            Expect(CurrentSelectionMode(SelectionMode::Custom)),
        ])
    })
}

#[test]
fn delete_surround() -> Result<(), anyhow::Error> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("(hello (world))".to_string())),
            Editor(MatchLiteral("rl".to_string())),
            App(HandleKeyEvents(keys!("g v j").to_vec())),
            Expect(CurrentSelectedTexts(&["world"])),
            Expect(CurrentSelectionMode(SelectionMode::Custom)),
            Expect(CurrentComponentContent("(hello world)")),
        ])
    })
}

#[test]
fn change_surround_selection_not_on_enclosure() -> Result<(), anyhow::Error> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("(hello (world))".to_string())),
            Editor(MatchLiteral("rl".to_string())),
            App(HandleKeyEvents(keys!("g f j l").to_vec())),
            Expect(CurrentSelectedTexts(&["{world}"])),
            Expect(CurrentSelectionMode(SelectionMode::Custom)),
            Expect(CurrentComponentContent("(hello {world})")),
        ])
    })
}

#[test]
fn change_surround_selection_on_enclosure() -> Result<(), anyhow::Error> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("(hello)".to_string())),
            Editor(MatchLiteral("(hello)".to_string())),
            App(HandleKeyEvents(keys!("g f j l").to_vec())),
            Expect(CurrentSelectedTexts(&["{hello}"])),
        ])
    })
}

#[test]
fn replace_with_pattern() -> Result<(), anyhow::Error> {
    fn run_test(
        mode: LocalSearchConfigMode,
        content: &str,
        search_pattern: &str,
        replace_pattern: &str,
        expected_content: &'static str,
        expected_selected_text: &'static [&'static str],
    ) -> anyhow::Result<()> {
        execute_test(|s| {
            {
                Box::new([
                    App(OpenFile {
                        path: s.main_rs(),
                        owner: BufferOwner::User,
                        focus: true,
                    }),
                    Editor(SetContent(content.to_string())),
                    App(UpdateLocalSearchConfig {
                        update: LocalSearchConfigUpdate::Mode(mode),
                        scope: Scope::Local,
                        if_current_not_found: IfCurrentNotFound::LookForward,
                        run_search_after_config_updated: true,
                        component_id: None,
                    }),
                    App(UpdateLocalSearchConfig {
                        update: LocalSearchConfigUpdate::Search(search_pattern.to_string()),
                        scope: Scope::Local,
                        if_current_not_found: IfCurrentNotFound::LookForward,
                        run_search_after_config_updated: true,
                        component_id: None,
                    }),
                    App(UpdateLocalSearchConfig {
                        update: LocalSearchConfigUpdate::Replacement(replace_pattern.to_string()),
                        scope: Scope::Local,
                        if_current_not_found: IfCurrentNotFound::LookForward,
                        run_search_after_config_updated: true,
                        component_id: None,
                    }),
                    Editor(ReplaceWithPattern),
                    Expect(CurrentComponentContent(expected_content)),
                    Expect(CurrentSelectedTexts(expected_selected_text)),
                ])
            }
        })
    }
    run_test(
        LocalSearchConfigMode::NamingConventionAgnostic,
        "aBull aCow TheBull theBull",
        "TheBull",
        "the mummy",
        "aBull aCow TheMummy theBull",
        &["TheMummy"],
    )?;
    run_test(
        LocalSearchConfigMode::Regex(RegexConfig {
            escaped: false,
            case_sensitive: false,
            match_whole_word: false,
        }),
        "ali_123 abu_456 adam_99",
        r"abu_(\d+)",
        "boodan_$1",
        "ali_123 boodan_456 adam_99",
        &["boodan_456"],
    )?;
    run_test(
        LocalSearchConfigMode::AstGrep,
        "fn main() {let x = f(y); f(y)}",
        r"f($A)",
        "g($A,$A)",
        "fn main() {let x = g(y,y); f(y)}",
        &["g(y,y)"],
    )?;
    Ok(())
}

#[test]
fn replace_extended_selection_should_not_derail_selection_range() -> Result<(), anyhow::Error> {
    execute_test(|s| {
        {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("foo bar spam".to_string())),
                App(SetClipboardContent {
                    copied_texts: CopiedTexts::one("x".to_string()),
                }),
                Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
                Editor(EnableSelectionExtension),
                Editor(MoveSelection(Right)),
                Expect(CurrentSelectedTexts(&["foo bar"])),
                Editor(ReplaceWithCopiedText { cut: false }),
                Expect(CurrentSelectedTexts(&["x"])),
                Expect(CurrentComponentContent("x spam")),
            ])
        }
    })
}

#[test]
fn move_left_right() -> Result<(), anyhow::Error> {
    execute_test(|s| {
        {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("ho".to_string())),
                Editor(EnterInsertMode(Direction::Start)),
                Editor(MoveCharacterForward),
                Editor(MoveCharacterForward),
                Editor(MoveCharacterForward),
                Editor(Insert("x".to_string())),
                Expect(CurrentComponentContent("hox")),
                Editor(MoveCharacterBack),
                Editor(MoveCharacterBack),
                Editor(MoveCharacterBack),
                Editor(MoveCharacterBack),
                Editor(Insert("y".to_string())),
                Expect(CurrentComponentContent("yhox")),
            ])
        }
    })
}

#[serial]
#[test]
fn yank_ring() -> Result<(), anyhow::Error> {
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
                Expect(CurrentSelectedTexts(&["a2", "b2", "c2"])),
                Editor(Copy),
                Editor(MoveSelection(Right)),
                Editor(Copy),
                Expect(CurrentSelectedTexts(&["a3", "b3", "c3"])),
                Editor(Paste),
                Editor(ReplaceWithPreviousCopiedText),
                Expect(CurrentSelectedTexts(&["a2", "b2", "c2"])),
                Expect(CurrentComponentContent(
                    "
a1 a2 a3 a2
b1 b2 b3 b2
c1 c2 c3 c2"
                        .trim(),
                )),
                Editor(ReplaceWithPreviousCopiedText),
                Expect(CurrentSelectedTexts(&["a1", "b1", "c1"])),
                Expect(CurrentComponentContent(
                    "
a1 a2 a3 a1
b1 b2 b3 b1
c1 c2 c3 c1"
                        .trim(),
                )),
                Editor(ReplaceWithPreviousCopiedText),
                Expect(CurrentSelectedTexts(&["a3", "b3", "c3"])),
                Expect(CurrentComponentContent(
                    "
a1 a2 a3 a3
b1 b2 b3 b3
c1 c2 c3 c3"
                        .trim(),
                )),
                Editor(ReplaceWithNextCopiedText),
                Expect(CurrentSelectedTexts(&["a1", "b1", "c1"])),
                Expect(CurrentComponentContent(
                    "
a1 a2 a3 a1
b1 b2 b3 b1
c1 c2 c3 c1"
                        .trim(),
                )),
                Expect(CurrentCopiedTextHistoryOffset(-2)),
                // Moving the selection should reset the copied text history offset,
                Editor(MoveSelection(Left)),
                Expect(CurrentCopiedTextHistoryOffset(0)),
            ])
        }
    })
}

#[test]
/// Primary cursor should remain in position when entering insert mode
fn multi_cursor_insert() -> Result<(), anyhow::Error> {
    execute_test(|s| {
        {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("hello world".to_string())),
                Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Subword)),
                Editor(MoveSelectionWithPriorChange(
                    Right,
                    Some(PriorChange::EnterMultiCursorMode),
                )),
                Expect(CurrentSelectedTexts(&["hello", "world"])),
                Editor(EnterInsertMode(Direction::End)),
                App(HandleKeyEvent(key!("x"))),
                Editor(EnterNormalMode),
                Editor(CursorKeepPrimaryOnly),
                Editor(MoveSelection(Current(IfCurrentNotFound::LookForward))),
                Expect(CurrentSelectedTexts(&["worldx"])),
            ])
        }
    })
}

#[test]
fn movement_current_look_forward_backward() -> Result<(), anyhow::Error> {
    execute_test(|s| {
        {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("hello world is good".to_string())),
                Editor(MatchLiteral("hello".to_string())),
                App(MarkFileAndToggleMark),
                Editor(MatchLiteral("good".to_string())),
                App(MarkFileAndToggleMark),
                Editor(MatchLiteral("world".to_string())),
                Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Mark)),
                Expect(CurrentSelectedTexts(&["good"])),
                Editor(MatchLiteral("world".to_string())),
                Expect(CurrentSelectedTexts(&["world"])),
                Editor(SetSelectionMode(IfCurrentNotFound::LookBackward, Mark)),
                Expect(CurrentSelectedTexts(&["hello"])),
            ])
        }
    })
}

#[test]
fn search_backward() -> Result<(), anyhow::Error> {
    execute_test(|s| {
        {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("fo_b xxx FoB".to_string())),
                Editor(MatchLiteral("xxx".to_string())),
                App(HandleKeyEvents(keys!("/").to_vec())),
                // Expect((IfCurrentNotFound::LookBackward)),
                App(HandleKeyEvents(keys!("q").to_vec())),
                // Naming-convention agnostic search "n fo_b"
                App(HandleKeyEvents(keys!("n space f o _ b").to_vec())),
                App(HandleKeyEvents(keys!("enter").to_vec())),
                // App(HandleKeyEvents(keys!("/ q ( enter").to_vec())),
                Expect(CurrentSelectedTexts(&["fo_b"])),
            ])
        }
    })
}

#[test]
fn selection_set_history_updates_upon_edit() -> Result<(), anyhow::Error> {
    execute_test(|s| {
        {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("foo bar spam".to_string())),
                Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
                Editor(MoveSelection(Right)),
                Editor(MoveSelection(Right)),
                Expect(CurrentSelectedTexts(&["spam"])),
                Editor(MoveSelection(Left)),
                Expect(CurrentSelectedTexts(&["bar"])),
                Editor(DeleteWithMovement(Right)),
                Expect(CurrentComponentContent("foo spam")),
                Editor(GoBack),
                Expect(CurrentSelectedTexts(&["spam"])),
            ])
        }
    })
}

#[test]
fn show_current_tree_sitter_node_sexp() -> Result<(), anyhow::Error> {
    execute_test(|s| {
        {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("fn main() {}".to_string())),
                Editor(SetSelectionMode(IfCurrentNotFound::LookForward, SyntaxNode)),
                Editor(ShowCurrentTreeSitterNodeSexp),
                App(OtherWindow),
                Expect(CurrentComponentContent(
                    "(function_item name: (identifier) parameters: (parameters) body: (block))",
                )),
            ])
        }
    })
}

#[serial]
#[test]
fn yank_paste_extended_selection() -> Result<(), anyhow::Error> {
    execute_test(|s| {
        {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("who lives in a".to_string())),
                Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Subword)),
                Editor(EnableSelectionExtension),
                Editor(MoveSelection(Right)),
                Expect(CurrentSelectedTexts(&["who lives"])),
                Editor(Copy),
                Editor(Paste),
                Expect(CurrentComponentContent("who lives who lives in a")),
                Expect(CurrentSelectedTexts(&["who lives"])),
                Editor(EnterInsertMode(Direction::End)),
                Editor(Insert("foo".to_string())),
                Editor(EnterInsertMode(Direction::End)),
                Expect(CurrentComponentContent("who lives who livesfoo in a")),
            ])
        }
    })
}

#[test]
fn last_contiguous_selection_mode() -> Result<(), anyhow::Error> {
    execute_test(|s| {
        {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("who lives in a".to_string())),
                Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
                App(MarkFileAndToggleMark),
                Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Mark)),
                Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
                Expect(CurrentSelectedTexts(&["who"])),
                Editor(MoveSelection(Right)),
                Expect(CurrentSelectedTexts(&["lives"])),
                App(UseLastNonContiguousSelectionMode(
                    IfCurrentNotFound::LookForward,
                )),
                Expect(CurrentSelectedTexts(&["who"])),
                Expect(CurrentSelectionMode(Mark)),
            ])
        }
    })
}

#[test]
fn test_indent_dedent() -> anyhow::Result<()> {
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
    foo()
}
"
                .to_string(),
            )),
            Editor(MatchLiteral("fn".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, SyntaxNode)),
            Editor(Indent),
            Expect(CurrentComponentContent(
                "
    fn main() {
        foo()
    }
",
            )),
            Expect(CurrentSelectedTexts(&["fn main() {
        foo()
    }"])),
            Editor(Dedent),
            Expect(CurrentComponentContent(
                "
fn main() {
    foo()
}
",
            )),
            Expect(CurrentSelectedTexts(&["fn main() {
    foo()
}"])),
        ])
    })
}

#[test]
fn test_dedent_in_column_mode() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("fom".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Character)),
            Expect(CurrentSelectedTexts(&["f"])),
            Editor(Indent),
            Expect(CurrentComponentContent("    fom")),
            Expect(CurrentSelectedTexts(&["f"])),
            Editor(Dedent),
            Expect(CurrentComponentContent("fom")),
            Expect(CurrentSelectedTexts(&["f"])),
        ])
    })
}

#[test]
fn test_over_dedent() -> anyhow::Result<()> {
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
    foo()
}
"
                .to_string(),
            )),
            Editor(MatchLiteral("fn".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, SyntaxNode)),
            Editor(Dedent),
            Expect(CurrentSelectedTexts(&["fn main() {
foo()
}"])),
        ])
    })
}

#[test]
fn cycle_primary_selection_forward() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("foo bar spam".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Subword)),
            Editor(CursorAddToAllSelections),
            Expect(CurrentPrimarySelection("foo")),
            Editor(CyclePrimarySelection(Direction::End)),
            Expect(CurrentPrimarySelection("bar")),
            Editor(CyclePrimarySelection(Direction::End)),
            Expect(CurrentPrimarySelection("spam")),
            Editor(CyclePrimarySelection(Direction::End)),
            Expect(CurrentPrimarySelection("foo")),
        ])
    })
}

#[test]
fn cycle_primary_selection_backward() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("foo bar spam".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Subword)),
            Editor(CursorAddToAllSelections),
            Expect(CurrentPrimarySelection("foo")),
            Editor(CyclePrimarySelection(Direction::Start)),
            Expect(CurrentPrimarySelection("spam")),
            Editor(CyclePrimarySelection(Direction::Start)),
            Expect(CurrentPrimarySelection("bar")),
            Editor(CyclePrimarySelection(Direction::Start)),
            Expect(CurrentPrimarySelection("foo")),
        ])
    })
}

#[test]
fn cycle_primary_selection_should_based_on_range_order() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("foo bar spam".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
            Editor(MoveSelection(Right)),
            Editor(MoveSelection(Right)),
            Expect(CurrentPrimarySelection("spam")),
            Editor(MoveSelectionWithPriorChange(
                Left,
                Some(PriorChange::EnterMultiCursorMode),
            )),
            Editor(MoveSelection(Left)),
            Editor(EnterNormalMode),
            Expect(CurrentPrimarySelection("foo")),
            Editor(CyclePrimarySelection(Direction::End)),
            Expect(CurrentPrimarySelection("bar")),
            Editor(CyclePrimarySelection(Direction::End)),
            Expect(CurrentPrimarySelection("spam")),
        ])
    })
}

#[test]
fn insert_mode_enter_auto_indent() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(
                "
foo bar
  spam
  hey"
                .trim()
                .to_string(),
            )),
            Editor(MatchLiteral("spam".to_string())),
            Editor(EnterInsertMode(Direction::End)),
            App(HandleKeyEvents(keys!("enter enter").to_vec())),
            Editor(Insert("baz".to_string())),
            Expect(CurrentComponentContent(
                "foo bar\n  spam\n  \n  baz\n  hey".trim(),
            )),
        ])
    })
}

#[test]
fn expand_to_nearest_enclosure_1_inside() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("hello (world yo)".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
            Editor(MoveSelection(Right)),
            Editor(MoveSelection(Expand)),
            Expect(CurrentSelectedTexts(&["world yo"])),
        ])
    })
}

#[test]
fn expand_to_nearest_enclosure_1_inside_2() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("hello (world yo)".to_string())),
            Editor(MatchLiteral("yo".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
            Expect(CurrentSelectedTexts(&["yo"])),
            Editor(MoveSelection(Expand)),
            Expect(CurrentSelectedTexts(&["world yo"])),
        ])
    })
}

#[test]
fn expand_to_nearest_enclosure_2_around() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("hello ((world_yo))".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
            Editor(MoveSelection(Right)),
            Expect(CurrentSelectedTexts(&["world_yo"])),
            Editor(MoveSelection(Expand)),
            Expect(CurrentSelectedTexts(&["(world_yo)"])),
            Editor(MoveSelection(Expand)),
            Expect(CurrentSelectedTexts(&["((world_yo))"])),
        ])
    })
}

#[test]
fn expand_to_nearest_enclosure_3_nested_brackets() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("{hello (world yo)}".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
            Editor(MoveSelection(Right)),
            Editor(MoveSelection(Expand)),
            Expect(CurrentSelectedTexts(&["hello (world yo)"])),
        ])
    })
}

#[test]
fn expand_to_nearest_enclosure_4_brackets_and_quotes() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("hello '{World Foo} bar'".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
            Editor(MoveSelection(Right)),
            Editor(MoveSelection(Expand)),
            Expect(CurrentSelectedTexts(&["World Foo"])),
            Editor(MoveSelection(Expand)),
            Expect(CurrentSelectedTexts(&["{World Foo}"])),
            Editor(MoveSelection(Expand)),
            Expect(CurrentSelectedTexts(&["{World Foo} bar"])),
            Editor(MoveSelection(Expand)),
            Expect(CurrentSelectedTexts(&["'{World Foo} bar'"])),
        ])
    })
}

#[test]
/// Quotes expansion must be between an odd-position quote with an even-position quote
/// never the other way around
fn expand_to_nearest_enclosure_5() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("'hello world' (foo bar 'spam baz')".to_string())),
            Editor(MatchLiteral("foo".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
            Editor(MoveSelection(Expand)),
            Expect(CurrentSelectedTexts(&["foo bar 'spam baz'"])),
        ])
    })
}

#[test]
fn expand_to_nearest_enclosure_6_with_escaped_quotes() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(
                r#"result1.query.contains("\"require\" @keyword.import")"#.to_string(),
            )),
            Editor(MatchLiteral("require".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
            Editor(MoveSelection(Expand)),
            Expect(CurrentSelectedTexts(&[r#"\"require\" @keyword.import"#])),
        ])
    })
}

#[test]
fn expand_to_nearest_enclosure_7_cursor_on_open_enclosure() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(r#"foo bar (hello world)"#.to_string())),
            Editor(MatchLiteral("(".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
            Editor(MoveSelection(Expand)),
            Expect(CurrentSelectedTexts(&["(hello world)"])),
        ])
    })
}

#[test]
fn expand_to_nearest_enclosure_8_cursor_on_close_enclosure() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(r#"foo bar (hello world)"#.to_string())),
            Editor(MatchLiteral(")".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
            Editor(MoveSelection(Expand)),
            Expect(CurrentSelectedTexts(&["(hello world)"])),
        ])
    })
}

#[test]
fn split_selections() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(
                "
fooz bar fooy
bar foox foow
foov foou bar
"
                .trim()
                .to_string(),
            )),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Line)),
            Editor(MoveSelectionWithPriorChange(
                Right,
                Some(PriorChange::EnterMultiCursorMode),
            )),
            Expect(CurrentSelectedTexts(&["fooz bar fooy", "bar foox foow"])),
            Editor(MatchLiteral("foo".to_string())),
            Expect(CurrentSelectionMode(SelectionMode::Find {
                search: Search {
                    mode: LocalSearchConfigMode::Regex(RegexConfig {
                        escaped: true,
                        case_sensitive: false,
                        match_whole_word: false,
                    }),
                    search: "foo".to_string(),
                },
            })),
            Expect(CurrentMode(Mode::Normal)),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
            Expect(CurrentSelectedTexts(&["fooz", "fooy", "foox", "foow"])),
        ])
    })
}

#[test]
fn select_next_line_when_cursor_is_at_last_space_of_current_line() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("abc \n yo".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Character)),
            Editor(MoveSelection(Last)),
            Editor(MoveSelection(Right)),
            Expect(CurrentSelectedTexts(&[" "])),
            Editor(MoveSelection(Left)),
            Expect(CurrentSelectedTexts(&["c"])),
            Editor(MoveSelection(Right)),
            Expect(CurrentSelectedTexts(&[" "])),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Line)),
            Expect(CurrentSelectedTexts(&["yo"])),
        ])
    })
}

#[test]
fn first_last_char() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("babyHelloCamp".to_string())),
            Editor(MatchLiteral("Hello".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Character)),
            Editor(MoveSelection(Last)),
            Expect(CurrentSelectedTexts(&["o"])),
            Editor(MoveSelection(First)),
            Expect(CurrentSelectedTexts(&["H"])),
        ])
    })
}

#[test]
fn first_last_word() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("hello HTTPNetworkRequest yo".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Subword)),
            Editor(MoveSelection(Right)),
            Expect(CurrentSelectedTexts(&["HTTP"])),
            Editor(MoveSelection(Last)),
            Expect(CurrentSelectedTexts(&["Request"])),
            Editor(MoveSelection(First)),
            Expect(CurrentSelectedTexts(&["HTTP"])),
        ])
    })
}

#[test]
fn swap_till_last() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(
                "fn main(foo: T, apple: T, banana: T, coffee: T) {}".to_string(),
            )),
            Editor(MatchLiteral("apple: T".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, SyntaxNode)),
            Expect(CurrentSelectedTexts(&["apple: T"])),
            Editor(EnterSwapMode),
            Editor(MoveSelection(Last)),
            Expect(CurrentComponentContent(
                "fn main(foo: T, banana: T, coffee: T, apple: T) {}",
            )),
        ])
    })
}

#[test]
fn swap_till_first() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(
                "fn main(foo: T, apple: T, banana: T, coffee: T) {}".to_string(),
            )),
            Editor(MatchLiteral("banana: T".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, SyntaxNode)),
            Expect(CurrentSelectedTexts(&["banana: T"])),
            Editor(EnterSwapMode),
            Editor(MoveSelection(First)),
            Expect(CurrentComponentContent(
                "fn main(banana: T, foo: T, apple: T, coffee: T) {}",
            )),
        ])
    })
}

#[test]
fn add_cursor_till_first() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(
                "fn main(foo: T, apple: T, banana: T, coffee: T) {}".to_string(),
            )),
            Editor(MatchLiteral("banana: T".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, SyntaxNode)),
            Expect(CurrentSelectedTexts(&["banana: T"])),
            Editor(MoveSelectionWithPriorChange(
                First,
                Some(PriorChange::EnterMultiCursorMode),
            )),
            Expect(CurrentSelectedTexts(&["foo: T", "apple: T", "banana: T"])),
        ])
    })
}

#[test]
fn add_cursor_till_last() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(
                "fn main(foo: T, apple: T, banana: T, coffee: T) {}".to_string(),
            )),
            Editor(MatchLiteral("apple: T".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, SyntaxNode)),
            Expect(CurrentSelectedTexts(&["apple: T"])),
            Editor(MoveSelectionWithPriorChange(
                Last,
                Some(PriorChange::EnterMultiCursorMode),
            )),
            Expect(CurrentSelectedTexts(&[
                "apple: T",
                "banana: T",
                "coffee: T",
            ])),
        ])
    })
}

#[test]
fn delete_cursor_forward() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(
                "fn main(foo: T, apple: T, coffee: T) {}".to_string(),
            )),
            Editor(MatchLiteral("apple: T".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, SyntaxNode)),
            Expect(CurrentSelectedTexts(&["apple: T"])),
            Editor(CursorAddToAllSelections),
            Expect(CurrentSelectedTexts(&["foo: T", "apple: T", "coffee: T"])),
            Editor(CyclePrimarySelection(Direction::End)),
            Expect(CurrentPrimarySelection("apple: T")),
            Editor(DeleteCurrentCursor(Direction::End)),
            Expect(CurrentSelectedTexts(&["foo: T", "coffee: T"])),
            Expect(CurrentPrimarySelection("coffee: T")),
            Editor(DeleteCurrentCursor(Direction::End)),
            Expect(CurrentSelectedTexts(&["foo: T"])),
            Expect(CurrentPrimarySelection("foo: T")),
            Editor(DeleteCurrentCursor(Direction::End)),
            Expect(CurrentSelectedTexts(&["foo: T"])),
            Expect(CurrentPrimarySelection("foo: T")),
        ])
    })
}

#[test]
fn delete_cursor_backward() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(
                "fn main(foo: T, apple: T, coffee: T) {}".to_string(),
            )),
            Editor(MatchLiteral("apple: T".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, SyntaxNode)),
            Expect(CurrentSelectedTexts(&["apple: T"])),
            Editor(CursorAddToAllSelections),
            Expect(CurrentSelectedTexts(&["foo: T", "apple: T", "coffee: T"])),
            Editor(CyclePrimarySelection(Direction::End)),
            Expect(CurrentPrimarySelection("apple: T")),
            Editor(DeleteCurrentCursor(Direction::Start)),
            Expect(CurrentSelectedTexts(&["foo: T", "coffee: T"])),
            Expect(CurrentPrimarySelection("foo: T")),
            Editor(DeleteCurrentCursor(Direction::Start)),
            Expect(CurrentSelectedTexts(&["coffee: T"])),
            Expect(CurrentPrimarySelection("coffee: T")),
            Editor(DeleteCurrentCursor(Direction::Start)),
            Expect(CurrentSelectedTexts(&["coffee: T"])),
            Expect(CurrentPrimarySelection("coffee: T")),
        ])
    })
}

#[test]
fn break_selection() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("hello world\n\tbar spam".to_string())),
            Editor(MatchLiteral("spam".to_string())),
            Editor(BreakSelection),
            Editor(BreakSelection),
            Expect(CurrentSelectedTexts(&["spam"])),
            Expect(CurrentComponentContent("hello world\n\tbar\n\t\n\tspam")),
        ])
    })
}

#[test]
fn delete_empty_lines() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(
                "
hello

world

yo"
                .trim()
                .to_string(),
            )),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Line)),
            Editor(MoveSelection(Next)),
            Expect(CurrentSelectedTexts(&[""])),
            Editor(DeleteWithMovement(Right)),
            Expect(CurrentSelectedTexts(&["world"])),
        ])
    })
}

#[test]
fn empty_lines_navigation_line_full() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(
                "
foo
bar


spam
baz


bomb
bam
"
                .trim()
                .to_string(),
            )),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, LineFull)),
            Expect(CurrentSelectedTexts(&["foo\n"])),
            Editor(MoveSelection(Down)),
            Expect(CurrentSelectedTexts(&["\n"])),
            Editor(MoveSelection(Left)),
            Expect(CurrentSelectedTexts(&["bar\n"])),
            Editor(MoveSelection(Right)),
            Editor(MoveSelection(Down)),
            Editor(MoveSelection(Left)),
            Expect(CurrentSelectedTexts(&["baz\n"])),
            Editor(MoveSelection(Right)),
            Editor(MoveSelection(Up)),
            Editor(MoveSelection(Right)),
            Expect(CurrentSelectedTexts(&["spam\n"])),
        ])
    })
}

#[test]
fn empty_lines_navigation_line_trimmed() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(
                "
foo
bar


spam
baz


bomb
bam
"
                .trim()
                .to_string(),
            )),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Line)),
            Expect(CurrentSelectedTexts(&["foo"])),
            Editor(MoveSelection(Down)),
            Expect(CurrentSelectedTexts(&[""])),
            Editor(MoveSelection(Previous)),
            Expect(CurrentSelectedTexts(&["bar"])),
            Editor(MoveSelection(Next)),
            Editor(MoveSelection(Down)),
            Editor(MoveSelection(Previous)),
            Expect(CurrentSelectedTexts(&["baz"])),
            Editor(MoveSelection(Next)),
            Editor(MoveSelection(Up)),
            Editor(MoveSelection(Next)),
            Expect(CurrentSelectedTexts(&["spam"])),
        ])
    })
}

#[test]
fn visual_select_anchor_change_selection_mode() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("helloWorld fooBar".trim().to_string())),
            Editor(SetSelectionModeWithPriorChange(
                IfCurrentNotFound::LookForward,
                Word,
                Some(PriorChange::EnableSelectionExtension),
            )),
            Expect(CurrentSelectedTexts(&["helloWorld"])),
            App(HandleKeyEvent(key!("l"))),
            Expect(CurrentSelectedTexts(&["helloWorld fooBar"])),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Subword)),
            Expect(CurrentSelectedTexts(&["helloWorld foo"])),
        ])
    })
}

#[test]
fn background_editor_not_in_buffer_list() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::System,
                focus: false,
            }),
            Expect(OpenedFilesCount(0)),
        ])
    })
}

#[test]
fn background_editor_focused_not_in_buffer_list() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::System,
                focus: true,
            }),
            Expect(OpenedFilesCount(0)),
        ])
    })
}

#[test]
fn background_editor_forefront_on_edit() -> anyhow::Result<()> {
    execute_test(|_| {
        Box::new([
            App(HandleKeyEvents(
                keys!("space q f o o : : f o o enter").to_vec(),
            )),
            Expect(OpenedFilesCount(0)),
            WaitForAppMessage(regex!("AddQuickfixListEntries")),
            Expect(CurrentComponentTitle(markup_focused_tab(" 🦀 main.rs "))),
            Editor(EnterInsertMode(Direction::Start)),
            App(HandleKeyEvents(keys!("a a esc").to_vec())),
            Expect(OpenedFilesCount(1)),
        ])
    })
}

#[test]
fn background_editor_user_from_explorer() -> anyhow::Result<()> {
    execute_test(|_| {
        Box::new([
            App(HandleKeyEvents(
                keys!("space ; q s r c enter enter q m a i n . r s enter enter").to_vec(),
            )),
            Expect(CurrentComponentTitle(markup_focused_tab(" 🦀 main.rs "))),
            Expect(OpenedFilesCount(1)),
        ])
    })
}

#[test]
fn background_editor_closing_no_system_buffer() -> anyhow::Result<()> {
    execute_test(|_| {
        Box::new([
            App(OpenSearchPrompt {
                scope: Scope::Global,
                if_current_not_found: IfCurrentNotFound::LookForward,
            }),
            App(HandleKeyEvents(keys!("f o o enter").to_vec())),
            WaitForAppMessage(regex!("AddQuickfixListEntries")),
            Expect(CurrentComponentTitle(markup_focused_tab(" 🦀 foo.rs "))),
            Expect(OpenedFilesCount(0)),
            App(CloseCurrentWindow),
            Expect(OpenedFilesCount(0)),
            Expect(CurrentComponentTitle(
                "[ROOT] (Cannot be saved)".to_string(),
            )),
        ])
    })
}

#[test]
fn search_current_selection() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(
                "foo bar test foo bary moss foo bars".to_string(),
            )),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
            Editor(EnableSelectionExtension),
            Editor(MoveSelection(Right)),
            Expect(CurrentSelectedTexts(&["foo bar"])),
            Expect(SelectionExtensionEnabled(true)),
            Editor(SearchCurrentSelection(
                IfCurrentNotFound::LookForward,
                Scope::Local,
            )),
            Expect(CurrentSelectedTexts(&["foo bar"])),
            Expect(SelectionExtensionEnabled(false)),
            Expect(PromptHistory(
                PromptHistoryKey::Search,
                ["l/foo bar".to_string()].to_vec(),
            )),
        ])
    })
}

#[test]
fn search_current_selection_history_should_be_prepended_with_l() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("w / o fx".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
            Editor(EnableSelectionExtension),
            Editor(MoveSelection(Right)),
            Expect(CurrentSelectedTexts(&["w / o"])),
            Editor(SearchCurrentSelection(
                IfCurrentNotFound::LookForward,
                Scope::Local,
            )),
            Expect(CurrentSelectedTexts(&["w / o"])),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
            Editor(MoveSelection(Right)),
            Editor(MoveSelection(Right)),
            Expect(CurrentSelectedTexts(&["fx"])),
            App(OpenSearchPrompt {
                scope: Scope::Local,
                if_current_not_found: IfCurrentNotFound::LookForward,
            }),
            Expect(PromptHistory(
                PromptHistoryKey::Search,
                ["l/w \\/ o".to_string()].to_vec(),
            )),
        ])
    })
}

#[test]
fn should_search_backward_if_primary_and_secondary_cursor_swapped() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("  hello world  ".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Line)),
            Editor(SwapCursor),
            App(HandleKeyEvent(key!("s"))), // Word selection mode (Qwerty)
            Expect(CurrentSelectedTexts(&["world"])),
        ])
    })
}

#[test]
fn git_hunk_should_compare_against_buffer_content_not_file_content() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(EnterInsertMode(Direction::Start)),
            App(HandleKeyEvents(keys!("h e l l o").to_vec())),
            Editor(EnterNormalMode),
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                GitHunk(DiffMode::UnstagedAgainstCurrentBranch),
            )),
            Editor(CursorAddToAllSelections),
            Expect(CurrentSelectedTexts(&["hellomod foo;"])),
        ])
    })
}

#[test]
fn should_trim_parent_line_if_not_enough_space() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetRectangle(Rectangle {
                origin: Position::default(),
                width: 20,
                height: 3,
            })),
            Editor(SetContent(
                "
fn main() {
    fn foo() {
        bar();
    }
}
"
                .trim()
                .to_string(),
            )),
            Editor(MatchLiteral("bar".to_string())),
            Expect(CurrentSelectedTexts(&["bar"])),
            Expect(EditorGrid(
                "
🦀  main.rs [*]
1│fn main() {
3│        █ar();
"
                .trim(),
            )),
        ])
    })
}

#[test]
fn should_prioritize_wrapped_selection_if_no_space_left() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetRectangle(Rectangle {
                origin: Position::default(),
                width: 20,
                height: 2,
            })),
            Editor(SetContent("foofoofoo barbarbar".trim().to_string())),
            Editor(MatchLiteral("bar".to_string())),
            Expect(CurrentSelectedTexts(&["bar"])),
            Expect(EditorGrid(
                "
🦀  main.rs [*]
↪│█arbarbar"
                    .trim(),
            )),
        ])
    })
}

#[test]
fn hidden_parent_lines_count_should_take_at_most_50_percent_of_render_area() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetRectangle(Rectangle {
                origin: Position::default(),
                width: 20,
                height: 5,
            })),
            Editor(SetContent(
                "
fn foo() {
  fn bar() {
    fn spam() {
        xxx();
        yyy();
    }
  }
}"
                .trim()
                .to_string(),
            )),
            Editor(MatchLiteral("yyy".to_string())),
            Expect(EditorGrid(
                "🦀  main.rs [*]
1│fn foo() {
2│  fn bar() {
5│        █yy();
6│    }"
                    .trim(),
            )),
        ])
    })
}

#[test]
fn surround_extended_selection() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("foo bar".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
            Editor(EnableSelectionExtension),
            Editor(MoveSelection(Right)),
            App(HandleKeyEvents(keys!("g , j").to_vec())),
            Expect(CurrentComponentContent("(foo bar)")),
        ])
    })
}

#[test]
fn undo_redo_1() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("foo bar".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
            Editor(DeleteWithMovement(Right)),
            Editor(DeleteWithMovement(Right)),
            Expect(CurrentComponentContent("")),
            Editor(Undo),
            Expect(CurrentComponentContent("bar")),
            Expect(CurrentSelectedTexts(&["bar"])),
            Editor(Undo),
            Expect(CurrentComponentContent("foo bar")),
            Expect(CurrentSelectedTexts(&["foo"])),
            Editor(Redo),
            Expect(CurrentComponentContent("bar")),
            Expect(CurrentSelectedTexts(&["bar"])),
            Editor(Redo),
            Expect(CurrentComponentContent("")),
            Editor(Undo),
            Expect(CurrentComponentContent("bar")),
            Expect(CurrentSelectedTexts(&["bar"])),
        ])
    })
}

#[serial]
#[test]
fn undo_redo_should_clear_redo_stack_upon_new_edits() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("foo bar".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
            Editor(DeleteWithMovement(Right)),
            Editor(DeleteWithMovement(Left)),
            Expect(CurrentComponentContent("")),
            Editor(Undo),
            Expect(CurrentComponentContent("bar")),
            Expect(CurrentSelectedTexts(&["bar"])),
            Editor(Copy),
            Editor(Paste),
            Expect(CurrentComponentContent("barbar")),
            Editor(Undo),
            Editor(Redo),
            Editor(Redo),
            Expect(CurrentComponentContent("barbar")),
        ])
    })
}

#[test]
fn undo_redo_multicursor() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("foo bar".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
            Editor(CursorAddToAllSelections),
            Editor(EnterInsertMode(Direction::End)),
            App(HandleKeyEvents(keys!("x").to_vec())),
            Expect(CurrentComponentContent("foox barx")),
            Editor(Undo),
            Editor(Redo),
            Editor(EnterNormalMode),
            Expect(CurrentSelectedTexts(&["x", "x"])),
            Editor(SetSelectionMode(IfCurrentNotFound::LookBackward, Word)),
            Expect(CurrentSelectedTexts(&["foox", "barx"])),
        ])
    })
}

#[test]
/// Edits that intersect with its previous edit will be ignored
fn multicursor_intersected_edits() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("fn main() { foo() }".to_string())),
            Editor(MatchLiteral("foo()".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, SyntaxNode)),
            Editor(MoveSelectionWithPriorChange(
                Up,
                Some(PriorChange::EnterMultiCursorMode),
            )),
            Expect(CurrentSelectedTexts(&["{ foo() }", "foo()"])),
            Editor(DeleteWithMovement(Right)),
            // Expect the primary cursor is still there
            // And the Deletion of `foo()` is ignored
            Expect(AppGrid(" 🦀  main.rs [*]\n1│fn main█)".to_string())),
        ])
    })
}

#[test]
fn multicursor_insertion_at_same_range_is_not_counted_as_intersected_edits() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("fooBar".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Subword)),
            Editor(CursorAddToAllSelections),
            Expect(CurrentSelectedTexts(&["foo", "Bar"])),
            Editor(Change),
            App(HandleKeyEvents(keys!("x y").to_vec())),
            Expect(CurrentComponentContent("xyxy")),
        ])
    })
}

#[test]
fn movement_up() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(
                "
foo bar
    spam
baz
tim
"
                .trim()
                .to_string(),
            )),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Line)),
            Editor(MoveSelection(Last)),
            Expect(CurrentSelectedTexts(&["tim"])),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
            Editor(MoveSelection(Up)),
            Expect(CurrentSelectedTexts(&["baz"])),
        ])
    })
}

#[test]
fn movement_down() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(
                "
foo bar
spam
    baz
"
                .trim()
                .to_string(),
            )),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
            Expect(CurrentSelectedTexts(&["foo"])),
            Editor(MoveSelection(Down)),
            Expect(CurrentSelectedTexts(&["spam"])),
        ])
    })
}

#[test]
fn move_line_downward() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(
                "
foo bar
    spam
    baz
"
                .trim()
                .to_string(),
            )),
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                SelectionMode::Line,
            )),
            Expect(CurrentSelectedTexts(&["foo bar"])),
            Editor(MoveSelection(Right)),
            Expect(CurrentSelectedTexts(&["spam"])),
            Editor(MoveSelection(Left)),
            Expect(CurrentSelectedTexts(&["foo bar"])),
            Editor(MoveSelection(Right)),
            Expect(CurrentSelectedTexts(&["spam"])),
        ])
    })
}

#[test]
fn move_line_upward() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(
                "
    spam
    baz
foo bar
hello
"
                .trim()
                .to_string(),
            )),
            Editor(MatchLiteral("foo bar".to_string())),
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                SelectionMode::Line,
            )),
            Expect(CurrentSelectedTexts(&["foo bar"])),
            Editor(MoveSelection(Left)),
            Expect(CurrentSelectedTexts(&["baz"])),
            Editor(MoveSelection(Right)),
            Expect(CurrentSelectedTexts(&["foo bar"])),
        ])
    })
}

#[test]
fn move_down_from_indented_line_to_last_dedented_line() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("    fo\nb".to_string())),
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                SelectionMode::Line,
            )),
            Expect(CurrentSelectedTexts(&["fo"])),
            Editor(MoveSelection(Right)),
            Expect(CurrentSelectedTexts(&["b"])),
            Editor(MoveSelection(Left)),
            Expect(CurrentSelectedTexts(&["fo"])),
        ])
    })
}

#[test]
fn delete_forward_last_dedented_lines() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("    fo\nb".to_string())),
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                SelectionMode::Line,
            )),
            Expect(CurrentSelectedTexts(&["fo"])),
            Editor(MoveSelection(Right)),
            Expect(CurrentSelectedTexts(&["b"])),
            Editor(DeleteWithMovement(Right)),
            Expect(CurrentSelectedTexts(&["fo"])),
        ])
    })
}

#[test]
fn the_first_line_should_be_selected_when_a_file_is_opened() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Expect(CurrentSelectedTexts(&["mod foo;"])),
        ])
    })
}

#[test]
fn insert_multiwidth_unicode_characters() -> Result<(), anyhow::Error> {
    execute_test(|s| {
        {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("hello world".trim().to_string())),
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    SelectionMode::Subword,
                )),
                Expect(CurrentSelectedTexts(&["hello"])),
                Editor(EnterInsertMode(Direction::End)),
                Editor(Insert("仁".to_string())),
                App(HandleKeyEvents(keys!("!").to_vec())),
                Expect(CurrentComponentContent("hello仁! world")),
            ])
        }
    })
}

#[test]
fn go_to_line_number() -> Result<(), anyhow::Error> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("foo\nbar\nspam".trim().to_string())),
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                SelectionMode::Line,
            )),
            App(OpenMoveToIndexPrompt(None)),
            App(HandleKeyEvents(keys!("3 enter").to_vec())),
            Expect(CurrentSelectedTexts(&["spam"])),
        ])
    })
}

#[test]
fn test_search_query_should_not_trim_surrounding_whitespace() -> Result<(), anyhow::Error> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("xfoo foobarfoo foo".trim().to_string())),
            App(OpenSearchPrompt {
                scope: Scope::Local,
                if_current_not_found: IfCurrentNotFound::LookForward,
            }),
            App(HandleKeyEvents(keys!("f o o space enter").to_vec())),
            Editor(CursorAddToAllSelections),
            Expect(CurrentSelectedTexts(&["foo ", "foo "])),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
            Expect(CurrentSelectedTexts(&["xfoo", "foobarfoo"])),
        ])
    })
}

#[test]
fn vertical_movement_sticky_column_position_based_selection_mode() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(
                "
spam bar
foo
java script"
                    .to_string(),
            )),
            Editor(MatchLiteral("bar".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Subword)),
            Expect(CurrentSelectedTexts(&["bar"])),
            Editor(MoveSelection(Down)),
            Expect(CurrentSelectedTexts(&["foo"])),
            Editor(MoveSelection(Down)),
            // Should be script, because it is the same column as `bar`
            Expect(CurrentSelectedTexts(&["script"])),
            Editor(MoveSelection(Left)),
            Expect(CurrentSelectedTexts(&["java"])),
            Editor(MoveSelection(Up)),
            Expect(CurrentSelectedTexts(&["foo"])),
            Editor(MoveSelection(Up)),
            // The Left movement should have reset the sticky column
            Expect(CurrentSelectedTexts(&["spam"])),
        ])
    })
}

#[test]
fn vertical_movement_sticky_column_iter_based_selection_mode() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(
                "
spam bar
foo
java script"
                    .to_string(),
            )),
            Editor(MatchLiteral("bar".to_string())),
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                SelectionMode::Find {
                    search: Search {
                        mode: LocalSearchConfigMode::Regex(RegexConfig {
                            escaped: false,
                            case_sensitive: false,
                            match_whole_word: false,
                        }),
                        search: "[a-zA-Z]+".to_string(),
                    },
                },
            )),
            Expect(CurrentSelectedTexts(&["bar"])),
            Editor(MoveSelection(Down)),
            Expect(CurrentSelectedTexts(&["foo"])),
            Editor(MoveSelection(Down)),
            // Should be script, because it is the same column as `bar`
            Expect(CurrentSelectedTexts(&["script"])),
            Editor(MoveSelection(Left)),
            Expect(CurrentSelectedTexts(&["java"])),
            Editor(MoveSelection(Up)),
            Expect(CurrentSelectedTexts(&["foo"])),
            Editor(MoveSelection(Up)),
            // The Left movement should have reset the sticky column
            Expect(CurrentSelectedTexts(&["spam"])),
        ])
    })
}

#[test]
fn multicursor_maintain_selections_uses_search_config() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(
                "
foo
for
fuor
"
                .trim()
                .to_string(),
            )),
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                SelectionMode::Line,
            )),
            Editor(CursorAddToAllSelections),
            Expect(CurrentSelectedTexts(&["foo", "for", "fuor"])),
            // Keep only selections matching `r/f.o`
            App(HandleKeyEvents(keys!("r h r / f . o enter").to_vec())),
            Expect(CurrentSelectedTexts(&["foo", "fuor"])),
        ])
    })
}

#[test]
fn line_move_right_back_to_historical_position() -> anyhow::Result<()> {
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
                "
                .trim()
                .to_string(),
            )),
            Editor(MatchLiteral("bar".to_string())),
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                SelectionMode::Line,
            )),
            Expect(CurrentSelectedTexts(&["bar();"])),
            Editor(MoveSelection(Left)),
            Editor(MoveSelection(Left)),
            Expect(CurrentSelectedTexts(&["fn main() {"])),
            App(HandleKeyEvents(keys!("backspace").to_vec())),
            Expect(CurrentSelectedTexts(&["foo();"])),
            App(HandleKeyEvents(keys!("backspace").to_vec())),
            Expect(CurrentSelectedTexts(&["bar();"])),
            App(HandleKeyEvents(keys!("tab").to_vec())),
            Expect(CurrentSelectedTexts(&["foo();"])),
            App(HandleKeyEvents(keys!("tab").to_vec())),
            Expect(CurrentSelectedTexts(&["fn main() {"])),
        ])
    })
}

#[test]
fn toggle_line_comment() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("hello world".to_string())),
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                SelectionMode::Line,
            )),
            Editor(ToggleLineComment),
            Expect(CurrentSelectedTexts(&["// hello world"])),
            Expect(CurrentComponentContent("// hello world")),
            Editor(ToggleLineComment),
            Expect(CurrentSelectedTexts(&["hello world"])),
            Expect(CurrentComponentContent("hello world")),
        ])
    })
}

#[test]
fn toggle_block_comment() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("hello world".to_string())),
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                SelectionMode::Line,
            )),
            Editor(ToggleBlockComment),
            Expect(CurrentSelectedTexts(&["/* hello world */"])),
            Expect(CurrentComponentContent("/* hello world */")),
            Editor(ToggleBlockComment),
            Expect(CurrentSelectedTexts(&["hello world"])),
            Expect(CurrentComponentContent("hello world")),
        ])
    })
}

#[serial]
#[test]
fn still_able_to_select_when_cursor_is_beyond_last_char() -> anyhow::Result<()> {
    fn run_test(
        selection_mode: SelectionMode,
        selected_texts: &'static [&'static str],
    ) -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("hello\n".to_string())),
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    SelectionMode::Line,
                )),
                Editor(MoveSelection(Last)),
                Editor(MoveSelection(Next)),
                Expect(EditorCursorPosition(Position::new(1, 0))),
                Expect(CurrentSelectedTexts(&[""])),
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    selection_mode.clone(),
                )),
                Expect(CurrentSelectedTexts(selected_texts)),
            ])
        })
    }
    run_test(Word, &["hello"])?;
    run_test(SyntaxNode, &["hello"])?;
    run_test(Subword, &["hello"])?;
    run_test(Character, &["\n"])?;
    Ok(())
}

#[test]
fn anchor_should_maintain_selection_mode() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(
                "kebab-case camelCase snake_case UPPER_SNAKE_CASE".to_string(),
            )),
            Editor(MatchLiteral("camel".to_string())),
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                SelectionMode::Subword,
            )),
            Expect(CurrentSelectedTexts(&["camel"])),
            Editor(EnableSelectionExtension),
            Editor(MoveSelection(Right)),
            Editor(MoveSelection(Right)),
            Expect(CurrentSelectedTexts(&["camelCase snake"])),
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                SelectionMode::Word,
            )),
            Expect(CurrentSelectedTexts(&["camelCase snake_case"])),
            Editor(SwapExtensionAnchor),
            Expect(CurrentSelectionMode(SelectionMode::Subword)),
            Editor(MoveSelection(Left)),
            Expect(CurrentSelectedTexts(&["case camelCase snake_case"])),
            Editor(SwapExtensionAnchor),
            Expect(CurrentSelectionMode(SelectionMode::Word)),
            Editor(MoveSelection(Right)),
            Expect(CurrentSelectedTexts(&[
                "case camelCase snake_case UPPER_SNAKE_CASE",
            ])),
        ])
    })
}

#[test]
/// When primary selection anchors overlap with hidden parent lines,
/// the primary selection anchors should not be missing.
fn primary_selection_anchor_overlap_with_hidden_parent_line() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            App(TerminalDimensionChanged(crate::app::Dimension {
                height: 6,
                // Set width longer than content so that there's no wrapping
                width: 20,
            })),
            Editor(SetContent(
                "
fn main() {
  first();
  second();
  t();
}
"
                .to_string(),
            )),
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                SelectionMode::SyntaxNode,
            )),
            Editor(SwapCursor),
            Expect(AppGrid(
                " 🦀  main.rs [*]
2│fn main() {
5│  t();
6│█
7│"
                .to_string(),
            )),
            Expect(RangeStyleKey(
                "t();",
                Some(StyleKey::UiPrimarySelectionAnchors),
            )),
        ])
    })
}

#[test]
fn global_git_hunk_and_local_git_hunk_should_not_cause_multiple_info_windows_to_be_shown(
) -> anyhow::Result<()> {
    execute_test(|s| {
        let diff_mode = DiffMode::UnstagedAgainstCurrentBranch;

        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("new content".to_string())),
            Editor(Save),
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                GitHunk(diff_mode),
            )),
            Expect(ExpectKind::ComponentsOrder(
                [ComponentKind::SuggestiveEditor, ComponentKind::GlobalInfo].to_vec(),
            )),
            App(GetRepoGitHunks(diff_mode)),
            Expect(ExpectKind::ComponentsOrder(
                [
                    ComponentKind::SuggestiveEditor,
                    ComponentKind::QuickfixList,
                    ComponentKind::GlobalInfo,
                ]
                .to_vec(),
            )),
        ])
    })
}

#[test]
fn escaping_quicfix_list_mode_should_not_change_selection() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("memento mori".to_string())),
            Editor(Save),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Line)),
            Expect(CurrentSelectionMode(Line)),
            Expect(CurrentSelectedTexts(&["memento mori"])),
            App(OpenSearchPrompt {
                scope: Scope::Global,
                if_current_not_found: IfCurrentNotFound::LookForward,
            }),
            App(HandleKeyEvents(keys!("m o r i enter").to_vec())),
            WaitForAppMessage(regex!("AddQuickfixListEntries")),
            Expect(CurrentGlobalMode(Some(GlobalMode::QuickfixListItem))),
            Expect(CurrentSelectedTexts(&["mori"])),
            App(HandleKeyEvents(keys!("esc").to_vec())),
            Expect(CurrentGlobalMode(None)),
            Expect(CurrentSelectedTexts(&["mori"])),
        ])
    })
}

#[test]
fn first_line_of_multiline_selection_that_is_taller_than_viewport_should_be_at_top_when_aligning_top(
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
// padding 1
// padding 2
// padding 3

fn main() {
  this_is_a_long_line_for_testing_wrapping();
  // padding x
  // padding y
  // padding z
  foo { // this line should be at top
    x: 2
    // padding x
    // padding y
    // padding z
    // padding z
    // padding z
    // padding z
    // padding z
    // padding z
    // padding z
  }
}
// padding 4
// padding 5
// padding 6"
                    .to_string(),
            )),
            App(SetGlobalTitle("[Global Title]".to_string())),
            App(TerminalDimensionChanged(Dimension {
                height: 9,
                width: 300,
            })),
            Editor(MatchLiteral("foo".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, SyntaxNode)),
            Expect(AppGrid(
                " 🦀  main.rs [*]
 6│fn main() {
 9│  // padding y
10│  // padding z
11│  █oo { // this line should be at top
12│    x: 2
13│    // padding x
14│    // padding y
 [Global Title]"
                    .to_string(),
            )),
            Editor(AlignViewTop),
            Expect(AppGrid(
                " 🦀  main.rs [*]
 6│fn main() {
11│  █oo { // this line should be at top
12│    x: 2
13│    // padding x
14│    // padding y
15│    // padding z
16│    // padding z
 [Global Title]"
                    .to_string(),
            )),
        ])
    })
}

#[test]
fn last_line_of_multiline_selection_should_be_at_bottom_when_aligning_bottom() -> anyhow::Result<()>
{
    fn run_test(width: usize, height: usize, expected_output: &'static str) -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent(
                    "
// padding 1
// padding 2
// padding 3

fn main() {
  this_is_a_long_line_for_testing_wrapping();
  foo {
    x: 2
  } // this line should be at bottom
}
// padding 4
// padding 5
// padding 6"
                        .to_string(),
                )),
                App(SetGlobalTitle("[Global Title]".to_string())),
                App(TerminalDimensionChanged(Dimension { height, width })),
                Editor(MatchLiteral("foo".to_string())),
                Editor(SetSelectionMode(IfCurrentNotFound::LookForward, SyntaxNode)),
                Editor(AlignViewBottom),
                Expect(AppGrid(expected_output.to_string())),
            ])
        })
    }

    // Case 1: Nothing is wrapped

    run_test(
        300,
        9,
        " 🦀  main.rs [*]
 4│// padding 3
 5│
 6│fn main() {
 7│  this_is_a_long_line_for_testing_wrapping();
 8│  █oo {
 9│    x: 2
10│  } // this line should be at bottom
 [Global Title]",
    )?;

    // Case 2: The long line is wrapped
    run_test(
        30,
        7,
        " 🦀  main.rs [*]
 6│fn main() {
 8│  █oo {
 9│    x: 2
10│  } // this line should be
 ↪│ at bottom
 [Global Title]",
    )?;

    Ok(())
}

#[test]
fn middle_line_of_multiline_selection_should_be_centered_when_aligning_center() -> anyhow::Result<()>
{
    fn run_test(width: usize, height: usize, expected_output: &'static str) -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent(
                    "
// padding 1
// padding 2
// padding 3

fn main() {
  this_is_a_long_line_for_testing_wrapping();
  foo {
    x: 2 // this line should be at center
  }
}
// padding 4
// padding 5
// padding 6"
                        .to_string(),
                )),
                App(SetGlobalTitle("[Global Title]".to_string())),
                App(TerminalDimensionChanged(Dimension { height, width })),
                Editor(MatchLiteral("foo".to_string())),
                Editor(SetSelectionMode(IfCurrentNotFound::LookForward, SyntaxNode)),
                Editor(AlignViewCenter),
                Expect(AppGrid(expected_output.to_string())),
            ])
        })
    }

    // Case 1: Nothing is wrapped

    run_test(
        300,
        9,
        " 🦀  main.rs [*]
 6│fn main() {
 7│  this_is_a_long_line_for_testing_wrapping();
 8│  █oo {
 9│    x: 2 // this line should be at center
10│  }
11│}
12│// padding 4
 [Global Title]",
    )?;

    // Case 2: Some line is wrapped
    run_test(
        30,
        7,
        " 🦀  main.rs [*]
 6│fn main() {
 8│  █oo {
 9│    x: 2 // this line
 ↪│should be at center
10│  }
 [Global Title]",
    )?;

    // Case 3: available height <= height of `foo` node (3 lines):
    //     center the cursor instead of the middle line of the `foo` node

    run_test(
        300,
        5,
        " 🦀  main.rs [*]
 6│fn main() {
 8│  █oo {
 9│    x: 2 // this line should be at center
 [Global Title]",
    )?;

    Ok(())
}

#[test]
fn align_view_should_work_for_extended_selection() -> anyhow::Result<()> {
    fn run_test(dispatch: DispatchEditor, expected_output: &'static str) -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent(
                    "
// padding 1
// padding 2
// padding 3

xxx
yyy
zzz

// padding 4
// padding 5
// padding 6"
                        .to_string(),
                )),
                App(TerminalDimensionChanged(Dimension {
                    height: 9,
                    width: 300,
                })),
                Editor(MatchLiteral("xxx".to_string())),
                Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Line)),
                Editor(EnableSelectionExtension),
                Editor(MoveSelection(Right)),
                Editor(MoveSelection(Right)),
                Expect(CurrentSelectedTexts(&["xxx\nyyy\nzzz"])),
                Editor(dispatch.clone()),
                Expect(AppGrid(expected_output.to_string())),
            ])
        })
    }
    run_test(
        AlignViewTop,
        " 🦀  main.rs [*]
 6│xxx
 7│yyy
 8│█zz
 9│
10│// padding 4
11│// padding 5
12│// padding 6",
    )?;
    run_test(
        AlignViewCenter,
        " 🦀  main.rs [*]
 5│
 6│xxx
 7│yyy
 8│█zz
 9│
10│// padding 4
11│// padding 5",
    )?;
    run_test(
        AlignViewBottom,
        " 🦀  main.rs [*]
 2│// padding 1
 3│// padding 2
 4│// padding 3
 5│
 6│xxx
 7│yyy
 8│█zz",
    )?;
    Ok(())
}

#[serial]
#[test]
fn copy_paste_special_character_in_word_selection_mode() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("│".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
            Expect(CurrentSelectedTexts(&["│"])),
            Editor(Copy),
            Editor(Paste),
            Expect(CurrentComponentContent("││")),
        ])
    })
}

#[serial]
#[test]
fn recalculate_scroll_offset_consider_last_line_of_multiline_selection() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("│".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
            Expect(CurrentSelectedTexts(&["│"])),
            Editor(Copy),
            Editor(Paste),
            Expect(CurrentComponentContent("││")),
        ])
    })
}

#[test]
fn deleting_selection_extended_with_jump() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("foo bar spam chuck".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
            Expect(CurrentSelectedTexts(&["foo"])),
            // Jump to "spam"
            Editor(SetRectangle(Rectangle {
                origin: Position::default(),
                width: 20,
                height: 5,
            })),
            Editor(EnableSelectionExtension),
            App(HandleKeyEvents(keys!("m s").to_vec())),
            Expect(CurrentSelectedTexts(&["foo bar spam"])),
            Editor(DeleteWithMovement(Right)),
            Expect(CurrentComponentContent("chuck")),
        ])
    })
}

#[test]
fn git_hunk_gutter() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Line)),
            Editor(EnterInsertMode(Direction::End)),
            // Insert one new line
            App(HandleKeyEvents(keys!("enter a l p h a esc").to_vec())),
            // Modify one line
            Editor(MatchLiteral("main".to_string())),
            Editor(DeleteWithMovement(Right)),
            Editor(EnterNormalMode),
            // Delete one line
            Editor(MatchLiteral("println".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Line)),
            Editor(DeleteWithMovement(Left)),
            Editor(EnterNormalMode),
            App(TerminalDimensionChanged(Dimension {
                height: 9,
                width: 20,
            })),
            Expect(EditorGrid(
                r#"🦀  main.rs [*]
1│mod foo;
2│alpha
3│
4│fn () {
5│    █oo::foo();
6│}
7│"#,
            )),
            Expect(GridCellBackground(2, 1, GitGutterStyles::new().insertion)),
            Expect(GridCellBackground(4, 1, GitGutterStyles::new().replacement)),
            Expect(GridCellBackground(6, 1, GitGutterStyles::new().deletion)),
        ])
    })
}

#[test]
fn move_to_hunks_consisting_of_only_a_single_empty_line_and_delete_it() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.gitignore(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Line)),
            // Insert one new empty line
            Editor(BreakSelection),
            // Expect a new line is inserted at the beginning
            Expect(CurrentComponentContent("\ntarget/\n")),
            // Move to the last line of the file
            Editor(MoveSelection(Last)),
            // Move to the hunk created by the new empty line,
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                GitHunk(DiffMode::UnstagedAgainstCurrentBranch),
            )),
            Expect(CurrentSelectedTexts(&[""])),
            // Delete the empty line hunk
            Editor(DeleteOne),
            // Expect the leading new line is deleted
            Expect(CurrentComponentContent("target/\n")),
        ])
    })
}

#[test]
fn git_blame() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Line)),
            Editor(GitBlame),
            Expect(EditorInfoContentMatches(regex!("Commit: [0-9a-f]{40}"))),
            Expect(EditorInfoContentMatches(regex!(r"Author: .+ <[^>]+>"))),
            Expect(EditorInfoContentMatches(regex!(
                r"Date: \d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}"
            ))),
            Expect(EditorInfoContentMatches(regex!("Message: .+"))),
            Expect(EditorInfoContentMatches(regex!("URL: .+"))),
        ])
    })
}

#[test]
fn save_conflict_resolved_by_force_reload() -> anyhow::Result<()> {
    execute_test(move |s| {
        Box::new([
            App(TerminalDimensionChanged(Dimension {
                height: 100,
                width: 300,
            })),
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(EnterInsertMode(Direction::Start)),
            App(HandleKeyEvents(keys!("o u r s enter").to_vec())),
            Shell(
                //sed -i '$a\theirs' filename
                "sed",
                [
                    "-i".to_string(),
                    "$a\\theirs".to_string(),
                    s.main_rs().display_absolute(),
                ]
                .to_vec(),
            ),
            Editor(Save),
            Expect(CurrentComponentTitle(
                "Failed to save src/main.rs: The content of the file is newer.".to_string(),
            )),
            Expect(CompletionDropdownContent("Merge\nForce Save\nForce Reload")),
            App(HandleKeyEvents(keys!("r e l o a d").to_vec())),
            // Expect dropdown info of Force Reload shows the diff of
            // the changes to be made to the EDITOR content
            Expect(CompletionDropdownInfoContent(
                "@@ -1,7 +1,7 @@
-ours
 mod foo;
 
 fn main() {
     foo::foo();
     println!(\"Hello, world!\");
 }
+theirs
",
            )),
            App(HandleKeyEvents(keys!("enter").to_vec())),
            Expect(CurrentComponentContentMatches(regex!("theirs"))),
            Expect(Not(Box::new(EditorIsDirty()))),
            Editor(EnterInsertMode(Direction::Start)),
            // Editing and saving again should be fine
            App(HandleKeyEvents(keys!("n e w").to_vec())),
            Editor(Save),
            Expect(CurrentComponentPath(Some(s.main_rs()))),
        ])
    })
}

#[test]
fn save_conflict_resolved_by_force_save() -> anyhow::Result<()> {
    execute_test(move |s| {
        Box::new([
            App(TerminalDimensionChanged(Dimension {
                height: 100,
                width: 300,
            })),
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(EnterInsertMode(Direction::Start)),
            App(HandleKeyEvents(keys!("o u r s enter").to_vec())),
            Shell(
                //sed -i '$a\theirs' filename
                "sed",
                [
                    "-i".to_string(),
                    "$a\\theirs".to_string(),
                    s.main_rs().display_absolute(),
                ]
                .to_vec(),
            ),
            Editor(Save),
            Expect(CurrentComponentTitle(
                "Failed to save src/main.rs: The content of the file is newer.".to_string(),
            )),
            Expect(CompletionDropdownContent("Merge\nForce Save\nForce Reload")),
            App(HandleKeyEvents(keys!("s a v e").to_vec())),
            // Expect dropdown info of Force Save shows the diff of
            // the changes to be made to the SYSTEM content
            Expect(CompletionDropdownInfoContent(
                "@@ -1,7 +1,7 @@
+ours
 mod foo;
 
 fn main() {
     foo::foo();
     println!(\"Hello, world!\");
 }
-theirs
",
            )),
            App(HandleKeyEvents(keys!("enter").to_vec())),
            Expect(CurrentComponentContentMatches(regex!("ours"))),
            // Editing and saving again should be fine
            Editor(EnterInsertMode(Direction::Start)),
            App(HandleKeyEvents(keys!("n e w").to_vec())),
            Editor(Save),
            Expect(CurrentComponentPath(Some(s.main_rs()))),
        ])
    })
}

#[test]
fn save_conflict_resolved_by_3_way_merge() -> anyhow::Result<()> {
    execute_test(move |s| {
        Box::new([
            App(TerminalDimensionChanged(Dimension {
                height: 100,
                width: 300,
            })),
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(EnterInsertMode(Direction::Start)),
            App(HandleKeyEvents(keys!("o u r s enter").to_vec())),
            Shell(
                //sed -i '$a\theirs' filename
                "sed",
                [
                    "-i".to_string(),
                    "$a\\theirs".to_string(),
                    s.main_rs().display_absolute(),
                ]
                .to_vec(),
            ),
            Editor(Save),
            Expect(CurrentComponentTitle(
                "Failed to save src/main.rs: The content of the file is newer.".to_string(),
            )),
            Expect(CompletionDropdownContent("Merge\nForce Save\nForce Reload")),
            App(HandleKeyEvents(keys!("m e r g e enter").to_vec())),
            Expect(CurrentComponentContentMatches(regex!("(?s)ours.*theirs"))),
            // Editing and saving again should be fine
            Editor(EnterInsertMode(Direction::Start)),
            App(HandleKeyEvents(keys!("n e w").to_vec())),
            Editor(Save),
            Expect(CurrentComponentPath(Some(s.main_rs()))),
        ])
    })
}

#[test]
fn gracefully_reload_buffer_when_there_is_conflict() -> anyhow::Result<()> {
    execute_test(move |s| {
        Box::new([
            App(TerminalDimensionChanged(Dimension {
                height: 100,
                width: 300,
            })),
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(EnterInsertMode(Direction::Start)),
            App(HandleKeyEvents(keys!("o u r s enter").to_vec())),
            Shell(
                //sed -i '$a\theirs' filename
                "sed",
                [
                    "-i".to_string(),
                    "$a\\theirs".to_string(),
                    s.main_rs().display_absolute(),
                ]
                .to_vec(),
            ),
            Editor(ReloadFile { force: false }),
            Expect(CurrentComponentTitle(
                "Failed to save src/main.rs: The content of the file is newer.".to_string(),
            )),
        ])
    })
}

#[test]
fn search_prompt_should_show_words_within_file_as_suggestions() -> anyhow::Result<()> {
    execute_test(move |s| {
        Box::new([
            App(TerminalDimensionChanged(Dimension {
                height: 100,
                width: 300,
            })),
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("snake_case kebab-case camelCase".to_string())),
            App(OpenSearchPrompt {
                scope: Scope::Local,
                if_current_not_found: IfCurrentNotFound::LookForward,
            }),
            // The suggested words should include snake_case, kebab-case and camelCase
            Expect(ExpectKind::CompletionDropdownContent(
                "
camelCase
kebab-case
snake_case
"
                .trim(),
            )),
        ])
    })
}

#[test]
fn last_wrapped_line_with_trailing_newline_char() -> anyhow::Result<()> {
    execute_test(move |s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("foo bar spam baz\n".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Line)),
            App(TerminalDimensionChanged(Dimension {
                height: 10,
                width: 300,
            })),
            // Expect Line 2 is present due to the trailing newline char
            Expect(AppGrid(
                " 🦀  main.rs [*]
1│█oo bar spam baz
2│"
                .to_string(),
            )),
            // Decrease the rendering area to induce text wrapping
            App(TerminalDimensionChanged(Dimension {
                height: 10,
                width: 17,
            })),
            Expect(AppGrid(
                " 🦀  main.rs [*]
1│█oo bar spam
↪│baz
2│"
                .to_string(),
            )),
        ])
    })
}

#[test]
fn align_view_with_cursor_direction_end_and_selection_exceeds_viewport_height() -> anyhow::Result<()>
{
    execute_test(move |s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(
                "
fn main() {
    x();
    y();
    z();
    a();
    b();
    c();
    d();
} // last line
"
                .to_string(),
            )),
            Editor(MatchLiteral("fn".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, SyntaxNode)),
            App(TerminalDimensionChanged(Dimension {
                height: 7,
                width: 300,
            })),
            Editor(SwapCursor),
            Editor(AlignViewTop),
            // Expect the cursor is not gone
            Expect(AppGrid(
                " 🦀  main.rs [*]
 2│fn main() {
10│█ // last line
11│"
                .to_string(),
            )),
        ])
    })
}

#[test]
fn files_longer_than_65535_lines() -> anyhow::Result<()> {
    execute_test(move |s| {
        Box::new([
            App(OpenFile {
                path: s.gitignore(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(
                (0..65536).map(|i| format!("Line {}", i + 1)).join("\n"),
            )),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Line)),
            App(TerminalDimensionChanged(Dimension {
                height: 7,
                width: 300,
            })),
            Editor(MoveSelection(Last)),
            Expect(AppGrid(
                " 🙈  .gitignore [*]
65535│Line 65535
65536│█ine 65536"
                    .to_string(),
            )),
        ])
    })
}

#[test]
fn delete_until_no_more_meaningful_selection_should_not_stuck() -> anyhow::Result<()> {
    execute_test(move |s| {
        Box::new([
            App(OpenFile {
                path: s.gitignore(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("a = hello()".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
            Editor(DeleteWithMovement(Right)),
            Editor(DeleteWithMovement(Right)),
            Expect(CurrentComponentContent("()")),
            Expect(CurrentSelectedTexts(&["("])),
        ])
    })
}

#[test]
fn entering_normal_mode_from_insert_mode_in_scratch_buffer() -> anyhow::Result<()> {
    execute_test(move |_| {
        Box::new([
            Expect(CurrentComponentTitle(
                "[ROOT] (Cannot be saved)".to_string(),
            )),
            Editor(EnterInsertMode(Direction::End)),
            App(HandleKeyEvent(key!("esc"))),
            Expect(CurrentMode(Mode::Normal)),
        ])
    })
}

#[test]
fn align_selections() -> anyhow::Result<()> {
    execute_test(move |s| {
        Box::new([
            App(OpenFile {
                path: s.gitignore(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent(
                "
1)
2)
10)
"
                .to_string(),
            )),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Line)),
            Editor(CursorAddToAllSelections),
            Editor(AlignSelections(Direction::End)),
            Expect(CurrentComponentContent(
                "
 1)
 2)
10)
",
            )),
            Expect(CurrentSelectedTexts(&["1)", "2)", "10)"])),
            Editor(AlignSelections(Direction::Start)),
            Expect(CurrentComponentContent(
                "
 1)
 2)
 10)
",
            )),
            Expect(CurrentSelectedTexts(&["1)", "2)", "10)"])),
        ])
    })
}

#[test]
fn swap_with_intersecting_selections_should_not_elongate_selections() -> anyhow::Result<()> {
    execute_test(move |s| {
        Box::new([
            App(OpenFile {
                path: s.gitignore(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("1.0".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
            Editor(EnterSwapMode),
            Editor(MoveSelection(Next)),
            Editor(MoveSelection(Next)),
            Expect(CurrentComponentContent(".01")),
            Editor(MoveSelection(Previous)),
            Editor(MoveSelection(Previous)),
            Expect(CurrentComponentContent("1.0")),
        ])
    })
}
