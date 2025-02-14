use crate::app::{LocalSearchConfigUpdate, Scope};
use crate::buffer::BufferOwner;
use crate::char_index_range::CharIndexRange;
use crate::clipboard::CopiedTexts;
use crate::components::editor::{DispatchEditor::*, Movement::*};
use crate::context::{LocalSearchConfigMode, Search};
use crate::list::grep::RegexConfig;
use crate::lsp::process::LspNotification;
use crate::quickfix_list::{Location, QuickfixListItem};
use crate::rectangle::Rectangle;
use crate::selection::CharIndex;
use crate::style::Style;
use crate::test_app::*;

use crate::{
    components::editor::{Direction, Mode, ViewAlignment},
    grid::StyleKey,
    position::Position,
    selection::SelectionMode,
    themes::Theme,
};

use itertools::Itertools;
use my_proc_macros::{hex, key, keys};

use SelectionMode::*;

use super::editor::IfCurrentNotFound;
use super::editor::SurroundKind;

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
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                Token {
                    skip_symbols: false,
                },
            )),
            Editor(EnableSelectionExtension),
            Editor(MoveSelection(Right)),
            Editor(MoveSelection(Right)),
            Expect(CurrentSelectedTexts(&["fn f("])),
            Editor(SwapExtensionAnchor),
            Editor(MoveSelection(Right)),
            Expect(CurrentSelectedTexts(&["f("])),
            Editor(Reset),
            Expect(CurrentSelectedTexts(&["f"])),
            Editor(MoveSelection(Right)),
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
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                Token {
                    skip_symbols: false,
                },
            )),
            Editor(Delete(Direction::End)),
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
            Editor(Delete(Direction::End)),
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
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                Token {
                    skip_symbols: false,
                },
            )),
            Editor(Delete(Direction::End)),
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
            Editor(Delete(Direction::End)),
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
            Editor(Delete(Direction::End)),
            Expect(CurrentComponentContent("fn main(a:A) {}")),
            Expect(CurrentSelectedTexts(&["a:A"])),
        ])
    })
}

#[test]
fn delete_should_not_kill_if_not_possible_1() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("fn maima() {}".to_string())),
            Editor(MatchLiteral("ma".to_string())),
            Editor(Delete(Direction::End)),
            Expect(CurrentComponentContent("fn ima() {}")),
            // Expect the current selection is the character after "ma"
            Expect(CurrentSelectedTexts(&["i"])),
        ])
    })
}

#[test]
/// If the current selection is the only selection in the selection mode
fn delete_should_not_kill_if_not_possible_2() -> anyhow::Result<()> {
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
            Editor(Delete(Direction::End)),
            Expect(CurrentComponentContent("fn main() {}")),
            Expect(CurrentSelectedTexts(&[""])),
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
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                Word {
                    skip_symbols: false,
                },
            )),
            Editor(ToggleMark),
            Editor(MoveSelection(Right)),
            Editor(MoveSelection(Right)),
            Editor(ToggleMark),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Mark)),
            Editor(CursorAddToAllSelections),
            Expect(CurrentSelectedTexts(&["foo", "spam"])),
            Editor(CursorKeepPrimaryOnly),
            Expect(CurrentSelectedTexts(&["spam"])),
            Editor(ToggleMark),
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
            Editor(DeleteWordBackward { short: true }),
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
fn test_delete_extended_selection() -> anyhow::Result<()> {
    let run_test = |direction: Direction,
                    expected_selected_texts: &'static [&'static str]|
     -> anyhow::Result<()> {
        execute_test(move |s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("who lives in a pineapple".to_string())),
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    Word {
                        skip_symbols: false,
                    },
                )),
                Editor(MoveSelection(Right)),
                Editor(EnableSelectionExtension),
                Editor(MoveSelection(Right)),
                Expect(CurrentSelectedTexts(&["lives in"])),
                Editor(Delete(direction.clone())),
                Expect(CurrentComponentContent("who a pineapple")),
                Expect(CurrentSelectedTexts(expected_selected_texts)),
            ])
        })
    };
    run_test(Direction::End, &["a"])?;
    run_test(Direction::Start, &["who"])
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
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                Word {
                    skip_symbols: false,
                },
            )),
            Editor(MoveSelection(Right)),
            Editor(EnableSelectionExtension),
            Editor(MoveSelection(Right)),
            Expect(CurrentSelectedTexts(&["lives in"])),
            Editor(Delete(Direction::End)),
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
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                Word {
                    skip_symbols: false,
                },
            )),
            Editor(EnableSelectionExtension),
            Editor(MoveSelection(Right)),
            Expect(CurrentSelectedTexts(&["who lives"])),
            Editor(Delete(Direction::Start)),
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
            Editor(SetContent("who lives in a pineapple".to_string())),
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                Token {
                    skip_symbols: false,
                },
            )),
            Editor(MoveSelection(Right)),
            Editor(SelectAll),
            Editor(Delete(Direction::End)),
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
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                Token {
                    skip_symbols: false,
                },
            )),
            // Go to the middle of the file
            Editor(MoveSelection(Index(3))),
            Expect(CurrentSelectedTexts(&["camelCase"])),
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
            Editor(DeleteWordBackward { short: true }),
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
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                Token {
                    skip_symbols: false,
                },
            )),
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
#[ignore = "Undo tree should be removed soon, I don't use it at all."]
fn undo_tree() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("\n".to_string())),
            Editor(Insert("a".to_string())),
            Editor(Insert("bc".to_string())),
            Editor(EnterUndoTreeMode),
            // Previous = undo
            Editor(MoveSelection(Left)),
            Expect(CurrentComponentContent("a\n")),
            // Next = redo
            Editor(MoveSelection(Right)),
            Expect(CurrentComponentContent("abc\n")),
            Editor(MoveSelection(Left)),
            Expect(CurrentComponentContent("a\n")),
            Editor(Insert("de".to_string())),
            Editor(EnterUndoTreeMode),
            // Down = go to previous history branch
            Editor(MoveSelection(Down)),
            // We are able to retrive the "bc" insertion, which is otherwise impossible without the undo tree
            Expect(CurrentComponentContent("abc\n")),
            // Up = go to next history branch
            Editor(MoveSelection(Up)),
            Expect(CurrentComponentContent("ade\n")),
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
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                Word {
                    skip_symbols: false,
                },
            )),
            Editor(MoveSelection(Right)),
            Editor(MoveSelection(Right)),
            Editor(ToggleMark),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Mark)),
            Expect(CurrentSelectedTexts(&["spim"])),
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                Word {
                    skip_symbols: false,
                },
            )),
            Editor(MoveSelection(Left)),
            Editor(MoveSelection(Left)),
            // Kill "foo"
            Editor(Delete(Direction::End)),
            Expect(CurrentComponentContent("bar spim")),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Mark)),
            // Expect mark position is updated, and still selects "spim"
            Expect(CurrentSelectedTexts(&["spim"])),
            // Remove "spim"
            Editor(Change),
            Expect(CurrentComponentContent("bar ")),
            Editor(EnterNormalMode),
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                Word {
                    skip_symbols: false,
                },
            )),
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
            Editor(Open(Direction::Start)),
            Expect(CurrentMode(Mode::Insert)),
            Editor(Insert("c:C".to_string())),
            Expect(CurrentComponentContent("fn x(c:C, a:A, b:B){}".trim())),
            Editor(EnterNormalMode),
            Editor(MatchLiteral("b:B".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, SyntaxNode)),
            Editor(Open(Direction::Start)),
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
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                Token {
                    skip_symbols: false,
                },
            )),
            Editor(Open(Direction::Start)),
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
            Editor(Open(Direction::End)),
            Expect(CurrentMode(Mode::Insert)),
            Editor(Insert("c:C".to_string())),
            Expect(CurrentComponentContent("fn x(a:A, c:C, b:B){}".trim())),
            Editor(EnterNormalMode),
            Editor(MatchLiteral("b:B".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, SyntaxNode)),
            Expect(CurrentSelectedTexts(&["b:B"])),
            Editor(Open(Direction::End)),
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
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                Token {
                    skip_symbols: false,
                },
            )),
            Expect(CurrentSelectedTexts(&["hello"])),
            Editor(Open(Direction::End)),
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
                use_system_clipboard: false,
            }),
            Editor(MatchLiteral("bar".to_string())),
            Editor(EnterInsertMode(Direction::End)),
            Editor(Paste {
                direction: Direction::End,
                use_system_clipboard: false,
            }),
            Expect(CurrentComponentContent("foo barhaha spam")),
            Editor(Insert("Hello".to_string())),
            Expect(CurrentComponentContent("foo barhahaHello spam")),
        ])
    })
}

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
            Editor(Copy {
                use_system_clipboard: false,
            }),
            Editor(EnterInsertMode(Direction::End)),
            Editor(Paste {
                direction: Direction::End,
                use_system_clipboard: false,
            }),
            Expect(CurrentComponentContent("fn main(a:Aa:A,b:B){}")),
            Editor(Insert("Hello".to_string())),
            Expect(CurrentComponentContent("fn main(a:Aa:AHello,b:B){}")),
        ])
    })
}

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
                use_system_clipboard: false,
            }),
            Editor(MatchLiteral("bar".to_string())),
            Editor(Paste {
                direction: Direction::End,
                use_system_clipboard: false,
            }),
            Expect(CurrentComponentContent("foo barhaha spam")),
            Expect(CurrentSelectedTexts(&["haha"])),
        ])
    })
}

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
            Editor(Copy {
                use_system_clipboard: false,
            }),
            Editor(Paste {
                direction: Direction::End,
                use_system_clipboard: false,
            }),
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

#[test]
fn smart_paste() -> anyhow::Result<()> {
    fn test(direction: Direction, expected_result: &'static str) -> Result<(), anyhow::Error> {
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
                    use_system_clipboard: false,
                }),
                Editor(MatchLiteral("a:A".to_string())),
                Editor(SetSelectionMode(IfCurrentNotFound::LookForward, SyntaxNode)),
                Expect(CurrentSelectedTexts(&["a:A"])),
                Editor(Paste {
                    direction: direction.clone(),
                    use_system_clipboard: false,
                }),
                Expect(CurrentComponentContent(expected_result)),
                Expect(CurrentSelectedTexts(&["c:C"])),
            ])
        })
    }
    test(Direction::End, "fn main(a:A, c:C, b:B) {}")?;
    test(Direction::Start, "fn main(c:C, a:A, b:B) {}")
}

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
                use_system_clipboard: false,
            }),
            Editor(MatchLiteral("bar".to_string())),
            Editor(Paste {
                direction: Direction::Start,
                use_system_clipboard: false,
            }),
            Expect(CurrentComponentContent("foo hahabar spam")),
            Expect(CurrentSelectedTexts(&["haha"])),
        ])
    })
}

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
                use_system_clipboard: false,
            }),
            Editor(ReplaceWithCopiedText {
                cut: false,
                use_system_clipboard: false,
            }),
            Expect(CurrentComponentContent(
                "let z = S(c);fn f(){ let x = S(a); let y = S(b); }",
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
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                Word {
                    skip_symbols: false,
                },
            )),
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
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                Word {
                    skip_symbols: false,
                },
            )),
            Editor(EnterInsertMode(Direction::End)),
            Editor(Insert("hello".to_string())),
            Expect(CurrentComponentContent("fnhello main() {}")),
        ])
    })
}

#[test]
fn highlight_kill() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("fn main() {}".to_string())),
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                Token {
                    skip_symbols: false,
                },
            )),
            Editor(EnableSelectionExtension),
            Editor(MoveSelection(Right)),
            Expect(CurrentSelectedTexts(&["fn main"])),
            Editor(Delete(Direction::End)),
            Expect(CurrentSelectedTexts(&["("])),
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
fn highlight_change() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("hello world yo".to_string())),
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                Word {
                    skip_symbols: false,
                },
            )),
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
            Expect(CurrentSelectedTexts(&["hey"])),
            Editor(ScrollPageDown),
            Expect(CurrentLine("3")),
            Editor(ScrollPageDown),
            Expect(CurrentLine("3")),
            Editor(ScrollPageUp),
            Expect(CurrentLine("2 hey")),
            Editor(ScrollPageUp),
            Expect(CurrentLine("1")),
            Editor(ScrollPageUp),
            Expect(CurrentLine("1")),
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
            Expect(EditorGrid("🦀  src/main.rs [*]\n3│█amma\n4│lok")),
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
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                Word {
                    skip_symbols: false,
                },
            )),
            Editor(ShowJumps {
                use_current_selection_mode: false,
            }),
            // Expect the jump to be the first character of each word
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
🦀  src/main.rs [*]
1│fn main() {
3│  █eta()
4│}
"
                .trim(),
            )),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Line)),
            Editor(ShowJumps {
                use_current_selection_mode: true,
            }),
            Expect(JumpChars(&['f', 'b', '}'])),
            App(HandleKeyEvent(key!("f"))),
            Expect(CurrentSelectedTexts(&["fn main() {"])),
        ])
    })
}

#[test]
fn highlight_and_jump() -> anyhow::Result<()> {
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
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                Word {
                    skip_symbols: false,
                },
            )),
            Editor(MoveSelection(Right)),
            Editor(EnableSelectionExtension),
            Editor(ShowJumps {
                use_current_selection_mode: false,
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
            Editor(ShowJumps {
                use_current_selection_mode: false,
            }),
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                Word {
                    skip_symbols: false,
                },
            )),
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
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                Word {
                    skip_symbols: false,
                },
            )),
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
🦀  src/main.rs [*]
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
            Editor(ToggleMark),
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
                            column_index as usize,
                            parent_lines_background,
                        )))
                    })
                    .collect(),
            ),
            // Mark the "fn" token
            Editor(MatchLiteral("fn".to_string())),
            Editor(ToggleMark),
            // Go to "print()" and skip the first 3 lines for rendering
            Editor(MatchLiteral("print()".to_string())),
            Editor(SetScrollOffset(3)),
            Expect(EditorGrid(
                "
🦀  src/main.rs [*]
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
// hello world\n hey
"
                .trim()
                .to_string(),
            )),
            Editor(SetRectangle(Rectangle {
                origin: Position::default(),
                width: 14,
                height: 4,
            })),
            Editor(MatchLiteral("world".to_string())),
            Editor(EnterInsertMode(Direction::End)),
            Expect(EditorGrid(
                "
🦀  src/main.
1│// hello
↪│world█
2│ hey
"
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
                            range: Position { line: 1, column: 2 }..Position { line: 1, column: 5 },
                        },
                        None,
                    )]
                    .to_vec(),
                ),
            )),
            Expect(ExpectKind::BufferQuickfixListItems(
                [Position { line: 1, column: 2 }..Position { line: 1, column: 5 }].to_vec(),
            )),
            // 1. Testing edit that does not affect the line of the quickfix item
            Editor(MatchLiteral("fn".to_string())),
            Editor(EnterInsertMode(Direction::Start)),
            Editor(Insert("hello".to_string())),
            // 1a. The position range should remain the same
            Expect(ExpectKind::BufferQuickfixListItems(
                [Position { line: 1, column: 2 }..Position { line: 1, column: 5 }].to_vec(),
            )),
            Editor(EnterNormalMode),
            // 2. Testing edit that affects the line of the quickfix item
            Editor(MatchLiteral("let".to_string())),
            Editor(EnterInsertMode(Direction::Start)),
            Editor(Insert("hello".to_string())),
            // 2a. The position range should be updated
            Expect(ExpectKind::BufferQuickfixListItems(
                [Position { line: 1, column: 7 }..Position {
                    line: 1,
                    column: 10,
                }]
                .to_vec(),
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
            Editor(SetContent("fn main() { let x = 123 }".trim().to_string())),
            Editor(SetLanguage(shared::language::from_extension("rs").unwrap())),
            Editor(SetRectangle(Rectangle {
                origin: Position::default(),
                width: 100,
                height: 2,
            })),
            Editor(ApplySyntaxHighlight),
            Expect(ExpectKind::HighlightSpans(
                0..2,
                StyleKey::Syntax("keyword.function".to_string()),
            )),
            Editor(MatchLiteral("fn".to_string())),
            Editor(EnterInsertMode(Direction::Start)),
            Editor(Insert("hello".to_string())),
            Expect(ExpectKind::HighlightSpans(
                5..7,
                StyleKey::Syntax("keyword.function".to_string()),
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
            Editor(SetLanguage(shared::language::from_extension("rs").unwrap())),
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
🦀  src/main.rs [*]
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
                        Some(StyleKey::Syntax("keyword.function".to_string())),
                    )
                })
                .collect(),
            ),
            Expect(
                // Expect the left parenthesis of the outbound parent line "fn main() { // too long" is highlighted properly
                ExpectKind::GridCellStyleKey(
                    Position::new(1, 9),
                    Some(StyleKey::Syntax("punctuation.bracket".to_string())),
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
                        Some(StyleKey::Syntax("keyword".to_string())),
                    )
                })
                .collect(),
            ),
            // Expect decorations overrides syntax highlighting
            Editor(MatchLiteral("fn".to_string())),
            Editor(ToggleMark),
            // Move cursor to next line, so that "fn" is not selected,
            //  so that we can test the style applied to "fn" ,
            // otherwise the style of primary selection anchors will override the mark style
            Editor(MatchLiteral("let".to_string())),
            Expect(EditorGrid(
                "
🦀  src/main.rs [*]
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
            Editor(SetContent("".to_string())),
            Editor(SetRectangle(Rectangle {
                origin: Position::default(),
                width: 20,
                height: 2,
            })),
            Expect(EditorGrid(
                "
🦀  src/main.rs [*]
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
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                Word {
                    skip_symbols: false,
                },
            )),
            Editor(MoveSelection(Right)),
            Editor(MoveSelection(Right)),
            Editor(ToggleMark),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Mark)),
            Expect(CurrentSelectedTexts(&["spim"])),
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                Word {
                    skip_symbols: false,
                },
            )),
            Editor(MoveSelection(Left)),
            Editor(MoveSelection(Left)),
            // Kill "foo"
            Editor(Delete(Direction::End)),
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
            Editor(SetLanguage(shared::language::from_extension("rs").unwrap())),
            Editor(MatchLiteral("bar".to_string())),
            Editor(ToggleMark),
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
            App(HandleKeyEvents(keys!("v s (").to_vec())),
            Editor(SetContent("fn main() { (x.y()) }".to_string())),
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
🦀  src/main.rs [*]
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
            Expect(EditorGrid("🦀  src/main.rs [*]\n1│👩  █bc\n\n\n\n\n\n\n")),
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
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                Word {
                    skip_symbols: false,
                },
            )),
            Expect(CurrentSelectedTexts(&["world"])),
            Editor(Delete(Direction::Start)),
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
    let _ = editor.enter_insert_mode(Direction::End)?;

    let current_range = editor.buffer().tree().unwrap().root_node().range();
    let _ = editor.insert("fn hello() {}")?;
    // Modifying the content in insert mode should not cause the tree to be reparsed
    let new_range = editor.buffer().tree().unwrap().root_node().range();
    assert_eq!(current_range, new_range);

    // Entering normal mode should reparse the tree
    editor.enter_normal_mode()?;
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
                Editor(Delete(Direction::End)),
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
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                Word {
                    skip_symbols: false,
                },
            )),
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
            Expect(CurrentComponentTitle(" 🦀 src/main.rs")),
            Editor(EnterInsertMode(Direction::Start)),
            App(HandleKeyEvents(keys!("a a esc").to_vec())),
            Expect(EditorIsDirty()),
            Expect(CurrentComponentTitle(" 🦀 src/main.rs [*]")),
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
            Expect(CurrentComponentTitle(" 🦀 src/main.rs [*]")),
            Editor(Save),
            Expect(Not(Box::new(EditorIsDirty()))),
            Expect(CurrentComponentTitle(" 🦀 src/main.rs")),
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
                Editor(SetLanguage(shared::language::from_extension("rs").unwrap())),
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
                use_system_clipboard: false,
            }),
            Editor(EnterInsertMode(Direction::Start)),
            Editor(Paste {
                direction: Direction::End,
                use_system_clipboard: false,
            }),
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
            App(HandleKeyEvents(keys!("f u j").to_vec())),
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
            App(HandleKeyEvents(keys!("f o j").to_vec())),
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
            App(HandleKeyEvents(keys!("f h j").to_vec())),
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
            App(HandleKeyEvents(keys!("f m j l").to_vec())),
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
            App(HandleKeyEvents(keys!("f m j l").to_vec())),
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
                        show_config_after_enter: false,
                        if_current_not_found: IfCurrentNotFound::LookForward,
                        run_search_after_config_updated: true,
                    }),
                    App(UpdateLocalSearchConfig {
                        update: LocalSearchConfigUpdate::Search(search_pattern.to_string()),
                        scope: Scope::Local,
                        show_config_after_enter: false,
                        if_current_not_found: IfCurrentNotFound::LookForward,
                        run_search_after_config_updated: true,
                    }),
                    App(UpdateLocalSearchConfig {
                        update: LocalSearchConfigUpdate::Replacement(replace_pattern.to_string()),
                        scope: Scope::Local,
                        show_config_after_enter: false,
                        if_current_not_found: IfCurrentNotFound::LookForward,
                        run_search_after_config_updated: true,
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
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    Token {
                        skip_symbols: false,
                    },
                )),
                Expect(CurrentSelectedTexts(&["a1", "b1", "c1"])),
                Editor(Copy {
                    use_system_clipboard: false,
                }),
                Editor(MoveSelection(Right)),
                Expect(CurrentSelectedTexts(&["a2", "b2", "c2"])),
                Editor(Copy {
                    use_system_clipboard: false,
                }),
                Editor(MoveSelection(Right)),
                Editor(Copy {
                    use_system_clipboard: false,
                }),
                Expect(CurrentSelectedTexts(&["a3", "b3", "c3"])),
                Editor(Paste {
                    direction: Direction::End,
                    use_system_clipboard: false,
                }),
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
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    Word {
                        skip_symbols: false,
                    },
                )),
                Editor(EnterMultiCursorMode),
                Editor(MoveSelection(Right)),
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
                Editor(ToggleMark),
                Editor(MatchLiteral("good".to_string())),
                Editor(ToggleMark),
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
                Editor(SetContent(
                    "
.to_string(),
)),
Editor(MatchLiteral(amos.foo())),
"
                    .to_string(),
                )),
                Editor(MatchLiteral("Editor".to_string())),
                Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Line)),
                App(HandleKeyEvents(keys!("Q ( enter").to_vec())),
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    Token {
                        skip_symbols: false,
                    },
                )),
                Editor(MoveSelection(Left)),
                Expect(CurrentSelectedTexts(&["to_string"])),
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
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    Token {
                        skip_symbols: false,
                    },
                )),
                Editor(MoveSelection(Last)),
                Expect(CurrentSelectedTexts(&["spam"])),
                Editor(MoveSelection(Left)),
                Expect(CurrentSelectedTexts(&["bar"])),
                Editor(Delete(Direction::Start)),
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
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    Word {
                        skip_symbols: false,
                    },
                )),
                Editor(EnableSelectionExtension),
                Editor(MoveSelection(Right)),
                Expect(CurrentSelectedTexts(&["who lives"])),
                Editor(Copy {
                    use_system_clipboard: false,
                }),
                Editor(Paste {
                    direction: Direction::End,
                    use_system_clipboard: false,
                }),
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
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    Token {
                        skip_symbols: false,
                    },
                )),
                Editor(ToggleMark),
                Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Mark)),
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    Token {
                        skip_symbols: false,
                    },
                )),
                Expect(CurrentSelectedTexts(&["who"])),
                Editor(MoveSelection(Last)),
                Expect(CurrentSelectedTexts(&["a"])),
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
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                Word {
                    skip_symbols: false,
                },
            )),
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
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                Word {
                    skip_symbols: false,
                },
            )),
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
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                Token {
                    skip_symbols: false,
                },
            )),
            Editor(MoveSelection(Last)),
            Expect(CurrentPrimarySelection("spam")),
            Editor(EnterMultiCursorMode),
            Editor(MoveSelection(Left)),
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
fn delete_line_should_not_dedent_next_line() -> anyhow::Result<()> {
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
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Line)),
            Editor(Delete(Direction::End)),
            Expect(CurrentComponentContent("  spam\n  hey")),
            Expect(CurrentSelectedTexts(&["spam"])),
            Editor(Undo),
            Expect(CurrentSelectedTexts(&["foo bar"])),
        ])
    })
}

#[test]
fn delete_multiple_lines_should_not_dedent_next_line() -> anyhow::Result<()> {
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
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Line)),
            Editor(EnterExtendMode),
            App(HandleKeyEvent(key!("k"))),
            Expect(CurrentSelectedTexts(&["foo bar\n  spam"])),
            Editor(Delete(Direction::End)),
            Expect(CurrentComponentContent("  hey")),
            Expect(CurrentSelectedTexts(&["hey"])),
            Editor(Undo),
            Expect(CurrentSelectedTexts(&["foo bar\n  spam"])),
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
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                Token { skip_symbols: true },
            )),
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
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                Token {
                    skip_symbols: false,
                },
            )),
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
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                Token { skip_symbols: true },
            )),
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
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                Token {
                    skip_symbols: false,
                },
            )),
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
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                Token { skip_symbols: true },
            )),
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
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                Token {
                    skip_symbols: false,
                },
            )),
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
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                Token {
                    skip_symbols: false,
                },
            )),
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
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                Token {
                    skip_symbols: false,
                },
            )),
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
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                Token {
                    skip_symbols: false,
                },
            )),
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
            Editor(EnterMultiCursorMode),
            Editor(MoveSelection(Down)),
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
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                Token {
                    skip_symbols: false,
                },
            )),
            Expect(CurrentSelectedTexts(&["fooz", "fooy", "foox", "foow"])),
        ])
    })
}

#[test]
fn select_current_line_when_cursor_is_at_last_space_of_current_line() -> anyhow::Result<()> {
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
            Expect(CurrentSelectedTexts(&["abc "])),
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
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                Word {
                    skip_symbols: false,
                },
            )),
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
fn first_last_token() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("who lives in\na pineapple".to_string())),
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                Token {
                    skip_symbols: false,
                },
            )),
            Editor(MoveSelection(Last)),
            Expect(CurrentSelectedTexts(&["in"])),
            Editor(MoveSelection(Down)),
            Editor(MoveSelection(First)),
            Expect(CurrentSelectedTexts(&["a"])),
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
            Editor(EnterMultiCursorMode),
            Editor(MoveSelection(First)),
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
            Editor(EnterMultiCursorMode),
            Editor(MoveSelection(Last)),
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
            Editor(MoveSelection(Right)),
            Expect(CurrentSelectedTexts(&[""])),
            Editor(Delete(Direction::End)),
            Expect(CurrentSelectedTexts(&["world"])),
        ])
    })
}

#[test]
fn empty_lines_navigation() -> anyhow::Result<()> {
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
            Editor(MoveSelection(Right)),
            Expect(CurrentSelectedTexts(&["\n"])),
            Editor(MoveSelection(Up)),
            Expect(CurrentSelectedTexts(&["bar\n"])),
            Editor(MoveSelection(Down)),
            Editor(MoveSelection(Right)),
            Editor(MoveSelection(Up)),
            Expect(CurrentSelectedTexts(&["baz\n"])),
            Editor(MoveSelection(Down)),
            Editor(MoveSelection(Left)),
            Editor(MoveSelection(Down)),
            Expect(CurrentSelectedTexts(&["spam\n"])),
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
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                Token {
                    skip_symbols: false,
                },
            )),
            Expect(CurrentSelectedTexts(&["helloWorld"])),
            Editor(EnterExtendMode),
            App(HandleKeyEvent(key!("l"))),
            Expect(CurrentSelectedTexts(&["helloWorld fooBar"])),
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                Word {
                    skip_symbols: false,
                },
            )),
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
            App(HandleKeyEvents(keys!("n q f o o : : f o o enter").to_vec())),
            Expect(OpenedFilesCount(0)),
            Expect(CurrentComponentTitle(" 🦀 src/main.rs")),
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
                keys!("space f m a i n . r s enter").to_vec(),
            )),
            Expect(CurrentComponentTitle(" 🦀 src/main.rs")),
            Expect(OpenedFilesCount(1)),
        ])
    })
}

#[test]
fn background_editor_closing_no_system_buffer() -> anyhow::Result<()> {
    execute_test(|_| {
        Box::new([
            App(HandleKeyEvents(keys!("n q f o o enter").to_vec())),
            Expect(CurrentComponentTitle(" 🦀 src/foo.rs")),
            Expect(OpenedFilesCount(0)),
            App(CloseCurrentWindow),
            Expect(OpenedFilesCount(0)),
            Expect(CurrentComponentTitle("[ROOT] (Cannot be saved)")),
        ])
    })
}

#[test]
fn toggle_editor_tag() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Expect(CurrentComponentTitle(" 🦀 src/main.rs")),
            App(HandleKeyEvent(key!("1"))),
            Expect(CurrentComponentTitle(" 🦀 src/main.rs #1")),
            App(HandleKeyEvent(key!("1"))),
            Expect(CurrentComponentTitle(" 🦀 src/main.rs")),
        ])
    })
}

#[test]
fn jump_editor_tag() -> anyhow::Result<()> {
    execute_test(|s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            App(HandleKeyEvent(key!("1"))),
            Expect(CurrentComponentTitle(" 🦀 src/main.rs #1")),
            App(OpenFile {
                path: s.foo_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            App(HandleKeyEvent(key!("2"))),
            Expect(CurrentComponentTitle(" 🦀 src/foo.rs #2")),
            App(HandleKeyEvent(key!("1"))),
            Expect(CurrentComponentTitle(" 🦀 src/main.rs #1")),
            App(HandleKeyEvent(key!("2"))),
            Expect(CurrentComponentTitle(" 🦀 src/foo.rs #2")),
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
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                Token { skip_symbols: true },
            )),
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
            Editor(SetContent("hello world()".to_string())),
            Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Line)),
            Editor(SwapCursor),
            App(HandleKeyEvent(key!("s"))), // Token selection mode (Qwerty)
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
            Editor(Insert("hello".to_string())),
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                GitHunk(crate::git::DiffMode::UnstagedAgainstCurrentBranch),
            )),
            Editor(CursorAddToAllSelections),
            Expect(CurrentSelectedTexts(&["hellomod foo;\n"])),
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
🦀  src/main.rs [*]
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
                width: 7,
                height: 2,
            })),
            Editor(SetContent("foo bar".trim().to_string())),
            Editor(MatchLiteral("bar".to_string())),
            Expect(CurrentSelectedTexts(&["bar"])),
            Expect(EditorGrid(
                "
🦀
↪│█ar
"
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
                "
🦀  src/main.rs [*]
1│fn foo() {
2│  fn bar() {
4│        xxx();
5│        █yy();
"
                .trim(),
            )),
        ])
    })
}
