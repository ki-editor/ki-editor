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
    fn enclose_left_bracket() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main() { x.y() }");
        let mut context = Context::default();

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
        let mut context = Context::default();

        // Select 'x.y()'
        editor.match_literal(&context, "x.y()")?;

        assert_eq!(editor.get_selected_texts(), vec!["x.y()"]);

        editor.handle_events(keys!(") } ] >")).unwrap();

        assert_eq!(editor.text(), "fn main() { <[{(x.y())}]> }");
        assert_eq!(editor.get_selected_texts(), vec!["<[{(x.y())}]>"]);
        Ok(())
    }

    #[test]
    fn filters_should_be_cleared_after_changing_selection_mode() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "foo bar spam");
        let mut context = Context::default();
        editor.apply_dispatches(
            &mut context,
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
            &mut context,
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
            let mut context = Context::default();
            editor.apply_dispatches(
                &mut context,
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
        let mut context = Context::default();
        editor.apply_dispatches(
            &mut context,
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
        let mut context = Context::default();
        editor.apply_dispatch(&mut context, SetSelectionMode(SelectionMode::LineTrimmed))?;
        assert_eq!(editor.get_selected_texts(), ["hello  "]);
        editor.apply_dispatch(&mut context, MoveSelection(Movement::Up))?;
        assert_eq!(editor.get_selected_texts(), ["  hello  \n"]);
        editor.apply_dispatch(&mut context, MoveSelection(Movement::Down))?;
        assert_eq!(editor.get_selected_texts(), ["hello  "]);
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
}
