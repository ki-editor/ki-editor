#[cfg(test)]
mod test_editor {

    use crate::{
        components::{
            component::Component,
            editor::{Direction, Editor, Mode, Movement},
        },
        context::Context,
        position::Position,
        selection::SelectionMode,
    };

    use my_proc_macros::{key, keys};
    use pretty_assertions::assert_eq;
    use tree_sitter_rust::language;

    #[test]
    fn select_character() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main() { let x = 1; }");
        let context = Context::default();
        editor.set_selection_mode(&context, SelectionMode::Character)?;
        assert_eq!(editor.get_selected_texts(), vec!["f"]);
        editor.move_selection(&context, Movement::Next)?;
        assert_eq!(editor.get_selected_texts(), vec!["n"]);

        editor.move_selection(&context, Movement::Previous)?;
        assert_eq!(editor.get_selected_texts(), vec!["f"]);
        Ok(())
    }

    #[test]
    fn select_kids() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main(x: usize, y: Vec<A>) {}");
        let context = Context::new();

        editor.match_literal(&context, "x")?;
        editor.move_selection(&context, Movement::Next)?;

        assert_eq!(editor.get_selected_texts(), vec!["x"]);

        editor.select_kids()?;
        assert_eq!(editor.get_selected_texts(), vec!["x: usize, y: Vec<A>"]);
        Ok(())
    }

    #[test]
    fn exchange_sibling() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main(x: usize, y: Vec<A>) {}");
        let context = Context::default();
        editor.set_selection_mode(&context, SelectionMode::OutermostNode)?;
        // Move token to "x: usize"
        for _ in 0..3 {
            editor.move_selection(&context, Movement::Next)?;
        }

        assert_eq!(editor.get_selected_texts(), vec!["x: usize"]);

        editor.set_selection_mode(&context, SelectionMode::SyntaxTree)?;
        editor.exchange(Movement::Next, &context)?;
        assert_eq!(editor.text(), "fn main(y: Vec<A>, x: usize) {}");

        editor.exchange(Movement::Previous, &context)?;
        assert_eq!(editor.text(), "fn main(x: usize, y: Vec<A>) {}");
        Ok(())
    }

    #[test]
    fn exchange_sibling_2() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "use a;\nuse b;\nuse c;");
        let context = Context::default();

        // Select first statement
        editor.set_selection_mode(&context, SelectionMode::Token)?;
        editor.set_selection_mode(&context, SelectionMode::SyntaxTree)?;
        editor.move_selection(&context, Movement::Up)?;
        assert_eq!(editor.get_selected_texts(), vec!["use a;"]);

        editor.set_selection_mode(&context, SelectionMode::SyntaxTree)?;
        editor.exchange(Movement::Next, &context)?;
        assert_eq!(editor.text(), "use b;\nuse a;\nuse c;");
        editor.exchange(Movement::Next, &context)?;
        assert_eq!(editor.text(), "use b;\nuse c;\nuse a;");
        Ok(())
    }

    #[test]
    fn raise() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main() { let x = a.b(c()); }");
        let context = Context::default();
        // Move selection to "c()"
        editor.match_literal(&context, "c()")?;
        assert_eq!(editor.get_selected_texts(), vec!["c()"]);

        editor.raise(&context)?;
        assert_eq!(editor.text(), "fn main() { let x = c(); }");

        editor.raise(&context)?;
        assert_eq!(editor.text(), "fn main() { c() }");
        Ok(())
    }

    #[test]
    fn exchange_line() -> anyhow::Result<()> {
        // Multiline source code
        let mut editor = Editor::from_text(
            language(),
            "
fn main() {
    let x = 1;
    let y = 2;
}",
        );

        let context = Context::default();
        editor.select_line(Movement::Next, &context)?;
        editor.select_line(Movement::Next, &context)?;

        editor.exchange(Movement::Next, &context)?;
        assert_eq!(
            editor.text(),
            "
    let x = 1;
fn main() {
    let y = 2;
}"
        );

        editor.exchange(Movement::Previous, &context)?;
        assert_eq!(
            editor.text(),
            "
fn main() {
    let x = 1;
    let y = 2;
}"
        );
        Ok(())
    }

    #[test]
    fn exchange_character() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main() { let x = 1; }");
        let context = Context::default();
        editor.set_selection_mode(&context, SelectionMode::Character)?;
        editor.exchange(Movement::Next, &context)?;
        assert_eq!(editor.text(), "nf main() { let x = 1; }");
        editor.exchange(Movement::Next, &context)?;
        assert_eq!(editor.text(), "n fmain() { let x = 1; }");

        editor.exchange(Movement::Previous, &context)?;
        assert_eq!(editor.text(), "nf main() { let x = 1; }");
        editor.exchange(Movement::Previous, &context)?;

        assert_eq!(editor.text(), "fn main() { let x = 1; }");
        Ok(())
    }

    #[test]
    fn multi_insert() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "struct A(usize, char)");
        let context = Context::default();

        // Select 'usize'
        editor.match_literal(&context, "usize")?;
        assert_eq!(editor.get_selected_texts(), vec!["usize"]);

        editor.set_selection_mode(&context, SelectionMode::SyntaxTree)?;
        editor.add_cursor(&Movement::Next, &context)?;

        assert_eq!(editor.get_selected_texts(), vec!["usize", "char"]);
        editor.enter_insert_mode(Direction::Start)?;
        editor.insert("pub ")?;

        assert_eq!(editor.text(), "struct A(pub usize, pub char)");

        editor.backspace()?;

        assert_eq!(editor.text(), "struct A(pubusize, pubchar)");
        assert_eq!(editor.get_selected_texts(), vec!["", ""]);
        Ok(())
    }

    #[test]
    fn multi_raise() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn f(){ let x = S(a); let y = S(b); }");
        let context = Context::default();

        // Select 'let x = S(a);'
        editor.match_literal(&context, "let x = S(a);")?;
        assert_eq!(editor.get_selected_texts(), vec!["let x = S(a);"]);

        editor.set_selection_mode(&context, SelectionMode::SyntaxTree)?;
        editor.add_cursor(&Movement::Next, &context)?;

        assert_eq!(
            editor.get_selected_texts(),
            vec!["let x = S(a);", "let y = S(b);"]
        );

        editor.set_selection_mode(&context, SelectionMode::OutermostNode)?;
        for _ in 0..5 {
            editor.move_selection(&context, Movement::Next)?;
        }

        assert_eq!(editor.get_selected_texts(), vec!["a", "b"]);

        editor.raise(&context)?;

        assert_eq!(editor.text(), "fn f(){ let x = a; let y = b; }");

        editor.undo()?;

        assert_eq!(editor.text(), "fn f(){ let x = S(a); let y = S(b); }");
        assert_eq!(editor.get_selected_texts(), vec!["a", "b"]);

        editor.redo()?;

        assert_eq!(editor.text(), "fn f(){ let x = a; let y = b; }");
        assert_eq!(editor.get_selected_texts(), vec!["a", "b"]);
        Ok(())
    }

    #[test]
    fn multi_exchange_sibling() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn f(x:a,y:b){} fn g(x:a,y:b){}");
        let context = Context::default();

        // Select 'fn f(x:a,y:b){}'
        editor.match_literal(&context, "fn f(x:a,y:b){}")?;
        assert_eq!(editor.get_selected_texts(), vec!["fn f(x:a,y:b){}"]);

        editor.set_selection_mode(&context, SelectionMode::SyntaxTree)?;
        editor.add_cursor(&Movement::Next, &context)?;
        assert_eq!(
            editor.get_selected_texts(),
            vec!["fn f(x:a,y:b){}", "fn g(x:a,y:b){}"]
        );

        editor.set_selection_mode(&context, SelectionMode::OutermostNode)?;
        for _ in 0..3 {
            editor.move_selection(&context, Movement::Next)?;
        }

        assert_eq!(editor.get_selected_texts(), vec!["x:a", "x:a"]);

        editor.set_selection_mode(&context, SelectionMode::SyntaxTree)?;
        editor.exchange(Movement::Next, &context)?;
        assert_eq!(editor.text(), "fn f(y:b,x:a){} fn g(y:b,x:a){}");
        assert_eq!(editor.get_selected_texts(), vec!["x:a", "x:a"]);

        editor.exchange(Movement::Previous, &context)?;
        assert_eq!(editor.text(), "fn f(x:a,y:b){} fn g(x:a,y:b){}");
        Ok(())
    }

    #[test]
    fn multi_paste() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(
            language(),
            "fn f(){ let x = S(spongebob_squarepants); let y = S(b); }",
        );
        let context = Context::default();

        // Select 'let x = S(a)'
        editor.match_literal(&context, "let x = S(spongebob_squarepants);")?;
        assert_eq!(
            editor.get_selected_texts(),
            vec!["let x = S(spongebob_squarepants);"]
        );

        editor.set_selection_mode(&context, SelectionMode::SyntaxTree)?;
        editor.add_cursor(&Movement::Next, &context)?;

        editor.set_selection_mode(&context, SelectionMode::OutermostNode)?;
        editor.move_selection(&context, Movement::Next)?;
        editor.move_selection(&context, Movement::Next)?;
        editor.move_selection(&context, Movement::Next)?;

        assert_eq!(
            editor.get_selected_texts(),
            vec!["S(spongebob_squarepants)", "S(b)"]
        );

        let context = Context::default();
        editor.cut()?;
        editor.enter_insert_mode(Direction::Start)?;

        editor.insert("Some(")?;
        editor.paste(&context)?;
        editor.insert(")")?;

        assert_eq!(
            editor.text(),
            "fn f(){ let x = Some(S(spongebob_squarepants)); let y = Some(S(b)); }"
        );
        Ok(())
    }

    #[test]
    fn toggle_highlight_mode() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn f(){ let x = S(a); let y = S(b); }");
        let context = Context::default();

        editor.set_selection_mode(&context, SelectionMode::Token)?;
        editor.toggle_highlight_mode();
        editor.move_selection(&context, Movement::Next)?;
        editor.move_selection(&context, Movement::Next)?;

        assert_eq!(editor.get_selected_texts(), vec!["fn f("]);

        // Toggle the second time should inverse the initial_range
        editor.toggle_highlight_mode();

        editor.move_selection(&context, Movement::Next)?;

        assert_eq!(editor.get_selected_texts(), vec!["f("]);

        editor.reset();

        assert_eq!(editor.get_selected_texts(), vec!["f"]);

        editor.move_selection(&context, Movement::Next)?;

        assert_eq!(editor.get_selected_texts(), vec!["("]);

        Ok(())
    }

    #[test]
    fn open_new_line() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(
            language(),
            "
fn f() {
    let x = S(a);
}
"
            .trim(),
        );

        // Move to the second line
        editor.select_line_at(1)?;

        assert_eq!(editor.get_selected_texts(), vec!["    let x = S(a);\n"]);

        editor.open_new_line()?;

        assert_eq!(editor.mode, Mode::Insert);

        editor.insert("let y = S(b);")?;

        assert_eq!(
            editor.text(),
            "
fn f() {
    let x = S(a);
    let y = S(b);
}"
            .trim()
        );
        Ok(())
    }

    #[test]
    fn paste_from_clipboard() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn f(){ let x = S(a); let y = S(b); }");
        let mut context = Context::default();

        context.set_clipboard_content("let z = S(c);".to_string());

        editor.reset();

        editor.paste(&context)?;

        assert_eq!(
            editor.text(),
            "let z = S(c);fn f(){ let x = S(a); let y = S(b); }"
        );
        Ok(())
    }

    #[test]
    fn enter_newline() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "");

        // Enter insert mode
        editor.enter_insert_mode(Direction::Start)?;
        // Type in 'hello'
        editor.handle_events(keys!("h e l l o"))?;

        // Type in enter
        editor.handle_events(keys!("enter"))?;

        // Type in 'world'
        editor.handle_events(keys!("w o r l d"))?;

        // Expect the text to be 'hello\nworld'
        assert_eq!(editor.text(), "hello\nworld");

        // Move cursor left
        editor.handle_events(keys!("left"))?;

        // Type in enter
        editor.handle_events(keys!("enter"))?;

        // Expect the text to be 'hello\nworl\nd'
        assert_eq!(editor.text(), "hello\nworl\nd");
        Ok(())
    }

    #[test]
    fn set_selection() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main() {}");

        // Select a range which highlights a node
        editor.set_selection(Position::new(0, 0)..Position::new(0, 2))?;

        assert_eq!(editor.selection_set.mode, SelectionMode::OutermostNode);

        // Select a range which does not highlights a node
        editor.set_selection(Position::new(0, 0)..Position::new(0, 1))?;

        assert_eq!(editor.selection_set.mode, SelectionMode::Custom);

        Ok(())
    }

    #[test]
    fn insert_mode_start() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main() {}");
        let context = Context::default();

        // Select the first word
        editor.set_selection_mode(&context, SelectionMode::Word)?;

        // Enter insert mode
        editor.enter_insert_mode(Direction::Start)?;

        // Type something
        editor.insert("hello")?;

        // Expect the text to be 'hellofn main() {}'
        assert_eq!(editor.text(), "hellofn main() {}");
        Ok(())
    }

    #[test]
    fn insert_mode_end() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main() {}");
        let context = Context::default();

        // Select the first token
        editor.set_selection_mode(&context, SelectionMode::Token)?;

        // Enter insert mode
        editor.enter_insert_mode(Direction::End)?;

        // Type something
        editor.insert("hello")?;

        // Expect the text to be 'fnhello main() {}'
        assert_eq!(editor.text(), "fnhello main() {}");
        Ok(())
    }

    #[test]
    fn highlight_kill() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main() {}");
        let context = Context::default();
        // Select first token
        editor.set_selection_mode(&context, SelectionMode::Token)?;
        editor.toggle_highlight_mode();
        editor.move_selection(&context, Movement::Next)?;

        assert_eq!(editor.get_selected_texts(), vec!["fn main"]);
        editor.kill(&context)?;
        assert_eq!(editor.get_selected_texts(), vec![""]);
        Ok(())
    }

    #[test]
    /// Kill means delete until the next selection
    fn delete_should_kill_if_possible_1() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main() {}");
        let context = Context::default();

        // Select first token
        editor.set_selection_mode(&context, SelectionMode::Token)?;

        // Delete
        editor.kill(&context)?;

        // Expect the text to be 'main() {}'
        assert_eq!(editor.text(), "main() {}");

        // Expect the current selection is 'main'
        assert_eq!(editor.get_selected_texts(), vec!["main"]);
        Ok(())
    }

    #[test]
    /// No gap between current and next selection
    fn delete_should_kill_if_possible_2() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main() {}");
        let context = Context::default();

        // Select first character
        editor.set_selection_mode(&context, SelectionMode::Character)?;

        // Delete
        editor.kill(&context)?;

        assert_eq!(editor.text(), "n main() {}");

        // Expect the current selection is 'n'
        assert_eq!(editor.get_selected_texts(), vec!["n"]);
        Ok(())
    }

    #[test]
    /// No next selection
    fn delete_should_kill_if_possible_3() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main() {}");
        let context = Context::default();

        // Select last token
        editor.set_selection_mode(&context, SelectionMode::Token)?;
        editor.move_selection(&context, Movement::Last)?;

        // Delete
        editor.kill(&context)?;

        assert_eq!(editor.text(), "fn main() {");

        // Expect the current selection is empty
        assert_eq!(editor.get_selected_texts(), vec![""]);
        Ok(())
    }

    #[test]
    fn delete_should_not_kill_if_not_possible() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn maima() {}");
        let context = Context::default();

        // Select first token
        editor.match_literal(&context, "ma")?;

        // Delete
        editor.kill(&context)?;

        // Expect the text to be 'fn ima() {}'
        assert_eq!(editor.text(), "fn ima() {}");

        // Expect the current selection is empty
        assert_eq!(editor.get_selected_texts(), vec![""]);
        Ok(())
    }

    #[test]
    fn enclose_left_bracket() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main() { x.y() }");
        let context = Context::default();

        // Select 'x.y()'
        editor.match_literal(&context, "x.y()")?;

        assert_eq!(editor.get_selected_texts(), vec!["x.y()"]);

        editor.handle_events(keys!("( { [ <")).unwrap();

        assert_eq!(editor.text(), "fn main() { <[{(x.y())}]> }");
        assert_eq!(editor.get_selected_texts(), vec!["<[{(x.y())}]>"]);
        Ok(())
    }

    #[test]
    fn enclose_right_bracket() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main() { x.y() }");
        let context = Context::default();

        // Select 'x.y()'
        editor.match_literal(&context, "x.y()")?;

        assert_eq!(editor.get_selected_texts(), vec!["x.y()"]);

        editor.handle_events(keys!(") } ] >")).unwrap();

        assert_eq!(editor.text(), "fn main() { <[{(x.y())}]> }");
        assert_eq!(editor.get_selected_texts(), vec!["<[{(x.y())}]>"]);
        Ok(())
    }

    #[test]
    fn multicursor_add_all() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(
            language(),
            "mod m { fn a(j:J){} fn b(k:K,l:L){} fn c(m:M,n:N,o:O){} }",
        );

        let context = Context::default();
        editor.match_literal(&context, "fn a")?;

        editor.set_selection_mode(&context, SelectionMode::OutermostNode)?;
        editor.set_selection_mode(&context, SelectionMode::SyntaxTree)?;

        assert_eq!(editor.get_selected_texts(), vec!["fn a(j:J){}"]);

        editor.add_cursor_to_all_selections(&context)?;

        editor.move_selection(&context, Movement::Down)?;
        editor.move_selection(&context, Movement::Next)?;
        editor.move_selection(&context, Movement::Down)?;

        assert_eq!(editor.get_selected_texts(), vec!["j:J", "k:K", "m:M"]);

        editor.add_cursor_to_all_selections(&context)?;

        assert_eq!(
            editor.get_selected_texts(),
            vec!["j:J", "k:K", "l:L", "m:M", "n:N", "o:O"]
        );

        Ok(())
    }

    #[test]
    fn enter_normal_mode_should_highlight_one_character() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn\nmain()\n{ x.y(); x.y(); x.y(); }");
        let context = Context::default();
        // Select x.y()
        editor.match_literal(&context, "x.y()")?;

        editor.enter_insert_mode(Direction::End)?;
        editor.enter_normal_mode()?;
        assert_eq!(editor.get_selected_texts(), vec![")"]);
        Ok(())
    }

    #[test]
    fn test_delete_word_backward_from_end_of_file() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn snake_case(camelCase: String) {}");
        let context = Context::default();

        // Go to the end of the file
        editor.set_selection_mode(&context, SelectionMode::Line)?;
        editor.enter_insert_mode(Direction::End)?;

        // Delete
        editor.delete_word_backward(&context)?;
        assert_eq!(editor.text(), "fn snake_case(camelCase: String) ");

        editor.delete_word_backward(&context)?;
        assert_eq!(editor.text(), "fn snake_case(camelCase: String");

        editor.delete_word_backward(&context)?;
        assert_eq!(editor.text(), "fn snake_case(camelCase: ");

        Ok(())
    }

    #[test]
    fn test_delete_word_backward_from_middle_of_file() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn snake_case(camelCase: String) {}");
        let context = Context::default();

        // Go to the middle of the file
        editor.set_selection_mode(&context, SelectionMode::Token)?;
        editor.move_selection(&context, Movement::Index(3))?;

        assert_eq!(editor.get_selected_texts(), vec!["camelCase"]);

        editor.enter_insert_mode(Direction::End)?;

        editor.delete_word_backward(&context)?;
        assert_eq!(editor.text(), "fn snake_case(camel: String) {}");

        editor.delete_word_backward(&context)?;
        assert_eq!(editor.text(), "fn snake_case(: String) {}");

        editor.delete_word_backward(&context)?;
        assert_eq!(editor.text(), "fn snake_case: String) {}");

        editor.delete_word_backward(&context)?;
        assert_eq!(editor.text(), "fn snake_: String) {}");

        editor.delete_word_backward(&context)?;
        assert_eq!(editor.text(), "fn : String) {}");

        editor.delete_word_backward(&context)?;
        assert_eq!(editor.text(), ": String) {}");

        editor.delete_word_backward(&context)?;
        assert_eq!(editor.text(), ": String) {}");

        Ok(())
    }

    #[test]
    fn home_end() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "hello\n");
        let context = Context::default();

        editor.enter_insert_mode(Direction::Start)?;

        editor.end(&context)?;
        editor.insert(" world")?;
        assert_eq!(editor.text(), "hello world\n");

        editor.home(&context)?;
        editor.insert("hey ")?;
        assert_eq!(editor.text(), "hey hello world\n");

        Ok(())
    }

    #[test]
    fn highlight_change() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "hello world yo");
        let context = Context::default();

        editor.toggle_highlight_mode();
        editor.set_selection_mode(&context, SelectionMode::Word)?;
        editor.move_selection(&context, Movement::Next)?;
        assert_eq!(editor.get_selected_texts(), vec!["hello world"]);
        editor.change()?;
        editor.insert("wow")?;
        assert_eq!(editor.get_selected_texts(), vec![""]);
        assert_eq!(editor.text(), "wow yo");
        Ok(())
    }
    #[test]
    fn scroll_page() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "1\n2 hey\n3");
        editor.set_rectangle(crate::rectangle::Rectangle {
            origin: Position::default(),
            width: 100,
            height: 3,
        });
        let context = Context::default();
        editor.scroll_page_down()?;
        assert_eq!(editor.current_line()?, "2 hey");
        editor.scroll_page_down()?;
        editor.match_literal(&context, "hey")?;
        assert_eq!(editor.get_selected_texts(), ["hey"]);
        editor.scroll_page_down()?;
        assert_eq!(editor.current_line()?, "3");
        editor.scroll_page_down()?;
        assert_eq!(editor.current_line()?, "3");

        editor.scroll_page_up()?;
        assert_eq!(editor.current_line()?, "2 hey");
        editor.scroll_page_up()?;
        assert_eq!(editor.current_line()?, "1");
        editor.scroll_page_up()?;
        assert_eq!(editor.current_line()?, "1");
        Ok(())
    }

    #[test]
    fn jump() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "Who lives on sea shore?\n yonky donkey");

        editor.set_rectangle(crate::rectangle::Rectangle {
            origin: Position::default(),
            width: 100,
            height: 1,
        });

        let context = Context::default();

        // In jump mode, the first stage labels each selection using their starting character,
        // On subsequent stages, the labels are random alphabets
        assert!(editor.jumps().is_empty());

        // Set the selection mode as word, and jump
        editor.set_selection_mode(&context, SelectionMode::Word)?;
        editor.jump(&context)?;

        // Expect the jump to be the first character of each word
        // Note 'y' and 'd' are excluded because they are out of view,
        // since the viewbox has only height of 1
        assert_eq!(editor.jump_chars(), &['w', 'l', 'o', 's', 's', '?']);

        // Press 's'
        editor.handle_key_event(&context, key!('s'))?;

        // Expect the jumps to be 'a' and 'b'
        assert_eq!(editor.jump_chars(), &['a', 'b']);

        // Press 'a'
        editor.handle_key_event(&context, key!('a'))?;

        // Expect the jumps to be empty
        assert!(editor.jump_chars().is_empty());

        // Expect the current selected content is 'sea'
        assert_eq!(editor.get_selected_texts(), vec!["sea"]);
        Ok(())
    }

    #[test]
    fn highlight_and_jump() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "who lives on sea shore?\n yonky donkey");

        editor.set_rectangle(crate::rectangle::Rectangle {
            origin: Position::default(),
            width: 100,
            height: 1,
        });

        let context = Context::default();
        // Set the selection mode as word
        editor.set_selection_mode(&context, SelectionMode::Word)?;
        editor.toggle_highlight_mode();
        editor.move_selection(&context, Movement::Next)?;
        editor.jump(&context)?;
        assert_eq!(editor.jump_chars(), &['w', 'l', 'o', 's', 's', '?']);
        Ok(())
    }

    #[test]
    fn jump_all_selection_start_with_same_char() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "who who who who");

        editor.set_rectangle(crate::rectangle::Rectangle {
            origin: Position::default(),
            width: 100,
            height: 1,
        });

        let context = Context::default();

        // Set the selection mode as word, and jump
        editor.set_selection_mode(&context, SelectionMode::Word)?;
        editor.jump(&context)?;

        // Expect the jump to NOT be the first character of each word
        // Since, the first character of each selection are the same, which is 'w'
        assert_eq!(editor.jump_chars(), &['a', 'b', 'c', 'd']);

        Ok(())
    }
}
