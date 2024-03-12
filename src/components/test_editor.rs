#[cfg(test)]
mod test_editor {
    use crate::components::editor::DispatchEditor::*;
    use crate::components::editor::Movement::*;

    use crate::rectangle::Rectangle;

    use crate::test_app::test_app::*;

    use crate::{
        components::{
            component::Component,
            editor::{Direction, DispatchEditor, Editor, Mode, Movement, ViewAlignment},
        },
        context::Context,
        grid::{Style, StyleKey},
        position::Position,
        selection::{Filter, FilterKind, FilterMechanism, FilterTarget, SelectionMode},
        selection_mode::inside::InsideKind,
        themes::Theme,
    };

    use itertools::Itertools;
    use my_proc_macros::{hex, key, keys};
    use pretty_assertions::assert_eq;
    use serial_test::serial;
    use tree_sitter_rust::language;
    use SelectionMode::*;

    #[test]
    fn raise_bottom_node() -> anyhow::Result<()> {
        execute_test(|s| {
            let input = "fn main() { x + 1 }";
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent(input.to_string())),
                Editor(MatchLiteral("x".to_string())),
                Editor(SetSelectionMode(SelectionMode::BottomNode)),
                Editor(Raise),
                Expect(CurrentComponentContent("fn main() { x }")),
            ])
        })
    }

    #[test]
    /// Example: from "hello" -> hello
    fn raise_inside() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("fn main() { (a, b) }".to_string())),
                Editor(MatchLiteral("b".to_string())),
                Editor(SetSelectionMode(SelectionMode::Inside(
                    InsideKind::Parentheses,
                ))),
                Expect(CurrentSelectedTexts(&["a, b"])),
                Editor(Raise),
                Expect(CurrentComponentContent("fn main() { a, b }")),
            ])
        })
    }

    #[test]
    fn toggle_highlight_mode() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent(
                    "fn f(){ let x = S(a); let y = S(b); }".to_string(),
                )),
                Editor(SetSelectionMode(BottomNode)),
                Editor(ToggleHighlightMode),
                Editor(MoveSelection(Next)),
                Editor(MoveSelection(Next)),
                Expect(CurrentSelectedTexts(&["fn f("])),
                // Toggle the second time should inverse the initial_range
                Editor(ToggleHighlightMode),
                Editor(MoveSelection(Next)),
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
                App(OpenFile(s.main_rs())),
                Editor(SetContent("fn main() {}".to_string())),
                Editor(SetSelectionMode(BottomNode)),
                Editor(Kill),
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
                App(OpenFile(s.main_rs())),
                Editor(SetContent("fn main() {}".to_string())),
                Editor(SetSelectionMode(Character)),
                Editor(Kill),
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
                App(OpenFile(s.main_rs())),
                Editor(SetContent("fn main() {}".to_string())),
                Editor(SetSelectionMode(BottomNode)),
                Editor(MoveSelection(Last)),
                Editor(Kill),
                Expect(CurrentComponentContent("fn main() {")),
            ])
        })
    }

    #[test]
    /// The selection mode is contiguous
    fn delete_should_kill_if_possible_4() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("fn main(a:A,b:B) {}".to_string())),
                Editor(MatchLiteral("a:A".to_string())),
                Editor(SetSelectionMode(SyntaxTree)),
                Editor(Kill),
                Expect(CurrentComponentContent("fn main(b:B) {}")),
                Expect(CurrentSelectedTexts(&["b:B"])),
            ])
        })
    }

    #[test]
    fn delete_should_not_kill_if_not_possible() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("fn maima() {}".to_string())),
                Editor(MatchLiteral("ma".to_string())),
                Editor(Kill),
                Expect(CurrentComponentContent("fn ima() {}")),
                // Expect the current selection is the character after "ma"
                Expect(CurrentSelectedTexts(&["i"])),
            ])
        })
    }

    #[test]
    fn toggle_untoggle_bookmark() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("foo bar spam".to_string())),
                Editor(SetSelectionMode(Word)),
                Editor(ToggleBookmark),
                Editor(MoveSelection(Next)),
                Editor(MoveSelection(Next)),
                Editor(ToggleBookmark),
                Editor(SetSelectionMode(Bookmark)),
                Editor(CursorAddToAllSelections),
                Expect(CurrentSelectedTexts(&["foo", "spam"])),
                Editor(CursorKeepPrimaryOnly),
                Expect(CurrentSelectedTexts(&["spam"])),
                Editor(ToggleBookmark),
                Editor(MoveSelection(Current)),
                Editor(CursorAddToAllSelections),
                Expect(CurrentSelectedTexts(&["foo"])),
            ])
        })
    }

    #[test]
    fn test_delete_word_backward_from_end_of_file() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent(
                    "fn snake_case(camelCase: String) {}".to_string(),
                )),
                Editor(SetSelectionMode(LineTrimmed)),
                // Go to the end of the file
                Editor(EnterInsertMode(Direction::End)),
                Editor(DeleteWordBackward),
                Expect(CurrentComponentContent(
                    "fn snake_case(camelCase: String) {",
                )),
                Editor(DeleteWordBackward),
                Expect(CurrentComponentContent("fn snake_case(camelCase: String) ")),
                Editor(DeleteWordBackward),
                Expect(CurrentComponentContent("fn snake_case(camelCase: String")),
                Editor(DeleteWordBackward),
                Expect(CurrentComponentContent("fn snake_case(camelCase: ")),
                Editor(DeleteWordBackward),
            ])
        })
    }

    #[test]
    fn test_delete_word_backward_from_middle_of_file() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent(
                    "fn snake_case(camelCase: String) {}".to_string(),
                )),
                Editor(SetSelectionMode(BottomNode)),
                // Go to the middle of the file
                Editor(MoveSelection(Index(3))),
                Expect(CurrentSelectedTexts(&["camelCase"])),
                Editor(EnterInsertMode(Direction::End)),
                Editor(DeleteWordBackward),
                Expect(CurrentComponentContent("fn snake_case(camel: String) {}")),
                Editor(DeleteWordBackward),
                Expect(CurrentComponentContent("fn snake_case(: String) {}")),
                Editor(DeleteWordBackward),
                Expect(CurrentComponentContent("fn snake_case: String) {}")),
                Editor(DeleteWordBackward),
                Expect(CurrentComponentContent("fn snake_: String) {}")),
                Editor(DeleteWordBackward),
                Expect(CurrentComponentContent("fn : String) {}")),
                Editor(DeleteWordBackward),
                Expect(CurrentComponentContent(": String) {}")),
                Editor(DeleteWordBackward),
                Expect(CurrentComponentContent(": String) {}")),
                Editor(DeleteWordBackward),
            ])
        })
    }

    #[test]
    fn kill_line_to_end() -> anyhow::Result<()> {
        let input = "lala\nfoo bar spam\nyoyo";
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
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
                App(OpenFile(s.main_rs())),
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
    fn undo_tree() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("\n".to_string())),
                Editor(Insert("a".to_string())),
                Editor(Insert("bc".to_string())),
                Editor(EnterUndoTreeMode),
                // Previous = undo
                Editor(MoveSelection(Previous)),
                Expect(CurrentComponentContent("a\n")),
                // Next = redo
                Editor(MoveSelection(Next)),
                Expect(CurrentComponentContent("abc\n")),
                Editor(MoveSelection(Previous)),
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
    fn multi_exchange_sibling() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("fn f(x:a,y:b){} fn g(x:a,y:b){}".to_string())),
                Editor(MatchLiteral("fn f(x:a,y:b){}".to_string())),
                Expect(CurrentSelectedTexts(&["fn f(x:a,y:b){}"])),
                Editor(SetSelectionMode(SyntaxTree)),
                Editor(CursorAddToAllSelections),
                Expect(CurrentSelectedTexts(&[
                    "fn f(x:a,y:b){}",
                    "fn g(x:a,y:b){}",
                ])),
                Editor(MoveSelection(FirstChild)),
                Editor(MoveSelection(Next)),
                Editor(MoveSelection(FirstChild)),
                Expect(CurrentSelectedTexts(&["x:a", "x:a"])),
                Editor(SetSelectionMode(SyntaxTree)),
                Editor(EnterExchangeMode),
                Editor(MoveSelection(Next)),
                Expect(CurrentComponentContent("fn f(y:b,x:a){} fn g(y:b,x:a){}")),
                Expect(CurrentSelectedTexts(&["x:a", "x:a"])),
                Editor(MoveSelection(Previous)),
                Expect(CurrentComponentContent("fn f(x:a,y:b){} fn g(x:a,y:b){}")),
            ])
        })
    }

    #[test]
    fn update_bookmark_position() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("foo bar spim".to_string())),
                Editor(SetSelectionMode(Word)),
                Editor(MoveSelection(Next)),
                Editor(MoveSelection(Next)),
                Editor(ToggleBookmark),
                Editor(SetSelectionMode(Bookmark)),
                Expect(CurrentSelectedTexts(&["spim"])),
                Editor(SetSelectionMode(Word)),
                Editor(MoveSelection(Previous)),
                Editor(MoveSelection(Previous)),
                // Kill "foo"
                Editor(Kill),
                Expect(CurrentComponentContent("bar spim")),
                Editor(SetSelectionMode(Bookmark)),
                // Expect bookmark position is updated, and still selects "spim"
                Expect(CurrentSelectedTexts(&["spim"])),
                // Remove "m" from "spim"
                Editor(EnterInsertMode(Direction::End)),
                Editor(Backspace),
                Expect(CurrentComponentContent("bar spi")),
                Editor(EnterNormalMode),
                Editor(SetSelectionMode(Bookmark)),
                // Expect the "spim" bookmark is removed
                // By the fact that "spi" is not selected
                Expect(CurrentSelectedTexts(&["i"])),
            ])
        })
    }

    #[test]
    fn move_to_line_start_end() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("hello\n".to_string())),
                Editor(EnterInsertMode(Direction::Start)),
                Editor(MoveToLineEnd),
                Editor(Insert(" world".to_string())),
                Expect(CurrentComponentContent("hello world\n")),
                Editor(MoveToLineStart),
                Editor(Insert("hey ".to_string())),
                Expect(CurrentComponentContent("hey hello world\n")),
            ])
        })
    }

    #[test]
    fn exchange_sibling() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("fn main(x: usize, y: Vec<A>) {}".to_string())),
                // Select first statement
                Editor(MatchLiteral("x: usize".to_string())),
                Editor(SetSelectionMode(SyntaxTree)),
                Editor(EnterExchangeMode),
                Editor(MoveSelection(Next)),
                Expect(CurrentComponentContent("fn main(y: Vec<A>, x: usize) {}")),
                Editor(MoveSelection(Previous)),
                Expect(CurrentComponentContent("fn main(x: usize, y: Vec<A>) {}")),
            ])
        })
    }

    #[test]
    fn exchange_sibling_2() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("use a;\nuse b;\nuse c;".to_string())),
                // Select first statement
                Editor(SetSelectionMode(TopNode)),
                Editor(SetSelectionMode(SyntaxTree)),
                Expect(CurrentSelectedTexts(&["use a;"])),
                Editor(EnterExchangeMode),
                Editor(MoveSelection(Next)),
                Expect(CurrentComponentContent("use b;\nuse a;\nuse c;")),
                Editor(MoveSelection(Next)),
                Expect(CurrentComponentContent("use b;\nuse c;\nuse a;")),
            ])
        })
    }

    #[test]
    fn select_character() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("fn main() { let x = 1; }".to_string())),
                Editor(SetSelectionMode(Character)),
                Expect(CurrentSelectedTexts(&["f"])),
                Editor(MoveSelection(Next)),
                Expect(CurrentSelectedTexts(&["n"])),
                Editor(MoveSelection(Previous)),
                Expect(CurrentSelectedTexts(&["f"])),
            ])
        })
    }

    #[test]
    fn raise() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("fn main() { let x = a.b(c()); }".to_string())),
                Editor(MatchLiteral("c()".to_string())),
                Editor(SetSelectionMode(SyntaxTree)),
                Editor(Raise),
                Expect(CurrentComponentContent("fn main() { let x = c(); }")),
                Editor(Raise),
                Expect(CurrentComponentContent("fn main() { c() }")),
            ])
        })
    }

    #[test]
    fn select_kids() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("fn main(x: usize, y: Vec<A>) {}".to_string())),
                Editor(MatchLiteral("x".to_string())),
                Editor(MoveSelection(Next)),
                Expect(CurrentSelectedTexts(&["x"])),
                Editor(SelectKids),
                Expect(CurrentSelectedTexts(&["x: usize, y: Vec<A>"])),
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
                App(OpenFile(s.main_rs())),
                Editor(SetContent("fn main() { Some((a).b()) }".to_string())),
                Editor(MatchLiteral("(a).b()".to_string())),
                Editor(SetSelectionMode(SyntaxTree)),
                Editor(Raise),
                Expect(CurrentComponentContent("fn main() { (a).b() }")),
            ])
        })
    }

    #[test]
    fn multi_raise() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent(
                    "fn f(){ let x = S(a); let y = S(b); }".to_string(),
                )),
                Editor(MatchLiteral("let x = S(a);".to_string())),
                Editor(SetSelectionMode(SyntaxTree)),
                Editor(CursorAddToAllSelections),
                Editor(MoveSelection(FirstChild)),
                Editor(MoveSelection(Next)),
                Editor(MoveSelection(FirstChild)),
                Editor(MoveSelection(Next)),
                Editor(MoveSelection(FirstChild)),
                Editor(MoveSelection(Next)),
                Expect(CurrentSelectedTexts(&["a", "b"])),
                Editor(Raise),
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
    fn open_new_line() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent(
                    "
fn f() {
    let x = S(a);
}
"
                    .trim()
                    .to_string(),
                )),
                Editor(MatchLiteral("let x = ".to_string())),
                Editor(OpenNewLine),
                Editor(Insert("let y = S(b);".to_string())),
                Expect(CurrentComponentContent(
                    "
fn f() {
    let x = S(a);
    let y = S(b);
}"
                    .trim(),
                )),
            ])
        })
    }

    #[test]
    fn exchange_line() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent(
                    // Multiline source code
                    "
fn main() {
    let x = 1;
    let y = 2;
}"
                    .trim()
                    .to_string(),
                )),
                Editor(SetSelectionMode(LineTrimmed)),
                Editor(Exchange(Next)),
                Expect(CurrentComponentContent(
                    "
let x = 1;
    fn main() {
    let y = 2;
}"
                    .trim(),
                )),
                Editor(Exchange(Previous)),
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
    fn exchange_character() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("fn main() { let x = 1; }".to_string())),
                Editor(SetSelectionMode(Character)),
                Editor(EnterExchangeMode),
                App(HandleKeyEvent(key!("l"))),
                // Editor(MoveSelection(Next)),
                Expect(CurrentComponentContent("nf main() { let x = 1; }")),
                Editor(MoveSelection(Next)),
                Expect(CurrentComponentContent("n fmain() { let x = 1; }")),
                Editor(MoveSelection(Previous)),
                Expect(CurrentComponentContent("nf main() { let x = 1; }")),
                Editor(MoveSelection(Previous)),
                Expect(CurrentComponentContent("fn main() { let x = 1; }")),
            ])
        })
    }

    #[test]
    fn multi_insert() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("struct A(usize, char)".to_string())),
                Editor(MatchLiteral("usize".to_string())),
                Editor(SetSelectionMode(SyntaxTree)),
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
    fn paste_from_clipboard() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent(
                    "fn f(){ let x = S(a); let y = S(b); }".to_string(),
                )),
                App(SetClipboardContent("let z = S(c);".to_string())),
                Editor(Paste),
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
                App(OpenFile(s.main_rs())),
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
                App(OpenFile(s.main_rs())),
                Editor(SetContent("fn main() {}".to_string())),
                Editor(SetSelectionMode(Word)),
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
                App(OpenFile(s.main_rs())),
                Editor(SetContent("fn main() {}".to_string())),
                Editor(SetSelectionMode(Word)),
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
                App(OpenFile(s.main_rs())),
                Editor(SetContent("fn main() {}".to_string())),
                Editor(SetSelectionMode(BottomNode)),
                Editor(ToggleHighlightMode),
                Editor(MoveSelection(Next)),
                Expect(CurrentSelectedTexts(&["fn main"])),
                Editor(Kill),
                Expect(CurrentSelectedTexts(&["("])),
            ])
        })
    }

    #[test]
    fn multicursor_add_all() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent(
                    "mod m { fn a(j:J){} fn b(k:K,l:L){} fn c(m:M,n:N,o:O){} }".to_string(),
                )),
                Editor(MatchLiteral("fn a".to_string())),
                Editor(SetSelectionMode(SyntaxTree)),
                Expect(CurrentSelectedTexts(&["fn a(j:J){}"])),
                Editor(CursorAddToAllSelections),
                Editor(MoveSelection(FirstChild)),
                Editor(MoveSelection(Next)),
                Editor(MoveSelection(FirstChild)),
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
                App(OpenFile(s.main_rs())),
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
                App(OpenFile(s.main_rs())),
                Editor(SetContent("hello world yo".to_string())),
                Editor(SetSelectionMode(Word)),
                Editor(ToggleHighlightMode),
                Editor(MoveSelection(Next)),
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
                App(OpenFile(s.main_rs())),
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
    fn jump() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
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
                Editor(SetSelectionMode(Word)),
                Editor(DispatchEditor::Jump),
                // Expect the jump to be the first character of each word
                // Note 'y' and 'd' are excluded because they are out of view,
                // since the viewbox has only height of 1
                Expect(JumpChars(&['w', 'l', 'o', 's', 's', '?'])),
                App(HandleKeyEvent(key!("s"))),
                Expect(JumpChars(&['a', 'b'])),
                App(HandleKeyEvent(key!("a"))),
                Expect(JumpChars(&[])),
                Expect(CurrentSelectedTexts(&["sea"])),
            ])
        })
    }

    #[test]
    fn highlight_and_jump() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent(
                    "Who lives on sea shore?\n yonky donkey".to_string(),
                )),
                Editor(SetRectangle(Rectangle {
                    origin: Position::default(),
                    width: 100,
                    height: 1,
                })),
                Editor(SetSelectionMode(Word)),
                Editor(MoveSelection(Next)),
                Editor(ToggleHighlightMode),
                Editor(DispatchEditor::Jump),
                // Expect the jump to be the first character of each word
                // Note 'y' and 'd' are excluded because they are out of view,
                // since the viewbox has only height of 1
                Expect(JumpChars(&['w', 'l', 'o', 's', 's', '?'])),
                App(HandleKeyEvent(key!("s"))),
                App(HandleKeyEvent(key!("b"))),
                Expect(CurrentSelectedTexts(&["lives on sea shore"])),
            ])
        })
    }

    #[test]
    fn jump_all_selection_start_with_same_char() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("who who who who".to_string())),
                Editor(SetRectangle(Rectangle {
                    origin: Position::default(),
                    width: 100,
                    height: 1,
                })),
                Editor(SetSelectionMode(Word)),
                Editor(DispatchEditor::Jump),
                // Expect the jump to NOT be the first character of each word
                // Since, the first character of each selection are the same, which is 'w'
                Expect(JumpChars(&['a', 'b', 'c', 'd'])),
            ])
        })
    }

    #[test]
    fn switch_view_alignment() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
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
                Editor(SetSelectionMode(Word)),
                Editor(MoveSelection(Next)),
                Editor(MoveSelection(Next)),
                Expect(CurrentSelectedTexts(&["c"])),
                Expect(CurrentViewAlignment(None)),
                Editor(SwitchViewAlignment),
                Expect(CurrentViewAlignment(Some(ViewAlignment::Top))),
                Editor(SwitchViewAlignment),
                Expect(CurrentViewAlignment(Some(ViewAlignment::Center))),
                Editor(SwitchViewAlignment),
                Expect(CurrentViewAlignment(Some(ViewAlignment::Bottom))),
                Editor(MoveSelection(Previous)),
                Expect(CurrentViewAlignment(None)),
            ])
        })
    }

    #[test]
    fn get_grid_parent_line() -> anyhow::Result<()> {
        let parent_lines_background = hex!("#badbad");
        let bookmark_background_color = hex!("#cebceb");
        let theme = {
            let mut theme = Theme::default();
            theme.ui.parent_lines_background = parent_lines_background;
            theme.ui.bookmark = Style::default().background_color(bookmark_background_color);
            theme
        };
        let width = 20;
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent(
                    "
// hello
fn main() {
  let x = 1;
  let y = 2;
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
                    height: 6,
                })),
                App(SetTheme(theme.clone())),
                // Go to "print()" and skip the first 3 lines for rendering
                Editor(MatchLiteral("print()".to_string())),
                Editor(SetScrollOffset(3)),
                // Expect `fn main()` is visible although it is out of view,
                // because it is amongst the parent lines of the current selection
                Expect(EditorGrid(
                    "
src/main.rs 🦀
2│fn main() {
4│  let y = 2;
5│  for a in b {
6│    let z = 4;
7│    █rint()
"
                    .trim(),
                )),
                // Bookmart "z"
                Editor(MatchLiteral("z".to_string())),
                Editor(ToggleBookmark),
                // Expect the parent lines of the current selections are highlighted with parent_lines_background,
                // regardless of whether the parent lines are inbound or outbound
                ExpectMulti(
                    [1, 3]
                        .into_iter()
                        .flat_map(|row_index| {
                            [0, width - 1].into_iter().map(move |column_index| {
                                GridCellBackground(
                                    row_index,
                                    column_index as usize,
                                    parent_lines_background,
                                )
                            })
                        })
                        .collect(),
                ),
                // Expect the current line is not treated as parent line
                ExpectMulti(
                    [0, width - 1]
                        .into_iter()
                        .map(|column_index| {
                            Not(Box::new(GridCellBackground(
                                5,
                                column_index as usize,
                                parent_lines_background,
                            )))
                        })
                        .collect(),
                ),
                // Bookmark the "fn" token
                Editor(MatchLiteral("fn".to_string())),
                Editor(ToggleBookmark),
                // Go to "print()" and skip the first 3 lines for rendering
                Editor(MatchLiteral("print()".to_string())),
                Editor(SetScrollOffset(3)),
                Expect(EditorGrid(
                    "
src/main.rs 🦀
2│fn main() {
4│  let y = 2;
5│  for a in b {
6│    let z = 4;
7│    █rint()
"
                    .trim(),
                )),
                // Expect the bookmarks of outbound parent lines are rendered properly
                // In this case, the outbound parent line is "fn main() {"
                ExpectMulti(
                    [2, 3]
                        .into_iter()
                        .map(|column_index| {
                            GridCellBackground(1, column_index as usize, bookmark_background_color)
                        })
                        .collect(),
                ),
                // Expect the bookmarks of inbound lines are rendered properly
                // In this case, we want to check that the bookmark on "z" is rendered
                Expect(GridCellBackground(4, 10, bookmark_background_color)),
            ])
        })
    }

    #[test]
    fn test_wrapped_lines() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent(
                    "
// hello world\n hey
"
                    .trim()
                    .to_string(),
                )),
                Editor(SetRectangle(Rectangle {
                    origin: Position::default(),
                    width: 13,
                    height: 4,
                })),
                Editor(MatchLiteral("world".to_string())),
                Editor(EnterInsertMode(Direction::End)),
                Expect(EditorGrid(
                    "
src/main.rs
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
    fn syntax_highlighting() -> anyhow::Result<()> {
        execute_test(|s| {
            let theme = Theme::default();
            Box::new([
                App(OpenFile(s.main_rs())),
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
                    width: 13,
                    height: 4,
                })),
                Editor(SetLanguage(shared::language::from_extension("rs").unwrap())),
                Editor(MatchLiteral("bar".to_string())),
                Editor(DispatchEditor::ApplySyntaxHighlight),
                Editor(SetRectangle(Rectangle {
                    origin: Position::default(),
                    width: 20,
                    height: 4,
                })),
                Editor(SwitchViewAlignment),
                // The "long" of "too long" is not shown, because it exceeded the view width
                Expect(EditorGrid(
                    "
src/main.rs 🦀
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
                        //
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
                        ExpectKind::GridCellStyleKey(position, Some(StyleKey::SyntaxKeyword))
                    })
                    .collect(),
                ),
                // Expect decorations overrides syntax highlighting
                Editor(MatchLiteral("fn".to_string())),
                Editor(ToggleBookmark),
                // Move cursor to next line, so that "fn" is not selected,
                //  so that we can test the style applied to "fn" ,
                // otherwise the style of primary selection anchors will override the bookmark style
                Editor(MatchLiteral("let".to_string())),
                Expect(EditorGrid(
                    "
src/main.rs 🦀
1│fn main() { // too
↪│ long
2│  █et foo = 1;
"
                    .trim(),
                )),
                ExpectMulti(
                    [Position::new(1, 2), Position::new(1, 3)]
                        .into_iter()
                        .map(|position| {
                            ExpectKind::GridCellStyleKey(position, Some(StyleKey::UiBookmark))
                        })
                        .collect(),
                ),
            ])
        })
    }

    #[test]
    fn empty_content_should_have_one_line() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("".to_string())),
                Editor(SetRectangle(Rectangle {
                    origin: Position::default(),
                    width: 20,
                    height: 2,
                })),
                Expect(EditorGrid(
                    "
src/main.rs 🦀
1│█
"
                    .trim(),
                )),
            ])
        })
    }

    #[test]
    fn update_bookmark_position_with_undo_and_redo() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("foo bar spim".to_string())),
                Editor(SetSelectionMode(Word)),
                Editor(MoveSelection(Next)),
                Editor(MoveSelection(Next)),
                Editor(ToggleBookmark),
                Editor(SetSelectionMode(Bookmark)),
                Expect(CurrentSelectedTexts(&["spim"])),
                Editor(SetSelectionMode(Word)),
                Editor(MoveSelection(Previous)),
                Editor(MoveSelection(Previous)),
                // Kill "foo"
                Editor(Kill),
                Expect(CurrentComponentContent("bar spim")),
                // Expect bookmark position is updated (still selects "spim")
                Editor(SetSelectionMode(Bookmark)),
                Expect(CurrentSelectedTexts(&["spim"])),
                Editor(Undo),
                Expect(CurrentComponentContent("foo bar spim")),
                // Expect bookmark position is updated (still selects "spim")
                Editor(SetSelectionMode(Bookmark)),
                Expect(CurrentSelectedTexts(&["spim"])),
                Editor(Redo),
                // Expect bookmark position is updated (still selects "spim")
                Editor(SetSelectionMode(Bookmark)),
                Expect(CurrentSelectedTexts(&["spim"])),
            ])
        })
    }

    #[test]
    fn saving_should_not_destroy_bookmark_if_selections_not_modified() -> anyhow::Result<()> {
        let input = "// foo bar spim\n    fn foo() {}\n";

        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent(input.to_string())),
                Editor(SetLanguage(shared::language::from_extension("rs").unwrap())),
                Editor(MatchLiteral("bar".to_string())),
                Editor(ToggleBookmark),
                Editor(SetSelectionMode(Bookmark)),
                Editor(Save),
                // Expect the content is formatted (second line dedented)
                Expect(CurrentComponentContent("// foo bar spim\nfn foo() {}\n")),
                Editor(SetSelectionMode(Character)),
                Expect(CurrentSelectedTexts(&["b"])),
                // Expect the bookmark on "bar" is not destroyed
                Editor(SetSelectionMode(Bookmark)),
                Expect(CurrentSelectedTexts(&["bar"])),
            ])
        })
    }

    #[test]
    fn omit() -> Result<(), anyhow::Error> {
        fn run_test(
            (input, kind, target, mechanism, expected_output): (
                &str,
                FilterKind,
                FilterTarget,
                FilterMechanism,
                &'static [&'static str],
            ),
        ) -> anyhow::Result<()> {
            execute_test(|s| {
                Box::new([
                    App(OpenFile(s.main_rs())),
                    Editor(SetContent(input.to_string())),
                    Editor(SetSelectionMode(SelectionMode::Word)),
                    Editor(FilterPush(Filter::new(kind, target, mechanism.clone()))),
                    Editor(DispatchEditor::CursorAddToAllSelections),
                    Expect(CurrentSelectedTexts(expected_output)),
                    Editor(FilterClear),
                    Editor(CursorKeepPrimaryOnly),
                    Editor(CursorAddToAllSelections),
                    Expect(Not(Box::new(CurrentSelectedTexts(expected_output)))),
                ])
            })
        }
        use regex::Regex as R;
        use FilterKind::*;
        use FilterMechanism::*;
        use FilterTarget::*;
        let cases: &[(&str, FilterKind, FilterTarget, FilterMechanism, &[&str])] = &[
            (
                "foo bar spam",
                Keep,
                Content,
                Literal("a".to_string()),
                &["bar", "spam"],
            ),
            (
                "foo bar spam",
                Keep,
                Content,
                Literal("a".to_string()),
                &["bar", "spam"],
            ),
            (
                "foo bar spam",
                Remove,
                Content,
                Literal("a".to_string()),
                &["foo"],
            ),
            (
                "hello wehello",
                Keep,
                Content,
                Regex(R::new(r"^he")?),
                &["hello"],
            ),
            (
                "hello wehello",
                Remove,
                Content,
                Regex(R::new(r"^he")?),
                &["wehello"],
            ),
        ];
        for case in cases.to_owned() {
            run_test(case)?;
        }

        Ok(())
    }

    #[test]
    fn surround() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("fn main() { x.y() }".to_string())),
                Editor(MatchLiteral("x.y()".to_string())),
                App(HandleKeyEvents(keys!("( { [ < ' `").to_vec())),
                App(HandleKeyEvent(key!('"'))),
                Expect(CurrentComponentContent(
                    "fn main() { \"`'<[{(x.y())}]>'`\" }",
                )),
                Expect(CurrentSelectedTexts(&["\"`'<[{(x.y())}]>'`\""])),
            ])
        })
    }
}
