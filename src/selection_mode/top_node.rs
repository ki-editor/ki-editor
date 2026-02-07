use super::{ByteRange, IterBasedSelectionMode};
use itertools::Itertools;

pub struct TopNode;

impl IterBasedSelectionMode for TopNode {
    fn iter<'a>(
        &self,
        params: &super::SelectionModeParams<'a>,
    ) -> anyhow::Result<Box<dyn Iterator<Item = ByteRange> + 'a>> {
        let buffer = params.buffer;
        let tree = buffer.tree().ok_or(anyhow::anyhow!(
            "TopNode::iter: cannot find Treesitter language"
        ))?;
        let root_node_id = tree.root_node().id();
        Ok(Box::new(
            tree_sitter_traversal2::traverse(tree.walk(), tree_sitter_traversal2::Order::Pre)
                .filter(|node| node.id() != root_node_id)
                .chunk_by(|node| node.byte_range().start)
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
}

#[cfg(test)]
mod test_top_node {
    use crate::{buffer::Buffer, selection::Selection};

    use super::*;

    #[test]
    fn case_1() {
        let buffer = Buffer::new(
            Some(tree_sitter_rust::LANGUAGE.into()),
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
