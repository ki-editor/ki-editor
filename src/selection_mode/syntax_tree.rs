use super::{ByteRange, SelectionMode};

pub struct SyntaxTree;

impl SelectionMode for SyntaxTree {
    fn name(&self) -> &'static str {
        "SYNTAX TREE"
    }
    fn iter<'a>(
        &'a self,
        params: super::SelectionModeParams<'a>,
    ) -> anyhow::Result<Box<dyn Iterator<Item = super::ByteRange> + 'a>> {
        let buffer = params.buffer;
        let current_selection = params.current_selection;
        let node = buffer.get_current_node(current_selection, false)?;

        if let Some(parent) = node.parent() {
            Ok(Box::new(
                (0..parent.named_child_count())
                    .filter_map(move |i| parent.named_child(i))
                    .map(|node| ByteRange::new(node.byte_range())),
            ))
        } else {
            Ok(Box::new(std::iter::empty()))
        }
    }
    fn current(
        &self,
        super::SelectionModeParams {
            buffer,
            current_selection,
            ..
        }: super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        let byte_range = buffer
            .get_current_node(current_selection, true)?
            .byte_range();
        let range = buffer.byte_range_to_char_index_range(&byte_range)?;
        Ok(Some(current_selection.clone().set_range(range)))
    }
    fn up(
        &self,
        params: super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        self.select_vertical(params, true)
    }
    fn down(
        &self,
        params: super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        self.select_vertical(params, false)
    }
}

impl SyntaxTree {
    fn select_vertical(
        &self,
        params: super::SelectionModeParams,
        go_up: bool,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        let mut node = params
            .buffer
            .get_current_node(params.current_selection, false)?;
        while let Some(some_node) = get_node(node, go_up) {
            // This is necessary because sometimes the parent node can have the same range as
            // the current node
            if some_node.range() != node.range() {
                return ByteRange::new(some_node.byte_range())
                    .to_selection(params.buffer, params.current_selection)
                    .map(Some);
            }
            node = some_node;
        }
        Ok(None)
    }
}

fn get_node(node: tree_sitter::Node, go_up: bool) -> Option<tree_sitter::Node> {
    match go_up {
        true => node.parent(),
        false => node.named_child(0),
    }
}

#[cfg(test)]
mod test_syntax_tree {
    use crate::{
        buffer::Buffer,
        char_index_range::CharIndexRange,
        context::Context,
        selection::{CharIndex, Filters, Selection},
        selection_mode::SelectionModeParams,
    };

    use super::*;

    #[test]
    fn case_1() {
        let buffer = Buffer::new(
            tree_sitter_rust::language(),
            "fn main() { let x = X {z,b,c:d} }",
        );
        SyntaxTree.assert_all_selections(
            &buffer,
            Selection::default().set_range((CharIndex(23)..CharIndex(24)).into()),
            &[(23..24, "z"), (25..26, "b"), (27..30, "c:d")],
        );
    }

    #[test]
    fn case_2() {
        let buffer = Buffer::new(tree_sitter_rust::language(), "fn main() { let x = S(a); }");
        SyntaxTree.assert_all_selections(
            &buffer,
            Selection::default().set_range((CharIndex(20)..CharIndex(21)).into()),
            &[(20..21, "S"), (21..24, "(a)")],
        );
    }

    #[test]
    /// Getting the current node should get the largest node where its start range
    /// is same as the current selection start range
    fn current_1() -> anyhow::Result<()> {
        let buffer = Buffer::new(tree_sitter_rust::language(), "fn main(a:A,b:B) {  }");
        let input_range: CharIndexRange = (CharIndex(8)..CharIndex(12)).into();
        assert_eq!(buffer.slice(&input_range)?, "a:A,");
        let selection = SyntaxTree.current(SelectionModeParams {
            context: &Context::default(),
            buffer: &buffer,
            current_selection: &Selection::new(input_range),
            cursor_direction: &crate::components::editor::Direction::Start,
            filters: &Filters::default(),
        });

        let new_range = selection.unwrap().unwrap().range();
        assert_eq!(buffer.slice(&new_range)?, "a:A");
        Ok(())
    }

    #[test]
    /// Selecting the current node should not select the root node
    /// when the current selection is at the beginning of the buffer
    fn current_2() -> anyhow::Result<()> {
        let buffer = Buffer::new(tree_sitter_rust::language(), "use a; use b;");
        let input_range: CharIndexRange = (CharIndex(0)..CharIndex(1)).into();
        let selection = SyntaxTree.current(SelectionModeParams {
            context: &Context::default(),
            buffer: &buffer,
            current_selection: &Selection::new(input_range),
            cursor_direction: &crate::components::editor::Direction::Start,
            filters: &Filters::default(),
        });

        let new_range = selection.unwrap().unwrap().range();
        assert_eq!(buffer.slice(&new_range)?, "use a;");
        Ok(())
    }

    #[test]
    fn up() {
        let buffer = Buffer::new(
            tree_sitter_rust::language(),
            "fn main() { let x = X {z,b,c:d} }",
        );

        let child_range = (CharIndex(23)..CharIndex(24)).into();
        let context = Context::default();
        let selection = SyntaxTree.up(SelectionModeParams {
            context: &context,
            buffer: &buffer,
            current_selection: &Selection::new(child_range),
            cursor_direction: &crate::components::editor::Direction::Start,
            filters: &Filters::default(),
        });

        let parent_range = selection.unwrap().unwrap().range();
        assert_eq!(parent_range, (CharIndex(22)..CharIndex(31)).into());

        let selection = SyntaxTree.down(SelectionModeParams {
            context: &context,
            buffer: &buffer,
            current_selection: &Selection::new(parent_range),
            cursor_direction: &crate::components::editor::Direction::Start,
            filters: &Filters::default(),
        });

        let child_range = selection.unwrap().unwrap().range();
        assert_eq!(child_range, child_range);
    }
}
