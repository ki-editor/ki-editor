use crate::selection_mode::ApplyMovementResult;

use super::{ByteRange, SelectionMode, TopNode};

pub struct SyntaxTree;

impl SelectionMode for SyntaxTree {
    fn name(&self) -> &'static str {
        "SYNTAX TREE"
    }
    fn jumps(
        &self,
        params: super::SelectionModeParams,
        chars: Vec<char>,
        line_number_range: std::ops::Range<usize>,
    ) -> anyhow::Result<Vec<crate::components::editor::Jump>> {
        // Why do we use TopNode.jumps?
        // Because I realize I only use TopNode for jumping, and I never use jump in SyntaxTree
        // With this decision, TopNode selection mode can be removed from user-space, and we get one more keymap space
        TopNode.jumps(params, chars, line_number_range)
    }
    fn iter<'a>(
        &'a self,
        params: super::SelectionModeParams<'a>,
    ) -> anyhow::Result<Box<dyn Iterator<Item = super::ByteRange> + 'a>> {
        let buffer = params.buffer;
        let current_selection = params.current_selection;
        let node = buffer
            .get_current_node(current_selection, false)?
            .ok_or(anyhow::anyhow!(
                "SyntaxTree::iter: Cannot find Treesitter language"
            ))?;

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
        params: super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        super::TopNode.current(params)
    }
    fn up(
        &self,
        params: super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        let result = self.parent(params)?;
        Ok(result.map(|result| result.selection))
    }
    fn down(
        &self,
        params: super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        let result = self.first_child(params)?;
        Ok(result.map(|result| result.selection))
    }
    fn parent(
        &self,
        params: super::SelectionModeParams,
    ) -> anyhow::Result<Option<ApplyMovementResult>> {
        self.select_vertical(params, true)
    }
    fn first_child(
        &self,
        params: super::SelectionModeParams,
    ) -> anyhow::Result<Option<ApplyMovementResult>> {
        self.select_vertical(params, false)
    }
}

impl SyntaxTree {
    fn select_vertical(
        &self,
        params: super::SelectionModeParams,
        go_up: bool,
    ) -> anyhow::Result<Option<ApplyMovementResult>> {
        let Some(mut node) = params
            .buffer
            .get_current_node(params.current_selection, false)?
        else {
            return Ok(None);
        };
        while let Some(some_node) = get_node(node, go_up) {
            // This is necessary because sometimes the parent node can have the same range as
            // the current node
            if some_node.range() != node.range() {
                return Ok(Some(ApplyMovementResult::from_selection(
                    ByteRange::new(some_node.byte_range())
                        .to_selection(params.buffer, params.current_selection)?,
                )));
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
        selection::{CharIndex, Filters, Selection},
        selection_mode::SelectionModeParams,
    };

    use super::*;

    #[test]
    fn case_1() {
        let buffer = Buffer::new(
            Some(tree_sitter_rust::language()),
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
        let buffer = Buffer::new(
            Some(tree_sitter_rust::language()),
            "fn main() { let x = S(a); }",
        );
        SyntaxTree.assert_all_selections(
            &buffer,
            Selection::default().set_range((CharIndex(20)..CharIndex(21)).into()),
            &[(20..21, "S"), (21..24, "(a)")],
        );
    }

    #[test]
    fn parent() {
        let buffer = Buffer::new(
            Some(tree_sitter_rust::language()),
            "fn main() { let x = X {z,b,c:d} }",
        );

        let child_range = (CharIndex(23)..CharIndex(24)).into();
        let selection = SyntaxTree.parent(SelectionModeParams {
            buffer: &buffer,
            current_selection: &Selection::new(child_range),
            cursor_direction: &crate::components::editor::Direction::Start,
            filters: &Filters::default(),
        });

        let parent_range = selection.unwrap().unwrap().selection.range();
        assert_eq!(parent_range, (CharIndex(22)..CharIndex(31)).into());

        let selection = SyntaxTree.first_child(SelectionModeParams {
            buffer: &buffer,
            current_selection: &Selection::new(parent_range),
            cursor_direction: &crate::components::editor::Direction::Start,
            filters: &Filters::default(),
        });

        let child_range = selection.unwrap().unwrap().selection.range();
        assert_eq!(child_range, child_range);
    }

    #[test]
    fn current() {
        let buffer = Buffer::new(
            Some(tree_sitter_rust::language()),
            "
fn main() {
  let x = X;
}"
            .trim(),
        );

        // Let the range be the space before `let`
        let range = (CharIndex(12)..CharIndex(13)).into();
        let selection = SyntaxTree.current(SelectionModeParams {
            buffer: &buffer,
            current_selection: &Selection::new(range),
            cursor_direction: &crate::components::editor::Direction::Start,
            filters: &Filters::default(),
        });

        let actual_range = buffer.slice(&selection.unwrap().unwrap().range()).unwrap();
        // Although the cursor is placed before `let`, the expected selection should be
        // `let x = X;`, which is the largest node of the current line
        assert_eq!(actual_range, "let x = X;");
    }
}
