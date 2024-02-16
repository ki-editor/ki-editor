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
}
