#[cfg(test)]
mod test_editor {

    use crate::{
        app::Dispatch,
        components::{
            component::Component,
            editor::{Direction, DispatchEditor, Editor, Mode, Movement, ViewAlignment},
            suggestive_editor::Info,
        },
        context::Context,
        grid::{Style, StyleKey},
        position::Position,
        selection::{Filter, FilterKind, FilterMechanism, FilterTarget, SelectionMode},
        selection_mode::inside::InsideKind,
        themes::Theme,
    };
    use DispatchEditor::*;

    use itertools::Itertools;
    use my_proc_macros::{hex, key, keys};
    use pretty_assertions::assert_eq;
    use tree_sitter_rust::language;

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

        editor.exchange(&context, Movement::Next)?;
        assert_eq!(
            editor.text(),
            "
let x = 1;
    fn main() {
    let y = 2;
}"
        );

        editor.exchange(&context, Movement::Previous)?;
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
        editor.exchange(&context, Movement::Next)?;
        assert_eq!(editor.text(), "nf main() { let x = 1; }");
        editor.exchange(&context, Movement::Next)?;
        assert_eq!(editor.text(), "n fmain() { let x = 1; }");

        editor.exchange(&context, Movement::Previous)?;
        assert_eq!(editor.text(), "nf main() { let x = 1; }");
        editor.exchange(&context, Movement::Previous)?;

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
        editor.add_cursor(&context, &Movement::Next)?;

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

        assert_eq!(editor.selection_set.mode, SelectionMode::SyntaxTree);

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
        editor.set_selection_mode(&context, SelectionMode::BottomNode)?;

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
        editor.set_selection_mode(&context, SelectionMode::BottomNode)?;
        editor.toggle_highlight_mode();
        editor.handle_movement(&context, Movement::Next)?;

        assert_eq!(editor.get_selected_texts(), vec!["fn main"]);
        editor.kill(&context)?;
        assert_eq!(editor.get_selected_texts(), vec!["("]);
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

        editor.set_selection_mode(&context, SelectionMode::SyntaxTree)?;

        assert_eq!(editor.get_selected_texts(), vec!["fn a(j:J){}"]);

        editor.add_cursor_to_all_selections(&context)?;

        editor.handle_movements(&context, &[Movement::Down, Movement::Next, Movement::Down])?;
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
    fn highlight_change() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "hello world yo");
        let context = Context::default();

        editor.toggle_highlight_mode();
        editor.set_selection_mode(&context, SelectionMode::Word)?;
        editor.handle_movement(&context, Movement::Next)?;
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
        editor.handle_movement(&context, Movement::Next)?;
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

    #[test]
    fn switch_view_alignment() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(
            language(),
            "abcde".split("").collect_vec().join("\n").trim(),
        );
        let context = Context::default();
        editor.set_rectangle(crate::rectangle::Rectangle {
            origin: Position::default(),
            width: 100,
            height: 4,
        });
        editor.set_selection_mode(&context, SelectionMode::Word)?;
        editor.handle_movement(&context, Movement::Next)?;
        editor.handle_movement(&context, Movement::Next)?;
        assert_eq!(editor.get_selected_texts(), ["c"]);
        assert_eq!(editor.current_view_alignment, None);

        editor.switch_view_alignment();
        assert_eq!(editor.current_view_alignment, Some(ViewAlignment::Top));

        editor.switch_view_alignment();
        assert_eq!(editor.current_view_alignment, Some(ViewAlignment::Center));

        editor.switch_view_alignment();
        assert_eq!(editor.current_view_alignment, Some(ViewAlignment::Bottom));
        editor.apply_dispatch(&context, MoveSelection(Movement::Previous))?;
        assert_eq!(editor.current_view_alignment, None);

        Ok(())
    }

    #[test]
    fn get_grid_parent_line() -> anyhow::Result<()> {
        let content = "
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
        .trim();
        let mut editor = Editor::from_text(language(), content);
        editor.set_rectangle(crate::rectangle::Rectangle {
            origin: Position::default(),
            width: 20,
            height: 6,
        });

        let mut theme = Theme::default();
        let parent_lines_background = hex!("#badbad");
        theme.ui.parent_lines_background = parent_lines_background;
        let bookmark_background_color = hex!("#cebceb");
        theme.ui.bookmark = Style::default().background_color(bookmark_background_color);
        let mut context = Context::default().set_theme(theme);

        // Go to "print()" and skip the first 3 lines for rendering
        editor.match_literal(&context, "print()")?;
        editor.set_scroll_offset(3);

        let result = editor.get_grid(&mut context);
        // Expect `fn main()` is visible although it is out of view,
        // because it is amongst the parent lines of the current selection
        let expected_grid = "
[No title]
2│fn main() {
4│  let y = 2;
5│  for a in b {
6│    let z = 4;
7│    print()
"
        .trim();
        assert_eq!(result.grid.to_string(), expected_grid);

        // Bookmark "z"
        editor.match_literal(&context, "z")?;
        editor.toggle_bookmarks();

        // Expect the parent lines of the current selections are highlighted with parent_lines_background,
        // regardless of whether the parent lines are inbound or outbound
        assert!([1, 3].into_iter().all(|row_index| {
            result.grid.rows[row_index]
                .iter()
                .all(|cell| cell.background_color == parent_lines_background)
        }));

        // Expect the current line is not treated as parent line
        assert!(!result.grid.rows[5]
            .iter()
            .any(|cell| cell.background_color == parent_lines_background));

        // Bookmark the "fn" token
        editor.match_literal(&context, "fn")?;
        editor.toggle_bookmarks();

        // Go to "print()" and skip the first 3 lines for rendering
        editor.match_literal(&context, "print()")?;
        editor.set_scroll_offset(3);

        let result = editor.get_grid(&mut context);
        assert_eq!(result.grid.to_string(), expected_grid);

        // Expect the decorations of outbound parent lines are rendered properly
        // In this case, the outbound parent line is "fn main() {"
        assert!(result.grid.rows[1][2..4]
            .iter()
            .all(|cell| bookmark_background_color == cell.background_color));

        // Expect the decorations of inbound lines are rendered properly
        // In this case, we want to check that the bookmark on "z" is rendered
        let z_cell = result.grid.rows[4][10].clone();
        assert_eq!(z_cell.symbol, "z");
        assert!(z_cell.background_color == bookmark_background_color);

        Ok(())
    }

    #[test]
    fn test_wrapped_lines() -> anyhow::Result<()> {
        let content = "
// hello world\n hey
"
        .trim();
        let mut context = Context::default();
        let mut editor = Editor::from_text(language(), content);
        editor.set_rectangle(crate::rectangle::Rectangle {
            origin: Position::default(),
            width: 13,
            height: 4,
        });

        editor.match_literal(&context, "world")?;
        editor.enter_insert_mode(Direction::End)?;
        let result = editor.get_grid(&mut context);

        let expected_grid = "
[No title]
1│// hello
↪│world
2│ hey
"
        .trim();
        assert_eq!(result.grid.to_string(), expected_grid);

        // Expect the cursor is after 'd'
        assert_eq!(
            result.cursor.unwrap().position(),
            &Position { line: 2, column: 7 }
        );
        Ok(())
    }

    #[test]
    fn syntax_highlighting() -> anyhow::Result<()> {
        let theme = Theme::default();
        let mut context = Context::default().set_theme(theme.clone());
        let content = "
fn main() { // too long
  let foo = 1;
  let bar = baba; let wrapped = coco;
}
"
        .trim();
        let mut editor = Editor::from_text(language(), content);
        editor.set_language(shared::language::from_extension("rs").unwrap())?;
        editor.match_literal(&context, "bar")?;
        editor.apply_syntax_highlighting(&mut context)?;
        editor.set_rectangle(crate::rectangle::Rectangle {
            origin: Position::default(),
            width: 20,
            height: 4,
        });
        editor.align_cursor_to_top();
        let result = editor.get_grid(&mut context);

        // The "long" of "too long" is not shown, because it exceeded the view width
        assert_eq!(
            result.to_string(),
            "
[No title]
1│fn main() { // too
3│  let █ar = baba;
↪│let wrapped = coco
"
            .trim()
        );
        let ranges = &[
            //
            // Expect the `fn` keyword of the outbound parent line "fn main() { // too long" is highlighted properly
            Position::new(1, 2)..=Position::new(1, 3),
            //
            // Expect the `let` keyword of line 3 (which is inbound and not wrapped) is highlighted properly
            Position::new(2, 4)..=Position::new(2, 6),
            //
            // Expect the `let` keyword of line 3 (which is inbound but wrapped) is highlighted properly
            Position::new(3, 2)..=Position::new(3, 4),
        ];

        result
            .grid
            .assert_ranges(ranges, |cell| cell.source == Some(StyleKey::SyntaxKeyword));

        // Expect decorations overrides syntax highlighting
        editor.match_literal(&context, "fn")?;
        editor.toggle_bookmarks();
        // Move cursor to next line, so that "fn" is not selected,
        //  so that we can test the style applied to "fn" ,
        // otherwise the style of primary selection anchors will override the bookmark style
        editor.match_literal(&context, "let")?;
        let result = editor.get_grid(&mut context);

        assert_eq!(
            result.grid.to_string(),
            "
[No title]
1│fn main() { // too
↪│ long
2│  let foo = 1;
"
            .trim()
        );
        result
            .grid
            .assert_range(&(Position::new(1, 2)..=Position::new(1, 3)), |cell| {
                cell.source == Some(StyleKey::UiBookmark)
            });

        Ok(())
    }

    #[test]
    fn empty_content_should_have_one_line() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "");
        editor.set_rectangle(crate::rectangle::Rectangle {
            origin: Position::default(),
            width: 20,
            height: 2,
        });
        let mut context = Context::default();
        let result = editor.get_grid(&mut context);
        assert_eq!(
            result.grid.to_string(),
            "
[No title]
1│
"
            .trim()
        );
        assert_eq!(result.cursor.unwrap().position(), &Position::new(1, 2));
        Ok(())
    }

    #[test]
    fn update_bookmark_position_with_undo_and_redo() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "foo bar spim");
        let context = Context::default();
        editor.set_selection_mode(&context, SelectionMode::Word)?;
        editor.handle_movements(&context, &[Movement::Next, Movement::Next])?;
        editor.toggle_bookmarks();
        editor.set_selection_mode(&context, SelectionMode::Bookmark)?;
        assert_eq!(editor.get_selected_texts(), ["spim"]);
        editor.set_selection_mode(&context, SelectionMode::Word)?;
        editor.handle_movements(&context, &[Movement::Previous, Movement::Previous])?;
        // Kill "foo"
        editor.kill(&context)?;

        assert_eq!(editor.content(), "bar spim");

        // Expect bookmark position is updated, and still selects "spim"
        editor.set_selection_mode(&context, SelectionMode::Bookmark)?;

        assert_eq!(editor.get_selected_texts(), ["spim"]);

        // Undo
        editor.undo()?;
        assert_eq!(editor.content(), "foo bar spim");

        // Expect bookmark position is updated, and still selects "spim"
        editor.set_selection_mode(&context, SelectionMode::Bookmark)?;
        assert_eq!(editor.get_selected_texts(), ["spim"]);

        // Redo
        editor.redo()?;
        assert_eq!(editor.content(), "bar spim");

        // Expect bookmark position is updated, and still selects "spim"
        editor.set_selection_mode(&context, SelectionMode::Bookmark)?;
        assert_eq!(editor.get_selected_texts(), ["spim"]);

        Ok(())
    }

    #[test]
    fn saving_should_not_destroy_bookmark_if_selections_not_modified() -> anyhow::Result<()> {
        let input = "// foo bar spim\nfn foo() {}\n";

        let mut editor = Editor::from_text(language(), input);
        editor.set_language(shared::language::from_extension("rs").unwrap())?;
        let context = Context::default();
        editor.set_selection_mode(&context, SelectionMode::Word)?;
        editor.handle_movements(&context, &[Movement::Next, Movement::Next])?;
        editor.toggle_bookmarks();
        editor.set_selection_mode(&context, SelectionMode::Bookmark)?;
        assert_eq!(editor.get_selected_texts(), ["bar"]);

        // Expect the formatted content is the same as the input
        let formatted_content = editor.get_formatted_content().unwrap();
        assert_eq!(formatted_content, input);

        editor.save()?;
        editor.set_selection_mode(&context, SelectionMode::Character)?;
        assert_eq!(editor.get_selected_texts(), ["b"]);
        editor.set_selection_mode(&context, SelectionMode::Bookmark)?;
        assert_eq!(editor.get_selected_texts(), ["bar"]);

        Ok(())
    }

    #[test]
    fn filters_should_be_cleared_after_changing_selection_mode() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "foo bar spam");
        let context = Context::default();
        editor.apply_dispatches(
            &context,
            [
                SetSelectionMode(SelectionMode::Word),
                FilterPush(Filter::new(
                    FilterKind::Keep,
                    FilterTarget::Content,
                    FilterMechanism::Literal("a".to_string()),
                )),
                CursorAddToAllSelections,
            ]
            .to_vec(),
        )?;
        assert_eq!(editor.get_selected_texts(), &["bar", "spam"]);
        editor.apply_dispatches(
            &context,
            [
                CursorKeepPrimaryOnly,
                SetSelectionMode(SelectionMode::LineTrimmed),
                SetSelectionMode(SelectionMode::Word),
                CursorAddToAllSelections,
            ]
            .to_vec(),
        )?;
        assert_eq!(editor.get_selected_texts(), &["foo", "bar", "spam"]);

        Ok(())
    }

    #[test]
    fn omit() -> Result<(), anyhow::Error> {
        fn run_test(
            (input, kind, target, mechanism, expected_output): (
                &str,
                FilterKind,
                FilterTarget,
                FilterMechanism,
                &[&str],
            ),
        ) -> anyhow::Result<()> {
            let mut editor = Editor::from_text(language(), input);
            let context = Context::default();
            editor.apply_dispatches(
                &context,
                [
                    SetSelectionMode(SelectionMode::Word),
                    FilterPush(Filter::new(kind, target, mechanism)),
                    DispatchEditor::CursorAddToAllSelections,
                ]
                .to_vec(),
            )?;
            // Assert the selection is only "bar" and "spam"
            assert_eq!(
                editor.get_selected_texts(),
                expected_output,
                "Expected output is {:?}",
                expected_output
            );
            Ok(())
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
    fn raise_bottom_node() -> anyhow::Result<()> {
        let input = "fn main() { x + 1 }";
        let mut editor = Editor::from_text(language(), input);
        let context = Context::default();
        editor.apply_dispatches(
            &context,
            [
                MatchLiteral("x + 1".to_string()),
                SetSelectionMode(SelectionMode::TopNode),
                MoveSelection(Movement::Down),
                Raise,
            ]
            .to_vec(),
        )?;
        assert_eq!(editor.content(), "fn main() { x }");
        Ok(())
    }

    #[test]
    fn hierarchy_of_line() -> anyhow::Result<()> {
        let input = "  hello  \n ";
        let mut editor = Editor::from_text(language(), input);
        let context = Context::default();
        editor.apply_dispatch(&context, SetSelectionMode(SelectionMode::LineTrimmed))?;
        assert_eq!(editor.get_selected_texts(), ["hello  "]);
        editor.apply_dispatch(&context, MoveSelection(Movement::Up))?;
        assert_eq!(editor.get_selected_texts(), ["  hello  \n"]);
        editor.apply_dispatch(&context, MoveSelection(Movement::Down))?;
        assert_eq!(editor.get_selected_texts(), ["hello  "]);
        Ok(())
    }
}
