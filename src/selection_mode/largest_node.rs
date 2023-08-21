use crate::selection::Selection;

use super::{ByteRange, SelectionMode};
use itertools::Itertools;

pub struct LargestNode;

impl SelectionMode for LargestNode {
    fn name(&self) -> &'static str {
        "LARGEST NODE"
    }
    fn iter<'a>(
        &self,
        _current_selection: &'a Selection,
        buffer: &'a crate::buffer::Buffer,
    ) -> anyhow::Result<Box<dyn Iterator<Item = ByteRange> + 'a>> {
        Ok(Box::new(
            tree_sitter_traversal::traverse(
                buffer.tree().walk(),
                tree_sitter_traversal::Order::Pre,
            )
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
}

#[cfg(test)]
mod test_largest_node {
    use crate::buffer::Buffer;

    use super::*;

    #[test]
    fn case_1() {
        let buffer = Buffer::new(
            tree_sitter_rust::language(),
            "fn main(x: usize) { let x = 1; }",
        );
        LargestNode.assert_all_selections(
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
