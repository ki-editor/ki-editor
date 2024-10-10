use crate::components::editor::IfCurrentNotFound;

use super::{
    syntax_node::get_node, ApplyMovementResult, ByteRange, SelectionMode, SyntaxNode, Token,
};
use itertools::Itertools;

pub(crate) struct TopNode;

impl SelectionMode for TopNode {
    fn iter<'a>(
        &self,
        params: super::SelectionModeParams<'a>,
    ) -> anyhow::Result<Box<dyn Iterator<Item = ByteRange> + 'a>> {
        let buffer = params.buffer;
        let tree = buffer.tree().ok_or(anyhow::anyhow!(
            "TopNode::iter: cannot find Treesitter language"
        ))?;
        let root_node_id = tree.root_node().id();
        Ok(Box::new(
            tree_sitter_traversal::traverse(tree.walk(), tree_sitter_traversal::Order::Pre)
                .filter(|node| node.id() != root_node_id)
                .group_by(|node| node.byte_range().start)
                .into_iter()
                .map(|(_, group)| {
                    ByteRange::new(
                        group
                            .into_iter()
                            .max_by_key(|node| node.byte_range().end)
                            .unwrap()
                            .byte_range(),
                    )
                })
                .collect_vec()
                .into_iter(),
        ))
    }
    fn last(
        &self,
        params: super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        SyntaxNode { coarse: true }.last(params)
    }

    fn first(
        &self,
        params: super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        SyntaxNode { coarse: true }.first(params)
    }

    fn parent(
        &self,
        params: super::SelectionModeParams,
    ) -> anyhow::Result<Option<ApplyMovementResult>> {
        self.select_vertical(params.clone(), true)
    }
    fn first_child(
        &self,
        params: super::SelectionModeParams,
    ) -> anyhow::Result<Option<ApplyMovementResult>> {
        self.select_vertical(params, false)
    }
    fn real_next(
        &self,
        params: super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        let buffer = params.buffer;
        let current_selection = params.current_selection;
        let node = buffer
            .get_current_node(current_selection, false)?
            .ok_or(anyhow::anyhow!(
                "SyntaxNode::iter: Cannot find Treesitter language"
            ))?;
        Ok(node.next_named_sibling().and_then(|node| {
            ByteRange::new(node.byte_range())
                .to_selection(params.buffer, params.current_selection)
                .ok()
        }))
    }
    fn real_previous(
        &self,
        params: super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        let buffer = params.buffer;
        let current_selection = params.current_selection;
        let node = buffer
            .get_current_node(current_selection, false)?
            .ok_or(anyhow::anyhow!(
                "SyntaxNode::iter: Cannot find Treesitter language"
            ))?;
        Ok(node.prev_named_sibling().and_then(|node| {
            ByteRange::new(node.byte_range())
                .to_selection(params.buffer, params.current_selection)
                .ok()
        }))
    }
}
impl TopNode {
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
        while let Some(some_node) = get_node(node, go_up, true) {
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

#[cfg(test)]
mod test_top_node {
    use crate::{buffer::Buffer, selection::Selection};

    use super::*;

    #[test]
    fn case_1() {
        let buffer = Buffer::new(
            Some(tree_sitter_rust::language()),
            "fn main(x: usize) { let x = 1; }",
        );
        TopNode.assert_all_selections(
            &buffer,
            Selection::default(),
            &[
                (0..32, "fn main(x: usize) { let x = 1; }"),
                (3..7, "main"),
                (7..17, "(x: usize)"),
                (8..16, "x: usize"),
                (9..10, ":"),
                (11..16, "usize"),
                (16..17, ")"),
                (18..32, "{ let x = 1; }"),
                (20..30, "let x = 1;"),
                (24..25, "x"),
                (26..27, "="),
                (28..29, "1"),
                (29..30, ";"),
                (31..32, "}"),
            ],
        );
    }
}
